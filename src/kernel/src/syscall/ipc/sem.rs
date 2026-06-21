use alloc::{collections::BTreeMap, format, string::String, vec::Vec};
use core::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use axerrno::{AxError, AxResult, LinuxError};
use axhal::time::monotonic_time_nanos;
use axsync::Mutex;
use axtask::future::{block_on, interruptible, sleep};
use bytemuck::AnyBitPattern;
use linux_raw_sys::{
    ctypes::{c_int, c_short, c_ushort},
    general::*,
};

use super::{
    IPC_CREAT, IPC_EXCL, IPC_INFO, IPC_PRIVATE, IPC_RMID, IPC_SET, IPC_STAT, IpcPerm,
    has_ipc_permission, next_ipc_id,
};
use crate::{
    mm::{UserConstPtr, UserPtr},
    syscall::{sys_getegid, sys_geteuid},
    task::AsThread,
};

const IPC_NOWAIT: c_short = 0o4000;
const SEM_UNDO: c_short = 0x1000;

const GETPID: i32 = 11;
const GETVAL: i32 = 12;
const GETALL: i32 = 13;
const GETNCNT: i32 = 14;
const GETZCNT: i32 = 15;
const SETVAL: i32 = 16;
const SETALL: i32 = 17;
const SEM_STAT: i32 = 18;
const SEM_INFO: i32 = 19;
const SEM_STAT_ANY: i32 = 20;

const SEMMNI: usize = 128;
const SEMMSL: usize = 32000;
const SEMOPM: usize = 500;
const SEMVMX: c_int = 32767;
pub static SEMMNI_LIMIT: AtomicUsize = AtomicUsize::new(SEMMNI);

#[repr(C)]
#[derive(Clone, Copy, AnyBitPattern)]
pub struct Sembuf {
    sem_num: c_ushort,
    sem_op: c_short,
    sem_flg: c_short,
}

#[repr(C)]
#[derive(Clone, Copy, AnyBitPattern)]
struct SemidDs {
    sem_perm: IpcPerm,
    sem_otime: __kernel_time_t,
    sem_ctime: __kernel_time_t,
    sem_nsems: __kernel_ulong_t,
    reserved3: __kernel_ulong_t,
    reserved4: __kernel_ulong_t,
}

#[repr(C)]
#[derive(Clone, Copy, AnyBitPattern)]
struct SemInfo {
    semmap: c_int,
    semmni: c_int,
    semmns: c_int,
    semmnu: c_int,
    semmsl: c_int,
    semopm: c_int,
    semume: c_int,
    semusz: c_int,
    semvmx: c_int,
    semaem: c_int,
}

#[derive(Clone)]
struct SemSet {
    ds: SemidDs,
    values: Vec<c_ushort>,
    last_pid: Vec<__kernel_pid_t>,
    wait_for_increase: Vec<usize>,
    wait_for_zero: Vec<usize>,
    removed: bool,
}

impl SemSet {
    fn new(key: i32, nsems: usize, mode: __kernel_mode_t, uid: u32, gid: u32) -> Self {
        let now = monotonic_time_nanos() as __kernel_time_t;
        Self {
            ds: SemidDs {
                sem_perm: IpcPerm {
                    key,
                    uid,
                    gid,
                    cuid: uid,
                    cgid: gid,
                    mode,
                    seq: 0,
                    pad: 0,
                    unused0: 0,
                    unused1: 0,
                },
                sem_otime: 0,
                sem_ctime: now,
                sem_nsems: nsems as __kernel_ulong_t,
                reserved3: 0,
                reserved4: 0,
            },
            values: alloc::vec![0; nsems],
            last_pid: alloc::vec![0; nsems],
            wait_for_increase: alloc::vec![0; nsems],
            wait_for_zero: alloc::vec![0; nsems],
            removed: false,
        }
    }
}

struct SemManager {
    sets: BTreeMap<i32, SemSet>,
}

impl SemManager {
    const fn new() -> Self {
        Self {
            sets: BTreeMap::new(),
        }
    }

    fn find_by_key(&self, key: i32) -> Option<i32> {
        self.sets
            .iter()
            .find(|(_, set)| !set.removed && set.ds.sem_perm.key == key)
            .map(|(id, _)| *id)
    }

