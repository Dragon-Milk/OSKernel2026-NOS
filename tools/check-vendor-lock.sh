#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
src_dir="${1:-"$repo_root/src"}"
lock_file="$src_dir/Cargo.lock"
vendor_dir="$src_dir/vendor"

if [ ! -f "$lock_file" ]; then
    echo "missing Cargo.lock: $lock_file" >&2
    exit 1
fi

if [ ! -d "$vendor_dir" ]; then
    echo "missing vendor directory: $vendor_dir" >&2
    exit 1
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

lock_crates="$tmpdir/lock-crates.tsv"
vendor_crates="$tmpdir/vendor-crates.tsv"
missing_crates="$tmpdir/missing-crates.tsv"

awk '
function flush_package() {
    if (name != "" && version != "" && source ~ /^registry\+/) {
        print name "\t" version
    }
}

$0 == "[[package]]" {
    flush_package()
    name = ""
    version = ""
    source = ""
    next
}

$1 == "name" && $2 == "=" {
    name = $3
    gsub(/"/, "", name)
    next
}

$1 == "version" && $2 == "=" {
    version = $3
    gsub(/"/, "", version)
    next
}

$1 == "source" && $2 == "=" {
    source = $3
    gsub(/"/, "", source)
    next
}

END {
    flush_package()
}
' "$lock_file" | sort -u > "$lock_crates"

find "$vendor_dir" -mindepth 2 -maxdepth 2 -name Cargo.toml -print0 |
while IFS= read -r -d '' cargo_toml; do
    awk '
    $0 == "[package]" {
        in_package = 1
        next
    }

    /^\[/ && in_package {
        exit
    }

    in_package && $1 == "name" && $2 == "=" {
        name = $3
        gsub(/"/, "", name)
        next
    }

    in_package && $1 == "version" && $2 == "=" {
        version = $3
        gsub(/"/, "", version)
        next
    }

    END {
        if (name != "" && version != "") {
            print name "\t" version
        }
    }
    ' "$cargo_toml"
done | sort -u > "$vendor_crates"

comm -23 "$lock_crates" "$vendor_crates" > "$missing_crates"

if [ -s "$missing_crates" ]; then
    echo "vendor is missing registry crates required by Cargo.lock:" >&2
    while IFS="$(printf '\t')" read -r name version; do
        printf '  - %s %s\n' "$name" "$version" >&2
    done < "$missing_crates"
    exit 1
fi

echo "vendor-lock check passed: every registry crate in Cargo.lock has an exact vendor match."
