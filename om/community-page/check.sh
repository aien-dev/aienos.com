#!/bin/sh
# Host orchestration only. page.om emits the /community/ skeleton and its three documents,
# client.om emits community.js and the WebAssembly call worker. Every published byte comes
# from the Omega text literals; this script compiles, runs twice, compares and publishes.
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
for run in first second; do
    "$work/page" > "$work/$run.html" 3> "$work/$run-policy.html" 4> "$work/$run-terms.html" 5> "$work/$run-privacy.html"
    "$work/client" > "$work/$run.js" 3> "$work/$run-worker.js"
done
for out in .html -policy.html -terms.html -privacy.html .js -worker.js; do
    cmp "$work/first$out" "$work/second$out"
    [ -s "$work/first$out" ]
done
# Negative control: a closed output descriptor must fail the run, never publish a short page.
if "$work/page" > /dev/null 3> /dev/null 4>&- 5> /dev/null; then
    echo 'Closed terms descriptor wrongly accepted' >&2; exit 5
fi
if "$work/client" > /dev/null 3>&-; then
    echo 'Closed worker descriptor wrongly accepted' >&2; exit 5
fi
stale=0
publish() {
    if [ ! -f "$2" ] || ! cmp -s "$1" "$2"; then stale=1; fi
    cp "$1" "$2"
    sha256sum "$2" > "$3"
}
publish "$work/first.html" main.html main.html.sha256
publish "$work/first-policy.html" policy.html policy.html.sha256
publish "$work/first-terms.html" terms.html terms.html.sha256
publish "$work/first-privacy.html" privacy.html privacy.html.sha256
publish "$work/first.js" ../../public/js/community.js community.js.sha256
publish "$work/first-worker.js" ../../public/js/community-worker.js community-worker.js.sha256
if [ "$stale" = 1 ] && [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
    echo 'Artifacts changed; commit the generated files with their Omega sources' >&2; exit 4
fi
printf 'COMMUNITY-GENERATED PASS compiler=%s\n' "$pin"
