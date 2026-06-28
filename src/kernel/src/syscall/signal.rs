use core::{future::poll_fn, task::Poll};

use axerrno::{AxError, AxResult, LinuxError};
use axhal::uspace::UserContext;
use axtask::{
    current,
    future::{self, block_on},
};
use linux_raw_sys::general::{
    MINSIGSTKSZ, RLIMIT_SIGPENDING, SI_TKILL, SI_USER, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK,
    kernel_sigaction, siginfo, timespec, SS_DISABLE, SS_ONSTACK,
};
use starry_process::Pid;
use starry_signal::{SignalDisposition, SignalInfo, SignalSet, SignalStack, Signo};
use starry_vm::{VmMutPtr, VmPtr};

use crate::{
    task::{
        AsThread, block_next_signal, check_signals, get_process_data, get_task, processes,
        do_exit, send_signal_to_process, send_signal_to_thread,
    },
    time::TimeValueLike,
};

pub(crate) fn check_sigset_size(size: usize) -> AxResult<()> {
    if size != size_of::<SignalSet>() && size != 0 {
        return Err(AxError::InvalidInput);
    }
    Ok(())
}

fn parse_signo(signo: u32) -> AxResult<Signo> {
    Signo::from_repr(signo as u8).ok_or(AxError::InvalidInput)
}

pub fn sys_rt_sigprocmask(
    how: i32,
    set: *const SignalSet,
    oldset: *mut SignalSet,
    sigsetsize: usize,
) -> AxResult<isize> {
    check_sigset_size(sigsetsize)?;

    let curr = current();
    let sig = &curr.as_thread().signal;
    let old = sig.blocked();

    if let Some(oldset) = oldset.nullable() {
        oldset.vm_write(old)?;
    }

    if let Some(set) = set.nullable() {
        let set = unsafe { set.vm_read_uninit()?.assume_init() };

        let set = match how as u32 {
            SIG_BLOCK => old | set,
            SIG_UNBLOCK => old & !set,
            SIG_SETMASK => set,
            _ => return Err(AxError::InvalidInput),
        };

        debug!("sys_rt_sigprocmask <= {set:?}");
        sig.set_blocked(set);
    }

    Ok(0)
}

pub fn sys_rt_sigaction(
    signo: u32,
    act: *const kernel_sigaction,
    oldact: *mut kernel_sigaction,
    sigsetsize: usize,
) -> AxResult<isize> {
    let Some(signo) = Signo::from_repr(signo as u8) else {
        warn!("sys_rt_sigaction: invalid signo {signo}");
        return Err(AxError::InvalidInput);
    };
    if matches!(signo, Signo::SIGKILL | Signo::SIGSTOP) {
        warn!("sys_rt_sigaction: cannot change action for {signo:?}");
        return Err(AxError::InvalidInput);
    }
    check_sigset_size(sigsetsize)?;

    let curr = current();
    let mut actions = curr.as_thread().proc_data.signal.actions.lock();
    if let Some(oldact) = oldact.nullable() {
        oldact.vm_write(actions[signo].clone().into())?;
    }
    if let Some(act) = act.nullable() {
        let act = unsafe { act.vm_read_uninit()?.assume_init() }.into();
        debug!("sys_rt_sigaction <= signo: {signo:?}, act: {act:?}");
        actions[signo] = act;
    }
    Ok(0)
}

pub fn sys_rt_sigpending(set: *mut SignalSet, sigsetsize: usize) -> AxResult<isize> {
    check_sigset_size(sigsetsize)?;
    set.vm_write(current().as_thread().signal.pending())?;
    Ok(0)
}

fn make_siginfo(signo: u32, code: i32) -> AxResult<Option<SignalInfo>> {
    if signo == 0 {
        return Ok(None);
    }
    let signo = parse_signo(signo)?;
    Ok(Some(SignalInfo::new_user(
        signo,
        code,
        current().as_thread().proc_data.proc.pid(),
    )))
}

fn is_default_sigabrt_for_current(tgid: Pid, tid: Pid, signo: Signo) -> bool {
    if signo != Signo::SIGABRT {
        return false;
    }

    let curr = current();
    if curr.as_thread().proc_data.proc.pid() != tgid || (curr.id().as_u64() as Pid) != tid {
        return false;
    }

    matches!(
        &curr.as_thread().proc_data.signal.actions.lock()[Signo::SIGABRT].disposition,
        SignalDisposition::Default
    )
}

fn abort_exit_status() -> i32 {
    (Signo::SIGABRT as i32) | 0x80
}

fn check_kill_permission(pid: Pid, sig: &Option<SignalInfo>) -> AxResult<()> {
    if sig.is_none() {
        return Ok(());
    }

    let curr = current();
    let curr_ids = curr.as_thread().proc_data.ids();
    if curr_ids.1 == 0 {
        return Ok(());
    }

    let target = get_process_data(pid)?;
    let target_ids = target.ids();
    if curr_ids.0 == target_ids.0
        || curr_ids.0 == target_ids.2
        || curr_ids.1 == target_ids.0
        || curr_ids.1 == target_ids.2
    {
        Ok(())
    } else {
        Err(AxError::OperationNotPermitted)
    }
}

