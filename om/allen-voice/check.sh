#!/bin/sh
# Regenerate Allen's voice controller (public/js/allen-voice.js) with the pinned
# arm64 omc, then run its behaviour test with node and two negative controls.
# The compiler is the one vendored for the Hive page; its pin is recorded once,
# in ../hive-page/omc.sha256. OMEGA_OMC overrides the compiler path, never its pin.
# OMEGA_ACCEPT_CHANGE=1 accepts a rewritten artifact; otherwise stale output exits 4.
# Without node the test prints TEST NOT_RUN; that is not a pass.
# Exit 2: missing compiler; 3: wrong pin; 4: stale artifact rewritten;
# 5: nondeterministic output; 6: empty output; 7: behaviour test failed;
# 8: a negative control passed. Compile/run failures propagate.
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
work=$(mktemp -d "${TMPDIR:-/tmp}/allen-voice.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
"$omc" < voice.om > "$work/voice"
chmod +x "$work/voice"
"$work/voice" > "$work/first.js"
"$work/voice" > "$work/second.js"
if ! cmp -s "$work/first.js" "$work/second.js"; then
    printf 'allen-voice.js: generator output differs between runs\n' >&2
    exit 5
fi
if [ ! -s "$work/first.js" ]; then
    printf 'allen-voice.js: generator wrote nothing\n' >&2
    exit 6
fi
artifact=../../public/js/allen-voice.js
stale=0
if [ ! -f "$artifact" ] || ! cmp -s "$artifact" "$work/first.js"; then
    stale=1
fi
cp "$work/first.js" "$artifact"
hash=$(sha256sum "$artifact" | cut -d' ' -f1)
printf '%s  %s\n' "$hash" "$artifact" > allen-voice.js.sha256
printf '%s %s bytes sha256 %s\n' "$artifact" "$(wc -c < "$artifact" | tr -d ' ')" "$hash"
printf 'omc sha256 %s\n' "$have"

if ! command -v node > /dev/null 2>&1; then
    printf 'TEST NOT_RUN: node is not installed\n'
else
    if ! node test-voice.cjs "$artifact"; then
        printf 'TEST FAIL: %s\n' "$artifact" >&2
        exit 7
    fi
    # Negative controls: a controller that sends recognized words itself, and
    # one that reads replies aloud by default, must both fail the test.
    sed 's/input.dispatchEvent(new Event(.input., { bubbles: true }));/&\n    if (window.allen) window.allen.send(value);/' "$artifact" > "$work/autosend.js"
    sed 's/speaking = stored();/speaking = true;/' "$artifact" > "$work/speakon.js"
    for mutant in autosend speakon; do
        if cmp -s "$artifact" "$work/$mutant.js"; then
            printf 'NEGATIVE CONTROL %s: the mutation did not apply\n' "$mutant" >&2
            exit 8
        fi
        if node test-voice.cjs "$work/$mutant.js" > "$work/$mutant.log" 2>&1; then
            printf 'NEGATIVE CONTROL FAIL: the %s mutant passed the test\n' "$mutant" >&2
            exit 8
        fi
        printf 'negative control %s: rejected (%s)\n' "$mutant" "$(grep -m1 -o 'AssertionError[^\n]*' "$work/$mutant.log" || printf 'test failed')"
    done
fi

if [ "$stale" = 1 ]; then
    printf 'Generated artifact was rewritten; commit it with its Omega source\n' >&2
    if [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
        exit 4
    fi
fi
