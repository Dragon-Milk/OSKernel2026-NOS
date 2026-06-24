#!/usr/bin/env bash
set -u

SRC_DIR="${SRC_DIR:-/work/src}"
TEST_DIR="${TEST_DIR:-/test}"
WORK_DIR="${WORK_DIR:-/work}"
LOG_DIR="${LOG_DIR:-/work/logs}"

mkdir -p "$LOG_DIR"

TS="$(date +%Y%m%d-%H%M%S)"
RUN_TAG="${RUN_TAG:-rv-built-run-${TS}}"

timestamp_awk='
BEGIN {
  start=systime();
  last=start;
}
{
  now=systime();
  printf("[%s +%ds Δ%ds] %s\n", strftime("%Y-%m-%d %H:%M:%S", now), now-start, now-last, $0);
  fflush();
  last=now;
}
'

find_kernel() {
  if [ -n "${RV_KERNEL:-}" ] && [ -f "$RV_KERNEL" ]; then
    echo "$RV_KERNEL"
    return 0
  fi

  if [ -f "$SRC_DIR/kernel-rv" ]; then
    echo "$SRC_DIR/kernel-rv"
    return 0
  fi

  if [ -f "$WORK_DIR/kernel-rv" ]; then
    echo "$WORK_DIR/kernel-rv"
    return 0
  fi

  echo "ERROR: kernel-rv not found. Tried:" >&2
  echo "  \$RV_KERNEL=${RV_KERNEL:-}" >&2
  echo "  $SRC_DIR/kernel-rv" >&2
  echo "  $WORK_DIR/kernel-rv" >&2
  return 1
}

prepare_sdcard() {
  cd "$TEST_DIR" || exit 1

  local src_xz="${SDCARD_RV_XZ:-${SDCARD_RV_SRC:-$TEST_DIR/sdcard-rv.img.xz}}"
  local out_img="${SDCARD_RV_OUT:-$TEST_DIR/sdcard-rv-run-${TS}.img}"

  if [ ! -f "$src_xz" ]; then
    echo "ERROR: RV compressed sdcard image not found: $src_xz" >&2
    echo "       expected default: $TEST_DIR/sdcard-rv.img.xz" >&2
    echo "       override with: SDCARD_RV_XZ=/path/to/sdcard-rv.img.xz" >&2
    return 1
  fi

  case "$src_xz" in
    *.xz) ;;
    *)
      echo "ERROR: RV sdcard source must be a .xz image: $src_xz" >&2
      return 1
      ;;
  esac

  case "$out_img" in
    "$TEST_DIR/sdcard-rv.img"|"$TEST_DIR/sdcard-rv1.img"|"$TEST_DIR/sdcard-rv.img.xz")
      echo "ERROR: refusing to overwrite original RV sdcard image: $out_img" >&2
      return 1
      ;;
  esac

  rm -f "$out_img"
  if ! xz -dc "$src_xz" > "$out_img"; then
    echo "ERROR: failed to decompress RV sdcard image: $src_xz" >&2
    rm -f "$out_img"
    return 1
  fi

  sync
  echo "$out_img"
}

cleanup_sdcard() {
  local img="${RV_SDCARD_PATH:-}"
  [ -n "$img" ] || return 0
  [ -f "$img" ] || return 0

  if [ -n "${RV_LOG:-}" ] && [ -f "$RV_LOG" ]; then
    echo "[cleanup] remove temporary RV sdcard image: $img" | tee -a "$RV_LOG"
  else
    echo "[cleanup] remove temporary RV sdcard image: $img"
  fi

  rm -f "$img"
}

on_exit() {
  local status=$?
  cleanup_sdcard
  exit "$status"
}

main() {
  echo "===== RV BUILT KERNEL RUN START: ${RUN_TAG} ====="
  echo "start time: $(date '+%Y-%m-%d %H:%M:%S')"
  echo

  RV_KERNEL_PATH="$(find_kernel)" || exit 1
  RV_SDCARD_PATH="$(prepare_sdcard)" || exit 1
  trap on_exit EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM
  RV_LOG="$LOG_DIR/${RUN_TAG}.log"

  cd "$WORK_DIR" || exit 1

  {
    echo "===== [RV] run built kernel ====="
    echo "kernel: $RV_KERNEL_PATH"
    echo "sdcard: $RV_SDCARD_PATH"
    echo "log: $RV_LOG"
    echo "start: $(date '+%Y-%m-%d %H:%M:%S')"
  } | tee -a "$RV_LOG"

  qemu-system-riscv64 \
    -machine virt \
    -kernel "$RV_KERNEL_PATH" \
    -m 1G \
    -nographic \
    -smp 1 \
    -bios default \
    -drive file="$RV_SDCARD_PATH",if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
    -no-reboot \
    -device virtio-net-device,netdev=net \
    -netdev user,id=net \
    -rtc base=utc \
    2>&1 | awk "$timestamp_awk" | tee -a "$RV_LOG"

  RV_STATUS=${PIPESTATUS[0]}

  {
    echo "===== [RV] qemu exit status: ${RV_STATUS} ====="
    echo "end: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "===== RV BUILT KERNEL RUN END: ${RUN_TAG} ====="
    echo "log saved: $RV_LOG"
  } | tee -a "$RV_LOG"

  sync
  exit "$RV_STATUS"
}

main "$@"
