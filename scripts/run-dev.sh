#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
cargo build -q -p app
PIMSIANG_MODEL_DIR="$PWD/models/int8" exec ./target/debug/pimsiang
