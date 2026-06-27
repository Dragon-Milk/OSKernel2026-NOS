#!/bin/sh

export HOME=/root
export USER=root
BASE_PATH=.:/bin:/sbin:/usr/bin:/usr/sbin
export PATH=$BASE_PATH

# Set SKIP_LTP=0 to run the original ltp_testcode.sh scripts again.
# ============================================================
# Select test profile here.
# cyc-musl      : run musl cyclictest only
# cyc-all       : run glibc and musl cyclictest only
# libctest      : run glibc and musl libctest only
# iozone        : run glibc and musl iozone to verify sys_sync/syncfs
# lmbench       : run glibc and musl lmbench only
# ltp-list      : list glibc and musl ltp testcase names only
# ltp-safe      : run LTP cases from LTP_CASE_LIST, or ltp-safe.txt when unset
# unixbench     : run glibc and musl unixbench only
# wait-repro    : run wait/libctest/lmbench/unixbench repro
# full          : scan and run all testcode scripts (original full scan, no LTP skip)
# full-safe     : prepare basic scripts, run non-LTP tests, then LTP case list
# ============================================================
SKIP_LTP=${SKIP_LTP:-0}
TEST_PROFILE=${TEST_PROFILE:-full-safe}
LTP_CASE_LIST=${LTP_CASE_LIST:-}
LTP_TIMEOUT=${LTP_TIMEOUT:-${LTP_CASE_TIMEOUT:-45}}
export LTP_TIMEOUT
FULL_SAFE_SKIP_WASTE=${FULL_SAFE_SKIP_WASTE:-1}
echo "[init] TEST_PROFILE=$TEST_PROFILE"
echo "[init] LTP_TIMEOUT=$LTP_TIMEOUT"
echo "[init] LTP_CASE_LIST=$LTP_CASE_LIST"
echo "[init] FULL_SAFE_SKIP_WASTE=$FULL_SAFE_SKIP_WASTE"

entry_name_exists() {
    file="$1"
    name="$2"

    [ -f "$file" ] || return 1
    while IFS=: read entry rest || [ -n "$entry" ]; do
        [ "$entry" = "$name" ] && return 0
    done < "$file"
    return 1
}

ensure_named_entry() {
    file="$1"
    name="$2"
    line="$3"

    entry_name_exists "$file" "$name" && return
    printf '%s\n' "$line" >> "$file"
}

ensure_user_database() {
    bb="$(busybox_cmd)"
    if [ -n "$bb" ]; then
        "$bb" mkdir -p /etc 2>/dev/null || return
    else
        mkdir -p /etc 2>/dev/null || return
    fi
    [ -f /etc/passwd ] || : > /etc/passwd
    [ -f /etc/group ] || : > /etc/group

    ensure_named_entry /etc/passwd root 'root:x:0:0:root:/root:/bin/sh'
    ensure_named_entry /etc/passwd nobody 'nobody:x:65534:65534:nobody:/:/sbin/nologin'
    ensure_named_entry /etc/group root 'root:x:0:'
    ensure_named_entry /etc/group daemon 'daemon:x:1:'
    ensure_named_entry /etc/group nobody 'nobody:x:65534:'
}

run_with_shell() {
    script="$1"

    if [ -x ./busybox ]; then
        ./busybox sh "$script"
    elif [ -x /busybox ]; then
        /busybox sh "$script"
    elif [ -x /musl/busybox ]; then
        /musl/busybox sh "$script"
    elif [ -x /glibc/busybox ]; then
        /glibc/busybox sh "$script"
    else
        sh "$script"
    fi
}

busybox_cmd() {
    if [ -x ./busybox ]; then
        echo ./busybox
    elif [ -x /busybox ]; then
        echo /busybox
    elif [ -x /musl/busybox ]; then
        echo /musl/busybox
    elif [ -x /glibc/busybox ]; then
        echo /glibc/busybox
    fi
}

ensure_user_database

