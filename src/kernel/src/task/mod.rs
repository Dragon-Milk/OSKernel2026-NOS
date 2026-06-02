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

use axhal::uspace::UserContext;
use axpoll::PollSet;
use axsync::{Mutex, spin::SpinNoIrq};
use axtask::{TaskExt, TaskInner};
use extern_trait::extern_trait;
use scope_local::{ActiveScope, Scope};
use spin::RwLock;
use starry_process::Process;
use starry_signal::{
    Signo,
    api::{ProcessSignalManager, SignalActions, ThreadSignalManager},
};

pub use self::{futex::*, ops::*, resources::*, signal::*, stat::*, timer::*, user::*};
use crate::mm::AddrSpace;

#[derive(Debug, Clone, Copy)]
pub struct RestartSyscall {
    pub pre_syscall_context: UserContext,
    pub sysno: usize,
}

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

    restart_syscall: AssumeSync<RefCell<Option<RestartSyscall>>>,
}

impl Thread {
    /// Create a new [`Thread`].
    pub fn new(tid: u32, proc_data: Arc<ProcessData>) -> Box<Self> {
        Box::new(Thread {
            signal: ThreadSignalManager::new(tid, proc_data.signal.clone()),
            proc_data,
            clear_child_tid: AtomicUsize::new(0),
            robust_list_head: AtomicUsize::new(0),
            time: AssumeSync(RefCell::new(TimeManager::new())),
            exit: Arc::new(AtomicBool::new(false)),
            oom_score_adj: AtomicI32::new(200),
            accessing_user_memory: AtomicBool::new(false),
            exit_event: Arc::default(),
            restart_syscall: AssumeSync(RefCell::new(None)),
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

    pub fn set_restart_syscall(&self, restart: RestartSyscall) {
        *self.restart_syscall.borrow_mut() = Some(restart);
    }

    pub fn restart_syscall(&self) -> Option<RestartSyscall> {
        *self.restart_syscall.borrow()
    }

    pub fn clear_restart_syscall(&self) {
        *self.restart_syscall.borrow_mut() = None;
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
            exit_signal,

            signal: Arc::new(ProcessSignalManager::new(
                signal_actions,
                crate::config::SIGNAL_TRAMPOLINE,
            )),

            futex_table: Arc::new(FutexTable::new()),

            umask: AtomicU32::new(0o022),
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
}
