#!/bin/sh
# Host-only coordination/oracle driver, never part of the serving path.
set -eu
cd "$(dirname "$0")/../.."
sh om/hive-render/check.sh
OMEGA_ACCEPT_CHANGE=1 sh om/hive-page/check.sh
sh om/hive-page/check.sh
sh om/hive-page/gate-fresh.sh
nice -n 19 cargo test -q -j2 -p hive-feed --bin hive-receive
nice -n 19 cargo build -q -j2 -p hive-feed --bin hive-receive
python3 om/hive-render/test-receive.py target/debug/hive-receive om/hive-render/hive-render
nice -n 19 cargo test -q -j2 -p site-builder
python3 scripts/test-hive-cosmic.py
