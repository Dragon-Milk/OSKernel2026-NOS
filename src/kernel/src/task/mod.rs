//! User task management.
//!
//! 本模块负责：
//! - 管理用户态进程和线程的生命周期
//! - 维护进程共享数据（`ProcessData`）和线程私有数据（`Thread`）
//! - 处理信号、futex、资源限制、时间管理等
//!
//! 核心不变量：
//! - `Thread` 通过 `AssumeSync` 包装 `RefCell<TimeManager>`，因为仅在上下文切换时独占访问
//! - `ProcessData` 由同一进程的所有线程共享
//!
//! 修改注意：
//! - 新增线程字段时需考虑并发安全性
//! - 进程退出事件通过 `PollSet` 通知等待的父进程

mod futex;
mod ops;
mod resources;
mod signal;
mod stat;
mod timer;
mod user;

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::{
    cell::RefCell,
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering},
};

use axerrno::{AxError, AxResult};
use axpoll::PollSet;
use axsync::{Mutex, spin::SpinNoIrq};
use axtask::{TaskExt, TaskInner};
use extern_trait::extern_trait;
use linux_raw_sys::general::SCHED_NORMAL;
use scope_local::{ActiveScope, Scope};
use spin::RwLock;
use starry_process::Process;
use starry_signal::{
    Signo,
    api::{ProcessSignalManager, SignalActions, ThreadSignalManager},
};

pub use self::{futex::*, ops::*, resources::*, signal::*, stat::*, timer::*, user::*};
use crate::mm::AddrSpace;

///  A wrapper type that assumes the inner type is `Sync`.
/// 假设内部类型是 `Sync` 的包装类型。
///
/// 用于包装实际仅在独占访问时才会被可变借用的类型（如 `RefCell`），
/// 使其可以安全地存储在需要 `Sync` 的结构体中。
#[repr(transparent)]
pub struct AssumeSync<T>(pub T);

unsafe impl<T> Sync for AssumeSync<T> {}

impl<T> Deref for AssumeSync<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The inner data of a thread.
/// 线程的内部数据。
///
/// 表示一个用户态线程的所有私有状态，包括信号处理、时间管理、退出标志等。
/// 线程通过 `Arc<ProcessData>` 共享进程级别的资源。
pub struct Thread {
    /// The process data shared by all threads in the process.
    /// 进程内所有线程共享的数据。
    pub proc_data: Arc<ProcessData>,

    /// Cached process ID for fast sys_getpid.
    pub pid: u32,

    /// The clear thread tid field
    ///
    /// See <https://manpages.debian.org/unstable/manpages-dev/set_tid_address.2.en.html#clear_child_tid>
    ///
    /// When the thread exits, the kernel clears the word at this address if it
    /// is not NULL.
    /// 线程退出时内核会自动清零的地址字段，用于 glibc 线程清理。
    clear_child_tid: AtomicUsize,

    /// The head of the robust list
    /// robust 互斥锁链表头，用于进程崩溃时恢复互斥锁状态。
    robust_list_head: AtomicUsize,

    /// The thread-level signal manager
    /// 线程级信号管理器。
    pub signal: Arc<ThreadSignalManager>,

    /// Time manager
    ///
    /// This is assumed to be `Sync` because it's only borrowed mutably during
    /// context switches, which is exclusive to the current thread.
    /// 时间管理器，通过 `AssumeSync` 包装，仅在上下文切换时独占访问。
    pub time: AssumeSync<RefCell<TimeManager>>,

    /// Linux-compatible scheduler policy reported by scheduler syscalls.
    sched_policy: AtomicU32,

    /// Linux-compatible realtime priority reported by scheduler syscalls.
    sched_priority: AtomicI32,

    /// Linux SCHED_DEADLINE runtime reported by sched_getattr.
    sched_runtime: AtomicUsize,

