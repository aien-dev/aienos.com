#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
omc="$root/tools/community-qualification/bin/omc-57f5f425"
src="$root/tools/community-qualification/src/snapshot.om"
out="$root/evidence/community-qualification/snapshot"
tmp="$root/evidence/community-qualification/tmp"
mkdir -p "$tmp"
# RED control: a valid native stub executes but cannot produce the required JSON.
printf '%s\n' 'fn main to int' 'ret 0' 'end' > "$tmp/stub.om"
"$omc" < "$tmp/stub.om" > "$tmp/stub.elf"
test -s "$tmp/stub.elf"
chmod +x "$tmp/stub.elf"
printf '%b\n' 'M\t1\t2026-10-23T00:00:00Z\thttps://github.com/aien-dev/aienos.com' | "$tmp/stub.elf" > "$tmp/stub.json"
test ! -s "$tmp/stub.json"
"$omc" < "$src" > "$out"
test -s "$out"
chmod +x "$out"
file "$out" | grep -q "ELF 64-bit LSB executable, ARM aarch64"
printf '%b\n' 'M\t1\t2026-10-23T00:00:00Z\thttps://github.com/aien-dev/aienos.com' | "$out" > "$tmp/got.json"
grep -Fq '"schema_version":1' "$tmp/got.json"
grep -Fq '"generated_utc":"2026-10-23T00:00:00Z"' "$tmp/got.json"
grep -Fq '"source_url":"https://github.com/aien-dev/aienos.com"' "$tmp/got.json"
grep -Fq '"jobs":[]' "$tmp/got.json"
jq -e '.schema_version == 1 and .jobs == [] and (.generated_utc|type == "string") and (.source_url|startswith("https://github.com/"))' "$tmp/got.json" >/dev/null
! grep -Eq '"status":"(accepted|pass_reported)"' "$tmp/got.json"
for input in \
  'X\t1\t2026-10-23T00:00:00Z\thttps://github.com/aien-dev/aienos.com' \
  'M\t1\t2026-10-23T00:00:00X\thttps://github.com/aien-dev/aienos.com' \
  'M\t1\t2026-10-23T00:00:00Z\thttps://github.com/aien-dev/aienos.com/"x' \
  'M\t1\t2026-10-23T00:00:00Z'; do
  set +e
  printf '%b\n' "$input" | "$out" > "$tmp/refused.json" 2> "$tmp/refused.err"
  status=$?
  set -e
  test "$status" -ne 0
  test ! -s "$tmp/refused.json"
done
set +e
head -c 8192 /dev/zero | "$out" > "$tmp/oversize.json" 2> "$tmp/oversize.err"
status=$?
set -e
test "$status" -ne 0
test ! -s "$tmp/oversize.json"
echo PASS
