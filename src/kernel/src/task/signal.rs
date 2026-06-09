use core::sync::atomic::{AtomicBool, Ordering};

use axerrno::{AxError, AxResult};
use axhal::uspace::UserContext;
use axtask::{current, TaskInner};
use starry_process::{init_proc, Pid, Process};
use starry_signal::{SignalInfo, SignalOSAction, SignalSet};

use super::{
    AsThread, Thread, do_exit, get_process_data, get_process_group, get_task,
};

pub fn check_signals(
    thr: &Thread,
    uctx: &mut UserContext,
    restore_blocked: Option<SignalSet>,
) -> bool {
    let restart_syscall = thr.restart_syscall();
    let restart_context = restart_syscall.map(|restart| restart.pre_syscall_context);
    let Some((sig, os_action, _restarted_syscall)) =
        thr.signal
            .check_signals_with_restart(uctx, restore_blocked, restart_context)
    else {
        return false;
    };
    if restart_syscall.is_some() {
        thr.clear_restart_syscall();
    }

    let signo = sig.signo();
    match os_action {
        SignalOSAction::Terminate => {
            do_exit(signo as i32, true);
        }
        SignalOSAction::CoreDump => {
            // TODO: implement core dump
            do_exit(128 + signo as i32, true);
        }
        SignalOSAction::Stop => {
            // TODO: implement stop
            do_exit(1, true);
        }
        SignalOSAction::Continue => {
            // TODO: implement continue
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

pub(super) fn send_signal_thread_inner_with_source(
    task: &TaskInner,
    thr: &Thread,
    sig: SignalInfo,
    _source: &'static str,
) {
    if thr.signal.send_signal(sig) {
        task.interrupt();
    }
}

#[allow(dead_code)]
pub(super) fn send_signal_thread_inner(task: &TaskInner, thr: &Thread, sig: SignalInfo) {
    send_signal_thread_inner_with_source(task, thr, sig, "send_signal_thread_inner");
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
#[allow(dead_code)]
pub fn send_signal_to_thread(tgid: Option<Pid>, tid: Pid, sig: Option<SignalInfo>) -> AxResult<()> {
    send_signal_to_thread_with_source(tgid, tid, sig, "send_signal_to_thread")
}

pub fn send_signal_to_thread_with_source(
    tgid: Option<Pid>,
    tid: Pid,
    sig: Option<SignalInfo>,
    source: &'static str,
) -> AxResult<()> {
    let task = get_task(tid)?;
    let thread = task.try_as_thread().ok_or(AxError::OperationNotPermitted)?;
    if tgid.is_some_and(|tgid| thread.proc_data.proc.pid() != tgid) {
        return Err(AxError::NoSuchProcess);
    }

    if let Some(sig) = sig {
        info!("Send signal {:?} to thread {}", sig.signo(), tid);
        send_signal_thread_inner_with_source(&task, thread, sig, source);
    }

    Ok(())
}

/// Sends a signal to a process.
pub fn send_signal_to_process(pid: Pid, sig: Option<SignalInfo>) -> AxResult<()> {
    send_signal_to_process_with_source(pid, sig, "send_signal_to_process")
}

pub fn send_signal_to_process_with_source(
    pid: Pid,
    sig: Option<SignalInfo>,
    _source: &'static str,
) -> AxResult<()> {
    let proc_data = match get_process_data(pid) {
        Ok(proc_data) => proc_data,
        Err(AxError::NoSuchProcess) if process_tree_contains(&init_proc(), pid) => return Ok(()),
        Err(err) => return Err(err),
    };

    if let Some(sig) = sig {
        let signo = sig.signo();
        info!("Send signal {signo:?} to process {pid}");
        let target_tid = proc_data.signal.send_signal(sig);
        if let Some(tid) = target_tid
            && let Ok(task) = get_task(tid)
        {
            task.interrupt();
        }
    }

    Ok(())
}

/// Sends a signal to a process group.
pub fn send_signal_to_process_group(pgid: Pid, sig: Option<SignalInfo>) -> AxResult<()> {
    send_signal_to_process_group_with_source(pgid, sig, "send_signal_to_process_group")
}

pub fn send_signal_to_process_group_with_source(
    pgid: Pid,
    sig: Option<SignalInfo>,
    source: &'static str,
) -> AxResult<()> {
    let pg = get_process_group(pgid)?;

    if let Some(sig) = sig {
        info!("Send signal {:?} to process group {}", sig.signo(), pgid);
        for proc in pg.processes() {
            send_signal_to_process_with_source(proc.pid(), Some(sig.clone()), source)?;
        }
    }

    Ok(())
}

/// Sends a fatal signal to the current process.
#[allow(dead_code)]
pub fn raise_signal_fatal(sig: SignalInfo) -> AxResult<()> {
    raise_signal_fatal_with_source(sig, "raise_signal_fatal")
}

pub fn raise_signal_fatal_with_source(sig: SignalInfo, source: &'static str) -> AxResult<()> {
    let curr = current();
    let proc_data = &curr.as_thread().proc_data;

    let signo = sig.signo();
    info!("Send fatal signal {signo:?} to the current process");
    let target_tid = proc_data.signal.send_signal(sig);
    if let Some(tid) = target_tid
        && let Ok(task) = get_task(tid)
    {
        task.interrupt();
    } else {
        // No task wants to handle the signal, abort the task
        do_exit(signo as i32, true);
    }

    Ok(())
}

pub fn raise_signal_current_thread_with_source(
    sig: SignalInfo,
    _source: &'static str,
) -> AxResult<()> {
    let curr = current();
    let thr = curr.as_thread();
    let proc_data = &thr.proc_data;

    let signo = sig.signo();
    info!("Send fatal signal {signo:?} to the current thread");
    let should_interrupt = thr.signal.send_signal(sig);
    if should_interrupt {
        curr.interrupt();
    } else {
        do_exit(signo as i32, true);
    }

    Ok(())
}
