use alloc::{borrow::Cow, collections::BTreeMap, sync::Arc};
use core::{
    sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering},
    task::Context,
};

use axerrno::{AxError, AxResult};
use axhal::time::{
    NANOS_PER_MICROS, NANOS_PER_SEC, TimeValue, monotonic_time_nanos, nanos_to_ticks,
    wall_time_nanos,
};
use axpoll::{IoEvents, Pollable};
use axtask::current;
use linux_raw_sys::general::{
    __kernel_clockid_t, CLOCK_BOOTTIME, CLOCK_MONOTONIC, CLOCK_MONOTONIC_COARSE,
    CLOCK_MONOTONIC_RAW, CLOCK_PROCESS_CPUTIME_ID, CLOCK_REALTIME, CLOCK_REALTIME_COARSE,
    CLOCK_THREAD_CPUTIME_ID, O_CLOEXEC, O_NONBLOCK, TIMER_ABSTIME, itimerspec, itimerval,
    sigevent, timespec, timeval,
};
use kspin::SpinNoIrq;
use starry_process::Pid;
use starry_signal::{SignalInfo, Signo};
use starry_vm::{VmMutPtr, VmPtr};

use crate::{
    file::{FileLike, IoDst},
    task::{AsThread, ITimerType, send_signal_to_process},
    time::TimeValueLike,
};

const CAP_SYS_TIME: u32 = 25;

const TIME_OK: isize = 0;
const ADJ_OFFSET: u32 = 0x0001;
const ADJ_FREQUENCY: u32 = 0x0002;
const ADJ_MAXERROR: u32 = 0x0004;
const ADJ_ESTERROR: u32 = 0x0008;
const ADJ_STATUS: u32 = 0x0010;
const ADJ_TIMECONST: u32 = 0x0020;
const ADJ_TAI: u32 = 0x0080;
const ADJ_SETOFFSET: u32 = 0x0100;
const ADJ_MICRO: u32 = 0x1000;
const ADJ_NANO: u32 = 0x2000;
const ADJ_TICK: u32 = 0x4000;
const ADJ_OFFSET_SINGLESHOT: u32 = 0x8001;
const SUPPORTED_ADJTIMEX_MODES: u32 = ADJ_OFFSET
    | ADJ_FREQUENCY
    | ADJ_MAXERROR
    | ADJ_ESTERROR
    | ADJ_STATUS
    | ADJ_TIMECONST
    | ADJ_TAI
    | ADJ_SETOFFSET
    | ADJ_MICRO
    | ADJ_NANO
    | ADJ_TICK
    | ADJ_OFFSET_SINGLESHOT;

static WALL_TIME_OFFSET_NS: AtomicI64 = AtomicI64::new(0);
static TIMEX_STATE: SpinNoIrq<TimexState> = SpinNoIrq::new(TimexState::new());
static NEXT_TIMER_ID: AtomicI32 = AtomicI32::new(1);
static POSIX_TIMERS: SpinNoIrq<BTreeMap<i32, PosixTimer>> = SpinNoIrq::new(BTreeMap::new());

pub fn clear_posix_timers_for_process(pid: Pid) {
    POSIX_TIMERS.lock().retain(|_, timer| timer.pid != pid);
}

const TFD_CLOEXEC: u32 = O_CLOEXEC;
const TFD_NONBLOCK: u32 = O_NONBLOCK;
const TFD_TIMER_ABSTIME: u32 = 1 << 0;
const TFD_TIMER_CANCEL_ON_SET: u32 = 1 << 1;

#[derive(Clone, Copy)]
struct TimexState {
    offset: isize,
    freq: isize,
    maxerror: isize,
    esterror: isize,
    constant: isize,
    tick: isize,
}

impl TimexState {
    const fn new() -> Self {
        Self {
            offset: 0,
            freq: 0,
            maxerror: 0,
            esterror: 0,
            constant: 0,
            tick: 0,
        }
    }
}

pub fn realtime_nanos() -> u64 {
    wall_time_nanos().saturating_add_signed(WALL_TIME_OFFSET_NS.load(Ordering::Relaxed))
}

pub fn realtime_time() -> TimeValue {
    TimeValue::from_nanos(realtime_nanos() as _)
}

