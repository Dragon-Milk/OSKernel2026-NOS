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
        mm-ipc-security)
            echo "01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18"
            ;;
        common-easy)
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
                'acct01' \
                'acct02' \
                'adjtimex01' \
                'adjtimex02' \
                'adjtimex03' \
                'alarm02' \
                'alarm03' \
                'alarm05' \
                'alarm06' \
                'alarm07' \
                'arch_prctl01' \
                'autogroup01' \
                'clock_adjtime01' \
                'clock_adjtime02' \
                'clock_getres01' \
                'clock_gettime01' \
                'clock_gettime02' \
                'clock_gettime03' \
                'clock_gettime04' \
                'clock_nanosleep01' \
                'clock_nanosleep02' \
                'clock_nanosleep03' \
                'clock_nanosleep04' \
                'clock_settime01' \
                'clock_settime02' \
                'clock_settime03' \
                'clone01' \
                'clone02' \
                'clone03'
            ;;
        process:02)
            printf '%s\n' \
                'clone04' \
                'clone05' \
                'clone06' \
                'clone07' \
                'clone08' \
                'clone09' \
                'clone301' \
                'clone302' \
                'clone303' \
                'exec_with_inh' \
                'exec_without_inh' \
                'execl01' \
                'execle01' \
                'execlp01' \
                'execv01' \
                'execve01' \
                'execve02' \
                'execve03' \
                'execve04' \
                'execve05' \
                'execve06' \
                'execveat01' \
                'execveat02' \
                'execveat03' \
                'execveat_errno' \
                'execvp01' \
                'exit01' \
                'exit02' \
                'exit_group01' \
                'fork01'
            ;;
        process:03)
            printf '%s\n' \
                'fork03' \
                'fork04' \
                'fork05' \
                'fork07' \
                'fork08' \
                'fork09' \
                'fork10' \
                'fork13' \
                'fork14' \
                'fork_exec_loop' \
                'fork_procs' \
                'getcontext01' \
                'getcpu01' \
                'getdomainname01' \
                'getegid01' \
                'getegid01_16' \
                'getegid02' \
                'getegid02_16' \
                'geteuid01' \
                'geteuid01_16' \
                'geteuid02' \
                'geteuid02_16' \
                'getgid01' \
                'getgid01_16' \
                'getgid03' \
                'getgid03_16' \
                'getgroups01' \
                'getgroups01_16' \
                'getgroups03' \
                'getgroups03_16'
            ;;
        process:04)
            printf '%s\n' \
                'gethostname01' \
                'gethostname02' \
                'getitimer01' \
                'getitimer02' \
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
                'getresgid01_16' \
                'getresgid02' \
                'getresgid02_16' \
                'getresgid03' \
                'getresgid03_16' \
                'getresuid01' \
                'getresuid01_16' \
                'getresuid02' \
                'getresuid02_16' \
                'getresuid03' \
                'getresuid03_16' \
                'getrlimit01' \
                'getrlimit02' \
                'getrlimit03' \
                'getrusage01' \
                'getrusage02'
            ;;
        process:05)
            printf '%s\n' \
                'getrusage03' \
                'getrusage04' \
                'getsid01' \
                'getsid02' \
                'gettid01' \
                'gettid02' \
                'gettimeofday01' \
                'gettimeofday02' \
                'getuid01' \
                'getuid01_16' \
                'getuid03' \
                'getuid03_16' \
                'hangup01' \
                'ioprio_get01' \
                'ioprio_set01' \
                'ioprio_set02' \
                'ioprio_set03' \
                'kcmp01' \
                'kcmp02' \
                'kcmp03' \
                'kill02' \
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
                'kill13' \
                'leapsec01' \
                'nanosleep01' \
                'nanosleep02' \
                'nanosleep04' \
                'newuname01' \
                'nice01' \
                'nice02' \
                'nice03' \
                'nice04' \
                'nice05' \
                'nptl01' \
                'pause01' \
                'pause02' \
                'pause03' \
                'personality01' \
                'personality02' \
                'pidfd_getfd01' \
                'pidfd_getfd02' \
                'pidfd_open01' \
                'pidfd_open02' \
                'pidfd_open03' \
                'pidfd_open04' \
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
                'prctl06' \
                'prctl06_execve' \
                'prctl07' \
                'prctl08' \
                'prctl09' \
                'prctl10' \
                'proc_sched_rt01' \
                'pth_str01' \
                'pth_str02' \
                'pth_str03' \
                'ptrace01' \
                'ptrace02' \
                'ptrace03' \
                'ptrace04' \
                'ptrace05' \
                'ptrace06' \
                'ptrace07' \
                'ptrace08' \
                'ptrace09' \
                'ptrace10' \
                'ptrace11' \
                'rt_sigaction01' \
                'rt_sigaction02' \
                'rt_sigaction03' \
                'rt_sigprocmask01' \
                'rt_sigprocmask02' \
                'rt_sigqueueinfo01' \
                'rt_sigsuspend01' \
                'rtc01'
            ;;
        process:08)
            printf '%s\n' \
                'rtc02' \
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
                'sched_tc0' \
                'sched_tc1' \
                'sched_tc2' \
                'sched_tc3'
            ;;
        process:09)
            printf '%s\n' \
                'sched_tc4' \
                'sched_tc5' \
                'sched_tc6' \
                'sched_yield01' \
                'set_thread_area01' \
                'set_tid_address01' \
                'setdomainname01' \
                'setdomainname02' \
                'setdomainname03' \
                'setegid01' \
                'setegid02' \
                'setfsgid01' \
                'setfsgid01_16' \
                'setfsgid02' \
                'setfsgid02_16' \
                'setfsgid03' \
                'setfsgid03_16' \
                'setfsuid01' \
                'setfsuid01_16' \
                'setfsuid02' \
                'setfsuid02_16' \
                'setfsuid03' \
                'setfsuid03_16' \
                'setfsuid04' \
                'setfsuid04_16' \
                'setgid01' \
                'setgid01_16' \
                'setgid02' \
                'setgid02_16' \
                'setgid03'
            ;;
        process:10)
            printf '%s\n' \
                'setgid03_16' \
                'setgroups01' \
                'setgroups01_16' \
                'setgroups02' \
                'setgroups02_16' \
                'setgroups03' \
                'setgroups03_16' \
                'setgroups04' \
                'setgroups04_16' \
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
                'setpriority02' \
                'setregid01' \
                'setregid01_16' \
                'setregid02' \
                'setregid02_16' \
                'setregid03' \
                'setregid03_16' \
                'setregid04' \
                'setregid04_16' \
                'setresgid01'
            ;;
        process:11)
            printf '%s\n' \
                'setresgid01_16' \
                'setresgid02' \
                'setresgid02_16' \
                'setresgid03' \
                'setresgid03_16' \
                'setresgid04' \
                'setresgid04_16' \
                'setresuid01' \
                'setresuid01_16' \
                'setresuid02' \
                'setresuid02_16' \
                'setresuid03' \
                'setresuid03_16' \
                'setresuid04' \
                'setresuid04_16' \
                'setresuid05' \
                'setresuid05_16' \
                'setreuid01' \
                'setreuid01_16' \
                'setreuid02' \
                'setreuid02_16' \
                'setreuid03' \
                'setreuid03_16' \
                'setreuid04' \
                'setreuid04_16' \
                'setreuid05' \
                'setreuid05_16' \
                'setreuid06' \
                'setreuid06_16' \
                'setreuid07'
            ;;
        process:12)
            printf '%s\n' \
                'setreuid07_16' \
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
                'setuid01_16' \
                'setuid03' \
                'setuid03_16' \
                'setuid04' \
                'setuid04_16' \
                'sgetmask01' \
                'sigaction01' \
                'sigaction02' \
                'sigaltstack01' \
                'sigaltstack02' \
                'sighold02' \
                'signal01' \
                'signal02' \
                'signal03' \
                'signal04' \
                'signal05' \
                'signal06' \
                'signalfd01' \
                'signalfd4_01'
            ;;
        process:13)
            printf '%s\n' \
                'signalfd4_02' \
                'sigpending02' \
                'sigprocmask01' \
                'sigrelse01' \
                'sigsuspend01' \
                'sigtimedwait01' \
                'sigwait01' \
                'sigwaitinfo01' \
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
                'timerfd02' \
                'timerfd04' \
                'timerfd_create01' \
                'timerfd_gettime01' \
                'timerfd_settime01' \
                'timerfd_settime02'
            ;;
        process:14)
            printf '%s\n' \
                'times01' \
                'times03' \
                'tkill01' \
                'tkill02' \
                'ulimit01' \
                'uname01' \
                'uname02' \
                'uname04' \
                'utsname01' \
                'utsname02' \
                'utsname03' \
                'utsname04' \
                'vfork' \
                'vfork01' \
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
                'chown01_16' \
                'chown02' \
                'chown02_16' \
                'chown03' \
                'chown03_16' \
                'chown04' \
                'chown04_16' \
                'chown05' \
                'chown05_16' \
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
                'fchown01_16' \
                'fchown02' \
                'fchown02_16' \
                'fchown03' \
                'fchown03_16' \
                'fchown04' \
                'fchown04_16' \
                'fchown05' \
                'fchown05_16' \
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
                'lchown01_16' \
                'lchown02' \
                'lchown02_16' \
                'lchown03' \
                'lchown03_16' \
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
        mm-ipc-security:01)
            printf '%s\n' \
                'acct02_helper' \
                'add_key01' \
                'add_key02' \
                'add_key03' \
                'add_key04' \
                'add_key05' \
                'ask_password.sh' \
                'aslr01' \
                'assign_password.sh' \
                'binfmt_misc01.sh' \
                'binfmt_misc02.sh' \
                'binfmt_misc_lib.sh' \
                'bpf_map01' \
                'bpf_prog01' \
                'bpf_prog02' \
                'bpf_prog03' \
                'bpf_prog04' \
                'bpf_prog05' \
                'bpf_prog06' \
                'bpf_prog07' \
                'cap_bounds_r' \
                'cap_bounds_rw' \
                'cap_bset_inh_bounds' \
                'capget01' \
                'capget02' \
                'capset01' \
                'capset02' \
                'capset03' \
                'capset04' \
                'cfs_bandwidth01'
            ;;
        mm-ipc-security:02)
            printf '%s\n' \
                'cgroup_core01' \
                'cgroup_core02' \
                'cgroup_core03' \
                'cgroup_fj_common.sh' \
                'cgroup_fj_function.sh' \
                'cgroup_fj_proc' \
                'cgroup_fj_stress.sh' \
                'cgroup_lib.sh' \
                'cgroup_regression_3_1.sh' \
                'cgroup_regression_3_2.sh' \
                'cgroup_regression_5_1.sh' \
                'cgroup_regression_5_2.sh' \
                'cgroup_regression_6_1.sh' \
                'cgroup_regression_6_2.sh' \
                'cgroup_regression_fork_processes' \
                'cgroup_regression_getdelays' \
                'cgroup_regression_test.sh' \
                'cgroup_xattr' \
                'change_password.sh' \
                'check_keepcaps' \
                'check_pe' \
                'check_setkey' \
                'check_simple_capset' \
                'chroot01' \
                'chroot02' \
                'chroot03' \
                'chroot04' \
                'cpuacct.sh' \
                'cpuacct_task' \
                'cpuctl_def_task01'
            ;;
        mm-ipc-security:03)
            printf '%s\n' \
                'cpuctl_def_task02' \
                'cpuctl_def_task03' \
                'cpuctl_def_task04' \
                'cpuctl_fj_cpu-hog' \
                'cpuctl_fj_simple_echo' \
                'cpuctl_latency_check_task' \
                'cpuctl_latency_test' \
                'cpuctl_test01' \
                'cpuctl_test02' \
                'cpuctl_test03' \
                'cpuctl_test04' \
                'cpuset01' \
                'cpuset_base_ops_testset.sh' \
                'cpuset_cpu_hog' \
                'cpuset_exclusive_test.sh' \
                'cpuset_funcs.sh' \
                'cpuset_hierarchy_test.sh' \
                'cpuset_hotplug_test.sh' \
                'cpuset_inherit_testset.sh' \
                'cpuset_list_compute' \
                'cpuset_load_balance_test.sh' \
                'cpuset_mem_hog' \
                'cpuset_memory_pressure' \
                'cpuset_memory_pressure_testset.sh' \
                'cpuset_memory_spread_testset.sh' \
                'cpuset_memory_test' \
                'cpuset_memory_testset.sh' \
                'cpuset_regression_test.sh' \
                'cpuset_sched_domains_check' \
                'cpuset_sched_domains_test.sh'
            ;;
        mm-ipc-security:04)
            printf '%s\n' \
                'cpuset_syscall_test' \
                'cpuset_syscall_testset.sh' \
                'crypto_user01' \
                'crypto_user02' \
                'cve-2014-0196' \
                'cve-2015-3290' \
                'cve-2016-10044' \
                'cve-2016-7042' \
                'cve-2016-7117' \
                'cve-2017-16939' \
                'cve-2017-17052' \
                'cve-2017-17053' \
                'cve-2017-2618' \
                'cve-2017-2671' \
                'cve-2022-4378' \
                'data' \
                'data_space' \
                'ebizzy' \
                'event_generator' \
                'evm_overlay.sh' \
                'execl01_child' \
                'execle01_child' \
                'execlp01_child' \
                'execv01_child' \
                'execve01_child' \
                'execve06_child' \
                'execve_child' \
                'execveat_child' \
                'execvp01_child' \
                'filecapstest.sh'
            ;;
        mm-ipc-security:05)
            printf '%s\n' \
                'fork_freeze.sh' \
                'freeze_cancel.sh' \
                'freeze_kill_thaw.sh' \
                'freeze_move_thaw.sh' \
                'freeze_self_thaw.sh' \
                'freeze_sleep_thaw.sh' \
                'freeze_thaw.sh' \
                'freeze_write_freezing.sh' \
                'ftrace_lib.sh' \
                'ftrace_regression01.sh' \
                'ftrace_regression02.sh' \
                'ftrace_stress_test.sh' \
                'gdb01.sh' \
                'get_mempolicy01' \
                'get_mempolicy02' \
                'get_robust_list01' \
                'gethostid01' \
                'getpagesize01' \
                'getrandom01' \
                'getrandom02' \
                'getrandom03' \
                'getrandom04' \
                'getrandom05' \
                'getrusage03_child' \
                'hackbench' \
                'hugefallocate01' \
                'hugefallocate02' \
                'hugefork01' \
                'hugefork02' \
                'hugemmap01'
            ;;
        mm-ipc-security:06)
            printf '%s\n' \
                'hugemmap02' \
                'hugemmap04' \
                'hugemmap05' \
                'hugemmap06' \
                'hugemmap07' \
                'hugemmap08' \
                'hugemmap09' \
                'hugemmap10' \
                'hugemmap11' \
                'hugemmap12' \
                'hugemmap13' \
                'hugemmap14' \
                'hugemmap15' \
                'hugemmap16' \
                'hugemmap17' \
                'hugemmap18' \
                'hugemmap19' \
                'hugemmap20' \
                'hugemmap21' \
                'hugemmap22' \
                'hugemmap23' \
                'hugemmap24' \
                'hugemmap25' \
                'hugemmap26' \
                'hugemmap27' \
                'hugemmap28' \
                'hugemmap29' \
                'hugemmap30' \
                'hugemmap31' \
                'hugemmap32'
            ;;
        mm-ipc-security:07)
            printf '%s\n' \
                'hugeshmat01' \
                'hugeshmat02' \
                'hugeshmat03' \
                'hugeshmat04' \
                'hugeshmat05' \
                'hugeshmctl01' \
                'hugeshmctl02' \
                'hugeshmctl03' \
                'hugeshmdt01' \
                'hugeshmget01' \
                'hugeshmget02' \
                'hugeshmget03' \
                'hugeshmget05' \
                'ima_boot_aggregate' \
                'ima_conditionals.sh' \
                'ima_kexec.sh' \
                'ima_keys.sh' \
                'ima_measurements.sh' \
                'ima_mmap' \
                'ima_policy.sh' \
                'ima_selinux.sh' \
                'ima_setup.sh' \
                'ima_tpm.sh' \
                'ima_violations.sh' \
                'inh_capped' \
                'iogen' \
                'keyctl01' \
                'keyctl01.sh' \
                'keyctl02' \
                'keyctl03'
            ;;
        mm-ipc-security:08)
            printf '%s\n' \
                'keyctl04' \
                'keyctl05' \
                'keyctl06' \
                'keyctl07' \
                'keyctl08' \
                'keyctl09' \
                'ksm01' \
                'ksm02' \
                'ksm03' \
                'ksm04' \
                'ksm05' \
                'ksm06' \
                'ksm07' \
                'libcgroup_freezer' \
                'lock_torture.sh' \
                'mallinfo01' \
                'mallinfo02' \
                'mallinfo2_01' \
                'mallocstress' \
                'mallopt01' \
                'max_map_count' \
                'mbind01' \
                'mbind02' \
                'mbind03' \
                'mbind04' \
                'meltdown' \
                'mem02' \
                'mem_process' \
                'membarrier01' \
                'memcg_control_test.sh'
            ;;
        mm-ipc-security:09)
            printf '%s\n' \
                'memcg_failcnt.sh' \
                'memcg_force_empty.sh' \
                'memcg_lib.sh' \
                'memcg_limit_in_bytes.sh' \
                'memcg_max_usage_in_bytes_test.sh' \
                'memcg_memsw_limit_in_bytes_test.sh' \
                'memcg_move_charge_at_immigrate_test.sh' \
                'memcg_process' \
                'memcg_process_stress' \
                'memcg_regression_test.sh' \
                'memcg_stat_rss.sh' \
                'memcg_stat_test.sh' \
                'memcg_stress_test.sh' \
                'memcg_subgroup_charge.sh' \
                'memcg_test_1' \
                'memcg_test_2' \
                'memcg_test_3' \
                'memcg_test_4' \
                'memcg_test_4.sh' \
                'memcg_usage_in_bytes_test.sh' \
                'memcg_use_hierarchy_test.sh' \
                'memcontrol01' \
                'memcontrol02' \
                'memcontrol03' \
                'memcontrol04' \
                'memctl_test01' \
                'memfd_create01' \
                'memfd_create02' \
                'memfd_create03' \
                'memfd_create04'
            ;;
        mm-ipc-security:10)
            printf '%s\n' \
                'memtoy' \
                'mesgq_nstest' \
                'migrate_pages01' \
                'migrate_pages02' \
                'migrate_pages03' \
                'min_free_kbytes' \
                'mlock01' \
                'mlock02' \
                'mlock03' \
                'mlock04' \
                'mlock05' \
                'mlock201' \
                'mlock202' \
                'mlock203' \
                'mlockall01' \
                'mlockall02' \
                'mlockall03' \
                'mmapstress01' \
                'mmapstress02' \
                'mmapstress03' \
                'mmapstress04' \
                'mmapstress05' \
                'mmapstress06' \
                'mmapstress07' \
                'mmapstress08' \
                'mmapstress09' \
                'mmapstress10' \
                'mmstress' \
                'mmstress_dummy' \
                'move_pages01'
            ;;
        mm-ipc-security:11)
            printf '%s\n' \
                'move_pages02' \
                'move_pages03' \
                'move_pages04' \
                'move_pages05' \
                'move_pages06' \
                'move_pages07' \
                'move_pages09' \
                'move_pages10' \
                'move_pages11' \
                'move_pages12' \
                'mqns_01' \
                'mqns_02' \
                'mqns_03' \
                'mqns_04' \
                'msgstress01' \
                'mtest01' \
                'munlock01' \
                'munlock02' \
                'munlockall01' \
                'numa01.sh' \
                'oom01' \
                'oom02' \
                'oom03' \
                'oom04' \
                'oom05' \
                'overcommit_memory' \
                'page01' \
                'page02' \
                'pcrypt_aead01' \
                'perf_event_open01'
            ;;
        mm-ipc-security:12)
            printf '%s\n' \
                'perf_event_open02' \
                'perf_event_open03' \
                'pidns01' \
                'pidns02' \
                'pidns03' \
                'pidns04' \
                'pidns05' \
                'pidns06' \
                'pidns10' \
                'pidns12' \
                'pidns13' \
                'pidns16' \
                'pidns17' \
                'pidns20' \
                'pidns30' \
                'pidns31' \
                'pidns32' \
                'pids.sh' \
                'pids_task1' \
                'pids_task2' \
                'pipe2_02_child' \
                'pivot_root01' \
                'pkey01' \
                'print_caps' \
                'proc01' \
                'process_madvise01' \
                'process_vm01' \
                'process_vm_readv02' \
                'process_vm_readv03' \
                'process_vm_writev02'
            ;;
        mm-ipc-security:13)
            printf '%s\n' \
                'profil01' \
                'prot_hsymlinks' \
                'pt_test' \
                'ptem01' \
                'pthcli' \
                'pthserv' \
                'pty01' \
                'pty02' \
                'pty03' \
                'pty04' \
                'pty05' \
                'pty06' \
                'pty07' \
                'rcu_torture.sh' \
                'reboot01' \
                'reboot02' \
                'remap_file_pages01' \
                'remap_file_pages02' \
                'remove_password.sh' \
                'request_key01' \
                'request_key02' \
                'request_key03' \
                'request_key04' \
                'request_key05' \
                'run_capbounds.sh' \
                'run_cpuctl_latency_test.sh' \
                'run_cpuctl_stress_test.sh' \
                'run_cpuctl_test.sh' \
                'run_cpuctl_test_fj.sh' \
                'run_freezer.sh'
            ;;
        mm-ipc-security:14)
            printf '%s\n' \
                'run_memctl_test.sh' \
                'run_sched_cliserv.sh' \
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
                'sbrk01' \
                'sbrk02' \
                'sbrk03' \
                'sched_datafile' \
                'sched_driver' \
                'sched_stress.sh' \
                'set_mempolicy01' \
                'set_mempolicy02' \
                'set_mempolicy03' \
                'set_mempolicy04' \
                'set_mempolicy05' \
                'set_robust_list01' \
                'setns01' \
                'setns02' \
                'setpgid03_child' \
                'smack_common.sh' \
                'smack_file_access.sh'
            ;;
        mm-ipc-security:15)
            printf '%s\n' \
                'smack_notroot' \
                'smack_set_ambient.sh' \
                'smack_set_cipso.sh' \
                'smack_set_current.sh' \
                'smack_set_direct.sh' \
                'smack_set_doi.sh' \
                'smack_set_load.sh' \
                'smack_set_netlabel.sh' \
                'smack_set_onlycap.sh' \
                'smack_set_socket_labels' \
                'ssh-stress.sh' \
                'stack_clash' \
                'stack_space' \
                'starvation' \
                'stop_freeze_sleep_thaw_cont.sh' \
                'stop_freeze_thaw_cont.sh' \
                'stream01' \
                'stream02' \
                'stream03' \
                'stream04' \
                'stream05' \
                'stress' \
                'support_numa' \
                'swapoff01' \
                'swapoff02' \
                'swapon01' \
                'swapon02' \
                'swapon03' \
                'swapping01' \
                'syscall01'
            ;;
        mm-ipc-security:16)
            printf '%s\n' \
                'sysconf01' \
                'sysctl01' \
                'sysctl01.sh' \
                'sysctl02.sh' \
                'sysctl03' \
                'sysctl04' \
                'sysinfo01' \
                'sysinfo02' \
                'sysinfo03' \
                'syslog11' \
                'syslog12' \
                'test_controllers.sh' \
                'test_robind.sh' \
                'thp01' \
                'thp02' \
                'thp03' \
                'thp04' \
                'timed_forkbomb' \
                'timens01' \
                'tst_brk' \
                'tst_brkm' \
                'tst_cgctl' \
                'tst_check_drivers' \
                'tst_check_kconfigs' \
                'tst_checkpoint' \
                'tst_exit' \
                'tst_get_free_pids' \
                'tst_get_median' \
                'tst_get_unused_port' \
                'tst_getconf'
            ;;
        mm-ipc-security:17)
            printf '%s\n' \
                'tst_hexdump' \
                'tst_kvcmp' \
                'tst_lockdown_enabled' \
                'tst_ncpus' \
                'tst_ncpus_conf' \
                'tst_ncpus_max' \
                'tst_ns_create' \
                'tst_ns_exec' \
                'tst_ns_ifmove' \
                'tst_random' \
                'tst_res' \
                'tst_resm' \
                'tst_rod' \
                'tst_secureboot_enabled' \
                'tst_security.sh' \
                'tst_sleep' \
                'tst_supported_fs' \
                'tst_test.sh' \
                'tst_timeout_kill' \
                'unshare01' \
                'unshare01.sh' \
                'unshare02' \
                'userfaultfd01' \
                'userns01' \
                'userns02' \
                'userns03' \
                'userns04' \
                'userns05' \
                'userns06' \
                'userns06_capcheck'
            ;;
        mm-ipc-security:18)
            printf '%s\n' \
                'userns07' \
                'userns08' \
                'verify_caps_exec' \
                'vfork_freeze.sh' \
                'vhangup01' \
                'vhangup02' \
                'vma01' \
                'vma02' \
                'vma03' \
                'vma04' \
                'vma05.sh' \
                'vma05_vdso' \
                'wqueue01' \
                'wqueue02' \
                'wqueue03' \
                'wqueue04' \
                'wqueue05' \
                'wqueue06' \
                'wqueue07' \
                'wqueue08' \
                'wqueue09' \
                'write_freezing.sh'
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
