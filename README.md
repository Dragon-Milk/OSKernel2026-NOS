# NOS

NOS 是基于 [StarryOS](https://github.com/Starry-OS/StarryOS) 继续开发的 Linux 兼容操作系统内核。项目面向 2026 年操作系统设计赛内核实现赛道，当前评测入口主要支持 RISC-V 64 和 LoongArch64 两种 QEMU 平台，并面向 glibc 与 musl 两套用户态环境进行适配。

本项目遵循原项目及第三方依赖的相关开源协议。原始项目来源、许可声明和引用说明见 `reference/`。

## 快速开始

在比赛指定 Docker 环境或等价工具链环境中执行：

```bash
make all
```

`make all` 会构建两个评测入口内核：

```text
kernel-rv    # RISC-V 64 QEMU 平台内核
kernel-la    # LoongArch64 QEMU 平台内核
```

也可以按架构单独构建：

```bash
make kernel-rv
make kernel-la
make clean
```

## 项目概览

NOS 以 StarryOS/ArceOS 的内核框架为基础，围绕 Linux 兼容系统调用、用户程序加载、文件系统、内存管理、进程管理和测试调度继续扩展。内核能够从磁盘镜像中加载用户态程序，并通过 `init` 程序和启动脚本组织 basic、busybox、lua、libctest、lmbench、iozone、iperf、netperf、cyclictest、LTP 等测试场景。

当前实现重点包括：进程与线程生命周期管理、信号和定时器、futex 与同步原语、ELF 与脚本加载、虚拟地址空间管理、文件描述符与 VFS 抽象、管道和事件文件、procfs/devfs/sysfs/tmpfs 等伪文件系统、System V IPC、socket 网络接口，以及 epoll/poll/select 等 I/O 多路复用机制。

## 项目结构

```text
.
├── docs
│   └── prel                        # 初赛阶段文档、开发记录和评测说明
├── reference                       # StarryOS 来源、许可证、NOTICE 和参考资料
├── src                             # 内核源码、用户态 init、构建资源和工具
│   ├── init                        # no_std init 程序、init.sh 和测试调度脚本
│   ├── kernel                      # 内核主体 crate
│   │   └── src
│   │       ├── config              # 架构和平台相关配置
│   │       ├── file                # FileLike、FD 表、VFS、pipe、epoll、eventfd、socket 等文件抽象
│   │       ├── mm                  # 地址空间、页表、mmap、用户指针访问、ELF 加载
│   │       ├── pseudofs            # devfs、procfs、sysfs、tmpfs 等伪文件系统
│   │       ├── syscall             # Linux 兼容 syscall 分发和各类 syscall 实现
│   │       ├── task                # 进程、线程、调度、信号、futex、定时器和资源统计
│   │       ├── entry.rs            # 内核初始化和用户态 init 启动流程
│   │       ├── lib.rs              # kernel crate 根模块
│   │       └── time.rs             # 内核时间工具
│   ├── make                        # 构建规则、平台配置和 Makefile 片段
│   ├── scripts                     # 构建、运行、调试和测试辅助脚本
│   ├── target-check                # 目标平台检查或辅助构建配置
│   ├── tools                       # 镜像制作、测试处理和开发辅助工具
│   └── vendor                      # 随仓库提交的第三方依赖源码
├── COMMENTING.md                   # Rustdoc 注释模板和注释规范
├── README.md                       # 项目说明
└── Makefile                        # 比赛评测入口，提供 make all
```

## 核心模块

`src/kernel/src/lib.rs` 是内核 crate 的根模块，统一组织配置、入口、文件、内存、伪文件系统、系统调用、任务管理和时间等子系统。`entry.rs` 负责完成伪文件系统挂载、init 程序解析、用户地址空间创建、ELF 加载、进程线程初始化和标准 I/O 设置，是从内核态进入用户态的主要路径。

文件子系统以 `FileLike` 和文件描述符表为核心，向上承接 syscall，向下对接 VFS、pipe、eventfd、epoll、signalfd、pidfd、socket 和记录锁等实现。内存管理模块负责 VMA、页表、mmap/munmap、缺页处理、fork/clone 地址空间复制、文件映射、共享映射和 ELF 加载。

任务管理模块维护进程、线程、进程组、会话、信号、定时器、资源限制和退出回收流程。系统调用层按功能拆分为 fs、mm、task、net、ipc、io_mpx、sync、time、signal、resources 等子模块，集中提供 Linux 兼容接口。

伪文件系统模块负责挂载 `/dev`、`/proc`、`/sys`、tmpfs 等虚拟文件系统，提供标准设备节点和进程运行时信息。网络与 IPC 模块分别实现 socket 相关接口和 System V 消息队列、共享内存、信号量等传统 UNIX IPC 机制。

## 构建与评测说明

比赛评测会在项目根目录执行 `make all`。因此根目录 `Makefile` 是对外的稳定构建入口，`src/Makefile` 和 `src/make/` 负责进一步编排具体架构、平台配置、Cargo 构建和运行脚本。

用户态启动流程由 `src/init/main.rs` 和 `src/init/init.sh` 组成。`main.rs` 将启动脚本内联到 init 程序中，`init.sh` 负责设置环境、扫描或调度测试脚本、运行 LTP/性能测试，并在部分测试结束后执行清理逻辑，降低后台进程对后续测试的影响。

第三方 Rust 依赖和必要源码应随仓库提交，放在 `src/vendor/` 等源码目录中，避免评测环境构建时依赖网络下载。评测相关的隐藏目录过滤、工具链差异和磁盘镜像行为应以比赛说明为准。

## 开发约定

代码注释风格参考 `COMMENTING.md`。文档、开发记录和阶段性说明集中放在 `docs/prel/`。涉及原始 StarryOS、ArceOS 组件或其他第三方代码时，应在 `reference/` 中保留来源和许可说明。
