#!/bin/sh
# Verification and artifact capture only; all markup and CSS are authored in still.om.
set -eu
cd "$(dirname "$0")/../.."
omc=${OMEGA_OMC:-om/hive-page/omc}
pin=734f0a39319fbfa930bcc62cb30dbf06d47d78e126372b74f5baf478e6126b61
[ -x "$omc" ] || { echo 'still-building: missing compiler' >&2; exit 2; }
actual=$(sha256sum "$omc")
[ "${actual%% *}" = "$pin" ] || { echo 'still-building: compiler pin mismatch' >&2; exit 3; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
nice -n 19 ionice -c3 "$omc" < om/still-building-page/still.om > "$tmp/render"
chmod +x "$tmp/render"
for n in 1 2; do
  nice -n 19 ionice -c3 "$tmp/render" > "$tmp/page.$n" 3> "$tmp/style.$n"
done
for f in page style; do
  test -s "$tmp/$f.1"
  cmp "$tmp/$f.1" "$tmp/$f.2"
done
if [ "${OMEGA_ACCEPT_CHANGE:-0}" = 1 ]; then
  cp "$tmp/page.1" om/still-building-page/main.html
  cp "$tmp/style.1" public/assets/still-building.css
  sha256sum om/still-building-page/main.html > om/still-building-page/main.html.sha256
  sha256sum public/assets/still-building.css > om/still-building-page/still-building.css.sha256
fi
cmp "$tmp/page.1" om/still-building-page/main.html
cmp "$tmp/style.1" public/assets/still-building.css
sha256sum -c om/still-building-page/main.html.sha256 om/still-building-page/still-building.css.sha256
printf 'STILL-BUILDING-ARTIFACTS PASS\n'
