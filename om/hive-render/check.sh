#!/bin/sh
# Shell is build/test coordination only, not the renderer implementation.
set -eu
cd "$(dirname "$0")"
omc=${OMEGA_OMC:-../hive-page/omc}
expected=734f0a39319fbfa930bcc62cb30dbf06d47d78e126372b74f5baf478e6126b61
[ -x "$omc" ] || exit 2
[ "$(sha256sum "$omc" | cut -d ' ' -f1)" = "$expected" ] || exit 3
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
nice -n 19 "$omc" < render.om > "$scratch/one"
nice -n 19 "$omc" < render.om > "$scratch/two"
cmp "$scratch/one" "$scratch/two" || exit 5
[ -s "$scratch/one" ] || exit 6
chmod +x "$scratch/one"
python3 test-render.py "$scratch/one" "$omc" "$scratch"
sha256sum render.om > render.om.sha256
sha256sum "$scratch/one" | cut -d ' ' -f1 > hive-render.sha256
cp "$scratch/one" hive-render
printf '%s\n' 'HIVE-RENDER PASS: deterministic compiler output, escaper differential, schema refusals, XSS text and tree checks'