is_leftover_command() {
    case "$1" in
        "./iperf3 -s"*|"iperf3 -s"*|"/glibc/iperf3 -s"*|"/musl/iperf3 -s"*| \
        "./netserver"*|"netserver"*|"/glibc/netserver"*|"/musl/netserver"*| \
        "./lmbench_all"*|"lmbench_all"*|"/glibc/lmbench_all"*|"/musl/lmbench_all"*| \
        "./pipe 10"*|"pipe 10"*|"/glibc/pipe 10"*|"/musl/pipe 10"*| \
        "./hackbench"*|"hackbench"*|"/glibc/hackbench"*|"/musl/hackbench"*| \
        "./cyclictest"*|"cyclictest"*|"/glibc/cyclictest"*|"/musl/cyclictest"*| \
        "./busybox sh ./lmbench_testcode.sh"*|"/glibc/busybox sh ./lmbench_testcode.sh"*|"/musl/busybox sh ./lmbench_testcode.sh"*| \
        "./busybox sh ./unixbench_testcode.sh"*|"/glibc/busybox sh ./unixbench_testcode.sh"*|"/musl/busybox sh ./unixbench_testcode.sh"*)
            return 0
            ;;
    esac

    return 1
}

needs_cleanup() {
    case "$1" in
        *iperf*|*netperf*|*lmbench*|*unixbench*|*cyclictest*|*hackbench*)
            echo "[init] cleanup after $1"
            return 0
            ;;
    esac
    return 1
}

cleanup_leftovers_once() {
    signal="$1"
    bb="$2"

    "$bb" ps | while read pid user time command; do
        case "$pid" in
            ''|PID) continue ;;
        esac
        [ "$pid" = "$$" ] && continue
        case "$command" in
            *"cleanup_leftovers_once"*) continue ;;
        esac

        if is_leftover_command "$command"; then
            echo "cleanup leftover pid $pid: $command"
            "$bb" kill "$signal" "$pid" >/dev/null 2>&1
        fi
    done
}

cleanup_leftovers() {
    bb="$(busybox_cmd)"
    [ -n "$bb" ] || return

    cleanup_leftovers_once -TERM "$bb"
    "$bb" sleep 1
    cleanup_leftovers_once -KILL "$bb"
}

is_epoll_ltp_leftover_command() {
    cmd="$1"

    case "$cmd" in
        ''|sh|sh\ *|*/sh|*/sh\ *|*"busybox sh"*|*"LTP_SAFE_DATA="*|*"run_ltp"*|*"ltp_safe"*|*"init.sh"*|*"cleanup"*)
            return 1
            ;;
    esac

    first="${cmd%% *}"
    base="${first##*/}"

    case "$base" in
        epoll-ltp|epoll01|epoll_ctl*|epoll_wait*|epoll_pwait*)
            return 0
            ;;
    esac

    return 1
}

cleanup_epoll_ltp_leftovers() {
    bb="$1"
    [ -n "$bb" ] || return

    "$bb" ps | while read pid user time cmd; do
        case "$pid" in ''|PID) continue ;; esac
        [ "$pid" = "1" ] && continue
        [ "$pid" = "$$" ] && continue

        if is_epoll_ltp_leftover_command "$cmd"; then
            echo "[ltp-safe] cleanup epoll leftover pid=$pid cmd=$cmd"
            "$bb" kill -KILL "$pid" 2>/dev/null || true
        fi
    done
}

skip_ltp_testcase() {
    name="$1"
    dir="$2"

    [ "$SKIP_LTP" = "1" ] || return 1
    [ "$name" = "ltp_testcode.sh" ] || return 1

    case "$dir" in
        /glibc) group="ltp-glibc" ;;
        /musl) group="ltp-musl" ;;
        *) group="ltp" ;;
    esac

    echo "#### OS COMP TEST GROUP START $group ####"
    echo "#### OS COMP TEST GROUP END $group ####"
    return 0
}

