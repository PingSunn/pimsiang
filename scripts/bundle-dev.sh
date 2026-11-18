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
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp target/debug/pimsiang "$APP/Contents/MacOS/pimsiang"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>com.pingsunn.pimsiang</string>
  <key>CFBundleName</key><string>PimSiang</string>
  <key>CFBundleDisplayName</key><string>PimSiang</string>
  <key>CFBundleExecutable</key><string>pimsiang</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>LSUIElement</key><true/>
  <key>NSMicrophoneUsageDescription</key>
  <string>PimSiang listens while you hold Right Option so it can type what you say. Audio never leaves this Mac.</string>
</dict>
</plist>
PLIST
codesign --force --sign "$IDENTITY" --identifier com.pingsunn.pimsiang "$APP"
echo "$APP"
