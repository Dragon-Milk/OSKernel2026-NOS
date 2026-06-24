use alloc::{string::ToString, sync::Arc, vec::Vec};

use axfs::FS_CONTEXT;
use axhal::uspace::UserContext;
use axsync::Mutex;
use axtask::{AxTaskExt, spawn_task};
use starry_process::{Pid, Process};

use crate::{
    file::FD_TABLE,
    mm::{copy_from_kernel, load_user_app, new_user_aspace_empty},
    pseudofs::{self, dev::tty::N_TTY},
    task::{ProcessData, Thread, add_task_to_table, new_user_task, spawn_alarm_task},
};

/// Initialize and run initproc.
/// 初始化并运行第一个用户态进程 initproc。
///
/// 本函数负责完成从内核初始化阶段到用户态执行阶段的切换：
/// 挂载伪文件系统、创建用户地址空间、加载 init ELF、构造用户上下文、
/// 初始化进程/线程数据结构、绑定控制终端并设置标准 I/O。
///
/// # 参数
///
/// - `cmdlines`: 候选启动命令列表。函数会选择第一个能在文件系统中解析成功的可执行文件。
/// - `envs`: 传递给 init 进程的环境变量。
///
/// # 不变量
///
/// - 伪文件系统必须在解析 init 路径和创建 stdio 前完成挂载；
/// - 用户地址空间必须包含必要的内核映射，保证 syscall/trap 能正常返回内核；
/// - task 的页表根必须设置为 `uspace.page_table_root()`；
/// - `ProcessData` 和 `Thread` 必须在 `spawn_task` 前挂入 `AxTaskExt`；
/// - init 进程必须加入全局任务表，后续 wait/kill/procfs 才能观察到它。
///
/// # 当前限制
///
/// 目前只等待 init task 退出，没有等待所有子进程退出。
/// 如果 init 派生了后台进程，文件系统清理可能早于所有进程结束。
pub fn init(cmdlines: &[&[&str]], envs: &[&str]) {
    // step0: 挂载伪文件系统
    pseudofs::mount_all().expect("Failed to mount pseudofs");
    spawn_alarm_task();

    // step1: 解析 init 路径
    let (cmdline, loc) = cmdlines
        .iter()
        .find_map(|cmdline| {
            FS_CONTEXT
                .lock()
                .resolve(cmdline[0])
                .ok()
                .map(|loc| (cmdline, loc))
        })
        .expect("Failed to resolve executable path");
    let args = cmdline
        .iter()
        .copied()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let envs = envs.iter().copied().map(str::to_string).collect::<Vec<_>>();
    let path = loc
        .absolute_path()
        .expect("Failed to get executable absolute path");
    let name = loc.name();

    // step2: 创建用户地址空间
    let mut uspace = new_user_aspace_empty()
        .and_then(|mut it| {
            copy_from_kernel(&mut it)?;
            Ok(it)
        })
        .expect("Failed to create user address space");

    // step3: 加载 init ELF
    let (entry_vaddr, ustack_top) = load_user_app(&mut uspace, None, &args, &envs)
        .unwrap_or_else(|e| panic!("Failed to load user app: {}", e));

    // step4: 构造用户上下文
    let uctx = UserContext::new(entry_vaddr.into(), ustack_top, 0);
    let mut task = new_user_task(name, uctx, 0);
    task.ctx_mut().set_page_table_root(uspace.page_table_root());

    // step5: 初始化进程/线程数据结构
    let pid = task.id().as_u64() as Pid;
    let proc = Process::new_init(pid);
    proc.add_thread(pid);

    // step6: 将默认控制终端绑定到 init 进程。
    N_TTY.bind_to(&proc).expect("Failed to bind ntty");

    // step7: 构造 StarryOS 的进程资源数据。
    let proc = ProcessData::new(
        proc,
        path.to_string(),
        Arc::new(args.to_vec()),
        Arc::new(Mutex::new(uspace)),
        Arc::default(),
        None,
    );

    // step8: 为 init 进程初始化标准文件描述符。
    {
        let mut scope = proc.scope.write();
        crate::file::add_stdio(&mut FD_TABLE.scope_mut(&mut scope).write())
            .expect("Failed to add stdio");
    }

    // step9: 创建线程对象，并挂到底层 task 的扩展字段中。
    let thr = Thread::new(pid, proc);
    *task.task_ext_mut() = Some(AxTaskExt::from_impl(thr));

    // step10: 将 init task 放入调度器，并注册到全局任务表。
    let task = spawn_task(task);
    add_task_to_table(&task);

    // step11: 等待 init task 退出。
    // TODO: wait for all processes to finish
    let exit_code = task.join();
    info!("Init process exited with code: {exit_code:?}");

    // step12: init 退出后卸载所有文件系统并 flush rootfs。
    let cx = FS_CONTEXT.lock();
    cx.root_dir()
        .unmount_all()
        .expect("Failed to unmount all filesystems");
    cx.root_dir()
        .filesystem()
        .flush()
        .expect("Failed to flush rootfs");
}
