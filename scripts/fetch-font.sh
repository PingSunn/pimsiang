#!/usr/bin/env bash
set -euo pipefail

REVISION=7af0d90b4d532ffa4b041937b560c6df4e7b1e95
FONT_SHA256=7ec47ebcaa9d6459d974691cbb42fae52e1449478a607b071bfaf3ccdb433ce4
LICENSE_SHA256=cbc57ac0dddda1b70f246a16358fc3c2a32628a23a6d10ef64f746141d3fb159

cd "$(dirname "$0")/../crates/app/assets"
base="https://raw.githubusercontent.com/google/fonts/$REVISION/ofl/anuphan"
curl -fsSL -o "Anuphan[wght].ttf.part" "$base/Anuphan%5Bwght%5D.ttf"
echo "$FONT_SHA256  Anuphan[wght].ttf.part" | shasum -a 256 -c -
mv "Anuphan[wght].ttf.part" "Anuphan[wght].ttf"
curl -fsSL -o Anuphan-OFL.txt.part "$base/OFL.txt"
echo "$LICENSE_SHA256  Anuphan-OFL.txt.part" | shasum -a 256 -c -
mv Anuphan-OFL.txt.part Anuphan-OFL.txt
