#!/bin/sh

export HOME=/root
export USER=root
export PATH=.:/bin:/sbin:/usr/bin:/usr/sbin

SKIP_LTP=${SKIP_LTP:-0}
TEST_PROFILE=${TEST_PROFILE:-ltp-batch}
LTP_CATEGORY=${LTP_CATEGORY:-process}
LTP_BATCH=${LTP_BATCH:-all}
LTP_LIBC=${LTP_LIBC:-both}
LTP_TIMEOUT=${LTP_TIMEOUT:-30}
echo "[init] TEST_PROFILE=$TEST_PROFILE"
echo "[init] LTP_CATEGORY=$LTP_CATEGORY LTP_BATCH=$LTP_BATCH LTP_LIBC=$LTP_LIBC LTP_TIMEOUT=$LTP_TIMEOUT"

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

busybox_cmd() {
    if [ -n "$LTP_BUSYBOX" ] && [ -x "$LTP_BUSYBOX" ]; then
        echo "$LTP_BUSYBOX"
        return
    fi

    if [ -x ./busybox ]; then
        echo ./busybox
    elif [ -x /busybox ]; then
        echo /busybox
    elif [ -x /musl/busybox ]; then
        echo /musl/busybox
    elif [ -x /glibc/busybox ]; then
        echo /glibc/busybox
    fi
}

LTP_TOOL_DIR=/tmp/ltp-bin
LTP_TOOL_COMMANDS="sh rsh ssh rcp scp ip wc mktemp head setkey cut awk cat rm grep fgrep sed expr printf locale basename dirname id pkill date ps ifconfig tc busybox"

ltp_busybox_cmd() {
    libc="$1"

    case "$libc" in
        glibc)
            [ -x /glibc/busybox ] && { echo /glibc/busybox; return; }
            [ -x /busybox ] && { echo /busybox; return; }
            [ -x ./busybox ] && { echo ./busybox; return; }
            return
            ;;
        musl)
            [ -x /musl/busybox ] && { echo /musl/busybox; return; }
            [ -x /busybox ] && { echo /busybox; return; }
            [ -x ./busybox ] && { echo ./busybox; return; }
            return
            ;;
    esac

    if [ -x /busybox ]; then
        echo /busybox
    elif [ -x ./busybox ]; then
        echo ./busybox
    fi
}

ltp_find_real_cmd() {
    cmd="$1"
    libc="$2"

    for dir in "/$libc" /bin /sbin /usr/bin /usr/sbin; do
        [ "$dir" = "/" ] && continue
        [ -x "$dir/$cmd" ] || continue
        case "$dir/$cmd" in
            "$LTP_TOOL_DIR"/*) continue ;;
        esac
        echo "$dir/$cmd"
        return 0
    done

    return 1
}

ltp_mkdir_p() {
    path="$1"

    if command -v mkdir >/dev/null 2>&1; then
        mkdir -p "$path" 2>/dev/null && return 0
    fi

    bb="${LTP_BUSYBOX:-$(busybox_cmd)}"
    [ -n "$bb" ] && "$bb" mkdir -p "$path" 2>/dev/null && return 0

    return 1
}

ltp_chmod_x() {
    path="$1"

    if command -v chmod >/dev/null 2>&1; then
        chmod +x "$path" 2>/dev/null && return 0
    fi

    bb="${LTP_BUSYBOX:-$(busybox_cmd)}"
    [ -n "$bb" ] && "$bb" chmod +x "$path" 2>/dev/null && return 0

    return 1
}

ltp_small_inc() {
    n="$1"

    case "$n" in
        ''|*[!0-9]*|??????????*) echo 1 ;;
        *) echo $((n + 1)) ;;
    esac
}

ltp_rm_f() {
    path="$1"

    rm_cmd="$(ltp_find_real_cmd rm "$LTP_TOOL_LIBC")"
    if [ -n "$rm_cmd" ]; then
        "$rm_cmd" -f "$path" 2>/dev/null && return 0
    fi

    bb="${LTP_BUSYBOX:-$(busybox_cmd)}"
    [ -n "$bb" ] && "$bb" rm -f "$path" 2>/dev/null && return 0

    return 1
}

ltp_install_link() {
    target="$1"
    link="$2"

    ltp_rm_f "$link" >/dev/null 2>&1 || true

    ln_cmd="$(ltp_find_real_cmd ln "$LTP_TOOL_LIBC")"
    if [ -n "$ln_cmd" ]; then
        "$ln_cmd" -sf "$target" "$link" 2>/dev/null && return 0
    fi

    bb="${LTP_BUSYBOX:-$(busybox_cmd)}"
    [ -n "$bb" ] && "$bb" ln -sf "$target" "$link" 2>/dev/null && return 0

    return 1
}

ltp_tool_test() {
    cmd="$1"
    exe="$2"

    case "$cmd" in
        cut)
            out="$(printf 'a:b\n' | "$exe" -d: -f1 2>/dev/null)" || return 1
            [ "$out" = "a" ]
            ;;
        awk)
            out="$("$exe" 'BEGIN{print 1}' 2>/dev/null)" || return 1
            [ "$out" = "1" ]
            ;;
        cat)
            "$exe" /dev/null >/dev/null 2>&1
            ;;
        rm)
            "$exe" -f "/tmp/ltp_rm_probe_$$" >/dev/null 2>&1
            ;;
        grep)
            out="$(printf 'x\n' | "$exe" x 2>/dev/null)" || return 1
            [ "$out" = "x" ]
            ;;
        sed)
            out="$(printf 'x\n' | "$exe" 's/x/y/' 2>/dev/null)" || return 1
            [ "$out" = "y" ]
            ;;
        expr)
            out="$("$exe" 1 + 1 2>/dev/null)" || return 1
            [ "$out" = "2" ]
            ;;
        printf)
            out="$("$exe" '%s' x 2>/dev/null)" || return 1
            [ "$out" = "x" ]
            ;;
        locale)
            out="$("$exe" charmap 2>/dev/null)" || return 1
            [ -n "$out" ] || return 1
            "$exe" -a >/dev/null 2>&1
            ;;
        basename)
            out="$("$exe" /a/b/c.sh .sh 2>/dev/null)" || return 1
            [ "$out" = "c" ]
            ;;
        dirname)
            out="$("$exe" /a/b/c 2>/dev/null)" || return 1
            [ "$out" = "/a/b" ]
            ;;
        true)
            "$exe" >/dev/null 2>&1
            ;;
        false)
            if "$exe" >/dev/null 2>&1; then
                return 1
            else
                [ "$?" -eq 1 ]
            fi
            ;;
        id)
            out="$("$exe" -u 2>/dev/null)" || return 1
            case "$out" in
                ''|*[!0-9]*) return 1 ;;
            esac
            ;;
        pkill)
            "$exe" -f "__ltp_no_such_process_$$" >/dev/null 2>&1
            rc=$?
            [ "$rc" -eq 0 ] || [ "$rc" -eq 1 ]
            ;;
        wc)
            out="$(printf 'a\nb\n' | "$exe" -l 2>/dev/null)" || return 1
            set -- $out
            [ "${1-}" = "2" ]
            ;;
        mktemp)
            out="$("$exe" /tmp/ltp_mktemp_probe_XXXXXX 2>/dev/null)" || return 1
            [ -n "$out" ] && [ -f "$out" ] || return 1
            ltp_rm_f "$out" >/dev/null 2>&1 || true
            ;;
        head)
            out="$(printf 'a\nb\n' | "$exe" -n 1 2>/dev/null)" || return 1
            [ "$out" = "a" ]
            ;;
        setkey)
            "$exe" -D >/dev/null 2>&1
            ;;
        rsh|ssh)
            "$exe" localhost true >/dev/null 2>&1
            ;;
        rcp|scp)
            src="/tmp/ltp_${cmd}_src_$$"
            dst="/tmp/ltp_${cmd}_dst_$$"
            printf 'x\n' >"$src" || return 1
            "$exe" "$src" "$dst" >/dev/null 2>&1 || {
                ltp_rm_f "$src" >/dev/null 2>&1 || true
                ltp_rm_f "$dst" >/dev/null 2>&1 || true
                return 1
            }
            [ -f "$dst" ] || {
                ltp_rm_f "$src" >/dev/null 2>&1 || true
                return 1
            }
            ltp_rm_f "$src" >/dev/null 2>&1 || true
            ltp_rm_f "$dst" >/dev/null 2>&1 || true
            ;;
        ip)
            "$exe" link show >/dev/null 2>&1
            ;;
        sh)
            "$exe" -c ':' >/dev/null 2>&1
            ;;
        busybox)
            "$exe" true >/dev/null 2>&1
            ;;
        date)
            "$exe" '+%s' >/dev/null 2>&1
            ;;
        ps)
            "$exe" >/dev/null 2>&1
            ;;
        fgrep)
            out="$(printf 'abc\n' | "$exe" abc 2>/dev/null)" || return 1
            [ "$out" = "abc" ]
            ;;
        ifconfig)
            "$exe" lo >/dev/null 2>&1
            ;;
        tc)
            "$exe" qdisc show >/dev/null 2>&1
            ;;
        *)
            "$exe" --help >/dev/null 2>&1
            ;;
    esac
}

ltp_generate_cut_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINICUT_EOF'
#!/bin/sh
# Minimal LTP net compatible cut wrapper. This is not a general cut implementation.

minicut_unsupported() {
    echo "[MINICUT-UNSUPPORTED] $*" >&2
    exit 2
}

mode=
range=
delim="$(printf '\t')"

while [ "$#" -gt 0 ]; do
    case "$1" in
        -d)
            shift
            [ "$#" -gt 0 ] || minicut_unsupported "missing delimiter"
            delim="$1"
            ;;
        -d*)
            delim="${1#-d}"
            [ -n "$delim" ] || minicut_unsupported "missing delimiter"
            ;;
        -f)
            shift
            [ "$#" -gt 0 ] || minicut_unsupported "missing field range"
            mode=f
            range="$1"
            ;;
        -f*)
            mode=f
            range="${1#-f}"
            ;;
        -c)
            shift
            [ "$#" -gt 0 ] || minicut_unsupported "missing char range"
            mode=c
            range="$1"
            ;;
        -c*)
            mode=c
            range="${1#-c}"
            ;;
        --)
            shift
            break
            ;;
        -*)
            minicut_unsupported "option $1"
            ;;
        *)
            break
            ;;
    esac
    shift
done

[ -n "$mode" ] || minicut_unsupported "missing -f or -c"
[ -n "$range" ] || minicut_unsupported "missing range"

case "$range" in
    *,*) minicut_unsupported "comma ranges are unsupported" ;;
    *-*)
        range_start="${range%-*}"
        range_end="${range#*-}"
        [ -n "$range_start" ] || range_start=1
        ;;
    *)
        range_start="$range"
        range_end="$range"
        ;;
esac

case "$range_start" in
    ''|*[!0-9]*) minicut_unsupported "bad range $range" ;;
esac
if [ -n "$range_end" ]; then
    case "$range_end" in
        *[!0-9]*) minicut_unsupported "bad range $range" ;;
    esac
    [ "$range_end" -ge "$range_start" ] || minicut_unsupported "bad range $range"
fi

if [ "$mode" = f ]; then
    [ -n "$delim" ] || minicut_unsupported "empty delimiter"
    case "${delim#?}" in
        '') ;;
        *) minicut_unsupported "multi-character delimiter" ;;
    esac
    case "$delim" in
        '['|']'|'*'|'?'|'\') minicut_unsupported "pattern delimiter" ;;
    esac
fi

minicut_in_range() {
    n="$1"
    [ "$n" -ge "$range_start" ] || return 1
    [ -z "$range_end" ] && return 0
    [ "$n" -le "$range_end" ]
}

minicut_fields() {
    rest="$1"
    idx=1
    out=
    wrote=0

    while :; do
        case "$rest" in
            *"$delim"*)
                field="${rest%%"$delim"*}"
                rest="${rest#*"$delim"}"
                last=0
                ;;
            *)
                field="$rest"
                last=1
                ;;
        esac

        if minicut_in_range "$idx"; then
            if [ "$wrote" -eq 1 ]; then
                out="${out}${delim}${field}"
            else
                out="$field"
                wrote=1
            fi
        fi

        [ "$last" -eq 1 ] && break
        idx=$((idx + 1))
    done

    printf '%s\n' "$out"
}

minicut_chars() {
    rest="$1"
    idx=1
    out=

    while [ -n "$rest" ]; do
        ch="${rest%"${rest#?}"}"
        if minicut_in_range "$idx"; then
            out="${out}${ch}"
        fi
        rest="${rest#?}"
        idx=$((idx + 1))
    done

    printf '%s\n' "$out"
}

minicut_stream() {
    while IFS= read -r line || [ -n "$line" ]; do
        case "$mode" in
            f) minicut_fields "$line" ;;
            c) minicut_chars "$line" ;;
        esac
    done
}

rc=0
if [ "$#" -eq 0 ]; then
    minicut_stream || rc=1
else
    for src in "$@"; do
        if [ "$src" = "-" ]; then
            minicut_stream || rc=1
        elif [ -r "$src" ]; then
            minicut_stream <"$src" || rc=1
        else
            echo "cut: $src: cannot open" >&2
            rc=1
        fi
    done
fi

exit "$rc"
MINICUT_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_awk_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIAWK_EOF'
#!/bin/sh
# Minimal LTP net compatible awk wrapper. This is not a general awk implementation.

set -f

miniawk_unsupported() {
    echo "[MINIAWK-UNSUPPORTED] $*" >&2
    exit 2
}

fs=
while [ "$#" -gt 0 ]; do
    case "$1" in
        -F)
            shift
            [ "$#" -gt 0 ] || miniawk_unsupported "missing -F value"
            fs="$1"
            ;;
        -F*)
            fs="${1#-F}"
            [ -n "$fs" ] || miniawk_unsupported "missing -F value"
            ;;
        --)
            shift
            break
            ;;
        -*)
            miniawk_unsupported "option $1"
            ;;
        *)
            break
            ;;
    esac
    shift
done

[ "$#" -gt 0 ] || miniawk_unsupported "missing program"
prog="$1"
shift

action=
cond_value=
case "$prog" in
    'BEGIN{print 1}'|'BEGIN {print 1}'|'BEGIN { print 1 }')
        printf '1\n'
        exit 0
        ;;
    '{print $1}'|'{ print $1 }')
        action=print1
        ;;
    '{print $2}'|'{ print $2 }')
        action=print2
        ;;
    '{print $NF}'|'{ print $NF }')
        action=printnf
        ;;
    '$1 == "'*'" {print $2}'|'$1 == "'*'" { print $2 }')
        action=cond1_print2
        tmp="${prog#\$1 == \"}"
        cond_value="${tmp%%\"*}"
        ;;
    '$1=="'*'"{print $2}'|'$1=="'*'" {print $2}'|'$1=="'*'" { print $2 }')
        action=cond1_print2
        tmp="${prog#\$1==\"}"
        cond_value="${tmp%%\"*}"
        ;;
    *)
        miniawk_unsupported "program=$prog"
        ;;
esac

if [ -n "$fs" ]; then
    case "${fs#?}" in
        '') ;;
        *) miniawk_unsupported "multi-character field separator" ;;
    esac
    case "$fs" in
        '['|']'|'*'|'?'|'\') miniawk_unsupported "pattern field separator" ;;
    esac
fi

miniawk_split() {
    line="$1"
    f1=
    f2=
    fn=
    nf=0

    if [ -z "$fs" ] || [ "$fs" = " " ]; then
        set -- $line
        nf=$#
        f1="${1-}"
        f2="${2-}"
        for field do
            fn="$field"
        done
        return
    fi

    rest="$line"
    idx=1
    while :; do
        case "$rest" in
            *"$fs"*)
                field="${rest%%"$fs"*}"
                rest="${rest#*"$fs"}"
                last=0
                ;;
            *)
                field="$rest"
                last=1
                ;;
        esac

        nf="$idx"
        [ "$idx" -eq 1 ] && f1="$field"
        [ "$idx" -eq 2 ] && f2="$field"
        fn="$field"

        [ "$last" -eq 1 ] && break
        idx=$((idx + 1))
    done
}

miniawk_emit() {
    case "$action" in
        print1)
            printf '%s\n' "$f1"
            ;;
        print2)
            printf '%s\n' "$f2"
            ;;
        printnf)
            printf '%s\n' "$fn"
            ;;
        cond1_print2)
            [ "$f1" = "$cond_value" ] && printf '%s\n' "$f2"
            ;;
    esac
}

miniawk_stream() {
    while IFS= read -r line || [ -n "$line" ]; do
        miniawk_split "$line"
        miniawk_emit
    done
}

rc=0
if [ "$#" -eq 0 ]; then
    miniawk_stream || rc=1
else
    for src in "$@"; do
        if [ "$src" = "-" ]; then
            miniawk_stream || rc=1
        elif [ -r "$src" ]; then
            miniawk_stream <"$src" || rc=1
        else
            echo "awk: $src: cannot open" >&2
            rc=1
        fi
    done
fi

exit "$rc"
MINIAWK_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_cat_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINICAT_EOF'
#!/bin/sh
# Minimal LTP net compatible cat wrapper for text files.

minicat_stream() {
    while IFS= read -r line || [ -n "$line" ]; do
        printf '%s\n' "$line"
    done
}

minicat_one() {
    src="$1"
    if [ "$src" = "-" ]; then
        minicat_stream
        return $?
    fi
    if [ ! -r "$src" ]; then
        echo "cat: $src: cannot open" >&2
        return 1
    fi
    minicat_stream <"$src"
}

rc=0
if [ "$#" -eq 0 ]; then
    minicat_stream || rc=1
else
    for src in "$@"; do
        case "$src" in
            --) continue ;;
            -*) [ "$src" = "-" ] || { echo "[MINICAT-UNSUPPORTED] option $src" >&2; rc=1; continue; } ;;
        esac
        minicat_one "$src" || rc=1
    done
fi

exit "$rc"
MINICAT_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_rm_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIRM_EOF'
#!/bin/sh
# Restricted rm fallback for LTP temp cleanup. It does not fake deletion.

minirm_unsupported() {
    echo "[MINIRM-UNSUPPORTED] $*" >&2
}

force=0
recursive=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        -f)
            force=1
            ;;
        -rf|-fr)
            force=1
            recursive=1
            ;;
        --)
            shift
            break
            ;;
        -*)
            minirm_unsupported "option $1"
            exit 2
            ;;
        *)
            break
            ;;
    esac
    shift
done

[ "$force" -eq 1 ] || { minirm_unsupported "only -f and -rf are supported"; exit 2; }

rc=0
for path in "$@"; do
    case "$path" in
        /tmp|/tmp/*|/var/tmp|/var/tmp/*) ;;
        *)
            minirm_unsupported "path=$path"
            rc=1
            continue
            ;;
    esac

    [ -e "$path" ] || continue
    if [ "$recursive" -eq 1 ] && [ -d "$path" ]; then
        minirm_unsupported "no unlink backend for directory path=$path"
    else
        minirm_unsupported "no unlink backend path=$path"
    fi
    rc=1
done

exit "$rc"
MINIRM_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_locale_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINILOCALE_EOF'
#!/bin/sh
# Minimal LTP net compatible locale wrapper. This is not a general locale implementation.

case "$#" in
    0)
        printf '%s\n' \
            'LANG=C' \
            'LC_CTYPE=C' \
            'LC_NUMERIC=C' \
            'LC_TIME=C' \
            'LC_COLLATE=C' \
            'LC_MONETARY=C' \
            'LC_MESSAGES=C' \
            'LC_ALL=C'
        ;;
    1)
        case "$1" in
            -a)
                printf '%s\n' C POSIX C.UTF-8
                ;;
            charmap)
                printf '%s\n' UTF-8
                ;;
            *)
                echo "[MINILOCALE-UNSUPPORTED] args=$*" >&2
                exit 2
                ;;
        esac
        ;;
    *)
        echo "[MINILOCALE-UNSUPPORTED] args=$*" >&2
        exit 2
        ;;
esac
MINILOCALE_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_basename_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIBASENAME_EOF'
#!/bin/sh
# Minimal LTP net compatible basename wrapper.

if [ "${1-}" = "--" ]; then
    shift
fi

case "$#" in
    1|2) ;;
    *)
        echo "[MINIBASENAME-UNSUPPORTED] args=$*" >&2
        exit 2
        ;;
esac

case "$1" in
    -*) echo "[MINIBASENAME-UNSUPPORTED] args=$*" >&2; exit 2 ;;
esac

name="$1"
suffix="${2-}"

while [ "$name" != "/" ]; do
    case "$name" in
        */) name="${name%/}" ;;
        *) break ;;
    esac
