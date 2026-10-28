#!/bin/sh
# Regenerate the /agora/ skeleton and browser script using the pinned arm64 omc.
# Exit 2: missing compiler; 3: wrong pin; 4: stale artifact rewritten;
# 5: nondeterministic output; 6: empty output. Compile/run failures propagate.
set -eu
cd "$(dirname "$0")"
omc=${OMEGA_OMC:-$PWD/../hive-page/omc}
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
work=$(mktemp -d "${TMPDIR:-/tmp}/agora-page.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
cat contract.om agora.om agora-js.om > "$work/agora.om"
"$omc" < "$work/agora.om" > "$work/agora"
chmod +x "$work/agora"
"$work/agora" > "$work/first.html" 3> "$work/first.agora.js"
"$work/agora" > "$work/second.html" 3> "$work/second.agora.js"
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
publish "$work/first.agora.js" "$work/second.agora.js" ../../public/js/agora.js agora.js.sha256
printf 'omc sha256 %s\n' "$have"
if [ "$stale" = 1 ]; then
    printf 'Generated artifacts were rewritten; commit them with their Omega sources\n' >&2
    if [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
        exit 4
    fi
fi
