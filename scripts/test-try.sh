#!/bin/sh
# Tests for public/try.sh against a fake release folder (file:// base, no network).
# Usage: sh scripts/test-try.sh [path/to/try.sh]   Exit 0 only if every case passes.
set -u
script="${1:-$(dirname "$0")/../public/try.sh}"
root="$(mktemp -d "${TMPDIR:-/tmp}/try-test.XXXXXX")"
trap 'rm -rf -- "$root"' EXIT
fails=0
ok() { printf 'PASS %s\n' "$1"; }
bad() { printf 'FAIL %s\n' "$1"; fails=$((fails + 1)); }

rel="$root/release"; mkdir -p "$rel"
# A stand-in runner: prints its arguments and a marker so we can tell it ran.
printf '#!/bin/sh\necho "RUNNER-RAN $*"\n' > "$rel/aienos-run-linux-x86_64"
good="$(sha256sum "$rel/aienos-run-linux-x86_64" | awk '{print $1}')"
printf '%s  aienos-run-linux-x86_64\n' "$good" > "$rel/aienos-run-SHA256SUMS"

run() { # name, expect_exit, expect_text, forbid_text, env... -- args
  name="$1"; want_exit="$2"; want_text="$3"; forbid="$4"; shift 4
  out="$(env AIENOS_RELEASE_BASE="file://$rel" AIENOS_CACHE="$root/cache" AIENOS_TRY_TARGET=linux-x86_64 "$@" sh "$script" </dev/null 2>&1)"; code=$?
  [ "$code" -eq "$want_exit" ] || { bad "$name: exit $code, wanted $want_exit: $out"; return; }
  case "$out" in *"$want_text"*) ;; *) bad "$name: output lacks '$want_text': $out"; return ;; esac
  if [ -n "$forbid" ]; then case "$out" in *"$forbid"*) bad "$name: output has '$forbid': $out"; return ;; esac; fi
  ok "$name"
}

run "good download runs the runner" 0 "RUNNER-RAN" "MISMATCH" env
[ -x "$root/cache/bin/aienos-run" ] && ok "runner kept in cache, executable" || bad "runner not in cache"
run "second run reuses the verified runner" 0 "already downloaded and verified" "downloading" env

# Options reach the runner.
out="$(env AIENOS_RELEASE_BASE="file://$rel" AIENOS_CACHE="$root/cache" AIENOS_TRY_TARGET=linux-x86_64 sh "$script" --reset </dev/null 2>&1)"
case "$out" in *"RUNNER-RAN --reset"*) ok "arguments pass through" ;; *) bad "arguments lost: $out" ;; esac

# Download and verify only.
rm -rf "$root/cache"
run "no-run mode verifies but does not run" 0 "not run" "RUNNER-RAN" env AIENOS_TRY_NO_RUN=1

# Tampered runner: the manifest line stays, the bytes change. Must refuse and leave nothing behind.
rm -rf "$root/cache"
printf '#!/bin/sh\necho EVIL\n' > "$rel/aienos-run-linux-x86_64"
run "tampered runner is refused" 5 "MISMATCH" "EVIL" env
[ ! -e "$root/cache/bin/aienos-run" ] && ok "tampered runner left no file" || bad "tampered runner was kept"

# A previously verified runner that no longer matches the manifest is replaced, not trusted.
printf '#!/bin/sh\necho "RUNNER-RAN $*"\n' > "$rel/aienos-run-linux-x86_64"
run "restored runner passes" 0 "RUNNER-RAN" "" env
printf 'EVIL' > "$root/cache/bin/aienos-run"
run "stale cached runner is re-downloaded and re-verified" 0 "downloading" "EVIL" env

# Bad manifest lines.
printf 'nothex  aienos-run-linux-x86_64\n' > "$rel/aienos-run-SHA256SUMS"
run "non-hash manifest line is refused" 4 "not a sha256" "RUNNER-RAN" env
printf '%s  aienos-run-macos-aarch64\n' "$good" > "$rel/aienos-run-SHA256SUMS"
run "no line for this system is refused" 3 "no runner for linux-x86_64" "RUNNER-RAN" env
rm -f "$rel/aienos-run-SHA256SUMS"
run "no release yet says so" 3 "no AienOS runner is published yet" "RUNNER-RAN" env

# Unsupported systems: a fake uname on PATH.
shim="$root/shim"; mkdir -p "$shim"
unsupported() { # name, uname -s, uname -m, expected text
  printf '#!/bin/sh\nif [ "$1" = -s ]; then echo "%s"; else echo "%s"; fi\n' "$2" "$3" > "$shim/uname"; chmod +x "$shim/uname"
  out="$(env -u AIENOS_TRY_TARGET PATH="$shim:$PATH" AIENOS_RELEASE_BASE="file://$rel" AIENOS_CACHE="$root/cache" sh "$script" </dev/null 2>&1)"; code=$?
  [ "$code" -eq 2 ] && case "$out" in *"$4"*) ok "$1" ;; *) bad "$1: $out" ;; esac || bad "$1: exit $code: $out"
}
unsupported "Windows is pointed at the exe" MINGW64_NT x86_64 "aienos-run.exe"
unsupported "32-bit ARM is refused" Linux armv7l "not supported"

[ "$fails" -eq 0 ] && { echo "ALL PASS"; exit 0; }
echo "$fails FAILED"; exit 1
