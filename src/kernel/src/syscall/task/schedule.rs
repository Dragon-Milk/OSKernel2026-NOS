use core::mem::size_of;

use axerrno::{AxError, AxResult};
use axhal::time::TimeValue;
use axtask::{
    current,
    future::{block_on, interruptible, sleep},
    AxCpuMask, AxTaskRef,
};
use linux_raw_sys::general::{
    __kernel_clockid_t, timespec, CLOCK_MONOTONIC, CLOCK_REALTIME, PRIO_PGRP, PRIO_PROCESS,
    PRIO_USER, SCHED_BATCH, SCHED_DEADLINE, SCHED_FIFO, SCHED_IDLE, SCHED_NORMAL, SCHED_RR,
    TIMER_ABSTIME,
};
use starry_vm::{vm_load, vm_write_slice, VmMutPtr, VmPtr};

use crate::{
    syscall::time::realtime_time,
    task::{get_process_group, get_process_task, get_task, AsThread},
    time::TimeValueLike,
};

fn priority_syscall_value(nice: i32) -> isize {
    (20 - nice.clamp(-20, 19)) as isize
}

pub fn sys_sched_yield() -> AxResult<isize> {
    axtask::yield_now();
    Ok(0)
}

fn sleep_impl(clock: impl Fn() -> TimeValue, dur: TimeValue) -> (TimeValue, bool) {
    debug!("sleep_impl <= {dur:?}");

    let start = clock();

    if dur.as_nanos() == 0 {
        return (TimeValue::from_nanos(0), false);
    }

    let interrupted = block_on(interruptible(sleep(dur))).is_err();

    (clock().saturating_sub(start), interrupted)
}

/// Sleep some nanoseconds
pub fn sys_nanosleep(req: *const timespec, rem: *mut timespec) -> AxResult<isize> {
    // FIXME: AnyBitPattern
    let req = unsafe { req.vm_read_uninit()?.assume_init() }.try_into_time_value()?;
    debug!("sys_nanosleep <= req: {req:?}");

    let (actual, interrupted) = sleep_impl(axhal::time::monotonic_time, req);

    if interrupted
        && let Some(diff) = req.checked_sub(actual)
    {
        debug!("sys_nanosleep => rem: {diff:?}");
        if let Some(rem) = rem.nullable() {
            rem.vm_write(timespec::from_time_value(diff))?;
        }
        Err(AxError::Interrupted)
    } else {
        Ok(0)
    }
}

pub fn sys_clock_nanosleep(
    clock_id: __kernel_clockid_t,
    flags: u32,
    req: *const timespec,
    rem: *mut timespec,
) -> AxResult<isize> {
    let clock = match clock_id as u32 {
        CLOCK_REALTIME => realtime_time,
        CLOCK_MONOTONIC => axhal::time::monotonic_time,
        _ => {
            warn!("Unsupported clock_id: {clock_id}");
            return Err(AxError::OperationNotSupported);
        }
    };

    let req = unsafe { req.vm_read_uninit()?.assume_init() }.try_into_time_value()?;
    debug!("sys_clock_nanosleep <= clock_id: {clock_id}, flags: {flags}, req: {req:?}");

    let dur = if flags & TIMER_ABSTIME != 0 {
        req.saturating_sub(clock())
    } else {
        req
    };

    let (actual, interrupted) = sleep_impl(clock, dur);

    if interrupted
        && let Some(diff) = dur.checked_sub(actual)
    {
        debug!("sys_clock_nanosleep => rem: {diff:?}");
        if let Some(rem) = rem.nullable() {
            rem.vm_write(timespec::from_time_value(diff))?;
        }
        Err(AxError::Interrupted)
    } else {
        Ok(0)
    }
}

pub fn sys_sched_getaffinity(pid: i32, cpusetsize: usize, user_mask: *mut u8) -> AxResult<isize> {
    if pid < 0 {
        return Err(AxError::InvalidInput);
    }
    let task = sched_task(pid)?;

    let mask = task.cpumask();
    let mask_bytes = mask.as_bytes();
    if cpusetsize < mask_bytes.len() {
        return Err(AxError::InvalidInput);
    }

    vm_write_slice(user_mask, mask_bytes)?;

    Ok(mask_bytes.len() as _)
}

pub fn sys_sched_setaffinity(
    pid: i32,
    cpusetsize: usize,
    user_mask: *const u8,
) -> AxResult<isize> {
    if pid < 0 {
        return Err(AxError::InvalidInput);
    }
    let size = cpusetsize.min(axhal::cpu_num().div_ceil(8));
    let user_mask = vm_load(user_mask, size)?;
    let mut cpu_mask = AxCpuMask::new();

    for i in 0..(size * 8).min(axhal::cpu_num()) {
        if user_mask[i / 8] & (1 << (i % 8)) != 0 {
            cpu_mask.set(i, true);
        }
    }
    if cpu_mask.is_empty() {
        return Err(AxError::InvalidInput);
    }

    let task = sched_task(pid)?;
    let curr = current();
    let curr_pid = curr.as_thread().proc_data.proc.pid() as i32;
    let curr_euid = curr.as_thread().proc_data.ids().1;
    if pid != 0 && pid != curr_pid && curr_euid != 0 {
        return Err(AxError::OperationNotPermitted);
    }

    if task.id() == curr.id() {
        axtask::set_current_affinity(cpu_mask);
    }

    Ok(0)
}

