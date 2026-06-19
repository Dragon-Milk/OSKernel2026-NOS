use alloc::vec::Vec;
use core::{future::poll_fn, task::Poll};

use axerrno::{AxError, AxResult, LinuxError};
use axtask::{current, future::block_on};
use bitflags::bitflags;
use linux_raw_sys::general::{
    __WALL, __WCLONE, __WNOTHREAD, CLD_CONTINUED, CLD_DUMPED, CLD_EXITED, CLD_KILLED,
    CLD_STOPPED, P_ALL, P_PGID, P_PID, SIGCHLD, WCONTINUED, WEXITED, WNOHANG, WNOWAIT,
    WSTOPPED, WUNTRACED, rusage, siginfo,
};
use starry_process::{Pid, Process};
use starry_signal::Signo;
use starry_vm::{VmMutPtr, VmPtr};

use crate::task::{AsThread, cleanup_task_tables, get_process_data};

bitflags! {
    #[derive(Debug)]
    struct WaitOptions: u32 {
        /// Do not block when there are no processes wishing to report status.
        const WNOHANG = WNOHANG;
        /// Report the status of selected processes which are stopped due to a
        /// `SIGTTIN`, `SIGTTOU`, `SIGTSTP`, or `SIGSTOP` signal.
        const WUNTRACED = WUNTRACED;
        /// Report the status of selected processes which have terminated.
        const WEXITED = WEXITED;
        /// Report the status of selected processes that have continued from a
        /// job control stop by receiving a `SIGCONT` signal.
        const WCONTINUED = WCONTINUED;
        /// Report stopped children for waitid.
        const WSTOPPED = WSTOPPED;
        /// Don't reap, just poll status.
        const WNOWAIT = WNOWAIT;

        /// Don't wait on children of other threads in this group
        const WNOTHREAD = __WNOTHREAD;
        /// Wait on all children, regardless of type
        const WALL = __WALL;
        /// Wait for "clone" children only.
        const WCLONE = __WCLONE;
    }
}

#[derive(Debug, Clone, Copy)]
enum WaitPid {
    /// Wait for any child process
    Any,
    /// Wait for the child whose process ID is equal to the value.
    Pid(Pid),
    /// Wait for any child process whose process group ID is equal to the value.
    Pgid(Pid),
}

impl WaitPid {
    fn apply(&self, child: &Process) -> bool {
        match self {
            WaitPid::Any => true,
            WaitPid::Pid(pid) => child.pid() == *pid,
            WaitPid::Pgid(pgid) => child.group().pgid() == *pgid,
        }
    }
}

fn wait_should_return_eintr() -> bool {
    let curr = current();
    let thread = curr.as_thread();
    let mut pending = thread.signal.pending() & !thread.signal.blocked();
    pending.remove(Signo::SIGCHLD);
    for signo in 1..=64 {
        let Some(signo) = Signo::from_repr(signo) else {
            continue;
        };
        if pending.has(signo) {
            return !thread.proc_data.signal.can_restart(signo);
        }
    }
    false
}

fn reap_child(child: &Process) {
    current().as_thread().proc_data.add_child_times(1, 1);
    child.free();
    cleanup_task_tables();
}

fn write_empty_rusage(usage: *mut rusage) -> AxResult<()> {
    if let Some(usage) = usage.nullable() {
        let empty: rusage = unsafe { core::mem::zeroed() };
        usage.vm_write(empty)?;
    }
    Ok(())
}

