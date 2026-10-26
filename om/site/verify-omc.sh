#!/bin/sh
# verify-omc.sh -- refuse the vendored Omega compiler unless omega-mv vouches for it.
#
# A digest committed next to the binary proves nothing: one PR can change both.
# The trust anchor is omega-mv instead. The lock (om/site/omc.provenance) names
# a fixed omega-mv revision; this script fetches omega-mv fresh and requires
#   1. the revision is an ancestor of omega-mv main (it landed through review),
#   2. seed/out/omc.sha256 at that revision names the locked digest,
#   3. seed/out/omc at that revision is byte-identical to the vendored binary,
#   4. seed/omega-from-seed.sh at that revision rebuilds omc from the
#      hand-written seed hex and the result is byte-identical too.
# Moving to a newer compiler is a visible diff of the lock, never silent.
# Runs on Linux AArch64 only (the seed chain emits AArch64 executables); any
# other host fails, it does not skip.
#
# Usage: sh om/site/verify-omc.sh [vendored-omc [lock]]
# Prints OMC-PIN PASS and exits 0, or OMC-PIN FAIL <reason> and exits 1.
set -eu
REPO=https://github.com/aien-dev/omega-mv
OMC=${1:-om/site/omc}
LOCK=${2:-om/site/omc.provenance}

fail() { printf 'OMC-PIN FAIL %s\n' "$*"; exit 1; }
field() { sed -n "s/^$1 //p" "$LOCK"; }

[ -f "$OMC" ] || fail "no vendored compiler at $OMC"
[ -f "$LOCK" ] || fail "no lock at $LOCK"
[ "$(uname -s) $(uname -m)" = "Linux aarch64" ] || fail "host $(uname -s) $(uname -m) cannot run the seed chain; need Linux aarch64"

repo=$(field repository)
rev=$(field revision)
path=$(field path)
want=$(field sha256)
[ "$repo" = "$REPO" ] || fail "lock repository '$repo' is not $REPO"
printf '%s\n' "$rev" | grep -Eq '^[0-9a-f]{40}$' || fail "lock revision '$rev' is not a full commit sha"
printf '%s\n' "$want" | grep -Eq '^[0-9a-f]{64}$' || fail "lock sha256 '$want' is not a sha256"
[ "$path" = "seed/out/omc" ] || fail "lock path '$path' is not seed/out/omc"

have=$(sha256sum "$OMC" | cut -d' ' -f1)
[ "$have" = "$want" ] || fail "vendored $OMC is $have, lock says $want"

W=$(mktemp -d)
trap 'rm -rf "$W"' EXIT
git -C "$W" init -q
git -C "$W" fetch -q --filter=blob:none "$REPO" main "$rev" || fail "cannot fetch $REPO main and $rev"
main=$(git -C "$W" ls-remote "$REPO" refs/heads/main | cut -f1)
[ -n "$main" ] || fail "cannot read $REPO main"
git -C "$W" cat-file -e "$main^{commit}" || fail "fetched main $main is missing"
git -C "$W" merge-base --is-ancestor "$rev" "$main" || fail "$rev is not on $REPO main ($main)"
printf 'omc-pin: %s is an ancestor of omega-mv main %s\n' "$rev" "$main"

recorded=$(git -C "$W" show "$rev:seed/out/omc.sha256" | cut -d' ' -f1) || fail "no seed/out/omc.sha256 at $rev"
[ "$recorded" = "$want" ] || fail "omega-mv $rev records $recorded, lock says $want"
git -C "$W" show "$rev:seed/out/omc" > "$W/omc.committed" || fail "no seed/out/omc at $rev"
cmp -s "$W/omc.committed" "$OMC" || fail "vendored $OMC differs from seed/out/omc at $rev"
printf 'omc-pin: vendored binary equals seed/out/omc at %s (%s)\n' "$rev" "$want"

git -C "$W" -c advice.detachedHead=false checkout -q "$rev" || fail "cannot check out $rev"
sh "$W/seed/omega-from-seed.sh" > "$W/birth.log" 2>&1 || { cat "$W/birth.log"; fail "seed rebuild failed at $rev"; }
grep -q '^RC=0$' "$W/birth.log" || { cat "$W/birth.log"; fail "seed rebuild did not report RC=0"; }
cmp -s "$W/seed/out/omc" "$OMC" || fail "seed rebuild at $rev is $(sha256sum "$W/seed/out/omc" | cut -d' ' -f1), not the vendored binary"
printf 'omc-pin: seed/omega-from-seed.sh at %s rebuilt the same %s bytes\n' "$rev" "$(wc -c < "$OMC")"
printf 'OMC-PIN PASS %s %s\n' "$rev" "$want"
