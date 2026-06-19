# LTP Process Batch Handoff

This note records the current understanding of the OSKernel2026-NOS project and the LTP process-batch work so the next session can continue without rediscovery.

## Workspace And Runtime

- Repo: `/home/haoyue/Project/OSKernel2026-NOS`
- Current branch during this work: `ltp-process-cases-fix`
- Docker container used for tests: `angry_cori`
- Container workspace path: `/workspace`
- Main test image: `sdcard-rv.img`
- Common run command:

```sh
docker exec angry_cori sh -lc 'cd /workspace && make -C src run DISK_IMG=sdcard-rv.img TEST_PROFILE=ltp-batch LTP_CATEGORY=process LTP_BATCH=06 LTP_LIBC=glibc 2>&1 | tee batch06.txt'
```

- Compile-only command:

```sh
docker exec angry_cori sh -lc 'cd /workspace && make -C src kernel-rv'
```

- If a run is interrupted, check and kill stale QEMU/make processes:

```sh
docker exec angry_cori sh -lc 'ps -ef | grep -E "qemu-system-riscv64|make -C src run|LTP_BATCH=06" | grep -v grep'
docker exec angry_cori sh -lc 'kill <qemu-pid> <make-pid> <shell-pid> 2>/dev/null || true'
```

## Project Understanding

- The kernel is Rust based, with syscall handling centered in `src/kernel/src/syscall/mod.rs`.
- Process/thread state is in `src/kernel/src/task/mod.rs`.
- Fork/clone behavior is in `src/kernel/src/syscall/task/clone.rs`.
- Scheduling, sleep, priority, and personality-related syscalls are in `src/kernel/src/syscall/task/schedule.rs`.
- Signal syscalls and signal waiting are in `src/kernel/src/syscall/signal.rs`.
- LTP process batch case lists are duplicated in:
  - `src/init/ltp-cases/process-XX.txt`
  - `src/init/ltp-cases.sh`
- Keep those two in sync when adding/skipping cases.

## General Process Batch Strategy

- Prefer real kernel fixes for simple syscall semantics.
- Skip cases that are currently environment-dependent, hang indefinitely, or require larger missing kernel subsystems.
- For each batch:
  - Run in Docker with `TEST_PROFILE=ltp-batch`.
  - Search the log for `FAIL LTP CASE`, `TFAIL`, `TBROK`, `panicked`, `Segmentation fault`, `Out of memory`.
  - Fix clear kernel bugs first.
  - Skip only the remaining unsupported/hanging cases.

## Existing Dirty State Warning

- Do not assume the working tree is clean.
- At the time this note was written, these unrelated/log changes existed:
  - `D batch01.txt`
  - `D batch02.txt`
  - `?? batch06.txt`
- `src/kernel/src/syscall/signal.rs` also had a partial staged state from earlier work.
- `.git/objects` may have permission issues. Previous workaround for git commands:

```sh
GIT_OBJECT_DIRECTORY=.git/objects-codex GIT_ALTERNATE_OBJECT_DIRECTORIES=$PWD/.git/objects git status --short --branch
```

## Process Batch01-06 Overview

The process tests have been split into numbered batches under `src/init/ltp-cases/process-XX.txt`. The corresponding generated switch in `src/init/ltp-cases.sh` must stay in sync.

Run a specific batch by changing `LTP_BATCH`:

```sh
docker exec angry_cori sh -lc 'cd /workspace && make -C src run DISK_IMG=sdcard-rv.img TEST_PROFILE=ltp-batch LTP_CATEGORY=process LTP_BATCH=01 LTP_LIBC=glibc 2>&1 | tee batch01.txt'
```

### Batch01

Current scope:

```text
abort01
acct01
acct02
adjtimex01
adjtimex02
adjtimex03
alarm02
alarm03
alarm05
alarm06
alarm07
arch_prctl01
autogroup01
clock_adjtime01
clock_adjtime02
clock_getres01
clock_gettime01
clock_gettime02
clock_gettime03
clock_gettime04
clock_nanosleep01
clock_nanosleep02
clock_nanosleep03
clock_nanosleep04
clock_settime01
clock_settime02
clock_settime03
clone01
clone02
clone03
```

Main areas: abort/accounting stubs, adjtimex/timekeeping, alarm/timer delivery, clock APIs, and early clone coverage.

### Batch02

Current scope:

