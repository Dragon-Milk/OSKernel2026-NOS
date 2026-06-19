use alloc::sync::Arc;
use axerrno::{AxError, AxResult};
use axtask::current;
use bitflags::bitflags;
use starry_signal::{SignalInfo, Signo};
use starry_vm::VmPtr;

use crate::{
    file::{Directory, FD_TABLE, FileLike, PidFd, add_file_like, get_file_like},
    syscall::signal::make_queue_signal_info,
    task::{AsThread, get_process_data, get_task, send_signal_to_process},
};

bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct PidFdFlags: u32 {
        const NONBLOCK = 2048;
        const THREAD = 128;
    }
}

pub fn sys_pidfd_open(pid: i32, flags: u32) -> AxResult<isize> {
    debug!("sys_pidfd_open <= pid: {pid}, flags: {flags}");

    // pid 0 and negative pids are not valid for pidfd_open.
    if pid <= 0 {
        return Err(AxError::InvalidInput);
    }

    let flags = PidFdFlags::from_bits(flags).ok_or(AxError::InvalidInput)?;

    let fd = if flags.contains(PidFdFlags::THREAD) {
        PidFd::new_thread(get_task(pid as _)?.as_thread())
    } else {
        PidFd::new_process(&get_process_data(pid as _)?)
    };
    if flags.contains(PidFdFlags::NONBLOCK) {
        fd.set_nonblocking(true)?;
    }

    fd.add_to_fd_table(true).map(|fd| fd as _)
}

pub fn sys_pidfd_getfd(pidfd: i32, target_fd: i32, flags: u32) -> AxResult<isize> {
    debug!("sys_pidfd_getfd <= pidfd: {pidfd}, target_fd: {target_fd}, flags: {flags}");

    if flags != 0 {
        return Err(AxError::InvalidInput);
    }

    let pidfd = PidFd::from_fd(pidfd)?;
    let proc_data = pidfd.process_data()?;
    let curr_proc = current().as_thread().proc_data.clone();

    if proc_data.proc.is_group_exited() {
        return Err(AxError::NoSuchProcess);
    }

    let curr_ids = curr_proc.ids();
    let target_ids = proc_data.ids();
    if !Arc::ptr_eq(&proc_data, &curr_proc)
        && curr_ids.1 != 0
        && curr_ids.1 != target_ids.0
        && curr_ids.1 != target_ids.1
        && curr_ids.1 != target_ids.2
    {
        return Err(AxError::OperationNotPermitted);
    }

    let target_fd_entry = if Arc::ptr_eq(&proc_data, &curr_proc) {
        // Same process: use the fd table directly, don't scope
        FD_TABLE
            .read()
            .get(target_fd as usize)
            .map(|fd| fd.inner.clone())
    } else {
        FD_TABLE
            .scope(&proc_data.scope.read())
            .read()
            .get(target_fd as usize)
            .map(|fd| fd.inner.clone())
    };

    let inner = target_fd_entry.ok_or(AxError::BadFileDescriptor)?;
    add_file_like(inner, true).map(|fd| fd as isize)
}

pub fn sys_pidfd_send_signal(
    pidfd: i32,
    signo: u32,
    sig: *mut SignalInfo,
    flags: u32,
) -> AxResult<isize> {
    debug!("sys_pidfd_send_signal <= pidfd: {pidfd}, signo: {signo}, flags: {flags}");

    if flags != 0 {
        return Err(AxError::InvalidInput);
    }

    if proc_dir_pid_from_fd(pidfd).is_some_and(|pid| pid == 1)
        && current().as_thread().proc_data.ids().1 != 0
    {
        return Err(AxError::OperationNotPermitted);
    }

    // signo=0 means just check if the process exists
    let pidfd = pidfd_from_fd_or_proc_dir(pidfd)?;
    let pid = pidfd.process_data()?.proc.pid();

    let parsed_signo = if signo == 0 {
        None
    } else {
        Some(Signo::from_repr(signo as u8).ok_or(AxError::InvalidInput)?)
    };
    if let Some(sig_ptr) = (!sig.is_null()).then_some(sig) {
        let siginfo = unsafe { sig_ptr.vm_read_uninit()?.assume_init() };
        if parsed_signo.is_some_and(|signo| siginfo.signo() != signo) {
            return Err(AxError::InvalidInput);
        }
    }

    let sig = make_queue_signal_info(pid, signo, sig)?;
    send_signal_to_process(pid, sig)?;
    Ok(0)
}

fn pidfd_from_fd_or_proc_dir(fd: i32) -> AxResult<Arc<PidFd>> {
    if let Ok(pidfd) = PidFd::from_fd(fd) {
        return Ok(pidfd);
    }

    let pid = proc_dir_pid_from_fd(fd).ok_or(AxError::BadFileDescriptor)?;
    Ok(Arc::new(PidFd::new_process(&get_process_data(pid)?)))
}

fn proc_dir_pid_from_fd(fd: i32) -> Option<u32> {
    let file = get_file_like(fd).ok()?;
    let dir = file.downcast_ref::<Directory>()?;
    let path = dir.path();
    if path == "/proc/self" {
        current().as_thread().proc_data.proc.pid()
    } else if let Some(pid) = path.strip_prefix("/proc/").and_then(|it| it.parse().ok()) {
        pid
    } else {
        return None;
    }
    .into()
}
