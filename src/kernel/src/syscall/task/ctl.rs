use core::ffi::c_char;

use axerrno::{AxError, AxResult};
use axtask::current;
use linux_raw_sys::general::{__user_cap_data_struct, __user_cap_header_struct};
use starry_signal::Signo;
use starry_vm::{VmMutPtr, VmPtr, vm_write_slice};

use crate::{
    mm::vm_load_string,
    task::{AsThread, Capabilities, get_process_data},
};

const CAPABILITY_VERSION_1: u32 = 0x19980330;
const CAPABILITY_VERSION_2: u32 = 0x20071026;
const CAPABILITY_VERSION_3: u32 = 0x20080522;

fn validate_cap_header(
    header_ptr: *mut __user_cap_header_struct,
) -> AxResult<__user_cap_header_struct> {
    // FIXME: AnyBitPattern
    let mut header = unsafe { header_ptr.vm_read_uninit()?.assume_init() };

    // Accept all three capability versions; upgrade v1/v2 to v3 for the caller
    match header.version {
        CAPABILITY_VERSION_1 | CAPABILITY_VERSION_2 => {
            header.version = CAPABILITY_VERSION_3;
            header_ptr.vm_write(header)?;
        }
        CAPABILITY_VERSION_3 => {}
        _ => return Err(AxError::InvalidInput),
    }

    // capget/capset on a non-current process returns EPERM (not ESRCH)
    if header.pid != 0 {
        let curr_pid = current().as_thread().proc_data.proc.pid();
        if header.pid as u32 != curr_pid {
            // Returning EINVAL for non-existent pid matches Linux behaviour
            let _ = get_process_data(header.pid as u32).map_err(|_| AxError::InvalidInput)?;
        }
    }

    Ok(header)
}

pub fn sys_capget(
    header: *mut __user_cap_header_struct,
    data: *mut __user_cap_data_struct,
) -> AxResult<isize> {
    validate_cap_header(header)?;

    let capabilities = current().as_thread().proc_data.capabilities();
    data.vm_write(__user_cap_data_struct {
        effective: capabilities.effective,
        permitted: capabilities.permitted,
        inheritable: capabilities.inheritable,
    })?;
    Ok(0)
}

pub fn sys_capset(
    header: *mut __user_cap_header_struct,
    data: *mut __user_cap_data_struct,
) -> AxResult<isize> {
    let header = validate_cap_header(header)?;

    // Only root (or CAP_SETPCAP) can set capabilities on another process
    if header.pid != 0 {
        let curr_pid = current().as_thread().proc_data.proc.pid();
        if header.pid as u32 != curr_pid {
            return Err(AxError::OperationNotPermitted);
        }
    }

    // Validate the requested capability data: inheritable must be a subset of permitted
    let data = unsafe { data.vm_read_uninit()?.assume_init() };
    if data.inheritable & !data.permitted != 0 {
        return Err(AxError::OperationNotPermitted);
    }
    current()
        .as_thread()
        .proc_data
        .set_capabilities(Capabilities {
            effective: data.effective,
            permitted: data.permitted,
            inheritable: data.inheritable,
        });

    Ok(0)
}

pub fn sys_umask(mask: u32) -> AxResult<isize> {
    let curr = current();
    let old = curr.as_thread().proc_data.replace_umask(mask);
    Ok(old as isize)
}

pub fn sys_setreuid(ruid: u32, euid: u32) -> AxResult<isize> {
    let curr = current();
    let proc_data = &curr.as_thread().proc_data;
    let (old_ruid, old_euid, old_suid, _, _, _) = proc_data.ids();
    let new_ruid = (ruid != u32::MAX).then_some(ruid);
    let new_euid = (euid != u32::MAX).then_some(euid);

    if old_euid != 0 {
        check_unprivileged_id_change(new_ruid, old_ruid, old_euid, old_suid)?;
        check_unprivileged_id_change(new_euid, old_ruid, old_euid, old_suid)?;
    }

    let update_suid = new_ruid.is_some() || new_euid.is_some_and(|euid| euid != old_ruid);
    let new_suid = if update_suid {
        Some(new_euid.unwrap_or(old_euid))
    } else {
        None
    };
    proc_data.set_resuid(new_ruid, new_euid, new_suid);
    Ok(0)
}