    /// Linux SCHED_DEADLINE deadline reported by sched_getattr.
    sched_deadline: AtomicUsize,

    /// Linux SCHED_DEADLINE period reported by sched_getattr.
    sched_period: AtomicUsize,

    /// Linux nice value reported by getpriority/setpriority.
    nice: AtomicI32,

    /// Linux execution domain reported by personality(2).
    personality: AtomicUsize,

    /// Signal sent to this process when its parent dies, for prctl(PR_*_PDEATHSIG).
    parent_death_signal: AtomicU32,

    /// Linux timer slack in nanoseconds reported by prctl(PR_*_TIMERSLACK).
    timer_slack_ns: AtomicUsize,

    /// Default Linux timer slack used when PR_SET_TIMERSLACK resets with value 0.
    default_timer_slack_ns: AtomicUsize,

    /// The OOM score adjustment value.
    /// OOM 评分调整值。
    oom_score_adj: AtomicI32,

    /// Ready to exit
    /// 准备退出标志。
    pub exit: Arc<AtomicBool>,

    /// Indicates whether the thread is currently accessing user memory.
    /// 标记线程是否正在访问用户态内存。
    accessing_user_memory: AtomicBool,

    /// Self exit event
    /// 自身退出事件通知。
    pub exit_event: Arc<PollSet>,
}

impl Thread {
    /// Create a new [`Thread`].
    pub fn new(tid: u32, proc_data: Arc<ProcessData>) -> Box<Self> {
        let pid = proc_data.proc.pid();
        Box::new(Thread {
            signal: ThreadSignalManager::new(tid, proc_data.signal.clone()),
            proc_data,
            pid,
            clear_child_tid: AtomicUsize::new(0),
            robust_list_head: AtomicUsize::new(0),
            time: AssumeSync(RefCell::new(TimeManager::new())),
            sched_policy: AtomicU32::new(SCHED_NORMAL),
            sched_priority: AtomicI32::new(0),
            sched_runtime: AtomicUsize::new(0),
            sched_deadline: AtomicUsize::new(0),
            sched_period: AtomicUsize::new(0),
            nice: AtomicI32::new(0),
            personality: AtomicUsize::new(0),
            parent_death_signal: AtomicU32::new(0),
            timer_slack_ns: AtomicUsize::new(50_000),
            default_timer_slack_ns: AtomicUsize::new(50_000),
            exit: Arc::new(AtomicBool::new(false)),
            oom_score_adj: AtomicI32::new(200),
            accessing_user_memory: AtomicBool::new(false),
            exit_event: Arc::default(),
        })
    }

    /// Get the clear child tid field.
    pub fn clear_child_tid(&self) -> usize {
        self.clear_child_tid.load(Ordering::Relaxed)
    }

    /// Set the clear child tid field.
    pub fn set_clear_child_tid(&self, clear_child_tid: usize) {
        self.clear_child_tid
            .store(clear_child_tid, Ordering::Relaxed);
    }

    /// Get the robust list head.
    pub fn robust_list_head(&self) -> usize {
        self.robust_list_head.load(Ordering::SeqCst)
    }

    /// Set the robust list head.
    pub fn set_robust_list_head(&self, robust_list_head: usize) {
        self.robust_list_head
            .store(robust_list_head, Ordering::SeqCst);
    }

    /// Get the oom score adjustment value.
    pub fn oom_score_adj(&self) -> i32 {
        self.oom_score_adj.load(Ordering::SeqCst)
    }

    /// Set the oom score adjustment value.
    pub fn set_oom_score_adj(&self, value: i32) {
        self.oom_score_adj.store(value, Ordering::SeqCst);
    }

    /// Check if the thread is ready to exit.
    pub fn pending_exit(&self) -> bool {
        self.exit.load(Ordering::Acquire)
    }

    /// Set the thread to exit.
    pub fn set_exit(&self) {
        self.exit.store(true, Ordering::Release);
    }