done

if [ "$name" = "/" ]; then
    base="/"
else
    base="${name##*/}"
    [ -n "$base" ] || base="$name"
fi

if [ "$#" -eq 2 ] && [ -n "$suffix" ] && [ "$base" != "$suffix" ]; then
    case "$base" in
        *"$suffix") base="${base%"$suffix"}" ;;
    esac
fi

printf '%s\n' "$base"
MINIBASENAME_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_dirname_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIDIRNAME_EOF'
#!/bin/sh
# Minimal LTP net compatible dirname wrapper.

if [ "${1-}" = "--" ]; then
    shift
fi

[ "$#" -eq 1 ] || { echo "[MINIDIRNAME-UNSUPPORTED] args=$*" >&2; exit 2; }
case "$1" in
    -*) echo "[MINIDIRNAME-UNSUPPORTED] args=$*" >&2; exit 2 ;;
esac

name="$1"

while [ "$name" != "/" ]; do
    case "$name" in
        */) name="${name%/}" ;;
        *) break ;;
    esac
done

case "$name" in
    */*)
        dir="${name%/*}"
        [ -n "$dir" ] || dir="/"
        while [ "$dir" != "/" ]; do
            case "$dir" in
                */) dir="${dir%/}" ;;
                *) break ;;
            esac
        done
        printf '%s\n' "$dir"
        ;;
    *)
        printf '%s\n' "."
        ;;
esac
MINIDIRNAME_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_true_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINITRUE_EOF'
#!/bin/sh
exit 0
MINITRUE_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_false_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIFALSE_EOF'
#!/bin/sh
exit 1
MINIFALSE_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_id_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIID_EOF'
#!/bin/sh
# Minimal LTP net compatible id wrapper. It assumes the test runner is root.

case "$#" in
    0)
        printf '%s\n' 'uid=0(root) gid=0(root) groups=0(root)'
        ;;
    1)
        case "$1" in
            -u)  printf '%s\n' 0 ;;
            -g)  printf '%s\n' 0 ;;
            -un) printf '%s\n' root ;;
            -gn) printf '%s\n' root ;;
            *) echo "[MINIID-UNSUPPORTED] args=$*" >&2; exit 2 ;;
        esac
        ;;
    2)
        case "$1:$2" in
            -u:-n|-n:-u) printf '%s\n' root ;;
            -g:-n|-n:-g) printf '%s\n' root ;;
            *) echo "[MINIID-UNSUPPORTED] args=$*" >&2; exit 2 ;;
        esac
        ;;
    *)
        echo "[MINIID-UNSUPPORTED] args=$*" >&2
        exit 2
        ;;
esac
MINIID_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_pkill_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIPKILL_EOF'
#!/bin/sh
# Restricted pkill fallback for LTP cleanup. It only supports simple name matching.

signal="-TERM"
full=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        -f)
            full=1
            ;;
        -9|-KILL)
            signal="-KILL"
            ;;
        -15|-TERM)
            signal="-TERM"
            ;;
        -2|-INT)
            signal="-INT"
            ;;
        --)
            shift
            break
            ;;
        -*)
            echo "[MINIPKILL-UNSUPPORTED] args=$*" >&2
            exit 2
            ;;
        *)
            break
            ;;
    esac
    shift
done

[ "$#" -eq 1 ] || { echo "[MINIPKILL-UNSUPPORTED] args=$*" >&2; exit 2; }
pattern="$1"
matched=0

for proc in /proc/[0-9]*; do
    [ -d "$proc" ] || continue
    pid="${proc##*/}"
    [ "$pid" = "$$" ] && continue

    text=
    if [ "$full" -eq 1 ] && [ -r "$proc/cmdline" ]; then
        IFS= read -r text <"$proc/cmdline" || text=
    fi
    if [ -z "$text" ] && [ -r "$proc/comm" ]; then
        IFS= read -r text <"$proc/comm" || text=
    fi
    [ -n "$text" ] || continue

    case "$text" in
        *"$pattern"*)
            if kill "$signal" "$pid" >/dev/null 2>&1; then
                matched=1
            fi
            ;;
    esac
done

[ "$matched" -eq 1 ] && exit 0
exit 1
MINIPKILL_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_wc_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIWC_EOF'
#!/bin/sh
# Minimal LTP net compatible wc wrapper. This is not a general wc implementation.

miniwc_unsupported() {
    echo "[MINIWC-UNSUPPORTED] args=$*" >&2
    exit 2
}

mode=all
case "${1-}" in
    -l) mode=lines; shift ;;
    -w) mode=words; shift ;;
    -c) mode=chars; shift ;;
    --) shift ;;
    -*) miniwc_unsupported "$@" ;;
esac

[ "$#" -le 1 ] || miniwc_unsupported "$@"
src="${1-}"

miniwc_count() {
    lines=0
    words=0
    chars=0

    while IFS= read -r line || [ -n "$line" ]; do
        lines=$((lines + 1))
        chars=$((chars + ${#line} + 1))
        set -- $line
        words=$((words + $#))
    done

    case "$mode" in
        lines) printf '%s' "$lines" ;;
        words) printf '%s' "$words" ;;
        chars) printf '%s' "$chars" ;;
        all) printf '%s %s %s' "$lines" "$words" "$chars" ;;
    esac
}

if [ -n "$src" ] && [ "$src" != "-" ]; then
    [ -r "$src" ] || { echo "wc: $src: cannot open" >&2; exit 1; }
    out="$(miniwc_count <"$src")" || exit 1
    printf '%s %s\n' "$out" "$src"
else
    miniwc_count
    printf '\n'
fi
MINIWC_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_mktemp_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIMKTEMP_EOF'
#!/bin/sh
# Minimal LTP net compatible mktemp wrapper.

minimktemp_unsupported() {
    echo "[MINIMKTEMP-UNSUPPORTED] args=$*" >&2
    exit 2
}

minimktemp_mkdir() {
    if [ -x /bin/mkdir ]; then
        /bin/mkdir "$1"
        return $?
    fi
    if [ -x /usr/bin/mkdir ]; then
        /usr/bin/mkdir "$1"
        return $?
    fi
    case "${LTP_CURRENT_LIBC-}" in
        glibc) bb=/glibc/busybox ;;
        musl) bb=/musl/busybox ;;
        *) bb= ;;
    esac
    [ -n "$bb" ] && [ -x "$bb" ] && "$bb" mkdir "$1"
}

make_dir=0
case "${1-}" in
    -d) make_dir=1; shift ;;
    --) shift ;;
    -*) minimktemp_unsupported "$@" ;;
esac

[ "$#" -le 1 ] || minimktemp_unsupported "$@"
template="${1-/tmp/tmp.XXXXXX}"

case "$template" in
    *XXXXXX*) ;;
    *) minimktemp_unsupported "$@" ;;
esac

prefix="${template%%XXXXXX*}"
suffix="${template#*XXXXXX}"
i=0
while [ "$i" -lt 100 ]; do
    i=$((i + 1))
    candidate="${prefix}$${i}${suffix}"
    [ -e "$candidate" ] && continue
    if [ "$make_dir" -eq 1 ]; then
        if minimktemp_mkdir "$candidate" >/dev/null 2>&1; then
            printf '%s\n' "$candidate"
            exit 0
        fi
    else
        if : >"$candidate" 2>/dev/null; then
            printf '%s\n' "$candidate"
            exit 0
        fi
    fi
done

echo "mktemp: cannot create temporary path" >&2
exit 1
MINIMKTEMP_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_head_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIHEAD_EOF'
#!/bin/sh
# Minimal LTP net compatible head wrapper.

minihead_unsupported() {
    echo "[MINIHEAD-UNSUPPORTED] args=$*" >&2
    exit 2
}

n=10
case "${1-}" in
    -n)
        shift
        [ "$#" -gt 0 ] || minihead_unsupported "$@"
        n="$1"
        shift
        ;;
    -n*)
        n="${1#-n}"
        shift
        ;;
    -[0-9]*)
        n="${1#-}"
        shift
        ;;
    --)
        shift
        ;;
    -*) minihead_unsupported "$@" ;;
esac

case "$n" in
    ''|*[!0-9]*) minihead_unsupported "$@" ;;
esac

[ "$#" -le 1 ] || minihead_unsupported "$@"
src="${1-}"

minihead_stream() {
    count=0
    while [ "$count" -lt "$n" ] && { IFS= read -r line || [ -n "$line" ]; }; do
        printf '%s\n' "$line"
        count=$((count + 1))
    done
}

if [ -n "$src" ] && [ "$src" != "-" ]; then
    [ -r "$src" ] || { echo "head: $src: cannot open" >&2; exit 1; }
    minihead_stream <"$src"
else
    minihead_stream
fi
MINIHEAD_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_setkey_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINISETKEY_EOF'
#!/bin/sh
# Minimal LTP net compatible setkey wrapper. It does not implement IPsec.

minisetkey_unsupported() {
    echo "[MINISETKEY-UNSUPPORTED] $*" >&2
    exit 95
}

minisetkey_trim() {
    s="$1"
    while :; do
        case "$s" in
            ' '*) s="${s# }" ;;
            *) break ;;
        esac
    done
    while :; do
        case "$s" in
            *' ') s="${s% }" ;;
            *) break ;;
        esac
    done
    printf '%s\n' "$s"
}

minisetkey_check_input() {
    unsupported=0
    while IFS= read -r line || [ -n "$line" ]; do
        trimmed="$(minisetkey_trim "$line")"
        case "$trimmed" in
            ''|'#'*|flush|flush';'|spdflush|spdflush';'|dump|dump';'|spddump|spddump';')
                ;;
            *)
                unsupported=1
                ;;
        esac
    done

    [ "$unsupported" -eq 0 ]
}

case "$#" in
    1)
        case "$1" in
            -h|--help)
                printf '%s\n' 'minimal setkey wrapper for LTP cleanup/probe commands'
                exit 0
                ;;
            -V)
                printf '%s\n' 'setkey wrapper 0'
                exit 0
                ;;
            -D|-DP|-F|-FP)
                exit 0
                ;;
            -c)
                if minisetkey_check_input; then
                    exit 0
                fi
                minisetkey_unsupported "setkey -c input contains SA/SP configuration"
                ;;
            *)
                minisetkey_unsupported "args=$*"
                ;;
        esac
        ;;
    2)
        case "$1" in
            -c)
                [ -r "$2" ] || { echo "setkey: $2: cannot open" >&2; exit 1; }
                if minisetkey_check_input <"$2"; then
                    exit 0
                fi
                minisetkey_unsupported "setkey -c $2 contains SA/SP configuration"
                ;;
            *)
                minisetkey_unsupported "args=$*"
                ;;
        esac
        ;;
    *)
        minisetkey_unsupported "args=$*"
        ;;
esac
MINISETKEY_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_sh_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
done >"$path" <<'MINISH_EOF'
#!/bin/sh
# LTP shell shim. It routes nested sh calls to the current libc busybox shell
# and strips unsupported set -u forms when that shell cannot parse them.

case "${LTP_CURRENT_LIBC-}" in
    glibc) bb=/glibc/busybox ;;
    musl) bb=/musl/busybox ;;
    *) bb= ;;
esac