pub fn sys_kill(pid: i32, signo: u32) -> AxResult<isize> {
    warn!("[timer2-debug] sys_kill enter pid={pid} signo={signo}");
    let result = do_sys_kill(pid, signo);
    warn!("[timer2-debug] sys_kill return pid={pid} signo={signo} result={result:?}");
    result
}

fn do_sys_kill(pid: i32, signo: u32) -> AxResult<isize> {
    debug!("sys_kill: pid = {pid}, signo = {signo}");
    let sig = make_siginfo(signo, SI_USER as _)?;

    match pid {
        1.. => {
            check_kill_permission(pid as _, &sig)?;
            send_signal_to_process(pid as _, sig)?;
        }
        0 => {
            let pgid = current().as_thread().proc_data.proc.group().pgid();
            let pg = crate::task::get_process_group(pgid)?;
            for proc in pg.processes() {
                if check_kill_permission(proc.pid(), &sig).is_err() {
                    continue;
                }
                let _ = send_signal_to_process(proc.pid(), sig.clone());
            }
        }
        -1 => {
            let curr_pid = current().as_thread().proc_data.proc.pid();
            if let Some(sig) = sig {
                for proc_data in processes() {
                    // POSIX.1 requires that kill(-1,sig) send sig to all processes that
                    //    the calling process may send signals to, except possibly for some
                    //    implementation-defined system processes.  Linux allows a process
                    //    to signal itself, but on Linux the call kill(-1,sig) does not
                    //    signal the calling process.
                    if proc_data.proc.is_init() || proc_data.proc.pid() == curr_pid {
                        continue;
                    }
                    if check_kill_permission(proc_data.proc.pid(), &Some(sig.clone())).is_err() {
                        continue;
                    }
                    let _ = send_signal_to_process(proc_data.proc.pid(), Some(sig.clone()));
                }
            }
        }
        ..-1 => {
            let pg = crate::task::get_process_group((-pid) as Pid)?;
            let mut delivered = false;
            for proc in pg.processes() {
                if check_kill_permission(proc.pid(), &sig).is_err() {
                    continue;
                }
                if send_signal_to_process(proc.pid(), sig.clone()).is_ok() {
                    delivered = true;
                }
            }
            if !delivered {
                return Err(AxError::NoSuchProcess);
            }
        }
    }
    Ok(0)
}

pub fn sys_tkill(tid: Pid, signo: u32) -> AxResult<isize> {
    warn!("[timer2-debug] sys_tkill enter tid={tid} signo={signo}");
    let result = do_sys_tkill(tid, signo);
    warn!("[timer2-debug] sys_tkill return tid={tid} signo={signo} result={result:?}");
    result
}

fn do_sys_tkill(tid: Pid, signo: u32) -> AxResult<isize> {
    if tid == Pid::MAX {
        return Err(AxError::InvalidInput);
    }
    let signo = parse_signo(signo)?;
    let curr_pid = current().as_thread().proc_data.proc.pid();
    if is_default_sigabrt_for_current(curr_pid, tid, signo) {
        do_exit(abort_exit_status(), true);
        return Ok(0);
    }
    let sig = Some(SignalInfo::new_user(signo, SI_TKILL, curr_pid));
    send_signal_to_thread(None, tid, sig)?;
    Ok(0)
}

pub fn sys_tgkill(tgid: i32, tid: i32, signo: u32) -> AxResult<isize> {
    warn!("[timer2-debug] sys_tgkill enter tgid={tgid} tid={tid} signo={signo}");
    let result = do_sys_tgkill(tgid, tid, signo);
    warn!(
        "[timer2-debug] sys_tgkill return tgid={tgid} tid={tid} signo={signo} result={result:?}"
    );
    result
}

fn do_sys_tgkill(tgid: i32, tid: i32, signo: u32) -> AxResult<isize> {
    if tgid <= 0 || tid <= 0 {
        return Err(AxError::InvalidInput);
    }
    let tgid = tgid as Pid;
    let tid = tid as Pid;
    let signo = parse_signo(signo)?;
    if is_default_sigabrt_for_current(tgid, tid, signo) {
        do_exit(abort_exit_status(), true);
        return Ok(0);
    }
    let sig = Some(SignalInfo::new_user(
        signo,
        SI_TKILL,
        current().as_thread().proc_data.proc.pid(),
    ));
    if let Some(sig) = &sig {
        if sig.signo().is_realtime() {
            let target = get_task(tid)?;
            let thread = target
                .try_as_thread()
                .ok_or(AxError::OperationNotPermitted)?;
            if tgid != 0 && thread.proc_data.proc.pid() != tgid {
                return Err(AxError::NoSuchProcess);
            }
            if thread.proc_data.rlim.read()[RLIMIT_SIGPENDING].current == 0 {
                return Err(AxError::WouldBlock);
            }
        }
    }
    send_signal_to_thread(Some(tgid), tid, sig)?;
    Ok(0)
}

