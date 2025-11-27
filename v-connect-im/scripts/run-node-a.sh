#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

BIN="$ROOT_DIR/target/release/v-connect-im"
CONF="$ROOT_DIR/config/node-a.toml"

if [[ ! -x "$BIN" ]]; then
  echo "Building release binary..."
  (cd "$ROOT_DIR" && cargo build --release)
fi

mkdir -p "$ROOT_DIR/data/v-connect-im-node-A"

echo "Starting node-A (http:8080, ws:5200)"
"$BIN" -c "$CONF"