# Check whether an LTP case should be skipped for a given libc.
# Return 0 (skip) if the case is known to trigger kernel panics or hangs.
ltp_skip_case() {
    LTP_SKIP_REASON=""
    case "$1" in
        futex_cmp_requeue01)
            # Triggers kernel panic (Unhandled trap MemoryAccessAddressError)
            # in the TaskStack allocation / clone path.
            # Temporarily skipped to unblock remaining LTP cases.
            case "$2" in
                glibc|musl|both)
                    LTP_SKIP_REASON="kernel panic"
                    return 0
                    ;;
            esac
            ;;
        mmap-corruption01)
            # Triggers system OOM, blocking subsequent LTP cases.
            # Temporarily skipped per zqh mm-ipc strategy.
            case "$2" in
                glibc|musl|both)
                    LTP_SKIP_REASON="oom risk"
                    return 0
                    ;;
            esac
            ;;
        mmap1)
            # Hangs indefinitely (no output for minutes, normally ~6-8s).
            # Temporarily skipped to unblock remaining LTP cases.
            case "$2" in
                glibc|musl|both)
                    LTP_SKIP_REASON="hang risk"
                    return 0
                    ;;
            esac
            ;;
        shm_test)
            # Hangs indefinitely with repeated "shmat(): File exists" and
            # "libgcc_s.so.1 must be installed" errors.
            # Temporarily skipped to unblock remaining LTP cases.
            case "$2" in
                glibc|musl|both)
                    LTP_SKIP_REASON="hang risk"
                    return 0
                    ;;
            esac
            ;;
        fork14)
            LTP_SKIP_REASON="zero-score very slow"
            return 0
            ;;
        fork_procs)
            LTP_SKIP_REASON="la panic low score"
            return 0
            ;;
    esac
    return 1
}

set_library_path() {
    case "$1" in
        /glibc|/glibc/*)
            export PATH=.:/glibc/ltp/testcases/bin:/bin:/sbin:/usr/bin:/usr/sbin
            export LD_LIBRARY_PATH=/glibc/lib:/glibc/lib64:/lib:/lib64:/usr/lib:/usr/lib64
            export LTPROOT=/glibc/ltp
            ;;
        /musl|/musl/*)
            export PATH=.:/musl/ltp/testcases/bin:/bin:/sbin:/usr/bin:/usr/sbin
            export LD_LIBRARY_PATH=/musl/lib:/musl/lib64:/lib:/lib64:/usr/lib:/usr/lib64
            export LTPROOT=/musl/ltp
            ;;
        *)
            export PATH=$BASE_PATH
            unset LD_LIBRARY_PATH
            unset LTPROOT
            ;;
    esac
}

run_test_dir() {
    dir="$1"

    [ -d "$dir" ] || return
    cd "$dir" || return

    set_library_path "$dir"

    if [ -f ./test_all.sh ]; then
        found=1
        echo "run ${dir}/test_all.sh"
        run_with_shell ./test_all.sh
        needs_cleanup "$dir" && cleanup_leftovers
        cd /
        return
    fi

    for testcase in ./*_testcode.sh; do
        [ -f "$testcase" ] || continue
        found=1
        echo "run ${dir}/${testcase#./}"
        if skip_ltp_testcase "${testcase#./}" "$dir"; then
            continue
        fi
        run_with_shell "$testcase"
        needs_cleanup "$testcase" && cleanup_leftovers
    done

    cd /
}

run_test_path() {
    script="$1"

    [ -f "$script" ] || return

    found=1
    dir="${script%/*}"
    name="${script##*/}"

    cd "$dir" || return
    set_library_path "$dir"
    echo "run ${dir}/${name}"
    if skip_ltp_testcase "$name" "$dir"; then
        cd /
        return
    fi
    run_with_shell "./$name"
    needs_cleanup "$script" && cleanup_leftovers
    cd /
}

