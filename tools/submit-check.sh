#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CHECK_DIR="${CHECK_DIR:-$ROOT/.submit-check-cargo}"
DOCKER_IMAGE="${DOCKER_IMAGE:-zhouzhouyi/os-contest:20260510}"

# 轻量 cargo build 参数
CARGO_TARGETS="${CARGO_TARGETS:-riscv64gc-unknown-none-elf loongarch64-unknown-none-softfloat}"
CARGO_FEATURES="${CARGO_FEATURES:-axfeat/defplat axfeat/bus-mmio axfeat/dwarf qemu}"
CARGO_PROFILE_ARGS="${CARGO_PROFILE_ARGS:---release}"
CARGO_OFFLINE_ARGS="${CARGO_OFFLINE_ARGS:---locked --offline}"

echo "[1/5] show git workspace status"
cd "$ROOT"

if [ -n "$(git status --porcelain)" ]; then
    echo "WARNING: working tree is not clean."
    echo "This lightweight check uses a fresh clone, so uncommitted changes will NOT be included."
    git status --short
fi

echo "[2/5] fresh clone"
rm -rf "$CHECK_DIR"
git clone --recursive "$ROOT" "$CHECK_DIR"

echo "[3/5] clean cloned workspace"
cd "$CHECK_DIR"
git clean -xfd

echo "[4/5] check submitted layout"
test -f "$CHECK_DIR/Makefile" || {
    echo "ERROR: root Makefile is missing."
    exit 1
}

test -d "$CHECK_DIR/src" || {
    echo "ERROR: src directory is missing in fresh clone."
    exit 1
}

test -f "$CHECK_DIR/src/Cargo.toml" || {
    echo "ERROR: src/Cargo.toml is missing in fresh clone."
    echo "This usually means it is untracked, ignored, or not committed."
    exit 1
}

echo "CHECK_DIR=$(realpath "$CHECK_DIR")"
echo "Cargo.toml found: $CHECK_DIR/src/Cargo.toml"

if [ -d "$CHECK_DIR/src/.cargo" ]; then
    echo "WARNING: src/.cargo exists."
    echo "Official evaluator may filter hidden files/directories; make sure root Makefile can recreate it."
fi

if [ -f "$CHECK_DIR/src/cargo-config.toml" ]; then
    echo "found src/cargo-config.toml"
else
    echo "WARNING: src/cargo-config.toml not found."
fi

echo "[5/5] cargo metadata and cargo build in container"

DOCKER_ENV_ARGS=()
if [ -n "${RUSTUP_TOOLCHAIN:-}" ]; then
    DOCKER_ENV_ARGS+=("-e" "RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN}")
fi

docker run --rm \
    "${DOCKER_ENV_ARGS[@]}" \
    -v "$(realpath "$CHECK_DIR")":/work \
    -v ~/img:/test \
    -w /work \
    "$DOCKER_IMAGE" \
    bash -lc "
        set -euo pipefail

        echo 'toolchains:'
        rustup toolchain list

        echo 'installed targets:'
        rustup target list --installed

        cd /work/src

        # 不跑 make，但尽量模拟 root Makefile 可能做的 .cargo 恢复逻辑。
        if [ -f cargo-config.toml ]; then
            mkdir -p .cargo
            cp cargo-config.toml .cargo/config.toml
            echo 'refreshed .cargo/config.toml from cargo-config.toml'
        fi

        echo 'cargo metadata'
        cargo metadata ${CARGO_OFFLINE_ARGS} >/tmp/cargo-metadata.json
        echo 'cargo metadata ok'

        for target in ${CARGO_TARGETS}; do
            echo \"cargo build target=\$target\"
            cargo build \
                --target \"\$target\" \
                --target-dir /work/src/target \
                ${CARGO_OFFLINE_ARGS} \
                ${CARGO_PROFILE_ARGS} \
                --features \"${CARGO_FEATURES}\"
        done

        echo 'cargo build check passed'
    "

echo "lightweight cargo pre-check passed"
