#!/bin/sh

export HOME=/root
export USER=root
BASE_PATH=.:/bin:/sbin:/usr/bin:/usr/sbin
export PATH=$BASE_PATH

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
# ltp-only      : run glibc and musl ltp only
# ltp-list      : list glibc and musl ltp testcase names only
# ltp-batch     : run one or all embedded phase1 LTP category batches (LTP_BATCH=all|01|02...)
# perf          : run stable profile with kernel-side perf summary when built with perf-profile
# unixbench     : run glibc and musl unixbench only
# wait-repro    : run wait/libctest/lmbench/unixbench repro
# full          : scan and run all testcode scripts
# full-safe     : run all non-LTP testcode scripts plus LTP safe whitelist from ltp-safe.txt
# ============================================================
SKIP_LTP=${SKIP_LTP:-0}
TEST_PROFILE=${TEST_PROFILE:-ltp-batch}
LTP_CATEGORY=${LTP_CATEGORY:-process}
LTP_BATCH=${LTP_BATCH:-all}
LTP_LIBC=${LTP_LIBC:-both}
echo "[init] TEST_PROFILE=$TEST_PROFILE"
echo "[init] LTP_CATEGORY=$LTP_CATEGORY LTP_BATCH=$LTP_BATCH LTP_LIBC=$LTP_LIBC"

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
    mkdir -p /etc || return
    [ -f /etc/passwd ] || : > /etc/passwd
    [ -f /etc/group ] || : > /etc/group

    ensure_named_entry /etc/passwd root 'root:x:0:0:root:/root:/bin/sh'
    ensure_named_entry /etc/passwd nobody 'nobody:x:65534:65534:nobody:/:/sbin/nologin'
    ensure_named_entry /etc/group root 'root:x:0:'
    ensure_named_entry /etc/group daemon 'daemon:x:1:'
    ensure_named_entry /etc/group nobody 'nobody:x:65534:'
}

ensure_user_database

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

run_all_non_ltp_testcode_tests() {
    # Scan / /glibc /musl for *_testcode.sh, skip ltp_testcode.sh and test_all.sh.
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

            run_test_path "$testcase"
        done
    done
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

run_ltp_dir() {
    dir="$1"
    group="$2"
    target_dir="ltp/testcases/bin"

    echo "run ${dir}/ltp"
    echo "#### OS COMP TEST GROUP START $group ####"

    if [ -d "$dir/$target_dir" ] && cd "$dir"; then
        set_library_path "$dir"

        for name in abs01 brk01 brk02; do
            file="$target_dir/$name"
            [ -f "$file" ] || continue

            echo "RUN LTP CASE $name"
            "$file"
            ret=$?
            echo "FAIL LTP CASE $name : $ret"
        done

        cd /
    fi

    echo "#### OS COMP TEST GROUP END $group ####"
}

run_ltp_only_tests() {
    found=1
    SKIP_LTP=0
    run_ltp_dir /glibc ltp-glibc
    run_ltp_dir /musl ltp-musl
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

run_ltp_one_batch_libc() {
    libc="$1"
    batch="$2"

    case "$libc" in
        glibc) dir=/glibc ;;
        musl) dir=/musl ;;
        *)
            echo "[LTP-BATCH-ERROR] unsupported libc: $libc"
            return
            ;;
    esac

    target_dir="$dir/ltp/testcases/bin"

    echo "[LTP-BATCH] category=$LTP_CATEGORY batch=$batch libc=$libc"

    if [ ! -d "$target_dir" ]; then
        echo "[LTP-BATCH-ERROR] $libc directory not found: $target_dir"
        return
    fi

    if ! cd "$dir"; then
        echo "[LTP-BATCH-ERROR] cannot cd to $dir"
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
    case_timeout="${LTP_CASE_TIMEOUT:-45}"

    ltp_batch_cases "$LTP_CATEGORY" "$batch" | while read name; do
        [ -n "$name" ] || continue
        file="ltp/testcases/bin/$name"

        if [ ! -f "$file" ]; then
            echo "[LTP-BATCH-MISSING] $libc $name: $dir/$file"
            continue
        fi

        echo "RUN LTP CASE $name"

        if [ -n "$bb" ]; then
            "$bb" timeout "$case_timeout" "$file"
        else
            "$file"
        fi
        ret=$?
        echo "FAIL LTP CASE $name : $ret"
    done

    cd /
}