fn valid_clock_id(clock_id: __kernel_clockid_t) -> bool {
    matches!(
        clock_id as u32,
        CLOCK_REALTIME
            | CLOCK_REALTIME_COARSE
            | CLOCK_MONOTONIC
            | CLOCK_MONOTONIC_RAW
            | CLOCK_MONOTONIC_COARSE
            | CLOCK_BOOTTIME
            | CLOCK_PROCESS_CPUTIME_ID
            | CLOCK_THREAD_CPUTIME_ID
    ) || matches!(clock_id, 8 | 9)
        || is_dynamic_cpu_clock(clock_id)
}

fn is_dynamic_cpu_clock(clock_id: __kernel_clockid_t) -> bool {
    clock_id < 0 && matches!(clock_id & 7, 2 | 6)
}

pub fn sys_clock_gettime(clock_id: __kernel_clockid_t, ts: *mut timespec) -> AxResult<isize> {
    let nanos = match clock_id as u32 {
        CLOCK_REALTIME | CLOCK_REALTIME_COARSE => realtime_nanos(),
        CLOCK_MONOTONIC | CLOCK_MONOTONIC_RAW | CLOCK_MONOTONIC_COARSE | CLOCK_BOOTTIME => {
            monotonic_time_nanos()
        }
        CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID if !is_dynamic_cpu_clock(clock_id) => {
            let (utime, stime) = current()
                .as_thread()
                .time
                .try_borrow()
                .map_err(|_| AxError::WouldBlock)?
                .output();
            (utime + stime).as_nanos() as u64
        }
        _ if is_dynamic_cpu_clock(clock_id) => {
            let (utime, stime) = current()
                .as_thread()
                .time
                .try_borrow()
                .map_err(|_| AxError::WouldBlock)?
                .output();
            (utime + stime).as_nanos() as u64
        }
        _ => {
            warn!("Called sys_clock_gettime for unsupported clock {clock_id}");
            return Err(AxError::InvalidInput);
        }
    };
    // Compute timespec directly from nanos to avoid intermediate Duration allocation
    ts.vm_write(timespec {
        tv_sec: (nanos / NANOS_PER_SEC) as _,
        tv_nsec: (nanos % NANOS_PER_SEC) as _,
    })?;
    Ok(0)
}

#[repr(C)]
pub(crate) struct Timezone {
    tz_minuteswest: i32,
    tz_dsttime: i32,
}

pub fn sys_gettimeofday(ts: *mut timeval, tz: *mut Timezone) -> AxResult<isize> {
    let nanos = realtime_nanos();
    // Compute timeval directly from nanos to avoid intermediate Duration allocation
    if let Some(ts) = ts.nullable() {
        ts.vm_write(timeval {
            tv_sec: (nanos / NANOS_PER_SEC) as _,
            tv_usec: ((nanos % NANOS_PER_SEC) / NANOS_PER_MICROS) as _,
        })?;
    }
    if let Some(tz) = tz.nullable() {
        tz.vm_write(Timezone {
            tz_minuteswest: 0,
            tz_dsttime: 0,
        })?;
    }
    Ok(0)
}

pub fn sys_clock_getres(clock_id: __kernel_clockid_t, res: *mut timespec) -> AxResult<isize> {
    if !valid_clock_id(clock_id) {
        warn!("Called sys_clock_getres for unsupported clock {clock_id}");
        return Err(AxError::InvalidInput);
    }
    if let Some(res) = res.nullable() {
        res.vm_write(timespec::from_time_value(TimeValue::from_nanos(1)))?;
    }
    Ok(0)
}

