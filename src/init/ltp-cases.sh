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
        net|net-core)
            echo "01 02 03"
            ;;
        net-all)
            echo "01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22"
            ;;
        net-script)
            echo "01 02 03 04 05 06 07 08 09 10 11 12 13 14 15"
            ;;
        net-deferred)
            echo "01 02 03 04"
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
            for n in \
                abort01 \
                acct01 \
                acct02 \
                adjtimex01 \
                adjtimex02 \
                adjtimex03 \
                alarm02 \
                alarm03 \
                alarm05 \
                alarm06 \
                alarm07 \
                arch_prctl01 \
                autogroup01 \
                clock_adjtime01 \
                clock_adjtime02 \
                clock_getres01 \
                clock_gettime01 \
                clock_gettime02 \
                clock_gettime03 \
                clock_gettime04 \
                clock_nanosleep01 \
                clock_nanosleep02 \
                clock_nanosleep03 \
                clock_nanosleep04 \
                clock_settime01 \
                clock_settime02 \
                clock_settime03 \
                clone01 \
                clone02 \
                clone03
            do echo "$n"; done
            ;;
        process:02)
            for n in \
                clone04 \
                clone05 \
                clone06 \
                clone07 \
                clone08 \
                clone09 \
                clone301 \
                clone302 \
                clone303 \
                exec_with_inh \
                exec_without_inh \
                execl01 \
                execle01 \
                execlp01 \
                execv01 \
                execve01 \
                execve02 \
                execve03 \
                execve04 \
                execve05 \
                execve06 \
                execveat01 \
                execveat02 \
                execveat03 \
                execveat_errno \
                execvp01 \
                exit01 \
                exit02 \
                exit_group01 \
                fork01
            do echo "$n"; done
            ;;
        process:03)
            for n in \
                fork03 \
                fork04 \
                fork05 \
                fork07 \
                fork08 \
                fork09 \
                fork10 \
                fork13 \
                fork14 \
                fork_exec_loop \
                fork_procs \
                getcontext01 \
                getcpu01 \
                getdomainname01 \
                getegid01 \
                getegid01_16 \
                getegid02 \
                getegid02_16 \
                geteuid01 \
                geteuid01_16 \
                geteuid02 \
                geteuid02_16 \
                getgid01 \
                getgid01_16 \
                getgid03 \
                getgid03_16 \
                getgroups01 \
                getgroups01_16 \
                getgroups03 \
                getgroups03_16
            do echo "$n"; done
            ;;
        process:04)
            for n in \
                gethostname01 \
                gethostname02 \
                getitimer01 \
                getitimer02 \
                getpgid01 \
                getpgid02 \
                getpgrp01 \
                getpid01 \
                getpid02 \
                getppid01 \
                getppid02 \
                getpriority01 \
                getpriority02 \
                getresgid01 \
                getresgid01_16 \
                getresgid02 \
                getresgid02_16 \
                getresgid03 \
                getresgid03_16 \
                getresuid01 \
                getresuid01_16 \
                getresuid02 \
                getresuid02_16 \
                getresuid03 \
                getresuid03_16 \
                getrlimit01 \
                getrlimit02 \
                getrlimit03 \
                getrusage01 \
                getrusage02
            do echo "$n"; done
            ;;
        process:05)
            for n in \
                getrusage03 \
                getrusage04 \
                getsid01 \
                getsid02 \
                gettid01 \
                gettid02 \
                gettimeofday01 \
                gettimeofday02 \
                getuid01 \
                getuid01_16 \
                getuid03 \
                getuid03_16 \
                hangup01 \
                ioprio_get01 \
                ioprio_set01 \
                ioprio_set02 \
                ioprio_set03 \
                kcmp01 \
                kcmp02 \
                kcmp03 \
                kill02 \
                kill03 \
                kill05 \
                kill06 \
                kill07 \
                kill08 \
                kill09 \
                kill10 \
                kill11 \
                kill12
            do echo "$n"; done
            ;;
        process:06)
            for n in \
                kill13 \
                leapsec01 \
                nanosleep01 \
                nanosleep02 \
                nanosleep04 \
                newuname01 \
                nice01 \
                nice02 \
                nice03 \
                nice04 \
                nice05 \
                nptl01 \
                pause01 \
                pause02 \
                pause03 \
                personality01 \
                personality02 \
                pidfd_getfd01 \
                pidfd_getfd02 \
                pidfd_open01 \
                pidfd_open02 \
                pidfd_open03 \
                pidfd_open04 \
                pidfd_send_signal01 \
                pidfd_send_signal02 \
                pidfd_send_signal03 \
                prctl01 \
                prctl02 \
                prctl03 \
                prctl04
            do echo "$n"; done
            ;;
        process:07)
            for n in \
                prctl05 \
                prctl06 \
                prctl06_execve \
                prctl07 \
                prctl08 \
                prctl09 \
                prctl10 \
                proc_sched_rt01 \
                pth_str01 \
                pth_str02 \
                pth_str03 \
                ptrace01 \
                ptrace02 \
                ptrace03 \
                ptrace04 \
                ptrace05 \
                ptrace06 \
                ptrace07 \
                ptrace08 \
                ptrace09 \
                ptrace10 \
                ptrace11 \
                rt_sigaction01 \
                rt_sigaction02 \
                rt_sigaction03 \
                rt_sigprocmask01 \
                rt_sigprocmask02 \
                rt_sigqueueinfo01 \
                rt_sigsuspend01 \
                rtc01
            do echo "$n"; done
            ;;
        process:08)
            for n in \
                rtc02 \
                sched_get_priority_max01 \
                sched_get_priority_max02 \
                sched_get_priority_min01 \
                sched_get_priority_min02 \
                sched_getaffinity01 \
                sched_getattr01 \
                sched_getattr02 \
                sched_getparam01 \
                sched_getparam03 \
                sched_getscheduler01 \
                sched_getscheduler02 \
                sched_rr_get_interval01 \
                sched_rr_get_interval02 \
                sched_rr_get_interval03 \
                sched_setaffinity01 \
                sched_setattr01 \
                sched_setparam01 \
                sched_setparam02 \
                sched_setparam03 \
                sched_setparam04 \
                sched_setparam05 \
                sched_setscheduler01 \
                sched_setscheduler02 \
                sched_setscheduler03 \
                sched_setscheduler04 \
                sched_tc0 \
                sched_tc1 \
                sched_tc2 \
                sched_tc3
            do echo "$n"; done
            ;;
        process:09)
            for n in \
                sched_tc4 \
                sched_tc5 \
                sched_tc6 \
                sched_yield01 \
                set_thread_area01 \
                set_tid_address01 \
                setdomainname01 \
                setdomainname02 \
                setdomainname03 \
                setegid01 \
                setegid02 \
                setfsgid01 \
                setfsgid01_16 \
                setfsgid02 \
                setfsgid02_16 \
                setfsgid03 \
                setfsgid03_16 \
                setfsuid01 \
                setfsuid01_16 \
                setfsuid02 \
                setfsuid02_16 \
                setfsuid03 \
                setfsuid03_16 \
                setfsuid04 \
                setfsuid04_16 \
                setgid01 \
                setgid01_16 \
                setgid02 \
                setgid02_16 \
                setgid03
            do echo "$n"; done
            ;;
        process:10)
            for n in \
                setgid03_16 \
                setgroups01 \
                setgroups01_16 \
                setgroups02 \
                setgroups02_16 \
                setgroups03 \
                setgroups03_16 \
                setgroups04 \
                setgroups04_16 \
                sethostname01 \
                sethostname02 \
                sethostname03 \
                setitimer01 \
                setitimer02 \
                setpgid01 \
                setpgid02 \
                setpgid03 \
                setpgrp01 \
                setpgrp02 \
                setpriority01 \
                setpriority02 \
                setregid01 \
                setregid01_16 \
                setregid02 \
                setregid02_16 \
                setregid03 \
                setregid03_16 \
                setregid04 \
                setregid04_16 \
                setresgid01
            do echo "$n"; done
            ;;
        process:11)
            for n in \
                setresgid01_16 \
                setresgid02 \
                setresgid02_16 \
                setresgid03 \
                setresgid03_16 \
                setresgid04 \
                setresgid04_16 \
                setresuid01 \
                setresuid01_16 \
                setresuid02 \
                setresuid02_16 \
                setresuid03 \
                setresuid03_16 \
                setresuid04 \
                setresuid04_16 \
                setresuid05 \
                setresuid05_16 \
                setreuid01 \
                setreuid01_16 \
                setreuid02 \
                setreuid02_16 \
                setreuid03 \
                setreuid03_16 \
                setreuid04 \
                setreuid04_16 \
                setreuid05 \
                setreuid05_16 \
                setreuid06 \
                setreuid06_16 \
                setreuid07
            do echo "$n"; done
            ;;
        process:12)
            for n in \
                setreuid07_16 \
                setrlimit01 \
                setrlimit02 \
                setrlimit03 \
                setrlimit04 \
                setrlimit05 \
                setrlimit06 \
                setsid01 \
                settimeofday01 \
                settimeofday02 \
                setuid01 \
                setuid01_16 \
                setuid03 \
                setuid03_16 \
                setuid04 \
                setuid04_16 \
                sgetmask01 \
                sigaction01 \
                sigaction02 \
                sigaltstack01 \
                sigaltstack02 \
                sighold02 \
                signal01 \
                signal02 \
                signal03 \
                signal04 \
                signal05 \
                signal06 \
                signalfd01 \
                signalfd4_01
            do echo "$n"; done
            ;;
        process:13)
            for n in \
                signalfd4_02 \
                sigpending02 \
                sigprocmask01 \
                sigrelse01 \
                sigsuspend01 \
                sigtimedwait01 \
                sigwait01 \
                sigwaitinfo01 \
                ssetmask01 \
                stime01 \
                stime02 \
                tgkill01 \
                tgkill02 \
                tgkill03 \
                time-schedule \
                time01 \
                timer_delete01 \
                timer_delete02 \
                timer_getoverrun01 \
                timer_gettime01 \
                timer_settime01 \
                timer_settime02 \
                timer_settime03 \
                timerfd01 \
                timerfd02 \
                timerfd04 \
                timerfd_create01 \
                timerfd_gettime01 \
                timerfd_settime01 \
                timerfd_settime02
            do echo "$n"; done
            ;;
        process:14)
            for n in \
                times01 \
                times03 \
                tkill01 \
                tkill02 \
                ulimit01 \
                uname01 \
                uname02 \
                uname04 \
                utsname01 \
                utsname02 \
                utsname03 \
                utsname04 \
                vfork \
                vfork01 \
                vfork02 \
                wait01 \
                wait02 \
                wait401 \
                wait402 \
                wait403 \
                waitid01 \
                waitid02 \
                waitid03 \
                waitid04 \
                waitid05 \
                waitid06 \
                waitid07 \
                waitid08 \
                waitid09 \
                waitid10
            do echo "$n"; done
            ;;
        process:15)
            for n in \
                waitid11 \
                waitpid01 \
                waitpid03 \
                waitpid04 \
                waitpid06 \
                waitpid07 \
                waitpid08 \
                waitpid09 \
                waitpid10 \
                waitpid11 \
                waitpid12 \
                waitpid13
            do echo "$n"; done
            ;;
        fs:01)
            for n in \
                access01 \
                access02 \
                access03 \
                access04 \
                chdir01 \
                chdir04 \
                chmod01 \
                chmod03 \
                chmod05 \
                chmod06 \
                chmod07 \
                chown01 \
                chown01_16 \
                chown02 \
                chown02_16 \
                chown03 \
                chown03_16 \
                chown04 \
                chown04_16 \
                chown05 \
                chown05_16 \
                close01 \
                close02 \
                close_range01 \
                close_range02 \
                creat01 \
                creat03 \
                creat04 \
                creat05 \
                creat06
            do echo "$n"; done
            ;;
        fs:02)
            for n in \
                creat07 \
                creat08 \
                creat09 \
                dup01 \
                dup02 \
                dup03 \
                dup04 \
                dup05 \
                dup06 \
                dup07 \
                dup201 \
                dup202 \
                dup203 \
                dup204 \
                dup205 \
                dup206 \
                dup207 \
                dup3_01 \
                dup3_02 \
                faccessat01 \
                faccessat02 \
                faccessat201 \
                faccessat202 \
                fchdir01 \
                fchdir02 \
                fchdir03 \
                fchmod01 \
                fchmod02 \
                fchmod03 \
                fchmod04
            do echo "$n"; done
            ;;
        fs:03)
            for n in \
                fchmod05 \
                fchmod06 \
                fchmodat01 \
                fchmodat02 \
                fchown01 \
                fchown01_16 \
                fchown02 \
                fchown02_16 \
                fchown03 \
                fchown03_16 \
                fchown04 \
                fchown04_16 \
                fchown05 \
                fchown05_16 \
                fchownat01 \
                fchownat02 \
                fcntl01 \
                fcntl01_64 \
                fcntl02 \
                fcntl02_64 \
                fcntl03 \
                fcntl03_64 \
                fcntl04 \
                fcntl04_64 \
                fcntl05 \
                fcntl05_64 \
                fcntl07 \
                fcntl07_64 \
                fcntl08 \
                fcntl08_64
            do echo "$n"; done
            ;;
        fs:04)
            for n in \
                fcntl09 \
                fcntl09_64 \
                fcntl10 \
                fcntl10_64 \
                fcntl11 \
                fcntl11_64 \
                fcntl12 \
                fcntl12_64 \
                fcntl13 \
                fcntl13_64 \
                fcntl14 \
                fcntl14_64 \
                fcntl15 \
                fcntl15_64 \
                fcntl16 \
                fcntl16_64 \
                fcntl17 \
                fcntl17_64 \
                fcntl18 \
                fcntl18_64 \
                fcntl19 \
                fcntl19_64 \
                fcntl20 \
                fcntl20_64 \
                fcntl21 \
                fcntl21_64 \
                fcntl22 \
                fcntl22_64 \
                fcntl23 \
                fcntl23_64
            do echo "$n"; done
            ;;
        fs:05)
            for n in \
                fcntl24 \
                fcntl24_64 \
                fcntl25 \
                fcntl25_64 \
                fcntl26 \
                fcntl26_64 \
                fcntl27 \
                fcntl27_64 \
                fcntl29 \
                fcntl29_64 \
                fcntl30 \
                fcntl30_64 \
                fcntl31 \
                fcntl31_64 \
                fcntl32 \
                fcntl32_64 \
                fcntl33 \
                fcntl33_64 \
                fcntl34 \
                fcntl34_64 \
                fcntl35 \
                fcntl35_64 \
                fcntl36 \
                fcntl36_64 \
                fcntl37 \
                fcntl37_64 \
                fcntl38 \
                fcntl38_64 \
                fcntl39 \
                fcntl39_64
            do echo "$n"; done
            ;;
        fs:06)
            for n in \
                fdatasync01 \
                fdatasync02 \
                fdatasync03 \
                fgetxattr01 \
                fgetxattr02 \
                fgetxattr03 \
                flistxattr01 \
                flistxattr02 \
                flistxattr03 \
                fpathconf01 \
                fremovexattr01 \
                fremovexattr02 \
                fsetxattr01 \
                fsetxattr02 \
                fstat02 \
                fstat02_64 \
                fstat03 \
                fstat03_64 \
                fstatat01 \
                fstatfs01 \
                fstatfs01_64 \
                fstatfs02 \
                fstatfs02_64 \
                fsync01 \
                fsync02 \
                fsync03 \
                fsync04 \
                ftruncate01 \
                ftruncate01_64 \
                ftruncate03
            do echo "$n"; done
            ;;
        fs:07)
            for n in \
                ftruncate03_64 \
                ftruncate04 \
                ftruncate04_64 \
                getcwd01 \
                getcwd02 \
                getcwd03 \
                getcwd04 \
                getdents01 \
                getdents02 \
                getxattr01 \
                getxattr02 \
                getxattr03 \
                getxattr04 \
                getxattr05 \
                lchown01 \
                lchown01_16 \
                lchown02 \
                lchown02_16 \
                lchown03 \
                lchown03_16 \
                lgetxattr01 \
                lgetxattr02 \
                link02 \
                link04 \
                link05 \
                link08 \
                linkat01 \
                linkat02 \
                linktest.sh \
                listxattr01
            do echo "$n"; done
            ;;
        fs:08)
            for n in \
                listxattr02 \
                listxattr03 \
                llistxattr01 \
                llistxattr02 \
                llistxattr03 \
                llseek01 \
                llseek02 \
                llseek03 \
                lremovexattr01 \
                lseek01 \
                lseek02 \
                lseek07 \
                lseek11 \
                lstat01 \
                lstat01_64 \
                lstat02 \
                lstat02_64 \
                mkdir02 \
                mkdir03 \
                mkdir04 \
                mkdir05 \
                mkdir09 \
                mkdir_tests.sh \
                mkdirat01 \
                mkdirat02 \
                open01 \
                open02 \
                open03 \
                open04 \
                open06
            do echo "$n"; done
            ;;
        fs:09)
            for n in \
                open07 \
                open08 \
                open09 \
                open10 \
                open11 \
                open12 \
                open13 \
                open14 \
                openat01 \
                openat02 \
                openat03 \
                openat04 \
                openat201 \
                openat202 \
                openat203 \
                openfile \
                pathconf01 \
                pathconf02 \
                pread01 \
                pread01_64 \
                pread02 \
                pread02_64 \
                preadv01 \
                preadv01_64 \
                preadv02 \
                preadv02_64 \
                preadv03 \
                preadv03_64 \
                preadv201 \
                preadv201_64
            do echo "$n"; done
            ;;
        fs:10)
            for n in \
                preadv202 \
                preadv202_64 \
                preadv203 \
                preadv203_64 \
                pwrite01 \
                pwrite01_64 \
                pwrite02 \
                pwrite02_64 \
                pwrite03 \
                pwrite03_64 \
                pwrite04 \
                pwrite04_64 \
                pwritev01 \
                pwritev01_64 \
                pwritev02 \
                pwritev02_64 \
                pwritev03 \
                pwritev03_64 \
                pwritev201 \
                pwritev201_64 \
                pwritev202 \
                pwritev202_64 \
                read01 \
                read02 \
                read03 \
                read04 \
                readdir01 \
                readdir21 \
                readlink01 \
                readlink03
            do echo "$n"; done
            ;;
        fs:11)
            for n in \
                readlinkat01 \
                readlinkat02 \
                readv01 \
                readv02 \
                realpath01 \
                removexattr01 \
                removexattr02 \
                rename01 \
                rename03 \
                rename04 \
                rename05 \
                rename06 \
                rename07 \
                rename08 \
                rename09 \
                rename10 \
                rename11 \
                rename12 \
                rename13 \
                rename14 \
                renameat01 \
                renameat201 \
                renameat202 \
                rmdir01 \
                rmdir02 \
                rmdir03 \
                setxattr01 \
                setxattr02 \
                setxattr03 \
                stat01
            do echo "$n"; done
            ;;
        fs:12)
            for n in \
                stat01_64 \
                stat02 \
                stat02_64 \
                stat03 \
                stat03_64 \
                statfs01 \
                statfs01_64 \
                statfs02 \
                statfs02_64 \
                statfs03 \
                statfs03_64 \
                statvfs01 \
                statvfs02 \
                statx01 \
                statx02 \
                statx03 \
                statx04 \
                statx05 \
                statx06 \
                statx07 \
                statx08 \
                statx09 \
                statx10 \
                statx11 \
                statx12 \
                symlink01 \
                symlink02 \
                symlink03 \
                symlink04 \
                symlinkat01
            do echo "$n"; done
            ;;
        fs:13)
            for n in \
                sync01 \
                sync_file_range01 \
                sync_file_range02 \
                syncfs01 \
                truncate02 \
                truncate02_64 \
                truncate03 \
                truncate03_64 \
                unlink05 \
                unlink07 \
                unlink08 \
                unlink09 \
                unlinkat01 \
                write01 \
                write02 \
                write03 \
                write04 \
                write05 \
                write06 \
                writetest \
                writev01 \
                writev02 \
                writev03 \
                writev05 \
                writev06 \
                writev07
            do echo "$n"; done
            ;;
        mm-ipc:01)
            for n in \
                accept01 \
                accept02 \
                accept03 \
                accept4_01 \
                bind01 \
                bind02 \
                bind03 \
                bind04 \
                bind05 \
                bind06 \
                brk01 \
                brk02 \
                connect01 \
                connect02 \
                epoll-ltp \
                epoll_create01 \
                epoll_create02 \
                epoll_create1_01 \
                epoll_create1_02 \
                epoll_ctl01 \
                epoll_ctl02 \
                epoll_ctl03 \
                epoll_ctl04 \
                epoll_ctl05 \
                epoll_pwait01 \
                epoll_pwait02 \
                epoll_pwait03 \
                epoll_pwait04 \
                epoll_pwait05 \
                epoll_wait01
            do echo "$n"; done
            ;;
        mm-ipc:02)
            for n in \
                epoll_wait02 \
                epoll_wait03 \
                epoll_wait04 \
                epoll_wait05 \
                epoll_wait06 \
                epoll_wait07 \
                eventfd01 \
                eventfd02 \
                eventfd03 \
                eventfd04 \
                eventfd05 \
                eventfd06 \
                eventfd2_01 \
                eventfd2_02 \
                eventfd2_03 \
                futex_cmp_requeue01 \
                futex_cmp_requeue02 \
                futex_wait01 \
                futex_wait02 \
                futex_wait03 \
                futex_wait04 \
                futex_wait05 \
                futex_wait_bitset01 \
                futex_waitv01 \
                futex_waitv02 \
                futex_waitv03 \
                futex_wake01 \
                futex_wake02 \
                futex_wake03 \
                futex_wake04
            do echo "$n"; done
            ;;
        mm-ipc:03)
            for n in \
                getpeername01 \
                getsockname01 \
                getsockopt01 \
                getsockopt02 \
                listen01 \
                madvise01 \
                madvise02 \
                madvise03 \
                madvise05 \
                madvise06 \
                madvise07 \
                madvise08 \
                madvise09 \
                madvise10 \
                madvise11 \
                mincore01 \
                mincore02 \
                mincore03 \
                mincore04 \
                mmap-corruption01 \
                mmap001 \
                mmap01 \
                mmap02 \
                mmap03 \
                mmap04 \
                mmap05 \
                mmap06 \
                mmap08 \
                mmap09 \
                mmap1
            do echo "$n"; done
            ;;
        mm-ipc:04)
            for n in \
                mmap10 \
                mmap11 \
                mmap12 \
                mmap13 \
                mmap14 \
                mmap15 \
                mmap16 \
                mmap17 \
                mmap18 \
                mmap19 \
                mmap2 \
                mmap20 \
                mmap3 \
                mprotect01 \
                mprotect02 \
                mprotect03 \
                mprotect04 \
                mprotect05 \
                mq_notify01 \
                mq_notify02 \
                mq_notify03 \
                mq_open01 \
                mq_timedreceive01 \
                mq_timedsend01 \
                mq_unlink01 \
                mremap01 \
                mremap02 \
                mremap03 \
                mremap04 \
                mremap05
            do echo "$n"; done
            ;;
        mm-ipc:05)
            for n in \
                mremap06 \
                msg_comm \
                msgctl01 \
                msgctl02 \
                msgctl03 \
                msgctl04 \
                msgctl05 \
                msgctl06 \
                msgctl12 \
                msgget01 \
                msgget02 \
                msgget03 \
                msgget04 \
                msgget05 \
                msgrcv01 \
                msgrcv02 \
                msgrcv03 \
                msgrcv05 \
                msgrcv06 \
                msgrcv07 \
                msgrcv08 \
                msgsnd01 \
                msgsnd02 \
                msgsnd05 \
                msgsnd06 \
                msync01 \
                msync02 \
                msync03 \
                msync04 \
                munmap01
            do echo "$n"; done
            ;;
        mm-ipc:06)
            for n in \
                munmap02 \
                munmap03 \
                pipe01 \
                pipe02 \
                pipe03 \
                pipe04 \
                pipe05 \
                pipe06 \
                pipe07 \
                pipe08 \
                pipe09 \
                pipe10 \
                pipe11 \
                pipe12 \
                pipe13 \
                pipe14 \
                pipe15 \
                pipe2_01 \
                pipe2_02 \
                pipe2_04 \
                pipeio \
                poll01 \
                poll02 \
                ppoll01 \
                pselect01 \
                pselect01_64 \
                pselect02 \
                pselect02_64 \
                pselect03 \
                pselect03_64
            do echo "$n"; done
            ;;
        mm-ipc:07)
            for n in \
                recv01 \
                recvfrom01 \
                recvmmsg01 \
                recvmsg01 \
                recvmsg02 \
                recvmsg03 \
                select01 \
                select02 \
                select03 \
                select04 \
                sem_comm \
                sem_nstest \
                semctl01 \
                semctl02 \
                semctl03 \
                semctl04 \
                semctl05 \
                semctl06 \
                semctl07 \
                semctl08 \
                semctl09 \
                semget01 \
                semget02 \
                semget05 \
                semop01 \
                semop02 \
                semop03 \
                semop04 \
                semop05 \
                semtest_2ns
            do echo "$n"; done
            ;;
        mm-ipc:08)
            for n in \
                send01 \
                send02 \
                sendmmsg01 \
                sendmmsg02 \
                sendmsg01 \
                sendmsg02 \
                sendmsg03 \
                sendto01 \
                sendto02 \
                sendto03 \
                setsockopt01 \
                setsockopt02 \
                setsockopt03 \
                setsockopt04 \
                setsockopt05 \
                setsockopt06 \
                setsockopt07 \
                setsockopt08 \
                setsockopt09 \
                setsockopt10 \
                shm_comm \
                shm_test \
                shmat01 \
                shmat02 \
                shmat03 \
                shmat04 \
                shmat1 \
                shmctl01 \
                shmctl02 \
                shmctl03
            do echo "$n"; done
            ;;
        mm-ipc:09)
            for n in \
                shmctl04 \
                shmctl05 \
                shmctl06 \
                shmctl07 \
                shmctl08 \
                shmdt01 \
                shmdt02 \
                shmem_2nstest \
                shmget02 \
                shmget03 \
                shmget04 \
                shmget05 \
                shmget06 \
                shmnstest \
                shmt02 \
                shmt03 \
                shmt04 \
                shmt05 \
                shmt06 \
                shmt07 \
                shmt08 \
                shmt09 \
                shmt10 \
                socket01 \
                socket02 \
                socketcall01 \
                socketcall02 \
                socketcall03 \
                socketpair01 \
                socketpair02
            do echo "$n"; done
            ;;
        common-easy:01)
            for n in \
                abs01 \
                atof01 \
                confstr01 \
                float_bessel \
                float_exp_log \
                float_iperb \
                float_power \
                float_trigo \
                fptest01 \
                fptest02 \
                memcmp01 \
                memcpy01 \
                memset01 \
                nextafter01 \
                string01
            do echo "$n"; done
            ;;
        net:01|net-core:01)
            for n in \
                accept01 \
                accept02 \
                accept03 \
                accept4_01 \
                bind01 \
                bind02 \
                bind03 \
                bind04 \
                bind05 \
                bind06 \
                connect01 \
                connect02 \
                getaddrinfo_01 \
                gethostbyname_r01 \
                getsockname01 \
                getsockopt01 \
                getsockopt02 \
                listen01
            do echo "$n"; done
            ;;
        net:02|net-core:02)
            for n in \
                recv01 \
                recvfrom01 \
                recvmmsg01 \
                recvmsg01 \
                recvmsg02 \
                recvmsg03 \
                send01 \
                send02 \
                sendmmsg01 \
                sendmmsg02 \
                sendmsg01 \
                sendmsg02 \
                sendmsg03 \
                sendto01 \
                sendto02 \
                sendto03 \
                setsockopt01 \
                setsockopt02 \
                setsockopt03 \
                setsockopt04 \
                setsockopt05 \
                setsockopt06 \
                setsockopt07 \
                setsockopt08 \
                setsockopt09 \
                setsockopt10 \
                socket01 \
                socket02
            do echo "$n"; done
            ;;
        net:03|net-core:03)
            for n in \
                socketcall01 \
                socketcall02 \
                socketcall03 \
                socketpair01 \
                socketpair02 \
                sockioctl01
            do echo "$n"; done
            ;;
        net-script:01)
            for n in \
                ltpSockets.sh \
                arping01.sh \
                bbr01.sh \
                bbr02.sh \
                bind_noport01.sh \
                broken_ip-checksum.sh \
                broken_ip-dstaddr.sh \
                broken_ip-fragment.sh \
                broken_ip-ihl.sh \
                broken_ip-nexthdr.sh \
                broken_ip-plen.sh \
                broken_ip-protcol.sh \
                broken_ip-version.sh \
                busy_poll01.sh \
                busy_poll02.sh \
                busy_poll03.sh \
                icmp-uni-basic.sh \
                ping01.sh \
                ping02.sh \
                tcp4-uni-basic01 \
                tcp4-uni-basic02 \
                tcp4-uni-basic03 \
                tcp4-uni-basic04 \
                tcp4-uni-basic05 \
                tcp4-uni-basic06 \
                tcp4-uni-basic07 \
                tcp4-uni-basic08 \
                tcp4-uni-basic09 \
                tcp4-uni-basic10 \
                tcp4-uni-basic11
            do echo "$n"; done
            ;;
        net-script:02)
            for n in \
                tcp4-uni-basic12 \
                tcp4-uni-basic13 \
                tcp4-uni-basic14 \
                udp4-uni-basic01 \
                udp4-uni-basic02 \
                udp4-uni-basic03 \
                udp4-uni-basic04 \
                udp4-uni-basic05 \
                udp4-uni-basic06 \
                udp4-uni-basic07 \
                icmp4-multi-diffip01 \
                icmp4-multi-diffip02 \
                icmp4-multi-diffip03 \
                icmp4-multi-diffip04 \
                icmp4-multi-diffip05 \
                icmp4-multi-diffip06 \
                icmp4-multi-diffip07 \
                icmp4-multi-diffnic01 \
                icmp4-multi-diffnic02 \
                icmp4-multi-diffnic03 \
                icmp4-multi-diffnic04 \
                icmp4-multi-diffnic05 \
                icmp4-multi-diffnic06 \
                icmp4-multi-diffnic07 \
                tcp4-multi-diffip01 \
                tcp4-multi-diffip02 \
                tcp4-multi-diffip03 \
                tcp4-multi-diffip04 \
                tcp4-multi-diffip05 \
                tcp4-multi-diffip06
            do echo "$n"; done
            ;;
        net-script:03)
            for n in \
                tcp4-multi-diffip07 \
                tcp4-multi-diffip08 \
                tcp4-multi-diffip09 \
                tcp4-multi-diffip10 \
                tcp4-multi-diffip11 \
                tcp4-multi-diffip12 \
                tcp4-multi-diffip13 \
                tcp4-multi-diffip14 \
                tcp4-multi-diffnic01 \
                tcp4-multi-diffnic02 \
                tcp4-multi-diffnic03 \
                tcp4-multi-diffnic04 \
                tcp4-multi-diffnic05 \
                tcp4-multi-diffnic06 \
                tcp4-multi-diffnic07 \
                tcp4-multi-diffnic08 \
                tcp4-multi-diffnic09 \
                tcp4-multi-diffnic10 \
                tcp4-multi-diffnic11 \
                tcp4-multi-diffnic12 \
                tcp4-multi-diffnic13 \
                tcp4-multi-diffnic14 \
                tcp4-multi-diffport01 \
                tcp4-multi-diffport02 \
                tcp4-multi-diffport03 \
                tcp4-multi-diffport04 \
                tcp4-multi-diffport05 \
                tcp4-multi-diffport06 \
                tcp4-multi-diffport07 \
                tcp4-multi-diffport08
            do echo "$n"; done
            ;;
        net-script:04)
            for n in \
                tcp4-multi-diffport09 \
                tcp4-multi-diffport10 \
                tcp4-multi-diffport11 \
                tcp4-multi-diffport12 \
                tcp4-multi-diffport13 \
                tcp4-multi-diffport14 \
                tcp4-multi-sameport01 \
                tcp4-multi-sameport02 \
                tcp4-multi-sameport03 \
                tcp4-multi-sameport04 \
                tcp4-multi-sameport05 \
                tcp4-multi-sameport06 \
                tcp4-multi-sameport07 \
                tcp4-multi-sameport08 \
                tcp4-multi-sameport09 \
                tcp4-multi-sameport10 \
                tcp4-multi-sameport11 \
                tcp4-multi-sameport12 \
                tcp4-multi-sameport13 \
                tcp4-multi-sameport14 \
                udp4-multi-diffip01 \
                udp4-multi-diffip02 \
                udp4-multi-diffip03 \
                udp4-multi-diffip04 \
                udp4-multi-diffip05 \
                udp4-multi-diffip06 \
                udp4-multi-diffip07 \
                udp4-multi-diffnic01 \
                udp4-multi-diffnic02 \
                udp4-multi-diffnic03
            do echo "$n"; done
            ;;
        net-script:05)
            for n in \
                udp4-multi-diffnic04 \
                udp4-multi-diffnic05 \
                udp4-multi-diffnic06 \
                udp4-multi-diffnic07 \
                tcp4-uni-dsackoff01 \
                tcp4-uni-dsackoff02 \
                tcp4-uni-dsackoff03 \
                tcp4-uni-dsackoff04 \
                tcp4-uni-dsackoff05 \
                tcp4-uni-dsackoff06 \
                tcp4-uni-dsackoff07 \
                tcp4-uni-dsackoff08 \
                tcp4-uni-dsackoff09 \
                tcp4-uni-dsackoff10 \
                tcp4-uni-dsackoff11 \
                tcp4-uni-dsackoff12 \
                tcp4-uni-dsackoff13 \
                tcp4-uni-dsackoff14 \
                tcp4-uni-pktlossdup01 \
                tcp4-uni-pktlossdup02 \
                tcp4-uni-pktlossdup03 \
                tcp4-uni-pktlossdup04 \
                tcp4-uni-pktlossdup05 \
                tcp4-uni-pktlossdup06 \
                tcp4-uni-pktlossdup07 \
                tcp4-uni-pktlossdup08 \
                tcp4-uni-pktlossdup09 \
                tcp4-uni-pktlossdup10 \
                tcp4-uni-pktlossdup11 \
                tcp4-uni-pktlossdup12
            do echo "$n"; done
            ;;
        net-script:06)
            for n in \
                tcp4-uni-pktlossdup13 \
                tcp4-uni-pktlossdup14 \
                tcp4-uni-sackoff01 \
                tcp4-uni-sackoff02 \
                tcp4-uni-sackoff03 \
                tcp4-uni-sackoff04 \
                tcp4-uni-sackoff05 \
                tcp4-uni-sackoff06 \
                tcp4-uni-sackoff07 \
                tcp4-uni-sackoff08 \
                tcp4-uni-sackoff09 \
                tcp4-uni-sackoff10 \
                tcp4-uni-sackoff11 \
                tcp4-uni-sackoff12 \
                tcp4-uni-sackoff13 \
                tcp4-uni-sackoff14 \
                tcp4-uni-smallsend01 \
                tcp4-uni-smallsend02 \
                tcp4-uni-smallsend03 \
                tcp4-uni-smallsend04 \
                tcp4-uni-smallsend05 \
                tcp4-uni-smallsend06 \
                tcp4-uni-smallsend07 \
                tcp4-uni-smallsend08 \
                tcp4-uni-smallsend09 \
                tcp4-uni-smallsend10 \
                tcp4-uni-smallsend11 \
                tcp4-uni-smallsend12 \
                tcp4-uni-smallsend13 \
                tcp4-uni-smallsend14
            do echo "$n"; done
            ;;
        net-script:07)
            for n in \
                tcp4-uni-tso01 \
                tcp4-uni-tso02 \
                tcp4-uni-tso03 \
                tcp4-uni-tso04 \
                tcp4-uni-tso05 \
                tcp4-uni-tso06 \
                tcp4-uni-tso07 \
                tcp4-uni-tso08 \
                tcp4-uni-tso09 \
                tcp4-uni-tso10 \
                tcp4-uni-tso11 \
                tcp4-uni-tso12 \
                tcp4-uni-tso13 \
                tcp4-uni-tso14 \
                tcp4-uni-winscale01 \
                tcp4-uni-winscale02 \
                tcp4-uni-winscale03 \
                tcp4-uni-winscale04 \
                tcp4-uni-winscale05 \
                tcp4-uni-winscale06 \
                tcp4-uni-winscale07 \
                tcp4-uni-winscale08 \
                tcp4-uni-winscale09 \
                tcp4-uni-winscale10 \
                tcp4-uni-winscale11 \
                tcp4-uni-winscale12 \
                tcp4-uni-winscale13 \
                tcp4-uni-winscale14 \
                udp4-multi-diffport01 \
                udp4-multi-diffport02
            do echo "$n"; done
            ;;
        net-script:08)
            for n in \
                udp4-multi-diffport03 \
                udp4-multi-diffport04 \
                udp4-multi-diffport05 \
                udp4-multi-diffport06 \
                udp4-multi-diffport07 \
                icmp6-multi-diffip01 \
                icmp6-multi-diffip02 \
                icmp6-multi-diffip03 \
                icmp6-multi-diffip04 \
                icmp6-multi-diffip05 \
                icmp6-multi-diffip06 \
                icmp6-multi-diffip07 \
                icmp6-multi-diffnic01 \
                icmp6-multi-diffnic02 \
                icmp6-multi-diffnic03 \
                icmp6-multi-diffnic04 \
                icmp6-multi-diffnic05 \
                icmp6-multi-diffnic06 \
                icmp6-multi-diffnic07 \
                tcp6-multi-diffip01 \
                tcp6-multi-diffip02 \
                tcp6-multi-diffip03 \
                tcp6-multi-diffip04 \
                tcp6-multi-diffip05 \
                tcp6-multi-diffip06 \
                tcp6-multi-diffip07 \
                tcp6-multi-diffip08 \
                tcp6-multi-diffip09 \
                tcp6-multi-diffip10 \
                tcp6-multi-diffip11
            do echo "$n"; done
            ;;
        net-script:09)
            for n in \
                tcp6-multi-diffip12 \
                tcp6-multi-diffip13 \
                tcp6-multi-diffip14 \
                tcp6-multi-diffnic01 \
                tcp6-multi-diffnic02 \
                tcp6-multi-diffnic03 \
                tcp6-multi-diffnic04 \
                tcp6-multi-diffnic05 \
                tcp6-multi-diffnic06 \
                tcp6-multi-diffnic07 \
                tcp6-multi-diffnic08 \
                tcp6-multi-diffnic09 \
                tcp6-multi-diffnic10 \
                tcp6-multi-diffnic11 \
                tcp6-multi-diffnic12 \
                tcp6-multi-diffnic13 \
                tcp6-multi-diffnic14 \
                tcp6-multi-diffport01 \
                tcp6-multi-diffport02 \
                tcp6-multi-diffport03 \
                tcp6-multi-diffport04 \
                tcp6-multi-diffport05 \
                tcp6-multi-diffport06 \
                tcp6-multi-diffport07 \
                tcp6-multi-diffport08 \
                tcp6-multi-diffport09 \
                tcp6-multi-diffport10 \
                tcp6-multi-diffport11 \
                tcp6-multi-diffport12 \
                tcp6-multi-diffport13
            do echo "$n"; done
            ;;
        net-script:10)
            for n in \
                tcp6-multi-diffport14 \
                tcp6-multi-sameport01 \
                tcp6-multi-sameport02 \
                tcp6-multi-sameport03 \
                tcp6-multi-sameport04 \
                tcp6-multi-sameport05 \
                tcp6-multi-sameport06 \
                tcp6-multi-sameport07 \
                tcp6-multi-sameport08 \
                tcp6-multi-sameport09 \
                tcp6-multi-sameport10 \
                tcp6-multi-sameport11 \
                tcp6-multi-sameport12 \
                tcp6-multi-sameport13 \
                tcp6-multi-sameport14 \
                tcp6-uni-basic01 \
                tcp6-uni-basic02 \
                tcp6-uni-basic03 \
                tcp6-uni-basic04 \
                tcp6-uni-basic05 \
                tcp6-uni-basic06 \
                tcp6-uni-basic07 \
                tcp6-uni-basic08 \
                tcp6-uni-basic09 \
                tcp6-uni-basic10 \
                tcp6-uni-basic11 \
                tcp6-uni-basic12 \
                tcp6-uni-basic13 \
                tcp6-uni-basic14 \
                tcp6-uni-dsackoff01
            do echo "$n"; done
            ;;
        net-script:11)
            for n in \
                tcp6-uni-dsackoff02 \
                tcp6-uni-dsackoff03 \
                tcp6-uni-dsackoff04 \
                tcp6-uni-dsackoff05 \
                tcp6-uni-dsackoff06 \
                tcp6-uni-dsackoff07 \
                tcp6-uni-dsackoff08 \
                tcp6-uni-dsackoff09 \
                tcp6-uni-dsackoff10 \
                tcp6-uni-dsackoff11 \
                tcp6-uni-dsackoff12 \
                tcp6-uni-dsackoff13 \
                tcp6-uni-dsackoff14 \
                tcp6-uni-pktlossdup01 \
                tcp6-uni-pktlossdup02 \
                tcp6-uni-pktlossdup03 \
                tcp6-uni-pktlossdup04 \
                tcp6-uni-pktlossdup05 \
                tcp6-uni-pktlossdup06 \
                tcp6-uni-pktlossdup07 \
                tcp6-uni-pktlossdup08 \
                tcp6-uni-pktlossdup09 \
                tcp6-uni-pktlossdup10 \
                tcp6-uni-pktlossdup11 \
                tcp6-uni-pktlossdup12 \
                tcp6-uni-pktlossdup13 \
                tcp6-uni-pktlossdup14 \
                tcp6-uni-sackoff01 \
                tcp6-uni-sackoff02 \
                tcp6-uni-sackoff03
            do echo "$n"; done
            ;;
        net-script:12)
            for n in \
                tcp6-uni-sackoff04 \
                tcp6-uni-sackoff05 \
                tcp6-uni-sackoff06 \
                tcp6-uni-sackoff07 \
                tcp6-uni-sackoff08 \
                tcp6-uni-sackoff09 \
                tcp6-uni-sackoff10 \
                tcp6-uni-sackoff11 \
                tcp6-uni-sackoff12 \
                tcp6-uni-sackoff13 \
                tcp6-uni-sackoff14 \
                tcp6-uni-smallsend01 \
                tcp6-uni-smallsend02 \
                tcp6-uni-smallsend03 \
                tcp6-uni-smallsend04 \
                tcp6-uni-smallsend05 \
                tcp6-uni-smallsend06 \
                tcp6-uni-smallsend07 \
                tcp6-uni-smallsend08 \
                tcp6-uni-smallsend09 \
                tcp6-uni-smallsend10 \
                tcp6-uni-smallsend11 \
                tcp6-uni-smallsend12 \
                tcp6-uni-smallsend13 \
                tcp6-uni-smallsend14 \
                tcp6-uni-tso01 \
                tcp6-uni-tso02 \
                tcp6-uni-tso03 \
                tcp6-uni-tso04 \
                tcp6-uni-tso05
            do echo "$n"; done
            ;;
        net-script:13)
            for n in \
                tcp6-uni-tso06 \
                tcp6-uni-tso07 \
                tcp6-uni-tso08 \
                tcp6-uni-tso09 \
                tcp6-uni-tso10 \
                tcp6-uni-tso11 \
                tcp6-uni-tso12 \
                tcp6-uni-tso13 \
                tcp6-uni-tso14 \
                tcp6-uni-winscale01 \
                tcp6-uni-winscale02 \
                tcp6-uni-winscale03 \
                tcp6-uni-winscale04 \
                tcp6-uni-winscale05 \
                tcp6-uni-winscale06 \
                tcp6-uni-winscale07 \
                tcp6-uni-winscale08 \
                tcp6-uni-winscale09 \
                tcp6-uni-winscale10 \
                tcp6-uni-winscale11 \
                tcp6-uni-winscale12 \
                tcp6-uni-winscale13 \
                tcp6-uni-winscale14 \
                udp6-multi-diffip01 \
                udp6-multi-diffip02 \
                udp6-multi-diffip03 \
                udp6-multi-diffip04 \
                udp6-multi-diffip05 \
                udp6-multi-diffip06 \
                udp6-multi-diffip07
            do echo "$n"; done
            ;;
        net-script:14)
            for n in \
                udp6-multi-diffnic01 \
                udp6-multi-diffnic02 \
                udp6-multi-diffnic03 \
                udp6-multi-diffnic04 \
                udp6-multi-diffnic05 \
                udp6-multi-diffnic06 \
                udp6-multi-diffnic07 \
                udp6-multi-diffport01 \
                udp6-multi-diffport02 \
                udp6-multi-diffport03 \
                udp6-multi-diffport04 \
                udp6-multi-diffport05 \
                udp6-multi-diffport06 \
                udp6-multi-diffport07 \
                udp6-uni-basic01 \
                udp6-uni-basic02 \
                udp6-uni-basic03 \
                udp6-uni-basic04 \
                udp6-uni-basic05 \
                udp6-uni-basic06 \
                udp6-uni-basic07 \
                mcast-group-multiple-socket.sh \
                mcast-group-same-group.sh \
                mcast-group-single-socket.sh \
                mcast-group-source-filter.sh \
                mcast-pktfld01.sh \
                mcast-pktfld02.sh \
                mcast-queryfld01.sh \
                mcast-queryfld02.sh \
                mcast-queryfld03.sh
            do echo "$n"; done
            ;;
        net-script:15)
            for n in \
                mcast-queryfld04.sh \
                mcast-queryfld05.sh \
                mcast-queryfld06.sh \
                if-route-adddel.sh \
                if-route-addlarge.sh \
                netstat01.sh \
                route-change-dst.sh \
                route-change-gw.sh \
                route-change-if.sh \
                route-change-netlink-dst.sh \
                route-change-netlink-gw.sh \
                route-change-netlink-if.sh \
                route-redirect.sh \
                route4-rmmod \
                route6-rmmod \
                tcpdump01.sh \
                traceroute01.sh \
                tcp_fastopen_run.sh
            do echo "$n"; done
            ;;
        net-deferred:01)
            for n in \
                icmp_rate_limit01 \
                dhcpd_tests.sh \
                ftp-download-stress.sh \
                ftp-upload-stress.sh \
                ftp01.sh \
                nfs01.sh \
                nfs02.sh \
                nfs03.sh \
                nfs04.sh \
                nfs05.sh \
                nfs06.sh \
                nfs07.sh \
                nfs08.sh \
                nfs09.sh \
                nfs_flock \
                nfslock01.sh \
                nfsstat01.sh \
                ssh-stress.sh \
                route-change-netlink \
                geneve01.sh \
                geneve02.sh \
                gre01.sh \
                gre02.sh \
                ipvlan01.sh \
                macsec01.sh \
                macsec02.sh \
                macsec03.sh \
                macvlan01.sh \
                macvtap01.sh \
                vlan01.sh
            do echo "$n"; done
            ;;
        net-deferred:02)
            for n in \
                vlan02.sh \
                vlan03.sh \
                vxlan01.sh \
                vxlan02.sh \
                vxlan03.sh \
                vxlan04.sh \
                netns_breakns.sh \
                netns_comm.sh \
                netns_netlink \
                netns_sysfs.sh \
                can_bcm01 \
                can_filter \
                can_rcv_own_msgs \
                dccp01.sh \
                dccp_ipsec.sh \
                dccp_ipsec_vti.sh \
                icmp-uni-vti.sh \
                sctp01.sh \
                sctp_big_chunk \
                sctp_ipsec.sh \
                sctp_ipsec_vti.sh \
                tcp_ipsec.sh \
                tcp_ipsec_vti.sh \
                udp_ipsec.sh \
                udp_ipsec_vti.sh \
                vsock01 \
                test_1_to_1_accept_close \
                test_1_to_1_addrs \
                test_1_to_1_connect \
                test_1_to_1_connectx
            do echo "$n"; done
            ;;
        net-deferred:03)
            for n in \
                test_1_to_1_events \
                test_1_to_1_initmsg_connect \
                test_1_to_1_nonblock \
                test_1_to_1_recvfrom \
                test_1_to_1_recvmsg \
                test_1_to_1_rtoinfo \
                test_1_to_1_send \
                test_1_to_1_sendmsg \
                test_1_to_1_sendto \
                test_1_to_1_shutdown \
                test_1_to_1_socket_bind_listen \
                test_1_to_1_sockopt \
                test_1_to_1_threads \
                test_assoc_abort \
                test_assoc_shutdown \
                test_autoclose \
                test_basic \
                test_basic_v6 \
                test_connect \
                test_connectx \
                test_fragments \
                test_fragments_v6 \
                test_getname \
                test_getname_v6 \
                test_inaddr_any \
                test_inaddr_any_v6 \
                test_ioctl \
                test_peeloff \
                test_peeloff_v6 \
                test_recvmsg
            do echo "$n"; done
            ;;
        net-deferred:04)
            for n in \
                test_robind.sh \
                test_sctp_sendrecvmsg \
                test_sctp_sendrecvmsg_v6 \
                test_sockopt \
                test_sockopt_v6 \
                test_tcp_style \
                test_tcp_style_v6 \
                test_timetolive \
                test_timetolive_v6
            do echo "$n"; done
            ;;
        net-all:01) ltp_batch_cases net 01 ;;
        net-all:02) ltp_batch_cases net 02 ;;
        net-all:03) ltp_batch_cases net 03 ;;
        net-all:04) ltp_batch_cases net-script 01 ;;
        net-all:05) ltp_batch_cases net-script 02 ;;
        net-all:06) ltp_batch_cases net-script 03 ;;
        net-all:07) ltp_batch_cases net-script 04 ;;
        net-all:08) ltp_batch_cases net-script 05 ;;
        net-all:09) ltp_batch_cases net-script 06 ;;
        net-all:10) ltp_batch_cases net-script 07 ;;
        net-all:11) ltp_batch_cases net-script 08 ;;
        net-all:12) ltp_batch_cases net-script 09 ;;
        net-all:13) ltp_batch_cases net-script 10 ;;
        net-all:14) ltp_batch_cases net-script 11 ;;
        net-all:15) ltp_batch_cases net-script 12 ;;
        net-all:16) ltp_batch_cases net-script 13 ;;
        net-all:17) ltp_batch_cases net-script 14 ;;
        net-all:18) ltp_batch_cases net-script 15 ;;
        net-all:19) ltp_batch_cases net-deferred 01 ;;
        net-all:20) ltp_batch_cases net-deferred 02 ;;
        net-all:21) ltp_batch_cases net-deferred 03 ;;
        net-all:22) ltp_batch_cases net-deferred 04 ;;
        *)
            return 1
            ;;
    esac
}
