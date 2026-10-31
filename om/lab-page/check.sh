#!/bin/sh
# Host orchestration only. lab.om emits the /lab/ HTML, client.om emits lab.js and the run worker.
# Every captured byte (sources, compiler output, terminal transcripts) is escaped by Omega's emit.
set -eu
cd "$(dirname "$0")"
omc=${OMEGA_OMC:-../hive-page/omc}
pin=$(tr -d ' \n' < ../hive-page/omc.sha256)
[ "$(sha256sum "$omc" | cut -d' ' -f1)" = "$pin" ] || { echo 'Compiler pin mismatch' >&2; exit 3; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
"$omc" < lab.om > "$work/page"
"$omc" < client.om > "$work/client"
chmod +x "$work/page" "$work/client"
cat captures/nvcc.txt > "$work/sass.txt"
printf '\n' >> "$work/sass.txt"
cat captures/gb10.txt >> "$work/sass.txt"
page() {
    "$work/page" 3< axpy.om 4< axpy.cu 5< captures/headline.txt 6< captures/aarch64.txt \
        7< captures/spirv.txt 8< "$work/sass.txt" 9< "$1" > "$2"
}
page captures/home.txt "$work/first.html"
page captures/home.txt "$work/second.html"
"$work/client" > "$work/first.js" 3> "$work/first-worker.js"
"$work/client" > "$work/second.js" 3> "$work/second-worker.js"
cmp "$work/first.html" "$work/second.html"
cmp "$work/first.js" "$work/second.js"
cmp "$work/first-worker.js" "$work/second-worker.js"
[ -s "$work/first.html" ] && [ -s "$work/first.js" ] && [ -s "$work/first-worker.js" ]
# Negative control: an empty capture must refuse, never publish a blank panel.
if page /dev/null "$work/refused.html"; then
    echo 'Empty capture wrongly accepted' >&2; exit 5
fi
stale=0
publish() {
    if [ ! -f "$2" ] || ! cmp -s "$1" "$2"; then stale=1; fi
    cp "$1" "$2"
    sha256sum "$2" > "$3"
}
publish "$work/first.html" main.html main.html.sha256
publish "$work/first.js" ../../public/js/lab.js lab.js.sha256
publish "$work/first-worker.js" ../../public/js/lab-worker.js lab-worker.js.sha256
sha256sum ../../public/lab/axpy.wasm ../../public/lab/record.tsv > published.sha256
if [ "$stale" = 1 ] && [ "${OMEGA_ACCEPT_CHANGE:-0}" != 1 ]; then
    echo 'Artifacts changed; commit the generated files with their Omega sources' >&2; exit 4
fi
printf 'LAB-GENERATED PASS compiler=%s\n' "$pin"
