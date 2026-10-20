#!/bin/sh
# Try AienOS in one step:  curl -fsSL https://www.aienos.com/try.sh | sh
#
# What this does, in order, and nothing else:
#   1. Works out your system (Linux or macOS, x86_64 or aarch64).
#   2. Downloads aienos-run-SHA256SUMS from the latest release at
#      https://github.com/aien-dev/aienos/releases and reads the line for your system.
#   3. Downloads that one runner and checks its sha256 against the line.
#      A mismatch deletes the download and stops. Nothing runs.
#   4. Runs it. The runner fetches and verifies the AienOS image itself and opens
#      an AienOS console in this terminal.
#
# Pass options through:  curl -fsSL https://www.aienos.com/try.sh | sh -s -- --reset
# Download and verify only, do not run:  AIENOS_TRY_NO_RUN=1
# Where it keeps files: AIENOS_CACHE (default ~/.cache/aienos on Linux, ~/Library/Caches/AienOS on macOS).
# To remove everything: run with --reset, then delete that folder.
#
# Windows: this script does not run there. Download aienos-run.exe from https://www.aienos.com/try/
#
# The whole script is one function called on the last line, so a download cut
# short in the middle runs nothing.

main() {
  set -eu

  BASE="${AIENOS_RELEASE_BASE:-https://github.com/aien-dev/aienos/releases/latest/download}"
  SUMS="aienos-run-SHA256SUMS"

  say() { printf '%s\n' "$*"; }
  die() { printf 'aienos try: %s\n' "$*" >&2; exit "${2:-1}"; }

  # 1. Which system.
  if [ -n "${AIENOS_TRY_TARGET:-}" ]; then
    target="$AIENOS_TRY_TARGET"
  else
    case "$(uname -s)" in
      Linux) os=linux ;;
      Darwin) os=macos ;;
      *) die "this system ($(uname -s)) is not supported by this script. On Windows, download aienos-run.exe from https://www.aienos.com/try/" 2 ;;
    esac
    case "$(uname -m)" in
      x86_64 | amd64) arch=x86_64 ;;
      aarch64 | arm64) arch=aarch64 ;;
      *) die "this processor ($(uname -m)) is not supported. Supported: x86_64 and aarch64." 2 ;;
    esac
    target="$os-$arch"
  fi
  asset="aienos-run-$target"

  if [ -n "${AIENOS_CACHE:-}" ]; then
    cache="$AIENOS_CACHE"
  elif [ "${target%%-*}" = macos ]; then
    cache="$HOME/Library/Caches/AienOS"
  else
    cache="${XDG_CACHE_HOME:-$HOME/.cache}/aienos"
  fi

  # Tools: one downloader, one hasher.
  if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL --proto '=https,file' -o "$2" "$1"; }
  elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -q -O "$2" "$1"; }
  else
    die "need curl or wget to download the runner." 2
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    hash_of() { sha256sum "$1" | awk '{print $1}'; }
  elif command -v shasum >/dev/null 2>&1; then
    hash_of() { shasum -a 256 "$1" | awk '{print $1}'; }
  elif command -v openssl >/dev/null 2>&1; then
    hash_of() { openssl dgst -sha256 "$1" | awk '{print $NF}'; }
  else
    die "need sha256sum, shasum or openssl to check the download." 2
  fi

  mkdir -p "$cache/bin"
  work="$(mktemp -d "${TMPDIR:-/tmp}/aienos-try.XXXXXX")"
  trap 'rm -rf -- "$work"' EXIT INT TERM

  # 2. The signed-off list of runners for the latest release.
  say "AienOS: reading $BASE/$SUMS"
  if ! fetch "$BASE/$SUMS" "$work/sums"; then
    die "no AienOS runner is published yet (could not read $BASE/$SUMS). Releases: https://github.com/aien-dev/aienos/releases" 3
  fi
  want="$(awk -v a="$asset" '$2 == a || $2 == "*" a { print $1; exit }' "$work/sums")"
  if [ -z "$want" ]; then
    die "the latest release has no runner for $target. Listed there: $(awk '{printf "%s ", $2}' "$work/sums")" 3
  fi
  case "$want" in
    *[!0-9a-f]*) die "the manifest line for $asset is not a sha256 value." 4 ;;
  esac
  [ "${#want}" -eq 64 ] || die "the manifest line for $asset is not a sha256 value." 4

  # 3. The runner, checked before it is ever executed.
  bin="$cache/bin/aienos-run"
  if [ -f "$bin" ] && [ "$(hash_of "$bin")" = "$want" ]; then
    say "AienOS: runner already downloaded and verified ($want)"
  else
    say "AienOS: downloading $asset"
    fetch "$BASE/$asset" "$work/runner" || die "download of $BASE/$asset failed." 3
    got="$(hash_of "$work/runner")"
    if [ "$got" != "$want" ]; then
      die "sha256 MISMATCH for $asset: the manifest says $want, the download is $got. Deleted. Nothing was run." 5
    fi
    chmod +x "$work/runner"
    mv -f "$work/runner" "$bin"
    say "AienOS: sha256 verified ($got)"
  fi

  if [ -n "${AIENOS_TRY_NO_RUN:-}" ]; then
    say "AienOS: runner is at $bin (not run: AIENOS_TRY_NO_RUN is set)"
    exit 0
  fi

  # 4. Run it. This script was piped in, so give the runner the keyboard.
  trap - EXIT INT TERM
  rm -rf -- "$work"
  if (: </dev/tty) 2>/dev/null; then
    exec "$bin" "$@" </dev/tty
  fi
  exec "$bin" "$@"
}

main "$@"