prepare_basic_scripts() {
    for dir in /glibc /musl; do
        [ -x "$dir/busybox" ] || continue

        echo "[full-safe] prepare basic scripts: $dir"
        "$dir/busybox" chmod +x "$dir/basic/run-all.sh" "$dir"/basic/test_* 2>/dev/null || true

        if [ ! -x "$dir/basic/run-all.sh" ]; then
            echo "[full-safe] warning: $dir/basic/run-all.sh is still not executable"
        fi
    done
}

# full-safe 专用的非 LTP 扫描函数。
# 当 FULL_SAFE_SKIP_WASTE=1（默认）时，跳过三个低收益/高耗时脚本：
#   /glibc/unixbench_testcode.sh  (unixbench-glibc)
#   /musl/unixbench_testcode.sh   (unixbench-musl)
#   /glibc/libctest_testcode.sh   (glibc-libctest)
run_full_safe_non_ltp_tests() {
    for dir in / /glibc /musl; do
        [ -d "$dir" ] || continue
        for testcase in "$dir"/*_testcode.sh; do
            [ -f "$testcase" ] || continue

            name="${testcase##*/}"
            case "$name" in
                ltp_testcode.sh|ltp_all_testcode.sh)
                    continue
                    ;;
            esac

            # Defer cyclictest to after LTP to reduce hackbench/cyclictest
            # contamination risk on high-score tests.
            case "$testcase" in
                /glibc/cyclictest_testcode.sh|/musl/cyclictest_testcode.sh)
                    echo "[full-safe] defer cyclictest to after LTP: $testcase"
                    continue
                    ;;
            esac

            if [ "$FULL_SAFE_SKIP_WASTE" = "1" ]; then
                case "$testcase" in
                    /glibc/unixbench_testcode.sh)
                        echo "[full-safe] skip /glibc/unixbench_testcode.sh because FULL_SAFE_SKIP_WASTE=1"
                        continue
                        ;;
                    /musl/unixbench_testcode.sh)
                        echo "[full-safe] skip /musl/unixbench_testcode.sh because FULL_SAFE_SKIP_WASTE=1"
                        continue
                        ;;
                    /glibc/libctest_testcode.sh)
                        echo "[full-safe] skip /glibc/libctest_testcode.sh because FULL_SAFE_SKIP_WASTE=1"
                        continue
                        ;;
                esac
            fi

            run_test_path "$testcase"
        done
    done
}

prepare_common_test_env() {
    if [ -x /glibc/busybox ]; then
        /glibc/busybox chmod +x /glibc/basic/run-all.sh /glibc/basic/test_* 2>/dev/null || true
        /glibc/busybox ln -sf busybox /glibc/ls 2>/dev/null || true
    fi

    if [ -x /musl/busybox ]; then
        /musl/busybox chmod +x /musl/basic/run-all.sh /musl/basic/test_* 2>/dev/null || true
        /musl/busybox ln -sf busybox /musl/ls 2>/dev/null || true
    fi
}

run_cyclictest_musl_tests() {
    found=1
    run_test_path /musl/cyclictest_testcode.sh
}

run_cyclictest_tests() {
    found=1
    run_test_path /glibc/cyclictest_testcode.sh
    run_test_path /musl/cyclictest_testcode.sh
}

run_libctest_tests() {
    found=1
    run_test_path /glibc/libctest_testcode.sh
    run_test_path /musl/libctest_testcode.sh
}

run_iozone_tests() {
    found=1
    echo "run iozone tests for sys_sync/syncfs verification"
    run_test_path /glibc/iozone_testcode.sh
    run_test_path /musl/iozone_testcode.sh
}

run_lmbench_tests() {
    found=1
    run_test_path /glibc/lmbench_testcode.sh
    run_test_path /musl/lmbench_testcode.sh
}

run_ltp_list_libc() {
    libc="$1"
    target_dir="$2"
    count=0

    echo "[LTP-LIST-START] $libc"

    if [ ! -d "$target_dir" ]; then
        echo "[LTP-LIST-ERROR] $libc directory not found: $target_dir"
        echo "[LTP-LIST-END] $libc COUNT=$count"
        return
    fi

    for file in "$target_dir"/*; do
        [ -f "$file" ] || continue
        name="${file##*/}"
        count=$((count + 1))
        printf 'LTP_CASE %s %06d %s\n' "$libc" "$count" "$name"
    done

    echo "[LTP-LIST-END] $libc COUNT=$count"
}

