#!/bin/sh
# Verification and artifact capture only; all markup and CSS are authored in support.om.
set -eu
cd "$(dirname "$0")/../.."
omc=${OMEGA_OMC:-om/hive-page/omc}
pin=734f0a39319fbfa930bcc62cb30dbf06d47d78e126372b74f5baf478e6126b61
[ -x "$omc" ] || { echo 'support: missing compiler' >&2; exit 2; }
actual=$(sha256sum "$omc")
[ "${actual%% *}" = "$pin" ] || { echo 'support: compiler pin mismatch' >&2; exit 3; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
nice -n 19 ionice -c3 "$omc" < om/support-page/support.om > "$tmp/render"
chmod +x "$tmp/render"
for n in 1 2; do
  nice -n 19 ionice -c3 "$tmp/render" > "$tmp/page.$n" 3> "$tmp/entry.$n" 4> "$tmp/style.$n"
done
for f in page entry style; do
  test -s "$tmp/$f.1"
  cmp "$tmp/$f.1" "$tmp/$f.2"
done
if [ "${OMEGA_ACCEPT_CHANGE:-0}" = 1 ]; then
  cp "$tmp/page.1" om/support-page/main.html
  cp "$tmp/entry.1" om/support-page/entry.html
  cp "$tmp/style.1" public/assets/support.css
  sha256sum om/support-page/main.html > om/support-page/main.html.sha256
  sha256sum om/support-page/entry.html > om/support-page/entry.html.sha256
  sha256sum public/assets/support.css > om/support-page/support.css.sha256
fi
cmp "$tmp/page.1" om/support-page/main.html
cmp "$tmp/entry.1" om/support-page/entry.html
cmp "$tmp/style.1" public/assets/support.css
sha256sum -c om/support-page/main.html.sha256 om/support-page/entry.html.sha256 om/support-page/support.css.sha256
printf 'SUPPORT-ARTIFACTS PASS\n'