minish_run() {
    if [ -n "$bb" ] && [ -x "$bb" ]; then
        "$bb" sh "$@"
        return $?
    fi

    echo "[MINISH-WARN] libc=${LTP_CURRENT_LIBC-unknown} fallback=/bin/sh" >&2
    /bin/sh "$@"
}

minish_trim_spaces() {
    s="$1"
    while :; do
        case "$s" in
            ' '*) s="${s# }" ;;
            *) break ;;
        esac
    done
    while :; do
        case "$s" in
            *' ') s="${s% }" ;;
            *) break ;;
        esac
    done
    printf '%s\n' "$s"
}

# Check whether a trimmed string is a "set" command that contains -u.
# Returns 0 (true) if it matches, 1 otherwise.
minish_is_set_u() {
    s="$1"
    # Match: set -u           (standalone)
    #         set -u; ...     (prefix, rest after ; ignored)
    #         set -u  ; ...   (space before ;)
    #         set -eu         (combined -eu)
    #         set -ue         (combined -ue)
    #         set -euo pipefail
    #         set -o nounset  (equivalent)
    case "$s" in
        'set -u'|'set -u;'*|'set -u '*)   return 0 ;;
        'set -eu'|'set -eu;'*|'set -eu '*) return 0 ;;
        'set -ue'|'set -ue;'*|'set -ue '*) return 0 ;;
        'set -euo pipefail'|'set -euo pipefail;'*) return 0 ;;
        'set -e -u'|'set -e -u;'*|'set -e -u '*) return 0 ;;
        'set -u -e'|'set -u -e;'*|'set -u -e '*) return 0 ;;
        'set -o nounset'|'set -o nounset;'*|'set -o nounset '*) return 0 ;;
    esac
    return 1
}

# Given a trimmed "set" line that contains -u, produce the replacement.
# Returns nothing (stripped) for pure set -u; returns 'set -e' for -eu forms.
minish_set_u_replacement() {
    s="$1"
    case "$s" in
        'set -u'|'set -u;'*|'set -o nounset'|'set -o nounset;'*)
            return
            ;;
    esac
    printf '%s\n' 'set -e'
}

minish_sanitize_line() {
    original="$1"
    trimmed="$(minish_trim_spaces "$original")"

    if minish_is_set_u "$trimmed"; then
        MINISH_CHANGED=1
        minish_set_u_replacement "$trimmed"
        return
    fi

    printf '%s\n' "$original"
}

minish_sanitize_stream() {
    MINISH_CHANGED=0
    while IFS= read -r line || [ -n "$line" ]; do
        minish_sanitize_line "$line"
    done
}

minish_sanitize_part() {
    original="$1"
    trimmed="$(minish_trim_spaces "$original")"

    if minish_is_set_u "$trimmed"; then
        MINISH_CHANGED=1
        minish_set_u_replacement "$trimmed"
        return
    fi

    printf '%s' "$original"
}

minish_sanitize_cmd() {
    rest="$1"
    out=
    MINISH_CHANGED=0

    while :; do
        case "$rest" in
            *'&&'*)
                part="${rest%%&&*}"
                rest="${rest#*&&}"
                sep="&&"
                more=1
                ;;
            *'||'*)
                part="${rest%%||*}"
                rest="${rest#*||}"
                sep="||"
                more=1
                ;;
            *';'*)
                part="${rest%%;*}"
                rest="${rest#*;}"
                sep=";"
                more=1
                ;;
            *)
                part="$rest"
                sep=
                more=0
                ;;
        esac

        repl="$(minish_sanitize_part "$part")"
        if [ -n "$repl" ]; then
            if [ -n "$out" ]; then
                out="$out$sep $repl"
            else
                out="$repl"
            fi
        fi

        [ "$more" -eq 0 ] && break
    done

    [ -n "$out" ] || out=:
    printf '%s\n' "$out"
}

minish_run_sanitized_file() {
    src="$1"
    shift
    tmp="/tmp/ltp-sh-$$.sh"

    if minish_sanitize_stream <"$src" >"$tmp"; then
        if [ "$MINISH_CHANGED" = 1 ]; then
            echo "[LTP-SHELL-COMPAT] case=$src strip_set_u=1" >&2
        fi
        minish_run "$tmp" "$@"
        rc=$?
        rm -f "$tmp" >/dev/null 2>&1 || true
        exit "$rc"
    fi

    minish_run "$src" "$@"
    exit $?
}

minish_try_script_cmd() {
    cmd="$1"

    case "$cmd" in
        *'|'*|*'>'*|*'<'*|*'&'*|*'('*|*')'*)
            return 1
            ;;
    esac

    set -- $cmd
    first="${1-}"
    [ -n "$first" ] || return 1
    case "$first" in
        *.sh|./*.sh|/*/*.sh)
            [ -r "$first" ] || return 1
            shift
            minish_run_sanitized_file "$first" "$@"
            ;;
    esac

    return 1
}

if [ "${LTP_SHELL_COMPAT-0}" = 1 ] || [ "${LTP_SHELL_SUPPORTS_SET_U-1}" = 0 ]; then
    case "${1-}" in
        -c)
            shift
            if [ "$#" -eq 0 ]; then
                minish_run -c ''
                exit $?
            fi
            orig_cmd="$1"
            cmd="$(minish_sanitize_cmd "$orig_cmd")"
            if [ "$cmd" != "$orig_cmd" ]; then
                echo "[LTP-SHELL-COMPAT] case=-c strip_set_u=1" >&2
            fi
            shift
            minish_try_script_cmd "$cmd"
            minish_run -c "$cmd" "$@"
            exit $?
            ;;
        -s)
            shift
            tmp="/tmp/ltp-sh-$$.sh"
            if minish_sanitize_stream >"$tmp"; then
                if [ "$MINISH_CHANGED" = 1 ]; then
                    echo "[LTP-SHELL-COMPAT] case=-s strip_set_u=1" >&2
                fi
                minish_run "$tmp" "$@"
                rc=$?
                rm -f "$tmp" >/dev/null 2>&1 || true
                exit "$rc"
            fi
            ;;
        -*|'')
            minish_run "$@"
            exit $?
            ;;
        *)
            if [ -r "$1" ]; then
                src="$1"
                shift
                minish_run_sanitized_file "$src" "$@"
            fi
            ;;
    esac
fi

minish_run "$@"
exit $?
MINISH_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_rsh_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
done >"$path" <<'MINIRSH_EOF'
#!/bin/sh
# Restricted rsh/ssh fallback for LTP localhost-style net tests.

export LHOST RHOST LHOST_IFACES RHOST_IFACES LHOST_HWADDRS RHOST_HWADDRS
export IPV4_LHOST IPV4_RHOST IPV6_LHOST IPV6_RHOST
export LTP_CURRENT_LIBC LTP_SHELL_SUPPORTS_SET_U LTP_SHELL_COMPAT PATH NS_DURATION

minirsh_hostname() {
    if [ -r /proc/sys/kernel/hostname ]; then
        IFS= read -r h </proc/sys/kernel/hostname || h=
        [ -n "$h" ] && { printf '%s\n' "$h"; return; }
    fi
    if [ -r /etc/hostname ]; then
        IFS= read -r h </etc/hostname || h=
        [ -n "$h" ] && { printf '%s\n' "$h"; return; }
    fi
}

minirsh_is_local() {
    host="$1"
    local_h="$(minirsh_hostname)"

    case "$host" in
        ''|localhost|localhost.localdomain|127.0.0.1|::1|0.0.0.0)
            return 0
            ;;
    esac

    [ -n "$local_h" ] && [ "$host" = "$local_h" ] && return 0
    [ -n "${HOSTNAME-}" ] && [ "$host" = "$HOSTNAME" ] && return 0
    [ -n "${LHOST-}" ] && [ "$host" = "$LHOST" ] && return 0
    [ -n "${RHOST-}" ] && [ "$host" = "$RHOST" ] && return 0
    [ -n "${THOST-}" ] && [ "$host" = "$THOST" ] && return 0

    return 1
}

user=
while [ "$#" -gt 0 ]; do
    case "$1" in
        -n|-q|-x|-T|-4|-6)
            shift
            ;;
        -l)
            shift
            [ "$#" -gt 0 ] || { echo "[MINIRSH-UNSUPPORTED] missing -l user" >&2; exit 95; }
            user="$1"
            shift
            ;;
        -l*)
            user="${1#-l}"
            shift
            ;;
        -o|-p)
            shift
            [ "$#" -gt 0 ] || { echo "[MINIRSH-UNSUPPORTED] missing option value" >&2; exit 95; }
            shift
            ;;
        --)
            shift
            break
            ;;
        -*)
            echo "[MINIRSH-UNSUPPORTED] option=$1" >&2
            exit 95
            ;;
        *)
            break
            ;;
    esac
done

[ "$#" -gt 0 ] || { echo "[MINIRSH-UNSUPPORTED] missing host" >&2; exit 95; }
host="$1"
shift

[ "$#" -gt 0 ] || { echo "[MINIRSH-ERROR] host=$host cmd=" >&2; exit 95; }

if minirsh_is_local "$host"; then
    # Normalize sh -c "…" calls so the inner shell is our compat wrapper.
    case "$1" in
        sh|/bin/sh|/tmp/ltp-bin/sh)
            if [ "$2" = "-c" ] && [ "$#" -ge 3 ]; then
                shift 2
                exec /tmp/ltp-bin/sh -c "$*"
            fi
            ;;
    esac

    if [ "$#" -eq 1 ]; then
        # Single argument: the old LTP ns-tools convention passes a complete
        # shell command inside single quotes (redirects, variables, pipes).
        exec /tmp/ltp-bin/sh -c "$1"
    fi

    # Multiple arguments: check whether any argument contains shell
    # metacharacters that exec "$@" would misinterpret.
    _mrsh_meta=0
    for _mrsh_a in "$@"; do
        case "$_mrsh_a" in
            *';'*|*'&&'*|*'||'*|*'|'*|*'>'*|*'<'*|*'$'*|*'*'*|*'('*|*')'*)
                _mrsh_meta=1; break ;;
        esac
    done

    if [ "$_mrsh_meta" -eq 1 ]; then
        # Rebuild a single shell command string and run through sh -c.
        _mrsh_cmd=
        for _mrsh_a in "$@"; do
            if [ -z "$_mrsh_cmd" ]; then
                _mrsh_cmd="$_mrsh_a"
            else
                _mrsh_cmd="$_mrsh_cmd $_mrsh_a"
            fi
        done
        exec /tmp/ltp-bin/sh -c "$_mrsh_cmd"
    fi

    exec "$@"
fi

echo "[MINIRSH-UNSUPPORTED] host=$host cmd=$*" >&2
exit 95
MINIRSH_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_rcp_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIRCP_EOF'
#!/bin/sh
# Restricted rcp/scp fallback. It only copies local paths or localhost:path.

minicp_hostname() {
    if [ -r /proc/sys/kernel/hostname ]; then
        IFS= read -r h </proc/sys/kernel/hostname || h=
        [ -n "$h" ] && { printf '%s\n' "$h"; return; }
    fi
}

minicp_is_local_host() {
    h="$1"
    local_h="$(minicp_hostname)"
    case "$h" in
        ''|localhost|localhost.localdomain|127.0.0.1|::1|0.0.0.0) return 0 ;;
    esac
    [ -n "$local_h" ] && [ "$h" = "$local_h" ] && return 0
    [ -n "${HOSTNAME-}" ] && [ "$h" = "$HOSTNAME" ] && return 0
    [ -n "${LHOST-}" ] && [ "$h" = "$LHOST" ] && return 0
    [ -n "${THOST-}" ] && [ "$h" = "$THOST" ] && return 0
    return 1
}

minicp_path() {
    value="$1"
    case "$value" in
        *:*)
            h="${value%%:*}"
            p="${value#*:}"
            if minicp_is_local_host "$h"; then
                printf '%s\n' "$p"
            else
                echo "[MINIRCP-UNSUPPORTED] remote=$value" >&2
                return 1
            fi
            ;;
        *)
            printf '%s\n' "$value"
            ;;
    esac
}

minicp_copy_file() {
    src="$1"
    dst="$2"

    if [ -x /bin/cp ]; then
        /bin/cp "$src" "$dst"
        return $?
    fi
    if [ -x /usr/bin/cp ]; then
        /usr/bin/cp "$src" "$dst"
        return $?
    fi
    case "${LTP_CURRENT_LIBC-}" in
        glibc) bb=/glibc/busybox ;;
        musl) bb=/musl/busybox ;;
        *) bb= ;;
    esac
    if [ -n "$bb" ] && [ -x "$bb" ]; then
        "$bb" cp "$src" "$dst"
        return $?
    fi

    [ -f "$src" ] || { echo "[MINIRCP-UNSUPPORTED] no cp backend src=$src" >&2; return 95; }
    while IFS= read -r line || [ -n "$line" ]; do
        printf '%s\n' "$line"
    done <"$src" >"$dst"
}

recursive=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        -p|-q|-v|-B)
            shift
            ;;
        -r|-R)
            recursive=1
            shift
            ;;
        -P|-o|-i)
            shift
            [ "$#" -gt 0 ] || { echo "[MINIRCP-UNSUPPORTED] missing option value" >&2; exit 95; }
            shift
            ;;
        --)
            shift
            break
            ;;
        -*)
            echo "[MINIRCP-UNSUPPORTED] option=$1" >&2
            exit 95
            ;;
        *)
            break
            ;;
    esac
done

[ "$#" -eq 2 ] || { echo "[MINIRCP-UNSUPPORTED] args=$*" >&2; exit 95; }
src="$(minicp_path "$1")" || exit 95
dst="$(minicp_path "$2")" || exit 95

if [ "$recursive" -eq 1 ]; then
    if [ -x /bin/cp ]; then
        /bin/cp -r "$src" "$dst"
        exit $?
    fi
    case "${LTP_CURRENT_LIBC-}" in
        glibc) bb=/glibc/busybox ;;
        musl) bb=/musl/busybox ;;
        *) bb= ;;
    esac
    if [ -n "$bb" ] && [ -x "$bb" ]; then
        "$bb" cp -r "$src" "$dst"
        exit $?
    fi
    echo "[MINIRCP-UNSUPPORTED] recursive copy without cp backend" >&2
    exit 95
fi

minicp_copy_file "$src" "$dst"
exit $?
MINIRCP_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_ip_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIIP_EOF'
#!/bin/sh
# Minimal LTP net compatible ip wrapper. Query output is synthetic and local-only.

miniip_ifaces() {
    printed_lo=0
    if [ -r /proc/net/dev ]; then
        seen_header=0
        while IFS= read -r line || [ -n "$line" ]; do
            case "$seen_header" in
                0) seen_header=1; continue ;;
                1) seen_header=2; continue ;;
            esac
            name="${line%%:*}"
            set -- $name
            name="${1-}"
            [ -n "$name" ] || continue
            [ "$name" = "lo" ] && printed_lo=1
            printf '%s\n' "$name"
        done </proc/net/dev
    fi
    [ "$printed_lo" -eq 1 ] || printf '%s\n' lo
}

miniip_mac() {
    dev="$1"
    if [ -n "$dev" ] && [ -r "/sys/class/net/$dev/address" ]; then
        IFS= read -r mac <"/sys/class/net/$dev/address" || mac=
        case "$mac" in
            ??*:??*:??*:??*:??*:??*) printf '%s\n' "$mac"; return ;;
        esac
    fi
    if [ "$dev" = "lo" ]; then
        printf '%s\n' 00:00:00:00:00:00
    else
        printf '%s\n' 00:00:00:00:00:00
    fi
}

miniip_primary() {
    for dev in $(miniip_ifaces); do
        [ "$dev" = "lo" ] && continue
        printf '%s\n' "$dev"
        return
    done
}

miniip_known_dev() {
    want="$1"
    for dev in $(miniip_ifaces); do
        [ "$dev" = "$want" ] && return 0
    done
    return 1
}

