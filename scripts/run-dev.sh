#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

export PIMSIANG_MODEL_DIR="${PIMSIANG_MODEL_DIR:-$PWD/models/int8}"

if [ "${1:-}" = "--plain" ]; then
  cargo build -q -p app
  exec ./target/debug/pimsiang
fi

app="$(./scripts/bundle-dev.sh)"
exec open "$app" --env "PIMSIANG_MODEL_DIR=$PIMSIANG_MODEL_DIR"
