#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

export PIMSIANG_MODEL_DIR="${PIMSIANG_MODEL_DIR:-$PWD/models/int8}"

if [ "${1:-}" = "--plain" ]; then
  cargo build -q -p app
  exec ./target/debug/pimsiang
fi

BUNDLE_ID="com.pingsunn.pimsiang"

app="$PWD/$(./scripts/bundle-dev.sh)"

if pgrep -f "$app/Contents/MacOS/pimsiang" > /dev/null; then
  osascript -e "quit app id \"$BUNDLE_ID\"" > /dev/null 2>&1 || true
  for _ in $(seq 1 50); do
    pgrep -f "$app/Contents/MacOS/pimsiang" > /dev/null || break
    sleep 0.1
  done
  if pgrep -f "$app/Contents/MacOS/pimsiang" > /dev/null; then
    echo "PimSiang did not quit within 5 seconds; quit it from the menu bar and run again." >&2
    exit 1
  fi
fi
exec open "$app" --env "PIMSIANG_MODEL_DIR=$PIMSIANG_MODEL_DIR"
