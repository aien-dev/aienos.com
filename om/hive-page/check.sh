#!/bin/sh
# Regenerate the Hive skeleton and browser scripts using the pinned arm64 omc.
# CI on x86_64 checks the committed artifacts against their recorded hashes.
# OMEGA_OMC overrides the compiler path, never its pin.
# OMEGA_ACCEPT_CHANGE=1 accepts rewritten artifacts; otherwise stale output exits 4.
# Exit 2: missing compiler; 3: wrong pin; 4: stale artifact rewritten;
# 5: nondeterministic output; 6: empty output. Compile/run failures propagate.
set -eu
cd "$(dirname "$0")"
omc=${OMEGA_OMC:-$PWD/omc}
pin=$(tr -d ' \n' < omc.sha256)
if [ ! -f "$omc" ]; then
    printf 'No compiler at %s; set OMEGA_OMC to the pinned omc (%s)\n' "$omc" "$pin" >&2
    exit 2
fi
have=$(sha256sum "$omc" | cut -d' ' -f1)
if [ "$have" != "$pin" ]; then
    printf 'Compiler sha256 mismatch: recorded %s, found %s\n' "$pin" "$have" >&2
    exit 3
fi
work=$(mktemp -d "${TMPDIR:-/tmp}/hive-page.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
# One executable shares the contract between markup (fd 1) and hydrator (fd 3).
# Concatenation composes Omega units; it does not implement generation in shell.
cat contract.om hive.om hive-js.om > "$work/hive.om"
"$omc" < "$work/hive.om" > "$work/hive"
"$omc" < turing-look.om > "$work/turing"
chmod +x "$work/hive" "$work/turing"
"$work/hive" > "$work/first.html" 3> "$work/first.hive.js"
"$work/hive" > "$work/second.html" 3> "$work/second.hive.js"
"$work/turing" > "$work/first.turing.js"
"$work/turing" > "$work/second.turing.js"
stale=0
publish() {
    first=$1
    second=$2
    artifact=$3
    sidecar=$4
    if ! cmp -s "$first" "$second"; then
        printf '%s: generator output differs between runs\n' "$artifact" >&2
        exit 5
    fi
    if [ ! -s "$first" ]; then
        printf '%s: generator wrote nothing\n' "$artifact" >&2
        exit 6
    fi
    if [ ! -f "$artifact" ] || ! cmp -s "$artifact" "$first"; then
        stale=1
    fi
    cp "$first" "$artifact"
    hash=$(sha256sum "$artifact" | cut -d' ' -f1)
    printf '%s  %s\n' "$hash" "$artifact" > "$sidecar"
    printf '%s %s bytes sha256 %s\n' "$artifact" "$(wc -c < "$artifact" | tr -d ' ')" "$hash"
}
publish "$work/first.html" "$work/second.html" main.html main.html.sha256
publish "$work/first.hive.js" "$work/second.hive.js" ../../public/js/hive.js hive.js.sha256
publish "$work/first.turing.js" "$work/second.turing.js" ../../public/js/turing-look.js turing-look.js.sha256
printf 'omc sha256 %s\n' "$have"
if [ "$stale" = 1 ]; then
    printf 'Generated artifacts were rewritten; commit them with their Omega sources\n' >&2
    if [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
        exit 4
    fi
fi
