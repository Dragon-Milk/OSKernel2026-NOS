#!/usr/bin/env bash
set -u

SRC_DIR="${SRC_DIR:-/work/src}"
TEST_DIR="${TEST_DIR:-/test}"
WORK_DIR="${WORK_DIR:-/work}"
LOG_DIR="${LOG_DIR:-/work/logs}"

mkdir -p "$LOG_DIR"

TS="$(date +%Y%m%d-%H%M%S)"
RUN_TAG="${RUN_TAG:-la-built-run-${TS}}"

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
  if [ -n "${LA_KERNEL:-}" ] && [ -f "$LA_KERNEL" ]; then
    echo "$LA_KERNEL"
    return 0
  fi

  if [ -f "$SRC_DIR/kernel-la" ]; then
    echo "$SRC_DIR/kernel-la"
    return 0
  fi

  if [ -f "$WORK_DIR/kernel-la" ]; then
    echo "$WORK_DIR/kernel-la"
    return 0
  fi

  echo "ERROR: kernel-la not found. Tried:" >&2
  echo "  \$LA_KERNEL=${LA_KERNEL:-}" >&2
  echo "  $SRC_DIR/kernel-la" >&2
  echo "  $WORK_DIR/kernel-la" >&2
  return 1
}

prepare_sdcard() {
  cd "$TEST_DIR" || exit 1

  local src_xz="${SDCARD_LA_XZ:-${SDCARD_LA_SRC:-$TEST_DIR/sdcard-la.img.xz}}"
  local out_img="${SDCARD_LA_OUT:-$TEST_DIR/sdcard-la-run-${TS}.img}"

  if [ ! -f "$src_xz" ]; then
    echo "ERROR: LA compressed sdcard image not found: $src_xz" >&2
    echo "       expected default: $TEST_DIR/sdcard-la.img.xz" >&2
    echo "       override with: SDCARD_LA_XZ=/path/to/sdcard-la.img.xz" >&2
    return 1
  fi

  case "$src_xz" in
    *.xz) ;;
    *)
      echo "ERROR: LA sdcard source must be a .xz image: $src_xz" >&2
      return 1
      ;;
  esac

  case "$out_img" in
    "$TEST_DIR/sdcard-la.img"|"$TEST_DIR/sdcard-la1.img"|"$TEST_DIR/sdcard-la.img.xz")
      echo "ERROR: refusing to overwrite original LA sdcard image: $out_img" >&2
      return 1
      ;;
  esac

  rm -f "$out_img"
  if ! xz -dc "$src_xz" > "$out_img"; then
    echo "ERROR: failed to decompress LA sdcard image: $src_xz" >&2
    rm -f "$out_img"
    return 1
  fi

  sync
  echo "$out_img"
}

cleanup_sdcard() {
  local img="${LA_SDCARD_PATH:-}"
  [ -n "$img" ] || return 0
  [ -f "$img" ] || return 0

  if [ -n "${LA_LOG:-}" ] && [ -f "$LA_LOG" ]; then
    echo "[cleanup] remove temporary LA sdcard image: $img" | tee -a "$LA_LOG"
  else
    echo "[cleanup] remove temporary LA sdcard image: $img"
  fi

  rm -f "$img"
}

on_exit() {
  local status=$?
  cleanup_sdcard
  exit "$status"
}

main() {
  echo "===== LA BUILT KERNEL RUN START: ${RUN_TAG} ====="
  echo "start time: $(date '+%Y-%m-%d %H:%M:%S')"
  echo

  LA_KERNEL_PATH="$(find_kernel)" || exit 1
  LA_SDCARD_PATH="$(prepare_sdcard)" || exit 1
  trap on_exit EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM
  LA_LOG="$LOG_DIR/${RUN_TAG}.log"

  cd "$WORK_DIR" || exit 1

  {
    echo "===== [LA] run built kernel ====="
    echo "kernel: $LA_KERNEL_PATH"
    echo "sdcard: $LA_SDCARD_PATH"
    echo "log: $LA_LOG"
    echo "start: $(date '+%Y-%m-%d %H:%M:%S')"
  } | tee -a "$LA_LOG"

  qemu-system-loongarch64 \
    -machine virt \
    -kernel "$LA_KERNEL_PATH" \
    -m 1G \
    -nographic \
    -smp 1 \
    -drive file="$LA_SDCARD_PATH",if=none,format=raw,id=x0 \
    -device virtio-blk-pci,drive=x0 \
    -no-reboot \
    -device virtio-net-pci,netdev=net0 \
    -netdev user,id=net0,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555 \
    -rtc base=utc \
    2>&1 | awk "$timestamp_awk" | tee -a "$LA_LOG"

  LA_STATUS=${PIPESTATUS[0]}

  {
    echo "===== [LA] qemu exit status: ${LA_STATUS} ====="
    echo "end: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "===== LA BUILT KERNEL RUN END: ${RUN_TAG} ====="
    echo "log saved: $LA_LOG"
  } | tee -a "$LA_LOG"

  sync
  exit "$LA_STATUS"
}

main "$@"
