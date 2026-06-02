use alloc::string::String;
use core::{
    mem,
    sync::atomic::{AtomicBool, Ordering},
};

use axerrno::{AxError, AxResult};
use axhal::uspace::UserContext;
use axtask::{TaskInner, current};
use linux_raw_sys::general::kernel_sigset_t;
use starry_process::{Pid, Process, init_proc};
use starry_signal::{SignalInfo, SignalOSAction, SignalSet};

use super::{
    AsThread, ProcessData, Thread, do_exit, get_process_data, get_process_group, get_task,
};

pub fn ltp_trace_signal_set_bits(set: SignalSet) -> u64 {
    let raw: kernel_sigset_t = set.into();
    unsafe { mem::transmute::<kernel_sigset_t, u64>(raw) }
}

pub fn ltp_trace_proc_enabled(proc_data: &ProcessData) -> bool {
    let exe = proc_data.exe_path.read();
    if exe.contains("abort01") || exe.contains("tst_test") {
        return true;
    }
    drop(exe);

    let cmdline = proc_data.cmdline.read();
    cmdline
        .iter()
        .any(|arg| arg.contains("abort01") || arg.contains("tst_test"))
}

pub fn ltp_trace_current_enabled() -> bool {
    current()
        .try_as_thread()
        .is_some_and(|thr| ltp_trace_proc_enabled(&thr.proc_data))
}

pub fn ltp_trace_proc_label(proc_data: &ProcessData) -> String {
    let exe = proc_data.exe_path.read();
    let mut out = String::new();
    out.push_str("exe=");
    out.push_str(&exe);
    drop(exe);

    out.push_str(" cmd=");
    let cmdline = proc_data.cmdline.read();
    if cmdline.is_empty() {
        out.push_str("<empty>");
    } else {
        for (idx, arg) in cmdline.iter().enumerate() {
            if idx > 0 {
                out.push(' ');
            }
            out.push_str(arg);
        }
    }
    out
}

fn ltp_trace_current_label() -> String {
    current()
        .try_as_thread()
        .map(|thr| ltp_trace_proc_label(&thr.proc_data))
        .unwrap_or_else(|| "kernel-task".into())
}

fn ltp_trace_signal_delivery(
    source: &str,
    target_kind: &str,
    target_pid: Pid,
    target_tid: Option<Pid>,
    target_proc_data: Option<&ProcessData>,
    sig: &SignalInfo,
) {
    let target_matches = target_proc_data.is_some_and(ltp_trace_proc_enabled);
    if !target_matches && !ltp_trace_current_enabled() {
        return;
    }

    let curr = current();
    let curr_tid = curr.id().as_u64() as Pid;
    let curr_pid = curr
        .try_as_thread()
        .map(|thr| thr.proc_data.proc.pid())
        .unwrap_or(0);
    let target_label = target_proc_data
        .map(ltp_trace_proc_label)
        .unwrap_or_else(|| "<unknown>".into());
    warn!(
        "[ltp-sigtrace] source={} target={} target_pid={} target_tid={:?} signal={}({:?}) code={} \
         curr_pid={} curr_tid={} curr={} target={}",
        source,
        target_kind,
        target_pid,
        target_tid,
        sig.signo() as u8,
        sig.signo(),
        sig.code(),
        curr_pid,
        curr_tid,
        ltp_trace_current_label(),
        target_label,
    );
}

pub fn check_signals(
    thr: &Thread,
    uctx: &mut UserContext,
    restore_blocked: Option<SignalSet>,
) -> bool {
    let restart_syscall = thr.restart_syscall();
    let restart_context = restart_syscall.map(|restart| restart.pre_syscall_context);
    let Some((sig, os_action, restarted_syscall)) =
        thr.signal
            .check_signals_with_restart(uctx, restore_blocked, restart_context)
    else {
        return false;
    };
    if let Some(restart) = restart_syscall {
        thr.clear_restart_syscall();
        if ltp_trace_proc_enabled(&thr.proc_data) {
            warn!(
                "[ltp-restart] signal={}({:?}) os_action={:?} sysno={} restarted={} proc={}",
                sig.signo() as u8,
                sig.signo(),
                os_action,
                restart.sysno,
                restarted_syscall,
                ltp_trace_proc_label(&thr.proc_data),
            );
        }
    }

    let signo = sig.signo();
    let action_source = match os_action {
        SignalOSAction::Terminate => "check_signals:Terminate",
        SignalOSAction::CoreDump => "check_signals:CoreDump",
        SignalOSAction::Stop => "check_signals:Stop",
        SignalOSAction::Continue => "check_signals:Continue",
        SignalOSAction::Handler => "check_signals:Handler",
    };
    ltp_trace_signal_delivery(
        action_source,
        "signal-action",
        thr.proc_data.proc.pid(),
        Some(current().id().as_u64() as Pid),
        Some(&thr.proc_data),
        &sig,
    );
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
    source: &'static str,
) {
    ltp_trace_signal_delivery(
        source,
        "thread-inner",
        thr.proc_data.proc.pid(),
        Some(task.id().as_u64() as Pid),
        Some(&thr.proc_data),
        &sig,
    );
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
    source: &'static str,
) -> AxResult<()> {
    let proc_data = match get_process_data(pid) {
        Ok(proc_data) => proc_data,
        Err(AxError::NoSuchProcess) if process_tree_contains(&init_proc(), pid) => return Ok(()),
        Err(err) => return Err(err),
    };

    if let Some(sig) = sig {
        let signo = sig.signo();
        info!("Send signal {signo:?} to process {pid}");
        let trace_sig = sig.clone();
        let target_tid = proc_data.signal.send_signal(sig);
        ltp_trace_signal_delivery(
            source,
            "process",
            pid,
            target_tid.map(|tid| tid as Pid),
            Some(&proc_data),
            &trace_sig,
        );
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
    let trace_sig = sig.clone();
    let target_tid = proc_data.signal.send_signal(sig);
    ltp_trace_signal_delivery(
        source,
        "fatal-current-process",
        proc_data.proc.pid(),
        target_tid.map(|tid| tid as Pid),
        Some(proc_data),
        &trace_sig,
    );
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