pub fn sys_waitpid(
    pid: i32,
    exit_code: *mut i32,
    options: u32,
    usage: *mut rusage,
) -> AxResult<isize> {
    let options = WaitOptions::from_bits(options).ok_or(AxError::InvalidInput)?;
    info!("sys_waitpid <= pid: {pid:?}, options: {options:?}");
    if pid == i32::MIN {
        return Err(AxError::NoSuchProcess);
    }

    let curr = current();
    let proc_data = &curr.as_thread().proc_data;
    let proc = &proc_data.proc;

    let pid = if pid == -1 {
        WaitPid::Any
    } else if pid == 0 {
        WaitPid::Pgid(proc.group().pgid())
    } else if pid > 0 {
        WaitPid::Pid(pid as _)
    } else {
        WaitPid::Pgid(-pid as _)
    };

    // FIXME: add back support for WALL & WCLONE, since ProcessData may drop before
    // Process now.
    let children = proc
        .children()
        .into_iter()
        .filter(|child| pid.apply(child))
        .collect::<Vec<_>>();
    if children.is_empty() {
        return Err(AxError::from(LinuxError::ECHILD));
    }

    let check_children = || {
        if options.contains(WaitOptions::WUNTRACED) {
            if let Some((child, child_data, signo)) = children.iter().find_map(|child| {
                let child_data = get_process_data(child.pid()).ok()?;
                child_data
                    .child_wait_state()
                    .stopped
                    .map(|signo| (child, child_data, signo))
            }) {
                if let Some(exit_code) = exit_code.nullable() {
                    exit_code.vm_write(((signo as i32) << 8) | 0x7f)?;
                }
                if !options.contains(WaitOptions::WNOWAIT) {
                    child_data.consume_stopped();
                }
                return Ok(Some(child.pid() as _));
            }
        }

        if options.contains(WaitOptions::WCONTINUED) {
            if let Some((child, child_data)) = children.iter().find_map(|child| {
                let child_data = get_process_data(child.pid()).ok()?;
                child_data
                    .child_wait_state()
                    .continued
                    .then_some((child, child_data))
            }) {
                if let Some(exit_code) = exit_code.nullable() {
                    exit_code.vm_write(0xffff)?;
                }
                if !options.contains(WaitOptions::WNOWAIT) {
                    child_data.consume_continued();
                }
                return Ok(Some(child.pid() as _));
            }
        }

        if let Some(child) = children.iter().find(|child| child.is_zombie()) {
            let status = child.exit_code();
            if let Some(exit_code) = exit_code.nullable() {
                exit_code.vm_write(status)?;
            }
            write_empty_rusage(usage)?;
            if !options.contains(WaitOptions::WNOWAIT) {
                reap_child(child);
            }
            Ok(Some(child.pid() as _))
        } else if options.contains(WaitOptions::WNOHANG) {
            Ok(Some(0))
        } else {
            Ok(None)
        }
    };

    block_on(poll_fn(|cx| match check_children().transpose() {
        Some(res) => Poll::Ready(res),
        None => {
            proc_data.child_exit_event.register(cx.waker());
            if let Some(res) = check_children().transpose() {
                return Poll::Ready(res);
            }
            if curr.poll_interrupt(cx).is_ready() {
                match check_children().transpose() {
                    Some(res) => Poll::Ready(res),
                    None if wait_should_return_eintr() => Poll::Ready(Err(AxError::Interrupted)),
                    None => Poll::Pending,
                }
            } else {
                Poll::Pending
            }
        }
    }))
}

fn write_waitid_siginfo(info: *mut siginfo, child: &Process) -> AxResult<()> {
    let mut sig: siginfo = unsafe { core::mem::zeroed() };
    let status = child.exit_code();
    let termsig = status & 0x7f;
    let dumped = status & 0x80 != 0;
    sig.__bindgen_anon_1.__bindgen_anon_1.si_signo = SIGCHLD as _;
    sig.__bindgen_anon_1.__bindgen_anon_1.si_code = if dumped {
        CLD_DUMPED as _
    } else if termsig != 0 {
        CLD_KILLED as _
    } else {
        CLD_EXITED as _
    };
    sig.__bindgen_anon_1
        .__bindgen_anon_1
        ._sifields
        ._sigchld
        ._pid = child.pid() as _;
    sig.__bindgen_anon_1
        .__bindgen_anon_1
        ._sifields
        ._sigchld
        ._status = if termsig != 0 {
        termsig
    } else {
        (status >> 8) & 0xff
    } as _;
    info.vm_write(sig)?;
    Ok(())
}