    /// Check if the thread is accessing user memory.
    pub fn is_accessing_user_memory(&self) -> bool {
        self.accessing_user_memory.load(Ordering::Acquire)
    }

    /// Set the accessing user memory flag.
    pub fn set_accessing_user_memory(&self, accessing: bool) {
        self.accessing_user_memory
            .store(accessing, Ordering::Release);
    }

    pub fn sched_policy(&self) -> u32 {
        self.sched_policy.load(Ordering::SeqCst)
    }

    pub fn sched_priority(&self) -> i32 {
        self.sched_priority.load(Ordering::SeqCst)
    }

    pub fn set_sched_param(&self, policy: u32, priority: i32) {
        self.sched_policy.store(policy, Ordering::SeqCst);
        self.sched_priority.store(priority, Ordering::SeqCst);
    }

    pub fn sched_deadline_params(&self) -> (u64, u64, u64) {
        (
            self.sched_runtime.load(Ordering::SeqCst) as u64,
            self.sched_deadline.load(Ordering::SeqCst) as u64,
            self.sched_period.load(Ordering::SeqCst) as u64,
        )
    }

    pub fn set_sched_deadline_params(&self, runtime: u64, deadline: u64, period: u64) {
        self.sched_runtime.store(runtime as usize, Ordering::SeqCst);
        self.sched_deadline.store(deadline as usize, Ordering::SeqCst);
        self.sched_period.store(period as usize, Ordering::SeqCst);
    }

    pub fn nice(&self) -> i32 {
        self.nice.load(Ordering::SeqCst)
    }

    pub fn set_nice(&self, value: i32) {
        self.nice.store(value.clamp(-20, 19), Ordering::SeqCst);
    }

    pub fn personality(&self) -> usize {
        self.personality.load(Ordering::SeqCst)
    }

    pub fn set_personality(&self, value: usize) {
        self.personality.store(value, Ordering::SeqCst);
    }

    pub fn parent_death_signal(&self) -> u32 {
        self.parent_death_signal.load(Ordering::SeqCst)
    }

    pub fn set_parent_death_signal(&self, value: u32) {
        self.parent_death_signal.store(value, Ordering::SeqCst);
    }

    pub fn timer_slack_ns(&self) -> usize {
        self.timer_slack_ns.load(Ordering::SeqCst)
    }

    pub fn set_timer_slack_ns(&self, value: usize) {
        self.timer_slack_ns.store(value, Ordering::SeqCst);
    }

    pub fn default_timer_slack_ns(&self) -> usize {
        self.default_timer_slack_ns.load(Ordering::SeqCst)
    }

    pub fn set_default_timer_slack_ns(&self, value: usize) {
        self.default_timer_slack_ns.store(value, Ordering::SeqCst);
    }
}

#[extern_trait]
impl TaskExt for Box<Thread> {
    fn on_enter(&self) {
        let scope = self.proc_data.scope.read();
        unsafe { ActiveScope::set(&scope) };
        core::mem::forget(scope);
    }

    fn on_leave(&self) {
        ActiveScope::set_global();
        unsafe { self.proc_data.scope.force_read_decrement() };
    }
}

/// Helper trait to access the thread from a task.
/// 从 task 获取 thread 的辅助 trait。
pub trait AsThread {
    /// Try to get the thread from the task.
    /// 尝试从 task 获取 thread，内核 task 返回 None。
    fn try_as_thread(&self) -> Option<&Thread>;

    /// Get the thread from the task, panicking if it is a kernel task.
    /// 获取 thread，如果是内核 task 则 panic。
    fn as_thread(&self) -> &Thread {
        self.try_as_thread().expect("kernel task")
    }
}

impl AsThread for TaskInner {
    fn try_as_thread(&self) -> Option<&Thread> {
        self.task_ext()
            .map(|ext| ext.downcast_ref::<Box<Thread>>().as_ref())
    }
}

