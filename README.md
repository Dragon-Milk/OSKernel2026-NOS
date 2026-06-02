# NOS

基于 [StarryOS](https://github.com/Starry-OS/StarryOS) 开发的 OS ，遵循相关开源协议（详见 reference）

## 项目结构

```text
.
├── docs                        # 项目文档
│   └── prel                    # 初赛阶段文档、开发记录和评测说明
├── reference                   # 参考代码、StarryOS 原始许可证及来源说明
├── src                         # 项目源码和构建资源
│   ├── init                    # 用户态 init 程序和启动脚本
│   ├── kernel                  # 内核主体
│   │   └── src
│   │       ├── config          # 内核配置和平台相关配置
│   │       ├── file            # 文件抽象
│   │       ├── mm              # 内存管理、地址空间、用户指针访问、ELF 加载
│   │       ├── pseudofs        # procfs、devfs、sysfs、tmpfs 等伪文件系统
│   │       ├── syscall         # Linux 兼容系统调用入口和各类 syscall 实现
│   │       ├── task            # 进程、线程、调度、信号、futex、定时器等任务管理
│   │       ├── entry.rs        # 内核启动入口和用户程序启动流程
│   │       ├── lib.rs          # kernel crate 根模块
│   │       └── time.rs         # 内核时间相关工具
│   ├── make                    # 构建规则和 Makefile 片段
│   ├── scripts                 # 构建、运行、调试和测试辅助脚本
│   ├── target-check            # 目标平台检查或辅助构建配置
│   ├── tools                   # 镜像制作、测试处理等辅助工具
│   └── vendor                  # 随仓库提交的第三方依赖源码
├── COMMENTING.md               # Rustdoc 注释模板和注释规范
├── README.md                   # 项目说明
└── Makefile                    # 比赛评测入口构建脚本，提供 make all
```