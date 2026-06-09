#!/usr/bin/env python3
from collections import Counter
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
PLAN_DIR = ROOT / "temp" / "ltp-plan"
BATCH_DIR = ROOT / "src" / "init" / "ltp-cases"
BATCH_SIZE = 30

CATEGORIES = {
    "process": PLAN_DIR / "phase1-process.txt",
    "fs": PLAN_DIR / "phase1-fs.txt",
    "mm-ipc": PLAN_DIR / "phase1-mm-ipc.txt",
    "common-easy": PLAN_DIR / "phase1-common-easy.txt",
}


def read_names(path: Path) -> list[str]:
    names: list[str] = []
    with path.open() as f:
        for raw in f:
            name = raw.split("#", 1)[0].strip()
            if name:
                names.append(name)
    return names


def fail(errors: list[str], message: str) -> None:
    errors.append(message)
    print(f"ERROR: {message}", file=sys.stderr)


def check_category(category: str, source_path: Path, errors: list[str]) -> tuple[int, int]:
    source = read_names(source_path)
    batch_paths = sorted(BATCH_DIR.glob(f"{category}-*.txt"))

    if not batch_paths:
        fail(errors, f"{category}: no batch files found")
        return 0, 0

    merged: list[str] = []
    for batch_path in batch_paths:
        names = read_names(batch_path)
        if len(names) > BATCH_SIZE:
            fail(errors, f"{batch_path}: has {len(names)} names, max is {BATCH_SIZE}")
        merged.extend(names)

    counts = Counter(merged)
    duplicates = sorted(name for name, count in counts.items() if count > 1)
    missing = [name for name in source if name not in counts]
    extras = [name for name in merged if name not in set(source)]

    if duplicates:
        fail(errors, f"{category}: duplicate names: {', '.join(duplicates)}")
    if missing:
        fail(errors, f"{category}: missing names: {', '.join(missing)}")
    if extras:
        fail(errors, f"{category}: extra names: {', '.join(extras)}")
    if merged != source:
        fail(errors, f"{category}: batch order does not match {source_path}")

    return len(source), len(batch_paths)


def main() -> int:
    errors: list[str] = []
    total = 0
    total_batches = 0

    for category, source_path in CATEGORIES.items():
        source_count, batch_count = check_category(category, source_path, errors)
        total += source_count
        total_batches += batch_count
        print(f"{category}: {source_count} cases, {batch_count} batches")

    print(f"total: {total} cases, {total_batches} batches")

    if errors:
        return 1

    print("LTP batch check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
