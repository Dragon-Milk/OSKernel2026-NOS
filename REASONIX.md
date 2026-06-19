# REASONIX.md — NOS (kernel project)

## Stack

- **Language:** Rust (nightly-2026-02-25, edition 2024)
- **Framework:** ArceOS unikernel — builds on `axfeat` / `axalloc` / `axhal` / `axmm` / `axfs` / `axnet` / `axtask` and related core crates (workspace deps in `src/Cargo.toml`)
- **Targets:** `riscv64gc-unknown-none-elf` (primary), `loongarch64-unknown-none-softfloat` — no_std, freestanding
- **Kernel crate:** `starry-kernel` at `src/kernel/` — the only workspace member
- **Key toolchain:** `rust-src` + `llvm-tools`; rustfmt and clippy are **not** in the toolchain components

## Layout

| Path | What lives there |
|---|---|
| `src/kernel/src/` | Kernel proper — entry, MM, task/sched, syscall, file/VFS, pseudofs |
| `src/kernel/src/syscall/` | Linux-compatible syscall dispatch (fs/io_mpx/ipc/mm/net/sync/task) |
| `src/kernel/src/task/` | Process/thread, scheduler, signals, futex, timer |
| `src/kernel/src/mm/` | Address spaces, ELF loader, user pointer access |
| `src/kernel/src/file/` | VFS, epoll, eventfd, pipe, signalfd, pidfd |
| `src/kernel/src/pseudofs/` | procfs, devfs, sysfs, tmpfs |
| `src/init/` | Userspace init program + LTP test harness |
| `src/make/` | Build system (`*.mk` files), linker scripts, QEMU config |
| `src/scripts/` | CI test runner (`ci-test.py`), flash util (`flash.sh`) |
| `src/vendor/` | Vendored dependency crates (CARGO_NET_OFFLINE=true) |
| `src/.axconfig-*.toml` | Generated per-arch build config (do not edit by hand) |
| `docs/prel/` | Preliminary-stage dev notes |
| `reference/` | Reference code, StarryOS license, notices |
| `.local-toolchains/` | Prebuilt riscv64 + loongarch64 musl cross-compilers |

## Commands

| Command | Effect |
|---|---|
| `make all` | Build both kernel-rv (riscv64) and kernel-la (loongarch64) |
| `make kernel-rv` | Build riscv64 kernel binary |
| `make kernel-la` | Build loongarch64 kernel binary |
| `make run` | Boot under QEMU (ARCH env var selects arch) |
| `make clean` | Clean build artifacts + generated config files |
| `make perf-rv DISK_IMG=...` | RISC-V perf run (LOG=trace, TEST_PROFILE=perf) |
| `make perf-la DISK_IMG=...` | LoongArch perf run |

**Key build vars** (set in `src/Makefile`): `ARCH=riscv64|loongarch64`, `LOG=warn|trace`, `MEM=1G`, `BLK=y`, `NET=y`, `TEST_PROFILE=ltp-only|perf`, `LTP_CATEGORY=process|fs|mm|ipc|net`, `LTP_BATCH=NN`, `LTP_LIBC=glibc|musl`.

## Conventions

- **Doc comments** (`COMMENTING.md`): Chinese-language templates — `//!` for module docs, `///` for functions/types with `# 参数` / `# 返回值` sections. Inline `//` for implementation notes.
- **Formatting** (`rustfmt.toml`): `style_edition = "2024"`, `group_imports = "StdExternalCrate"`, `imports_granularity = "Crate"`, `format_strings = true`, `format_code_in_doc_comments = true`.
- **Syscall layout**: Each Linux syscall group has its own subdir under `src/kernel/src/syscall/` (fs, io_mpx, ipc, mm, net, sync, task), mirroring the Linux syscall table structure.

## Watch out for

- **No network fetching** — `CARGO_NET_OFFLINE=true` in `Makefile`; all dependencies vendored under `src/vendor/`. Any new crate dep must also be vendored.
- **Generated config files** — `src/.axconfig-*.toml` are produced by the build system from `src/make/defconfig.toml` + Makefile vars. Editing them by hand gets overwritten.
- **No rustfmt / clippy in toolchain** — `rustfmt` and `clippy` are commented out in `src/rust-toolchain.toml`. Formatting checks require explicit installation.
- **initramfs / disk image required at runtime** — `make run` and `make perf-*` need a `DISK_IMG` (sdcard image) with rootfs; no standalone kernel boot.
- **Two arch builds share one workspace** — `kernel-rv` and `kernel-la` are separate binaries for different targets. Config options (`.axconfig-*.toml`) are per-arch and regenerated each build.

## Progress log

* Before each task, read the latest part of `codex.log` to understand current progress, previous fixes, test results, and remaining issues.
* After completing each task, append a concise summary to `codex.log`.
* Record the task goal, root cause or findings, modified files, validation results, remaining issues, and next step.
* Append only. Never overwrite existing content.
* Do not copy large command outputs or claim tests passed unless they were actually run successfully.
