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
        storage|storage-safe)
            echo "01 02 03 04 05 06 07 08 09 10 11 12 13 14"
            ;;
        *)
            return 1
            ;;
    esac
}

ltp_batch_cases() {
    category="$1"
    batch="$2"

    # Map storage-safe to storage for case-list lookup;
    # dangerous-case skipping is handled at runtime in init.sh.
    [ "$category" = "storage-safe" ] && category="storage"

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
        storage:01)
            printf '%s\n' \
                'acl1' \
                'aio-stress' \
                'aio01' \
                'aio02' \
                'aiocp' \
                'aiodio_append' \
                'aiodio_sparse' \
                'block_dev' \
                'copy_file_range01' \
                'copy_file_range02' \
                'copy_file_range03' \
                'cp_tests.sh' \
                'cpio_tests.sh' \
                'creat07_child' \
                'create_datafile' \
                'create_file' \
                'df01.sh' \
                'dio_append' \
                'dio_read' \
                'dio_sparse' \
                'dio_truncate' \
                'diotest1' \
                'diotest2' \
                'diotest3' \
                'diotest4' \
                'diotest5' \
                'diotest6' \
                'dirty' \
                'dirtyc0w' \
                'dirtyc0w_child' \

            ;;
        storage:02)
            printf '%s\n' \
                'dirtyc0w_shmem' \
                'dirtyc0w_shmem_child' \
                'dirtypipe' \
                'dma_thread_diotest' \
                'doio' \
                'du01.sh' \
                'eject-tests.sh' \
                'fallocate01' \
                'fallocate02' \
                'fallocate03' \
                'fallocate04' \
                'fallocate05' \
                'fallocate06' \
                'fanotify01' \
                'fanotify02' \
                'fanotify03' \
                'fanotify04' \
                'fanotify05' \
                'fanotify06' \
                'fanotify07' \
                'fanotify08' \
                'fanotify09' \
                'fanotify10' \
                'fanotify11' \
                'fanotify12' \
                'fanotify13' \
                'fanotify14' \
                'fanotify15' \
                'fanotify16' \
                'fanotify17' \

            ;;
        storage:03)
            printf '%s\n' \
                'fanotify18' \
                'fanotify19' \
                'fanotify20' \
                'fanotify21' \
                'fanotify22' \
                'fanotify23' \
                'fanotify_child' \
                'file01.sh' \
                'flock01' \
                'flock02' \
                'flock03' \
                'flock04' \
                'flock06' \
                'fs_bind01.sh' \
                'fs_bind02.sh' \
                'fs_bind03.sh' \
                'fs_bind04.sh' \
                'fs_bind05.sh' \
                'fs_bind06.sh' \
                'fs_bind07-2.sh' \
                'fs_bind07.sh' \
                'fs_bind08.sh' \
                'fs_bind09.sh' \
                'fs_bind10.sh' \
                'fs_bind11.sh' \
                'fs_bind12.sh' \
                'fs_bind13.sh' \
                'fs_bind14.sh' \
                'fs_bind15.sh' \
                'fs_bind16.sh' \

            ;;
        storage:04)
            printf '%s\n' \
                'fs_bind17.sh' \
                'fs_bind18.sh' \
                'fs_bind19.sh' \
                'fs_bind20.sh' \
                'fs_bind21.sh' \
                'fs_bind22.sh' \
                'fs_bind23.sh' \
                'fs_bind24.sh' \
                'fs_bind_cloneNS01.sh' \
                'fs_bind_cloneNS02.sh' \
                'fs_bind_cloneNS03.sh' \
                'fs_bind_cloneNS04.sh' \
                'fs_bind_cloneNS05.sh' \
                'fs_bind_cloneNS06.sh' \
                'fs_bind_cloneNS07.sh' \
                'fs_bind_lib.sh' \
                'fs_bind_move01.sh' \
                'fs_bind_move02.sh' \
                'fs_bind_move03.sh' \
                'fs_bind_move04.sh' \
                'fs_bind_move05.sh' \
                'fs_bind_move06.sh' \
                'fs_bind_move07.sh' \
                'fs_bind_move08.sh' \
                'fs_bind_move09.sh' \
                'fs_bind_move10.sh' \
                'fs_bind_move11.sh' \
                'fs_bind_move12.sh' \
                'fs_bind_move13.sh' \
                'fs_bind_move14.sh' \

            ;;
        storage:05)
            printf '%s\n' \
                'fs_bind_move15.sh' \
                'fs_bind_move16.sh' \
                'fs_bind_move17.sh' \
                'fs_bind_move18.sh' \
                'fs_bind_move19.sh' \
                'fs_bind_move20.sh' \
                'fs_bind_move21.sh' \
                'fs_bind_move22.sh' \
                'fs_bind_rbind01.sh' \
                'fs_bind_rbind02.sh' \
                'fs_bind_rbind03.sh' \
                'fs_bind_rbind04.sh' \
                'fs_bind_rbind05.sh' \
                'fs_bind_rbind06.sh' \
                'fs_bind_rbind07-2.sh' \
                'fs_bind_rbind07.sh' \
                'fs_bind_rbind08.sh' \
                'fs_bind_rbind09.sh' \
                'fs_bind_rbind10.sh' \
                'fs_bind_rbind11.sh' \
                'fs_bind_rbind12.sh' \
                'fs_bind_rbind13.sh' \
                'fs_bind_rbind14.sh' \
                'fs_bind_rbind15.sh' \
                'fs_bind_rbind16.sh' \
                'fs_bind_rbind17.sh' \
                'fs_bind_rbind18.sh' \
                'fs_bind_rbind19.sh' \
                'fs_bind_rbind20.sh' \
                'fs_bind_rbind21.sh' \

            ;;
        storage:06)
            printf '%s\n' \
                'fs_bind_rbind22.sh' \
                'fs_bind_rbind23.sh' \
                'fs_bind_rbind24.sh' \
                'fs_bind_rbind25.sh' \
                'fs_bind_rbind26.sh' \
                'fs_bind_rbind27.sh' \
                'fs_bind_rbind28.sh' \
                'fs_bind_rbind29.sh' \
                'fs_bind_rbind30.sh' \
                'fs_bind_rbind31.sh' \
                'fs_bind_rbind32.sh' \
                'fs_bind_rbind33.sh' \
                'fs_bind_rbind34.sh' \
                'fs_bind_rbind35.sh' \
                'fs_bind_rbind36.sh' \
                'fs_bind_rbind37.sh' \
                'fs_bind_rbind38.sh' \
                'fs_bind_rbind39.sh' \
                'fs_bind_regression.sh' \
                'fs_di' \
                'fs_fill' \
                'fs_inod' \
                'fs_perms' \
                'fs_racer.sh' \
                'fs_racer_dir_create.sh' \
                'fs_racer_dir_test.sh' \
                'fs_racer_file_concat.sh' \
                'fs_racer_file_create.sh' \
                'fs_racer_file_link.sh' \
                'fs_racer_file_list.sh' \

            ;;
        storage:07)
            printf '%s\n' \
                'fs_racer_file_rename.sh' \
                'fs_racer_file_rm.sh' \
                'fs_racer_file_symlink.sh' \
                'fsconfig01' \
                'fsconfig02' \
                'fsconfig03' \
                'fsmount01' \
                'fsmount02' \
                'fsopen01' \
                'fsopen02' \
                'fspick01' \
                'fspick02' \
                'fsstress' \
                'fsx-linux' \
                'fsx.sh' \
                'ftest01' \
                'ftest02' \
                'ftest03' \
                'ftest04' \
                'ftest05' \
                'ftest06' \
                'ftest07' \
                'ftest08' \
                'futimesat01' \
                'growfiles' \
                'gzip_tests.sh' \
                'inode01' \
                'inode02' \
                'inotify01' \
                'inotify02' \

            ;;
        storage:08)
            printf '%s\n' \
                'inotify03' \
                'inotify04' \
                'inotify05' \
                'inotify06' \
                'inotify07' \
                'inotify08' \
                'inotify09' \
                'inotify10' \
                'inotify11' \
                'inotify12' \
                'inotify_init1_01' \
                'inotify_init1_02' \
                'io_cancel01' \
                'io_cancel02' \
                'io_control01' \
                'io_destroy01' \
                'io_destroy02' \
                'io_getevents01' \
                'io_getevents02' \
                'io_pgetevents01' \
                'io_pgetevents02' \
                'io_setup01' \
                'io_setup02' \
                'io_submit01' \
                'io_submit02' \
                'io_submit03' \
                'io_uring01' \
                'io_uring02' \
                'ioctl01' \
                'ioctl02' \

            ;;
        storage:09)
            printf '%s\n' \
                'ioctl03' \
                'ioctl04' \
                'ioctl05' \
                'ioctl06' \
                'ioctl07' \
                'ioctl08' \
                'ioctl09' \
                'ioctl_loop01' \
                'ioctl_loop02' \
                'ioctl_loop03' \
                'ioctl_loop04' \
                'ioctl_loop05' \
                'ioctl_loop06' \
                'ioctl_loop07' \
                'ioctl_ns01' \
                'ioctl_ns02' \
                'ioctl_ns03' \
                'ioctl_ns04' \
                'ioctl_ns05' \
                'ioctl_ns06' \
                'ioctl_ns07' \
                'ioctl_sg01' \
                'isofs.sh' \
                'lftest' \
                'ln_tests.sh' \
                'locktests' \
                'logrotate_tests.sh' \
                'mkfs01.sh' \
                'mknod01' \
                'mknod02' \

            ;;
        storage:10)
            printf '%s\n' \
                'mknod03' \
                'mknod04' \
                'mknod05' \
                'mknod06' \
                'mknod07' \
                'mknod08' \
                'mknod09' \
                'mknodat01' \
                'mknodat02' \
                'mkswap01.sh' \
                'mount01' \
                'mount02' \
                'mount03' \
                'mount03_suid_child' \
                'mount04' \
                'mount05' \
                'mount06' \
                'mount07' \
                'mount_setattr01' \
                'mountns01' \
                'mountns02' \
                'mountns03' \
                'mountns04' \
                'move_mount01' \
                'move_mount02' \
                'mv_tests.sh' \
                'name_to_handle_at01' \
                'name_to_handle_at02' \
                'nfs01.sh' \
                'nfs01_open_files' \

            ;;
        storage:11)
            printf '%s\n' \
                'nfs02.sh' \
                'nfs03.sh' \
                'nfs04.sh' \
                'nfs04_create_file' \
                'nfs05.sh' \
                'nfs05_make_tree' \
                'nfs06.sh' \
                'nfs07.sh' \
                'nfs08.sh' \
                'nfs09.sh' \
                'nfs_flock' \
                'nfs_flock_dgen' \
                'nfs_lib.sh' \
                'nfslock01.sh' \
                'nfsstat01.sh' \
                'nftw01' \
                'nftw6401' \
                'open12_child' \
                'open_by_handle_at01' \
                'open_by_handle_at02' \
                'open_tree01' \
                'open_tree02' \
                'openat02_child' \
                'posix_fadvise01' \
                'posix_fadvise01_64' \
                'posix_fadvise02' \
                'posix_fadvise02_64' \
                'posix_fadvise03' \
                'posix_fadvise03_64' \
                'posix_fadvise04' \

            ;;
        storage:12)
            printf '%s\n' \
                'posix_fadvise04_64' \
                'quota_remount_test01.sh' \
                'quotactl01' \
                'quotactl02' \
                'quotactl03' \
                'quotactl04' \
                'quotactl05' \
                'quotactl06' \
                'quotactl07' \
                'quotactl08' \
                'quotactl09' \
                'read_all' \
                'readahead01' \
                'readahead02' \
                'rwtest' \
                'sendfile01.sh' \
                'sendfile02' \
                'sendfile02_64' \
                'sendfile03' \
                'sendfile03_64' \
                'sendfile04' \
                'sendfile04_64' \
                'sendfile05' \
                'sendfile05_64' \
                'sendfile06' \
                'sendfile06_64' \
                'sendfile07' \
                'sendfile07_64' \
                'sendfile08' \
                'sendfile08_64' \

            ;;
        storage:13)
            printf '%s\n' \
                'sendfile09' \
                'sendfile09_64' \
                'shell_pipe01.sh' \
                'splice01' \
                'splice02' \
                'splice03' \
                'splice04' \
                'splice05' \
                'splice06' \
                'splice07' \
                'splice08' \
                'splice09' \
                'squashfs01' \
                'sysfs01' \
                'sysfs02' \
                'sysfs03' \
                'sysfs04' \
                'sysfs05' \
                'tar_tests.sh' \
                'tbio' \
                'tee01' \
                'tee02' \
                'tst_device' \
                'tst_fs_has_free' \
                'tst_fsfreeze' \
                'umask01' \
                'umount01' \
                'umount02' \
                'umount03' \
                'umount2_01' \

            ;;
        storage:14)
            printf '%s\n' \
                'umount2_02' \
                'unzip01.sh' \
                'ustat01' \
                'ustat02' \
                'utime01' \
                'utime02' \
                'utime03' \
                'utime04' \
                'utime05' \
                'utime06' \
                'utime07' \
                'utimensat01' \
                'utimes01' \
                'vmsplice01' \
                'vmsplice02' \
                'vmsplice03' \
                'vmsplice04' \
                'wc01.sh' \
                'zram01.sh' \
                'zram02.sh' \
                'zram03' \
                'zram_lib.sh' \

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
