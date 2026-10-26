#!/bin/sh
# Build the explicitly-authorized native C/WASM dependency and Omega controller.
# --generate: generate artifacts once, no test claims. Default: deterministic
# regeneration and exact committed-artifact checks. No package installation.
set -eu
cd "$(dirname "$0")"
omc=${OMEGA_OMC:-../hive-page/omc}
clang=${ALLEN_CLANG:-clang-18}
ld=${ALLEN_WASM_LD:-wasm-ld-18}
pin=$(tr -d ' \n' < ../hive-page/omc.sha256)
have=$(sha256sum "$omc" | cut -d' ' -f1)
[ "$pin" = "$have" ] || { printf 'Wrong Omega compiler pin\n' >&2; exit 3; }
"$clang" --version | grep -F 'clang version 18.1.3' >/dev/null || { printf 'Native dependency requires clang 18.1.3\n' >&2; exit 3; }
"$ld" --version | grep -F '18.1.3' >/dev/null || { printf 'Native dependency requires wasm-ld 18.1.3\n' >&2; exit 3; }
sha256sum -c native.sha256
work=$(mktemp -d "${TMPDIR:-/tmp}/allen-scene.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
build_wasm() {
    "$clang" --target=wasm32 -O2 -nostdlib -ffreestanding -fno-builtin \
      -ffunction-sections -fdata-sections -fvisibility=hidden -fno-ident \
      -Wl,--no-entry -Wl,--strip-all -Wl,--export-memory -Wl,--stack-first \
      -Wl,-z,stack-size=1048576 -Wl,--max-memory=67108864 -Wl,--gc-sections \
      -Wl,--export=scene_init -Wl,--export=scene_pixels -Wl,--export=scene_enter \
      -Wl,--export=scene_key -Wl,--export=scene_word -Wl,--export=scene_step \
      -Wl,--export=scene_draw -Wl,--export=scene_view -Wl,--export=scene_glyph \
      -Wl,--export=scene_audio -Wl,--export=scene_voice -fuse-ld="$ld" \
      adapter.c native/mind.c native/night.c native/paint.c native/allen_font.c \
      native/flow.c native/sky_real.c native/sky_data.c native/chirp.c -o "$1"
}
"$omc" < scene.om > "$work/controller"
chmod +x "$work/controller"
"$work/controller" > "$work/scene.js"
build_wasm "$work/scene.wasm"
if [ "${1:-}" != --generate ]; then
    "$work/controller" > "$work/again.js"
    build_wasm "$work/again.wasm"
    cmp "$work/scene.js" "$work/again.js"
    cmp "$work/scene.wasm" "$work/again.wasm"
fi
[ -s "$work/scene.js" ] && [ -s "$work/scene.wasm" ] || exit 6
if [ "${1:-}" = --generate ] || [ "${OMEGA_ACCEPT_CHANGE:-0}" = 1 ]; then
    mkdir -p ../../public/allen
    cp "$work/scene.js" ../../public/js/allen-scene.js
    cp "$work/scene.wasm" ../../public/allen/scene.wasm
    cp scene.om ../../public/allen/scene.om
    sha256sum ../../public/js/allen-scene.js ../../public/allen/scene.wasm ../../public/allen/scene.om > artifacts.sha256
else
    cmp "$work/scene.js" ../../public/js/allen-scene.js
    cmp "$work/scene.wasm" ../../public/allen/scene.wasm
    cmp scene.om ../../public/allen/scene.om
    sha256sum -c artifacts.sha256
fi
printf 'Generated native scene and Omega controller (no browser claim)\n'
