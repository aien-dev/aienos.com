#!/bin/sh
# Shell is build/test coordination only, not the verifier implementation (verify.om is).
# Checks the vendored compiler and units against their pins (omc.sha256, vendor/SOURCE), composes
# the OM-2 unit (om2, sha2, ec, rsa, verify.om; the om/tls/build.sh order), compiles it twice
# (byte-identical), runs tests/run.py (openssl CLI as the oracle), proves single-word mutants of
# verify.om fail, then installs ./verify and its sha256 sidecar verify.sha256.
set -eu
cd "$(dirname "$0")"
omc=./omc
[ -x "$omc" ] || { echo 'community-verify: compiler missing' >&2; exit 2; }
sha256sum -c --quiet omc.sha256 || { echo 'community-verify: compiler pin mismatch' >&2; exit 3; }
for u in sha2 ec rsa; do
	want=$(sed -n "s|^file om/tls/$u.om sha256 \([0-9a-f]*\)\$|\1|p" vendor/SOURCE)
	[ -n "$want" ] && [ "$(sha256sum "vendor/$u.om" | cut -d ' ' -f1)" = "$want" ] ||
		{ echo "community-verify: vendor/$u.om does not match vendor/SOURCE" >&2; exit 4; }
done
command -v openssl >/dev/null 2>&1 || { echo 'community-verify: NOT_RUN: openssl CLI (test oracle) missing' >&2; exit 3; }
lane=''
if command -v taskset >/dev/null 2>&1; then lane='taskset -c 8-9'; fi
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

compose() { # compose PROGRAM OUT
	{ printf 'om2\n'; cat vendor/sha2.om vendor/ec.om vendor/rsa.om "$1"; } > "$2.unit.om"
	nice -n 19 $lane "$omc" < "$2.unit.om" > "$2"
	chmod +x "$2"
}
compose verify.om "$scratch/one"
compose verify.om "$scratch/two"
cmp -s "$scratch/one" "$scratch/two" || { echo 'community-verify: compiler output not deterministic' >&2; exit 5; }

result=$(nice -n 19 $lane python3 tests/run.py "$scratch/one") || { printf '%s\n' "$result" >&2; exit 7; }
printf '%s\n' "$result"

killed=0
total=0
while IFS='|' read -r label from to; do
	[ -n "$label" ] || continue
	total=$((total + 1))
	sed "s/^$from\$/$to/" verify.om > "$scratch/m.om"
	changed=$(diff verify.om "$scratch/m.om" | grep -c '^<' || true)
	[ "$changed" = 1 ] || { echo "community-verify: mutant $label changed $changed lines" >&2; exit 8; }
	compose "$scratch/m.om" "$scratch/m" || { echo "community-verify: mutant $label did not compile" >&2; exit 9; }
	if nice -n 19 $lane python3 tests/run.py "$scratch/m" --fast > "$scratch/m.log" 2>&1; then
		echo "community-verify: mutant $label SURVIVED" >&2
	else
		killed=$((killed + 1))
		printf 'mutant %s killed: %s\n' "$label" "$(head -n 1 "$scratch/m.log" | cut -c 1-140)"
	fi
done <<'EOF'
rsa_floor_1024|if or lt mb 2048 gt mb 4096|if or lt mb 1024 gt mb 4096
digest_31|ret ecdsa_verify 256 pk hs 32|ret ecdsa_verify 256 pk hs 31
upper_hex|ret sub c 55|ret sub c 56
rs_as_es|a = 257|a = 7
trailing_bytes|if ne get vst 1 get vst 0|if gt get vst 1 get vst 0
cap_16385|global vin uchar 16385|global vin uchar 16386
exponent_bits|if gt eb 32|if gt eb 1
EOF
[ "$killed" = "$total" ] || { echo "community-verify: $killed of $total mutants killed" >&2; exit 10; }

cp "$scratch/one" verify
chmod 755 verify
sum=$(sha256sum verify | cut -d ' ' -f1)
printf '%s\n' "$sum" > verify.sha256
counts=$(printf '%s\n' "$result" | sed -n 's/^RESULT //p')
printf 'COMMUNITY-VERIFY PASS: %s mutants_killed=%s/%s binary=om/community-verify/verify sha256=%s\n' "$counts" "$killed" "$total" "$sum"
