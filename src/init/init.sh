#!/bin/sh

export HOME=/root
export USER=root
export PATH=.:/bin:/sbin:/usr/bin:/usr/sbin

SKIP_LTP=${SKIP_LTP:-0}
TEST_PROFILE=${TEST_PROFILE:-ltp-batch}
LTP_CATEGORY=${LTP_CATEGORY:-process}
LTP_BATCH=${LTP_BATCH:-all}
LTP_LIBC=${LTP_LIBC:-both}
LTP_TIMEOUT=${LTP_TIMEOUT:-30}
echo "[init] TEST_PROFILE=$TEST_PROFILE"
echo "[init] LTP_CATEGORY=$LTP_CATEGORY LTP_BATCH=$LTP_BATCH LTP_LIBC=$LTP_LIBC LTP_TIMEOUT=$LTP_TIMEOUT"

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
            "$file" </dev/null
            ret=$?
            case "$ret" in
                0) echo "PASS LTP CASE $name : $ret" ;;
                *) echo "FAIL LTP CASE $name : $ret" ;;
            esac
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

run_ltp_case_file() {
    file="$1"
    _case_ret=0

    if [ "$LTP_TIMEOUT" = "0" ]; then
        "$file" </dev/null
        _case_ret=$?
        return $_case_ret
    fi

    case "$LTP_TIMEOUT" in
        ''|*[!0-9]*)
            echo "[LTP-BATCH-TIMEOUT-DISABLED] invalid LTP_TIMEOUT=$LTP_TIMEOUT"
            LTP_TIMEOUT=0
            "$file" </dev/null
            _case_ret=$?
            return $_case_ret
            ;;
    esac

    if command -v timeout >/dev/null 2>&1; then
        timeout "$LTP_TIMEOUT" "$file" </dev/null
        _case_ret=$?
        return $_case_ret
    fi

    bb="$(busybox_cmd)"
    if [ -z "$bb" ]; then
        echo "[LTP-BATCH-TIMEOUT-DISABLED] timeout command not found and no busybox timer available"
        LTP_TIMEOUT=0
        "$file" </dev/null
        _case_ret=$?
        return $_case_ret
    fi

    "$file" </dev/null &
    child=$!
    (
        "$bb" sleep "$LTP_TIMEOUT"
        if "$bb" kill -0 "$child" >/dev/null 2>&1; then
            "$bb" kill -TERM "$child" >/dev/null 2>&1
            "$bb" sleep 1
            "$bb" kill -KILL "$child" >/dev/null 2>&1
        fi
    ) &
    timer=$!

    wait "$child"
    _case_ret=$?
    "$bb" kill "$timer" >/dev/null 2>&1
    wait "$timer" >/dev/null 2>&1

    case "$_case_ret" in
        137|143) return 124 ;;
        *) return "$_case_ret" ;;
    esac
}

ltp_has_case() {
    dir="$1"
    shift
    [ -d "$dir" ] || return 1
    for name in "$@"; do
        [ -f "$dir/$name" ] && return 0
    done
    return 1
}

ltp_bin_dir() {
    libc="$1"
    shift

    libc_dir="/$libc"

    for candidate in \
        "$libc_dir/ltp/testcases/bin" \
        /ltp/testcases/bin \
        ./ltp/testcases/bin \
        . \
        "$libc_dir" \
        ./*/ltp/testcases/bin \
        ./*/*/ltp/testcases/bin
    do
        if ltp_has_case "$candidate" "$@"; then
            cd "$candidate" && pwd
            return
        fi
    done

    return 1
}

ltp_no_bin() {
    libc="$1"
    batches="$2"
    shift 2

    echo "[LTP-BATCH-DIAG] ls /"
    if command -v ls >/dev/null 2>&1; then
        ls /
    else
        bb="$(busybox_cmd)"
        if [ -n "$bb" ]; then
            "$bb" ls /
        else
            echo "[LTP-BATCH-DIAG] ls unavailable"
        fi
    fi

    for path in "/$libc" /glibc /musl /ltp; do
        if [ -d "$path" ] || [ -f "$path" ]; then
            echo "[LTP-BATCH-DIAG] exists $path yes"
        else
            echo "[LTP-BATCH-DIAG] exists $path no"
        fi
    done

    for name in "$@"; do
        echo "[LTP-BATCH-DIAG] case $name"
    done
    echo "[LTP-BATCH-ERROR] ltp bin dir not found: libc=$libc category=$LTP_CATEGORY batches=$batches"
}