pub fn sys_clock_settime(clock_id: __kernel_clockid_t, ts: *const timespec) -> AxResult<isize> {
    if clock_id as u32 != CLOCK_REALTIME {
        return Err(AxError::InvalidInput);
    }
    let ts = unsafe { ts.vm_read_uninit()?.assume_init() }.try_into_time_value()?;
    if !current().as_thread().proc_data.has_capability(CAP_SYS_TIME) {
        return Err(AxError::OperationNotPermitted);
    }
    let target = ts.as_nanos() as i128;
    let current = wall_time_nanos() as i128;
    let offset = target.saturating_sub(current);
    let offset = offset.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
    WALL_TIME_OFFSET_NS.store(offset, Ordering::Relaxed);
    Ok(0)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Timex {
    modes: u32,
    offset: isize,
    freq: isize,
    maxerror: isize,
    esterror: isize,
    status: i32,
    constant: isize,
    precision: isize,
    tolerance: isize,
    time: timeval,
    tick: isize,
    ppsfreq: isize,
    jitter: isize,
    shift: i32,
    stabil: isize,
    jitcnt: isize,
    calcnt: isize,
    errcnt: isize,
    stbcnt: isize,
    tai: i32,
    padding: [i32; 11],
}

pub fn sys_adjtimex(buf: *mut Timex) -> AxResult<isize> {
    let mut timex = unsafe { buf.vm_read_uninit()?.assume_init() };
    if timex.modes != ADJ_OFFSET_SINGLESHOT && timex.modes & !SUPPORTED_ADJTIMEX_MODES != 0 {
        return Err(AxError::InvalidInput);
    }
    if timex.modes == 0x8000 {
        return Err(AxError::InvalidInput);
    }
    if timex.modes != 0 && !current().as_thread().proc_data.has_capability(CAP_SYS_TIME) {
        return Err(AxError::OperationNotPermitted);
    }
    if timex.modes & ADJ_TICK != 0
        && timex.tick != 0
        && timex.tick != 1000
        && !(9000..=11000).contains(&timex.tick)
    {
        return Err(AxError::InvalidInput);
    }

    let mut state = TIMEX_STATE.lock();
    if timex.modes != 0 {
        if timex.modes & ADJ_OFFSET != 0 {
            state.offset = timex.offset;
        }
        if timex.modes & ADJ_FREQUENCY != 0 {
            state.freq = timex.freq;
        }
        if timex.modes & ADJ_MAXERROR != 0 {
            state.maxerror = timex.maxerror;
        }
        if timex.modes & ADJ_ESTERROR != 0 {
            state.esterror = timex.esterror;
        }
        if timex.modes & ADJ_TIMECONST != 0 {
            state.constant = timex.constant;
        }
        if timex.modes & ADJ_TICK != 0 {
            state.tick = timex.tick;
        }
    }

    let nanos = realtime_nanos();
    timex.offset = state.offset;
    timex.freq = state.freq;
    timex.maxerror = state.maxerror;
    timex.esterror = state.esterror;
    timex.constant = state.constant;
    timex.tick = state.tick;
    timex.time = timeval {
        tv_sec: (nanos / NANOS_PER_SEC) as _,
        tv_usec: ((nanos % NANOS_PER_SEC) / NANOS_PER_MICROS) as _,
    };
    timex.precision = 1;
    timex.tolerance = 0;
    timex.tai = 0;
    buf.vm_write(timex)?;
    Ok(TIME_OK)
}

pub fn sys_clock_adjtime(clock_id: __kernel_clockid_t, buf: *mut Timex) -> AxResult<isize> {
    if clock_id as u32 != CLOCK_REALTIME {
        return Err(AxError::InvalidInput);
    }
    sys_adjtimex(buf)
}

#[derive(Clone, Copy, Default)]
struct PosixTimer {
    pid: Pid,
    clock_id: __kernel_clockid_t,
    signo: u8,
    generation: u64,
    interval_ns: u64,
    expires_mono_ns: u64,
    overrun: i32,
}

fn timespec_to_nanos(ts: timespec) -> AxResult<u64> {
    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= NANOS_PER_SEC as _ {
        return Err(AxError::InvalidInput);
    }
    (ts.tv_sec as u64)
        .checked_mul(NANOS_PER_SEC)
        .and_then(|it| it.checked_add(ts.tv_nsec as u64))
        .ok_or(AxError::InvalidInput)
}

fn timer_clock_now(clock_id: __kernel_clockid_t) -> AxResult<u64> {
    match clock_id as u32 {
        CLOCK_REALTIME => Ok(realtime_nanos()),
        CLOCK_MONOTONIC | CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID => {
            Ok(monotonic_time_nanos())
        }
        _ => Err(AxError::InvalidInput),
    }
}

fn timer_expiry_to_mono(clock_id: __kernel_clockid_t, flags: u32, value_ns: u64) -> AxResult<u64> {
    if value_ns == 0 {
        return Ok(0);
    }

    let delay = if flags & TIMER_ABSTIME != 0 {
        value_ns.saturating_sub(timer_clock_now(clock_id)?)
    } else {
        value_ns
    };
    Ok(monotonic_time_nanos().saturating_add(delay))
}

pub fn sys_timer_create(
    clock_id: __kernel_clockid_t,
    sevp: *const sigevent,
    timerid: *mut i32,
) -> AxResult<isize> {
    let _ = timer_clock_now(clock_id)?;
    let signo = if sevp.is_null() {
        Signo::SIGALRM as u8
    } else {
        let event = unsafe { sevp.vm_read_uninit()?.assume_init() };
        if event.sigev_notify == 1 {
            0
        } else if event.sigev_signo == 0 {
            Signo::SIGALRM as u8
        } else {
            event.sigev_signo as u8
        }
    };
    if signo != 0 && Signo::from_repr(signo).is_none() {
        return Err(AxError::InvalidInput);
    }

    let id = NEXT_TIMER_ID.fetch_add(1, Ordering::Relaxed);
    let pid = current().as_thread().proc_data.proc.pid();
    POSIX_TIMERS.lock().insert(
        id,
        PosixTimer {
            pid,
            clock_id,
            signo,
            ..Default::default()
        },
    );
    timerid.vm_write(id)?;
    Ok(0)
}

pub fn sys_timer_settime(
    timerid: i32,
    flags: u32,
    new_value: *const itimerspec,
    old_value: *mut itimerspec,
) -> AxResult<isize> {
    if new_value.is_null() {
        return Err(AxError::InvalidInput);
    }
    if let Some(old_value) = old_value.nullable() {
        sys_timer_gettime(timerid, old_value)?;
    }

    let new_value = unsafe { new_value.vm_read_uninit()?.assume_init() };
    let value_ns = timespec_to_nanos(new_value.it_value)?;
    let interval_ns = timespec_to_nanos(new_value.it_interval)?;

    let mut timers = POSIX_TIMERS.lock();
    let timer = timers.get_mut(&timerid).ok_or(AxError::InvalidInput)?;
    timer.generation = timer.generation.wrapping_add(1);
    timer.interval_ns = interval_ns;
    timer.expires_mono_ns = timer_expiry_to_mono(timer.clock_id, flags, value_ns)?;
    timer.overrun = 0;
    if flags & TIMER_ABSTIME != 0 && interval_ns != 0 {
        let now = timer_clock_now(timer.clock_id)?;
        if value_ns < now {
            let missed = (now - value_ns) / interval_ns;
            timer.overrun = missed.min(i32::MAX as u64) as i32;
        }
    }

    if value_ns != 0 && timer.signo != 0 {
        let generation = timer.generation;
        axtask::spawn_with_name(move || {
            loop {
                let expires_mono_ns = {
                    let timers = POSIX_TIMERS.lock();
                    let Some(timer) = timers.get(&timerid) else {
                        return;
                    };
                    if timer.generation != generation || timer.expires_mono_ns == 0 {
                        return;
                    }
                    timer.expires_mono_ns
                };

                let delay = expires_mono_ns.saturating_sub(monotonic_time_nanos());
                axtask::future::block_on(axtask::future::sleep(TimeValue::from_nanos(delay as _)));

                let mut timers = POSIX_TIMERS.lock();
                let Some(timer) = timers.get_mut(&timerid) else {
                    return;
                };
                if timer.generation != generation || timer.expires_mono_ns == 0 {
                    return;
                }
                if monotonic_time_nanos() < timer.expires_mono_ns {
                    return;
                }
                if let Some(signo) = Signo::from_repr(timer.signo) {
                    let _ = send_signal_to_process(timer.pid, Some(SignalInfo::new_kernel(signo)));
                }
                if timer.interval_ns == 0 {
                    timer.expires_mono_ns = 0;
                    return;
                }
                timer.expires_mono_ns = monotonic_time_nanos().saturating_add(timer.interval_ns);
            }
        }, "posix-timer".into());
    }

    Ok(0)
}

pub fn sys_timer_gettime(timerid: i32, curr_value: *mut itimerspec) -> AxResult<isize> {
    let timers = POSIX_TIMERS.lock();
    let timer = timers.get(&timerid).ok_or(AxError::InvalidInput)?;
    let now = monotonic_time_nanos();
    let remained = timer.expires_mono_ns.saturating_sub(now);
    curr_value.vm_write(itimerspec {
        it_interval: timespec {
            tv_sec: (timer.interval_ns / NANOS_PER_SEC) as _,
            tv_nsec: (timer.interval_ns % NANOS_PER_SEC) as _,
        },
        it_value: timespec {
            tv_sec: (remained / NANOS_PER_SEC) as _,
            tv_nsec: (remained % NANOS_PER_SEC) as _,
        },
    })?;
    Ok(0)
}

pub fn sys_timer_getoverrun(timerid: i32) -> AxResult<isize> {
    POSIX_TIMERS
        .lock()
        .get(&timerid)
        .map(|timer| timer.overrun as _)
        .ok_or(AxError::InvalidInput)
}

pub fn sys_timer_delete(timerid: i32) -> AxResult<isize> {
    POSIX_TIMERS
        .lock()
        .remove(&timerid)
        .map(|_| 0)
        .ok_or(AxError::InvalidInput)
}

struct TimerFd {
    clock_id: __kernel_clockid_t,
    nonblocking: AtomicBool,
    state: SpinNoIrq<TimerFdState>,
}

#[derive(Clone, Copy)]
struct TimerFdState {
    generation: u64,
    interval_ns: u64,
    expires_mono_ns: u64,
    value: itimerspec,
}

impl TimerFd {
    fn new(clock_id: __kernel_clockid_t) -> Arc<Self> {
        Arc::new(Self {
            clock_id,
            nonblocking: AtomicBool::new(false),
            state: SpinNoIrq::new(TimerFdState {
                generation: 0,
                interval_ns: 0,
                expires_mono_ns: 0,
                value: itimerspec {
                    it_interval: timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    },
                    it_value: timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    },
                },
            }),
        })
    }

    fn current_value(&self) -> itimerspec {
        let state = self.state.lock();
        let remained = state.expires_mono_ns.saturating_sub(monotonic_time_nanos());
        itimerspec {
            it_interval: state.value.it_interval,
            it_value: timespec {
                tv_sec: (remained / NANOS_PER_SEC) as _,
                tv_nsec: (remained % NANOS_PER_SEC) as _,
            },
        }
    }

    fn consume_ticks(&self) -> u64 {
        let mut state = self.state.lock();
        if state.expires_mono_ns == 0 {
            return 0;
        }
        let now = monotonic_time_nanos();
        if now < state.expires_mono_ns {
            return 0;
        }
        if state.interval_ns == 0 {
            state.expires_mono_ns = 0;
            state.value.it_value = timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            return 1;
        }
        let ticks = ((now - state.expires_mono_ns) / state.interval_ns) + 1;
        state.expires_mono_ns = state
            .expires_mono_ns
            .saturating_add(ticks.saturating_mul(state.interval_ns));
        ticks
    }

    fn armed(&self) -> bool {
        self.state.lock().expires_mono_ns != 0
    }
}

