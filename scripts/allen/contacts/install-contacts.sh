#!/bin/sh
# Build and install allen-contacts-ingest (seeded Omega; system units, User=allen-contacts)
# and the allen-contacts CLI.  Layout per CONTRACT.md "Users and directories".
# Usage: OMC=/path/to/seed/out/omc sh install-contacts.sh      (uses sudo -n)
# The compiler must be the pinned one: omega-mv 3364a1dfd0ca50c1b82bf19ba34932d8bc10a7a9,
# seed/out/omc sha256 below, reproduced by sh seed/omega-from-seed.sh at that commit.
set -eu
OMC_SHA=befb8199db8c4bca188a494dfaac2a6f0ee5c36fcea11188c75666c55fce8a1d
src=$(cd "$(dirname "$0")" && pwd)
S="sudo -n"
: "${OMC:?set OMC to the pinned seed/out/omc}"

[ "$(sha256sum "$OMC" | cut -d' ' -f1)" = "$OMC_SHA" ] || { echo "omc is not the pinned compiler" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
if nice -n 19 "$OMC" < "$src/allen_contacts_ingest.om" > "$tmp/allen-contacts-ingest" 2> "$tmp/err"; then
  :
else
  echo "omc refused allen_contacts_ingest.om:" >&2; cat "$tmp/err" >&2; exit 1
fi
[ ! -s "$tmp/err" ] || { cat "$tmp/err" >&2; exit 1; }
sha256sum "$src/allen_contacts_ingest.om" "$tmp/allen-contacts-ingest"

$S getent passwd allen-contacts >/dev/null
$S getent group allen-drop >/dev/null
[ -d /var/lib/allen-chat ] || $S install -d -o root -g root -m 0755 /var/lib/allen-chat
$S install -d -o allen-contacts -g allen-drop -m 2730 /var/lib/allen-chat/contacts-drop
$S install -d -o allen-contacts -g allen-contacts -m 0700 /var/lib/allen-contacts \
  /var/lib/allen-contacts/quarantine

$S install -d -o root -g root -m 0755 /opt/allen-contacts
$S install -o root -g root -m 0755 "$tmp/allen-contacts-ingest" /opt/allen-contacts/
$S install -o root -g root -m 0644 "$src/allen_contacts_ingest.om" \
  "$src/allen_contacts_ingest.py" /opt/allen-contacts/
$S install -o root -g root -m 0755 "$src/allen-contacts" /usr/local/bin/allen-contacts

$S install -o root -g root -m 0644 "$src/allen-contacts-ingest.service" \
  "$src/allen-contacts-ingest.path" /etc/systemd/system/
$S systemd-analyze verify /etc/systemd/system/allen-contacts-ingest.service \
  /etc/systemd/system/allen-contacts-ingest.path
$S systemctl daemon-reload
$S systemctl enable allen-contacts-ingest.path
$S systemctl restart allen-contacts-ingest.path
systemctl is-active allen-contacts-ingest.path
