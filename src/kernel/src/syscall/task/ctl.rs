use core::ffi::c_char;

use axerrno::{AxError, AxResult};
use axtask::current;
use linux_raw_sys::general::{__user_cap_data_struct, __user_cap_header_struct};
use starry_vm::{VmMutPtr, VmPtr, vm_write_slice};

use crate::{
    mm::vm_load_string,
    task::{AsThread, get_process_data},
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

    data.vm_write(__user_cap_data_struct {
        effective: u32::MAX,
        permitted: u32::MAX,
        inheritable: u32::MAX,
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

    Ok(0)
}

pub fn sys_umask(mask: u32) -> AxResult<isize> {
    let curr = current();
    let old = curr.as_thread().proc_data.replace_umask(mask);
    Ok(old as isize)
}

pub fn sys_setreuid(ruid: u32, euid: u32) -> AxResult<isize> {
    current().as_thread().proc_data.set_reuid(ruid, euid)?;
    Ok(0)
}

pub fn sys_setresuid(ruid: u32, euid: u32, suid: u32) -> AxResult<isize> {
    current()
        .as_thread()
        .proc_data
        .set_resuid(ruid, euid, suid)?;
    Ok(0)
}

pub fn sys_setresgid(rgid: u32, egid: u32, sgid: u32) -> AxResult<isize> {
    current()
        .as_thread()
        .proc_data
        .set_resgid(rgid, egid, sgid)?;
    Ok(0)
}

fn gid_arg_or_unchanged(gid: i32) -> AxResult<u32> {
    if gid < -1 {
        return Err(AxError::InvalidInput);
    }
    Ok(gid as u32)
}

pub fn sys_setregid(rgid: i32, egid: i32) -> AxResult<isize> {
    current()
        .as_thread()
        .proc_data
        .set_regid(gid_arg_or_unchanged(rgid)?, gid_arg_or_unchanged(egid)?)?;
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
        PR_SET_SECCOMP => {}
        PR_MCE_KILL => {}
        PR_CAPBSET_READ => {
            // Return 0 (capability not in bounding set) since we don't
            // implement capabilities.
            // The result is written to *(int *)arg3.
            (arg3 as *mut u32).vm_write(0)?;
        }
        PR_CAPBSET_DROP => {
            // Dropping a capability from the bounding set always succeeds
            // in a non-capability-aware kernel.
        }
        PR_CAP_AMBIENT => {
            // Ambient capabilities are always empty; operations are no-ops.
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
