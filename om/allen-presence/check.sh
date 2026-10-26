#!/bin/sh
# Build-only glue. The browser integration is authored in presence.om.
set -eu
cd "$(dirname "$0")"
omc=${OMEGA_OMC:-../hive-page/omc}
pin=$(tr -d ' \n' < ../hive-page/omc.sha256)
test "$(sha256sum "$omc" | cut -d' ' -f1)" = "$pin" || exit 3
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
"$omc" < presence.om > "$work/generate"
chmod +x "$work/generate"
"$work/generate" > "$work/first.js" 3> "$work/first.html"
"$work/generate" > "$work/second.js" 3> "$work/second.html"
[ -s "$work/first.js" ] || exit 6
cmp "$work/first.html" "$work/second.html"
cmp "$work/first.js" "$work/second.js"
artifact=../../public/js/allen-presence.js
if ! cmp -s "$work/first.js" "$artifact"; then
    [ "${OMEGA_ACCEPT_CHANGE:-0}" = 1 ] || { printf 'Presence artifact stale; regenerate with OMEGA_ACCEPT_CHANGE=1\n' >&2; exit 4; }
    cp "$work/first.js" "$artifact"
fi
if ! cmp -s "$work/first.html" launcher.html; then
    [ "${OMEGA_ACCEPT_CHANGE:-0}" = 1 ] || exit 4
    cp "$work/first.html" launcher.html
fi
sha256sum launcher.html > launcher.sha256
sha256sum "$artifact" > presence.sha256
printf 'Allen presence generator deterministic\n'
