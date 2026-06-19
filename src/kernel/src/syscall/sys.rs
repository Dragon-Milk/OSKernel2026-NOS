use alloc::{vec, vec::Vec};
use core::ffi::c_char;

use axconfig::ARCH;
use axerrno::{AxError, AxResult};
use axfs::FS_CONTEXT;
use axsync::Mutex;
use linux_raw_sys::{
    general::{GRND_INSECURE, GRND_NONBLOCK, GRND_RANDOM, NGROUPS_MAX},
    system::{new_utsname, sysinfo},
};
use starry_vm::{VmMutPtr, VmPtr, vm_load, vm_write_slice};

use crate::task::{AsThread, UtsState, processes};

pub fn sys_getuid() -> AxResult<isize> {
    Ok(axtask::current().as_thread().proc_data.ids().0 as _)
}

pub fn sys_geteuid() -> AxResult<isize> {
    Ok(axtask::current().as_thread().proc_data.ids().1 as _)
}

pub fn sys_getgid() -> AxResult<isize> {
    Ok(axtask::current().as_thread().proc_data.ids().3 as _)
}

pub fn sys_getegid() -> AxResult<isize> {
    Ok(axtask::current().as_thread().proc_data.ids().4 as _)
}

pub fn sys_setuid(uid: u32) -> AxResult<isize> {
    debug!("sys_setuid <= uid: {uid}");
    let curr = axtask::current();
    let proc_data = &curr.as_thread().proc_data;
    let (ruid, euid, suid, _, _, _) = proc_data.ids();
    if euid != 0 && uid != ruid && uid != euid && uid != suid {
        return Err(AxError::OperationNotPermitted);
    }
    if euid == 0 {
        proc_data.set_uid(uid);
    } else {
        proc_data.set_resuid(None, Some(uid), None);
    }
    Ok(0)
}

pub fn sys_setgid(gid: u32) -> AxResult<isize> {
    debug!("sys_setgid <= gid: {gid}");
    let curr = axtask::current();
    let proc_data = &curr.as_thread().proc_data;
    let (_, euid, _, rgid, egid, sgid) = proc_data.ids();
    if euid != 0 && gid != rgid && gid != egid && gid != sgid {
        return Err(AxError::OperationNotPermitted);
    }
    if euid == 0 {
        proc_data.set_gid(gid);
    } else {
        proc_data.set_resgid(None, Some(gid), None);
    }
    Ok(0)
}

pub fn sys_setfsuid(uid: u32) -> AxResult<isize> {
    debug!("sys_setfsuid <= uid: {uid}");
    let curr = axtask::current();
    let proc_data = &curr.as_thread().proc_data;
    let old_fsuid = proc_data.fsids().0;
    let (ruid, euid, suid, _, _, _) = proc_data.ids();
    if uid != u32::MAX && (euid == 0 || uid == ruid || uid == euid || uid == suid || uid == old_fsuid) {
        proc_data.set_fsuid(uid);
    }
    Ok(old_fsuid as _)
}

pub fn sys_setfsgid(gid: u32) -> AxResult<isize> {
    debug!("sys_setfsgid <= gid: {gid}");
    let curr = axtask::current();
    let proc_data = &curr.as_thread().proc_data;
    let old_fsgid = proc_data.fsids().1;
    let (_, euid, _, rgid, egid, sgid) = proc_data.ids();
    if gid != u32::MAX && (euid == 0 || gid == rgid || gid == egid || gid == sgid || gid == old_fsgid) {
        proc_data.set_fsgid(gid);
    }
    Ok(old_fsgid as _)
}

pub fn sys_getgroups(size: usize, list: *mut u32) -> AxResult<isize> {
    debug!("sys_getgroups <= size: {size}");
    let curr = axtask::current();
    let (group_count, groups) = curr.as_thread().proc_data.groups();
    if size == 0 {
        return Ok(group_count as _);
    }
    if size > NGROUPS_MAX as usize || size < group_count {
        return Err(AxError::InvalidInput);
    }
    vm_write_slice(list, &groups[..group_count])?;
    Ok(group_count as _)
}

pub fn sys_setgroups(size: usize, list: *const u32) -> AxResult<isize> {
    debug!("sys_setgroups <= size: {size}");
    if size > NGROUPS_MAX as usize {
        return Err(AxError::InvalidInput);
    }
    let curr = axtask::current();
    let proc_data = &curr.as_thread().proc_data;
    if proc_data.ids().1 != 0 {
        return Err(AxError::OperationNotPermitted);
    }
    let groups = if size == 0 {
        vec![]
    } else {
        vm_load(list.cast::<u32>(), size)?
    };
    proc_data.set_groups(&groups[..groups.len().min(32)]);
    Ok(0)
}

pub fn sys_getcpu(cpu: *mut u32, node: *mut u32) -> AxResult<isize> {
    debug!("sys_getcpu <= cpu: {cpu:p}, node: {node:p}");
    if let Some(cpu) = cpu.nullable() {
        cpu.vm_write(0)?;
    }
    if let Some(node) = node.nullable() {
        node.vm_write(0)?;
    }
    Ok(0)
}