/// [`Process`]-shared data.
/// 进程级共享数据。
///
/// 包含同一进程内所有线程共享的状态，如地址空间、信号管理器、资源限制等。
pub struct ProcessData {
    /// The process.
    /// 进程对象。
    pub proc: Arc<Process>,
    /// The executable path
    /// 可执行文件路径。
    pub exe_path: RwLock<String>,
    /// The command line arguments
    /// 命令行参数。
    pub cmdline: RwLock<Arc<Vec<String>>>,
    /// The virtual memory address space.
    /// 虚拟内存地址空间。
    // TODO: scopify
    pub aspace: Arc<Mutex<AddrSpace>>,
    /// The resource scope
    /// 资源作用域。
    pub scope: RwLock<Scope>,
    /// The user heap top
    /// 用户堆内存顶端地址。
    heap_top: AtomicUsize,

    /// The resource limits
    /// 资源限制。
    pub rlim: RwLock<Rlimits>,

    /// The child exit wait event
    /// 子进程退出等待事件。
    pub child_exit_event: Arc<PollSet>,
    /// Self exit event
    /// 自身退出事件。
    pub exit_event: Arc<PollSet>,
    /// Job-control stop wait event.
    pub stopped_event: Arc<PollSet>,
    /// The exit signal of the thread
    /// 线程退出时发送给父进程的信号。
    pub exit_signal: Option<Signo>,

    /// The process signal manager
    /// 进程级信号管理器。
    pub signal: Arc<ProcessSignalManager>,

    /// The futex table.
    /// futex 表，用于管理线程间的快速同步。
    futex_table: Arc<FutexTable>,

    /// The default mask for file permissions.
    /// 文件权限默认掩码。
    umask: AtomicU32,

    uid: AtomicU32,
    euid: AtomicU32,
    suid: AtomicU32,
    gid: AtomicU32,
    egid: AtomicU32,
    sgid: AtomicU32,
    fsuid: AtomicU32,
    fsgid: AtomicU32,
    groups: RwLock<(usize, [u32; 32])>,
    capabilities: RwLock<Capabilities>,
    cpu_limit_signal_sent: AtomicBool,
    did_exec: AtomicBool,
    child_wait_state: Mutex<ChildWaitState>,
    uts_state: Mutex<Option<UtsState>>,
    child_utime_ticks: AtomicUsize,
    child_stime_ticks: AtomicUsize,
    times_base_ticks: Mutex<Option<(usize, usize)>>,
}

#[derive(Clone, Copy)]
pub struct Capabilities {
    pub effective: u32,
    pub permitted: u32,
    pub inheritable: u32,
    pub bounding: u32,
}

#[derive(Clone, Copy)]
pub struct UtsState {
    pub nodename: [core::ffi::c_char; 65],
    pub domainname: [core::ffi::c_char; 65],
}

#[derive(Clone, Copy, Default)]
pub struct ChildWaitState {
    pub stopped: Option<u8>,
    pub continued: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            effective: u32::MAX,
            permitted: u32::MAX,
            inheritable: u32::MAX,
            bounding: u32::MAX,
        }
    }
}

