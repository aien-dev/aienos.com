#!/bin/sh
# Install allen-science into /opt/allen-science. Run as root:
#   sh install.sh AIEN_MATH_BINARY [OMC] [IRWASM] [MATH_ADMIT]
# Creates the allen-science system user (no home, no shell, no groups, no sudo), installs the
# service, the helper, the four pinned tools (math-admit with its math-admit.om source) root-owned
# and read-only, and the interim foreground launcher. It starts nothing and does not register the
# aien-supervise entry (allen-science.svc is a draft for /etc/aien-supervise/services/ once
# aien-supervise is live).
set -eu
SRC=$(cd "$(dirname "$0")" && pwd)
ENGINE=${1:?usage: install.sh AIEN_MATH_BINARY [OMC] [IRWASM] [MATH_ADMIT]}
OMC=${2:-/opt/allen-chat/omc}
IRWASM=${3:-/opt/allen-chat/irwasm}
ADMIT=${4:-$SRC/math-admit}
DEST=/opt/allen-science
[ "$(id -u)" = 0 ] || { echo "install.sh: run as root" >&2; exit 1; }

# Refuse binaries that do not match the pins the helper enforces at run time.
pin() { /usr/bin/python3.12 -I -c "import sys; sys.path.insert(0, '$SRC'); import science_sandbox as s; print(s.$1)"; }
for pair in "$ADMIT MATH_ADMIT_PIN" "$ENGINE AIEN_MATH_PIN" "$OMC OMC_PIN" "$IRWASM IRWASM_PIN"; do
  set -- $pair
  have=$(sha256sum "$1" | cut -d' ' -f1)
  want=$(pin "$2")
  [ "$have" = "$want" ] || { echo "install.sh: $1 sha256 $have, pinned $want" >&2; exit 1; }
done

if ! id allen-science >/dev/null 2>&1; then
  useradd --system --user-group --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin allen-science
fi

install -d -o root -g root -m 0755 "$DEST"
for f in allen_science.py science_protocol.py science_sandbox.py launch.py math-admit.om; do
  install -o root -g root -m 0644 "$SRC/$f" "$DEST/$f"
done
[ -f "$SRC/engine.json" ] && install -o root -g root -m 0644 "$SRC/engine.json" "$DEST/engine.json"
install -o root -g root -m 0755 "$ADMIT" "$DEST/math-admit"
install -o root -g root -m 0755 "$ENGINE" "$DEST/aien-math"
install -o root -g root -m 0755 "$OMC" "$DEST/omc"
install -o root -g root -m 0755 "$IRWASM" "$DEST/irwasm"
install -d -o root -g root -m 0755 /usr/local/libexec/aien-supervise
install -o root -g root -m 0755 "$SRC/allen-science-fg" /usr/local/libexec/aien-supervise/allen-science-fg
echo "installed $DEST; not started"
