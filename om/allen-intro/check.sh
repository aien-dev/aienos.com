#!/bin/sh
# Regenerate Allen's consent aside (intro.html) from intro.om with the pinned arm64 omc
# shared with om/hive-page. CI on x86_64 re-hashes the committed artifact in a Rust test.
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
work=$(mktemp -d "${TMPDIR:-/tmp}/allen-intro.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
"$omc" < intro.om > "$work/intro"
chmod +x "$work/intro"
"$work/intro" > "$work/first.html"
"$work/intro" > "$work/second.html"
if ! cmp -s "$work/first.html" "$work/second.html"; then
    printf 'intro.html: generator output differs between runs\n' >&2
    exit 5
fi
if [ ! -s "$work/first.html" ]; then
    printf 'intro.html: generator wrote nothing\n' >&2
    exit 6
fi
stale=0
if [ ! -f intro.html ] || ! cmp -s intro.html "$work/first.html"; then
    stale=1
fi
cp "$work/first.html" intro.html
hash=$(sha256sum intro.html | cut -d' ' -f1)
printf '%s  intro.html\n' "$hash" > intro.html.sha256
printf 'intro.html %s bytes sha256 %s\nomc sha256 %s\n' "$(wc -c < intro.html | tr -d ' ')" "$hash" "$have"
if [ "$stale" = 1 ]; then
    printf 'intro.html was rewritten; commit it with intro.om\n' >&2
    if [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
        exit 4
    fi
fi