impl ProcessData {
    /// Create a new [`ProcessData`].
    pub fn new(
        proc: Arc<Process>,
        exe_path: String,
        cmdline: Arc<Vec<String>>,
        aspace: Arc<Mutex<AddrSpace>>,
        signal_actions: Arc<SpinNoIrq<SignalActions>>,
        exit_signal: Option<Signo>,
    ) -> Arc<Self> {
        Arc::new(Self {
            proc,
            exe_path: RwLock::new(exe_path),
            cmdline: RwLock::new(cmdline),
            aspace,
            scope: RwLock::new(Scope::new()),
            heap_top: AtomicUsize::new(crate::config::USER_HEAP_BASE),

            rlim: RwLock::default(),

            child_exit_event: Arc::default(),
            exit_event: Arc::default(),
            stopped_event: Arc::default(),
            exit_signal,

            signal: Arc::new(ProcessSignalManager::new(
                signal_actions,
                crate::config::SIGNAL_TRAMPOLINE,
            )),

            futex_table: Arc::new(FutexTable::new()),

            umask: AtomicU32::new(0o022),
            uid: AtomicU32::new(0),
            euid: AtomicU32::new(0),
            suid: AtomicU32::new(0),
            gid: AtomicU32::new(0),
            egid: AtomicU32::new(0),
            sgid: AtomicU32::new(0),
            fsuid: AtomicU32::new(0),
            fsgid: AtomicU32::new(0),
            groups: RwLock::new((1, [0; 32])),
            capabilities: RwLock::new(Capabilities::default()),
            cpu_limit_signal_sent: AtomicBool::new(false),
            did_exec: AtomicBool::new(false),
            child_wait_state: Mutex::new(ChildWaitState::default()),
            uts_state: Mutex::new(None),
            child_utime_ticks: AtomicUsize::new(0),
            child_stime_ticks: AtomicUsize::new(0),
            times_base_ticks: Mutex::new(None),
        })
    }

    /// Get the top address of the user heap.
    pub fn get_heap_top(&self) -> usize {
        self.heap_top.load(Ordering::Acquire)
    }

    /// Set the top address of the user heap.
    pub fn set_heap_top(&self, top: usize) {
        self.heap_top.store(top, Ordering::Release)
    }

    /// Linux manual: A "clone" child is one which delivers no signal, or a
    /// signal other than SIGCHLD to its parent upon termination.
    pub fn is_clone_child(&self) -> bool {
        self.exit_signal != Some(Signo::SIGCHLD)
    }

    /// Get the umask.
    pub fn umask(&self) -> u32 {
        self.umask.load(Ordering::SeqCst)
    }

    /// Set the umask.
    pub fn set_umask(&self, umask: u32) {
        self.umask.store(umask, Ordering::SeqCst);
    }

    /// Set the umask and return the old value.
    pub fn replace_umask(&self, umask: u32) -> u32 {
        self.umask.swap(umask, Ordering::SeqCst)
    }

    pub fn ids(&self) -> (u32, u32, u32, u32, u32, u32) {
        (
            self.uid.load(Ordering::SeqCst),
            self.euid.load(Ordering::SeqCst),
            self.suid.load(Ordering::SeqCst),
            self.gid.load(Ordering::SeqCst),
            self.egid.load(Ordering::SeqCst),
            self.sgid.load(Ordering::SeqCst),
        )
    }

    pub fn set_uid(&self, uid: u32) {
        self.uid.store(uid, Ordering::SeqCst);
        self.euid.store(uid, Ordering::SeqCst);
        self.suid.store(uid, Ordering::SeqCst);
        self.fsuid.store(uid, Ordering::SeqCst);
    }

    pub fn set_gid(&self, gid: u32) {
        self.gid.store(gid, Ordering::SeqCst);
        self.egid.store(gid, Ordering::SeqCst);
        self.sgid.store(gid, Ordering::SeqCst);
        self.fsgid.store(gid, Ordering::SeqCst);
    }

    pub fn set_resuid(&self, ruid: Option<u32>, euid: Option<u32>, suid: Option<u32>) {
        if let Some(ruid) = ruid {
            self.uid.store(ruid, Ordering::SeqCst);
        }
        if let Some(euid) = euid {
            self.euid.store(euid, Ordering::SeqCst);
            self.fsuid.store(euid, Ordering::SeqCst);
        }
        if let Some(suid) = suid {
            self.suid.store(suid, Ordering::SeqCst);
        }
    }

