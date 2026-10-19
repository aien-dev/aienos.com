#!/bin/sh
# Records the kernel's own window code through a real conversation and encodes it:
#   public/allen/allen-window.mp4 (H.264, with the chirp at the merger) and allen-window-poster.jpg.
# Needs gcc and ffmpeg. $1: the read-only kernel snapshot (default: the 903277b copy under allen-site-src).
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
kernel=${1:-$HOME/workspace/allen-site-src/native/kernel}
work=$(mktemp -d)
gcc -O2 -std=gnu11 -fno-strict-aliasing -I"$kernel/ui" -I"$kernel/svc" -o "$work/video" "$here/video.c" \
  "$kernel/ui/allen_ui.c" "$kernel/ui/night.c" "$kernel/ui/mind.c" "$kernel/ui/flow.c" "$kernel/ui/paint.c" \
  "$kernel/ui/allen_font.c" "$kernel/ui/chirp.c" "$kernel/svc/allen_reply.c" "$kernel/svc/allen_notes.c" \
  "$kernel/svc/allen_intent.c" -lpthread
"$work/video" "$work/frames.rgbx" "$work/chirp.pcm"
ffmpeg -y -loglevel error -f rawvideo -pix_fmt rgb0 -s 1280x720 -framerate 31.25 -i "$work/frames.rgbx" \
  -f s16le -ar 48000 -ac 2 -i "$work/chirp.pcm" \
  -c:v libx264 -preset slow -crf 27 -pix_fmt yuv420p -c:a aac -b:a 96k -shortest -movflags +faststart \
  "$root/public/allen/allen-window.mp4"
ffmpeg -y -loglevel error -f rawvideo -pix_fmt rgb0 -s 1280x720 -framerate 31.25 -i "$work/frames.rgbx" \
  -vf "select=eq(n\,$((31*25)))" -frames:v 1 -q:v 3 "$root/public/allen/allen-window-poster.jpg"
rm -rf "$work"
ls -l "$root/public/allen/allen-window.mp4" "$root/public/allen/allen-window-poster.jpg"
