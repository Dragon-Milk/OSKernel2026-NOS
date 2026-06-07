#!/bin/sh

export HOME=/root
export USER=root
export PATH=.:/bin:/sbin:/usr/bin:/usr/sbin

# Set SKIP_LTP=0 to run the original ltp_testcode.sh scripts again.
# ============================================================
# Select test profile here.
# stable        : current safe 910-score profile
# cyc-musl      : run musl cyclictest only
# cyc-all       : run glibc and musl cyclictest only
# libctest      : run glibc and musl libctest only
# iozone        : run glibc and musl iozone to verify sys_sync/syncfs
# lmbench       : run glibc and musl lmbench only
# lmbench-only  : run original glibc lmbench script
# lmbench-fast  : run trimmed glibc lmbench
# perf          : run stable profile with kernel-side perf summary when built with perf-profile
# unixbench     : run glibc and musl unixbench only
# wait-repro    : run wait/libctest/lmbench/unixbench repro
# full          : scan and run all testcode scripts
# ============================================================
SKIP_LTP=${SKIP_LTP:-1}
TEST_PROFILE=${TEST_PROFILE:-full}

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

is_leftover_command() {
    case "$1" in
        "./iperf3 -s"*|"iperf3 -s"*|"/glibc/iperf3 -s"*|"/musl/iperf3 -s"*| \
        "./netserver"*|"netserver"*|"/glibc/netserver"*|"/musl/netserver"*| \
        "./lmbench_all"*|"lmbench_all"*|"/glibc/lmbench_all"*|"/musl/lmbench_all"*| \
        "./pipe 10"*|"pipe 10"*|"/glibc/pipe 10"*|"/musl/pipe 10"*| \
        "./busybox sh ./lmbench_testcode.sh"*|"/glibc/busybox sh ./lmbench_testcode.sh"*|"/musl/busybox sh ./lmbench_testcode.sh"*| \
        "./busybox sh ./unixbench_testcode.sh"*|"/glibc/busybox sh ./unixbench_testcode.sh"*|"/musl/busybox sh ./unixbench_testcode.sh"*)
            return 0
            ;;
    esac

    return 1
}

needs_cleanup() {
    case "$1" in
        *iperf*|*netperf*|*lmbench*|*unixbench*)
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

set_library_path() {
    case "$1" in
        /glibc|/glibc/*) export LD_LIBRARY_PATH=/glibc/lib ;;
        /musl|/musl/*) export LD_LIBRARY_PATH=/musl/lib ;;
        *) unset LD_LIBRARY_PATH ;;
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

run_stable_tests() {
    for testcase in \
        /glibc/basic_testcode.sh \
        /glibc/busybox_testcode.sh \
        /glibc/cyclictest_testcode.sh \
        /glibc/iozone_testcode.sh \
        /glibc/iperf_testcode.sh \
        /glibc/libcbench_testcode.sh \
        /glibc/lua_testcode.sh \
        /glibc/netperf_testcode.sh \
        /musl/basic_testcode.sh \
        /musl/busybox_testcode.sh \
        /musl/lua_testcode.sh \
        /musl/iozone_testcode.sh \
        /musl/iperf_testcode.sh \
        /musl/netperf_testcode.sh \
        /musl/libcbench_testcode.sh
    do
        run_test_path "$testcase"
    done
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

run_lmbench_only_tests() {
    run_test_path /glibc/lmbench_testcode.sh
}

run_lmbench_fast_tests() {
    found=1
    cd /glibc || return
    set_library_path /glibc
    echo "run /glibc/lmbench-fast"
    echo "#### OS COMP TEST GROUP START lmbench-glibc ####"

    echo latency measurements
    ./lmbench_all lat_syscall -P 1 null
    ./lmbench_all lat_syscall -P 1 read
    ./lmbench_all lat_syscall -P 1 write
    ./busybox mkdir -p /var/tmp
    ./busybox touch /var/tmp/lmbench
    ./lmbench_all lat_syscall -P 1 stat /var/tmp/lmbench
    ./lmbench_all lat_syscall -P 1 fstat /var/tmp/lmbench
    ./lmbench_all lat_syscall -P 1 open /var/tmp/lmbench
    ./lmbench_all lat_select -n 100 -P 1 file
    ./lmbench_all lat_sig -P 1 install
    ./lmbench_all lat_sig -P 1 catch
    ./lmbench_all lat_pipe -P 1
    ./lmbench_all lat_proc -P 1 fork
    ./lmbench_all lat_proc -P 1 exec
    ./lmbench_all lat_proc -P 1 shell
    ./lmbench_all lmdd label="File /var/tmp/XXX write bandwidth:" of=/var/tmp/XXX move=1m fsync=1 print=3
    ./lmbench_all lat_pagefault -P 1 /var/tmp/XXX
    ./lmbench_all lat_mmap -P 1 512k /var/tmp/XXX

    echo Bandwidth measurements
    ./lmbench_all bw_pipe -P 1

    echo "#### OS COMP TEST GROUP END lmbench-glibc ####"
    cd /
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
    stable)
        run_stable_tests
        ;;
    stable-lmbench)
        run_stable_tests
        run_lmbench_only_tests
        ;;
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
    lmbench-only)
        run_lmbench_only_tests
        ;;
    lmbench-fast)
        run_lmbench_fast_tests
        ;;
    perf)
        run_stable_tests
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
    *)
        echo "Unknown TEST_PROFILE=$TEST_PROFILE; using stable profile."
        run_stable_tests
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
