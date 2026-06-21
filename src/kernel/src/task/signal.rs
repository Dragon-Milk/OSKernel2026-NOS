use core::{
    future::poll_fn,
    sync::atomic::{AtomicBool, Ordering},
    task::Poll,
};

use axerrno::{AxError, AxResult};
use axhal::uspace::UserContext;
use axtask::{current, future::block_on, TaskInner};
use starry_process::{init_proc, Pid, Process};
use starry_signal::{SignalInfo, SignalOSAction, SignalSet};

use super::{do_exit, get_process_data, get_process_group, get_task, AsThread, Thread};

fn signal_exit_status(signo: starry_signal::Signo, core_dump: bool) -> i32 {
    let mut status = signo as i32;
    if core_dump {
        status |= 0x80;
    }
    status
}

pub fn check_signals(
    thr: &Thread,
    uctx: &mut UserContext,
    restore_blocked: Option<SignalSet>,
) -> bool {
    let Some((sig, os_action)) = thr.signal.check_signals(uctx, restore_blocked) else {
        return false;
    };

    let signo = sig.signo();
    match os_action {
        SignalOSAction::Terminate => {
            do_exit(signal_exit_status(signo, false), true);
        }
        SignalOSAction::CoreDump => {
            // TODO: implement core dump
            do_exit(signal_exit_status(signo, true), true);
        }
        SignalOSAction::Stop => {
            thr.proc_data.mark_stopped(signo);
            if let Some(parent) = thr.proc_data.proc.parent()
                && let Ok(data) = get_process_data(parent.pid())
            {
                data.child_exit_event.wake();
            }
            block_on(poll_fn(|cx| {
                if thr.proc_data.child_wait_state().stopped.is_none() {
                    Poll::Ready(())
                } else {
                    thr.proc_data.stopped_event.register(cx.waker());
                    Poll::Pending
                }
            }));
        }
        SignalOSAction::Continue => {
            thr.proc_data.mark_continued();
            if let Some(parent) = thr.proc_data.proc.parent()
                && let Ok(data) = get_process_data(parent.pid())
            {
                data.child_exit_event.wake();
            }
        }
        SignalOSAction::Handler => {
            // do nothing
        }
    }
    true
}

static BLOCK_NEXT_SIGNAL_CHECK: AtomicBool = AtomicBool::new(false);

pub fn block_next_signal() {
    BLOCK_NEXT_SIGNAL_CHECK.store(true, Ordering::SeqCst);
}

pub fn unblock_next_signal() -> bool {
    BLOCK_NEXT_SIGNAL_CHECK.swap(false, Ordering::SeqCst)
}

pub fn with_blocked_signals<R>(
    blocked: Option<SignalSet>,
    f: impl FnOnce() -> AxResult<R>,
) -> AxResult<R> {
    let curr = current();
    let sig = &curr.as_thread().signal;

    let old_blocked = blocked.map(|set| sig.set_blocked(set));
    f().inspect(|_| {
        if let Some(old) = old_blocked {
            sig.set_blocked(old);
        }
    })
}

pub(super) fn send_signal_thread_inner(task: &TaskInner, thr: &Thread, sig: SignalInfo) {
    if thr.signal.send_signal(sig) {
        task.interrupt();
    }
}

fn process_tree_contains(proc: &Process, pid: Pid) -> bool {
    if proc.pid() == pid {
        return true;
    }
    proc.children()
        .iter()
        .any(|child| process_tree_contains(child, pid))
}

/// Sends a signal to a thread.
pub fn send_signal_to_thread(tgid: Option<Pid>, tid: Pid, sig: Option<SignalInfo>) -> AxResult<()> {
    let task = get_task(tid)?;
    let thread = task.try_as_thread().ok_or(AxError::OperationNotPermitted)?;
    if tgid.is_some_and(|tgid| thread.proc_data.proc.pid() != tgid) {
        return Err(AxError::NoSuchProcess);
    }

    if let Some(sig) = sig {
        info!("Send signal {:?} to thread {}", sig.signo(), tid);
        send_signal_thread_inner(&task, thread, sig);
    }

    Ok(())
}

/// Sends a signal to a process.
pub fn send_signal_to_process(pid: Pid, sig: Option<SignalInfo>) -> AxResult<()> {
    let proc_data = match get_process_data(pid) {
        Ok(proc_data) => proc_data,
        Err(AxError::NoSuchProcess) if process_tree_contains(&init_proc(), pid) => return Ok(()),
        Err(err) => return Err(err),
    };

    if let Some(sig) = sig {
        let signo = sig.signo();
        info!("Send signal {signo:?} to process {pid}");
        if signo == starry_signal::Signo::SIGCONT {
            proc_data.mark_continued();
            if let Some(parent) = proc_data.proc.parent()
                && let Ok(data) = get_process_data(parent.pid())
            {
                data.child_exit_event.wake();
            }
        }
        if let Some(tid) = proc_data.signal.send_signal(sig)
            && let Ok(task) = get_task(tid)
        {
            task.interrupt();
        }
    }

    Ok(())
}

/// Sends a signal to a process group.
pub fn send_signal_to_process_group(pgid: Pid, sig: Option<SignalInfo>) -> AxResult<()> {
    let pg = get_process_group(pgid)?;

    if let Some(sig) = sig {
        info!("Send signal {:?} to process group {}", sig.signo(), pgid);
        for proc in pg.processes() {
            send_signal_to_process(proc.pid(), Some(sig.clone()))?;
        }
    }

    Ok(())
}

/// Sends a fatal signal to the current process.
pub fn raise_signal_fatal(sig: SignalInfo) -> AxResult<()> {
    let curr = current();
    let proc_data = &curr.as_thread().proc_data;

    let signo = sig.signo();
    info!("Send fatal signal {signo:?} to the current process");
    if let Some(tid) = proc_data.signal.send_signal(sig)
        && let Ok(task) = get_task(tid)
    {
        task.interrupt();
    } else {
        // No task wants to handle the signal, abort the task
        do_exit(signal_exit_status(signo, false), true);
    }

    Ok(())
}
