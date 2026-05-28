#!/bin/sh

export HOME=/root
export USER=root
export PATH=.:/bin:/sbin:/usr/bin:/usr/sbin

# Set SKIP_LTP=0 to run the original ltp_testcode.sh scripts again.
SKIP_LTP=${SKIP_LTP:-1}

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

run_test_dir() {
    dir="$1"

    [ -d "$dir" ] || return
    cd "$dir" || return

    if [ -f ./test_all.sh ]; then
        found=1
        echo "run ${dir}/test_all.sh"
        run_with_shell ./test_all.sh
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
    done

    cd /
}

cd /
found=0

for dir in / /glibc /musl; do
    run_test_dir "$dir"
done

if [ "$found" -eq 0 ]; then
    for testcase in /*_testcode.sh /scripts/*/*_testcode.sh; do
        [ -f "$testcase" ] || continue
        found=1
        dir="${testcase%/*}"
        name="${testcase##*/}"
        cd "$dir" || continue
        echo "run ${dir}/${name}"
        if skip_ltp_testcase "$name" "$dir"; then
            cd /
            continue
        fi
        run_with_shell "./$name"
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
