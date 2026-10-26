#!/bin/sh
# Install allen-science into /opt/allen-science. Run as root:
#   sh install.sh AIEN_MATH_BINARY [OMC] [IRWASM] [MATH_ADMIT]
# Creates the allen-science system user (no home, no shell, no groups, no sudo), installs the
# service, the helper, the required engine manifest, the four pinned tools (math-admit with its
# math-admit.om source) root-owned and read-only, the same science_protocol.py bytes beside the
# gateway adapter in /opt/allen-chat (a copy, not a symlink), and the interim foreground launcher.
# It starts nothing and does not register the aien-supervise entry (allen-science.svc is a draft
# for /etc/aien-supervise/services/ once aien-supervise is live).
# DESTDIR=/some/dir installs the same tree under that directory without creating the user or
# setting ownership (used by test_science_sandbox.py; needs no root).
set -eu
SRC=$(cd "$(dirname "$0")" && pwd)
ENGINE=${1:?usage: install.sh AIEN_MATH_BINARY [OMC] [IRWASM] [MATH_ADMIT]}
OMC=${2:-/opt/allen-chat/omc}
IRWASM=${3:-/opt/allen-chat/irwasm}
ADMIT=${4:-$SRC/math-admit}
DESTDIR=${DESTDIR:-}
DEST=$DESTDIR/opt/allen-science
GW=$DESTDIR/opt/allen-chat
LIBEXEC=$DESTDIR/usr/local/libexec/aien-supervise
if [ -z "$DESTDIR" ]; then
  [ "$(id -u)" = 0 ] || { echo "install.sh: run as root" >&2; exit 1; }
  OWN="-o root -g root"
else
  OWN=""
fi
helper() { /usr/bin/python3.12 -I -c "import sys; sys.path.insert(0, sys.argv[1]); import science_sandbox as s; $1" "$SRC" "$2"; }

# Refuse binaries that do not match the pins the helper enforces at run time.
for pair in "$ADMIT MATH_ADMIT_PIN" "$ENGINE AIEN_MATH_PIN" "$OMC OMC_PIN" "$IRWASM IRWASM_PIN"; do
  set -- $pair
  have=$(sha256sum "$1" | cut -d' ' -f1)
  want=$(helper "print(getattr(s, sys.argv[2]))" "$2")
  [ "$have" = "$want" ] || { echo "install.sh: $1 sha256 $have, pinned $want" >&2; exit 1; }
done
# The provenance manifest is required and must name exactly the pinned engine (commit + sha256).
problem=$(helper "print(s.manifest_problem(sys.argv[2]) or '')" "$SRC/engine.json")
[ -z "$problem" ] || { echo "install.sh: $problem" >&2; exit 1; }

if [ -z "$DESTDIR" ] && ! id allen-science >/dev/null 2>&1; then
  useradd --system --user-group --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin allen-science
fi

install -d $OWN -m 0755 "$DEST" "$GW" "$LIBEXEC"
for f in allen_science.py science_protocol.py science_sandbox.py launch.py math-admit.om engine.json; do
  install $OWN -m 0644 "$SRC/$f" "$DEST/$f"
done
install $OWN -m 0644 "$SRC/science_protocol.py" "$GW/science_protocol.py"
install $OWN -m 0755 "$ADMIT" "$DEST/math-admit"
install $OWN -m 0755 "$ENGINE" "$DEST/aien-math"
install $OWN -m 0755 "$OMC" "$DEST/omc"
install $OWN -m 0755 "$IRWASM" "$DEST/irwasm"
install $OWN -m 0755 "$SRC/allen-science-fg" "$LIBEXEC/allen-science-fg"
cmp -s "$DEST/science_protocol.py" "$GW/science_protocol.py" || {
  echo "install.sh: service and gateway science_protocol.py differ" >&2; exit 1; }
echo "installed $DEST and $GW/science_protocol.py; not started"