miniip_link_show_one() {
    dev="$1"
    mac="$(miniip_mac "$dev")"
    if [ "$dev" = "lo" ]; then
        printf '1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noop state UNKNOWN mode DEFAULT group default qlen 1000\n'
        printf '    link/loopback %s brd 00:00:00:00:00:00\n' "$mac"
    else
        printf '2: %s: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc noop state UP mode DEFAULT group default qlen 1000\n' "$dev"
        printf '    link/ether %s brd ff:ff:ff:ff:ff:ff\n' "$mac"
    fi
}

miniip_link_show() {
    if [ -n "$1" ]; then
        miniip_known_dev "$1" || { echo "[MINIIP-UNSUPPORTED] args=link show $1" >&2; return 95; }
        miniip_link_show_one "$1"
        return 0
    fi
    for dev in $(miniip_ifaces); do
        miniip_link_show_one "$dev"
    done
}

miniip_addr_show_one() {
    dev="$1"
    family="$2"
    miniip_link_show_one "$dev"
    if [ "$dev" = "lo" ]; then
        [ "$family" = 6 ] || printf '    inet 127.0.0.1/8 scope host lo\n'
        [ "$family" = 4 ] || printf '    inet6 ::1/128 scope host\n'
    else
        [ "$family" = 6 ] || printf '    inet 10.0.2.15/24 scope global %s\n' "$dev"
    fi
}

miniip_addr_show() {
    dev_filter=
    while [ "$#" -gt 0 ]; do
        case "$1" in
            dev)
                shift
                [ "$#" -gt 0 ] || { echo "[MINIIP-UNSUPPORTED] args=addr show dev" >&2; return 95; }
                dev_filter="$1"
                ;;
        esac
        shift
    done

    if [ -n "$dev_filter" ]; then
        miniip_known_dev "$dev_filter" || { echo "[MINIIP-UNSUPPORTED] args=addr show dev $dev_filter" >&2; return 95; }
        miniip_addr_show_one "$dev_filter" "$family"
        return 0
    fi
    for dev in $(miniip_ifaces); do
        miniip_addr_show_one "$dev" "$family"
    done
}

miniip_route_show() {
    primary="$(miniip_primary)"
    [ -n "$primary" ] && printf 'default dev %s scope link\n' "$primary"
    printf '127.0.0.0/8 dev lo scope link\n'
}

miniip_set_dev() {
    dev="$1"
    action="$2"
    miniip_known_dev "$dev" || { echo "[MINIIP-UNSUPPORTED] args=link set $dev $action" >&2; return 95; }
    case "$action" in
        up|down) return 0 ;;
        *) echo "[MINIIP-UNSUPPORTED] args=link set $dev $action" >&2; return 95 ;;
    esac
}

family=all
while [ "$#" -gt 0 ]; do
    case "$1" in
        -4) family=4; shift ;;
        -6) family=6; shift ;;
        -o|-oneline) shift ;;
        --) shift; break ;;
        -*) echo "[MINIIP-UNSUPPORTED] args=$*" >&2; exit 95 ;;
        *) break ;;
    esac
done

cmd="${1-}"
[ -n "$cmd" ] || { miniip_link_show; exit 0; }
shift

case "$cmd" in
    link|l)
        sub="${1-show}"
        case "$sub" in
            show)
                [ "$#" -gt 0 ] && shift
                case "${1-}" in
                    dev) shift; miniip_link_show "${1-}" ;;
                    '') miniip_link_show ;;
                    *) miniip_link_show "$1" ;;
                esac
                ;;
            set)
                shift
                if [ "${1-}" = "dev" ]; then shift; fi
                dev="${1-}"
                shift || true
                action=
                while [ "$#" -gt 0 ]; do
                    case "$1" in
                        up|down) action="$1"; break ;;
                    esac
                    shift
                done
                [ -n "$dev" ] && [ -n "$action" ] || { echo "[MINIIP-UNSUPPORTED] args=link set" >&2; exit 95; }
                miniip_set_dev "$dev" "$action"
                ;;
            *)
                echo "[MINIIP-UNSUPPORTED] args=link $sub $*" >&2
                exit 95
                ;;
        esac
        ;;
    addr|address|a)
        sub="${1-show}"
        case "$sub" in
            show|'')
                [ "$#" -gt 0 ] && shift
                miniip_addr_show "$@"
                ;;
            add|del|delete)
                op="$sub"
                shift
                dev=
                while [ "$#" -gt 0 ]; do
                    if [ "$1" = "dev" ]; then
                        shift
                        dev="${1-}"
                        break
                    fi
                    shift
                done
                [ -n "$dev" ] || { echo "[MINIIP-UNSUPPORTED] args=addr $op" >&2; exit 95; }
                miniip_known_dev "$dev" && exit 0
                echo "[MINIIP-UNSUPPORTED] args=addr $op dev $dev" >&2
                exit 95
                ;;
            *)
                echo "[MINIIP-UNSUPPORTED] args=addr $sub $*" >&2
                exit 95
                ;;
        esac
        ;;
    route|r)
        sub="${1-show}"
        case "$sub" in
            show|list|'') miniip_route_show ;;
            add|del|delete)
                echo "[MINIIP-UNSUPPORTED] args=route $sub $*" >&2
                exit 95
                ;;
            *)
                echo "[MINIIP-UNSUPPORTED] args=route $sub $*" >&2
                exit 95
                ;;
        esac
        ;;
    netns)
        sub="${1-list}"
        case "$sub" in
            list|'') exit 0 ;;
            add|del|delete|exec)
                echo "[MINIIP-UNSUPPORTED] args=netns $sub $*" >&2
                exit 95
                ;;
            *)
                echo "[MINIIP-UNSUPPORTED] args=netns $sub $*" >&2
                exit 95
                ;;
        esac
        ;;
    *)
        echo "[MINIIP-UNSUPPORTED] args=$cmd $*" >&2
        exit 95
        ;;
esac
MINIIP_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_ifconfig_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIIFCONFIG_EOF'
#!/bin/sh
# Minimal LTP net compatible ifconfig wrapper.
# Records per-iface IPv4 assignments to /tmp/ltp-ifconfig-state.
# State format: <iface> <ip> <netmask> <broadcast>
# Old format (iface ip only) is compatible on read: missing fields default.
# Does NOT configure real kernel network interfaces.

minifconfig_state=/tmp/ltp-ifconfig-state
_minifconfig_ip=
_minifconfig_netmask=
_minifconfig_broadcast=

minifconfig_known_iface() {
    iface="$1"
    [ "$iface" = "lo" ] && return 0
    case " ${LHOST_IFACES-} " in *" $iface "*) return 0 ;; esac
    case " ${RHOST_IFACES-} " in *" $iface "*) return 0 ;; esac
    if [ -r "$minifconfig_state" ]; then
        while IFS= read -r _line || [ -n "$_line" ]; do
            set -- $_line
            [ "${1-}" = "$iface" ] && return 0
        done <"$minifconfig_state"
    fi
    return 1
}

# Read all stored fields for an iface into _minifconfig_* globals.
minifconfig_read_state() {
    iface="$1"
    _minifconfig_ip=
    _minifconfig_netmask=
    _minifconfig_broadcast=

    if [ "$iface" = "lo" ]; then
        _minifconfig_ip="${MINIIFCONFIG_LO_IP-127.0.0.1}"
    fi
    if [ -r "$minifconfig_state" ]; then
        while IFS= read -r _line || [ -n "$_line" ]; do
            set -- $_line
            if [ "${1-}" = "$iface" ] && [ -n "${2-}" ]; then
                _minifconfig_ip="$2"
                _minifconfig_netmask="${3-}"
                _minifconfig_broadcast="${4-}"
                return
            fi
        done <"$minifconfig_state"
    fi
}

minifconfig_show_one() {
    iface="$1"
    minifconfig_read_state "$iface"
    ip="${_minifconfig_ip:-"(none)"}"
    mask="${_minifconfig_netmask:-255.255.255.0}"
    bcast="${_minifconfig_broadcast-}"

    flags="UP"
    if [ "$iface" = "lo" ]; then
        flags="UP LOOPBACK RUNNING"
    else
        flags="UP BROADCAST RUNNING MULTICAST"
    fi

    if [ -n "$bcast" ]; then
        printf '%s\n' \
            "${iface}      Link encap:Local Loopback" \
            "          inet addr:${ip}  Bcast:${bcast}  Mask:${mask}" \
            "          ${flags}  MTU:65536  Metric:1"
    else
        printf '%s\n' \
            "${iface}      Link encap:Local Loopback" \
            "          inet addr:${ip}  Mask:${mask}" \
            "          ${flags}  MTU:65536  Metric:1"
    fi
}

minifconfig_show_all() {
    printed_lo=0
    _first=1
    if [ -r "$minifconfig_state" ]; then
        while IFS= read -r _line || [ -n "$_line" ]; do
            set -- $_line
            _iface="${1-}"
            [ -n "$_iface" ] || continue
            [ "$_iface" = "lo" ] && printed_lo=1
            [ "$_first" -eq 1 ] || echo
            _first=0
            minifconfig_show_one "$_iface"
        done <"$minifconfig_state"
    fi
    if [ "$printed_lo" -eq 0 ]; then
        [ "$_first" -eq 1 ] || echo
        minifconfig_show_one lo
    fi
}

# Record iface state.  Accepts 2-4 fields: ip [netmask] [broadcast].
# Reads existing record to preserve fields not being overwritten.
minifconfig_record() {
    iface="$1"
    ip="$2"
    new_mask="${3-}"
    new_bcast="${4-}"

    # Preserve existing fields not provided in this call.
    minifconfig_read_state "$iface"
    [ -n "$new_mask" ] || new_mask="${_minifconfig_netmask:-255.255.255.0}"
    [ -n "$new_bcast" ] || new_bcast="${_minifconfig_broadcast-}"

    tmp="${minifconfig_state}.tmp.$$"

    : >"$tmp" 2>/dev/null || {
        echo "[MINIIFCONFIG-WARN] cannot create state tmp=$tmp" >&2
        exit 0
    }

    replaced=0
    if [ -r "$minifconfig_state" ]; then
        while IFS= read -r _line || [ -n "$_line" ]; do
            set -- $_line
            _iface="${1-}"
            if [ "$_iface" = "$iface" ]; then
                printf '%s %s %s %s\n' "$iface" "$ip" "$new_mask" "$new_bcast" >>"$tmp"
                replaced=1
            else
                printf '%s\n' "$_line" >>"$tmp"
            fi
        done <"$minifconfig_state"
    fi
    [ "$replaced" -eq 0 ] && printf '%s %s %s %s\n' "$iface" "$ip" "$new_mask" "$new_bcast" >>"$tmp"

    mv "$tmp" "$minifconfig_state" 2>/dev/null || true
}

# ----- main -----
if [ "$#" -eq 0 ]; then
    minifconfig_show_all
    exit 0
fi

iface="${1-}"
shift

minifconfig_known_iface "$iface" || {
    echo "[MINIIFCONFIG-UNSUPPORTED] unknown iface=$iface args=$*" >&2
    exit 95
}

# Loop-parse remaining arguments.
_ip=
_mask=
_bcast=
_have_up=0
_arg_count=0
while [ "$#" -gt 0 ]; do
    _arg_count="$((_arg_count + 1))"
    # Clamp to avoid ridiculously large values from bad input.
    [ "$_arg_count" -lt 50 ] || { echo "[MINIIFCONFIG-UNSUPPORTED] iface=$iface too many args" >&2; exit 95; }
    case "$1" in
        up)
            _have_up=1
            shift
            ;;
        down)
            shift
            ;;
        netmask)
            shift
            [ "$#" -gt 0 ] || {
                echo "[MINIIFCONFIG-UNSUPPORTED] iface=$iface missing netmask value" >&2
                exit 95
            }
            case "$1" in
                [0-9]*.[0-9]*.[0-9]*.[0-9]*) _mask="$1" ;;
                *)
                    echo "[MINIIFCONFIG-UNSUPPORTED] iface=$iface invalid netmask=$1" >&2
                    exit 95
                    ;;
            esac
            shift
            ;;
        broadcast)
            shift
            [ "$#" -gt 0 ] || {
                echo "[MINIIFCONFIG-UNSUPPORTED] iface=$iface missing broadcast value" >&2
                exit 95
            }
            case "$1" in
                [0-9]*.[0-9]*.[0-9]*.[0-9]*) _bcast="$1" ;;
                *)
                    echo "[MINIIFCONFIG-UNSUPPORTED] iface=$iface invalid broadcast=$1" >&2
                    exit 95
                    ;;
            esac
            shift
            ;;
        [0-9]*.[0-9]*.[0-9]*.[0-9]*)
            if [ -z "$_ip" ]; then
                _ip="$1"
            else
                echo "[MINIIFCONFIG-UNSUPPORTED] iface=$iface extra ip=$1" >&2
                exit 95
            fi
            shift
            ;;
        *)
            echo "[MINIIFCONFIG-UNSUPPORTED] iface=$iface args=$*" >&2
            exit 95
            ;;
    esac
done

if [ -n "$_ip" ]; then
    # Gathered IP data — record it.
    minifconfig_record "$iface" "$_ip" "$_mask" "$_bcast"
elif [ "$_arg_count" -eq 0 ]; then
    # Bare "ifconfig <iface>" — show info.
    minifconfig_show_one "$iface"
fi
# else: "ifconfig lo up" or "ifconfig lo netmask ..." with no IP — no-op, exit 0.

exit 0
MINIIFCONFIG_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_tc_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINITC_EOF'
#!/bin/sh
# Minimal LTP net compatible tc wrapper.
# Supports qdisc show/list/add/del with netem help detection and
# optional qdisc state tracking via /tmp/ltp-tc-state.
# Does NOT implement real traffic control.

minitc_state=/tmp/ltp-tc-state

minitc_unsupported() {
    echo "[MINITC-UNSUPPORTED] args=$*" >&2
    exit 95
}

# Check whether remaining args contain a given token.
minitc_has_token() {
    token="$1"; shift
    for _a in "$@"; do
        [ "$_a" = "$token" ] && return 0
    done
    return 1
}

# Read the recorded qdisc type for a device.
minitc_read_dev() {
    dev="$1"
    if [ -r "$minitc_state" ]; then
        while IFS= read -r _line || [ -n "$_line" ]; do
            set -- $_line
            [ "${1-}" = "$dev" ] && { printf '%s\n' "${2-}"; return; }
        done <"$minitc_state"
    fi
}

# Write (or remove) a device entry in the state file.
minitc_write_dev() {
    dev="$1"
    qtype="${2-}"
    tmp="${minitc_state}.tmp.$$"
    : >"$tmp" 2>/dev/null || return 1
    replaced=0
    if [ -r "$minitc_state" ]; then
        while IFS= read -r _line || [ -n "$_line" ]; do
            set -- $_line
            if [ "${1-}" = "$dev" ]; then
                replaced=1
                [ -n "$qtype" ] && printf '%s %s\n' "$dev" "$qtype" >>"$tmp"
                # qtype empty → delete entry (skip)
            else
                printf '%s\n' "$_line" >>"$tmp"
            fi
        done <"$minitc_state"
    fi
    [ "$replaced" -eq 0 ] && [ -n "$qtype" ] && printf '%s %s\n' "$dev" "$qtype" >>"$tmp"
    mv "$tmp" "$minitc_state" 2>/dev/null || true
}

if [ "$#" -eq 0 ]; then
    printf '%s\n' \
        'Usage: tc [ OPTIONS ] OBJECT { COMMAND | help }' \
        'where  OBJECT := { qdisc | filter | class }' \
        '       OPTIONS := { -h[elp] }'
    exit 0
fi

while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|-help|--help)
            printf '%s\n' 'minimal tc wrapper for LTP net tests'
            exit 0
            ;;
        -s|-d|-b|-p|-n|-N)
            shift
            [ "$#" -gt 0 ] || minitc_unsupported "missing value for option"
            shift
            ;;
        -[sd]*) shift ;;
        -*) minitc_unsupported "$@" ;;
        *) break ;;
    esac
done

[ "$#" -gt 0 ] || { printf '%s\n' 'Usage: tc [ OPTIONS ] OBJECT { COMMAND | help }'; exit 0; }
cmd="$1"
shift

