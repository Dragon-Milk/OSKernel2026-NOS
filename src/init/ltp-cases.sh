# Auto-generated from src/init/ltp-cases/*.txt.
# This file is embedded into the init binary by src/init/main.rs.

ltp_batch_ids() {
  case "$1" in
  process)
    echo "01 02 03 04 05 06 07 08 09 10 11 12 13 14 15"
    ;;
  fs)
    echo "01 02 03 04 05 06 07 08 09 10 11 12 13"
    ;;
  mm-ipc)
    echo "01 02 03 04 05 06 07 08 09"
    ;;
  common-easy)
    echo "01"
    ;;
  part-b | testb)
    echo "01"
    ;;
  *)
    return 1
    ;;
  esac
}

ltp_batch_cases() {
  category="$1"
  batch="$2"

  case "$category:$batch" in
  process:01)
    printf '%s\n' \
      'abort01' \
      'adjtimex01' \
      'adjtimex03' \
      'alarm02' \
      'alarm03' \
      'alarm05' \
      'alarm06' \
      'alarm07' \
      'clock_adjtime01' \
      'clock_getres01' \
      'clock_gettime01' \
      'clock_gettime02' \
      'clock_nanosleep01' \
      'clock_nanosleep04' \
      'clock_settime01' \
      'clock_settime02' \
      'clone01' \
      'clone03'

    ;;
  process:02)
    printf '%s\n' \
      'clone05' \
      'clone06' \
      'clone07' \
      'clone09' \
      'clone301' \
      'clone302' \
      'clone08' \
      'execl01' \
      'execlp01' \
      'execle01' \
      'execve01' \
      'execv01' \
      'execveat03' \
      'execveat_errno' \
      'execvp01' \
      'exit01' \
      'exit02' \
      'exit_group01' \
      'fork01' \
      'fork03' \
      'fork04' \
      'fork07' \
      'fork08' \
      'fork09' \
      'fork10' \
      'fork14'

    ;;
  process:03)
    printf '%s\n' \
      'fork_procs' \
      'getcpu01' \
      'getdomainname01' \
      'getegid01' \
      'getegid02' \
      'geteuid01' \
      'geteuid02' \
      'getgid01' \
      'getgid03' \
      'getgroups01' \
      'getgroups03'

    ;;
  process:04)
    printf '%s\n' \
      'gethostname01' \
      'getitimer01' \
      'getpgid01' \
      'getpgid02' \
      'getpgrp01' \
      'getpid01' \
      'getpid02' \
      'getppid01' \
      'getppid02' \
      'getpriority01' \
      'getpriority02' \
      'getresgid01' \
      'getresgid02' \
      'getresgid03' \
      'getresuid01' \
      'getresuid02' \
      'getresuid03' \
      'getrlimit01' \
      'getrlimit02' \
      'getrlimit03' \
      'getrusage01' \
      'getrusage02'

    ;;
  process:05)
    printf '%s\n' \
      'getsid01' \
      'getsid02' \
      'gettid01' \
      'gettid02' \
      'gettimeofday01' \
      'gettimeofday02' \
      'getuid01' \
      'getuid03' \
      'kill03' \
      'kill05' \
      'kill06' \
      'kill07' \
      'kill08' \
      'kill09' \
      'kill10' \
      'kill11' \
      'kill12'

    ;;
  process:06)
    printf '%s\n' \
      'leapsec01' \
      'nanosleep02' \
      'nanosleep04' \
      'newuname01' \
      'nice02' \
      'nice03' \
      'nptl01' \
      'pause01' \
      'pause03' \
      'personality01' \
      'personality02' \
      'pidfd_getfd01' \
      'pidfd_getfd02' \
      'pidfd_open01' \
      'pidfd_open02' \
      'pidfd_open03' \
      'pidfd_send_signal01' \
      'pidfd_send_signal02' \
      'pidfd_send_signal03' \
      'prctl01' \
      'prctl02' \
      'prctl03' \
      'prctl04'

    ;;
  process:07)
    printf '%s\n' \
      'prctl05' \
      'prctl08' \
      'pth_str02' \
      'rt_sigaction01' \
      'rt_sigaction02' \
      'rt_sigaction03' \
      'rt_sigprocmask01' \
      'rt_sigprocmask02' \
      'rt_sigsuspend01'

    ;;
  process:08)
    printf '%s\n' \
      'sched_get_priority_max01' \
      'sched_get_priority_max02' \
      'sched_get_priority_min01' \
      'sched_get_priority_min02' \
      'sched_getaffinity01' \
      'sched_getattr01' \
      'sched_getattr02' \
      'sched_getparam01' \
      'sched_getparam03' \
      'sched_getscheduler01' \
      'sched_getscheduler02' \
      'sched_rr_get_interval01' \
      'sched_rr_get_interval02' \
      'sched_rr_get_interval03' \
      'sched_setaffinity01' \
      'sched_setattr01' \
      'sched_setparam01' \
      'sched_setparam02' \
      'sched_setparam03' \
      'sched_setparam04' \
      'sched_setparam05' \
      'sched_setscheduler01' \
      'sched_setscheduler02' \
      'sched_setscheduler03' \
      'sched_setscheduler04' \
      'sched_tc2' \
      'sched_tc3'

    ;;
  process:09)
    printf '%s\n' \
      'sched_tc4' \
      'sched_tc5' \
      'sched_yield01' \
      'set_tid_address01' \
      'setdomainname01' \
      'setdomainname02' \
      'setdomainname03' \
      'setegid01' \
      'setegid02' \
      'setfsgid01' \
      'setfsgid02' \
      'setfsgid03' \
      'setfsuid01' \
      'setfsuid02' \
      'setfsuid03' \
      'setfsuid04' \
      'setgid01' \
      'setgid02' \
      'setgid03'

    ;;
  process:10)
    printf '%s\n' \
      'setgroups01' \
      'setgroups02' \
      'sethostname01' \
      'sethostname02' \
      'sethostname03' \
      'setitimer01' \
      'setitimer02' \
      'setpgid01' \
      'setpgid02' \
      'setpgid03' \
      'setpgrp01' \
      'setpgrp02' \
      'setpriority01' \
      'setregid01' \
      'setregid02' \
      'setregid03' \
      'setregid04' \
      'setresgid01'

    ;;
  process:11)
    printf '%s\n' \
      'setresgid02' \
      'setresgid03' \
      'setresgid04' \
      'setresuid01' \
      'setresuid02' \
      'setresuid03' \
      'setresuid04' \
      'setresuid05' \
      'setreuid01' \
      'setreuid02' \
      'setreuid03' \
      'setreuid04' \
      'setreuid05' \
      'setreuid06' \
      'setreuid07'

    ;;
  process:12)
    printf '%s\n' \
      'setrlimit01' \
      'setrlimit02' \
      'setrlimit03' \
      'setrlimit04' \
      'setrlimit05' \
      'setrlimit06' \
      'setsid01' \
      'settimeofday01' \
      'settimeofday02' \
      'setuid01' \
      'setuid03' \
      'setuid04' \
      'signal01' \
      'sigaction02' \
      'sigaltstack01' \
      'sigaltstack02' \
      'sighold02' \
      'signal02' \
      'signal03' \
      'signal04' \
      'signal05' \
      'signalfd01' \
      'signalfd4_01'

    ;;
  process:13)
    printf '%s\n' \
      'signalfd4_02' \
      'sigpending02' \
      'sigprocmask01' \
      'sigsuspend01' \
      'ssetmask01' \
      'stime01' \
      'stime02' \
      'tgkill01' \
      'tgkill02' \
      'tgkill03' \
      'time-schedule' \
      'time01' \
      'timer_delete01' \
      'timer_delete02' \
      'timer_getoverrun01' \
      'timer_gettime01' \
      'timer_settime01' \
      'timer_settime02' \
      'timer_settime03' \
      'timerfd01' \
      'timerfd02'

    ;;
  process:14)
    printf '%s\n' \
      'times01' \
      'times03' \
      'tkill01' \
      'tkill02' \
      'uname01' \
      'uname02' \
      'uname04' \
      'utsname01' \
      'utsname02' \
      'utsname03' \
      'utsname04' \
      'vfork02' \
      'wait01' \
      'wait02' \
      'wait401' \
      'wait402' \
      'wait403' \
      'waitid01' \
      'waitid02' \
      'waitid03' \
      'waitid04' \
      'waitid05' \
      'waitid06' \
      'waitid07' \
      'waitid08' \
      'waitid09' \
      'waitid10'

    ;;
  process:15)
    printf '%s\n' \
      'waitid11' \
      'waitpid01' \
      'waitpid03' \
      'waitpid04' \
      'waitpid06' \
      'waitpid07' \
      'waitpid08' \
      'waitpid09' \
      'waitpid10' \
      'waitpid11' \
      'waitpid12' \
      'waitpid13'

    ;;
  fs:01)
    printf '%s\n' \
      'access01' \
      'access02' \
      'access03' \
      'access04' \
      'chdir01' \
      'chdir04' \
      'chmod01' \
      'chmod03' \
      'chmod05' \
      'chmod06' \
      'chmod07' \
      'chown01' \
      'chown02' \
      'chown03' \
      'chown04' \
      'chown05' \
      'close01' \
      'close02' \
      'close_range01' \
      'close_range02' \
      'creat01' \
      'creat03' \
      'creat04' \
      'creat05' \
      'creat06'

    ;;
  fs:02)
    printf '%s\n' \
      'creat07' \
      'creat08' \
      'creat09' \
      'dup01' \
      'dup02' \
      'dup03' \
      'dup04' \
      'dup05' \
      'dup06' \
      'dup07' \
      'dup201' \
      'dup202' \
      'dup203' \
      'dup204' \
      'dup205' \
      'dup206' \
      'dup207' \
      'dup3_01' \
      'dup3_02' \
      'faccessat01' \
      'faccessat02' \
      'faccessat201' \
      'faccessat202' \
      'fchdir01' \
      'fchdir02' \
      'fchdir03' \
      'fchmod01' \
      'fchmod02' \
      'fchmod03' \
      'fchmod04'

    ;;
  fs:03)
    printf '%s\n' \
      'fchmod05' \
      'fchmod06' \
      'fchmodat01' \
      'fchmodat02' \
      'fchown01' \
      'fchown02' \
      'fchown03' \
      'fchown04' \
      'fchown05' \
      'fchownat01' \
      'fchownat02' \
      'fcntl01' \
      'fcntl01_64' \
      'fcntl02' \
      'fcntl02_64' \
      'fcntl03' \
      'fcntl03_64' \
      'fcntl04' \
      'fcntl04_64' \
      'fcntl05' \
      'fcntl05_64' \
      'fcntl07' \
      'fcntl07_64' \
      'fcntl08' \
      'fcntl08_64'

    ;;
  fs:04)
    printf '%s\n' \
      'fcntl09' \
      'fcntl09_64' \
      'fcntl10' \
      'fcntl10_64' \
      'fcntl11' \
      'fcntl11_64' \
      'fcntl12' \
      'fcntl12_64' \
      'fcntl13' \
      'fcntl13_64' \
      'fcntl14' \
      'fcntl14_64' \
      'fcntl15' \
      'fcntl15_64' \
      'fcntl16' \
      'fcntl16_64' \
      'fcntl17' \
      'fcntl17_64' \
      'fcntl18' \
      'fcntl18_64' \
      'fcntl19' \
      'fcntl19_64' \
      'fcntl20' \
      'fcntl20_64' \
      'fcntl21' \
      'fcntl21_64' \
      'fcntl22' \
      'fcntl22_64' \
      'fcntl23' \
      'fcntl23_64'

    ;;
  fs:05)
    printf '%s\n' \
      'fcntl24' \
      'fcntl24_64' \
      'fcntl25' \
      'fcntl25_64' \
      'fcntl26' \
      'fcntl26_64' \
      'fcntl27' \
      'fcntl27_64' \
      'fcntl29' \
      'fcntl29_64' \
      'fcntl30' \
      'fcntl30_64' \
      'fcntl31' \
      'fcntl31_64' \
      'fcntl32' \
      'fcntl32_64' \
      'fcntl33' \
      'fcntl33_64' \
      'fcntl34' \
      'fcntl34_64' \
      'fcntl35' \
      'fcntl35_64' \
      'fcntl36' \
      'fcntl36_64' \
      'fcntl37' \
      'fcntl37_64' \
      'fcntl38' \
      'fcntl38_64' \
      'fcntl39' \
      'fcntl39_64'

    ;;
  fs:06)
    printf '%s\n' \
      'fdatasync01' \
      'fdatasync02' \
      'fdatasync03' \
      'fgetxattr01' \
      'fgetxattr02' \
      'fgetxattr03' \
      'flistxattr01' \
      'flistxattr02' \
      'flistxattr03' \
      'fpathconf01' \
      'fremovexattr01' \
      'fremovexattr02' \
      'fsetxattr01' \
      'fsetxattr02' \
      'fstat02' \
      'fstat02_64' \
      'fstat03' \
      'fstat03_64' \
      'fstatat01' \
      'fstatfs01' \
      'fstatfs01_64' \
      'fstatfs02' \
      'fstatfs02_64' \
      'fsync01' \
      'fsync02' \
      'fsync03' \
      'fsync04' \
      'ftruncate01' \
      'ftruncate01_64' \
      'ftruncate03'

    ;;
  fs:07)
    printf '%s\n' \
      'ftruncate03_64' \
      'ftruncate04' \
      'ftruncate04_64' \
      'getcwd01' \
      'getcwd02' \
      'getcwd03' \
      'getcwd04' \
      'getdents01' \
      'getdents02' \
      'getxattr01' \
      'getxattr02' \
      'getxattr03' \
      'getxattr04' \
      'getxattr05' \
      'lchown01' \
      'lchown02' \
      'lchown03' \
      'lgetxattr01' \
      'lgetxattr02' \
      'link02' \
      'link04' \
      'link05' \
      'link08' \
      'linkat01' \
      'linkat02' \
      'linktest.sh' \
      'listxattr01'

    ;;
  fs:08)
    printf '%s\n' \
      'listxattr02' \
      'listxattr03' \
      'llistxattr01' \
      'llistxattr02' \
      'llistxattr03' \
      'llseek01' \
      'llseek02' \
      'llseek03' \
      'lremovexattr01' \
      'lseek01' \
      'lseek02' \
      'lseek07' \
      'lseek11' \
      'lstat01' \
      'lstat01_64' \
      'lstat02' \
      'lstat02_64' \
      'mkdir02' \
      'mkdir03' \
      'mkdir04' \
      'mkdir05' \
      'mkdir09' \
      'mkdir_tests.sh' \
      'mkdirat01' \
      'mkdirat02' \
      'open01' \
      'open02' \
      'open03' \
      'open04' \
      'open06'

    ;;
  fs:09)
    printf '%s\n' \
      'open07' \
      'open08' \
      'open09' \
      'open10' \
      'open11' \
      'open12' \
      'open13' \
      'open14' \
      'openat01' \
      'openat02' \
      'openat03' \
      'openat04' \
      'openat201' \
      'openat202' \
      'openat203' \
      'openfile' \
      'pathconf01' \
      'pathconf02' \
      'pread01' \
      'pread01_64' \
      'pread02' \
      'pread02_64' \
      'preadv01' \
      'preadv01_64' \
      'preadv02' \
      'preadv02_64' \
      'preadv03' \
      'preadv03_64' \
      'preadv201' \
      'preadv201_64'

    ;;
  fs:10)
    printf '%s\n' \
      'preadv202' \
      'preadv202_64' \
      'preadv203' \
      'preadv203_64' \
      'pwrite01' \
      'pwrite01_64' \
      'pwrite02' \
      'pwrite02_64' \
      'pwrite03' \
      'pwrite03_64' \
      'pwrite04' \
      'pwrite04_64' \
      'pwritev01' \
      'pwritev01_64' \
      'pwritev02' \
      'pwritev02_64' \
      'pwritev03' \
      'pwritev03_64' \
      'pwritev201' \
      'pwritev201_64' \
      'pwritev202' \
      'pwritev202_64' \
      'read01' \
      'read02' \
      'read03' \
      'read04' \
      'readdir01' \
      'readdir21' \
      'readlink01' \
      'readlink03'

    ;;
  fs:11)
    printf '%s\n' \
      'readlinkat01' \
      'readlinkat02' \
      'readv01' \
      'readv02' \
      'realpath01' \
      'removexattr01' \
      'removexattr02' \
      'rename01' \
      'rename03' \
      'rename04' \
      'rename05' \
      'rename06' \
      'rename07' \
      'rename08' \
      'rename09' \
      'rename10' \
      'rename11' \
      'rename12' \
      'rename13' \
      'rename14' \
      'renameat01' \
      'renameat201' \
      'renameat202' \
      'rmdir01' \
      'rmdir02' \
      'rmdir03' \
      'setxattr01' \
      'setxattr02' \
      'setxattr03' \
      'stat01'

    ;;
  fs:12)
    printf '%s\n' \
      'stat01_64' \
      'stat02' \
      'stat02_64' \
      'stat03' \
      'stat03_64' \
      'statfs01' \
      'statfs01_64' \
      'statfs02' \
      'statfs02_64' \
      'statfs03' \
      'statfs03_64' \
      'statvfs01' \
      'statvfs02' \
      'statx01' \
      'statx02' \
      'statx03' \
      'statx04' \
      'statx05' \
      'statx06' \
      'statx07' \
      'statx08' \
      'statx09' \
      'statx10' \
      'statx11' \
      'statx12' \
      'symlink01' \
      'symlink02' \
      'symlink03' \
      'symlink04' \
      'symlinkat01'

    ;;
  fs:13)
    printf '%s\n' \
      'sync01' \
      'sync_file_range01' \
      'sync_file_range02' \
      'syncfs01' \
      'truncate02' \
      'truncate02_64' \
      'truncate03' \
      'truncate03_64' \
      'unlink05' \
      'unlink07' \
      'unlink08' \
      'unlink09' \
      'unlinkat01' \
      'write01' \
      'write02' \
      'write03' \
      'write04' \
      'write05' \
      'write06' \
      'writetest' \
      'writev01' \
      'writev02' \
      'writev03' \
      'writev05' \
      'writev06' \
      'writev07'

    ;;
  mm-ipc:01)
    printf '%s\n' \
      'accept01' \
      'accept02' \
      'accept03' \
      'accept4_01' \
      'bind01' \
      'bind02' \
      'bind03' \
      'bind04' \
      'bind05' \
      'bind06' \
      'brk01' \
      'brk02' \
      'connect01' \
      'connect02' \
      'epoll-ltp' \
      'epoll_create01' \
      'epoll_create02' \
      'epoll_create1_01' \
      'epoll_create1_02' \
      'epoll_ctl01' \
      'epoll_ctl02' \
      'epoll_ctl03' \
      'epoll_ctl04' \
      'epoll_ctl05' \
      'epoll_pwait01' \
      'epoll_pwait02' \
      'epoll_pwait03' \
      'epoll_pwait04' \
      'epoll_pwait05' \
      'epoll_wait01'

    ;;
  mm-ipc:02)
    printf '%s\n' \
      'epoll_wait02' \
      'epoll_wait03' \
      'epoll_wait04' \
      'epoll_wait05' \
      'epoll_wait06' \
      'epoll_wait07' \
      'eventfd01' \
      'eventfd02' \
      'eventfd03' \
      'eventfd04' \
      'eventfd05' \
      'eventfd06' \
      'eventfd2_01' \
      'eventfd2_02' \
      'eventfd2_03' \
      'futex_cmp_requeue01' \
      'futex_cmp_requeue02' \
      'futex_wait01' \
      'futex_wait02' \
      'futex_wait03' \
      'futex_wait04' \
      'futex_wait05' \
      'futex_wait_bitset01' \
      'futex_waitv01' \
      'futex_waitv02' \
      'futex_waitv03' \
      'futex_wake01' \
      'futex_wake02' \
      'futex_wake03' \
      'futex_wake04'

    ;;
  mm-ipc:03)
    printf '%s\n' \
      'getpeername01' \
      'getsockname01' \
      'getsockopt01' \
      'getsockopt02' \
      'listen01' \
      'madvise01' \
      'madvise02' \
      'madvise03' \
      'madvise05' \
      'madvise06' \
      'madvise07' \
      'madvise08' \
      'madvise09' \
      'madvise10' \
      'madvise11' \
      'mincore01' \
      'mincore02' \
      'mincore03' \
      'mincore04' \
      'mmap-corruption01' \
      'mmap001' \
      'mmap01' \
      'mmap02' \
      'mmap03' \
      'mmap04' \
      'mmap05' \
      'mmap06' \
      'mmap08' \
      'mmap09' \
      'mmap1'

    ;;
  mm-ipc:04)
    printf '%s\n' \
      'mmap10' \
      'mmap11' \
      'mmap12' \
      'mmap13' \
      'mmap14' \
      'mmap15' \
      'mmap16' \
      'mmap17' \
      'mmap18' \
      'mmap19' \
      'mmap2' \
      'mmap20' \
      'mmap3' \
      'mprotect01' \
      'mprotect02' \
      'mprotect03' \
      'mprotect04' \
      'mprotect05' \
      'mq_notify01' \
      'mq_notify02' \
      'mq_notify03' \
      'mq_open01' \
      'mq_timedreceive01' \
      'mq_timedsend01' \
      'mq_unlink01' \
      'mremap01' \
      'mremap02' \
      'mremap03' \
      'mremap04' \
      'mremap05'

    ;;
  mm-ipc:05)
    printf '%s\n' \
      'mremap06' \
      'msg_comm' \
      'msgctl01' \
      'msgctl02' \
      'msgctl03' \
      'msgctl04' \
      'msgctl05' \
      'msgctl06' \
      'msgctl12' \
      'msgget01' \
      'msgget02' \
      'msgget03' \
      'msgget04' \
      'msgget05' \
      'msgrcv01' \
      'msgrcv02' \
      'msgrcv03' \
      'msgrcv05' \
      'msgrcv06' \
      'msgrcv07' \
      'msgrcv08' \
      'msgsnd01' \
      'msgsnd02' \
      'msgsnd05' \
      'msgsnd06' \
      'msync01' \
      'msync02' \
      'msync03' \
      'msync04' \
      'munmap01'

    ;;
  mm-ipc:06)
    printf '%s\n' \
      'munmap02' \
      'munmap03' \
      'pipe01' \
      'pipe02' \
      'pipe03' \
      'pipe04' \
      'pipe05' \
      'pipe06' \
      'pipe07' \
      'pipe08' \
      'pipe09' \
      'pipe10' \
      'pipe11' \
      'pipe12' \
      'pipe13' \
      'pipe14' \
      'pipe15' \
      'pipe2_01' \
      'pipe2_02' \
      'pipe2_04' \
      'pipeio' \
      'poll01' \
      'poll02' \
      'ppoll01' \
      'pselect01' \
      'pselect01_64' \
      'pselect02' \
      'pselect02_64' \
      'pselect03' \
      'pselect03_64'

    ;;
  mm-ipc:07)
    printf '%s\n' \
      'recv01' \
      'recvfrom01' \
      'recvmmsg01' \
      'recvmsg01' \
      'recvmsg02' \
      'recvmsg03' \
      'select01' \
      'select02' \
      'select03' \
      'select04' \
      'sem_comm' \
      'sem_nstest' \
      'semctl01' \
      'semctl02' \
      'semctl03' \
      'semctl04' \
      'semctl05' \
      'semctl06' \
      'semctl07' \
      'semctl08' \
      'semctl09' \
      'semget01' \
      'semget02' \
      'semget05' \
      'semop01' \
      'semop02' \
      'semop03' \
      'semop04' \
      'semop05' \
      'semtest_2ns'

    ;;
  mm-ipc:08)
    printf '%s\n' \
      'send01' \
      'send02' \
      'sendmmsg01' \
      'sendmmsg02' \
      'sendmsg01' \
      'sendmsg02' \
      'sendmsg03' \
      'sendto01' \
      'sendto02' \
      'sendto03' \
      'setsockopt01' \
      'setsockopt02' \
      'setsockopt03' \
      'setsockopt04' \
      'setsockopt05' \
      'setsockopt06' \
      'setsockopt07' \
      'setsockopt08' \
      'setsockopt09' \
      'setsockopt10' \
      'shm_comm' \
      'shm_test' \
      'shmat01' \
      'shmat02' \
      'shmat03' \
      'shmat04' \
      'shmat1' \
      'shmctl01' \
      'shmctl02' \
      'shmctl03'

    ;;
  mm-ipc:09)
    printf '%s\n' \
      'shmctl04' \
      'shmctl05' \
      'shmctl06' \
      'shmctl07' \
      'shmctl08' \
      'shmdt01' \
      'shmdt02' \
      'shmem_2nstest' \
      'shmget02' \
      'shmget03' \
      'shmget04' \
      'shmget05' \
      'shmget06' \
      'shmnstest' \
      'shmt02' \
      'shmt03' \
      'shmt04' \
      'shmt05' \
      'shmt06' \
      'shmt07' \
      'shmt08' \
      'shmt09' \
      'shmt10' \
      'socket01' \
      'socket02' \
      'socketcall01' \
      'socketcall02' \
      'socketcall03' \
      'socketpair01' \
      'socketpair02'

    ;;
  common-easy:01)
    printf '%s\n' \
      'abs01' \
      'atof01' \
      'confstr01' \
      'float_bessel' \
      'float_exp_log' \
      'float_iperb' \
      'float_power' \
      'float_trigo' \
      'fptest01' \
      'fptest02' \
      'memcmp01' \
      'memcpy01' \
      'memset01' \
      'nextafter01' \
      'string01'

    ;;
  part-b:01 | testb:01)
    printf '%s\n' \
      'ask_password.sh' \
      'assign_password.sh' \
      'binfmt_misc_lib.sh' \
      'capget01' \
      'capget02' \
      'capset01' \
      'capset02' \
      'capset03' \
      'capset04' \
      'chroot01' \
      'chroot02' \
      'chroot03' \
      'chroot04' \
      'data_space' \
      'ebizzy' \
      'ftrace_lib.sh' \
      'ftrace_regression01.sh' \
      'ftrace_regression02.sh' \
      'ftrace_stress_test.sh' \
      'getpagesize01' \
      'getrandom01' \
      'getrandom02' \
      'getrandom03' \
      'getrandom04' \
      'getrandom05' \
      'hackbench' \
      'ima_setup.sh' \
      'mallocstress' \
      'mem02' \
      'memfd_create02' \
      'mesgq_nstest' \
      'mlock01' \
      'mlock03' \
      'mlockall01' \
      'mmapstress01' \
      'mmapstress04' \
      'mmstress_dummy' \
      'page01' \
      'page02' \
      'print_caps' \
      'process_vm01' \
      'reboot01' \
      'run_cpuctl_test_fj.sh' \
      'runpwtests01.sh' \
      'runpwtests02.sh' \
      'runpwtests03.sh' \
      'runpwtests04.sh' \
      'runpwtests05.sh' \
      'runpwtests06.sh' \
      'runpwtests_exclusive01.sh' \
      'runpwtests_exclusive02.sh' \
      'runpwtests_exclusive03.sh' \
      'runpwtests_exclusive04.sh' \
      'runpwtests_exclusive05.sh' \
      'sbrk02' \
      'sched_stress.sh' \
      'set_robust_list01' \
      'smack_common.sh' \
      'smack_file_access.sh' \
      'smack_set_ambient.sh' \
      'smack_set_cipso.sh' \
      'smack_set_current.sh' \
      'smack_set_direct.sh' \
      'smack_set_doi.sh' \
      'smack_set_load.sh' \
      'smack_set_netlabel.sh' \
      'smack_set_onlycap.sh' \
      'stack_space' \
      'stream01' \
      'stream02' \
      'stream03' \
      'stream04' \
      'stream05' \
      'stress' \
      'syscall01' \
      'sysconf01' \
      'sysinfo01' \
      'sysinfo02' \
      'test_controllers.sh' \
      'test_robind.sh'

    ;;
  *)
    return 1
    ;;
  esac
}

# Auto-generated list of batch IDs for each category.
# Derived from actual files in src/init/ltp-cases/.
# ltp-safe.txt is NOT a batch file; ltp_safe_cases() is handled separately.

ltp_safe_cases() {
  printf '%s\n' "$LTP_SAFE_DATA"
}