pub(crate) fn make_queue_signal_info(
    tgid: Pid,
    signo: u32,
    sig: *const SignalInfo,
) -> AxResult<Option<SignalInfo>> {
    if signo == 0 {
        return Ok(None);
    }

    let signo = parse_signo(signo)?;
    let mut sig = unsafe { sig.vm_read_uninit()?.assume_init() };
    sig.set_signo(signo);
    if current().as_thread().proc_data.proc.pid() != tgid
        && (sig.code() >= 0 || sig.code() == SI_TKILL)
    {
        return Err(AxError::OperationNotPermitted);
    }
    Ok(Some(sig))
}

pub fn sys_rt_sigqueueinfo(
    tgid: Pid,
    signo: u32,
    sig: *const SignalInfo,
    sigsetsize: usize,
) -> AxResult<isize> {
    check_sigset_size(sigsetsize)?;

    let sig = make_queue_signal_info(tgid, signo, sig)?;
    let curr_pid = current().as_thread().proc_data.proc.pid();
    if let Ok(task) = get_task(tgid)
        && task
            .try_as_thread()
            .is_some_and(|thread| thread.proc_data.proc.pid() == curr_pid)
    {
        send_signal_to_thread(Some(curr_pid), tgid, sig)?;
    } else {
        send_signal_to_process(tgid, sig)?;
    }
    Ok(0)
}

pub fn sys_rt_tgsigqueueinfo(
    tgid: Pid,
    tid: Pid,
    signo: u32,
    sig: *const SignalInfo,
    sigsetsize: usize,
) -> AxResult<isize> {
    check_sigset_size(sigsetsize)?;

    let sig = make_queue_signal_info(tgid, signo, sig)?;
    send_signal_to_thread(Some(tgid), tid, sig)?;
    Ok(0)
}

pub fn sys_rt_sigreturn(uctx: &mut UserContext) -> AxResult<isize> {
    block_next_signal();
    current().as_thread().signal.restore(uctx);
    Ok(uctx.retval() as isize)
}

pub fn sys_rt_sigtimedwait(
    uctx: &mut UserContext,
    set: *const SignalSet,
    info: *mut siginfo,
    timeout: *const timespec,
    sigsetsize: usize,
) -> AxResult<isize> {
    check_sigset_size(sigsetsize)?;

    let set = unsafe { set.vm_read_uninit()?.assume_init() };

    let timeout = if let Some(ts) = timeout.nullable() {
        let ts = unsafe { ts.vm_read_uninit()?.assume_init() };
        Some(ts.try_into_time_value()?)
    } else {
        None
    };

    debug!("sys_rt_sigtimedwait => set = {set:?}, timeout = {timeout:?}");

    let curr = current();
    let thr = curr.as_thread();
    let signal = &thr.signal;

    let old_blocked = signal.blocked();

    uctx.set_retval(-LinuxError::EINTR.code() as usize);
    let fut = poll_fn(|cx| {
        if let Some(sig) = signal.dequeue_signal(&set) {
            Poll::Ready(Some(sig))
        } else if check_signals(thr, uctx, Some(old_blocked)) {
            Poll::Ready(None)
        } else {
            let _ = curr.poll_interrupt(cx);
            Poll::Pending
        }
    });

    let Ok(sig) = block_on(future::timeout(timeout, fut)) else {
        // Timeout
        return Err(AxError::WouldBlock);
    };
    let Some(sig) = sig else {
        // Interrupted
        return Ok(0);
    };

    if let Some(info) = info.nullable() {
        info.vm_write(sig.0)?;
    }

    Ok(sig.signo() as _)
}

pub fn sys_rt_sigsuspend(
    uctx: &mut UserContext,
    set: *const SignalSet,
    sigsetsize: usize,
) -> AxResult<isize> {
    check_sigset_size(sigsetsize)?;

    let curr = current();
    let thr = curr.as_thread();

    let set = unsafe { set.vm_read_uninit()?.assume_init() };
    let old_blocked = thr.signal.set_blocked(set);

    // sigsuspend always returns -EINTR when a signal is caught
    // We set this in uctx before check_signals so it's saved in SignalFrame
    uctx.set_retval(-LinuxError::EINTR.code() as usize);

    block_on(poll_fn(|cx| {
        if check_signals(thr, uctx, Some(old_blocked)) {
            return Poll::Ready(());
        }
        let _ = curr.poll_interrupt(cx);
        Poll::Pending
    }));

    thr.signal.set_blocked(old_blocked);
    // sigsuspend always returns -EINTR
    Err(AxError::Interrupted)
}

pub fn sys_sigaltstack(ss: *const SignalStack, old_ss: *mut SignalStack) -> AxResult<isize> {
    let curr = current();
    let sig = &curr.as_thread().signal;

    if let Some(old_ss) = old_ss.nullable() {
        old_ss.vm_write(sig.stack())?;
    }

    if let Some(ss) = ss.nullable() {
        let ss = unsafe { ss.vm_read_uninit()?.assume_init() };
        if ss.flags & !(SS_DISABLE | SS_ONSTACK) != 0 {
            return Err(AxError::InvalidInput);
        }
        if ss.size <= MINSIGSTKSZ as usize {
            return Err(AxError::NoMemory);
        }
        sig.set_stack(ss);
    }
    Ok(0)
}
