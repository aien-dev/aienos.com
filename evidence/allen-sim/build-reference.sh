#!/bin/sh
# Builds the kernel's own window code (read-only snapshot given as $1, default the 903277b copy) with
# the host C compiler and a small driver, runs it and writes reference.txt next to this script.
set -e
here=$(cd "$(dirname "$0")" && pwd)
ui=${1:-$HOME/workspace/allen-site-src/native/kernel/ui}
out=$(mktemp -d)
gcc -O2 -std=gnu11 -fno-strict-aliasing -I"$ui" -o "$out/reference" "$here/reference.c" \
  "$ui/allen_ui.c" "$ui/night.c" "$ui/mind.c" "$ui/flow.c" "$ui/paint.c" "$ui/allen_font.c" "$ui/chirp.c" -lpthread
{ "$out/reference" 1280; "$out/reference" 800; "$out/reference"; } > "$here/reference.txt"; cp "$out/reference" /tmp/allen-reference-bin
rm -rf "$out"
wc -l "$here/reference.txt"