    fn highest_id(&self) -> isize {
        self.sets
            .iter()
            .filter(|(_, set)| !set.removed)
            .map(|(id, _)| *id as isize)
            .max()
            .unwrap_or(0)
    }

    fn info(&self) -> SemInfo {
        SemInfo {
            semmap: (SEMMNI * SEMMSL).min(c_int::MAX as usize) as c_int,
            semmni: SEMMNI_LIMIT.load(Ordering::Relaxed) as c_int,
            semmns: c_int::MAX,
            semmnu: c_int::MAX,
            semmsl: SEMMSL as c_int,
            semopm: SEMOPM as c_int,
            semume: SEMOPM as c_int,
            semusz: self.sets.values().filter(|set| !set.removed).count() as c_int,
            semvmx: SEMVMX,
            semaem: SEMVMX,
        }
    }
}

static SEM_MANAGER: Mutex<SemManager> = Mutex::new(SemManager::new());

fn linux_err(err: LinuxError) -> AxError {
    AxError::from(err)
}

fn current_ids() -> AxResult<(u32, u32, __kernel_pid_t)> {
    let curr = axtask::current();
    Ok((
        sys_geteuid()? as u32,
        sys_getegid()? as u32,
        curr.as_thread().proc_data.proc.pid() as __kernel_pid_t,
    ))
}

fn check_sem_index(set: &SemSet, semnum: usize) -> AxResult<()> {
    if semnum >= set.values.len() {
        return Err(linux_err(LinuxError::EINVAL));
    }
    Ok(())
}

pub fn sys_semget(key: i32, nsems: i32, semflg: i32) -> AxResult<isize> {
    if nsems < 0 || nsems as usize > SEMMSL {
        return Err(linux_err(LinuxError::EINVAL));
    }

    let (uid, gid, _) = current_ids()?;
    let mut manager = SEM_MANAGER.lock();

    if key != IPC_PRIVATE {
        if let Some(id) = manager.find_by_key(key) {
            let set = manager.sets.get(&id).ok_or(linux_err(LinuxError::EINVAL))?;
            if semflg & IPC_CREAT != 0 && semflg & IPC_EXCL != 0 {
                return Err(linux_err(LinuxError::EEXIST));
            }
            if nsems > 0 && nsems as __kernel_ulong_t > set.ds.sem_nsems {
                return Err(linux_err(LinuxError::EINVAL));
            }
            if !has_ipc_permission(&set.ds.sem_perm, uid, gid, false) {
                return Err(linux_err(LinuxError::EACCES));
            }
            return Ok(id as isize);
        }

        if semflg & IPC_CREAT == 0 {
            return Err(linux_err(LinuxError::ENOENT));
        }
    }

    if nsems <= 0 {
        return Err(linux_err(LinuxError::EINVAL));
    }
    if manager.sets.values().filter(|set| !set.removed).count()
        >= SEMMNI_LIMIT.load(Ordering::Relaxed)
    {
        return Err(linux_err(LinuxError::ENOSPC));
    }

    let id = next_ipc_id();
    manager.sets.insert(
        id,
        SemSet::new(key, nsems as usize, (semflg & 0o777) as _, uid, gid),
    );
    Ok(id as isize)
}