pub fn sys_setregid(rgid: u32, egid: u32) -> AxResult<isize> {
    let curr = current();
    let proc_data = &curr.as_thread().proc_data;
    let (_, euid, _, old_rgid, old_egid, old_sgid) = proc_data.ids();
    let new_rgid = (rgid != u32::MAX).then_some(rgid);
    let new_egid = (egid != u32::MAX).then_some(egid);

    if euid != 0 {
        if let Some(rgid) = new_rgid
            && rgid != old_rgid
            && rgid != old_egid
        {
            return Err(AxError::OperationNotPermitted);
        }
        if let Some(egid) = new_egid
            && egid != old_rgid
            && egid != old_egid
            && egid != old_sgid
        {
            return Err(AxError::OperationNotPermitted);
        }
    }

    let update_sgid = new_rgid.is_some() || new_egid.is_some_and(|egid| egid != old_rgid);
    let new_sgid = if update_sgid {
        Some(new_egid.unwrap_or(old_egid))
    } else {
        None
    };
    proc_data.set_resgid(new_rgid, new_egid, new_sgid);
    Ok(0)
}

pub fn sys_setresuid(ruid: u32, euid: u32, suid: u32) -> AxResult<isize> {
    let curr = current();
    let proc_data = &curr.as_thread().proc_data;
    let (old_ruid, old_euid, old_suid, _, _, _) = proc_data.ids();
    let new_ruid = (ruid != u32::MAX).then_some(ruid);
    let new_euid = (euid != u32::MAX).then_some(euid);
    let new_suid = (suid != u32::MAX).then_some(suid);

    if old_euid != 0 {
        check_unprivileged_id_change(new_ruid, old_ruid, old_euid, old_suid)?;
        check_unprivileged_id_change(new_euid, old_ruid, old_euid, old_suid)?;
        check_unprivileged_id_change(new_suid, old_ruid, old_euid, old_suid)?;
    }

    proc_data.set_resuid(new_ruid, new_euid, new_suid);
    Ok(0)
}

pub fn sys_getresuid(ruid: *mut u32, euid: *mut u32, suid: *mut u32) -> AxResult<isize> {
    let (r, e, s, _, _, _) = current().as_thread().proc_data.ids();
    ruid.vm_write(r)?;
    euid.vm_write(e)?;
    suid.vm_write(s)?;
    Ok(0)
}

pub fn sys_setresgid(rgid: u32, egid: u32, sgid: u32) -> AxResult<isize> {
    let curr = current();
    let proc_data = &curr.as_thread().proc_data;
    let (_, euid, _, old_rgid, old_egid, old_sgid) = proc_data.ids();
    let new_rgid = (rgid != u32::MAX).then_some(rgid);
    let new_egid = (egid != u32::MAX).then_some(egid);
    let new_sgid = (sgid != u32::MAX).then_some(sgid);

    if euid != 0 {
        check_unprivileged_id_change(new_rgid, old_rgid, old_egid, old_sgid)?;
        check_unprivileged_id_change(new_egid, old_rgid, old_egid, old_sgid)?;
        check_unprivileged_id_change(new_sgid, old_rgid, old_egid, old_sgid)?;
    }

    proc_data.set_resgid(new_rgid, new_egid, new_sgid);
    Ok(0)
}

fn check_unprivileged_id_change(
    new_id: Option<u32>,
    real_id: u32,
    effective_id: u32,
    saved_id: u32,
) -> AxResult<()> {
    if let Some(id) = new_id
        && id != real_id
        && id != effective_id
        && id != saved_id
    {
        return Err(AxError::OperationNotPermitted);
    }
    Ok(())
}

pub fn sys_getresgid(rgid: *mut u32, egid: *mut u32, sgid: *mut u32) -> AxResult<isize> {
    let (_, _, _, r, e, s) = current().as_thread().proc_data.ids();
    rgid.vm_write(r)?;
    egid.vm_write(e)?;
    sgid.vm_write(s)?;
    Ok(0)
}

