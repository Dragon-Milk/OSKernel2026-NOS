# NOS 初赛开发文档

- [NOS 初赛开发文档](#nos-初赛开发文档)
  - [1. 概述](#1-概述)
  - [2. 与 StarryOS 基座的关系](#2-与-starryos-基座的关系)
  - [3. NOS 设计与实现](#3-nos-设计与实现)
    - [3.1 启动流程与评测调度](#31-启动流程与评测调度)
    - [3.2 进程与线程管理](#32-进程与线程管理)
    - [3.3 内存管理](#33-内存管理)
    - [3.4 文件系统与文件描述符](#34-文件系统与文件描述符)
    - [3.5 伪文件系统与设备文件](#35-伪文件系统与设备文件)
    - [3.6 系统调用兼容层](#36-系统调用兼容层)
    - [3.7 IPC 与网络支持](#37-ipc-与网络支持)
    - [3.8 双架构构建与平台适配](#38-双架构构建与平台适配)
  - [4. 初赛阶段评测支持情况](#4-初赛阶段评测支持情况)
  - [5. 总结与展望](#5-总结与展望)
  - [6. 参考与开源说明](#6-参考与开源说明)

## 1. 概述

NOS 是一个使用 Rust 编写的 `no_std` 宏内核项目，基于 StarryOS 继续开发。当前仓库的内核 crate 名为 `starry-kernel`，底层使用 ArceOS 的运行时、任务、内存、文件系统、网络和硬件抽象组件，在其上组织 Linux 兼容进程模型、地址空间、文件对象、伪文件系统和系统调用。

初赛阶段的工作目标不是重新设计一套与基座无关的内核，而是在可构建、可启动的 StarryOS 主体框架上补齐比赛镜像所需的运行环境和 Linux 边界语义。当前代码面向 RISC-V 64 与 LoongArch 64 两种目标，并在用户态 init 中提供 basic、busybox、cyclictest、libc-test、iozone、lmbench、LTP 等测试的分组入口。

本文使用知识图谱中的 `entry/init`、`task`、`mm`、`file`、`pseudofs`、`syscall` 和 `make` 模块划分作为阅读导航，但具体描述、代码片段和限制均以当前仓库源码为准。本文不把目录存在等同于功能完整，也不根据测试列表推断测试已经通过。

从当前代码看，NOS 的主调用关系可以概括为：

1. 根 Makefile 分别生成 RISC-V 与 LoongArch 内核；
2. `src/init/main.rs` 将测试脚本、LTP case 列表和构建参数嵌入 init 程序；
3. `entry::init` 挂载伪文件系统、加载首个用户程序并创建 init 进程；
4. 用户态通过 syscall 进入 `handle_syscall`，再分发到任务、内存、文件、IPC、网络等子模块；
5. 文件、地址空间、信号和资源限制等进程级状态由 `ProcessData` 统一持有，线程私有状态由 `Thread` 持有。

## 2. 与 StarryOS 基座的关系

仓库的 `README.md` 明确说明 NOS 基于 StarryOS 开发；`src/Cargo.toml` 仍保留 StarryOS 的项目元数据，内核依赖 `axruntime`、`axtask`、`axfs`、`axnet`、`starry-process`、`starry-signal`、`starry-vm` 等基座组件。因此，以下主体应理解为继承并继续扩展，而不是 NOS 从零实现：

- ArceOS 的启动运行时、任务调度器、页表和物理内存接口；
- `axfs-ng` VFS 与 ext4 文件系统接入；
- `axnet-ng` 网络栈和 socket 底层对象；
- `starry-process`、`starry-signal` 提供的进程、进程组、会话和信号基础模型；
- QEMU 平台抽象、设备驱动框架及 Cargo workspace 的主体组织。

当前仓库没有保留一份可与 NOS 逐文件对照的“未修改 StarryOS 基座源码”。`reference/` 中主要是许可证、NOTICE 和来源说明，不能据此进行可靠的源码 diff。因此，本文只把下列能由当前源码、开发日志或提交历史交叉确认的内容描述为初赛阶段的新增、重构或重点修补：

- 根目录双架构提交构建入口、离线 vendor 准备与参数转发；
- 可按 profile、libc、类别、批次或 case 列表运行的 init/LTP 调度；
- 脚本解释器与 BusyBox applet 的运行时回退；
- COW 页引用计数与 fork/clone 相关状态继承修补；
- 文件权限、路径解析、文件锁、扩展属性、inode flags 等 Linux 文件语义适配；
- SysV IPC、socket 参数和错误路径、进程身份与 wait/signal 等 syscall 兼容修补；
- 为比赛程序补充的 procfs、devfs、tmpfs、`/sys` 节点和设备节点。

`src/vendor/` 是离线构建所需的第三方依赖源码集合。将依赖随仓库提交是构建方案的一部分，但这些依赖本身不应计作 NOS 团队原创实现。

## 3. NOS 设计与实现

### 3.1 启动流程与评测调度

启动模块解决两个问题：一是从内核初始化过渡到第一个用户进程；二是在比赛磁盘镜像中选择可用 shell，并按构建参数运行目标测试。init 程序在编译期嵌入脚本和 LTP 列表，避免依赖根文件系统额外提供调度脚本。

```rust
const INIT_SCRIPT: &str = concat!(
    "LTP_SAFE_DATA='", include_str!("ltp-cases/ltp-safe.txt"), "'\n",
    include_str!("ltp-cases.sh"), "\n",
    include_str!("init.sh")
);

pub const CMDLINES: &[&[&str]] = &[
    &["/bin/sh", "-c", INIT_SCRIPT],
    &["/busybox", "sh", "-c", INIT_SCRIPT],
    &["/musl/busybox", "sh", "-c", INIT_SCRIPT],
    &["/glibc/busybox", "sh", "-c", INIT_SCRIPT],
];
```

来源：`src/init/main.rs::INIT_SCRIPT`、`src/init/main.rs::CMDLINES`

`entry::init` 依次挂载伪文件系统、创建用户地址空间、加载 init ELF、构造 `ProcessData` 与 `Thread`、初始化标准文件描述符，然后将 task 放入调度器。init 退出后，入口代码会卸载文件系统并 flush 根文件系统。当前实现只等待 init task，自身注释也说明尚未等待所有子进程退出。

评测调度通过环境变量选择 profile，并支持按 LTP 类别、批次和 libc 执行。批次 runner 会检查 case 文件、设置动态库环境、使用 timeout 限制单项运行时间，并把标准输入重定向到 `/dev/null`，避免测试程序读走 case 列表管道。

```sh
if [ "$LTP_BATCH" = "all" ]; then
    batches="$(ltp_batch_ids "$LTP_CATEGORY")" || {
        echo "[LTP-BATCH-ERROR] category not found: $LTP_CATEGORY"
        return
    }
else
    batches="$LTP_BATCH"
fi

for batch in $batches; do
    run_ltp_one_batch_libc "$libc" "$batch"
done
```

来源：`src/init/init.sh::run_ltp_batch_libc`

`src/init/ltp-cases/` 将 process、fs、mm-ipc、storage 等 case 分批保存，`ltp-cases.sh` 提供类别和批次到 case 名称的映射。对于已知可能阻塞或污染后续运行的 case，当前 runner 还包含 timeout、skip 和残留进程清理逻辑。这些机制用于缩小复现范围和保护后续测试，不代表被跳过功能已经实现。

初赛作用：启动与调度层先保证测试脚本能够找到解释器、正确设置 glibc/musl 环境，并允许单批次、单 case 回归。这样可以把“测试没有真正启动”和“内核语义不兼容”区分开。

### 3.2 进程与线程管理

当前代码没有采用参考文档中的 `ProcessControlBlock`/`TaskControlBlock` 命名，而是在 ArceOS `TaskInner` 上挂接 NOS 的 `Thread` 扩展。`Thread` 保存线程私有的信号、定时器、调度兼容字段和退出状态，并通过 `Arc<ProcessData>` 共享进程资源。

```rust
pub struct Thread {
    pub proc_data: Arc<ProcessData>,
    pub pid: u32,
    clear_child_tid: AtomicUsize,
    robust_list_head: AtomicUsize,
    pub signal: Arc<ThreadSignalManager>,
    pub time: AssumeSync<RefCell<TimeManager>>,
    sched_policy: AtomicU32,
    sched_priority: AtomicI32,
    nice: AtomicI32,
    personality: AtomicUsize,
    pub exit: Arc<AtomicBool>,
    pub exit_event: Arc<PollSet>,
}
```

来源：`src/kernel/src/task/mod.rs::Thread`（省略部分同类字段）

进程级结构保存地址空间、资源作用域、资源限制、凭据、进程信号管理器、futex 表和子进程等待事件。文件系统上下文和文件描述符表通过 `Scope` 进入线程运行时的 active scope，同一进程内的线程可以共享这些资源。

```rust
pub struct ProcessData {
    pub proc: Arc<Process>,
    pub exe_path: RwLock<String>,
    pub cmdline: RwLock<Arc<Vec<String>>>,
    pub aspace: Arc<Mutex<AddrSpace>>,
    pub scope: RwLock<Scope>,
    heap_top: AtomicUsize,
    pub rlim: RwLock<Rlimits>,
    pub child_exit_event: Arc<PollSet>,
    pub signal: Arc<ProcessSignalManager>,
    futex_table: Arc<FutexTable>,
    umask: AtomicU32,
    uid: AtomicU32,
    euid: AtomicU32,
    gid: AtomicU32,
    egid: AtomicU32,
}
```

来源：`src/kernel/src/task/mod.rs::ProcessData`（省略部分身份与统计字段）

`clone` 根据 flags 决定地址空间、文件描述符表、文件系统上下文和信号处理表是共享还是复制。新线程创建后还会继承调度策略、优先级、nice、personality 和 timer slack：

```rust
let thr = Thread::new(tid, new_proc_data.clone());
let old_thread = curr.as_thread();
thr.set_sched_param(old_thread.sched_policy(), old_thread.sched_priority());
let (runtime, deadline, period) = old_thread.sched_deadline_params();
thr.set_sched_deadline_params(runtime, deadline, period);
thr.set_nice(old_thread.nice());
thr.set_personality(old_thread.personality());
thr.set_timer_slack_ns(old_thread.timer_slack_ns());
```

来源：`src/kernel/src/syscall/task/clone.rs::CloneArgs::do_clone`

全局任务表使用弱引用登记 TID、PID、进程组和会话，供 wait、kill、pidfd 和 procfs 查询；退出路径会处理 `clear_child_tid`、robust futex、记录锁释放、父进程唤醒和孤儿进程回收。

初赛作用：这组结构直接承载 clone/clone3、execve、wait4/waitid、uid/gid、signal、futex、pidfd 和调度参数相关测试。需要注意，当前 `set_sched_param` 主要维护 Linux 兼容字段，底层 Cargo feature 仍为 `sched-rr`，不能仅凭 syscall 能返回参数就声称已经具备完整 Linux 实时调度语义。

### 3.3 内存管理

内存模块在基座页表和内存集合组件上封装用户地址空间。`AddrSpace` 同时维护合法虚拟地址范围、VMA 集合和页表，提供查找空闲区、映射、取消映射、权限修改、缺页处理和 fork 克隆接口。

```rust
pub struct AddrSpace {
    va_range: VirtAddrRange,
    areas: MemorySet<Backend>,
    pt: PageTable,
}

pub fn map(
    &mut self,
    start: VirtAddr,
    size: usize,
    flags: MappingFlags,
    populate: bool,
    backend: Backend,
) -> AxResult {
    self.validate_region(start, size)?;
    let area = MemoryArea::new(start, size, flags, backend);
    self.areas.map(area, &mut self.pt, false)?;
    if populate {
        self.populate_area(start, size, flags)?;
    }
    Ok(())
}
```

来源：`src/kernel/src/mm/aspace/mod.rs::AddrSpace`、`src/kernel/src/mm/aspace/mod.rs::AddrSpace::map`

映射后端通过 `BackendOps` 统一线性映射、匿名/私有 COW、文件映射和共享页。`mmap` 根据 `MAP_PRIVATE`、`MAP_SHARED`、文件描述符和设备映射类型选择后端；缺页时由后端按需分配或装入页面。

```rust
#[enum_dispatch]
pub trait BackendOps {
    fn page_size(&self) -> PageSize;
    fn map(&self, range: VirtAddrRange, flags: MappingFlags, pt: &mut PageTableCursor)
        -> AxResult;
    fn unmap(&self, range: VirtAddrRange, pt: &mut PageTableCursor) -> AxResult;
    fn protect(
        &self,
        range: VirtAddrRange,
        new_flags: MappingFlags,
        pt: &mut PageTableCursor,
    ) -> AxResult;
    fn populate(
        &self,
        range: VirtAddrRange,
        flags: MappingFlags,
        access_flags: MappingFlags,
        pt: &mut PageTableCursor,
    ) -> AxResult<(usize, Option<PopulateCallback>)>;
}
```

来源：`src/kernel/src/mm/aspace/backend/mod.rs::BackendOps`（省略 `clone_map` 签名）

私有映射使用 COW。当前引用计数为 `u16`，fork 克隆时先去掉写权限，再给父子页表映射同一物理页；写缺页时，单引用页面只恢复权限，多引用页面复制到新页。

```rust
struct FrameRefCnt(u16);

struct FrameTableRefCount {
    table: BTreeMap<PhysAddr, Arc<SpinNoIrq<FrameRefCnt>>>,
}

// clone_map 中：
frame.0 += 1;
if frame.0 == u16::MAX {
    warn!("frame reference count overflow");
    return Err(AxError::BadAddress);
}
old_pt.protect(*vaddr, cow_flags)?;
new_pt.map(*vaddr, *paddr, self.size, cow_flags)?;
```

来源：`src/kernel/src/mm/aspace/backend/cow.rs::FrameRefCnt`、`src/kernel/src/mm/aspace/backend/cow.rs::CowBackend::clone_map`

用户指针访问由 `UserPtr`、`UserConstPtr` 和 `VmIo` 封装。访问前检查用户地址范围和 VMA 权限，需要时先 populate 页面；实际 copy 期间设置 `accessing_user_memory`，让内核态用户拷贝触发的缺页能够转回当前进程地址空间处理。

初赛作用：该模块对应 brk、mmap/munmap/mprotect/mremap、mincore、ELF 装载、fork COW、用户指针错误检查和共享内存映射。初赛开发日志可以确认 COW 引用计数扩宽是围绕大量 fork 场景的修补，但地址空间主体框架仍来自基座组件。

### 3.4 文件系统与文件描述符

文件层使用 `FileLike` 把普通文件、目录、pipe/FIFO、socket、eventfd、epoll、signalfd 和 pidfd 放入同一文件描述符表。接口提供读写、状态、poll、ioctl、非阻塞模式和文件锁所需的 inode/OFD 身份。

```rust
pub trait FileLike: Pollable + DowncastSync {
    fn read(&self, _dst: &mut IoDst) -> AxResult<usize>;
    fn write(&self, _src: &mut IoSrc) -> AxResult<usize>;
    fn stat(&self) -> AxResult<Kstat>;
    fn path(&self) -> Cow<'_, str>;
    fn ioctl(&self, _cmd: u32, _arg: usize) -> AxResult<usize>;
    fn nonblocking(&self) -> bool;
    fn set_nonblocking(&self, _nonblocking: bool) -> AxResult;
    fn inode_key(&self) -> Option<record_lock::InodeKey>;
    fn file_position(&self) -> u64;
    fn ofd_owner(&self) -> u64;
}

#[derive(Clone)]
pub struct FileDescriptor {
    pub inner: Arc<dyn FileLike>,
    pub cloexec: bool,
}
```

来源：`src/kernel/src/file/mod.rs::FileLike`、`src/kernel/src/file/mod.rs::FileDescriptor`（省略带默认实现的方法体）

描述符表是进程 scope 中的 `FlattenObjects`，上限同时受编译期容量和 `RLIMIT_NOFILE` 约束。`dup`/fork 共享 `Arc<dyn FileLike>`，所以普通文件对象中单独分配的 `ofd_id` 也随 open file description 共享。

```rust
scope_local::scope_local! {
    pub static FD_TABLE:
        Arc<RwLock<FlattenObjects<FileDescriptor, AX_FILE_LIMIT>>> = Arc::default();
}

pub fn add_file_like(f: Arc<dyn FileLike>, cloexec: bool) -> AxResult<c_int> {
    let max_nofile = current().as_thread().proc_data.rlim.read()[RLIMIT_NOFILE].current;
    let mut table = FD_TABLE.write();
    if table.count() as u64 >= max_nofile {
        return Err(AxError::TooManyOpenFiles);
    }
    let fd = FileDescriptor { inner: f, cloexec };
    Ok(table.add(fd).map_err(|_| AxError::TooManyOpenFiles)? as c_int)
}
```

来源：`src/kernel/src/file/mod.rs::FD_TABLE`、`src/kernel/src/file/mod.rs::add_file_like`

初赛阶段重点补充了记录锁和 BSD `flock`。记录锁以 `(device, inode)` 作为全局 inode 身份，区分进程级 POSIX owner 与 open-file-description 级 OFD owner；区间统一表示为 `[start, end)`，`None` 表示锁到 EOF。关闭 fd、最后一个 OFD 引用释放和进程退出分别触发不同的锁释放路径。

```rust
pub type InodeKey = (u64, u64);
pub type PosixOwner = Weak<ProcessData>;
pub type OfdOwner = u64;

#[derive(Debug, Clone)]
pub struct LockInterval {
    pub start: i64,
    pub end: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    Read,
    Write,
}
```

来源：`src/kernel/src/file/record_lock.rs::LockInterval`、`src/kernel/src/file/record_lock.rs::LockType`

此外，当前文件层还包含：

- pipe 与命名 FIFO 的共享环形缓冲、阻塞/非阻塞读写、`SIGPIPE` 和容量控制；
- epoll 的 interest 表、ready queue 与 waker 注册；
- eventfd 的 64 位计数器与 semaphore 模式；
- inode 级扩展属性兼容存储；
- immutable、append-only、nodump 等 inode flags；
- VFS 路径搜索、父目录权限、sticky bit、只读挂载与 `*at` 路径解析检查。

初赛作用：这些能力主要服务 fs、storage、busybox、iozone 和 LTP 中的 fd、fcntl、poll/epoll、xattr、权限和错误码测试。当前 xattr 和 inode flags 是内核侧兼容存储，不应等同于底层 ext4 已原生持久化这些元数据。

### 3.5 伪文件系统与设备文件

伪文件系统为用户程序提供运行时必需但不来自根 ext4 镜像的节点。启动时统一挂载 devfs、多个 tmpfs、procfs，并使用 tmpfs 组织最小 `/sys` 树。

```rust
pub fn mount_all() -> LinuxResult<()> {
    let fs = FS_CONTEXT.lock();
    mount_at(&fs, "/dev", dev::new_devfs())?;
    mount_at(&fs, "/dev/shm", tmp::MemoryFs::new())?;
    mount_at(&fs, "/tmp", tmp::MemoryFs::new())?;
    mount_at(&fs, "/var/tmp", tmp::MemoryFs::new())?;
    mount_at(&fs, "/proc", proc::new_procfs())?;
    mount_at(&fs, "/sys", tmp::MemoryFs::new())?;
    // 后续创建图形设备和 loop 设备所需的最小 /sys 节点。
    Ok(())
}
```

来源：`src/kernel/src/pseudofs/mod.rs::mount_all`（省略目录存在性检查和节点创建细节）

`MemoryFs` 使用 slab 保存 inode，目录项使用哈希表，普通文件内容交给页缓存管理；根目录权限为 `01777`，适合作为 `/tmp`、`/dev/shm` 和 `/var/tmp`。它实现 create、lookup、link、unlink、rename、exchange、symlink 和元数据更新等 VFS 操作。

```rust
pub struct MemoryFs {
    inodes: Mutex<Slab<Arc<Inode>>>,
    mount_flags: Mutex<u32>,
    root: Mutex<Option<DirEntry>>,
}

#[derive(Default)]
struct FileContent {
    length: Mutex<u64>,
    symlink: Mutex<Option<String>>,
}

#[derive(Default)]
struct DirContent {
    entries: Mutex<HashMap<FileName, InodeRef>>,
}
```

来源：`src/kernel/src/pseudofs/tmp.rs::MemoryFs`、`src/kernel/src/pseudofs/tmp.rs::FileContent`、`src/kernel/src/pseudofs/tmp.rs::DirContent`

procfs 通过不可缓存的动态目录把进程、线程和地址空间状态投影为 `/proc/<pid>`、`task`、`maps`、`fd` 等节点，同时提供测试程序会读取的部分系统文件。devfs 注册 `null`、`zero`、`full`、`random`、`urandom`、RTC、TTY、PTY、loop 等设备，部分输入、日志和内存跟踪节点由 feature 控制。

初赛作用：procfs 为进程观察、pidfd、路径和测试框架探测提供接口；tmpfs 支撑临时文件和 SysV/POSIX IPC 运行目录；devfs 提供 shell、LTP 和设备相关测试期望的设备节点。当前 `/sys` 和部分 `/proc/sys` 内容是面向兼容性的最小实现，其中存在固定值或“接受写入但不执行完整 Linux 行为”的节点，文档不将其描述为完整 sysfs/procfs。

### 3.6 系统调用兼容层

syscall 层按功能拆分为 fs、I/O 多路复用、IPC、mm、net、sync、task、signal、resources、sys 和 time。用户态循环从 `UserContext` 取得 syscall 号和参数，统一调用 Rust 函数并把 `AxError` 转换为 Linux 负 errno。

```rust
mod fs;
mod io_mpx;
mod ipc;
mod mm;
mod net;
mod resources;
mod signal;
mod sync;
mod sys;
mod task;
mod time;
```

来源：`src/kernel/src/syscall/mod.rs` 模块声明

```rust
pub fn handle_syscall(uctx: &mut UserContext) {
    let Some(sysno) = Sysno::new(uctx.sysno()) else {
        warn!("Invalid syscall number: {}", uctx.sysno());
        uctx.set_retval(-LinuxError::ENOSYS.code() as _);
        return;
    };

    let result = match sysno {
        Sysno::ioctl => sys_ioctl(uctx.arg0() as _, uctx.arg1() as _, uctx.arg2() as _),
        Sysno::mmap => sys_mmap(
            uctx.arg0(), uctx.arg1(), uctx.arg2() as _,
            uctx.arg3() as _, uctx.arg4() as _, uctx.arg5() as _,
        ),
        // 其余 syscall 按功能继续分发。
        _ => Err(AxError::Unsupported),
    };

    match result {
        Ok(ret) => uctx.set_retval(ret as _),
        Err(AxError::Interrupted) if should_restart_interrupted_syscall(sysno) => {
            restart_syscall(uctx);
        }
        Err(err) => uctx.set_retval(-LinuxError::from(err).code() as _),
    }
}
```

来源：`src/kernel/src/syscall/mod.rs::handle_syscall`（按原分发结构节选）

分发层还处理 `wait4` 的可重启 syscall 逻辑。未知 syscall 返回 `ENOSYS`，已识别但未实现的 syscall 进入 `Unsupported`。代码中还有一组 dummy fd 分支用于让部分探测型接口返回占位文件描述符，这种做法只提供有限兼容，不能表述为 io_uring、BPF、fanotify 等子系统已经实现。

初赛阶段的 syscall 工作主要集中在 Linux 边界语义：参数组合检查、用户指针错误、权限、资源生命周期、阻塞与唤醒、返回值和 errno。目录数量或 match 分支数量不能直接代表兼容程度；具体支持范围仍应以函数实现和实际测试结果为准。

### 3.7 IPC 与网络支持

IPC 与网络对象都复用文件描述符、用户指针和 poll 框架。匿名 pipe、FIFO、eventfd、signalfd、epoll 属于 fd 型 IPC；futex 位于任务同步层；System V 消息队列、共享内存和 semaphore 由 syscall IPC 模块管理。

eventfd 使用原子计数器保存状态，并用两个 `PollSet` 分别唤醒读端和写端。semaphore 模式每次读减一，普通模式一次读出并清空当前计数。

```rust
pub struct EventFd {
    count: AtomicU64,
    semaphore: bool,
    non_blocking: AtomicBool,
    poll_rx: PollSet,
    poll_tx: PollSet,
}

pub fn new(initval: u64, semaphore: bool) -> Arc<Self> {
    Arc::new(Self {
        count: AtomicU64::new(initval),
        semaphore,
        non_blocking: AtomicBool::new(false),
        poll_rx: PollSet::new(),
        poll_tx: PollSet::new(),
    })
}
```

来源：`src/kernel/src/file/event.rs::EventFd`、`src/kernel/src/file/event.rs::EventFd::new`

System V 共享内存把 key、shmid、内核对象和每个进程的映射地址分别建表。`ShmInner` 保存共享物理页、attach 区间、删除标志和 `shmid_ds`，`shmat` 最终通过 `Backend::new_shared` 映射到进程地址空间。

```rust
pub struct ShmManager {
    key_shmid: BiBTreeMap<i32, i32>,
    shmid_inner: BTreeMap<i32, Arc<Mutex<ShmInner>>>,
    pid_shmid_vaddr: BTreeMap<Pid, BiBTreeMap<i32, VirtAddr>>,
}
```

来源：`src/kernel/src/syscall/ipc/shm.rs::ShmManager`

socket 层把 `axnet::Socket` 包装为 `FileLike`，使 socket 可以使用普通 fd、read/write、非阻塞标志和 poll/epoll。syscall 子模块再实现 socket、bind、connect、listen、accept、send/recv、sendmsg/recvmsg、socket option 和地址转换。

```rust
pub struct Socket(pub SocketInner);

impl FileLike for Socket {
    fn read(&self, dst: &mut IoDst) -> AxResult<usize> {
        self.recv(dst, RecvOptions::default())
    }

    fn write(&self, src: &mut IoSrc) -> AxResult<usize> {
        self.send(src, SendOptions::default())
    }

    fn set_nonblocking(&self, nonblocking: bool) -> AxResult<()> {
        self.0.set_option(SetSocketOption::NonBlocking(&nonblocking))
    }
}
```

来源：`src/kernel/src/file/net.rs::Socket`、`src/kernel/src/file/net.rs::FileLike for Socket`（节选）

初赛作用：IPC 模块面向 LTP 的 semaphore、message queue、shared memory、futex、pipe 和 epoll case；网络模块面向 iperf、netperf 及 LTP socket case。开发日志显示团队使用单 case 列表定位 send/recv flag、mmsg、accept 错误路径等问题，但本文不据此声明网络或 IPC 测试已经全部通过。

### 3.8 双架构构建与平台适配

根目录 Makefile 是比赛提交入口。默认目标先准备离线 vendor，再分别调用 `src/Makefile` 构建 RISC-V 与 LoongArch，最终把 `kernel-rv` 和 `kernel-la` 复制到仓库根目录。

```make
.DEFAULT_GOAL := all
export CARGO_NET_OFFLINE := true

all: prepare-vendor kernel-rv kernel-la

kernel-rv: prepare-vendor
	@$(MAKE) -C $(SRC_DIR) $(SRC_MAKE_ARGS) OUT_CONFIG=$(RV_OUT_CONFIG) $@
	@cp $(SRC_DIR)/$@ $(ROOT_DIR)/$@

kernel-la: prepare-vendor
	@$(MAKE) -C $(SRC_DIR) $(SRC_MAKE_ARGS) OUT_CONFIG=$(LA_OUT_CONFIG) $@
	@cp $(SRC_DIR)/$@ $(ROOT_DIR)/$@
```

来源：`Makefile`

内层规则为两种架构选择不同 target、总线和链接脚本：RISC-V 使用 MMIO 与 `linker_riscv64-qemu-virt-contest.lds`，LoongArch 使用 PCI 与 `linker_loongarch64-qemu-contest.lds`。`platform.mk` 再通过架构选择对应的 axplat 平台包。

```make
kernel-rv:
	@$(MAKE) ARCH=riscv64 BUS=mmio LD_SCRIPT=$(RV_LD_SCRIPT) \
		OUT_CONFIG=$(RV_OUT_CONFIG) build
	@cp $(RV_ELF) $@

kernel-la:
	@$(MAKE) ARCH=loongarch64 BUS=pci LD_SCRIPT=$(LA_LD_SCRIPT) \
		OUT_CONFIG=$(LA_OUT_CONFIG) build
	@cp $(LA_ELF) $@
```

来源：`src/Makefile`

构建时还会把 `TEST_PROFILE`、`LTP_CATEGORY`、`LTP_BATCH`、`LTP_LIBC`、`LTP_TIMEOUT` 和 `LTP_CASE_LIST` 生成到 `src/init/env.rs`，再由 init 作为环境变量传入脚本。这使同一套源码可以生成不同测试入口，但提交版与性能分析版仍是不同 feature 组合：`perf` profile 会启用 `perf-profile` 和 trace 日志，不应作为默认提交构建。

初赛作用：双架构规则保证评测入口一致，离线 vendor 避免构建期访问包仓库，参数嵌入则把本地复现方式与最终内核镜像绑定。当前仓库还保留 x86_64、aarch64 配置代码，但根目录提交目标只构建 RISC-V 64 和 LoongArch 64，本文不据此声称另外两种架构也完成了初赛适配。

## 4. 初赛阶段评测支持情况

当前 init 脚本和开发日志表明，初赛阶段围绕以下测试进行了适配：

| 测试方向 | 当前仓库中的直接依据 | 保守说明 |
| --- | --- | --- |
| basic、busybox | shell 选择、脚本路径准备、BusyBox applet fallback | 说明测试可被调度，不代表所有 applet 或 syscall 通过 |
| cyclictest | 独立 profile、调度参数 syscall、定时器与 clone 继承代码 | 当前底层为 round-robin，不能表述为完整实时调度 |
| libc-test、lua、lmbench | profile、动态库路径、通用 runner | 部分 case 可能受超时、缺失语义或性能影响 |
| iozone、storage | iozone profile、sync/fsync、storage 分批与 skip 列表 | safe/diagnostic 分类包含隔离策略，不等于全量通过 |
| LTP process | 15 个批次及 task/signal/wait/credentials syscall | 批次存在只表示具备运行入口 |
| LTP fs | 13 个批次及权限、路径、fcntl、lock、xattr 修补 | 底层和兼容层仍有未实现或简化语义 |
| LTP mm-ipc | 9 个批次及 mmap、COW、SysV IPC 实现 | 高压力和异常路径仍需继续验证 |
| iperf、netperf、LTP net | 网络测试入口、socket 与 mmsg 等实现 | 网络 case 可能阻塞，当前主要依赖小范围回归 |

需要特别说明三点：

1. `init.sh` 中出现 START/END 输出、case 名称或 timeout，只能证明 runner 具备相应调度路径；
2. skip 列表用于隔离可能 panic、长时间阻塞或污染环境的 case，被跳过项不计作已支持；
3. 开发日志中的某次运行结果受当时提交、架构、libc、磁盘镜像和参数影响，本文不把这些结果写成当前版本的绝对结论。

因此，较准确的阶段性表述是：NOS 已建立双架构构建与细粒度评测调度框架，并围绕进程、内存、文件、IPC、网络和伪文件系统中的一批 Linux 兼容问题进行了实现与修补；各测试集合的最终通过情况仍需要在指定评测环境中重新运行确认。

## 5. 总结与展望

NOS 初赛阶段的主要工作，是在 StarryOS/ArceOS 主体框架上建立可重复定位问题的运行路径，并逐步把“syscall 存在”推进到参数、权限、错误码、阻塞和资源回收更接近 Linux 行为。当前代码已经形成较清楚的层次：

- `entry` 与 init 负责启动和测试组织；
- `Thread`/`ProcessData` 连接底层任务与 Linux 进程语义；
- `AddrSpace` 与多种 backend 组织用户 VMA、COW、文件和共享映射；
- `FileLike` 与 scope-local fd table 统一普通文件、IPC 和 socket；
- pseudofs 提供运行环境所需的动态节点；
- syscall 层按功能集中处理用户 ABI 和 errno。

后续工作应优先处理当前源码中可以直接看到的限制：

1. 让调度策略和优先级真正连接到底层调度器，而不只保存兼容字段；
2. 减少 dummy fd、固定 proc/sys 值和只接受写入的占位接口；
3. 完善 namespace、procfs/sysfs、tmpfs rename 原子性等简化实现；
4. 继续校验 mmap、共享内存、文件锁和 socket 在并发及异常退出下的生命周期；
5. 在 RISC-V/LoongArch、glibc/musl 组合中运行可复现回归，并把 skip 项与已确认问题分别维护；
6. 保持默认提交构建与性能诊断构建分离，避免调试日志和诊断 feature 影响评测。

这些工作应继续采用“小批次复现—最小修补—双架构回归”的方式推进，避免用扩大 skip 或无条件返回成功掩盖尚未实现的内核语义。

## 6. 参考与开源说明

NOS 基于 StarryOS 开发，遵循相关开源协议。项目来源、版权和许可说明见：

- `README.md`
- `reference/LICENSE`
- `reference/NOTICE`
- `reference/`

当前 `reference/LICENSE` 为 Apache License 2.0，`reference/NOTICE` 保留了原项目贡献者及后续修改者信息。仓库中的 vendor 依赖还可能带有各自许可证，分发或复用时应同时保留相应依赖的版权与许可文件。

本文参考了 `temp/初赛文档.md` 的“概述—模块设计与实现—总结展望”写法，但所有 NOS 结构体、函数、调用关系和限制均重新从当前仓库源码核对，没有沿用参考项目的 PCB、TCB、VFS 或 HAL 实现结论。
