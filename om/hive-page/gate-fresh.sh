#!/bin/sh
# Host-only gate: actual AArch64 compiler path and read-only Node wasm oracle.
set -eu
cd "$(dirname "$0")/../.."
[ "$(uname -m)" = aarch64 ] || { echo 'NOT_RUN requires AArch64 Linux'; exit 3; }
B=${FRESH_GATE_OUT:-/tmp/hive-fresh-gate}
mkdir -p "$B"
nice -n 19 sh om/hive-page/check.sh
: > "$B/native.status"
i=0
while read -r now generated previous stale ok expected; do
    case "$now" in ''|'#'*) continue ;; esac
    i=$((i+1))
    cat om/hive-page/fresh.om > "$B/case-$i.om"
    printf 'fn main to int\nret int fresh long %s long %s long %s long %s %s\nend\n' \
        "$now" "$generated" "$previous" "$stale" "$ok" >> "$B/case-$i.om"
    om/hive-page/omc < "$B/case-$i.om" > "$B/case-$i"
    chmod +x "$B/case-$i"
    set +e
    "$B/case-$i"
    rc=$?
    set -e
    printf '%s\n' "$rc" >> "$B/native.status"
    [ "$rc" = "$expected" ] || { echo "FAIL native case $i exit=$rc expected=$expected"; exit 1; }
done < om/hive-page/fresh.cases
node om/hive-page/fresh-oracle.js public/wasm/hive-fresh.wasm om/hive-page/fresh.cases "$B/native.status"
sha256sum om/hive-page/fresh.om public/wasm/hive-fresh.wasm om/hive-page/irwasm
