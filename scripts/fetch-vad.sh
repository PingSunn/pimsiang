#!/usr/bin/env bash
set -euo pipefail

REVISION=6478567951ae5c9979ad7b234185b5515f4be7a1
SHA256=2623a2953f6ff3d2c1e61740c6cdb7168133479b267dfef114a4a3cc5bdd788f

dest="${1:?usage: fetch-vad.sh <model-dir>}"
mkdir -p "$dest"
curl -fsSL -o "$dest/silero_vad.onnx.part" \
  "https://raw.githubusercontent.com/snakers4/silero-vad/$REVISION/src/silero_vad/data/silero_vad.onnx"
echo "$SHA256  $dest/silero_vad.onnx.part" | shasum -a 256 -c -
mv "$dest/silero_vad.onnx.part" "$dest/silero_vad.onnx"