impl FileLike for TimerFd {
    fn read(&self, dst: &mut IoDst) -> axio::Result<usize> {
        if dst.remaining_mut() < size_of::<u64>() {
            return Err(AxError::InvalidInput);
        }
        let ticks = self.consume_ticks();
        if ticks == 0 {
            if self.nonblocking() {
                return Err(AxError::WouldBlock);
            }
            loop {
                axtask::yield_now();
                let ticks = self.consume_ticks();
                if ticks != 0 {
                    dst.write(&ticks.to_ne_bytes())?;
                    return Ok(size_of::<u64>());
                }
            }
        }
        dst.write(&ticks.to_ne_bytes())?;
        Ok(size_of::<u64>())
    }

    fn path(&self) -> Cow<'_, str> {
        "anon_inode:[timerfd]".into()
    }

    fn nonblocking(&self) -> bool {
        self.nonblocking.load(Ordering::Relaxed)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> AxResult {
        self.nonblocking.store(nonblocking, Ordering::Relaxed);
        Ok(())
    }
}

impl Pollable for TimerFd {
    fn poll(&self) -> IoEvents {
        let mut events = IoEvents::empty();
        events.set(IoEvents::IN, self.armed());
        events
    }

    fn register(&self, _context: &mut Context<'_>, _events: IoEvents) {}
}

fn zero_itimerspec() -> itimerspec {
    itimerspec {
        it_interval: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        it_value: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
    }
}

impl TimerFdState {
    fn disarmed(generation: u64) -> Self {
        Self {
            generation,
            interval_ns: 0,
            expires_mono_ns: 0,
            value: zero_itimerspec(),
        }
    }
}

pub fn sys_timerfd_create(clock_id: __kernel_clockid_t, flags: u32) -> AxResult<isize> {
    if flags & !(TFD_CLOEXEC | TFD_NONBLOCK) != 0 {
        return Err(AxError::InvalidInput);
    }
    let _ = timer_clock_now(clock_id)?;
    let timerfd = TimerFd::new(clock_id);
    timerfd.set_nonblocking(flags & TFD_NONBLOCK != 0)?;
    crate::file::add_file_like(timerfd, flags & TFD_CLOEXEC != 0).map(|fd| fd as _)
}

pub fn sys_timerfd_settime(
    fd: i32,
    flags: u32,
    new_value: *const itimerspec,
    old_value: *mut itimerspec,
) -> AxResult<isize> {
    if flags & !(TFD_TIMER_ABSTIME | TFD_TIMER_CANCEL_ON_SET) != 0 || new_value.is_null() {
        return Err(AxError::InvalidInput);
    }
    let timerfd = TimerFd::from_fd(fd)?;
    if let Some(old_value) = old_value.nullable() {
        old_value.vm_write(timerfd.current_value())?;
    }
    let new_value = unsafe { new_value.vm_read_uninit()?.assume_init() };
    let value_ns = timespec_to_nanos(new_value.it_value)?;
    let interval_ns = timespec_to_nanos(new_value.it_interval)?;
    let expires_mono_ns = timer_expiry_to_mono(timerfd.clock_id, flags, value_ns)?;
    {
        let mut state = timerfd.state.lock();
        let generation = state.generation.wrapping_add(1);
        *state = if value_ns == 0 {
            TimerFdState::disarmed(generation)
        } else {
            TimerFdState {
                generation,
                interval_ns,
                expires_mono_ns,
                value: new_value,
            }
        };
    }
    Ok(0)
}

pub fn sys_timerfd_gettime(fd: i32, curr_value: *mut itimerspec) -> AxResult<isize> {
    let timerfd = TimerFd::from_fd(fd)?;
    curr_value.vm_write(timerfd.current_value())?;
    Ok(0)
}

#[repr(C)]
pub struct Tms {
    /// user time
    tms_utime: usize,
    /// system time
    tms_stime: usize,
    /// user time of children
    tms_cutime: usize,
    /// system time of children
    tms_cstime: usize,
}

pub fn sys_times(tms: *mut Tms) -> AxResult<isize> {
    let curr = current();
    let (utime, stime) = curr
        .as_thread()
        .time
        .try_borrow()
        .map_err(|_| AxError::WouldBlock)?
        .output();
    let utime = nanos_to_ticks(utime.as_nanos() as u64) as usize;
    let stime = nanos_to_ticks(stime.as_nanos() as u64) as usize;
    let (utime, stime) = curr.as_thread().proc_data.normalize_times(utime, stime);
    let (cutime, cstime) = curr.as_thread().proc_data.child_times();
    if let Some(tms) = tms.nullable() {
        tms.vm_write(Tms {
            tms_utime: utime,
            tms_stime: stime,
            tms_cutime: cutime,
            tms_cstime: cstime,
        })?;
    }
    Ok(nanos_to_ticks(monotonic_time_nanos()) as _)
}

pub fn sys_getitimer(which: i32, value: *mut itimerval) -> AxResult<isize> {
    let ty = ITimerType::from_repr(which).ok_or(AxError::InvalidInput)?;
    let (it_interval, it_value) = current()
        .as_thread()
        .time
        .try_borrow()
        .map_err(|_| AxError::WouldBlock)?
        .get_itimer(ty);

    value.vm_write(itimerval {
        it_interval: timeval::from_time_value(it_interval),
        it_value: timeval::from_time_value(it_value),
    })?;
    Ok(0)
}

pub fn sys_setitimer(
    which: i32,
    new_value: *const itimerval,
    old_value: *mut itimerval,
) -> AxResult<isize> {
    let ty = ITimerType::from_repr(which).ok_or(AxError::InvalidInput)?;
    let curr = current();

    let (interval, remained) = match new_value.nullable() {
        Some(new_value) => {
            // FIXME: AnyBitPattern
            let new_value = unsafe { new_value.vm_read_uninit()?.assume_init() };
            (
                new_value.it_interval.try_into_time_value()?.as_nanos() as usize,
                new_value.it_value.try_into_time_value()?.as_nanos() as usize,
            )
        }
        None => (0, 0),
    };

    debug!("sys_setitimer <= type: {ty:?}, interval: {interval:?}, remained: {remained:?}");

    let old = curr
        .as_thread()
        .time
        .try_borrow_mut()
        .map_err(|_| AxError::WouldBlock)?
        .set_itimer(ty, interval, remained);

    if let Some(old_value) = old_value.nullable() {
        old_value.vm_write(itimerval {
            it_interval: timeval::from_time_value(old.0),
            it_value: timeval::from_time_value(old.1),
        })?;
    }
    Ok(0)
}
