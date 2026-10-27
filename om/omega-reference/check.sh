#!/bin/sh
# Host orchestration only. page.om (seeded Omega) writes main.html from the
# vendored omega-harness guide and examples; this script checks the inputs,
# gates every example with the pinned omc and publishes the page.
#   check.sh            full check; needs OMEGA_OMC, the omc pinned in vendor.pin
#   check.sh --hashes   inputs and output against their recorded hashes only
# Optional OMEGA_HARNESS=<omega-harness clone> also compares vendor/ with
# git archive of the pinned commit.
set -eu
cd "$(dirname "$0")"
vendor_ok() {
    (cd vendor && sha256sum --quiet -c ../vendor.sha256) || { echo 'vendor/ differs from vendor.sha256' >&2; exit 3; }
    [ "$(find vendor -type f | wc -l)" -eq "$(wc -l < vendor.sha256)" ] || { echo 'vendor/ holds a file vendor.sha256 does not list' >&2; exit 3; }
}
vendor_ok
if [ "${1:-}" = --hashes ]; then
    sha256sum --quiet -c main.html.sha256
    printf 'OMEGA-REFERENCE HASHES PASS files=%s main=%s\n' "$(wc -l < vendor.sha256)" "$(cut -d' ' -f1 main.html.sha256)"
    exit 0
fi
[ $# -eq 0 ] || { echo 'usage: check.sh [--hashes]' >&2; exit 2; }
omc=${OMEGA_OMC:?set OMEGA_OMC to the omc pinned in vendor.pin}
pin=$(awk '$1 == "omc-sha256" { print $2 }' vendor.pin)
commit=$(awk '$1 == "omega-harness" { print $2 }' vendor.pin)
[ "$(sha256sum "$omc" | cut -d' ' -f1)" = "$pin" ] || { echo 'Compiler pin mismatch' >&2; exit 3; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
if [ -n "${OMEGA_HARNESS:-}" ]; then
    mkdir "$work/h"
    git -C "$OMEGA_HARNESS" archive "$commit" examples skills/writing-omega/SKILL.md skills/omega-encoding/examples | tar -x -C "$work/h"
    rm -f "$work/h/examples/WOLFRAM.md"
    diff -r "$work/h" vendor > /dev/null || { echo "vendor/ differs from omega-harness $commit" >&2; exit 3; }
    echo "vendor/ equals omega-harness $commit"
fi
# Every worked example on the page: compiled and run by the harness gate.
bash vendor/examples/gate.sh "$omc" > "$work/gate.log" 2>&1 || { cat "$work/gate.log" >&2; exit 5; }
gate=$(grep '^GATE PASS' "$work/gate.log")
echo "$gate"
# Negative control: one changed expected exit code must fail the gate.
cp -R vendor "$work/mut"
echo 9 > "$work/mut/examples/hello.exit"
if bash "$work/mut/examples/gate.sh" "$omc" > "$work/mut.log" 2>&1; then
    echo 'Mutated example wrongly passed the gate' >&2; exit 5
fi
echo "negative control: $(grep '^GATE FAIL' "$work/mut.log")"
# Error 24 has no example file: the recipe in the guide, run at both sizes.
e24() {
    { printf 'fn main to int\nx = ulong 0\nc = true\nwhile c bound 1\n'
      yes 'x = add x 1' | head -n "$1"
      printf 'c = false\nend\nret int and x 1\nend\n'; } > "$work/e24.om"
    rc=0; "$omc" < "$work/e24.om" > "$work/e24" 2> "$work/e24.err" || rc=$?
}
e24 270000
[ "$rc" = 24 ] && [ "$(cat "$work/e24.err")" = 'omc: line 270006: error 24' ] || { echo "error 24 recipe: rc $rc" >&2; exit 5; }
e24 140000
[ "$rc" = 0 ] || { echo "error 24 recipe, short loop: rc $rc" >&2; exit 5; }
echo 'error 24 recipe: 270000 lines refused with error 24 at line 270006, 140000 lines compiled'
"$omc" < page.om > "$work/page"
chmod +x "$work/page"
"$work/page" > "$work/first.html"
"$work/page" > "$work/second.html"
cmp "$work/first.html" "$work/second.html"
[ -s "$work/first.html" ]
# Negative control: a missing pin must refuse, not publish links to no commit.
mkdir "$work/nopin"
cp vendor.sha256 "$work/nopin/"
ln -s "$PWD/vendor" "$work/nopin/vendor"
if (cd "$work/nopin" && "$work/page" > refused.html 2>/dev/null); then
    echo 'Missing vendor.pin wrongly accepted' >&2; exit 5
fi
stale=0
if [ ! -f main.html ] || ! cmp -s "$work/first.html" main.html; then stale=1; fi
cp "$work/first.html" main.html
sha256sum main.html > main.html.sha256
if [ "$stale" = 1 ] && [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
    echo 'main.html changed; commit it with its Omega source and vendor/' >&2; exit 4
fi
printf 'OMEGA-REFERENCE PASS compiler=%s harness=%s main=%s\n' "$pin" "$commit" "$(cut -d' ' -f1 main.html.sha256)"
