#!/bin/sh
# SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
# AienOS alpha bootstrap: curl -fsSL https://aienos.com/install.sh | sh
#
# This is the only part of AienOS that is not written in Omega. It is a host dependency, kept
# small on purpose: it checks the platform, downloads one release, checks its Ed25519 signature
# and SHA-256 before unpacking anything, installs into ~/.local/share/aienos, adds a menu entry and
# a desktop icon, and offers to open the wizard (bin/aienos, written in Omega).
# Omega replacement path: a static Omega "aienos-get" that does the same with the omnet ed25519
# and omega-mv sha256 units; it needs an HTTPS word (or a TLS unit) in Omega first. Until then
# curl, openssl, sha256sum and tar do the network and the checks. See HOST-DEPENDENCIES.md.
#
# It never uses sudo and never writes outside your home folder. Run it again at any time: an
# installed, verified release is kept as it is and only the links are refreshed.
#
# Exit status: 0 installed, or a friendly "not supported here" (nothing touched);
#              1 a signature or checksum did not match (nothing installed);
#              2 a needed tool or the download is missing.
set -eu

TAG=alpha-20261026.1
REPO=aien-dev/AienOS-nv
BUNDLE=aienos-$TAG-linux-arm64.tar.gz
# An explicit tag and asset names, never "latest": this script installs exactly this release.
BASE_URL=${AIENOS_BASE_URL:-https://github.com/$REPO/releases/download/$TAG}
PROBE=${AIENOS_PROBE_ROOT:-}
GOOD_DRIVER=580.173.02
MORE=https://www.aienos.com/alpha/

# The alpha release key (Ed25519). The same key is published in the release (release.pub) and at
# https://www.aienos.com/alpha/. It is an alpha key, held in a TPM-bound vault, and it is replaced
# by a ceremony key before beta.
RELEASE_PUB_HEX=ff949504b427680039d53c33c85c448546b57d590eb1ecee3bb2ed6c2eaaf54a
RELEASE_PUB_PEM='-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA/5SVBLQnaAA51TwzyFxEhUa1fVkOsezuO7LtbC6q9Uo=
-----END PUBLIC KEY-----'

say() { printf '%s\n' "$*"; }
not_here() {
    say "$1"
    say "This alpha runs on Linux arm64 with an NVIDIA GB10 (DGX Spark) only."
    say "Here is what is coming: $MORE"
    say "Nothing was changed on this machine."
    exit 0
}

# 1. The platform, before anything is downloaded.
os=$(uname -s 2>/dev/null || echo unknown)
arch=$(uname -m 2>/dev/null || echo unknown)
[ "$os" = Linux ] || not_here "This computer runs $os."
[ "$arch" = aarch64 ] || not_here "This computer is $arch."
[ -r "$PROBE/sys/module/nvidia/version" ] || not_here "No NVIDIA kernel driver is loaded here."
driver=$(cat "$PROBE/sys/module/nvidia/version")
gb10=0
for dev in "$PROBE"/sys/bus/pci/devices/*; do
    [ -r "$dev/vendor" ] && [ -r "$dev/device" ] || continue
    if [ "$(cat "$dev/vendor")" = 0x10de ] && [ "$(cat "$dev/device")" = 0x2e12 ]; then
        gb10=1
    fi
done
[ "$gb10" = 1 ] || not_here "NVIDIA kernel driver $driver is loaded, but there is no GB10 here."
say "Found an NVIDIA GB10 with kernel driver $driver."
[ "$driver" = "$GOOD_DRIVER" ] || not_here "This alpha is built and tested against kernel driver $GOOD_DRIVER only; this machine has $driver."

# 2. Tools. All of these ship with DGX OS.
for t in curl sha256sum tar openssl mktemp; do
    command -v "$t" >/dev/null 2>&1 || { say "AienOS needs '$t', which is not installed."; exit 2; }
done
[ -n "${HOME:-}" ] && [ -d "$HOME" ] || { say "HOME is not set to a folder."; exit 2; }

base=$HOME/.local/share/aienos
tmp=$(mktemp -d "${TMPDIR:-/tmp}/aienos-install.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

fetch() {
    curl -fsSL --retry 3 --proto '=https,http' -o "$tmp/$1" "$BASE_URL/$1" \
        || { say "Could not download $BASE_URL/$1"; exit 2; }
}

# 3. Download, then check the signature and the checksum before unpacking anything.
say "Downloading AienOS $TAG ..."
fetch SHA256SUMS
fetch SHA256SUMS.sig
fetch "$BUNDLE"
printf '%s\n' "$RELEASE_PUB_PEM" > "$tmp/release.pub.pem"
if ! openssl pkeyutl -verify -pubin -inkey "$tmp/release.pub.pem" -rawin \
        -in "$tmp/SHA256SUMS" -sigfile "$tmp/SHA256SUMS.sig" >/dev/null 2>&1; then
    say "The signature on SHA256SUMS does not match the AienOS release key."
    say "Nothing was installed."
    exit 1
fi
want=$(awk -v f="$BUNDLE" '$2 == f || $2 == "*" f { print $1 }' "$tmp/SHA256SUMS")
have=$(sha256sum "$tmp/$BUNDLE" | cut -d' ' -f1)
if [ -z "$want" ] || [ "$want" != "$have" ]; then
    say "The download does not match its signed SHA-256 (expected ${want:-none}, got $have)."
    say "Nothing was installed."
    exit 1
fi
say "Signature and SHA-256 match (release key $RELEASE_PUB_HEX)."

# 4. Unpack into a staging folder, check every file again, then move it into place.
mkdir -p "$base/releases"
if tar -tzf "$tmp/$BUNDLE" | grep -Eq '^/|(^|/)\.\.(/|$)'; then
    say "The bundle contains an unsafe path. Nothing was installed."
    exit 1
fi
stage=$base/releases/.$TAG.new
rm -rf "$stage"
mkdir -p "$stage"
tar -xzf "$tmp/$BUNDLE" -C "$stage" --no-same-owner --no-same-permissions
if ! (cd "$stage" && sha256sum --quiet -c SHA256SUMS >/dev/null 2>&1); then
    rm -rf "$stage"
    say "A file inside the bundle does not match its SHA-256. Nothing was installed."
    exit 1
fi
printf '%s\n' "$have" > "$stage/.bundle-sha256"
dest=$base/releases/$TAG
if [ -f "$dest/.bundle-sha256" ] && [ "$(cat "$dest/.bundle-sha256")" = "$have" ] \
        && (cd "$dest" && sha256sum --quiet -c SHA256SUMS >/dev/null 2>&1); then
    rm -rf "$stage"
    say "AienOS $TAG is already installed and intact."
else
    if [ -e "$dest" ]; then
        rm -rf "$dest.old"
        mv "$dest" "$dest.old"
    fi
    mv "$stage" "$dest"
    rm -rf "$dest.old"
fi
cp "$tmp/SHA256SUMS" "$dest/release.SHA256SUMS"
cp "$tmp/SHA256SUMS.sig" "$dest/release.SHA256SUMS.sig"
ln -sfn "releases/$TAG" "$base/current.new"
mv -T "$base/current.new" "$base/current"

# 5. The command, the menu entry, the desktop icon. Every path written is listed in
#    installed-files so "aienos uninstall" removes exactly these.
list=$base/installed-files
: > "$list.new"
mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications" \
    "$HOME/.local/share/icons/hicolor/scalable/apps"
ln -sfn "$base/current/bin/aienos" "$HOME/.local/bin/aienos"
printf '%s\n' "$HOME/.local/bin/aienos" >> "$list.new"
icon=$HOME/.local/share/icons/hicolor/scalable/apps/aienos.svg
cp "$base/current/share/aienos/aienos.svg" "$icon"
printf '%s\n' "$icon" >> "$list.new"
# The launcher template ships in the bundle; only its two paths are filled in here.
entry() {
    sed -e "s|@AIENOS@|$base/current/bin/aienos|g" -e "s|@ICON@|$icon|g" \
        "$base/current/share/aienos/aienos.desktop.in"
}
menu=$HOME/.local/share/applications/aienos.desktop
entry > "$menu"
printf '%s\n' "$menu" >> "$list.new"
desk=
if command -v xdg-user-dir >/dev/null 2>&1; then
    desk=$(xdg-user-dir DESKTOP 2>/dev/null || true)
fi
[ -n "$desk" ] || desk=$HOME/Desktop
if [ "$desk" != "$HOME" ] && [ -d "$desk" ]; then
    entry > "$desk/aienos.desktop"
    chmod 755 "$desk/aienos.desktop"
    if command -v gio >/dev/null 2>&1; then
        gio set "$desk/aienos.desktop" metadata::trusted true >/dev/null 2>&1 || true
    fi
    printf '%s\n' "$desk/aienos.desktop" >> "$list.new"
fi
mv "$list.new" "$list"

say "Installed AienOS $TAG in $base"
say "Remove it at any time with: aienos uninstall"
case ":$PATH:" in
    *":$HOME/.local/bin:"*) cmd=aienos ;;
    *) cmd=$HOME/.local/bin/aienos ;;
esac

# 6. Offer the wizard. Piped through sh, stdin is the download, so ask on the terminal.
if [ "${AIENOS_NO_WIZARD:-0}" != 1 ] && [ -t 1 ] && { : < /dev/tty; } 2>/dev/null; then
    printf 'Open the setup wizard now? [Y/n] '
    read -r ans < /dev/tty || ans=n
    case $ans in
        ""|y|Y|yes|YES) rm -rf "$tmp"; trap - EXIT HUP INT TERM
            exec "$base/current/bin/aienos" wizard < /dev/tty ;;
    esac
fi
say "Start the wizard with: $cmd   (or double-click AienOS on your desktop)"