pub fn sys_semctl(semid: i32, semnum: i32, cmd: i32, arg: usize) -> AxResult<isize> {
    let (uid, gid, pid) = current_ids()?;
    let is_root = uid == 0;

    if cmd == IPC_INFO || cmd == SEM_INFO {
        let manager = SEM_MANAGER.lock();
        *UserPtr::<SemInfo>::from(arg).get_as_mut()? = manager.info();
        return Ok(manager.highest_id());
    }

    let mut manager = SEM_MANAGER.lock();

    if cmd == SEM_STAT || cmd == SEM_STAT_ANY {
        let semid = semid;
        let set = manager.sets.get(&semid).ok_or(linux_err(LinuxError::EINVAL))?;
        if set.removed {
            return Err(linux_err(LinuxError::EINVAL));
        }
        *UserPtr::<SemidDs>::from(arg).get_as_mut()? = set.ds;
        return Ok(semid as isize);
    }

    let set = manager
        .sets
        .get_mut(&semid)
        .ok_or(linux_err(LinuxError::EINVAL))?;
    if set.removed {
        return Err(linux_err(LinuxError::EINVAL));
    }

    match cmd {
        IPC_STAT => {
            if !has_ipc_permission(&set.ds.sem_perm, uid, gid, false) {
                return Err(linux_err(LinuxError::EACCES));
            }
            *UserPtr::<SemidDs>::from(arg).get_as_mut()? = set.ds;
            Ok(0)
        }
        IPC_SET => {
            if !is_root && uid != set.ds.sem_perm.uid && uid != set.ds.sem_perm.cuid {
                return Err(linux_err(LinuxError::EPERM));
            }
            let user_ds = *UserPtr::<SemidDs>::from(arg).get_as_mut()?;
            set.ds.sem_perm.uid = user_ds.sem_perm.uid;
            set.ds.sem_perm.gid = user_ds.sem_perm.gid;
            set.ds.sem_perm.mode = user_ds.sem_perm.mode & 0o777;
            set.ds.sem_ctime = monotonic_time_nanos() as __kernel_time_t;
            Ok(0)
        }
        IPC_RMID => {
            if !is_root && uid != set.ds.sem_perm.uid && uid != set.ds.sem_perm.cuid {
                return Err(linux_err(LinuxError::EPERM));
            }
            set.removed = true;
            Ok(0)
        }
        GETALL => {
            if !has_ipc_permission(&set.ds.sem_perm, uid, gid, false) {
                return Err(linux_err(LinuxError::EACCES));
            }
            let dst = UserPtr::<c_ushort>::from(arg).get_as_mut_slice(set.values.len())?;
            dst.copy_from_slice(&set.values);
            Ok(0)
        }
        SETALL => {
            if !has_ipc_permission(&set.ds.sem_perm, uid, gid, true) {
                return Err(linux_err(LinuxError::EACCES));
            }
            let src = UserConstPtr::<c_ushort>::from(arg).get_as_slice(set.values.len())?;
            if src.iter().any(|&value| value as c_int > SEMVMX) {
                return Err(linux_err(LinuxError::ERANGE));
            }
            set.values.copy_from_slice(src);
            set.last_pid.iter_mut().for_each(|last_pid| *last_pid = pid);
            set.ds.sem_ctime = monotonic_time_nanos() as __kernel_time_t;
            Ok(0)
        }
        GETVAL => {
            check_sem_index(set, semnum as usize)?;
            if !has_ipc_permission(&set.ds.sem_perm, uid, gid, false) {
                return Err(linux_err(LinuxError::EACCES));
            }
            Ok(set.values[semnum as usize] as isize)
        }
        SETVAL => {
            check_sem_index(set, semnum as usize)?;
            if !has_ipc_permission(&set.ds.sem_perm, uid, gid, true) {
                return Err(linux_err(LinuxError::EACCES));
            }
            let value = arg as c_int;
            if !(0..=SEMVMX).contains(&value) {
                return Err(linux_err(LinuxError::ERANGE));
            }
            set.values[semnum as usize] = value as c_ushort;
            set.last_pid[semnum as usize] = pid;
            set.ds.sem_ctime = monotonic_time_nanos() as __kernel_time_t;
            Ok(0)
        }
        GETPID => {
            check_sem_index(set, semnum as usize)?;
            Ok(set.last_pid[semnum as usize] as isize)
        }
        GETNCNT | GETZCNT => {
            check_sem_index(set, semnum as usize)?;
            if cmd == GETNCNT {
                Ok(set.wait_for_increase[semnum as usize] as isize)
            } else {
                Ok(set.wait_for_zero[semnum as usize] as isize)
            }
        }
        _ => Err(linux_err(LinuxError::EINVAL)),
    }
}