```text
clone04
clone05
clone06
clone07
clone08
clone09
clone301
clone302
clone303
exec_with_inh
exec_without_inh
execl01
execle01
execlp01
execv01
execve01
execve02
execve03
execve04
execve05
execve06
execveat01
execveat02
execveat03
execveat_errno
execvp01
exit01
exit02
exit_group01
fork01
fork03
fork04
fork05
fork07
fork08
fork09
fork10
fork13
fork14
```

Main areas: clone/clone3 validation, execve/execveat argument and inheritance behavior, exit/exit_group, and fork stress cases.

Known context from prior work:
- `fork13` and `fork14` were previously long-running/problematic.
- `fork14` was discussed as slower than `fork13`; do not remove it silently unless using the project skip policy.
- Fork-related work was eventually merged into earlier batches before batch03 work continued.

### Batch03

Current scope:

```text
fork_exec_loop
fork_procs
getcontext01
getcpu01
getdomainname01
getegid01
getegid01_16
getegid02
getegid02_16
geteuid01
geteuid01_16
geteuid02
geteuid02_16
getgid01
getgid01_16
getgid03
getgid03_16
getgroups01
getgroups01_16
getgroups03
getgroups03_16
```

Main areas: fork pressure, process identity syscalls, 16-bit compatibility TCONF handling, and group list behavior.

Known context from prior work:
- `getgroups01` had failed because `getgroups(negative/invalid size)` did not return `EINVAL` as expected.
- The expected kernel behavior is that invalid `size` should fail without modifying the gidset array.
- 16-bit compatibility cases commonly return `TCONF` on this platform and should not be treated as failures.

### Batch04

Current scope:

```text
gethostname01
gethostname02
getitimer01
getitimer02
getpgid01
getpgid02
getpgrp01
getpid01
getpid02
getppid01
getppid02
getpriority01
getpriority02
getresgid01
getresgid01_16
getresgid02
getresgid02_16
getresgid03
getresgid03_16
getresuid01
getresuid01_16
getresuid02
getresuid02_16
getresuid03
getresuid03_16
getrlimit01
getrlimit02
getrlimit03
getrusage01
getrusage02
```

Main areas: hostname, itimer, process group/session ids, pid/ppid, getpriority, uid/gid families, resource limits, and basic rusage.

Known context from prior work:
- Scheduler-parameter syscalls and priority reporting were investigated around this phase.
- Process-group registration was later adjusted so `setpgid(child, child)` and `setpgid(pid, 0)` create/register process groups correctly.
- UID/GID inheritance through fork was fixed later and is relevant to these identity cases.

### Batch05

Current scope:

```text
getrusage03
getrusage04
getsid01
getsid02
gettid01
gettid02
gettimeofday01
gettimeofday02
getuid01
getuid01_16
getuid03
getuid03_16
hangup01
ioprio_get01
ioprio_set01
ioprio_set02
ioprio_set03
kcmp01
kcmp02
kcmp03
kill02
kill03
kill05
kill06
kill07
kill08
kill09
kill10
kill11
kill12
```

Batch05 had been worked on before batch06. Do not revert these changes unless explicitly asked.

Implemented/fixed earlier:
- `gettimeofday(tv, tz)` accepts and writes `tz`; syscall dispatch passes both args.
- `setitimer` uses `try_borrow_mut`.
- `kill` permission checks and process-group signal sending were improved.
- `fork` inherits uid/gid/resuid/resgid.
- `setpgid(child, child)` / `setpgid(pid, 0)` registers process groups.
- Alarm heap panic was avoided when the heap changes during poll.

Batch05 skipped cases:
- `getrusage03`
- `getrusage04`
- `hangup01`
- `kill02`

Batch05 was reported passing after those skips.

### Batch06

Original batch06 scope included:

```text
kill13
leapsec01
nanosleep01
nanosleep02
nanosleep04
newuname01
nice01
nice02
nice03
nice04
nice05
nptl01
pause01
pause02
pause03
personality01
personality02
pidfd_getfd01
pidfd_getfd02
pidfd_open01
pidfd_open02
pidfd_open03
pidfd_open04
pidfd_send_signal01
pidfd_send_signal02
pidfd_send_signal03
prctl01
prctl02
prctl03
prctl04
```

## Batch06 Changes Made