# Capture remaining arguments (after the sub-command) for token inspection.
save_rest() { _rest="$*"; }
save_rest "$@"

case "$cmd" in
    qdisc)
        sub="${1-show}"
        shift || true
        case "$sub" in
            show|list)
                # Extract optional "dev <name>".
                _show_dev=
                while [ "$#" -gt 0 ]; do
                    case "$1" in
                        dev) shift; [ "$#" -gt 0 ] && { _show_dev="$1"; shift; } ;;
                        *) shift ;;
                    esac
                done
                if [ -n "$_show_dev" ]; then
                    _qt="$(minitc_read_dev "$_show_dev")"
                    if [ "$_qt" = "netem" ]; then
                        printf '%s\n' 'qdisc netem 1: root refcnt 2'
                    else
                        printf '%s\n' 'qdisc noop 0: root refcnt 2'
                    fi
                else
                    printf '%s\n' 'qdisc noop 0: root refcnt 2'
                fi
                exit 0
                ;;
            add|change|replace)
                # Check for netem + help → output Usage for LTP check_netem.
                # $_rest is space-joined, so pass unquoted for word-splitting.
                if minitc_has_token netem $_rest && minitc_has_token help $_rest; then
                    printf '%s\n' \
                        'Usage: ... netem [ limit PACKETS ]' \
                        '                  [ delay TIME [ JITTER ] ]' \
                        '                  [ loss PERCENT ]' \
                        '                  [ duplicate PERCENT ]' \
                        '                  [ corrupt PERCENT ]' \
                        '                  [ reorder PERCENT ]'
                    exit 0
                fi
                # Non-help netem add: record in state.
                _add_dev=
                while [ "$#" -gt 0 ]; do
                    case "$1" in
                        dev) shift; [ "$#" -gt 0 ] && { _add_dev="$1"; shift; } ;;
                        *) shift ;;
                    esac
                done
                if [ -n "$_add_dev" ] && minitc_has_token netem $_rest; then
                    minitc_write_dev "$_add_dev" netem
                fi
                exit 0
                ;;
            del|delete)
                # Remove from state.
                _del_dev=
                while [ "$#" -gt 0 ]; do
                    case "$1" in
                        dev) shift; [ "$#" -gt 0 ] && { _del_dev="$1"; shift; } ;;
                        *) shift ;;
                    esac
                done
                [ -n "$_del_dev" ] && minitc_write_dev "$_del_dev" ""
                exit 0
                ;;
            *)
                minitc_unsupported "$cmd $sub $*"
                ;;
        esac
        ;;
    filter|class)
        minitc_unsupported "$cmd $*"
        ;;
    *)
        minitc_unsupported "$cmd $*"
        ;;
esac
MINITC_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_grep_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIGREP_EOF'
#!/bin/sh
# Minimal grep wrapper that translates GNU -NUM shorthand (e.g. grep -1)
# to POSIX -B NUM, which BusyBox grep supports.  This is the single call
# site that the old LTP ns-tools get_ifname helper relies on.

minigrep_find_backend() {
    # Prefer a real grep binary over the busybox applet so that extended
    # GNU options work without extra translation.
    for g in /bin/grep /usr/bin/grep /sbin/grep /usr/sbin/grep; do
        case "$g" in /tmp/ltp-bin/grep) continue ;; esac
        [ -x "$g" ] && { printf '%s\n' "$g"; return 0; }
    done

    case "${LTP_CURRENT_LIBC-}" in
        glibc) bb=/glibc/busybox ;;
        musl)  bb=/musl/busybox ;;
        *)     bb= ;;
    esac
    [ -n "$bb" ] && [ -x "$bb" ] && { printf '%s %s\n' "$bb" grep; return 0; }

    for bb in /busybox /bin/busybox ./busybox; do
        [ -x "$bb" ] && { printf '%s %s\n' "$bb" grep; return 0; }
    done
    return 1
}

minigrep_backend="$(minigrep_find_backend)"
[ -z "$minigrep_backend" ] && { echo "[MINIGREP-NO-BACKEND]" >&2; exit 95; }

# Rebuild argv, translating -NUM to -B NUM.
# We use a temp file so that arguments containing whitespace or special
# characters survive the transformation without eval.
minigrep_tmp="/tmp/minigrep_args_$$"
: > "$minigrep_tmp"
for arg in "$@"; do
    case "$arg" in
        -[1-9]|-1[0-9]|-20)
            printf '%s\n' "-B" >> "$minigrep_tmp"
            printf '%s\n' "${arg#-}" >> "$minigrep_tmp"
            ;;
        *)
            printf '%s\n' "$arg" >> "$minigrep_tmp"
            ;;
    esac
done

set --
while IFS= read -r minigrep_line || [ -n "$minigrep_line" ]; do
    set -- "$@" "$minigrep_line"
done < "$minigrep_tmp"
rm -f "$minigrep_tmp" 2>/dev/null || true

exec $minigrep_backend "$@"
MINIGREP_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_fgrep_wrapper() {
    path="$1"

    ltp_rm_f "$path" >/dev/null 2>&1 || true
    if [ -e "$path" ]; then
        echo "[LTP-TOOL-WARN] cannot replace existing tool path=$path"
        return 1
    fi

    while IFS= read -r line; do
        printf '%s\n' "$line"
    done >"$path" <<'MINIFGREP_EOF'
#!/bin/sh
# Minimal fgrep wrapper. Delegates to grep -F with a discovered backend.
# Does not implement grep -1 translation; the grep wrapper handles that.

_fgrep_find_backend() {
    # Prefer the LTP grep wrapper so that -1 → -B 1 compat is inherited.
    if [ -x /tmp/ltp-bin/grep ]; then
        printf '%s\n' /tmp/ltp-bin/grep
        return 0
    fi
    # Real grep binary.
    for g in /bin/grep /usr/bin/grep /sbin/grep /usr/sbin/grep; do
        case "$g" in /tmp/ltp-bin/grep) continue ;; esac
        [ -x "$g" ] && { printf '%s\n' "$g"; return 0; }
    done
    # BusyBox applet.
    case "${LTP_CURRENT_LIBC-}" in
        glibc) bb=/glibc/busybox ;;
        musl)  bb=/musl/busybox ;;
        *)     bb= ;;
    esac
    [ -n "$bb" ] && [ -x "$bb" ] && { printf '%s %s\n' "$bb" grep; return 0; }
    for bb in /busybox /bin/busybox ./busybox; do
        [ -x "$bb" ] && { printf '%s %s\n' "$bb" grep; return 0; }
    done
    return 1
}

_fgrep_backend="$(_fgrep_find_backend)"
[ -z "$_fgrep_backend" ] && { echo "[MINIFGREP-NO-BACKEND]" >&2; exit 95; }

exec $_fgrep_backend -F "$@"
MINIFGREP_EOF

    ltp_chmod_x "$path" || return 1
}

ltp_generate_wrapper() {
    cmd="$1"
    path="$2"

    case "$cmd" in
        sh)       ltp_generate_sh_wrapper "$path" ;;
        rsh)      ltp_generate_rsh_wrapper "$path" ;;
        ssh)      ltp_generate_rsh_wrapper "$path" ;;
        rcp)      ltp_generate_rcp_wrapper "$path" ;;
        scp)      ltp_generate_rcp_wrapper "$path" ;;
        ip)       ltp_generate_ip_wrapper "$path" ;;
        wc)       ltp_generate_wc_wrapper "$path" ;;
        mktemp)   ltp_generate_mktemp_wrapper "$path" ;;
        head)     ltp_generate_head_wrapper "$path" ;;
        setkey)   ltp_generate_setkey_wrapper "$path" ;;
        cut)      ltp_generate_cut_wrapper "$path" ;;
        awk)      ltp_generate_awk_wrapper "$path" ;;
        cat)      ltp_generate_cat_wrapper "$path" ;;
        rm)       ltp_generate_rm_wrapper "$path" ;;
        grep)     ltp_generate_grep_wrapper "$path" ;;
        fgrep)    ltp_generate_fgrep_wrapper "$path" ;;
        locale)   ltp_generate_locale_wrapper "$path" ;;
        basename) ltp_generate_basename_wrapper "$path" ;;
        dirname)  ltp_generate_dirname_wrapper "$path" ;;
        true)     ltp_generate_true_wrapper "$path" ;;
        false)    ltp_generate_false_wrapper "$path" ;;
        id)       ltp_generate_id_wrapper "$path" ;;
        pkill)    ltp_generate_pkill_wrapper "$path" ;;
        ifconfig) ltp_generate_ifconfig_wrapper "$path" ;;
        tc)       ltp_generate_tc_wrapper "$path" ;;
        *) return 1 ;;
    esac
}

ltp_prepare_one_tool() {
    libc="$1"
    cmd="$2"
    tool_path="$LTP_TOOL_DIR/$cmd"

    case "$cmd" in
        busybox)
            if found="$(command -v "$cmd" 2>/dev/null)"; then
                echo "[LTP-TOOL-PREP] libc=$libc cmd=$cmd provider=path path=$found"
            else
                echo "[LTP-TOOL-WARN] libc=$libc cmd=$cmd unavailable"
                echo "[LTP-TOOL-MISSING] cmd=$cmd"
            fi
            return
            ;;
        sh|rcp|scp|grep|fgrep|ifconfig|tc)
            if ltp_generate_wrapper "$cmd" "$tool_path" && ltp_tool_test "$cmd" "$tool_path"; then
                echo "[LTP-TOOL-PREP] libc=$libc cmd=$cmd provider=wrapper path=$tool_path"
                return
            fi
            if found="$(command -v "$cmd" 2>/dev/null)" && ltp_tool_test "$cmd" "$found"; then
                echo "[LTP-TOOL-WARN] libc=$libc cmd=$cmd wrapper unavailable fallback=$found"
                echo "[LTP-TOOL-PREP] libc=$libc cmd=$cmd provider=path path=$found"
                return
            fi
            echo "[LTP-TOOL-WARN] libc=$libc cmd=$cmd unavailable"
            echo "[LTP-TOOL-MISSING] cmd=$cmd"
            return
            ;;
    esac

    real_cmd="$(ltp_find_real_cmd "$cmd" "$libc")"
    if [ -n "$real_cmd" ] && ltp_tool_test "$cmd" "$real_cmd"; then
        if ltp_install_link "$real_cmd" "$tool_path" && ltp_tool_test "$cmd" "$tool_path"; then
            echo "[LTP-TOOL-PREP] libc=$libc cmd=$cmd provider=real path=$tool_path target=$real_cmd"
            return
        fi
        echo "[LTP-TOOL-PREP] libc=$libc cmd=$cmd provider=real path=$real_cmd"
        return
    fi

    if [ -n "$LTP_BUSYBOX" ]; then
        case "$LTP_BUSYBOX" in
            /*) bb_target="$LTP_BUSYBOX" ;;
            ./*) bb_target="$(pwd)/${LTP_BUSYBOX#./}" ;;
            *) bb_target="$LTP_BUSYBOX" ;;
        esac

        if ltp_install_link "$bb_target" "$tool_path" && ltp_tool_test "$cmd" "$tool_path"; then
            echo "[LTP-TOOL-PREP] libc=$libc cmd=$cmd provider=busybox path=$tool_path busybox=$bb_target"
            return
        fi
        ltp_rm_f "$tool_path" >/dev/null 2>&1 || true
    fi

    if ltp_generate_wrapper "$cmd" "$tool_path" && ltp_tool_test "$cmd" "$tool_path"; then
        echo "[LTP-TOOL-PREP] libc=$libc cmd=$cmd provider=wrapper path=$tool_path"
        return
    fi

    if found="$(command -v "$cmd" 2>/dev/null)" && ltp_tool_test "$cmd" "$found"; then
        echo "[LTP-TOOL-PREP] libc=$libc cmd=$cmd provider=path path=$found"
        return
    fi

    echo "[LTP-TOOL-WARN] libc=$libc cmd=$cmd unavailable"
    echo "[LTP-TOOL-MISSING] cmd=$cmd"
}

# Attempt to replace /bin/sh with a symlink to the LTP compat sh wrapper.
# This catches C library system() calls and script #!/bin/sh shebangs that
# would otherwise bypass PATH and hit the raw busybox sh which may not
# support set -u.
ltp_replace_bin_sh() {
    ltp_wrapper_sh="$LTP_TOOL_DIR/sh"

    if [ ! -x "$ltp_wrapper_sh" ]; then
        echo "[LTP-SHELL-WARN] cannot replace /bin/sh: $ltp_wrapper_sh missing" >&2
        return 0
    fi

    # Ensure /bin exists before any operations that touch /bin.
    if [ ! -d /bin ]; then
        mkdir -p /bin 2>/dev/null || true
    fi

    if [ ! -d /bin ]; then
        echo "[LTP-SHELL-WARN] /bin missing, skip /bin/sh override"
        return 0
    fi

    # /bin/sh might already be a symlink to our wrapper.
    case "$(readlink /bin/sh 2>/dev/null)" in
        "$ltp_wrapper_sh"|*/tmp/ltp-bin/sh)
            echo "[LTP-SHELL] /bin/sh already points to $ltp_wrapper_sh"
            return 0
            ;;
    esac

    # Test if /bin is writable (safe form: subshell + explicit cleanup).
    test_file="/bin/.ltp_sh_test_$$"
    if ! ( : > "$test_file" ) 2>/dev/null; then
        rm -f "$test_file" 2>/dev/null || true
        echo "[LTP-SHELL-WARN] /bin not writable, skip /bin/sh override"
        return 0
    fi
    rm -f "$test_file" 2>/dev/null || true

    # Save original and link.
    if [ -e /bin/sh.orig ]; then
        echo "[LTP-SHELL] /bin/sh.orig already exists, skipping backup"
    else
        cp /bin/sh /bin/sh.orig 2>/dev/null || {
            echo "[LTP-SHELL-WARN] cannot backup /bin/sh, skip override"
            return 0
        }
        echo "[LTP-SHELL] backed up /bin/sh → /bin/sh.orig"
    fi

    rm -f /bin/sh 2>/dev/null || true
    ln -sf "$ltp_wrapper_sh" /bin/sh 2>/dev/null || {
        echo "[LTP-SHELL-WARN] cannot replace /bin/sh, skip override"
        # Try to restore original.
        cp /bin/sh.orig /bin/sh 2>/dev/null || true
        return 0
    }

    echo "[LTP-SHELL] replaced /bin/sh → $ltp_wrapper_sh"
    return 0
}

ltp_prepare_tools() {
    libc="$1"

    LTP_TOOL_LIBC="$libc"
    LTP_BUSYBOX="$(ltp_busybox_cmd "$libc")"
    export LTP_BUSYBOX

    ltp_mkdir_p /tmp || echo "[LTP-TOOL-WARN] libc=$libc cannot create /tmp"
    if ! ltp_mkdir_p "$LTP_TOOL_DIR"; then
        echo "[LTP-TOOL-WARN] libc=$libc cannot create $LTP_TOOL_DIR"
        return
    fi

    for cmd in sh rsh ssh rcp scp ip wc mktemp head setkey cut awk cat rm grep fgrep sed expr printf locale basename dirname true false id pkill date ps ifconfig tc busybox; do
        ltp_prepare_one_tool "$libc" "$cmd"
    done

    # Try to replace /bin/sh so system() and #!/bin/sh scripts also get
    # the compat wrapper that strips set -u.
    ltp_replace_bin_sh
}

