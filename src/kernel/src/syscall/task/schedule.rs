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
    PRIO_USER, SCHED_FIFO, SCHED_NORMAL, SCHED_RR, TIMER_ABSTIME,
};
use starry_vm::{vm_load, vm_write_slice, VmMutPtr, VmPtr};

use crate::{
    task::{get_process_data, get_process_group, get_task, AsThread},
    time::TimeValueLike,
};

pub fn sys_sched_yield() -> AxResult<isize> {
    axtask::yield_now();
    Ok(0)
}

fn sleep_impl(clock: impl Fn() -> TimeValue, dur: TimeValue) -> TimeValue {
    debug!("sleep_impl <= {dur:?}");

    let start = clock();

    // TODO: currently ignoring concrete clock type
    // We detect EINTR manually if the slept time is not enough.
    let _ = block_on(interruptible(sleep(dur)));

    clock() - start
}

/// Sleep some nanoseconds
pub fn sys_nanosleep(req: *const timespec, rem: *mut timespec) -> AxResult<isize> {
    // FIXME: AnyBitPattern
    let req = unsafe { req.vm_read_uninit()?.assume_init() }.try_into_time_value()?;
    debug!("sys_nanosleep <= req: {req:?}");

    let actual = sleep_impl(axhal::time::monotonic_time, req);

    if let Some(diff) = req.checked_sub(actual) {
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
        CLOCK_REALTIME => axhal::time::wall_time,
        CLOCK_MONOTONIC => axhal::time::monotonic_time,
        _ => {
            warn!("Unsupported clock_id: {clock_id}");
            return Err(AxError::InvalidInput);
        }
    };

    let req = unsafe { req.vm_read_uninit()?.assume_init() }.try_into_time_value()?;
    debug!("sys_clock_nanosleep <= clock_id: {clock_id}, flags: {flags}, req: {req:?}");

    let dur = if flags & TIMER_ABSTIME != 0 {
        req.saturating_sub(clock())
    } else {
        req
    };

    let actual = sleep_impl(clock, dur);

    if let Some(diff) = dur.checked_sub(actual) {
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
    if cpusetsize * 8 < axhal::cpu_num() {
        return Err(AxError::InvalidInput);
    }

    // TODO: support other threads
    if pid != 0 {
        return Err(AxError::OperationNotPermitted);
    }

    let mask = current().cpumask();
    let mask_bytes = mask.as_bytes();

    vm_write_slice(user_mask, mask_bytes)?;

    Ok(mask_bytes.len() as _)
}

pub fn sys_sched_setaffinity(
    _pid: i32,
    cpusetsize: usize,
    user_mask: *const u8,
) -> AxResult<isize> {
    let size = cpusetsize.min(axhal::cpu_num().div_ceil(8));
    let user_mask = vm_load(user_mask, size)?;
    let mut cpu_mask = AxCpuMask::new();

    for i in 0..(size * 8).min(axhal::cpu_num()) {
        if user_mask[i / 8] & (1 << (i % 8)) != 0 {
            cpu_mask.set(i, true);
        }
    }

    // TODO: support other threads
    axtask::set_current_affinity(cpu_mask);

    Ok(0)
}

fn normalize_sched_priority(policy: u32, priority: i32) -> AxResult<i32> {
    match policy {
        SCHED_FIFO | SCHED_RR => Ok(priority.clamp(1, 99)),
        SCHED_NORMAL => Ok(0),
        _ => Err(AxError::InvalidInput),
    }
}

fn sched_task(pid: i32) -> AxResult<AxTaskRef> {
    if pid <= 0 {
        Ok(current().clone())
    } else {
        get_task(pid as _)
    }
}

pub fn sys_sched_getscheduler(pid: i32) -> AxResult<isize> {
    let task = sched_task(pid)?;
    Ok(task.as_thread().sched_policy() as _)
}

fn read_sched_priority(param: *const (), policy: u32) -> AxResult<i32> {
    let fallback = match policy {
        SCHED_FIFO | SCHED_RR => 1,
        SCHED_NORMAL => 0,
        _ => return Err(AxError::InvalidInput),
    };
    let Some(_) = param.nullable() else {
        return Ok(fallback);
    };
    let Ok(bytes) = vm_load(param.cast(), size_of::<i32>()) else {
        return Ok(fallback);
    };
    let priority = i32::from_ne_bytes(bytes.as_slice().try_into().unwrap());
    Ok(priority)
}

pub fn sys_sched_setparam(pid: i32, param: *const ()) -> AxResult<isize> {
    let task = sched_task(pid)?;
    let policy = task.as_thread().sched_policy();
    let priority = normalize_sched_priority(policy, read_sched_priority(param, policy)?)?;
    task.as_thread().set_sched_param(policy, priority);
    Ok(0)
}

pub fn sys_sched_setscheduler(pid: i32, policy: i32, param: *const ()) -> AxResult<isize> {
    let policy = policy as u32;
    let priority = normalize_sched_priority(policy, read_sched_priority(param, policy)?)?;
    sched_task(pid)?
        .as_thread()
        .set_sched_param(policy, priority);
    Ok(0)
}

pub fn sys_sched_getparam(pid: i32, param: *mut ()) -> AxResult<isize> {
    let priority = sched_task(pid)?.as_thread().sched_priority();
    let _ = vm_write_slice(param.cast(), &priority.to_ne_bytes());
    Ok(0)
}

pub fn sys_sched_get_priority_max(policy: i32) -> AxResult<isize> {
    match policy as u32 {
        SCHED_FIFO | SCHED_RR => Ok(99),
        SCHED_NORMAL => Ok(0),
        _ => Err(AxError::InvalidInput),
    }
}

pub fn sys_sched_get_priority_min(policy: i32) -> AxResult<isize> {
    match policy as u32 {
        SCHED_FIFO | SCHED_RR => Ok(1),
        SCHED_NORMAL => Ok(0),
        _ => Err(AxError::InvalidInput),
    }
}

pub fn sys_sched_rr_get_interval(_pid: i32, tp: *mut timespec) -> AxResult<isize> {
    let ts = timespec {
        tv_sec: 0,
        tv_nsec: 100_000_000,
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
    fn default_for(policy: u32, priority: u32) -> Self {
        Self {
            size: size_of::<Self>() as u32,
            sched_policy: policy,
            sched_flags: 0,
            sched_nice: 0,
            sched_priority: priority,
            sched_runtime: 0,
            sched_deadline: 0,
            sched_period: 0,
        }
    }
}

pub fn sys_sched_setattr(pid: i32, attr: *const u8, _flags: u32) -> AxResult<isize> {
    let size: u32 = unsafe { (attr as *const u32).vm_read_uninit()?.assume_init() };
    if size >= size_of::<SchedAttr>() as u32 {
        let attr = unsafe { (attr as *const SchedAttr).vm_read_uninit()?.assume_init() };
        let priority = normalize_sched_priority(attr.sched_policy, attr.sched_priority as _)?;
        sched_task(pid)?
            .as_thread()
            .set_sched_param(attr.sched_policy, priority);
    }
    Ok(0)
}

pub fn sys_sched_getattr(pid: i32, attr: *mut u8, size: u32, _flags: u32) -> AxResult<isize> {
    if size < size_of::<SchedAttr>() as u32 {
        return Err(AxError::InvalidInput);
    }
    let task = sched_task(pid)?;
    let (policy, priority) = (
        task.as_thread().sched_policy(),
        task.as_thread().sched_priority() as _,
    );
    let sched_attr = SchedAttr::default_for(policy, priority);
    (attr as *mut SchedAttr).vm_write(sched_attr)?;
    Ok(0)
}

pub fn sys_getpriority(which: u32, who: u32) -> AxResult<isize> {
    debug!("sys_getpriority <= which: {which}, who: {who}");

    match which {
        PRIO_PROCESS => {
            if who != 0 {
                let _proc = get_process_data(who)?;
            }
            Ok(20)
        }
        PRIO_PGRP => {
            if who != 0 {
                let _pg = get_process_group(who)?;
            }
            Ok(20)
        }
        PRIO_USER => {
            if who == 0 {
                Ok(20)
            } else {
                Err(AxError::NoSuchProcess)
            }
        }
        _ => Err(AxError::InvalidInput),
    }
}