Files touched for batch06:
- `src/kernel/src/syscall/task/schedule.rs`
- `src/kernel/src/task/mod.rs`
- `src/kernel/src/syscall/task/clone.rs`
- `src/kernel/src/syscall/mod.rs`
- `src/init/ltp-cases/process-06.txt`
- `src/init/ltp-cases.sh`

Implemented:
- `sleep_impl()` now uses `saturating_sub()` instead of `clock() - start`.
  - This fixed a `leapsec01` panic: `overflow when subtracting durations`.
- Added per-thread Linux nice value storage in `Thread`.
- Added per-thread personality storage in `Thread`.
- Fork/clone now inherits scheduler policy/priority, nice, and personality.
- Added `sys_setpriority()`.
- Changed `sys_getpriority()` to return Linux raw syscall value `20 - nice`, because libc converts that back to user-visible nice.
- Added `sys_personality()`.
- Added syscall dispatch for `setpriority` and `personality`.

Important note on `pause`:
- A direct `Sysno::pause` dispatch could not be added for RISC-V because the syscall enum has no `pause` variant on this arch.
- RISC-V glibc appears to implement `pause()` through another signal-wait path, and the current behavior returns `EFAULT` or exits unexpectedly.
- `pause01`, `pause02`, and `pause03` were therefore skipped for now.

## Batch06 Current Case List

Current `process-06.txt` contains:

```text
leapsec01
nanosleep02
nanosleep04
newuname01
nice01
nice02
nice03
nice04
nptl01
personality01
personality02
pidfd_getfd01
pidfd_open01
pidfd_open02
pidfd_open03
pidfd_open04
pidfd_send_signal01
pidfd_send_signal02
pidfd_send_signal03
prctl01
prctl02
prctl03
prctl04
```

## Batch06 Validation So Far

Compile passed:

```sh
docker exec angry_cori sh -lc 'cd /workspace && make -C src kernel-rv'
```

After updating the skip list and fixing nice/personality, the interrupted batch06 run reached `pidfd_getfd01`. The log showed these cases passed:

- `leapsec01`
- `nanosleep02`
- `nanosleep04`
- `newuname01`
- `nice01`
- `nice02`
- `nice03`
- `nice04`
- `nptl01`
- `personality01`
- `personality02`

The run was interrupted before full completion, so the remaining pidfd/prctl cases still need a full rerun.

## LTP Cases Not Successfully Fixed Yet

These were removed or left unresolved and still need real fixes if full upstream-style coverage is required:

Batch05:

- `getrusage03`
  - Skipped in the previous passing batch05 run.
  - Needs a real rusage accounting implementation if this case must pass without skip.

- `getrusage04`
  - Skipped in the previous passing batch05 run.
  - Needs a real rusage accounting implementation if this case must pass without skip.

- `hangup01`
  - Skipped in the previous passing batch05 run.
  - Likely needs fuller terminal/session/hangup semantics.

- `kill02`
  - Skipped in the previous passing batch05 run.
  - Needs deeper signal/permission/session behavior investigation.

Batch06:

- `kill13`
  - Fails with `Segmentation fault`.
  - Log also shows `sh: zcat: not found` while parsing `/proc/config.gz`.
  - Likely environment/proc-config path plus robustness issue.

- `nanosleep01`
  - Fails with `TBROK: Test killed by SIGSEGV`.
  - Needs investigation of the exact child/process crash path.

- `nice05`
  - Requires real scheduler weighting: lower nice should receive more CPU time.
  - Current implementation only stores/reports nice value; it does not affect the scheduler.

- `pause01`
  - `pause()` returns `EFAULT` instead of `EINTR`, then checkpoint timeout.

- `pause02`
  - Child reports `Pause returned -1 but errno is 14 (Bad address)`.

- `pause03`
  - `pause()` returns unexpectedly; expected child termination by `SIGKILL`.

- `pidfd_getfd02`
  - Initially passed the first invalid-argument checks, then hung for more than 60 seconds with no output.
  - Needs pidfd/getfd behavior or test-hang investigation.

## Recommended Next Steps

1. Run the current batch06 to completion.
2. If pidfd/prctl cases fail, inspect `batch06.txt` and fix or skip only the failing unsupported cases.
3. Do not commit until asked; current tree also contains earlier batch05 changes and unrelated deleted log files.
4. If committing later, avoid staging `batch*.txt` logs and unrelated deletions.
