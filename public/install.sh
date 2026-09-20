#!/usr/bin/env bash
# AIEN OS Universal Installer Entrypoint
# Fetches and executes the canonical multi-platform installer from aien-sovereign-core.
set -euo pipefail
exec curl -fsSL https://raw.githubusercontent.com/aien-dev/aien-sovereign-core/main/install.sh | bash "$@"