ltp_exit_label() {
    # LTP exit codes: 0=TPASS 1=TFAIL 2=TBROK 4=TWARN 32=TCONF
    # timeout (124), killed-by-signal (137=SIGKILL, 143=SIGTERM)
    case "$1" in
        0)   echo "PASS" ;;
        1)   echo "FAIL" ;;
        2)   echo "BROK" ;;
        4)   echo "WARN" ;;
        32)  echo "CONF" ;;
        124) echo "TIMEOUT" ;;
        125) echo "TIMEOUT-ERR" ;;
        126) echo "EXEC-ERR" ;;
        127) echo "NOT-FOUND" ;;
        137) echo "KILLED" ;;
        139) echo "SEGV" ;;
        143) echo "TERM" ;;
        *)   echo "UNKNOWN" ;;
    esac
}

run_ltp_one_batch_libc() {
    libc="$1"
    batch="$2"

    echo "[LTP-BATCH] category=$LTP_CATEGORY batch=$batch libc=$libc"

    ltp_batch_cases "$LTP_CATEGORY" "$batch" | while read name; do
        [ -n "$name" ] || continue
        file="./$name"

        if [ ! -f "$file" ]; then
            echo "[LTP-BATCH-MISSING] $libc $name: $LTP_BIN/$name"
            continue
        fi

        echo "RUN LTP CASE $name"
        run_ltp_case_file "$file"
        case_ret=$?
        label="$(ltp_exit_label "$case_ret")"
        case "$case_ret" in
            0)
                echo "PASS LTP CASE $name : $case_ret"
                ;;
            124|137|143)
                echo "[LTP-BATCH-TIMEOUT] $libc $name after ${LTP_TIMEOUT}s : $case_ret"
                echo "TIMEOUT LTP CASE $name : $case_ret"
                ;;
            *)
                echo "$label LTP CASE $name : $case_ret"
                ;;
        esac
    done
}

run_ltp_batch_libc() {
    libc="$1"

    case "$libc" in
        glibc) dir=/glibc ;;
        musl) dir=/musl ;;
        *)
            echo "[LTP-BATCH-ERROR] unsupported libc: $libc"
            return
            ;;
    esac

    group="ltp-$libc"

    echo "#### OS COMP TEST GROUP START $group ####"

    if [ "$LTP_BATCH" = "all" ]; then
        batches="$(ltp_batch_ids "$LTP_CATEGORY")" || {
            echo "[LTP-BATCH-ERROR] unknown category: $LTP_CATEGORY"
            echo "#### OS COMP TEST GROUP END $group ####"
            return
        }
    else
        batches="$LTP_BATCH"
    fi

    echo "[LTP-BATCH-PLAN] category=$LTP_CATEGORY batches=$batches libc=$libc"

    first_cases=""
    first_count=0
    for batch in $batches; do
        cases="$(ltp_batch_cases "$LTP_CATEGORY" "$batch")" || {
            echo "[LTP-BATCH-ERROR] batch not found: category=$LTP_CATEGORY batch=$batch"
            echo "#### OS COMP TEST GROUP END $group ####"
            return
        }
        for name in $cases; do
            if [ "$first_count" -lt 5 ]; then
                first_cases="$first_cases $name"
                first_count=$((first_count + 1))
            fi
        done
    done

    LTP_BIN="$(ltp_bin_dir "$libc" $first_cases)" || {
        ltp_no_bin "$libc" "$batches" $first_cases
        echo "#### OS COMP TEST GROUP END $group ####"
        return
    }
    export LTP_BIN
    echo "[LTP-BATCH-LTP-BIN] libc=$libc dir=$LTP_BIN"

    if ! cd "$LTP_BIN"; then
        echo "[LTP-BATCH-ERROR] cannot cd to ltp bin dir: $LTP_BIN"
        echo "#### OS COMP TEST GROUP END $group ####"
        return
    fi

    set_library_path "$dir"
    old_path="$PATH"
    export PATH="$LTP_BIN:$PATH"

    for batch in $batches; do
        run_ltp_one_batch_libc "$libc" "$batch"
    done

    export PATH="$old_path"
    cd /
    echo "#### OS COMP TEST GROUP END $group ####"
}

run_ltp_batch_tests() {
    found=1

    case "$LTP_CATEGORY" in
        process|fs|mm-ipc|common-easy|net|net-core|net-all|net-script|net-deferred) ;;
        *)
            echo "[LTP-BATCH-ERROR] unsupported category: $LTP_CATEGORY"
            return
            ;;
    esac

    if [ "$LTP_BATCH" = "all" ]; then
        if ! ltp_batch_ids "$LTP_CATEGORY" >/dev/null 2>&1; then
            echo "[LTP-BATCH-ERROR] unknown category: $LTP_CATEGORY"
            return
        fi
    else
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