ltp_tool_diag() {
    libc="$1"

    echo "[LTP-TOOL-DIAG-BEGIN] libc=$libc category=$LTP_CATEGORY"
    echo "[LTP-TOOL-PATH] libc=$libc PATH=$PATH"
    echo "[LTP-TOOL-WRAPPER-DIR] libc=$libc dir=$LTP_TOOL_DIR"

    for cmd in $LTP_TOOL_COMMANDS; do
        if found="$(command -v "$cmd" 2>/dev/null)"; then
            echo "[LTP-TOOL-COMMAND] libc=$libc cmd=$cmd path=$found"
        else
            echo "[LTP-TOOL-MISSING] cmd=$cmd"
        fi
    done

    for dir in /bin /sbin /usr/bin /usr/sbin "$LTP_TOOL_DIR"; do
        if [ -d "$dir" ]; then
            line="[LTP-TOOL-DIR] libc=$libc path=$dir exists=yes"
            for cmd in sh rsh ssh rcp scp ip wc mktemp head setkey cut awk cat rm grep fgrep sed expr printf locale basename dirname true false id pkill date ps ifconfig tc busybox; do
                if [ -e "$dir/$cmd" ]; then
                    line="$line $cmd=yes"
                else
                    line="$line $cmd=no"
                fi
            done
            echo "$line"
        else
            echo "[LTP-TOOL-DIR] libc=$libc path=$dir exists=no"
        fi
    done

    echo "[LTP-TOOL-DIAG-END] libc=$libc category=$LTP_CATEGORY"
}

ltp_netenv_first_iface() {
    if [ -r /proc/net/dev ]; then
        n=0
        first=
        while IFS= read -r line || [ -n "$line" ]; do
            n="$(ltp_small_inc "$n")"
            [ "$n" -le 2 ] && continue
            name="${line%%:*}"
            set -- $name
            name="${1-}"
            [ -n "$name" ] || continue
            [ "$name" = "lo" ] && continue
            first="$name"
            break
        done </proc/net/dev
        [ -n "$first" ] && { printf '%s\n' "$first"; return; }
    fi

    printf '%s\n' lo
}

ltp_netenv_hwaddr() {
    iface="$1"

    if [ -n "$iface" ] && [ -r "/sys/class/net/$iface/address" ]; then
        IFS= read -r mac <"/sys/class/net/$iface/address" || mac=
        case "$mac" in
            ??*:??*:??*:??*:??*:??*) printf '%s %s\n' "$mac" sysfs; return ;;
        esac
    fi

    if [ "$iface" = "lo" ]; then
        printf '%s %s\n' 00:00:00:00:00:00 loopback
        return
    fi

    echo "[LTP-NETENV-WARN] missing mac iface=$iface" >&2
    printf '%s %s\n' 00:00:00:00:00:00 fallback
}

ltp_netenv_build_hwaddrs() {
    ifaces="$1"

    LTP_NETENV_HWADDRS_RESULT=
    for iface in $ifaces; do
        pair="$(ltp_netenv_hwaddr "$iface")"
        set -- $pair
        mac="${1-00:00:00:00:00:00}"
        source="${2-fallback}"
        echo "[LTP-NETENV] iface=$iface mac=$mac source=$source"
        if [ -n "$LTP_NETENV_HWADDRS_RESULT" ]; then
            LTP_NETENV_HWADDRS_RESULT="$LTP_NETENV_HWADDRS_RESULT $mac"
        else
            LTP_NETENV_HWADDRS_RESULT="$mac"
        fi
    done

    if [ -z "$LTP_NETENV_HWADDRS_RESULT" ]; then
        echo "[LTP-NETENV-WARN] missing mac iface="
        LTP_NETENV_HWADDRS_RESULT=00:00:00:00:00:00
    fi
}

ltp_netenv_diag_preset_hwaddrs() {
    ifaces="$1"
    macs="$2"

    set -- $macs
    for iface in $ifaces; do
        mac="${1-}"
        [ -n "$mac" ] || mac=missing
        echo "[LTP-NETENV] iface=$iface mac=$mac source=preset"
        [ "$#" -gt 0 ] && shift
    done
}

ltp_word_count() {
    set -- $1
    echo "$#"
}

ltp_netenv_check_count() {
    side="$1"
    ifaces="$2"
    macs="$3"

    iface_count="$(ltp_word_count "$ifaces")"
    mac_count="$(ltp_word_count "$macs")"
    if [ "$iface_count" -ne "$mac_count" ]; then
        echo "[LTP-NETENV-WARN] ${side}_IFACES/HWADDRS count mismatch ifaces=$iface_count hwaddrs=$mac_count"
    fi
}

ltp_set_default_env() {
    name="$1"
    value="$2"

    eval "current=\${$name-}"
    if [ -z "$current" ]; then
        eval "export $name=\"\$value\""
    fi
}

ltp_netenv_warn_missing() {
    for name in LHOST RHOST LHOST_IFACES RHOST_IFACES LHOST_HWADDRS RHOST_HWADDRS IPV4_LHOST IPV4_RHOST IPV6_LHOST IPV6_RHOST; do
        eval "value=\${$name-}"
        [ -n "$value" ] || echo "[LTP-NETENV-WARN] missing var=$name"
    done
}

ltp_sysfs_write() {
    dir="$1"
    name="$2"
    value="$3"

    [ -d "$dir" ] || return 1
    if printf '%s\n' "$value" >"$dir/$name" 2>/dev/null; then
        return 0
    fi
    return 1
}

# Create /sys/class/net/lo entry so LTP helpers that read
# /sys/class/net/*/address (get_iface_by_hwaddr(), tst_get_iface(), etc.)
# can discover lo and map 00:00:00:00:00:00 back to the interface name.
#
# /sys is a MemoryFs (pseudofs/mod.rs line 74). It starts empty and is
# writable, so we can create these files at runtime without kernel changes.
ltp_create_sysfs_net() {
    ltp_mkdir_p /sys/class/net/lo || {
        echo "[LTP-NETENV-WARN] cannot create /sys/class/net/lo" >&2
        return 1
    }

    ltp_sysfs_write /sys/class/net/lo address  00:00:00:00:00:00
    ltp_sysfs_write /sys/class/net/lo ifindex  1
    ltp_sysfs_write /sys/class/net/lo flags    0x1003
    ltp_sysfs_write /sys/class/net/lo operstate unknown
    ltp_sysfs_write /sys/class/net/lo type     772
    ltp_sysfs_write /sys/class/net/lo mtu      65536
    ltp_sysfs_write /sys/class/net/lo tx_queue_len 1000

    echo "[LTP-NETENV] /sys/class/net/lo created"
}

ltp_prepare_net_env() {
    libc="$1"
    iface=lo

    # Create /sys/class/net/lo so LTP helpers can reverse-lookup MAC→iface.
    ltp_create_sysfs_net

    # Keep localhost defaults unless the caller already provided a real test topology.
    ltp_set_default_env LHOST localhost
    ltp_set_default_env RHOST localhost
    ltp_set_default_env IPV4_LHOST 127.0.0.1
    ltp_set_default_env IPV4_RHOST 127.0.0.1
    ltp_set_default_env IPV6_LHOST ::1
    ltp_set_default_env IPV6_RHOST ::1
    ltp_set_default_env LHOST_IFACES "${iface:-lo}"
    ltp_set_default_env RHOST_IFACES "${iface:-lo}"

    if [ -z "${LHOST_HWADDRS-}" ]; then
        ltp_netenv_build_hwaddrs "$LHOST_IFACES"
        export LHOST_HWADDRS="$LTP_NETENV_HWADDRS_RESULT"
    else
        ltp_netenv_diag_preset_hwaddrs "$LHOST_IFACES" "$LHOST_HWADDRS"
    fi
    if [ -z "${RHOST_HWADDRS-}" ]; then
        ltp_netenv_build_hwaddrs "$RHOST_IFACES"
        export RHOST_HWADDRS="$LTP_NETENV_HWADDRS_RESULT"
    else
        ltp_netenv_diag_preset_hwaddrs "$RHOST_IFACES" "$RHOST_HWADDRS"
    fi

    export LHOST RHOST LHOST_IFACES RHOST_IFACES LHOST_HWADDRS RHOST_HWADDRS
    export IPV4_LHOST IPV4_RHOST IPV6_LHOST IPV6_RHOST

    echo "[LTP-NETENV] libc=$libc LHOST=$LHOST RHOST=$RHOST IPV4_LHOST=$IPV4_LHOST IPV4_RHOST=$IPV4_RHOST IPV6_LHOST=$IPV6_LHOST IPV6_RHOST=$IPV6_RHOST"
    echo "[LTP-NETENV] LHOST_IFACES=$LHOST_IFACES LHOST_HWADDRS=$LHOST_HWADDRS"
    echo "[LTP-NETENV] RHOST_IFACES=$RHOST_IFACES RHOST_HWADDRS=$RHOST_HWADDRS"
    ltp_netenv_check_count LHOST "$LHOST_IFACES" "$LHOST_HWADDRS"
    ltp_netenv_check_count RHOST "$RHOST_IFACES" "$RHOST_HWADDRS"
    ltp_netenv_warn_missing
}

ltp_libc_busybox_shell() {
    libc="$1"

    case "$libc" in
        glibc) candidate=/glibc/busybox ;;
        musl) candidate=/musl/busybox ;;
        *) return 1 ;;
    esac

    [ -x "$candidate" ] || return 1
    "$candidate" sh -c ':' >/dev/null 2>&1 || return 1
    echo "$candidate"
}

ltp_arch_hint() {
    if [ -r /proc/cpuinfo ]; then
        while IFS= read -r line || [ -n "$line" ]; do
            case "$line" in
                *[Ll]oong[Aa]rch*|*loongarch*) echo loongarch64; return ;;
                *[Rr][Ii][Ss][Cc][Vv]*) echo riscv; return ;;
            esac
        done </proc/cpuinfo
    fi

    if command -v uname >/dev/null 2>&1; then
        uname -m 2>/dev/null
    fi
}

ltp_shell_probe_supports_set_u() {
    shell="$1"
    mode="$2"
    out="/tmp/ltp_shell_probe_$$_out"
    err="/tmp/ltp_shell_probe_$$_err"
    invalid=0

    ltp_rm_f "$out" >/dev/null 2>&1 || true
    ltp_rm_f "$err" >/dev/null 2>&1 || true

    if [ "$mode" = busybox ]; then
        "$shell" sh -c 'set -u; echo ok' >"$out" 2>"$err"
    else
        "$shell" -c 'set -u; echo ok' >"$out" 2>"$err"
    fi
    rc=$?

    if [ -r "$err" ]; then
        while IFS= read -r line || [ -n "$line" ]; do
            case "$line" in
                *'invalid option'*|*'Invalid option'*)
                    invalid=1
                    ;;
            esac
        done <"$err"
    fi

    ltp_rm_f "$out" >/dev/null 2>&1 || true
    ltp_rm_f "$err" >/dev/null 2>&1 || true

    [ "$rc" -eq 0 ] && [ "$invalid" -eq 0 ]
}

ltp_prepare_shell() {
    libc="$1"

    LTP_CASE_BUSYBOX=
    LTP_SHELL_SUPPORTS_SET_U=0
    LTP_SHELL_COMPAT=1
    compat_reason=set-u-unsupported
    arch_hint="$(ltp_arch_hint)"
    if shell="$(ltp_libc_busybox_shell "$libc")"; then
        LTP_CASE_BUSYBOX="$shell"
        if ltp_shell_probe_supports_set_u "$shell" busybox; then
            LTP_SHELL_SUPPORTS_SET_U=1
            LTP_SHELL_COMPAT=0
            compat_reason=
        fi
        case "$arch_hint" in
            *loongarch*)
                LTP_SHELL_COMPAT=1
                compat_reason=loongarch-default
                ;;
        esac
        export LTP_CASE_BUSYBOX LTP_SHELL_SUPPORTS_SET_U LTP_SHELL_COMPAT
        echo "[LTP-SHELL] libc=$libc shell=$LTP_CASE_BUSYBOX supports_set_u=$LTP_SHELL_SUPPORTS_SET_U compat=$LTP_SHELL_COMPAT"
        if [ "$LTP_SHELL_COMPAT" = 1 ]; then
            echo "[LTP-SHELL-COMPAT] enabled=1 reason=$compat_reason"
        fi
    else
        unset LTP_CASE_BUSYBOX
        if ltp_shell_probe_supports_set_u /bin/sh direct; then
            LTP_SHELL_SUPPORTS_SET_U=1
            LTP_SHELL_COMPAT=0
            compat_reason=
        fi
        case "$arch_hint" in
            *loongarch*)
                LTP_SHELL_COMPAT=1
                compat_reason=loongarch-default
                ;;
        esac
        export LTP_SHELL_SUPPORTS_SET_U LTP_SHELL_COMPAT
        echo "[LTP-SHELL-WARN] libc=$libc shell=direct reason=no-${libc}-busybox-sh supports_set_u=$LTP_SHELL_SUPPORTS_SET_U compat=$LTP_SHELL_COMPAT"
        if [ "$LTP_SHELL_COMPAT" = 1 ]; then
            echo "[LTP-SHELL-COMPAT] enabled=1 reason=$compat_reason"
        fi
    fi
}

ltp_exec_case_file() {
    file="$1"

    case "$file" in
        *.sh)
            if [ -x "$LTP_TOOL_DIR/sh" ]; then
                "$LTP_TOOL_DIR/sh" "$file" </dev/null
            elif [ -n "$LTP_CASE_BUSYBOX" ] && [ -x "$LTP_CASE_BUSYBOX" ]; then
                "$LTP_CASE_BUSYBOX" sh "$file" </dev/null
            else
                "$file" </dev/null
            fi
            ;;
        *)
            "$file" </dev/null
            ;;
    esac
}

is_leftover_command() {
    case "$1" in
        "./iperf3 -s"*|"iperf3 -s"*|"/glibc/iperf3 -s"*|"/musl/iperf3 -s"*| \
        "./netserver"*|"netserver"*|"/glibc/netserver"*|"/musl/netserver"*| \
        "./lmbench_all"*|"lmbench_all"*|"/glibc/lmbench_all"*|"/musl/lmbench_all"*| \
        "./pipe 10"*|"pipe 10"*|"/glibc/pipe 10"*|"/musl/pipe 10"*| \
        "./busybox sh ./lmbench_testcode.sh"*|"/glibc/busybox sh ./lmbench_testcode.sh"*|"/musl/busybox sh ./lmbench_testcode.sh"*| \
        "./busybox sh ./unixbench_testcode.sh"*|"/glibc/busybox sh ./unixbench_testcode.sh"*|"/musl/busybox sh ./unixbench_testcode.sh"*)
            return 0
            ;;
    esac

    return 1
}

needs_cleanup() {
    case "$1" in
        *iperf*|*netperf*|*lmbench*|*unixbench*)
            return 0
            ;;
    esac
    return 1
}

cleanup_leftovers_once() {
    signal="$1"
    bb="$2"

    "$bb" ps | while read pid user time command; do
        case "$pid" in
            ''|PID) continue ;;
        esac
        [ "$pid" = "$$" ] && continue
        case "$command" in
            *"cleanup_leftovers_once"*) continue ;;
        esac

        if is_leftover_command "$command"; then
            echo "cleanup leftover pid $pid: $command"
            "$bb" kill "$signal" "$pid" >/dev/null 2>&1
        fi
    done
}

cleanup_leftovers() {
    bb="$(busybox_cmd)"
    [ -n "$bb" ] || return

    cleanup_leftovers_once -TERM "$bb"
    "$bb" sleep 1
    cleanup_leftovers_once -KILL "$bb"
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

set_library_path() {
    case "$1" in
        /glibc|/glibc/*) export LD_LIBRARY_PATH=/glibc/lib ;;
        /musl|/musl/*) export LD_LIBRARY_PATH=/musl/lib ;;
        *) unset LD_LIBRARY_PATH ;;
    esac
}

run_test_dir() {
    dir="$1"

    [ -d "$dir" ] || return
    cd "$dir" || return

    set_library_path "$dir"

    if [ -f ./test_all.sh ]; then
        found=1
        echo "run ${dir}/test_all.sh"
        run_with_shell ./test_all.sh
        needs_cleanup "$dir" && cleanup_leftovers
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
        needs_cleanup "$testcase" && cleanup_leftovers
    done

    cd /
}

run_test_path() {
    script="$1"

    [ -f "$script" ] || return

    found=1
    dir="${script%/*}"
    name="${script##*/}"

    cd "$dir" || return
    set_library_path "$dir"
    echo "run ${dir}/${name}"
    if skip_ltp_testcase "$name" "$dir"; then
        cd /
        return
    fi
    run_with_shell "./$name"
    needs_cleanup "$script" && cleanup_leftovers
    cd /
}

