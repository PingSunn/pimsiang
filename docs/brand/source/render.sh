#!/usr/bin/env bash
set -euo pipefail

source_dir="$(cd "$(dirname "$0")" && pwd)"
brand="$(dirname "$source_dir")"
root="$(cd "$brand/../.." && pwd)"
tool="$source_dir/render/target/release/brand-render"
export PATH="$HOME/.cargo/bin:$PATH"

cargo build --release --locked --manifest-path "$source_dir/render/Cargo.toml"
python3 "$source_dir/mascot.py"

png() { "$tool" png "$@"; }

exports="$brand/exports"
mkdir -p "$exports"
for svg in "$brand"/mascot/pimsiang-myna-*.svg; do
  name="$(basename "$svg" .svg)"
  case "$name" in
    *-head-*) png "$svg" "$exports/$name-256.png" 256 ;;
    *) png "$svg" "$exports/$name-512.png" 512 ;;
  esac
done
for svg in "$brand"/pimsiang-wordmark.svg "$brand"/pimsiang-wordmark-dark.svg; do
  png "$svg" "$exports/$(basename "$svg" .svg)-1600.png" 1600
done
for state in "" -listening -transcribing -attention; do
  for size in 18 36 54; do
    png "$brand/pimsiang-menubar$state.svg" "$exports/pimsiang-menubar$state-$size.png" "$size"
  done
done
png "$brand/pimsiang-icon-maskable.svg" "$exports/pimsiang-icon-maskable-1024.png" 1024
png "$brand/pimsiang-icon.svg" "$exports/pimsiang-icon-1024.png" 1024

iconset="$(mktemp -d)/PimSiang.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  png "$brand/pimsiang-icon.svg" "$iconset/icon_${size}x${size}.png" "$size"
  png "$brand/pimsiang-icon.svg" "$iconset/icon_${size}x${size}@2x.png" "$((size * 2))"
done
iconutil -c icns "$iconset" -o "$brand/PimSiang.icns"
rm -rf "$(dirname "$iconset")"
mkdir -p "$root/packaging/macos"
cp "$brand/PimSiang.icns" "$root/packaging/macos/PimSiang.icns"

assets="$root/crates/app/assets"
mkdir -p "$assets/mascot" "$assets/menubar" "$root/crates/platform-macos/assets"
for svg in "$brand"/mascot/pimsiang-myna-*.svg; do
  name="$(basename "$svg" .svg)"
  name="${name#pimsiang-myna-}"
  case "$name" in
    head-default | head-default-dark) png "$svg" "$assets/mascot/$name.png" 128 ;;
    default | default-dark | waving | waving-dark | writing | writing-dark | confused | confused-dark | sleeping | sleeping-dark | searching | searching-dark | oops | oops-dark)
      png "$svg" "$assets/mascot/$name.png" 360 ;;
  esac
done
for state in "" -listening -transcribing -attention; do
  png "$brand/pimsiang-menubar$state.svg" "$assets/menubar/glyph$state.png" 54
done
for state in listening writing confused default; do
  png "$brand/mascot/pimsiang-myna-head-$state-dark.svg" "$root/crates/platform-macos/assets/pill-$state.png" 96
done
