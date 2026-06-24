#!/usr/bin/env bash
set -euo pipefail

FILE="${1:-src/init/ltp-cases/ltp-safe.txt}"
SORT_CASES="${SORT_CASES:-1}"     # 1=去重后排序；0=只去重，保留原顺序
KEEP_BACKUP="${KEEP_BACKUP:-0}"   # 1=成功后也保留备份；0=成功后删除备份

if [ ! -f "$FILE" ]; then
  echo "ERROR: file not found: $FILE" >&2
  exit 1
fi

TS="$(date +%Y%m%d-%H%M%S)"
BAK="${FILE}.bak-${TS}"
TMPDIR="$(mktemp -d)"
HEADER="$TMPDIR/header"
BODY="$TMPDIR/body"
SORTED_BODY="$TMPDIR/sorted_body"
DUPS="$TMPDIR/dups"
OUT="$TMPDIR/out"

cleanup() {
  status=$?

  rm -rf "$TMPDIR"

  if [ "$status" -eq 0 ] && [ "$KEEP_BACKUP" != "1" ]; then
    rm -f "$BAK"
  fi

  exit "$status"
}
trap cleanup EXIT INT TERM

cp "$FILE" "$BAK"

before_total="$(awk 'NF && $1 !~ /^#/ { c++ } END { print c+0 }' "$FILE")"
before_unique="$(awk 'NF && $1 !~ /^#/ { seen[$1]=1 } END { print length(seen) }' "$FILE")"

awk -v header="$HEADER" -v body="$BODY" -v dups="$DUPS" '
{
  line = $0
  sub(/\r$/, "", line)

  # 保留空行和纯注释行；排序后统一放到文件开头
  if (line ~ /^[[:space:]]*$/ || line ~ /^[[:space:]]*#/) {
    print line > header
    next
  }

  # 以第一个字段作为 case 名；允许后面带注释
  name = line
  sub(/^[[:space:]]*/, "", name)
  sub(/[[:space:]].*$/, "", name)

  if (name == "") {
    print line > header
    next
  }

  if (!seen[name]++) {
    printf "%s\t%s\n", name, line > body
  } else {
    dup_count[name]++
  }
}
END {
  for (name in dup_count) {
    printf "%s %d\n", name, dup_count[name] > dups
  }
}
' "$FILE"

touch "$HEADER" "$BODY" "$DUPS"

if [ "$SORT_CASES" = "1" ]; then
  LC_ALL=C sort -k1,1 "$BODY" | cut -f2- > "$SORTED_BODY"
else
  cut -f2- "$BODY" > "$SORTED_BODY"
fi

{
  cat "$HEADER"
  if [ -s "$HEADER" ] && [ -s "$SORTED_BODY" ]; then
    echo
  fi
  cat "$SORTED_BODY"
} > "$OUT"

mv "$OUT" "$FILE"

after_total="$(awk 'NF && $1 !~ /^#/ { c++ } END { print c+0 }' "$FILE")"
after_unique="$(awk 'NF && $1 !~ /^#/ { seen[$1]=1 } END { print length(seen) }' "$FILE")"

echo "dedupe/sort done: $FILE"
echo "sort: $SORT_CASES"
echo "backup: $BAK"
if [ "$KEEP_BACKUP" = "1" ]; then
  echo "backup kept"
else
  echo "backup removed on success"
fi
echo "before: total=$before_total unique=$before_unique duplicate=$((before_total - before_unique))"
echo "after : total=$after_total unique=$after_unique duplicate=$((after_total - after_unique))"

if [ -s "$DUPS" ]; then
  echo
  echo "removed duplicate cases:"
  LC_ALL=C sort "$DUPS"
else
  echo
  echo "no duplicate cases found"
fi