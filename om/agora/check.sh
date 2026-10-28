#!/bin/sh
# Agora core gate: pinned compiler, vendored unit hashes, the full wire scenario
# (test/run.sh) on the real server, then mutants that remove one check each; every
# mutant must compile and must fail the scenario, which proves the check exists.
# OMEGA_OMC names the pinned omc (sha256 in omc.sha256); it overrides the path, never the pin.
# Exit 2: missing compiler or tool; 3: wrong pin; 4: vendored unit hash mismatch;
# 5: scenario failed on the real server; 6: a mutant survived or did not apply.
set -eu
cd "$(dirname "$0")"
here=$PWD
pin=$(tr -d ' \n' < omc.sha256)
omc=${OMEGA_OMC:-}
if [ -z "$omc" ] || [ ! -f "$omc" ]; then
    printf 'No compiler: set OMEGA_OMC to the pinned omc (%s)\n' "$pin" >&2
    exit 2
fi
have=$(sha256sum "$omc" | cut -d' ' -f1)
if [ "$have" != "$pin" ]; then
    printf 'Compiler sha256 mismatch: recorded %s, found %s\n' "$pin" "$have" >&2
    exit 3
fi
for t in curl sha256sum; do
    command -v "$t" > /dev/null || { printf 'missing tool: %s\n' "$t" >&2; exit 2; }
done
# Each vendored unit matches its sidecar; a verbatim copy also matches its upstream source hash
# (sha256.om is a recorded mechanical derivation, so its provenance hash names the upstream file).
for u in http sha256 json jsonw; do
    want=$(tr -d ' \n' < "$u.om.sha256")
    got=$(sha256sum "$u.om" | cut -d' ' -f1)
    src=$(sed -n 's/^source_sha256 //p' "$u.om.provenance")
    if grep -q '^note Derived mechanically' "$u.om.provenance"; then src=$got; fi
    if [ "$got" != "$want" ] || [ "$got" != "$src" ]; then
        printf '%s.om: sha256 %s, sidecar %s, provenance %s\n' "$u" "$got" "$want" "$src" >&2
        exit 4
    fi
done
printf 'pin ok: omc %s; vendored http sha256 json jsonw match their sidecars\n' "$pin"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/agora-check.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
build() {
    { printf 'om2\n'; cat sha256.om http.om "$1"; } | "$omc" > "$2"
    chmod +x "$2"
}
build agora.om "$tmp/agora"
{ printf 'om2\n'; cat sha256.om test/pow.om; } | "$omc" > "$tmp/pow"
chmod +x "$tmp/pow"
mkdir -p "$tmp/real"
if ! sh test/run.sh "$tmp/agora" "$tmp/pow" "$tmp/real"; then
    exit 5
fi

# mutant NAME SED_SCRIPT: one check removed from agora.om; the scenario must fail on it
survived=0
mutant() {
    m=$tmp/m-$1
    sed "$2" agora.om > "$m.om"
    if cmp -s agora.om "$m.om"; then
        printf 'mutant %s: sed did not change agora.om\n' "$1" >&2
        exit 6
    fi
    build "$m.om" "$m"
    mkdir -p "$m.d"
    if sh test/run.sh "$m" "$tmp/pow" "$m.d" > "$m.log" 2>&1; then
        printf 'mutant %s: SURVIVED (scenario passed)\n' "$1"
        survived=$((survived + 1))
    else
        printf 'mutant %s: killed, %s\n' "$1" "$(sed -n 's/^FAIL //p' "$m.log" | head -n 1)"
    fi
}
mutant pow-off 's/^if not powhash add get c_b ci extra$/if not true/'
mutant body-cap-off 's/^r = lbody 8192$/r = lbody 65536/'
mutant author-rate-off '/^fn rlchk ai ulong to ulong$/,/^end$/s/^if isaien ai$/if true/'
mutant takedown-ignored 's/^put th_hid get p_th p add he 1$/put th_hid get p_th p 0/; s/^put p_hid p add he 1$/put p_hid p 0/'
mutant kind-change-allowed 's/^if ne ulong get a_kd ai kd$/if not true/; s/^if ne ulong get a_kd ai get g_x XKD$/if not true/'
mutant utf8-control-off 's/^while lt i n bound 999999$/while lt i 0 bound 999999/'
mutant imported-at-aien-queued 's/^if and eq get g_v VLIVE 1 eq get g_v VHN 0$/if eq get g_v VLIVE 1/'
if [ "$survived" -ne 0 ]; then
    printf '%d mutant(s) survived\n' "$survived" >&2
    exit 6
fi
printf 'PASS agora core: scenario on the real server, 7 of 7 mutants killed\n'