fn normalize_sched_priority(policy: u32, priority: i32) -> AxResult<i32> {
    match policy {
        SCHED_FIFO | SCHED_RR => {
            if (1..=99).contains(&priority) {
                Ok(priority)
            } else {
                Err(AxError::InvalidInput)
            }
        }
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE => {
            if priority == 0 {
                Ok(0)
            } else {
                Err(AxError::InvalidInput)
            }
        }
        SCHED_DEADLINE => Ok(0),
        _ => Err(AxError::InvalidInput),
    }
}

fn sched_task(pid: i32) -> AxResult<AxTaskRef> {
    if pid < 0 {
        Err(AxError::InvalidInput)
    } else if pid == 0 {
        Ok(current().clone())
    } else {
        get_process_task(pid as _)
    }
}

pub fn sys_sched_getscheduler(pid: i32) -> AxResult<isize> {
    let task = sched_task(pid)?;
    Ok(task.as_thread().sched_policy() as _)
}

fn read_sched_priority(param: *const (), policy: u32) -> AxResult<i32> {
    if param.is_null() {
        return Err(AxError::InvalidInput);
    }
    let bytes = vm_load(param.cast(), size_of::<i32>())?;
    let priority = i32::from_ne_bytes(bytes.as_slice().try_into().unwrap());
    normalize_sched_priority(policy, priority)
}

fn check_sched_permission(task: &AxTaskRef, policy: u32, priority: i32) -> AxResult<()> {
    let curr = current();
    let curr_euid = curr.as_thread().proc_data.ids().1;
    if curr_euid == 0 {
        return Ok(());
    }

    let target_pid = task.as_thread().proc_data.proc.pid();
    let curr_pid = curr.as_thread().proc_data.proc.pid();
    if target_pid != curr_pid || matches!(policy, SCHED_FIFO | SCHED_RR | SCHED_DEADLINE) || priority != 0 {
        return Err(AxError::OperationNotPermitted);
    }
    Ok(())
}

pub fn sys_sched_setparam(pid: i32, param: *const ()) -> AxResult<isize> {
    let priority = if param.is_null() {
        return Err(AxError::InvalidInput);
    } else {
        let bytes = vm_load(param.cast(), size_of::<i32>())?;
        i32::from_ne_bytes(bytes.as_slice().try_into().unwrap())
    };
    let task = sched_task(pid)?;
    let policy = task.as_thread().sched_policy();
    let priority = normalize_sched_priority(policy, priority)?;
    check_sched_permission(&task, policy, priority)?;
    task.as_thread().set_sched_param(policy, priority);
    Ok(0)
}

pub fn sys_sched_setscheduler(pid: i32, policy: i32, param: *const ()) -> AxResult<isize> {
    let policy = policy as u32;
    let priority = read_sched_priority(param, policy)?;
    let task = sched_task(pid)?;
    check_sched_permission(&task, policy, priority)?;
    task.as_thread().set_sched_param(policy, priority);
    Ok(0)
}

pub fn sys_sched_getparam(pid: i32, param: *mut ()) -> AxResult<isize> {
    if param.is_null() {
        return Err(AxError::InvalidInput);
    }
    let priority = sched_task(pid)?.as_thread().sched_priority();
    vm_write_slice(param.cast(), &priority.to_ne_bytes())?;
    Ok(0)
}

pub fn sys_sched_get_priority_max(policy: i32) -> AxResult<isize> {
    match policy as u32 {
        SCHED_FIFO | SCHED_RR => Ok(99),
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE | SCHED_DEADLINE => Ok(0),
        _ => Err(AxError::InvalidInput),
    }
}

pub fn sys_sched_get_priority_min(policy: i32) -> AxResult<isize> {
    match policy as u32 {
        SCHED_FIFO | SCHED_RR => Ok(1),
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE | SCHED_DEADLINE => Ok(0),
        _ => Err(AxError::InvalidInput),
    }
}

