#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

BIN="$ROOT_DIR/target/release/v-connect-im"
CONF="$ROOT_DIR/config/node-b.toml"

if [[ ! -x "$BIN" ]]; then
  echo "Building release binary..."
  (cd "$ROOT_DIR" && cargo build --release)
fi

mkdir -p "$ROOT_DIR/data/v-connect-im-node-B"

echo "Starting node-B (http:8081, ws:5202)"
"$BIN" -c "$CONF"

