#!/usr/bin/env bash
# AIEN OS Universal Installer Entrypoint
# Checks out the canonical source before running its repository-aware installer.
set -euo pipefail

command -v git >/dev/null 2>&1 || {
  echo "Error: git is required to install AIEN OS." >&2
  exit 1
}

AIEN_INSTALL_DIR="$(mktemp -d "${TMPDIR:-/tmp}/aienos-install.XXXXXX")"
cleanup() {
  rm -rf -- "$AIEN_INSTALL_DIR"
}
trap cleanup EXIT INT TERM

git clone --depth 1 https://github.com/aien-dev/aien-sovereign-core.git "$AIEN_INSTALL_DIR/source"
cd "$AIEN_INSTALL_DIR/source"
bash ./install.sh "$@"
