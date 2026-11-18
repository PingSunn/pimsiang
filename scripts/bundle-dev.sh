#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

APP="target/PimSiang.app"
CERT_NAME="PimSiang Self-Signed"

if [ -n "${PIMSIANG_SIGN_IDENTITY:-}" ]; then
  IDENTITY="$PIMSIANG_SIGN_IDENTITY"
elif security find-identity -v -p codesigning | grep -Fq "\"$CERT_NAME\""; then
  IDENTITY="$CERT_NAME"
else
  IDENTITY="-"
  echo "warning: signing ad hoc; macOS permission grants reset on every rebuild. Run scripts/make-signing-cert.sh once to fix this." >&2
fi

cargo build -q -p app --bin pimsiang
./scripts/bundle.sh target/debug/pimsiang "$APP" "$IDENTITY"
echo "$APP"
