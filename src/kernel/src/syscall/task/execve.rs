use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::ffi::c_char;

use axerrno::{AxError, AxResult};
use axhal::uspace::UserContext;
use axtask::current;
use starry_vm::vm_load_until_nul;

use crate::{
    config::USER_HEAP_BASE,
    file::{FD_TABLE, resolve_at},
    mm::{load_user_app, vm_load_string},
    task::AsThread,
};

fn can_execute(mode: axfs_ng_vfs::NodePermission, owner: u32, group: u32) -> bool {
    let (_, euid, _, _, egid, _) = current().as_thread().proc_data.ids();

    if euid == 0 {
        return mode.intersects(
            axfs_ng_vfs::NodePermission::OWNER_EXEC
                | axfs_ng_vfs::NodePermission::GROUP_EXEC
                | axfs_ng_vfs::NodePermission::OTHER_EXEC,
        );
    }

    if euid == owner {
        mode.contains(axfs_ng_vfs::NodePermission::OWNER_EXEC)
    } else if egid == group {
        mode.contains(axfs_ng_vfs::NodePermission::GROUP_EXEC)
    } else {
        mode.contains(axfs_ng_vfs::NodePermission::OTHER_EXEC)
    }
}

pub fn sys_execve(
    uctx: &mut UserContext,
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> AxResult<isize> {
    let path = vm_load_string(path)?;
    let (args, envs) = load_exec_args_envs(argv, envp)?;

    execve_inner(uctx, path, args, envs)
}

fn load_exec_args_envs(
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> AxResult<(Vec<String>, Vec<String>)> {
    let mut args = if argv.is_null() {
        Vec::new()
    } else {
        vm_load_until_nul(argv)?
            .into_iter()
            .map(vm_load_string)
            .collect::<Result<Vec<_>, _>>()?
    };
    if args.is_empty() {
        args = vec![String::new()];
    }

    let envs = if envp.is_null() {
        Vec::new()
    } else {
        vm_load_until_nul(envp)?
            .into_iter()
            .map(vm_load_string)
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok((args, envs))
}

fn execve_inner(
    uctx: &mut UserContext,
    path: String,
    args: Vec<String>,
    envs: Vec<String>,
) -> AxResult<isize> {
    // Check for filename too long (execve03 expects ENAMETOOLONG)
    // The filename is the last component of the path
    if path.rsplit('/').next().unwrap_or(&path).len() > 255 {
        return Err(AxError::NameTooLong);
    }

    // Check execute permission before attempting to load the file
    if let Ok(loc) = resolve_at(linux_raw_sys::general::AT_FDCWD, Some(&path), 0)
        .and_then(|it| it.into_file().ok_or(AxError::BadFileDescriptor))
    {
        let metadata = loc.metadata()?;
        if !can_execute(metadata.mode, metadata.uid, metadata.gid) {
            return Err(AxError::PermissionDenied);
        }
    }

    debug!("sys_execve <= path: {path:?}, args: {args:?}, envs: {envs:?}");

    let curr = current();
    let proc_data = &curr.as_thread().proc_data;

    if proc_data.proc.threads().len() > 1 {
        // TODO: handle multi-thread case
        error!("sys_execve: multi-thread not supported");
        return Err(AxError::WouldBlock);
    }

    let mut aspace = proc_data.aspace.lock();
    let (entry_point, user_stack_base) =
        load_user_app(&mut aspace, Some(path.as_str()), &args, &envs)?;
    drop(aspace);

    if let Ok(loc) = resolve_at(linux_raw_sys::general::AT_FDCWD, Some(&path), 0)
        .and_then(|it| it.into_file().ok_or(AxError::BadFileDescriptor))
    {
        curr.set_name(loc.name());
        *proc_data.exe_path.write() = loc.absolute_path()?.to_string();
    } else {
        curr.set_name(path.rsplit('/').next().unwrap_or(&path));
        *proc_data.exe_path.write() = path.to_string();
    }
    crate::perf::perf_begin_process_exec(proc_data.proc.pid() as u64, path.as_str(), &args);
    *proc_data.cmdline.write() = Arc::new(args);
    proc_data.mark_exec();

    proc_data.set_heap_top(USER_HEAP_BASE);

    *proc_data.signal.actions.lock() = Default::default();

    // Clear set_child_tid after exec since the original address is no longer valid
    curr.as_thread().set_clear_child_tid(0);
    // Clear robust_list_head after exec since the original address is no longer valid
    curr.as_thread().set_robust_list_head(0);

    // Close CLOEXEC file descriptors
    let mut fd_table = FD_TABLE.write();
    let cloexec_fds = fd_table
        .ids()
        .filter(|it| fd_table.get(*it).unwrap().cloexec)
        .collect::<Vec<_>>();
    for fd in cloexec_fds {
        fd_table.remove(fd);
    }
    drop(fd_table);

    uctx.set_ip(entry_point.as_usize());
    uctx.set_sp(user_stack_base.as_usize());
    Ok(0)
}

const AT_EMPTY_PATH: usize = linux_raw_sys::general::AT_EMPTY_PATH as usize;
const AT_SYMLINK_NOFOLLOW: usize = linux_raw_sys::general::AT_SYMLINK_NOFOLLOW as usize;
const EXECVEAT_SUPPORTED_FLAGS: usize = AT_EMPTY_PATH | AT_SYMLINK_NOFOLLOW;

pub fn sys_execveat(
    uctx: &mut UserContext,
    dirfd: i32,
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
    flags: usize,
) -> AxResult<isize> {
    if flags & !EXECVEAT_SUPPORTED_FLAGS != 0 {
        return Err(AxError::InvalidInput);
    }

    let path = if path.is_null() {
        None
    } else {
        Some(vm_load_string(path)?)
    };
    let path_ref = path.as_deref();

    let loc = resolve_at(dirfd, path_ref, flags as u32)?
        .into_file()
        .ok_or(AxError::BadFileDescriptor)?;
    let metadata = loc.metadata()?;
    if !can_execute(metadata.mode, metadata.uid, metadata.gid) {
        return Err(AxError::PermissionDenied);
    }

    let path = loc.absolute_path()?.to_string();
    let (args, envs) = load_exec_args_envs(argv, envp)?;
    execve_inner(uctx, path, args, envs)
}
