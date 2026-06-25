#!/usr/bin/env bash
set -u

FAIL=0
WARN=0
HAVE_CARGO=0

say() { printf '%s\n' "$*"; }
ok() { printf '[OK] %s\n' "$*"; }
warn() { printf '[WARN] %s\n' "$*"; WARN=$((WARN + 1)); }
fail() { printf '[FAIL] %s\n' "$*"; FAIL=$((FAIL + 1)); }

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        fail "缺少命令：$1"
        return 1
    fi
    return 0
}

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$ROOT" ]; then
    echo "[FAIL] 当前目录不在 git 仓库中"
    exit 1
fi
cd "$ROOT" || exit 1

TMP_ROOT="$(mktemp -d /tmp/light-submit-check.XXXXXX)"
cleanup() {
    rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

check_vendor_tree_dir() {
    local tree_dir="$1"
    local label="$2"

    python3 - "$tree_dir" "$label" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
label = sys.argv[2]
vendor = root / "src" / "vendor"

if not vendor.exists():
    print(f"[WARN] {label}: src/vendor 不存在，跳过 vendor checksum 检查")
    sys.exit(0)

missing = []
mismatch = []
bad_json = []

for checksum in vendor.rglob(".cargo-checksum.json"):
    crate_dir = checksum.parent
    try:
        data = json.loads(checksum.read_text())
    except Exception as e:
        bad_json.append((checksum.relative_to(root), str(e)))
        continue

    files = data.get("files", {})
    if not isinstance(files, dict):
        continue

    for rel, expected in files.items():
        path = crate_dir / rel
        rel_path = path.relative_to(root)

        if not path.exists():
            missing.append(rel_path)
            continue

        if isinstance(expected, str) and expected:
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
            if actual != expected:
                mismatch.append((rel_path, expected, actual))

if bad_json:
    print(f"[FAIL] {label}: 存在无法解析的 .cargo-checksum.json")
    for path, err in bad_json[:30]:
        print(f"  {path}: {err}")
    if len(bad_json) > 30:
        print(f"  ... 仅显示前 30 个，共 {len(bad_json)} 个")
    sys.exit(2)

if missing:
    print(f"[FAIL] {label}: vendor checksum 声明存在但实际缺失的文件")
    for p in missing[:80]:
        print(f"  {p}")
    if len(missing) > 80:
        print(f"  ... 仅显示前 80 个，共 {len(missing)} 个")
    sys.exit(1)

if mismatch:
    print(f"[FAIL] {label}: vendor 文件内容与 .cargo-checksum.json 不匹配")
    for p, exp, act in mismatch[:40]:
        print(f"  {p}")
        print(f"    expected: {exp}")
        print(f"    actual:   {act}")
    if len(mismatch) > 40:
        print(f"  ... 仅显示前 40 个，共 {len(mismatch)} 个")
    sys.exit(1)

print(f"[OK] {label}: vendor checksum 未发现缺失或不匹配文件")
PY
}

check_vendor_lock_tree() {
    local tree_dir="$1"
    local label="$2"
    local checker="$tree_dir/tools/check-vendor-lock.sh"

    if [ ! -f "$checker" ]; then
        warn "$label: tools/check-vendor-lock.sh 不存在，跳过 Cargo.lock/vendor 精确版本检查"
        return 0
    fi

    if bash "$checker" "$tree_dir/src"; then
        ok "$label: Cargo.lock 中 registry crate 均有精确 vendor 版本"
    else
        return 1
    fi
}

check_cargo_offline_metadata_tree() {
    local tree_dir="$1"
    local label="$2"

    if [ "$HAVE_CARGO" != "1" ]; then
        warn "$label: 本机缺少 cargo，跳过 cargo offline metadata 检查"
        return 0
    fi

    local log="$TMP_ROOT/${label//[^A-Za-z0-9_.-]/_}.cargo-metadata.log"
    local found=0
    local bad=0

    for manifest in \
        "$tree_dir/src/Cargo.toml" \
        "$tree_dir/src/tools/"*/Cargo.toml
    do
        [ -f "$manifest" ] || continue
        found=1

        say "[cargo metadata] $label: ${manifest#$tree_dir/}"

        local cargo_home
        cargo_home="$(mktemp -d "$TMP_ROOT/cargo-home.XXXXXX")"

        if (
            cd "$(dirname "$manifest")" &&
            CARGO_HOME="$cargo_home" CARGO_NET_OFFLINE=true cargo metadata --offline --locked --format-version 1 --manifest-path "$manifest" >/dev/null
        ) >"$log" 2>&1; then
            ok "$label: ${manifest#$tree_dir/} 离线依赖解析通过"
        else
            say "[FAIL] $label: ${manifest#$tree_dir/} 离线依赖解析失败"
            sed -n '1,160p' "$log"
            bad=1
        fi
    done

    if [ "$found" = "0" ]; then
        warn "$label: 未发现 Cargo.toml，跳过 cargo metadata 检查"
    fi

    return "$bad"
}

export_ref_tree() {
    local ref="$1"
    local out="$2"

    mkdir -p "$out"
    if git archive "$ref" | tar -x -C "$out"; then
        return 0
    fi
    return 1
}

check_ltp_safe_dups_file() {
    local file="$1"
    local label="$2"

    if [ ! -f "$file" ]; then
        warn "$label: ltp-safe.txt 不存在，跳过重复检查"
        return 0
    fi

    local dup_count
    dup_count="$(grep -v '^[[:space:]]*$' "$file" | grep -v '^[[:space:]]*#' | sort | uniq -d | wc -l | tr -d ' ')"

    if [ "$dup_count" != "0" ]; then
        warn "$label: ltp-safe.txt 存在重复 case：$dup_count 个"
        grep -v '^[[:space:]]*$' "$file" | grep -v '^[[:space:]]*#' | sort | uniq -d | sed -n '1,80p'
        if [ "$dup_count" -gt 80 ]; then
            say "... 仅显示前 80 个"
        fi
    else
        ok "$label: ltp-safe.txt 无重复 case"
    fi
}

check_ltp_safe_dups_ref() {
    local ref="$1"
    local label="$2"
    local tmp="$TMP_ROOT/${label//[^A-Za-z0-9_.-]/_}.ltp-safe.txt"

    if git show "$ref:src/init/ltp-cases/ltp-safe.txt" > "$tmp" 2>/dev/null; then
        check_ltp_safe_dups_file "$tmp" "$label"
    else
        warn "$label: ref 中不存在 src/init/ltp-cases/ltp-safe.txt"
    fi
}

say "===== light submit check ====="
say "repo: $ROOT"
say "branch: $(git branch --show-current 2>/dev/null || echo '?')"
say "head: $(git rev-parse --short HEAD 2>/dev/null || echo '?')"
say

say "===== 0. required tools ====="
need_cmd git
need_cmd tar
need_cmd python3
if command -v cargo >/dev/null 2>&1; then
    HAVE_CARGO=1
    ok "cargo 存在"
else
    fail "缺少命令：cargo"
fi
say

say "===== 1. worktree status ====="
if [ -n "$(git status --porcelain)" ]; then
    fail "工作区不干净。当前文件系统检查不能代表平台 clean checkout，请先 commit/stash/revert 后再提交平台。"
    git status --short
else
    ok "工作区干净"
fi
say

say "===== 2. root build entry ====="
if [ -f Makefile ]; then
    ok "根目录 Makefile 存在"
else
    fail "根目录 Makefile 不存在，平台 make all 会失败"
fi

if [ -f Makefile ] && grep -qE '(^|[[:space:]])all:' Makefile 2>/dev/null; then
    ok "根目录 Makefile 包含 all 目标"
else
    warn "根目录 Makefile 未发现 all 目标，请确认平台 make all 能正常构建"
fi

if [ -f src/Makefile ]; then
    ok "src/Makefile 存在"
else
    warn "src/Makefile 不存在或路径异常"
fi
say

say "===== 3. vendor ignored/untracked files in worktree ====="
if [ -d src/vendor ]; then
    ignored_vendor_count="$(git ls-files -o -i --exclude-standard src/vendor 2>/dev/null | wc -l | tr -d ' ')"
    if [ "$ignored_vendor_count" != "0" ]; then
        fail "src/vendor 下存在被 .gitignore 忽略但未跟踪的文件：$ignored_vendor_count 个。平台 clean checkout 不会带这些文件。"
        git ls-files -o -i --exclude-standard src/vendor | sed -n '1,80p'
        if [ "$ignored_vendor_count" -gt 80 ]; then
            say "... 仅显示前 80 个"
        fi
        say "常用修复：git ls-files -o -i --exclude-standard src/vendor -z | xargs -0 git add -f"
    else
        ok "src/vendor 下没有 ignored+untracked 文件"
    fi
else
    warn "src/vendor 不存在，若项目依赖离线 vendor，平台可能失败"
fi
say

say "===== 4a. worktree vendor checksum ====="
check_vendor_tree_dir "$ROOT" "worktree"
rc=$?
if [ "$rc" -ne 0 ]; then
    FAIL=$((FAIL + 1))
fi
say

say "===== 4b. worktree Cargo.lock/vendor exact match ====="
check_vendor_lock_tree "$ROOT" "worktree"
rc=$?
if [ "$rc" -ne 0 ]; then
    FAIL=$((FAIL + 1))
fi
say

say "===== 4c. worktree cargo offline metadata ====="
check_cargo_offline_metadata_tree "$ROOT" "worktree"
rc=$?
if [ "$rc" -ne 0 ]; then
    FAIL=$((FAIL + 1))
fi
say

say "===== 5a. HEAD clean tree vendor checksum ====="
HEAD_TREE="$TMP_ROOT/head-tree"
if export_ref_tree HEAD "$HEAD_TREE"; then
    check_vendor_tree_dir "$HEAD_TREE" "HEAD clean tree"
    rc=$?
    if [ "$rc" -ne 0 ]; then
        FAIL=$((FAIL + 1))
    fi
else
    fail "无法从 HEAD 导出 clean tree"
fi
say

say "===== 5b. HEAD clean tree Cargo.lock/vendor exact match ====="
if [ -d "$HEAD_TREE" ]; then
    check_vendor_lock_tree "$HEAD_TREE" "HEAD clean tree"
    rc=$?
    if [ "$rc" -ne 0 ]; then
        FAIL=$((FAIL + 1))
    fi
fi
say

say "===== 5c. HEAD clean tree cargo offline metadata ====="
if [ -d "$HEAD_TREE" ]; then
    check_cargo_offline_metadata_tree "$HEAD_TREE" "HEAD clean tree"
    rc=$?
    if [ "$rc" -ne 0 ]; then
        FAIL=$((FAIL + 1))
    fi
fi
say

say "===== 6. platform/upstream tree check ====="
PLATFORM_REMOTE="${PLATFORM_REMOTE:-}"
PLATFORM_BRANCH="${PLATFORM_BRANCH:-}"
PLATFORM_REF=""

if [ -n "$PLATFORM_REMOTE" ] || [ -n "$PLATFORM_BRANCH" ]; then
    if [ -z "$PLATFORM_REMOTE" ] || [ -z "$PLATFORM_BRANCH" ]; then
        fail "指定平台远端时必须同时设置 PLATFORM_REMOTE 和 PLATFORM_BRANCH"
    else
        say "platform remote: $PLATFORM_REMOTE"
        say "platform branch: $PLATFORM_BRANCH"
        if git fetch --quiet "$PLATFORM_REMOTE"; then
            PLATFORM_REF="$PLATFORM_REMOTE/$PLATFORM_BRANCH"
        else
            fail "git fetch $PLATFORM_REMOTE 失败"
        fi
    fi
else
    UPSTREAM="$(git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null || true)"
    if [ -z "$UPSTREAM" ]; then
        warn "当前分支没有 upstream，无法确认平台拉取的远端提交。可用 PLATFORM_REMOTE=gitlab PLATFORM_BRANCH=dev 指定。"
    else
        PLATFORM_REF="$UPSTREAM"
        REMOTE_NAME="${UPSTREAM%%/*}"
        say "upstream: $UPSTREAM"
        git fetch --quiet "$REMOTE_NAME" || warn "git fetch $REMOTE_NAME 失败，后续 upstream 检查可能不准"
    fi
fi

if [ -n "$PLATFORM_REF" ]; then
    if git rev-parse --verify --quiet "$PLATFORM_REF^{commit}" >/dev/null; then
        LOCAL_HEAD="$(git rev-parse HEAD)"
        REMOTE_HEAD="$(git rev-parse "$PLATFORM_REF")"

        say "local HEAD:    $LOCAL_HEAD"
        say "platform ref:  $REMOTE_HEAD"

        if [ "$LOCAL_HEAD" != "$REMOTE_HEAD" ]; then
            fail "本地 HEAD 与 $PLATFORM_REF 不一致。平台通常拉远端分支，可能拿不到本地提交。"
            say "建议先 push 到平台使用的 remote/branch，再重新检查。"
        else
            ok "本地 HEAD 与 $PLATFORM_REF 一致"
        fi

        NEEDLE="src/vendor/indoc/tests/ui/printdoc-no-named-arg.stderr"
        if git ls-tree -r "$PLATFORM_REF" -- "$NEEDLE" | grep -q "$NEEDLE"; then
            ok "$PLATFORM_REF 包含平台曾报错缺失的 indoc 文件"
        else
            warn "$PLATFORM_REF 不包含 $NEEDLE；如果 Cargo.lock/vendor 仍引用它，平台会失败"
        fi

        REF_TREE="$TMP_ROOT/platform-tree"
        if export_ref_tree "$PLATFORM_REF" "$REF_TREE"; then
            check_vendor_tree_dir "$REF_TREE" "$PLATFORM_REF clean tree"
            rc=$?
            if [ "$rc" -ne 0 ]; then
                FAIL=$((FAIL + 1))
            fi

            check_vendor_lock_tree "$REF_TREE" "$PLATFORM_REF clean tree"
            rc=$?
            if [ "$rc" -ne 0 ]; then
                FAIL=$((FAIL + 1))
            fi

            check_cargo_offline_metadata_tree "$REF_TREE" "$PLATFORM_REF clean tree"
            rc=$?
            if [ "$rc" -ne 0 ]; then
                FAIL=$((FAIL + 1))
            fi
        else
            fail "无法从 $PLATFORM_REF 导出 clean tree"
        fi
    else
        fail "找不到平台 ref：$PLATFORM_REF"
    fi
fi
say

say "===== 7. ltp-safe duplicate check ====="
check_ltp_safe_dups_file "src/init/ltp-cases/ltp-safe.txt" "worktree"

HEAD_SAFE="$TMP_ROOT/head-ltp-safe.txt"
if git show HEAD:src/init/ltp-cases/ltp-safe.txt > "$HEAD_SAFE" 2>/dev/null; then
    check_ltp_safe_dups_file "$HEAD_SAFE" "HEAD"
else
    warn "HEAD 中不存在 src/init/ltp-cases/ltp-safe.txt"
fi

if [ -n "$PLATFORM_REF" ] && git rev-parse --verify --quiet "$PLATFORM_REF^{commit}" >/dev/null; then
    check_ltp_safe_dups_ref "$PLATFORM_REF" "$PLATFORM_REF"
fi
say

say "===== 8. hidden path risk ====="
hidden_count="$(git ls-files | grep -E '(^|/)\.[^/]+' | grep -vE '(^|/)\.gitignore$' | wc -l | tr -d ' ')"
if [ "$hidden_count" != "0" ]; then
    warn "仓库中存在隐藏路径文件。平台规则可能过滤隐藏文件/目录，若构建依赖这些文件会有风险。"
    git ls-files | grep -E '(^|/)\.[^/]+' | grep -vE '(^|/)\.gitignore$' | sed -n '1,60p'
    if [ "$hidden_count" -gt 60 ]; then
        say "... 仅显示前 60 个，共 $hidden_count 个"
    fi
else
    ok "未发现除 .gitignore 外的隐藏路径文件"
fi
say

say "===== result ====="
if [ "$FAIL" -ne 0 ]; then
    say "检查失败：$FAIL 个致命问题，$WARN 个警告。"
    exit 1
fi

if [ "$WARN" -ne 0 ]; then
    say "检查通过但有警告：$WARN 个。提交前建议确认。"
    exit 0
fi

say "检查通过，无明显提交风险。"
