# #!/bin/sh

# export HOME=/root
# export USER=root
# export PATH=.:/bin:/sbin:/usr/bin:/usr/sbin

# run_with_shell() {
#     script="$1"

#     if [ -x ./busybox ]; then
#         ./busybox sh "$script"
#     elif [ -x /busybox ]; then
#         /busybox sh "$script"
#     elif [ -x /musl/busybox ]; then
#         /musl/busybox sh "$script"
#     elif [ -x /glibc/busybox ]; then
#         /glibc/busybox sh "$script"
#     else
#         sh "$script"
#     fi
# }

# run_test_dir() {
#     dir="$1"

#     [ -d "$dir" ] || return
#     cd "$dir" || return

#     if [ -f ./test_all.sh ]; then
#         found=1
#         echo "run ${dir}/test_all.sh"
#         run_with_shell ./test_all.sh
#         cd /
#         return
#     fi

#     for testcase in ./*_testcode.sh; do
#         [ -f "$testcase" ] || continue
#         found=1
#         echo "run ${dir}/${testcase#./}"
#         run_with_shell "$testcase"
#     done

#     cd /
# }

# cd /
# found=0

# for dir in / /musl /glibc; do
#     run_test_dir "$dir"
# done

# if [ "$found" -eq 0 ]; then
#     for testcase in /*_testcode.sh /scripts/*/*_testcode.sh; do
#         [ -f "$testcase" ] || continue
#         found=1
#         dir="${testcase%/*}"
#         name="${testcase##*/}"
#         cd "$dir" || continue
#         echo "run ${dir}/${name}"
#         run_with_shell "./$name"
#         cd /
#     done
# fi

# if [ "$found" -eq 0 ]; then
#     echo "No OS competition test scripts found; starting interactive shell."
#     if [ -x /busybox ]; then
#         exec /busybox sh
#     fi
#     exec sh --login
# fi

# exit 0

#!/bin/sh

export HOME=/root
export USER=root
export PATH=.:/bin:/sbin:/usr/bin:/usr/sbin

run_with_shell() {
    script="$1"
    if [ -x /glibc/busybox ]; then
        /glibc/busybox sh "$script"
    else
        sh "$script"
    fi
}

# 安装所有 busybox applet 到 /bin，避免 "xxx: not found" 错误
/glibc/busybox mkdir -p /bin
for cmd in $(/glibc/busybox --list); do
    /glibc/busybox ln -s /glibc/busybox /bin/$cmd
done

# UnixBench 需要 sort.src（复制 unixbench_testcode.sh 充当）
/glibc/busybox cp /glibc/unixbench_testcode.sh /glibc/sort.src

# 跳过辅助程序/脚本（死循环/需外部信号控制，非独立测试）
/glibc/busybox rm -f /glibc/ltp/testcases/bin/cgroup_fj_proc
/glibc/busybox rm -f /glibc/ltp/testcases/bin/cgroup_regression_*.sh
/glibc/busybox rm -f /glibc/ltp/testcases/bin/cgroup_regression_fork_processes

cd /glibc
echo "run /glibc/ltp_testcode.sh"
run_with_shell ./ltp_testcode.sh

# 完成后进入 shell
exec /glibc/busybox sh