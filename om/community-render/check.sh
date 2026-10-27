#!/bin/sh
# Shell is build/test coordination only, not the renderer implementation (render.om is).
# Verifies the compiler pin, compiles twice (byte-identical), runs tests/run.py (hand corpus,
# fuzz differential against tests/oracle.py, XSS property check), proves single-word mutants
# of render.om fail, then installs ./render and its sha256 sidecar render.sha256.
set -eu
cd "$(dirname "$0")"
omc=${OMEGA_OMC:-../hive-page/omc}
expected=734f0a39319fbfa930bcc62cb30dbf06d47d78e126372b74f5baf478e6126b61
[ -x "$omc" ] || { echo "community-render: compiler missing: $omc" >&2; exit 2; }
[ "$(sha256sum "$omc" | cut -d ' ' -f1)" = "$expected" ] || { echo 'community-render: compiler pin mismatch' >&2; exit 3; }
lane=''
if command -v taskset >/dev/null 2>&1; then lane='taskset -c 8-9'; fi
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

nice -n 19 $lane "$omc" < render.om > "$scratch/one"
nice -n 19 $lane "$omc" < render.om > "$scratch/two"
cmp -s "$scratch/one" "$scratch/two" || { echo 'community-render: compiler output not deterministic' >&2; exit 5; }
[ -s "$scratch/one" ] || exit 6
chmod +x "$scratch/one"

result=$(nice -n 19 $lane python3 tests/run.py "$scratch/one") || { printf '%s\n' "$result" >&2; exit 7; }
printf '%s\n' "$result"

# Single-word mutants: each line pattern must match exactly once and the mutant must fail the tests.
killed=0
total=0
while IFS='|' read -r label from to; do
	[ -n "$label" ] || continue
	total=$((total + 1))
	sed "s/^$from\$/$to/" render.om > "$scratch/m.om"
	changed=$(diff render.om "$scratch/m.om" | grep -c '^<' || true)
	[ "$changed" = 1 ] || { echo "community-render: mutant $label changed $changed lines" >&2; exit 8; }
	nice -n 19 $lane "$omc" < "$scratch/m.om" > "$scratch/m" || { echo "community-render: mutant $label did not compile" >&2; exit 9; }
	chmod +x "$scratch/m"
	if nice -n 19 $lane python3 tests/run.py "$scratch/m" --fast > "$scratch/m.log" 2>&1; then
		echo "community-render: mutant $label SURVIVED" >&2
	else
		killed=$((killed + 1))
		printf 'mutant %s killed: %s\n' "$label" "$(head -n 1 "$scratch/m.log" | cut -c 1-140)"
	fi
done <<'EOF'
no_lt_escape|if eq c 60|if eq c 0
no_gt_escape|if eq c 62|if eq c 0
no_quot_escape|if eq c 34|if eq c 0
no_apos_escape|if eq c 39|if eq c 0
no_del_escape|if eq c 127|if eq c 0
cap_off_by_one|if gt n CAP|if ge n CAP
link_cap_17|const MAXL ulong 16|const MAXL ulong 17
any_scheme|if not ok|if false
fence_cap_9|const MAXB ulong 8|const MAXB ulong 9
shown_49|const SHOWN ulong 48|const SHOWN ulong 49
lang_17|const LANGMAX ulong 16|const LANGMAX ulong 17
tag_after_amp|if not or or isword pb eq pb 38 eq pb 47|if not or or isword pb eq pb 0 eq pb 47
overlong_c0|if lt c 194|if lt c 192
surrogate|hi = 159|hi = 191
paren_balance|ret gt x o|ret ge x o
EOF
[ "$killed" = "$total" ] || { echo "community-render: $killed of $total mutants killed" >&2; exit 10; }

cp "$scratch/one" render
chmod 755 render
sum=$(sha256sum render | cut -d ' ' -f1)
printf '%s\n' "$sum" > render.sha256
counts=$(printf '%s\n' "$result" | sed 's/^RESULT //')
printf 'COMMUNITY-RENDER PASS: %s mutants_killed=%s/%s binary=om/community-render/render sha256=%s\n' "$counts" "$killed" "$total" "$sum"
