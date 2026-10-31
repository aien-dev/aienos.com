#!/bin/sh
# Regenerate the /aien/ universe controller using the pinned arm64 omc.
# The compiler is the one vendored for the Hive page; its pin is recorded once,
# in ../hive-page/omc.sha256. ../allen-page/contract.om is the shared /aien/
# contract: the ids and routes this controller names come from its fields.
# CI on x86_64 checks the committed artifact against its recorded hash.
# OMEGA_OMC overrides the compiler path, never its pin.
# OMEGA_ACCEPT_CHANGE=1 accepts a rewritten artifact; otherwise stale output exits 4.
# Exit 2: missing compiler; 3: wrong pin; 4: stale artifact rewritten;
# 5: nondeterministic output; 6: empty output. Compile/run failures propagate.
set -eu
cd "$(dirname "$0")"
omc=${OMEGA_OMC:-$PWD/../hive-page/omc}
pin=$(tr -d ' \n' < ../hive-page/omc.sha256)
if [ ! -f "$omc" ]; then
    printf 'No compiler at %s; set OMEGA_OMC to the pinned omc (%s)\n' "$omc" "$pin" >&2
    exit 2
fi
have=$(sha256sum "$omc" | cut -d' ' -f1)
if [ "$have" != "$pin" ]; then
    printf 'Compiler sha256 mismatch: recorded %s, found %s\n' "$pin" "$have" >&2
    exit 3
fi
work=$(mktemp -d "${TMPDIR:-/tmp}/aien-universe.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
# Concatenation composes Omega units; it does not implement generation in shell.
cat ../allen-page/contract.om universe.om > "$work/universe.om"
"$omc" < "$work/universe.om" > "$work/universe"
chmod +x "$work/universe"
"$work/universe" > "$work/first.js"
"$work/universe" > "$work/second.js"
if ! cmp -s "$work/first.js" "$work/second.js"; then
    printf 'aien-universe.js: generator output differs between runs\n' >&2
    exit 5
fi
if [ ! -s "$work/first.js" ]; then
    printf 'aien-universe.js: generator wrote nothing\n' >&2
    exit 6
fi
artifact=../../public/js/aien-universe.js
stale=0
if [ ! -f "$artifact" ] || ! cmp -s "$artifact" "$work/first.js"; then
    stale=1
fi
cp "$work/first.js" "$artifact"
hash=$(sha256sum "$artifact" | cut -d' ' -f1)
printf '%s  %s\n' "$hash" "$artifact" > aien-universe.js.sha256
printf '%s %s bytes sha256 %s\n' "$artifact" "$(wc -c < "$artifact" | tr -d ' ')" "$hash"
printf 'omc sha256 %s\n' "$have"
if [ "$stale" = 1 ]; then
    printf 'Generated artifact was rewritten; commit it with its Omega sources\n' >&2
    if [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
        exit 4
    fi
fi