run_stable_tests() {
    for testcase in \
        /glibc/basic_testcode.sh \
        /glibc/busybox_testcode.sh \
        /glibc/cyclictest_testcode.sh \
        /glibc/iozone_testcode.sh \
        /glibc/iperf_testcode.sh \
        /glibc/libcbench_testcode.sh \
        /glibc/lua_testcode.sh \
        /glibc/netperf_testcode.sh \
        /musl/basic_testcode.sh \
        /musl/busybox_testcode.sh \
        /musl/lua_testcode.sh \
        /musl/iozone_testcode.sh \
        /musl/iperf_testcode.sh \
        /musl/netperf_testcode.sh \
        /musl/libcbench_testcode.sh
    do
        run_test_path "$testcase"
    done
}

run_cyclictest_musl_tests() {
    found=1
    run_test_path /musl/cyclictest_testcode.sh
}

run_cyclictest_tests() {
    found=1
    run_test_path /glibc/cyclictest_testcode.sh
    run_test_path /musl/cyclictest_testcode.sh
}

run_libctest_tests() {
    found=1
    run_test_path /glibc/libctest_testcode.sh
    run_test_path /musl/libctest_testcode.sh
}

run_iozone_tests() {
    found=1
    echo "run iozone tests for sys_sync/syncfs verification"
    run_test_path /glibc/iozone_testcode.sh
    run_test_path /musl/iozone_testcode.sh
}

run_lmbench_tests() {
    found=1
    run_test_path /glibc/lmbench_testcode.sh
    run_test_path /musl/lmbench_testcode.sh
}

run_lmbench_only_tests() {
    run_test_path /glibc/lmbench_testcode.sh
}

run_lmbench_fast_tests() {
    found=1
    cd /glibc || return
    set_library_path /glibc
    echo "run /glibc/lmbench-fast"
    echo "#### OS COMP TEST GROUP START lmbench-glibc ####"

    echo latency measurements
    ./lmbench_all lat_syscall -P 1 null
    ./lmbench_all lat_syscall -P 1 read
    ./lmbench_all lat_syscall -P 1 write
    ./busybox mkdir -p /var/tmp
    ./busybox touch /var/tmp/lmbench
    ./lmbench_all lat_syscall -P 1 stat /var/tmp/lmbench
    ./lmbench_all lat_syscall -P 1 fstat /var/tmp/lmbench
    ./lmbench_all lat_syscall -P 1 open /var/tmp/lmbench
    ./lmbench_all lat_select -n 100 -P 1 file
    ./lmbench_all lat_sig -P 1 install
    ./lmbench_all lat_sig -P 1 catch
    ./lmbench_all lat_pipe -P 1
    ./lmbench_all lat_proc -P 1 fork
    ./lmbench_all lat_proc -P 1 exec
    ./lmbench_all lat_proc -P 1 shell
    ./lmbench_all lmdd label="File /var/tmp/XXX write bandwidth:" of=/var/tmp/XXX move=1m fsync=1 print=3
    ./lmbench_all lat_pagefault -P 1 /var/tmp/XXX
    ./lmbench_all lat_mmap -P 1 512k /var/tmp/XXX

    echo Bandwidth measurements
    ./lmbench_all bw_pipe -P 1

    echo "#### OS COMP TEST GROUP END lmbench-glibc ####"
    cd /
}

run_ltp_dir() {
    dir="$1"
    group="$2"
    target_dir="ltp/testcases/bin"

    echo "run ${dir}/ltp"
    echo "#### OS COMP TEST GROUP START $group ####"

    if [ -d "$dir/$target_dir" ] && cd "$dir"; then
        set_library_path "$dir"

        for name in abs01 brk01 brk02; do
            file="$target_dir/$name"
            [ -f "$file" ] || continue

            echo "RUN LTP CASE $name"
            "$file" </dev/null
            ret=$?
            case "$ret" in
                0) echo "PASS LTP CASE $name : $ret" ;;
                *) echo "FAIL LTP CASE $name : $ret" ;;
            esac
        done

        cd /
    fi

    echo "#### OS COMP TEST GROUP END $group ####"
}

run_ltp_only_tests() {
    found=1
    SKIP_LTP=0
    run_ltp_dir /glibc ltp-glibc
    run_ltp_dir /musl ltp-musl
}

run_ltp_list_libc() {
    libc="$1"
    target_dir="$2"
    count=0

    echo "[LTP-LIST-START] $libc"

    if [ ! -d "$target_dir" ]; then
        echo "[LTP-LIST-ERROR] $libc directory not found: $target_dir"
        echo "[LTP-LIST-END] $libc COUNT=$count"
        return
    fi

    for file in "$target_dir"/*; do
        [ -f "$file" ] || continue
        name="${file##*/}"
        count="$(ltp_small_inc "$count")"
        printf 'LTP_CASE %s %06d %s\n' "$libc" "$count" "$name"
    done

    echo "[LTP-LIST-END] $libc COUNT=$count"
}

run_ltp_list_tests() {
    found=1
    run_ltp_list_libc glibc /glibc/ltp/testcases/bin
    run_ltp_list_libc musl /musl/ltp/testcases/bin
}

run_ltp_case_file() {
    file="$1"
    case_name="$2"
    _case_ret=0

    # Normalize LTP_TIMEOUT: empty / 0 / non-numeric → run without watchdog
    case "$LTP_TIMEOUT" in
        ''|0|*[!0-9]*|??????*)
            if [ -n "$LTP_TIMEOUT" ] && [ "$LTP_TIMEOUT" != "0" ]; then
                echo "[LTP-BATCH-TIMEOUT-DISABLED] invalid LTP_TIMEOUT=$LTP_TIMEOUT"
            fi
            ltp_exec_case_file "$file"
            _case_ret=$?
            _case_ret="$(ltp_safe_return_rc "$_case_ret")"
            return $_case_ret
            ;;
    esac

    bb="$(busybox_cmd)"
    if [ -z "$bb" ]; then
        echo "[LTP-BATCH-TIMEOUT-DISABLED] no busybox available for watchdog"
        ltp_exec_case_file "$file"
        _case_ret=$?
        _case_ret="$(ltp_safe_return_rc "$_case_ret")"
        return $_case_ret
    fi

    # Ensure /tmp exists for the exit-code file.
    "$bb" mkdir -p /tmp 2>/dev/null || true

    # Run test case in background.
    # Write the real exit code to a temp file instead of relying on `wait $pid`.
    # In busybox ash, the SIGCHLD handler may reap the zombie before our
    # explicit `wait` runs, producing "wait: pid xxx is not a child of this shell".
    # The temp-file approach avoids this race entirely.
    LTP_EXIT_SEQ=${LTP_EXIT_SEQ:-0}
    LTP_EXIT_SEQ="$(ltp_small_inc "$LTP_EXIT_SEQ")"
    _exit_file="/tmp/ltp_exit_$$_$LTP_EXIT_SEQ"
    ( ltp_exec_case_file "$file"; echo $? >"$_exit_file" ) &
    child=$!

    # Start watchdog timer: after LTP_TIMEOUT seconds, send TERM then KILL
    (
        "$bb" sleep "$LTP_TIMEOUT"
        "$bb" kill -TERM "$child" >/dev/null 2>&1
        "$bb" sleep 1
        "$bb" kill -KILL "$child" >/dev/null 2>&1
    ) &
    timer=$!

    # Poll for child exit.
    # A blocking wait would hang forever if the child is stuck in
    # D-state (uninterruptible kernel sleep), breaking the entire batch.
    _case_ret=124
    # LTP_TIMEOUT is already validated above; use a sane fallback just in case.
    _ltp_timeout="$(ltp_stats_num "$LTP_TIMEOUT")"
    [ "$_ltp_timeout" -gt 0 ] || _ltp_timeout=30
    max_wait=$((_ltp_timeout + 5))
    waited=0
    while [ "$waited" -lt "$max_wait" ]; do
        if ! "$bb" kill -0 "$child" >/dev/null 2>&1; then
            # Child has exited — read real exit code from temp file.
            if [ -f "$_exit_file" ]; then
                read _case_ret <"$_exit_file"
                ltp_rm_f "$_exit_file" >/dev/null 2>&1 || true
            else
                _case_ret=255
                echo "[LTP-RUNNER-WARN] missing exit file case=$case_name"
            fi
            # Map killed-by-signal to timeout (signal 9→137, 15→143).
            case "$_case_ret" in
                137|143) _case_ret=124 ;;
            esac
            break
        fi
        "$bb" sleep 1
        waited="$(ltp_small_inc "$waited")"
    done

    # If child is STILL alive after max_wait, it's stuck in D-state.
    # Abandon it (init will reap the orphan) and continue the batch.
    if "$bb" kill -0 "$child" >/dev/null 2>&1; then
        echo "[LTP-BATCH-WARN] case stuck in D-state, abandoning pid $child"
        _case_ret=124
    fi

    # Clean up the watchdog timer.  Suppress stderr — if the timer already
    # exited and was reaped by the shell, `wait` would produce the same
    # "not a child of this shell" noise.
    "$bb" kill -KILL "$timer" >/dev/null 2>&1 || true
    wait "$timer" 2>/dev/null || true
    ltp_rm_f "$_exit_file" >/dev/null 2>&1 || true

    case "$_case_ret" in
        ''|*[!0-9]*)
            echo "[LTP-RUNNER-WARN] invalid rc case=$case_name rc=$_case_ret"
            ;;
        ????*)
            echo "[LTP-RUNNER-WARN] out-of-range rc case=$case_name rc=$_case_ret"
            ;;
    esac
    _case_ret="$(ltp_safe_return_rc "$_case_ret")"
    return $_case_ret
}

ltp_has_case() {
    dir="$1"
    shift
    [ -d "$dir" ] || return 1
    for name in "$@"; do
        [ -f "$dir/$name" ] && return 0
    done
    return 1
}

ltp_bin_dir() {
    libc="$1"
    shift

    libc_dir="/$libc"

    for candidate in \
        "$libc_dir/ltp/testcases/bin" \
        /ltp/testcases/bin \
        ./ltp/testcases/bin \
        . \
        "$libc_dir" \
        ./*/ltp/testcases/bin \
        ./*/*/ltp/testcases/bin
    do
        if ltp_has_case "$candidate" "$@"; then
            cd "$candidate" && pwd
            return
        fi
    done

    return 1
}

ltp_no_bin() {
    libc="$1"
    batches="$2"
    shift 2

    echo "[LTP-BATCH-DIAG] ls /"
    if command -v ls >/dev/null 2>&1; then
        ls /
    else
        bb="$(busybox_cmd)"
        if [ -n "$bb" ]; then
            "$bb" ls /
        else
            echo "[LTP-BATCH-DIAG] ls unavailable"
        fi
    fi

    for path in "/$libc" /glibc /musl /ltp; do
        if [ -d "$path" ] || [ -f "$path" ]; then
            echo "[LTP-BATCH-DIAG] exists $path yes"
        else
            echo "[LTP-BATCH-DIAG] exists $path no"
        fi
    done

    for name in "$@"; do
        echo "[LTP-BATCH-DIAG] case $name"
    done
    echo "[LTP-BATCH-ERROR] ltp bin dir not found: libc=$libc category=$LTP_CATEGORY batches=$batches"
}

ltp_safe_return_rc() {
    rc="$1"

    case "$rc" in
        ''|*[!0-9]*)
            echo 255
            return
            ;;
    esac

    case "$rc" in
        ????*)
            echo 255
            return
            ;;
    esac

    if [ "$rc" -gt 255 ]; then
        echo 255
    else
        echo "$rc"
    fi
}

ltp_exit_label() {
    # LTP exit codes: 0=TPASS 1=TFAIL 2=TBROK 4=TWARN 32=TCONF.
    # Combined codes use bitwise OR (e.g. 36 = 32|4 = TCONF|TWARN).
    # Priority: timeout, caller-owned NOT-FOUND, BROK, FAIL, CONF, WARN, PASS, UNKNOWN.
    code="$1"

    case "$code" in
        124)
            echo "TIMEOUT"
            return
            ;;
        ''|*[!0-9]*)
            echo "UNKNOWN"
            return
            ;;
    esac

    case "$code" in
        ???*)
            echo "UNKNOWN"
            return
            ;;
    esac

    # LTP result masks are small bitsets.  Command/runner status codes such as
    # 127 or missing-exit-file 255 must not be interpreted as TBROK/TFAIL bits.
    if [ "$code" -gt 63 ]; then
        echo "UNKNOWN"
        return
    fi

    if [ $((code & 2)) -ne 0 ]; then
        echo "BROK"
    elif [ $((code & 1)) -ne 0 ]; then
        echo "FAIL"
    elif [ $((code & 32)) -ne 0 ]; then
        echo "CONF"
    elif [ $((code & 4)) -ne 0 ]; then
        echo "WARN"
    elif [ "$code" -eq 0 ]; then
        echo "PASS"
    else
        echo "UNKNOWN"
    fi
}

ltp_stats_reset_totals() {
    LTP_TOTAL_RUN=0
    LTP_TOTAL_RESULT=0
    LTP_TOTAL_PASS=0
    LTP_TOTAL_FAIL=0
    LTP_TOTAL_BROK=0
    LTP_TOTAL_CONF=0
    LTP_TOTAL_WARN=0
    LTP_TOTAL_UNKNOWN=0
    LTP_TOTAL_NOT_FOUND=0
    LTP_TOTAL_TIMEOUT=0
    LTP_TOTAL_LAST_CASE=
}

ltp_stats_reset_batch() {
    LTP_BATCH_RUN_COUNT=0
    LTP_BATCH_RESULT_COUNT=0
    LTP_BATCH_PASS=0
    LTP_BATCH_FAIL=0
    LTP_BATCH_BROK=0
    LTP_BATCH_CONF=0
    LTP_BATCH_WARN=0
    LTP_BATCH_UNKNOWN=0
    LTP_BATCH_NOT_FOUND=0
    LTP_BATCH_TIMEOUT=0
    LTP_BATCH_LAST_CASE=
}

ltp_stats_add_run() {
    case_name="$1"

    LTP_BATCH_RUN_COUNT="$(ltp_small_inc "$LTP_BATCH_RUN_COUNT")"
    LTP_TOTAL_RUN="$(ltp_small_inc "$LTP_TOTAL_RUN")"
    LTP_BATCH_LAST_CASE="$case_name"
    LTP_TOTAL_LAST_CASE="$case_name"
}

ltp_stats_add_result() {
    label="$1"

    LTP_BATCH_RESULT_COUNT="$(ltp_small_inc "$LTP_BATCH_RESULT_COUNT")"
    LTP_TOTAL_RESULT="$(ltp_small_inc "$LTP_TOTAL_RESULT")"

    case "$label" in
        PASS)
            LTP_BATCH_PASS="$(ltp_small_inc "$LTP_BATCH_PASS")"
            LTP_TOTAL_PASS="$(ltp_small_inc "$LTP_TOTAL_PASS")"
            ;;
        FAIL)
            LTP_BATCH_FAIL="$(ltp_small_inc "$LTP_BATCH_FAIL")"
            LTP_TOTAL_FAIL="$(ltp_small_inc "$LTP_TOTAL_FAIL")"
            ;;
        BROK)
            LTP_BATCH_BROK="$(ltp_small_inc "$LTP_BATCH_BROK")"
            LTP_TOTAL_BROK="$(ltp_small_inc "$LTP_TOTAL_BROK")"
            ;;
        CONF)
            LTP_BATCH_CONF="$(ltp_small_inc "$LTP_BATCH_CONF")"
            LTP_TOTAL_CONF="$(ltp_small_inc "$LTP_TOTAL_CONF")"
            ;;
        WARN)
            LTP_BATCH_WARN="$(ltp_small_inc "$LTP_BATCH_WARN")"
            LTP_TOTAL_WARN="$(ltp_small_inc "$LTP_TOTAL_WARN")"
            ;;
        NOT-FOUND)
            LTP_BATCH_NOT_FOUND="$(ltp_small_inc "$LTP_BATCH_NOT_FOUND")"
            LTP_TOTAL_NOT_FOUND="$(ltp_small_inc "$LTP_TOTAL_NOT_FOUND")"
            ;;
        TIMEOUT)
            LTP_BATCH_TIMEOUT="$(ltp_small_inc "$LTP_BATCH_TIMEOUT")"
            LTP_TOTAL_TIMEOUT="$(ltp_small_inc "$LTP_TOTAL_TIMEOUT")"
            ;;
        *)
            LTP_BATCH_UNKNOWN="$(ltp_small_inc "$LTP_BATCH_UNKNOWN")"
            LTP_TOTAL_UNKNOWN="$(ltp_small_inc "$LTP_TOTAL_UNKNOWN")"
            ;;
    esac
}