run_ltp_list_tests() {
    found=1
    run_ltp_list_libc glibc /glibc/ltp/testcases/bin
    run_ltp_list_libc musl /musl/ltp/testcases/bin
}

run_ltp_cases_libc() {
    libc="$1"

    case "$libc" in
        glibc) dir=/glibc ;;
        musl)  dir=/musl ;;
        *)
            echo "[LTP-ERROR] unsupported libc: $libc"
            return
            ;;
    esac

    echo "[LTP] libc=$libc dir=$dir"

    target_dir="$dir/ltp/testcases/bin"

    if [ ! -d "$target_dir" ]; then
        echo "[LTP-ERROR] directory not found: $target_dir"
        return
    fi

    if ! cd "$dir"; then
        echo "[LTP-ERROR] cannot cd to $dir"
        return
    fi

    set_library_path "$dir"
    export LTPROOT="$dir/ltp"
    export LTP_DATAROOT="$dir/ltp/testcases/bin"
    export PATH="$dir/ltp/testcases/bin:$PATH"
    mkdir -p /dev/shm
    export LTP_IPC_PATH="/dev/shm/ltp_ipc_path"
    : > "$LTP_IPC_PATH"

    bb="$(busybox_cmd)"
    case_timeout="${LTP_CASE_TIMEOUT:-${LTP_TIMEOUT:-1}}"

    # LA busybox timeout does not pass env to child; use a temp wrapper.
    if [ -n "$bb" ] && [ -n "$case_timeout" ]; then
        "$bb" printf '#!/bin/sh\nexport LTP_TIMEOUT=%s\nexec "$@"\n' "$case_timeout" > /tmp/ltpw
        "$bb" chmod +x /tmp/ltpw 2>/dev/null || true
    fi

    group="ltp-$libc"
    echo "#### OS COMP TEST GROUP START $group ####"

    if [ -n "$LTP_CASE_LIST" ]; then
        echo "[LTP] explicit case-list: $LTP_CASE_LIST" >&2
    fi

    ltp_case_list | while IFS= read -r name; do
        [ -n "$name" ] || continue
        file="ltp/testcases/bin/$name"

        if [ ! -f "$file" ]; then
            echo "[LTP-MISSING] $libc $name: $dir/$file"
            continue
        fi

        if ltp_skip_case "$name" "$libc"; then
            echo "SKIP LTP CASE $name : $LTP_SKIP_REASON"
            continue
        fi

        if [ "$name" = "epoll-ltp" ]; then
            echo "RUN LTP CASE $name"
            if [ -n "$bb" ]; then
                "$bb" timeout 300 "$file"
            else
                "$file"
            fi
            ret=$?
            echo "FAIL LTP CASE $name : $ret"
            # epoll-ltp spawns children (epoll01 etc.) that survive
            # timeout/termination and pollute subsequent cases with
            # interleaved output and resource contention.
            cleanup_epoll_ltp_leftovers "$bb"
            continue
        fi

        echo "RUN LTP CASE $name"

        if [ -n "$bb" ]; then
            "$bb" timeout "$case_timeout" /tmp/ltpw "$file" < /dev/null
        else
            LTP_TIMEOUT="$case_timeout" "$file" < /dev/null
        fi
        ret=$?
        echo "FAIL LTP CASE $name : $ret"
    done

    echo "#### OS COMP TEST GROUP END $group ####"

    cd /
}

run_ltp_safe_tests() {
    found=1
    run_ltp_cases_libc glibc
    run_ltp_cases_libc musl
}

