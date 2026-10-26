#!/bin/sh
# Regenerate Allen's startup sound (public/audio/allen-chirp.wav) and its browser
# controller (public/js/allen-audio.js) with the pinned arm64 omc, then, when an
# aienux clone is given, check the Omega synthesis byte for byte against the
# original C it ports.
#
# The compiler is the one vendored for the Hive page; its pin is recorded once,
# in ../hive-page/omc.sha256. OMEGA_OMC overrides the compiler path, never its pin.
# OMEGA_ACCEPT_CHANGE=1 accepts rewritten artifacts; otherwise stale output exits 4.
#
# Oracle (AIENUX_SRC=<aienux clone holding commit $PIN>): extracts native/kernel/ui
# at $PIN, checks the blob ids of the sound's sources, builds aienux's own host
# test ui/tests/chirp_test.c with chirp.c (HOSTCC, default cc), and requires
#   1. its chirp.wav to equal native.om's output byte for byte (whole night,
#      stereo, 590400 frames);
#   2. the web asset's samples to equal that file's left channel over frames
#      545280..590400, and right to equal left there;
#   3. a mutant oracle (CK_CHIRP_PEAK 22937 -> 22936) to differ (negative control).
# Without AIENUX_SRC the oracle prints ORACLE NOT_RUN; that is not a pass.
#
# Exit 2: missing compiler; 3: wrong pin; 4: stale artifact rewritten;
# 5: nondeterministic output; 6: empty output; 7: oracle mismatch;
# 8: negative control accepted; 9: oracle source missing or not the pinned blobs.
# Compile/run failures propagate.
set -eu
cd "$(dirname "$0")"
PIN=da55660ea7337f2b6a42095fcd1fb6ddebe1cc7c
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
work=$(mktemp -d "${TMPDIR:-/tmp}/allen-audio.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
# Concatenation composes Omega units; it does not implement generation in shell.
cat chirp.om asset.om > "$work/asset.om"
cat chirp.om native.om > "$work/native.om"
"$omc" < "$work/asset.om" > "$work/asset"
"$omc" < "$work/native.om" > "$work/native"
chmod +x "$work/asset" "$work/native"
"$work/asset" > "$work/first.wav" 3> "$work/first.js"
"$work/asset" > "$work/second.wav" 3> "$work/second.js"
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
mkdir -p ../../public/audio
publish "$work/first.wav" "$work/second.wav" ../../public/audio/allen-chirp.wav allen-chirp.wav.sha256
publish "$work/first.js" "$work/second.js" ../../public/js/allen-audio.js allen-audio.js.sha256
printf 'omc sha256 %s\n' "$have"

if [ -z "${AIENUX_SRC:-}" ]; then
    printf 'ORACLE NOT_RUN: set AIENUX_SRC to an aienux clone holding %s\n' "$PIN"
else
    if ! git -C "$AIENUX_SRC" cat-file -e "$PIN^{commit}" 2>/dev/null; then
        printf 'ORACLE: %s has no commit %s\n' "$AIENUX_SRC" "$PIN" >&2
        exit 9
    fi
    for want in \
        d3e732908a72a372816584d0fe3d62a3db256f1b:native/kernel/ui/chirp.c \
        359f0f4e63e5c194b005906d3bd7b03cfb7e8703:native/kernel/ui/chirp.h \
        4cecf8f2afcd8828915a87cefe2cbefab5ef7251:native/kernel/ui/night.h \
        7032fee2ca584587981297ffa22ade4c72560a76:native/kernel/ui/tests/chirp_test.c; do
        blob=${want%%:*}
        path=${want#*:}
        if [ "$(git -C "$AIENUX_SRC" rev-parse "$PIN:$path")" != "$blob" ]; then
            printf 'ORACLE: %s at %s is not blob %s\n' "$path" "$PIN" "$blob" >&2
            exit 9
        fi
    done
    mkdir "$work/src" "$work/out" "$work/mut" "$work/mutout"
    git -C "$AIENUX_SRC" archive "$PIN" native/kernel/ui | tar -x -C "$work/src"
    ui=$work/src/native/kernel/ui
    cc=${HOSTCC:-cc}
    "$cc" -std=c11 -O2 -Wall -Wextra -Werror -o "$work/oracle" "$ui/tests/chirp_test.c" "$ui/chirp.c"
    "$work/oracle" "$work/out"
    "$work/native" > "$work/native.wav"
    if ! cmp "$work/out/chirp.wav" "$work/native.wav"; then
        printf 'ORACLE FAIL: Omega night differs from aienux chirp_test.c chirp.wav\n' >&2
        exit 7
    fi
    # Frame 545280 of the stereo file starts at byte 44 + 545280 * 4; 45120 frames follow.
    od -An -v -td2 -w4 -j 2181164 -N 180480 "$work/out/chirp.wav" > "$work/pairs.txt"
    od -An -v -td2 -w2 -j 44 ../../public/audio/allen-chirp.wav > "$work/mono.txt"
    awk '{ print $1 }' "$work/pairs.txt" > "$work/left.txt"
    awk '$1 != $2 { bad = 1 } END { exit bad }' "$work/pairs.txt" || {
        printf 'ORACLE FAIL: left and right differ in the chirp\n' >&2
        exit 7
    }
    awk '{ print $1 }' "$work/mono.txt" > "$work/mono1.txt"
    if ! cmp -s "$work/left.txt" "$work/mono1.txt"; then
        printf 'ORACLE FAIL: allen-chirp.wav is not the oracle left channel at frames 545280..590400\n' >&2
        exit 7
    fi
    # Negative control: the same oracle with the peak one step lower must not match.
    git -C "$AIENUX_SRC" archive "$PIN" native/kernel/ui | tar -x -C "$work/mut"
    mui=$work/mut/native/kernel/ui
    sed 's/^#define CK_CHIRP_PEAK 22937 /#define CK_CHIRP_PEAK 22936 /' "$mui/chirp.h" > "$work/chirp.h"
    if cmp -s "$work/chirp.h" "$mui/chirp.h"; then
        printf 'NEGATIVE CONTROL: the mutation did not apply\n' >&2
        exit 8
    fi
    cp "$work/chirp.h" "$mui/chirp.h"
    "$cc" -std=c11 -O2 -Wall -Wextra -Werror -o "$work/mutant" "$mui/tests/chirp_test.c" "$mui/chirp.c"
    "$work/mutant" "$work/mutout" > /dev/null || true
    if cmp -s "$work/mutout/chirp.wav" "$work/native.wav"; then
        printf 'NEGATIVE CONTROL FAIL: a mutated oracle also matched\n' >&2
        exit 8
    fi
    printf 'ORACLE PASS: aienux %s chirp_test.c chirp.wav == Omega night (%s bytes); web asset == its left channel, frames 545280..590400; mutant oracle differs\n' \
        "$PIN" "$(wc -c < "$work/native.wav" | tr -d ' ')"
fi

if [ "$stale" = 1 ]; then
    printf 'Generated artifacts were rewritten; commit them with their Omega sources\n' >&2
    if [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
        exit 4
    fi
fi