pub fn sys_sched_rr_get_interval(pid: i32, tp: *mut timespec) -> AxResult<isize> {
    let task = sched_task(pid)?;
    let ts = if matches!(task.as_thread().sched_policy(), SCHED_FIFO) {
        timespec { tv_sec: 0, tv_nsec: 0 }
    } else {
        timespec {
            tv_sec: 0,
            tv_nsec: 100_000_000,
        }
    };
    tp.vm_write(ts)?;
    Ok(0)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SchedAttr {
    pub size: u32,
    pub sched_policy: u32,
    pub sched_flags: u64,
    pub sched_nice: i32,
    pub sched_priority: u32,
    pub sched_runtime: u64,
    pub sched_deadline: u64,
    pub sched_period: u64,
}

impl SchedAttr {
    fn default_for(policy: u32, priority: u32, runtime: u64, deadline: u64, period: u64) -> Self {
        Self {
            size: size_of::<Self>() as u32,
            sched_policy: policy,
            sched_flags: 0,
            sched_nice: 0,
            sched_priority: priority,
            sched_runtime: runtime,
            sched_deadline: deadline,
            sched_period: period,
        }
    }
}

pub fn sys_sched_setattr(pid: i32, attr: *const u8, _flags: u32) -> AxResult<isize> {
    if _flags != 0 {
        return Err(AxError::InvalidInput);
    }
    if attr.is_null() {
        return Err(AxError::InvalidInput);
    }
    let size: u32 = unsafe { (attr as *const u32).vm_read_uninit()?.assume_init() };
    if size >= size_of::<SchedAttr>() as u32 {
        let attr = unsafe { (attr as *const SchedAttr).vm_read_uninit()?.assume_init() };
        let priority = normalize_sched_priority(attr.sched_policy, attr.sched_priority as _)?;
        let task = sched_task(pid)?;
        check_sched_permission(&task, attr.sched_policy, priority)?;
        task.as_thread().set_sched_param(attr.sched_policy, priority);
        task.as_thread().set_sched_deadline_params(
            attr.sched_runtime,
            attr.sched_deadline,
            attr.sched_period,
        );
    }
    Ok(0)
}

pub fn sys_sched_getattr(pid: i32, attr: *mut u8, size: u32, flags: u32) -> AxResult<isize> {
    if flags != 0 {
        return Err(AxError::InvalidInput);
    }
    if attr.is_null() {
        return Err(AxError::InvalidInput);
    }
    if size < size_of::<SchedAttr>() as u32 {
        return Err(AxError::InvalidInput);
    }
    let task = sched_task(pid)?;
    let (policy, priority) = (
        task.as_thread().sched_policy(),
        task.as_thread().sched_priority() as _,
    );
    let (runtime, deadline, period) = task.as_thread().sched_deadline_params();
    let sched_attr = SchedAttr::default_for(policy, priority, runtime, deadline, period);
    (attr as *mut SchedAttr).vm_write(sched_attr)?;
    Ok(0)
}

pub fn sys_getpriority(which: u32, who: u32) -> AxResult<isize> {
    debug!("sys_getpriority <= which: {which}, who: {who}");

    match which {
        PRIO_PROCESS => {
            let task = if who == 0 {
                current().clone()
            } else {
                get_process_task(who)?
            };
            Ok(priority_syscall_value(task.as_thread().nice()))
        }
        PRIO_PGRP => {
            if who != 0 {
                let _pg = get_process_group(who)?;
            }
            Ok(priority_syscall_value(current().as_thread().nice()))
        }
        PRIO_USER => {
            if who == 0 {
                Ok(priority_syscall_value(current().as_thread().nice()))
            } else {
                Err(AxError::NoSuchProcess)
            }
        }
        _ => Err(AxError::InvalidInput),
    }
}

pub fn sys_setpriority(which: u32, who: u32, prio: i32) -> AxResult<isize> {
    debug!("sys_setpriority <= which: {which}, who: {who}, prio: {prio}");

    let curr = current();
    let curr_nice = curr.as_thread().nice();
    let curr_euid = curr.as_thread().proc_data.ids().1;

    match which {
        PRIO_PROCESS => {
            let task = if who == 0 {
                curr.clone()
            } else {
                get_process_task(who)?
            };
            if curr_euid != 0 && !AxTaskRef::ptr_eq(&task, &curr) {
                return Err(AxError::OperationNotPermitted);
            }
            if prio < curr_nice && curr_euid != 0 {
                return Err(AxError::PermissionDenied);
            }
            task.as_thread().set_nice(prio);
            Ok(0)
        }
        PRIO_PGRP => {
            if who != 0 {
                let _pg = get_process_group(who)?;
            }
            if prio < curr_nice && curr_euid != 0 {
                return Err(AxError::PermissionDenied);
            }
            current().as_thread().set_nice(prio);
            Ok(0)
        }
        PRIO_USER => {
            if who == 0 {
                if prio < curr_nice && curr_euid != 0 {
                    return Err(AxError::PermissionDenied);
                }
                current().as_thread().set_nice(prio);
                Ok(0)
            } else {
                Err(AxError::NoSuchProcess)
            }
        }
        _ => Err(AxError::InvalidInput),
    }
}

pub fn sys_personality(persona: usize) -> AxResult<isize> {
    const GET_PERSONA: usize = 0xffff_ffff;

    let curr = current();
    let thread = curr.as_thread();
    let old = thread.personality();
    if persona != GET_PERSONA {
        thread.set_personality(persona);
    }
    Ok(old as _)
}
