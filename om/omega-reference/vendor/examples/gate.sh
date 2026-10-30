#!/usr/bin/env bash
# gate.sh OMC: compile and run every worked example against its expected result.
#
# Covers examples/*.om and skills/*/examples/*.om. Each NAME.om needs NAME.expect:
#   compile N         required; the compiler's exit status
#   stderr LINE       required when N is not 0; the compiler's whole stderr is exactly LINE
#   exit N            required when N is 0; the program's exit status
#   input dir         optional; the compiler reads a directory instead of NAME.om (error 1)
#   output full       optional; the compiler writes to /dev/full (error 2)
# Optional fixtures: NAME.stdout (exact stdout bytes, absent means empty), NAME.stdin (piped in,
# so reads can be partial), NAME.args (one argument per line), NAME.env (one KEY=VALUE per line).
# Every program runs with `env -i` plus NAME.env, in an empty scratch directory, under a 20 s
# timeout, and must write nothing to stderr. A compile that is not expected to succeed is never run.
#
# The compiler must match the pinned sha256 below; a different binary is refused, not tolerated.
# Exit 0 only when every example passes; 1 on any failure; 2 on a usage or pin error.
set -u

PIN_REV=e5b32567a7aff3ce594e3254a622a846a6cd190b
PIN_SHA=2c82480596bbdc50cda9626866209ef662d5834f278f839ff4ce0250c495029f

if [ "$#" -ne 1 ]; then
    echo "usage: $0 /path/to/omc   (omega-mv $PIN_REV seed/out/omc)" >&2
    exit 2
fi
omc=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
if [ ! -x "$omc" ]; then
    echo "GATE FAIL $1 is not an executable file" >&2
    exit 2
fi
got=$(sha256sum "$omc" | cut -d' ' -f1)
if [ "$got" != "$PIN_SHA" ]; then
    echo "GATE FAIL omc sha256 $got, pinned $PIN_SHA (omega-mv $PIN_REV)" >&2
    exit 2
fi

here=$(cd "$(dirname "$0")" && pwd)
root=$(dirname "$here")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

pass=0
fail=0
total=0

bad() {
    echo "FAIL $name: $*"
    fail=$((fail + 1))
}

# Expected files without a program are a silent loss of coverage, so they fail too.
for f in "$root"/examples/*.expect "$root"/skills/*/examples/*.expect; do
    [ -e "$f" ] || continue
    if [ ! -e "${f%.expect}.om" ]; then
        name=$(basename "${f%.expect}")
        total=$((total + 1))
        bad "expect file without $name.om"
    fi
done

for src in "$root"/examples/*.om "$root"/skills/*/examples/*.om; do
    [ -e "$src" ] || continue
    base=${src%.om}
    name=${base#"$root"/}
    total=$((total + 1))
    if [ ! -f "$base.expect" ]; then
        bad "no $name.expect"
        continue
    fi
    want_compile= want_stderr= want_exit= input=file output=file
    while IFS= read -r line || [ -n "$line" ]; do
        case $line in
            "compile "*) want_compile=${line#compile } ;;
            "stderr "*) want_stderr=${line#stderr } ;;
            "exit "*) want_exit=${line#exit } ;;
            "input dir") input=dir ;;
            "output full") output=full ;;
            "") ;;
            *) want_compile=invalid ;;
        esac
    done < "$base.expect"
    case $want_compile in
        ''|invalid|*[!0-9]*) bad "malformed $name.expect"; continue ;;
    esac
    if [ "$want_compile" != 0 ] && [ -z "$want_stderr" ]; then
        bad "$name.expect has no stderr line"; continue
    fi
    if [ "$want_compile" = 0 ]; then
        case $want_exit in
            ''|*[!0-9]*) bad "$name.expect has no exit line"; continue ;;
        esac
    fi

    d=$work/$(echo "$name" | tr / _)
    mkdir -p "$d/run"
    in=$src
    [ "$input" = dir ] && in=$d/run
    out=$d/p
    [ "$output" = full ] && out=/dev/full
    "$omc" < "$in" > "$out" 2> "$d/cerr"
    rc=$?
    if [ "$rc" != "$want_compile" ]; then
        bad "compile exit $rc, expected $want_compile: $(cat "$d/cerr")"
        continue
    fi
    if [ "$want_compile" != 0 ]; then
        if printf '%s\n' "$want_stderr" | cmp -s - "$d/cerr"; then
            echo "PASS $name (compile $rc: $want_stderr)"
            pass=$((pass + 1))
        else
            bad "compiler stderr '$(cat "$d/cerr")', expected '$want_stderr'"
        fi
        continue
    fi
    if [ -s "$d/cerr" ]; then
        bad "compiler stderr not empty: $(cat "$d/cerr")"
        continue
    fi

    chmod +x "$d/p"
    args=()
    envs=()
    if [ -f "$base.args" ]; then mapfile -t args < "$base.args"; fi
    if [ -f "$base.env" ]; then mapfile -t envs < "$base.env"; fi
    if [ -f "$base.stdin" ]; then
        (cd "$d/run" && cat "$base.stdin" | timeout 20 env -i "${envs[@]}" "$d/p" "${args[@]}" > "$d/out" 2> "$d/rerr")
    else
        (cd "$d/run" && timeout 20 env -i "${envs[@]}" "$d/p" "${args[@]}" < /dev/null > "$d/out" 2> "$d/rerr")
    fi
    rc=$?
    want_out=/dev/null
    [ -f "$base.stdout" ] && want_out=$base.stdout
    if [ "$rc" != "$want_exit" ]; then
        bad "exit $rc, expected $want_exit"
    elif ! cmp -s "$want_out" "$d/out"; then
        bad "stdout differs from $(basename "$want_out"): $(od -An -c "$d/out" | tr -s ' ' | tr -d '\n' | cut -c1-200)"
    elif [ -s "$d/rerr" ]; then
        bad "program stderr not empty: $(cat "$d/rerr")"
    else
        echo "PASS $name (exit $rc, $(wc -c < "$d/out" | tr -d ' ') stdout bytes)"
        pass=$((pass + 1))
    fi
done

if [ "$total" = 0 ]; then
    echo "GATE FAIL no examples found under $root"
    exit 1
fi
if [ "$fail" = 0 ] && [ "$pass" = "$total" ]; then
    echo "GATE PASS examples=$total pass=$pass fail=0 omc=$PIN_SHA"
    exit 0
fi
echo "GATE FAIL examples=$total pass=$pass fail=$fail omc=$got"
exit 1