    pub fn set_resgid(&self, rgid: Option<u32>, egid: Option<u32>, sgid: Option<u32>) {
        if let Some(rgid) = rgid {
            self.gid.store(rgid, Ordering::SeqCst);
        }
        if let Some(egid) = egid {
            self.egid.store(egid, Ordering::SeqCst);
            self.fsgid.store(egid, Ordering::SeqCst);
        }
        if let Some(sgid) = sgid {
            self.sgid.store(sgid, Ordering::SeqCst);
        }
    }

    pub fn fsids(&self) -> (u32, u32) {
        (
            self.fsuid.load(Ordering::SeqCst),
            self.fsgid.load(Ordering::SeqCst),
        )
    }

    pub fn set_fsuid(&self, fsuid: u32) -> u32 {
        self.fsuid.swap(fsuid, Ordering::SeqCst)
    }

    pub fn set_fsgid(&self, fsgid: u32) -> u32 {
        self.fsgid.swap(fsgid, Ordering::SeqCst)
    }

    pub fn groups(&self) -> (usize, [u32; 32]) {
        *self.groups.read()
    }

    pub fn set_groups(&self, groups: &[u32]) {
        let mut stored = [0; 32];
        stored[..groups.len()].copy_from_slice(groups);
        *self.groups.write() = (groups.len(), stored);
    }

    pub fn capabilities(&self) -> Capabilities {
        *self.capabilities.read()
    }

    pub fn set_capabilities(&self, capabilities: Capabilities) {
        *self.capabilities.write() = capabilities;
    }

    pub fn has_capability(&self, cap: u32) -> bool {
        cap < 32 && (self.capabilities.read().effective & (1 << cap)) != 0
    }

    pub fn mark_cpu_limit_signal_sent(&self) -> bool {
        self.cpu_limit_signal_sent.swap(true, Ordering::SeqCst)
    }

    pub fn reset_cpu_limit_signal(&self) {
        self.cpu_limit_signal_sent.store(false, Ordering::SeqCst);
    }

    pub fn did_exec(&self) -> bool {
        self.did_exec.load(Ordering::SeqCst)
    }

    pub fn mark_exec(&self) {
        self.did_exec.store(true, Ordering::SeqCst);
    }

    pub fn child_wait_state(&self) -> ChildWaitState {
        *self.child_wait_state.lock()
    }

    pub fn mark_stopped(&self, signo: Signo) {
        let mut state = self.child_wait_state.lock();
        state.stopped = Some(signo as u8);
        state.continued = false;
    }

    pub fn mark_continued(&self) {
        let mut state = self.child_wait_state.lock();
        state.stopped = None;
        state.continued = true;
        self.stopped_event.wake();
    }

    pub fn consume_stopped(&self) -> Option<u8> {
        self.child_wait_state.lock().stopped.take()
    }

    pub fn consume_continued(&self) -> bool {
        core::mem::take(&mut self.child_wait_state.lock().continued)
    }

    pub fn uts_state(&self) -> Option<UtsState> {
        *self.uts_state.lock()
    }

    pub fn set_uts_state(&self, uts_state: UtsState) {
        *self.uts_state.lock() = Some(uts_state);
    }

    pub fn child_times(&self) -> (usize, usize) {
        (
            self.child_utime_ticks.load(Ordering::SeqCst),
            self.child_stime_ticks.load(Ordering::SeqCst),
        )
    }

    pub fn add_child_times(&self, utime_ticks: usize, stime_ticks: usize) {
        self.child_utime_ticks
            .fetch_add(utime_ticks.max(1), Ordering::SeqCst);
        self.child_stime_ticks
            .fetch_add(stime_ticks.max(1), Ordering::SeqCst);
    }

    pub fn normalize_times(&self, utime_ticks: usize, stime_ticks: usize) -> (usize, usize) {
        let mut base = self.times_base_ticks.lock();
        let (base_utime, base_stime) = *base.get_or_insert((utime_ticks, stime_ticks));
        (
            utime_ticks.saturating_sub(base_utime),
            stime_ticks.saturating_sub(base_stime),
        )
    }
}