pub fn sys_get_mempolicy(
    _policy: *mut i32,
    _nodemask: *mut usize,
    _maxnode: usize,
    _addr: usize,
    _flags: usize,
) -> AxResult<isize> {
    warn!("Dummy get_mempolicy called");
    Ok(0)
}

/// prctl() is called with a first argument describing what to do, and further
/// arguments with a significance depending on the first one.
/// The first argument can be:
/// - PR_SET_NAME: set the name of the calling thread, using the value pointed to by `arg2`
/// - PR_GET_NAME: get the name of the calling
/// - PR_SET_SECCOMP: enable seccomp mode, with the mode specified in `arg2`
/// - PR_MCE_KILL: set the machine check exception policy
/// - PR_SET_MM options: set various memory management options (start/end code/data/brk/stack)
pub fn sys_prctl(
    option: u32,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> AxResult<isize> {
    use linux_raw_sys::prctl::*;

    debug!("sys_prctl <= option: {option}, args: {arg2}, {arg3}, {arg4}, {arg5}");

    match option {
        PR_SET_NAME => {
            let s = vm_load_string(arg2 as *const c_char)?;
            current().set_name(&s);
        }
        PR_GET_NAME => {
            let name = current().name();
            let len = name.len().min(15);
            let mut buf = [0; 16];
            buf[..len].copy_from_slice(&name.as_bytes()[..len]);
            vm_write_slice(arg2 as _, &buf)?;
        }
        PR_SET_PDEATHSIG => {
            if arg2 != 0 && Signo::from_repr(arg2 as u8).is_none() {
                return Err(AxError::InvalidInput);
            }
            current().as_thread().set_parent_death_signal(arg2 as u32);
        }
        PR_GET_PDEATHSIG => {
            (arg2 as *mut u32).vm_write(current().as_thread().parent_death_signal())?;
        }
        PR_SET_DUMPABLE => {
            if arg2 > 1 {
                return Err(AxError::InvalidInput);
            }
        }
        PR_GET_SECCOMP | PR_SET_SECCOMP => {
            return Err(AxError::InvalidInput);
        }
        PR_SET_TIMING => {
            if arg2 != PR_TIMING_STATISTICAL as usize {
                return Err(AxError::InvalidInput);
            }
        }
        PR_SET_TIMERSLACK => {
            let curr = current();
            let thr = curr.as_thread();
            let value = if arg2 == 0 {
                thr.default_timer_slack_ns()
            } else {
                arg2
            };
            thr.set_timer_slack_ns(value);
        }
        PR_GET_TIMERSLACK => {
            return Ok(current().as_thread().timer_slack_ns() as isize);
        }
        PR_SET_NO_NEW_PRIVS | PR_GET_NO_NEW_PRIVS | PR_SET_THP_DISABLE | PR_GET_THP_DISABLE => {
            return Err(AxError::InvalidInput);
        }
        PR_GET_SPECULATION_CTRL => {
            return Err(AxError::InvalidInput);
        }
        PR_SET_SECUREBITS | PR_CAPBSET_DROP => {
            return Err(AxError::OperationNotPermitted);
        }
        PR_MCE_KILL => {}
        PR_CAPBSET_READ => {
            // Return 0 (capability not in bounding set) since we don't
            // implement capabilities.
            // The result is written to *(int *)arg3.
            (arg3 as *mut u32).vm_write(0)?;
        }
        PR_CAP_AMBIENT => {
            match arg2 as u32 {
                PR_CAP_AMBIENT_CLEAR_ALL if arg3 == 0 && arg4 == 0 && arg5 == 0 => {}
                PR_CAP_AMBIENT_IS_SET if arg3 < 64 && arg4 == 0 && arg5 == 0 => return Ok(0),
                PR_CAP_AMBIENT_RAISE | PR_CAP_AMBIENT_LOWER if arg3 < 64 && arg4 == 0 && arg5 == 0 => {}
                _ => return Err(AxError::InvalidInput),
            }
        }
        PR_SET_MM => {
            // not implemented; but avoid annoying warnings
            return Err(AxError::InvalidInput);
        }
        _ => {
            warn!("sys_prctl: unsupported option {option}");
            return Err(AxError::InvalidInput);
        }
    }

    Ok(0)
}