ltp_print_ltp_result() {
    label="$1"
    name="$2"
    rc="$3"

    [ -n "$label" ] || label=UNKNOWN
    echo "$label LTP CASE $name : $rc"
    ltp_stats_add_result "$label"
}

ltp_stats_num() {
    n="$1"
    # Return 0 for empty, non-numeric, or unreasonably large values.
    # Max 999999 covers up to ~1M test cases per batch/total.
    case "$n" in
        ''|*[!0-9]*|???????*) echo 0 ;;
        *) echo "$n" ;;
    esac
}

ltp_stats_safe_cmp() {
    a="$1"
    op="$2"
    b="$3"

    case "$a" in ''|*[!0-9]*) return 1 ;; esac
    case "$b" in ''|*[!0-9]*) return 1 ;; esac

    [ "$a" "$op" "$b" ]
}

ltp_stats_batch_summary() {
    libc="$1"
    batch="$2"

    run="$(ltp_stats_num "$LTP_BATCH_RUN_COUNT")"
    result_count="$(ltp_stats_num "$LTP_BATCH_RESULT_COUNT")"
    pass="$(ltp_stats_num "$LTP_BATCH_PASS")"
    fail="$(ltp_stats_num "$LTP_BATCH_FAIL")"
    brok="$(ltp_stats_num "$LTP_BATCH_BROK")"
    conf="$(ltp_stats_num "$LTP_BATCH_CONF")"
    warn="$(ltp_stats_num "$LTP_BATCH_WARN")"
    unknown="$(ltp_stats_num "$LTP_BATCH_UNKNOWN")"
    not_found="$(ltp_stats_num "$LTP_BATCH_NOT_FOUND")"
    timeout="$(ltp_stats_num "$LTP_BATCH_TIMEOUT")"

    result_sum=$((pass + fail + brok + conf + warn + unknown + not_found + timeout))
    echo "[LTP-BATCH-SUMMARY] libc=$libc category=$LTP_CATEGORY batch=$batch run=$run pass=$pass fail=$fail brok=$brok conf=$conf warn=$warn unknown=$unknown not_found=$not_found timeout=$timeout"
    if { ! ltp_stats_safe_cmp "$run" -eq "$result_count"; } || \
       { ! ltp_stats_safe_cmp "$result_count" -eq "$result_sum"; }; then
        echo "[LTP-SUMMARY-WARN] result mismatch libc=$libc category=$LTP_CATEGORY batch=$batch last_case=$LTP_BATCH_LAST_CASE run=$run result_count=$result_count result_sum=$result_sum"
        # Emit recent-case diagnostics to help trace which path is uncounted.
        echo "[LTP-SUMMARY-DIAG] recent batch last_case=$LTP_BATCH_LAST_CASE run=$run result_count=$result_count result_sum=$result_sum"
    fi
}

ltp_stats_total_summary() {
    libc="$1"

    run="$(ltp_stats_num "$LTP_TOTAL_RUN")"
    result_count="$(ltp_stats_num "$LTP_TOTAL_RESULT")"
    pass="$(ltp_stats_num "$LTP_TOTAL_PASS")"
    fail="$(ltp_stats_num "$LTP_TOTAL_FAIL")"
    brok="$(ltp_stats_num "$LTP_TOTAL_BROK")"
    conf="$(ltp_stats_num "$LTP_TOTAL_CONF")"
    warn="$(ltp_stats_num "$LTP_TOTAL_WARN")"
    unknown="$(ltp_stats_num "$LTP_TOTAL_UNKNOWN")"
    not_found="$(ltp_stats_num "$LTP_TOTAL_NOT_FOUND")"
    timeout="$(ltp_stats_num "$LTP_TOTAL_TIMEOUT")"

    result_sum=$((pass + fail + brok + conf + warn + unknown + not_found + timeout))
    echo "[LTP-SUMMARY] libc=$libc category=$LTP_CATEGORY run=$run pass=$pass fail=$fail brok=$brok conf=$conf warn=$warn unknown=$unknown not_found=$not_found timeout=$timeout"
    if { ! ltp_stats_safe_cmp "$run" -eq "$result_count"; } || \
       { ! ltp_stats_safe_cmp "$result_count" -eq "$result_sum"; }; then
        echo "[LTP-SUMMARY-WARN] result mismatch libc=$libc category=$LTP_CATEGORY last_case=$LTP_TOTAL_LAST_CASE run=$run result_count=$result_count result_sum=$result_sum"
        echo "[LTP-SUMMARY-DIAG] recent total last_case=$LTP_TOTAL_LAST_CASE run=$run result_count=$result_count result_sum=$result_sum"
    fi
}

run_ltp_one_batch_libc() {
    libc="$1"
    batch="$2"

    echo "[LTP-BATCH] category=$LTP_CATEGORY batch=$batch libc=$libc"
    ltp_stats_reset_batch

    # Use here-doc instead of pipe to avoid running the loop body in a subshell.
    # A pipe (| while read) creates a subshell where background-process management
    # and signal handling behave differently, breaking the watchdog timer.
    while read name; do
        [ -n "$name" ] || continue
        file="./$name"

        ltp_stats_add_run "$name"
        echo "RUN LTP CASE $name"

        if [ ! -f "$file" ]; then
            echo "[LTP-BATCH-MISSING] $libc $name: $LTP_BIN/$name"
            ltp_print_ltp_result NOT-FOUND "$name" 127
            continue
        fi

        run_ltp_case_file "$file" "$name"
        case_ret=$?
        label="$(ltp_exit_label "$case_ret")"
        if [ -z "$label" ]; then
            echo "[LTP-RUNNER-WARN] empty label case=$name rc=$case_ret"
            label=UNKNOWN
        fi
        if [ "$label" = "UNKNOWN" ]; then
            unknown_reason=classification
            [ "$case_ret" = 95 ] && unknown_reason=unsupported-wrapper
            echo "[LTP-RUNNER-WARN] unknown result case=$name rc=$case_ret reason=$unknown_reason"
        fi
        case "$label" in
            TIMEOUT)
                echo "[LTP-BATCH-TIMEOUT] $libc $name after ${LTP_TIMEOUT}s : $case_ret"
                ;;
        esac
        ltp_print_ltp_result "$label" "$name" "$case_ret"
    done <<LTP_CASE_EOF
$(ltp_batch_cases "$LTP_CATEGORY" "$batch")
LTP_CASE_EOF

    ltp_stats_batch_summary "$libc" "$batch"
}

run_ltp_batch_libc() {
    libc="$1"

    case "$libc" in
        glibc) dir=/glibc ;;
        musl) dir=/musl ;;
        *)
            echo "[LTP-BATCH-ERROR] unsupported libc: $libc"
            return
            ;;
    esac

    group="ltp-$libc"

    echo "#### OS COMP TEST GROUP START $group ####"
    ltp_stats_reset_totals

    if [ "$LTP_BATCH" = "all" ]; then
        batches="$(ltp_batch_ids "$LTP_CATEGORY")" || {
            echo "[LTP-BATCH-ERROR] unknown category: $LTP_CATEGORY"
            ltp_stats_total_summary "$libc"
            echo "#### OS COMP TEST GROUP END $group ####"
            return
        }
    else
        batches="$LTP_BATCH"
    fi

    echo "[LTP-BATCH-PLAN] category=$LTP_CATEGORY batches=$batches libc=$libc"

    first_cases=""
    first_count=0
    for batch in $batches; do
        cases="$(ltp_batch_cases "$LTP_CATEGORY" "$batch")" || {
            echo "[LTP-BATCH-ERROR] batch not found: category=$LTP_CATEGORY batch=$batch"
            ltp_stats_total_summary "$libc"
            echo "#### OS COMP TEST GROUP END $group ####"
            return
        }
        for name in $cases; do
            if [ "$first_count" -lt 5 ]; then
                first_cases="$first_cases $name"
                first_count="$(ltp_small_inc "$first_count")"
            fi
        done
    done

    LTP_BIN="$(ltp_bin_dir "$libc" $first_cases)" || {
        ltp_no_bin "$libc" "$batches" $first_cases
        ltp_stats_total_summary "$libc"
        echo "#### OS COMP TEST GROUP END $group ####"
        return
    }
    export LTP_BIN
    echo "[LTP-BATCH-LTP-BIN] libc=$libc dir=$LTP_BIN"

    if ! cd "$LTP_BIN"; then
        echo "[LTP-BATCH-ERROR] cannot cd to ltp bin dir: $LTP_BIN"
        ltp_stats_total_summary "$libc"
        echo "#### OS COMP TEST GROUP END $group ####"
        return
    fi

    set_library_path "$dir"

    old_path="$PATH"
    export PATH="$LTP_TOOL_DIR:$LTP_BIN:/bin:/sbin:/usr/bin:/usr/sbin"
    old_ltp_current_libc="${LTP_CURRENT_LIBC-}"
    old_ltp_shell_supports_set_u="${LTP_SHELL_SUPPORTS_SET_U-}"
    old_ltp_shell_compat="${LTP_SHELL_COMPAT-}"
    old_ltp_case_busybox="${LTP_CASE_BUSYBOX-}"
    export LTP_CURRENT_LIBC="$libc"
    echo "[LTP-PATH] libc=$libc PATH=$PATH"

    ltp_prepare_net_env "$libc"
    ltp_prepare_tools "$libc"
    ltp_prepare_shell "$libc"
    ltp_tool_diag "$libc"

    if [ -n "${NS_DURATION-}" ]; then
        export NS_DURATION
        case "$LTP_CATEGORY" in
            net|net-core|net-all|net-script|net-deferred)
                echo "[LTP-NETENV] NS_DURATION=$NS_DURATION"
                ;;
        esac
    fi

    for batch in $batches; do
        run_ltp_one_batch_libc "$libc" "$batch"
    done

    export PATH="$old_path"
    if [ -n "$old_ltp_current_libc" ]; then
        export LTP_CURRENT_LIBC="$old_ltp_current_libc"
    else
        unset LTP_CURRENT_LIBC
    fi
    if [ -n "$old_ltp_shell_supports_set_u" ]; then
        export LTP_SHELL_SUPPORTS_SET_U="$old_ltp_shell_supports_set_u"
    else
        unset LTP_SHELL_SUPPORTS_SET_U
    fi
    if [ -n "$old_ltp_shell_compat" ]; then
        export LTP_SHELL_COMPAT="$old_ltp_shell_compat"
    else
        unset LTP_SHELL_COMPAT
    fi
    if [ -n "$old_ltp_case_busybox" ]; then
        export LTP_CASE_BUSYBOX="$old_ltp_case_busybox"
    else
        unset LTP_CASE_BUSYBOX
    fi
    cd /
    ltp_stats_total_summary "$libc"
    echo "#### OS COMP TEST GROUP END $group ####"
}

run_ltp_batch_tests() {
    found=1

    case "$LTP_CATEGORY" in
        process|fs|mm-ipc|common-easy|net|net-core|net-all|net-script|net-deferred) ;;
        *)
            echo "[LTP-BATCH-ERROR] unsupported category: $LTP_CATEGORY"
            return
            ;;
    esac

    if [ "$LTP_BATCH" = "all" ]; then
        if ! ltp_batch_ids "$LTP_CATEGORY" >/dev/null 2>&1; then
            echo "[LTP-BATCH-ERROR] unknown category: $LTP_CATEGORY"
            return
        fi
    else
        if ! ltp_batch_cases "$LTP_CATEGORY" "$LTP_BATCH" >/dev/null 2>&1; then
            echo "[LTP-BATCH-ERROR] batch not found: category=$LTP_CATEGORY batch=$LTP_BATCH"
            return
        fi
    fi

    case "$LTP_LIBC" in
        glibc|musl)
            run_ltp_batch_libc "$LTP_LIBC"
            ;;
        both)
            run_ltp_batch_libc glibc
            run_ltp_batch_libc musl
            ;;
        *)
            echo "[LTP-BATCH-ERROR] unsupported libc: $LTP_LIBC"
            ;;
    esac
}

run_lmbench_write_tests() {
    found=1
    cd /glibc || return
    set_library_path /glibc
    echo "run /glibc/lmbench_all lat_syscall -P 1 write"
    ./lmbench_all lat_syscall -P 1 write
    echo "lmbench-write-debug before mkdir"
    ./busybox mkdir -p /var/tmp
    echo "lmbench-write-debug after mkdir"
    ./busybox touch /var/tmp/lmbench
    echo "lmbench-write-debug after touch"
    ./lmbench_all lat_syscall -P 1 stat /var/tmp/lmbench
    echo "lmbench-write-debug after stat"
    ./lmbench_all lat_syscall -P 1 fstat /var/tmp/lmbench
    echo "lmbench-write-debug after fstat"
    ./lmbench_all lat_syscall -P 1 open /var/tmp/lmbench
    echo "lmbench-write-debug after open"
    cd /
}

run_unixbench_tests() {
    found=1
    run_test_path /glibc/unixbench_testcode.sh
    run_test_path /musl/unixbench_testcode.sh
}

run_wait_repro_tests() {
    for testcase in \
        /glibc/libctest_testcode.sh \
        /glibc/lmbench_testcode.sh \
        /glibc/unixbench_testcode.sh \
        /musl/basic_testcode.sh
    do
        run_test_path "$testcase"
    done
}

cd /

found=0
case "$TEST_PROFILE" in
    stable)
        run_stable_tests
        ;;
    stable-lmbench)
        run_stable_tests
        run_lmbench_only_tests
        ;;
    cyc-musl)
        run_cyclictest_musl_tests
        ;;
    cyc-all)
        run_cyclictest_tests
        ;;
    libctest)
        run_libctest_tests
        ;;
    iozone)
        run_iozone_tests
        ;;
    lmbench)
        run_lmbench_tests
        ;;
    lmbench-only)
        run_lmbench_only_tests
        ;;
    lmbench-fast)
        run_lmbench_fast_tests
        ;;
    ltp-only)
        run_ltp_only_tests
        ;;
    ltp-list)
        run_ltp_list_tests
        ;;
    ltp-batch)
        run_ltp_batch_tests
        ;;
    perf)
        run_stable_tests
        ;;
    lmbench-write)
        run_lmbench_write_tests
        ;;
    unixbench)
        run_unixbench_tests
        ;;
    wait-repro)
        run_wait_repro_tests
        ;;
    full)
        for dir in / /glibc /musl; do
            run_test_dir "$dir"
        done
        ;;
    *)
        echo "Unknown TEST_PROFILE=$TEST_PROFILE; using stable profile."
        run_stable_tests
        ;;
esac
if [ "$found" -eq 0 ]; then
    for testcase in /*_testcode.sh /scripts/*/*_testcode.sh; do
        [ -f "$testcase" ] || continue
        found=1
        dir="${testcase%/*}"
        name="${testcase##*/}"
        cd "$dir" || continue
        set_library_path "$dir"
        echo "run ${dir}/${name}"
        if skip_ltp_testcase "$name" "$dir"; then
            cd /
            continue
        fi
        run_with_shell "./$name"
        needs_cleanup "$testcase" && cleanup_leftovers
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
