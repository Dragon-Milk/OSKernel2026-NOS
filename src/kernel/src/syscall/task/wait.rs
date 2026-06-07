use alloc::vec::Vec;
use core::{future::poll_fn, task::Poll};

use axerrno::{AxError, AxResult, LinuxError};
use axtask::{current, future::block_on};
use bitflags::bitflags;
use linux_raw_sys::general::{
    __WALL, __WCLONE, __WNOTHREAD, WCONTINUED, WEXITED, WNOHANG, WNOWAIT, WUNTRACED,
};
use starry_process::{Pid, Process};
use starry_signal::Signo;
use starry_vm::{VmMutPtr, VmPtr};

use crate::task::{
    AsThread, ltp_trace_current_enabled, ltp_trace_proc_label, ltp_trace_signal_set_bits,
};

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

pub fn sys_waitpid(pid: i32, exit_code: *mut i32, options: u32) -> AxResult<isize> {
    let options = WaitOptions::from_bits_truncate(options);
    let raw_pid = pid;
    info!("sys_waitpid <= pid: {pid:?}, options: {options:?}");

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
        if let Some(child) = children.iter().find(|child| child.is_zombie()) {
            let status = child.exit_code();
            if let Some(exit_code) = exit_code.nullable() {
                exit_code.vm_write(status)?;
            }
            if !options.contains(WaitOptions::WNOWAIT) {
                child.free();
            }
            Ok(Some(child.pid() as _))
        } else if options.contains(WaitOptions::WNOHANG) {
            Ok(Some(0))
        } else {
            Ok(None)
        }
    };

    let wait_eintr_signals = || {
        let thread = curr.as_thread();
        let pending = thread.signal.pending();
        let blocked = thread.signal.blocked();
        let deliverable = pending & !blocked;
        let mut no_sigchld = deliverable;
        no_sigchld.remove(Signo::SIGCHLD);
        (pending, blocked, deliverable, no_sigchld)
    };

    block_on(poll_fn(|cx| match check_children().transpose() {
        Some(res) => Poll::Ready(res),
        None => {
            proc_data.child_exit_event.register(cx.waker());
            if curr.poll_interrupt(cx).is_ready() {
                match check_children().transpose() {
                    Some(res) => Poll::Ready(res),
                    None => {
                        let (pending, blocked, deliverable, no_sigchld) = wait_eintr_signals();
                        if no_sigchld.is_empty() {
                            Poll::Pending
                        } else {
                            if ltp_trace_current_enabled() {
                                let thread = curr.as_thread();
                                debug!(
                                    "[ltp-wait-eintr] curr_pid={} curr_tid={} wait_raw_pid={} \
                                     wait={:?} options={:?} pending_bits={:#018x} pending={:?} \
                                     blocked_bits={:#018x} blocked={:?} deliverable_bits={:#018x} \
                                     deliverable={:?} no_sigchld_bits={:#018x} no_sigchld={:?} \
                                     proc={}",
                                    proc.pid(),
                                    curr.id().as_u64(),
                                    raw_pid,
                                    pid,
                                    options,
                                    ltp_trace_signal_set_bits(pending),
                                    pending,
                                    ltp_trace_signal_set_bits(blocked),
                                    blocked,
                                    ltp_trace_signal_set_bits(deliverable),
                                    deliverable,
                                    ltp_trace_signal_set_bits(no_sigchld),
                                    no_sigchld,
                                    ltp_trace_proc_label(&thread.proc_data),
                                );
                            }
                            Poll::Ready(Err(AxError::Interrupted))
                        }
                    }
                }
            } else {
                Poll::Pending
            }
        }
    }))
}