run_ltp_batch_libc() {
    libc="$1"
    group="ltp-$libc"

    if [ "$LTP_BATCH" = "all" ]; then
        batches="$(ltp_batch_ids "$LTP_CATEGORY")" || {
            echo "[LTP-BATCH-ERROR] category not found: $LTP_CATEGORY"
            return
        }
    else
        batches="$LTP_BATCH"
    fi

    # count batches
    batch_count=0
    for b in $batches; do
        batch_count=$((batch_count + 1))
    done
    echo "[LTP-BATCH-PLAN] category=$LTP_CATEGORY batches=$batch_count libc=$libc"

    echo "#### OS COMP TEST GROUP START $group ####"

    for batch in $batches; do
        run_ltp_one_batch_libc "$libc" "$batch"
    done

    echo "#### OS COMP TEST GROUP END $group ####"
}

run_ltp_batch_tests() {
    found=1

    case "$LTP_CATEGORY" in
        process|fs|mm-ipc|common-easy) ;;
        *)
            echo "[LTP-BATCH-ERROR] unsupported category: $LTP_CATEGORY"
            return
            ;;
    esac

    if [ "$LTP_BATCH" != "all" ]; then
        if ! ltp_batch_cases "$LTP_CATEGORY" "$LTP_BATCH" >/dev/null 2>&1; then
            echo "[LTP-BATCH-ERROR] batch not found: category=$LTP_CATEGORY batch=$LTP_BATCH"
            return
        fi
    fi

    case "$LTP_LIBC" in
        glibc|musl)
            run_ltp_batch_libc "$LTP_LIBC"
            ;;
        both)
            run_ltp_batch_libc glibc
            run_ltp_batch_libc musl
            ;;
        *)
            echo "[LTP-BATCH-ERROR] unsupported libc: $LTP_LIBC"
            ;;
    esac
}

run_ltp_safe_libc() {
    libc="$1"

    case "$libc" in
        glibc) dir=/glibc ;;
        musl)  dir=/musl ;;
        *)
            echo "[LTP-SAFE-ERROR] unsupported libc: $libc"
            return
            ;;
    esac

    echo "[LTP-SAFE] libc=$libc dir=$dir"

    target_dir="$dir/ltp/testcases/bin"

    if [ ! -d "$target_dir" ]; then
        echo "[LTP-SAFE-ERROR] directory not found: $target_dir"
        return
    fi

    if ! cd "$dir"; then
        echo "[LTP-SAFE-ERROR] cannot cd to $dir"
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
    case_timeout="${LTP_CASE_TIMEOUT:-45}"

    group="ltp-$libc"
    echo "#### OS COMP TEST GROUP START $group ####"

    ltp_safe_cases | while read name; do
        [ -n "$name" ] || continue
        file="ltp/testcases/bin/$name"

        if [ ! -f "$file" ]; then
            echo "[LTP-SAFE-MISSING] $libc $name: $dir/$file"
            continue
        fi

        echo "RUN LTP CASE $name"

        if [ -n "$bb" ]; then
            "$bb" timeout "$case_timeout" "$file"
        else
            "$file"
        fi
        ret=$?
        echo "FAIL LTP CASE $name : $ret"
    done

    echo "#### OS COMP TEST GROUP END $group ####"

    cd /
}

run_ltp_safe_tests() {
    found=1

    case "$LTP_LIBC" in
        glibc|musl)
            run_ltp_safe_libc "$LTP_LIBC"
            ;;
        both)
            run_ltp_safe_libc glibc
            run_ltp_safe_libc musl
            ;;
        *)
            echo "[LTP-SAFE-ERROR] unsupported libc: $LTP_LIBC"
            ;;
    esac
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
    ltp-only)
        run_ltp_only_tests
        ;;
    ltp-list)
        run_ltp_list_tests
        ;;
    ltp-batch)
        run_ltp_batch_tests
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
    full-safe)
        run_all_non_ltp_testcode_tests
        run_ltp_safe_tests
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