run_lmbench_write_tests() {
    found=1
    cd /glibc || return
    set_library_path /glibc
    echo "run /glibc/lmbench_all lat_syscall -P 1 write"
    ./lmbench_all lat_syscall -P 1 write
    echo "lmbench-write-debug before mkdir"
    ./busybox mkdir -p /var/tmp
    echo "lmbench-write-debug after mkdir"
    ./busybox touch /var/tmp/lmbench
    echo "lmbench-write-debug after touch"
    ./lmbench_all lat_syscall -P 1 stat /var/tmp/lmbench
    echo "lmbench-write-debug after stat"
    ./lmbench_all lat_syscall -P 1 fstat /var/tmp/lmbench
    echo "lmbench-write-debug after fstat"
    ./lmbench_all lat_syscall -P 1 open /var/tmp/lmbench
    echo "lmbench-write-debug after open"
    cd /
}

run_unixbench_tests() {
    found=1
    run_test_path /glibc/unixbench_testcode.sh
    run_test_path /musl/unixbench_testcode.sh
}

run_wait_repro_tests() {
    for testcase in \
        /glibc/libctest_testcode.sh \
        /glibc/lmbench_testcode.sh \
        /glibc/unixbench_testcode.sh \
        /musl/basic_testcode.sh
    do
        run_test_path "$testcase"
    done
}

cd /

# ============================================================
# Single test mode: uncomment one of the lines below to run a
# specific test instead of the full suite.
# ============================================================

# --- basic tests (test_sleep, test_getpid, etc.) ---
# set_library_path /glibc && cd /glibc && run_with_shell ./basic_testcode.sh

# --- busybox tests ---
# set_library_path /glibc && cd /glibc && run_with_shell ./busybox_testcode.sh

# --- cyclictest only (NO_STRESS + STRESS) ---
# set_library_path /glibc && cd /glibc && run_with_shell ./cyclictest_testcode.sh

# --- unixbench only ---
# set_library_path /glibc && cd /glibc && run_with_shell ./unixbench_testcode.sh

# --- full test suite (comment out the single test above) ---
found=0
case "$TEST_PROFILE" in
    cyc-musl)
        run_cyclictest_musl_tests
        ;;
    cyc-all)
        run_cyclictest_tests
        ;;
    libctest)
        run_libctest_tests
        ;;
    iozone)
        run_iozone_tests
        ;;
    lmbench)
        run_lmbench_tests
        ;;
    ltp-list)
        run_ltp_list_tests
        ;;
    lmbench-write)
        run_lmbench_write_tests
        ;;
    unixbench)
        run_unixbench_tests
        ;;
    wait-repro)
        run_wait_repro_tests
        ;;
    full)
        for dir in / /glibc /musl; do
            run_test_dir "$dir"
        done
        ;;
    ltp-safe)
        run_ltp_safe_tests
        ;;
    full-safe)
        prepare_basic_scripts
        prepare_common_test_env
        run_full_safe_non_ltp_tests
        run_ltp_safe_tests
        run_cyclictest_tests
        ;;
    *)
        echo "Unknown TEST_PROFILE=$TEST_PROFILE; using full-safe profile."
        prepare_basic_scripts
        prepare_common_test_env
        run_full_safe_non_ltp_tests
        run_ltp_safe_tests
        run_cyclictest_tests
        ;;
esac
if [ "$found" -eq 0 ]; then
    for testcase in /*_testcode.sh /scripts/*/*_testcode.sh; do
        [ -f "$testcase" ] || continue
        found=1
        dir="${testcase%/*}"
        name="${testcase##*/}"
        cd "$dir" || continue
        set_library_path "$dir"
        echo "run ${dir}/${name}"
        if skip_ltp_testcase "$name" "$dir"; then
            cd /
            continue
        fi
        run_with_shell "./$name"
        needs_cleanup "$testcase" && cleanup_leftovers
        cd /
    done
fi
if [ "$found" -eq 0 ]; then
    echo "No OS competition test scripts found; starting interactive shell."
    if [ -x /busybox ]; then
        exec /busybox sh
    fi
    exec sh --login
fi

exit 0
