#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if [ "$#" -ne 3 ]; then
  echo "usage: scripts/bundle.sh <binary> <app-dir> <identity>" >&2
  exit 2
fi

BINARY="$1"
APP="$2"
IDENTITY="$3"
BUNDLE_ID="com.pingsunn.pimsiang"
ICON="packaging/macos/PimSiang.icns"

VERSION="$(awk -F'"' '/^version *=/ { print $2; exit }' Cargo.toml)"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/en.lproj" "$APP/Contents/Resources/th.lproj"
cp "$BINARY" "$APP/Contents/MacOS/pimsiang"

icon_entry=""
if [ -f "$ICON" ]; then
  cp "$ICON" "$APP/Contents/Resources/PimSiang.icns"
  icon_entry="<key>CFBundleIconFile</key><string>PimSiang</string>"
fi

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundleName</key><string>PimSiang</string>
  <key>CFBundleDisplayName</key><string>PimSiang</string>
  <key>CFBundleExecutable</key><string>pimsiang</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleLocalizations</key><array><string>en</string><string>th</string></array>
  $icon_entry
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>LSUIElement</key><true/>
  <key>NSMicrophoneUsageDescription</key>
  <string>PimSiang listens while you hold the Trigger key so it can type what you say. Audio never leaves this Mac.</string>
</dict>
</plist>
PLIST

cat > "$APP/Contents/Resources/en.lproj/InfoPlist.strings" <<'STRINGS'
"NSMicrophoneUsageDescription" = "PimSiang listens while you hold the Trigger key so it can type what you say. Audio never leaves this Mac.";
STRINGS

cat > "$APP/Contents/Resources/th.lproj/InfoPlist.strings" <<'STRINGS'
"NSMicrophoneUsageDescription" = "พิมพ์เสียงฟังเสียงของคุณขณะที่กดปุ่ม Trigger ค้างไว้ เพื่อพิมพ์สิ่งที่คุณพูด เสียงไม่ออกจากเครื่อง Mac เครื่องนี้";
STRINGS

plutil -lint "$APP/Contents/Info.plist" > /dev/null
plutil -lint "$APP/Contents/Resources/en.lproj/InfoPlist.strings" "$APP/Contents/Resources/th.lproj/InfoPlist.strings" > /dev/null

sign_args=(--force --sign "$IDENTITY" --identifier "$BUNDLE_ID")
if [ -n "${PIMSIANG_SIGN_KEYCHAIN:-}" ]; then
  sign_args+=(--keychain "$PIMSIANG_SIGN_KEYCHAIN")
fi
codesign "${sign_args[@]}" "$APP"