const fn pad_str(info: &str) -> [c_char; 65] {
    let mut data: [c_char; 65] = [0; 65];
    // this needs #![feature(const_copy_from_slice)]
    // data[..info.len()].copy_from_slice(info.as_bytes());
    unsafe {
        core::ptr::copy_nonoverlapping(info.as_ptr().cast(), data.as_mut_ptr(), info.len());
    }
    data
}

const UTS_NAME_LEN: usize = 64;

static UTS_STATE: Mutex<UtsState> = Mutex::new(UtsState {
    nodename: pad_str("starry"),
    domainname: pad_str("https://github.com/Starry-OS/StarryOS"),
});

pub fn sys_uname(name: *mut new_utsname) -> AxResult<isize> {
    let curr = axtask::current();
    let proc_data = &curr.as_thread().proc_data;
    let uts_state = proc_data.uts_state().unwrap_or_else(|| *UTS_STATE.lock());
    let utsname = new_utsname {
        sysname: pad_str("Linux"),
        nodename: uts_state.nodename,
        release: pad_str("10.0.0"),
        version: pad_str("10.0.0"),
        machine: pad_str(ARCH),
        domainname: uts_state.domainname,
    };
    name.vm_write(utsname)?;
    Ok(0)
}

pub fn sys_sysinfo(info: *mut sysinfo) -> AxResult<isize> {
    // FIXME: Zeroable
    let mut kinfo: sysinfo = unsafe { core::mem::zeroed() };
    kinfo.procs = processes().len() as _;
    kinfo.mem_unit = 1;
    info.vm_write(kinfo)?;
    Ok(0)
}

pub fn sys_syslog(_type: i32, _buf: *mut c_char, _len: usize) -> AxResult<isize> {
    Ok(0)
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct GetRandomFlags: u32 {
        const NONBLOCK = GRND_NONBLOCK;
        const RANDOM = GRND_RANDOM;
        const INSECURE = GRND_INSECURE;
    }
}

pub fn sys_getrandom(buf: *mut u8, len: usize, flags: u32) -> AxResult<isize> {
    if len == 0 {
        return Ok(0);
    }
    let flags = GetRandomFlags::from_bits_retain(flags);

    debug!("sys_getrandom <= buf: {buf:p}, len: {len}, flags: {flags:?}");

    let path = if flags.contains(GetRandomFlags::RANDOM) {
        "/dev/random"
    } else {
        "/dev/urandom"
    };

    let f = FS_CONTEXT.lock().resolve(path)?;
    let mut kbuf = vec![0; len];
    let len = f.entry().as_file()?.read_at(&mut kbuf, 0)?;

    vm_write_slice(buf, &kbuf)?;

    Ok(len as _)
}

pub fn sys_sethostname(name: *const c_char, len: usize) -> AxResult<isize> {
    debug!("sys_sethostname <= len={len}");
    let nodename = read_uts_name(name, len)?;
    let curr = axtask::current();
    let proc_data = &curr.as_thread().proc_data;
    if let Some(mut uts_state) = proc_data.uts_state() {
        uts_state.nodename = nodename;
        proc_data.set_uts_state(uts_state);
    } else {
        UTS_STATE.lock().nodename = nodename;
    }
    Ok(0)
}

pub fn sys_setdomainname(name: *const c_char, len: usize) -> AxResult<isize> {
    debug!("sys_setdomainname <= len={len}");
    let domainname = read_uts_name(name, len)?;
    let curr = axtask::current();
    let proc_data = &curr.as_thread().proc_data;
    if let Some(mut uts_state) = proc_data.uts_state() {
        uts_state.domainname = domainname;
        proc_data.set_uts_state(uts_state);
    } else {
        UTS_STATE.lock().domainname = domainname;
    }
    Ok(0)
}

pub fn current_uts_state() -> UtsState {
    *UTS_STATE.lock()
}

fn read_uts_name(name: *const c_char, len: usize) -> AxResult<[c_char; UTS_NAME_LEN + 1]> {
    if len > UTS_NAME_LEN {
        return Err(AxError::InvalidInput);
    }
    if axtask::current().as_thread().proc_data.ids().1 != 0 {
        return Err(AxError::OperationNotPermitted);
    }

    let mut data = [0; UTS_NAME_LEN + 1];
    if len > 0 {
        let bytes = vm_load(name.cast::<u8>(), len)?;
        for (dst, src) in data.iter_mut().zip(bytes.into_iter()) {
            *dst = src as c_char;
        }
    }
    Ok(data)
}

pub fn sys_ptrace(_request: i32, _pid: i32, _addr: usize, _data: usize) -> AxResult<isize> {
    Err(AxError::Unsupported)
}

pub fn sys_seccomp(_op: u32, _flags: u32, _args: *const ()) -> AxResult<isize> {
    warn!("dummy sys_seccomp");
    Ok(0)
}

#[cfg(target_arch = "riscv64")]
pub fn sys_riscv_flush_icache() -> AxResult<isize> {
    riscv::asm::fence_i();
    Ok(0)
}