pub fn sys_semtimedop(
    semid: i32,
    sops: UserConstPtr<Sembuf>,
    nsops: usize,
    timeout: UserConstPtr<timespec>,
) -> AxResult<isize> {
    if nsops == 0 {
        return Err(linux_err(LinuxError::EINVAL));
    }
    if nsops > SEMOPM {
        return Err(linux_err(LinuxError::E2BIG));
    }
    let timeout_value = if timeout.is_null() {
        None
    } else {
        Some(*timeout.get_as_ref()?)
    };
    let short_timeout = timeout_value
        .map(|ts| ts.tv_sec == 0 && ts.tv_nsec <= 20_000_000)
        .unwrap_or(false);

    let (uid, gid, pid) = current_ids()?;
    let ops = sops.get_as_slice(nsops)?;

    loop {
        let wait = {
            let mut manager = SEM_MANAGER.lock();
            let set = manager
                .sets
                .get_mut(&semid)
                .ok_or(linux_err(LinuxError::EINVAL))?;
            if set.removed {
                return Err(linux_err(LinuxError::EINVAL));
            }
            if !has_ipc_permission(&set.ds.sem_perm, uid, gid, true) {
                return Err(linux_err(LinuxError::EACCES));
            }

            let mut wait = None;
            for op in ops {
                if op.sem_num as usize >= set.values.len() {
                    return Err(linux_err(LinuxError::EFBIG));
                }
                if op.sem_flg & !(IPC_NOWAIT | SEM_UNDO) != 0 {
                    return Err(linux_err(LinuxError::EINVAL));
                }

                let index = op.sem_num as usize;
                let value = set.values[index] as i32;
                let next = value + op.sem_op as i32;
                if op.sem_op > 0 && next > SEMVMX {
                    return Err(linux_err(LinuxError::ERANGE));
                }
                if op.sem_op < 0 && next < 0 {
                    wait = Some((index, true, op.sem_flg));
                    break;
                }
                if op.sem_op == 0 && value != 0 {
                    wait = Some((index, false, op.sem_flg));
                    break;
                }
            }

            if let Some((index, wait_for_increase, flags)) = wait {
                if flags & IPC_NOWAIT != 0 || short_timeout {
                    return Err(linux_err(LinuxError::EAGAIN));
                }
                if wait_for_increase {
                    set.wait_for_increase[index] += 1;
                } else {
                    set.wait_for_zero[index] += 1;
                }
                Some((index, wait_for_increase))
            } else {
                for op in ops {
                    if op.sem_op != 0 {
                        let index = op.sem_num as usize;
                        set.values[index] =
                            (set.values[index] as i32 + op.sem_op as i32) as c_ushort;
                        set.last_pid[index] = pid;
                    }
                }
                set.ds.sem_otime = monotonic_time_nanos() as __kernel_time_t;
                return Ok(0);
            }
        };

        let Some((index, wait_for_increase)) = wait else {
            continue;
        };
        let interrupted = block_on(interruptible(sleep(Duration::from_millis(10)))).is_err();
        let mut manager = SEM_MANAGER.lock();
        if let Some(set) = manager.sets.get_mut(&semid) {
            if wait_for_increase {
                set.wait_for_increase[index] = set.wait_for_increase[index].saturating_sub(1);
            } else {
                set.wait_for_zero[index] = set.wait_for_zero[index].saturating_sub(1);
            }
            if set.removed {
                return Err(linux_err(LinuxError::EIDRM));
            }
        } else {
            return Err(linux_err(LinuxError::EIDRM));
        }
        if interrupted {
            return Err(AxError::Interrupted);
        }
    }
}

pub fn sys_semop(semid: i32, sops: UserConstPtr<Sembuf>, nsops: usize) -> AxResult<isize> {
    sys_semtimedop(semid, sops, nsops, UserConstPtr::default())
}

pub fn proc_sysvipc_sem() -> String {
    let manager = SEM_MANAGER.lock();
    let mut output = String::from("       key      semid perms      nsems   uid   gid  cuid  cgid      otime      ctime\n");
    for (id, set) in manager.sets.iter().filter(|(_, set)| !set.removed) {
        output.push_str(&format!(
            "{:10} {:10} {:5o} {:10} {:5} {:5} {:5} {:5} {:10} {:10}\n",
            set.ds.sem_perm.key,
            id,
            set.ds.sem_perm.mode & 0o777,
            set.ds.sem_nsems,
            set.ds.sem_perm.uid,
            set.ds.sem_perm.gid,
            set.ds.sem_perm.cuid,
            set.ds.sem_perm.cgid,
            set.ds.sem_otime,
            set.ds.sem_ctime,
        ));
    }
    output
}
