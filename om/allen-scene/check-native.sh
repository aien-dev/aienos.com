#!/bin/sh
# Run under nice -n 19 ionice -c3. This is an independent verification oracle,
# not a production dependency and not an iMac/QEMU qualification receipt.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
ref=${ALLEN_NATIVE_SOURCE:?Set ALLEN_NATIVE_SOURCE to a clean aienux checkout at the exact reference pin}
pin=da55660ea7337f2b6a42095fcd1fb6ddebe1cc7c
[ "$(git -C "$ref" rev-parse HEAD)" = "$pin" ] || { printf 'Original UI reference pin mismatch\n' >&2; exit 3; }
git -C "$ref" diff --quiet "$pin" -- native/kernel/ui native/kernel/svc/allen_scene.c native/kernel/svc/allen_scene.h native/kernel/svc/allen_code.c native/kernel/svc/allen_code.h || { printf 'Original UI reference modified\n' >&2; exit 3; }
work=$(mktemp -d "${TMPDIR:-/tmp}/allen-native-oracle.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
ui=$ref/native/kernel/ui
svc=$ref/native/kernel/svc
scene=$root/om/allen-scene
cc=${CC:-cc}
"$cc" --version
"$cc" -O2 -fno-builtin "$scene/oracle.c" "$scene/adapter.c" \
  "$ui/mind.c" "$ui/night.c" "$ui/paint.c" "$ui/allen_font.c" "$ui/flow.c" \
  "$ui/sky_real.c" "$ui/sky_data.c" "$ui/chirp.c" -o "$work/adapter-oracle"
"$cc" -O2 -fno-builtin -I"$ui" "$scene/oracle-ui.c" \
  "$ui/mind.c" "$ui/night.c" "$ui/paint.c" "$ui/allen_font.c" "$ui/flow.c" \
  "$ui/sky_real.c" "$ui/sky_data.c" "$ui/canvas.c" \
  "$svc/allen_scene.c" "$svc/allen_code.c" -o "$work/original-ui-oracle"
cd "$root"
node om/allen-scene/test-parity.cjs "$work/adapter-oracle" public/allen/scene.wasm "$work/original-ui-oracle"
node om/allen-scene/test-controller.cjs
