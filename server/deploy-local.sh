#!/usr/bin/env bash
set -euo pipefail

server_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
data_dir="${AIENOS_WAITLIST_DATA_DIR:-$HOME/.local/share/aienos-waitlist}"

cargo build --release --manifest-path "$server_dir/Cargo.toml"
install -d -m 0700 "$data_dir"
docker build -t aienos-waitlist:local -f "$server_dir/Dockerfile" "$server_dir"

if docker container inspect aienos-waitlist >/dev/null 2>&1; then
  docker rm -f aienos-waitlist
fi

docker run -d \
  --name aienos-waitlist \
  --restart unless-stopped \
  --read-only \
  --cap-drop ALL \
  --security-opt no-new-privileges \
  --user "$(id -u):$(id -g)" \
  --pids-limit 128 \
  --memory 256m \
  -p 127.0.0.1:18171:18171 \
  -e WAITLIST_BIND=0.0.0.0:18171 \
  -e WAITLIST_DB_PATH=/data/waitlist.sqlite3 \
  -v "$data_dir:/data:rw" \
  aienos-waitlist:local

curl --fail --silent --show-error --retry 10 --retry-delay 1 --retry-all-errors \
  http://127.0.0.1:18171/healthz