fn write_waitid_signal_siginfo(
    info: *mut siginfo,
    child: &Process,
    code: u32,
    status: u8,
) -> AxResult<()> {
    let mut sig: siginfo = unsafe { core::mem::zeroed() };
    sig.__bindgen_anon_1.__bindgen_anon_1.si_signo = SIGCHLD as _;
    sig.__bindgen_anon_1.__bindgen_anon_1.si_code = code as _;
    sig.__bindgen_anon_1
        .__bindgen_anon_1
        ._sifields
        ._sigchld
        ._pid = child.pid() as _;
    sig.__bindgen_anon_1
        .__bindgen_anon_1
        ._sifields
        ._sigchld
        ._status = status as _;
    info.vm_write(sig)?;
    Ok(())
}

fn zero_waitid_siginfo(info: *mut siginfo) -> AxResult<()> {
    let sig: siginfo = unsafe { core::mem::zeroed() };
    info.vm_write(sig)?;
    Ok(())
}

pub fn sys_waitid(idtype: u32, id: Pid, info: *mut siginfo, options: u32) -> AxResult<isize> {
    let options = WaitOptions::from_bits(options).ok_or(AxError::InvalidInput)?;
    if info.is_null()
        || !options.intersects(WaitOptions::WEXITED | WaitOptions::WSTOPPED | WaitOptions::WCONTINUED)
    {
        return Err(AxError::InvalidInput);
    }

    let curr = current();
    let proc = &curr.as_thread().proc_data.proc;
    let selector = match idtype {
        P_ALL => WaitPid::Any,
        P_PID => WaitPid::Pid(id),
        P_PGID => WaitPid::Pgid(if id == 0 { proc.group().pgid() } else { id }),
        _ => return Err(AxError::InvalidInput),
    };

    let children = proc
        .children()
        .into_iter()
        .filter(|child| selector.apply(child))
        .collect::<Vec<_>>();
    if children.is_empty() {
        return Err(AxError::from(LinuxError::ECHILD));
    }

    let check_children = || {
        if options.contains(WaitOptions::WSTOPPED) {
            if let Some((child, child_data, signo)) = children.iter().find_map(|child| {
                let child_data = get_process_data(child.pid()).ok()?;
                child_data
                    .child_wait_state()
                    .stopped
                    .map(|signo| (child, child_data, signo))
            }) {
                write_waitid_signal_siginfo(info, child, CLD_STOPPED, signo)?;
                if !options.contains(WaitOptions::WNOWAIT) {
                    child_data.consume_stopped();
                }
                return Ok(Some(0));
            }
        }

        if options.contains(WaitOptions::WCONTINUED) {
            if let Some((child, child_data)) = children.iter().find_map(|child| {
                let child_data = get_process_data(child.pid()).ok()?;
                child_data
                    .child_wait_state()
                    .continued
                    .then_some((child, child_data))
            }) {
                write_waitid_signal_siginfo(info, child, CLD_CONTINUED, Signo::SIGCONT as u8)?;
                if !options.contains(WaitOptions::WNOWAIT) {
                    child_data.consume_continued();
                }
                return Ok(Some(0));
            }
        }

        if options.contains(WaitOptions::WEXITED)
            && let Some(child) = children.iter().find(|child| child.is_zombie())
        {
            write_waitid_siginfo(info, child)?;
            if !options.contains(WaitOptions::WNOWAIT) {
                reap_child(child);
            }
            Ok(Some(0))
        } else if options.contains(WaitOptions::WNOHANG) {
            zero_waitid_siginfo(info)?;
            Ok(Some(0))
        } else {
            Ok(None)
        }
    };

    block_on(poll_fn(|cx| match check_children().transpose() {
        Some(res) => Poll::Ready(res),
        None => {
            curr.as_thread()
                .proc_data
                .child_exit_event
                .register(cx.waker());
            if let Some(res) = check_children().transpose() {
                return Poll::Ready(res);
            }
            if curr.poll_interrupt(cx).is_ready() {
                match check_children().transpose() {
                    Some(res) => Poll::Ready(res),
                    None if wait_should_return_eintr() => Poll::Ready(Err(AxError::Interrupted)),
                    None => Poll::Pending,
                }
            } else {
                Poll::Pending
            }
        }
    }))
}
