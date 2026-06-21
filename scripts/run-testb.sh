#!/bin/sh
set -eu

ARCH="${ARCH:-riscv64}"
LIBC="${LTP_LIBC:-both}"
CASE_TIMEOUT="${LTP_CASE_TIMEOUT:-45}"

case "$ARCH" in
    riscv64|rv)
        target=testb-rv
        disk_img="${DISK_IMG:-sdcard-rv.img}"
        ;;
    loongarch64|la)
        target=testb-la
        disk_img="${DISK_IMG:-sdcard-la.img}"
        ;;
    *)
        echo "unsupported ARCH=$ARCH, use riscv64/rv or loongarch64/la" >&2
        exit 2
        ;;
esac

exec make "$target" DISK_IMG="$disk_img" LTP_LIBC="$LIBC" LTP_CASE_TIMEOUT="$CASE_TIMEOUT"
