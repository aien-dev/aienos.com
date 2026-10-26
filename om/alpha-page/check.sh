#!/bin/sh
# Host orchestration only. Omega emits HTML/JS and escapes captured terminal/source bytes.
# Source inputs are pinned release artifacts, never downloaded during a site build.
set -eu
cd "$(dirname "$0")"
omc=${OMEGA_OMC:-../hive-page/omc}
pin=$(tr -d ' \n' < ../hive-page/omc.sha256)
[ "$(sha256sum "$omc" | cut -d' ' -f1)" = "$pin" ] || { echo 'Compiler pin mismatch' >&2; exit 3; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
"$omc" < page.om > "$work/page"
"$omc" < client.om > "$work/client"
chmod +x "$work/page" "$work/client"
"$work/page" 4< wizard.capture.txt 5< ../../public/alpha/wizard.om > "$work/first.html"
"$work/page" 4< wizard.capture.txt 5< ../../public/alpha/wizard.om > "$work/second.html"
"$work/client" > "$work/first.js"
"$work/client" > "$work/second.js"
cmp "$work/first.html" "$work/second.html"
cmp "$work/first.js" "$work/second.js"
[ -s "$work/first.html" ] && [ -s "$work/first.js" ]
# Negative control: missing capture input must refuse, not silently publish a blank capture.
if "$work/page" 4< /dev/null 5< ../../public/alpha/wizard.om > "$work/refused.html"; then
    echo 'Empty capture wrongly accepted' >&2; exit 5
fi
stale=0
publish() {
    if [ ! -f "$2" ] || ! cmp -s "$1" "$2"; then stale=1; fi
    cp "$1" "$2"
    sha256sum "$2" > "$3"
}
publish "$work/first.html" main.html main.html.sha256
publish "$work/first.js" ../../public/js/alpha.js alpha.js.sha256
sha256sum ../../public/alpha/wizard.om > wizard.om.sha256
if [ "$stale" = 1 ] && [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
    echo 'Artifacts changed; commit the generated files with their Omega sources' >&2; exit 4
fi
printf 'ALPHA-GENERATED PASS compiler=%s\n' "$pin"
