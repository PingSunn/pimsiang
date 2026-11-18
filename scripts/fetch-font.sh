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

MITR_SHA256=257e8612923ed13838b2f0a0f9b25527aa83513c657df049a5ee608e6d4a059d
MITR_LICENSE_SHA256=427ae4ac208b3cb1930bc2398141bed5880f0c057af4d2fc91b2f7ba8dcffb7d
mitr="https://raw.githubusercontent.com/google/fonts/$REVISION/ofl/mitr"
curl -fsSL -o Mitr-SemiBold.ttf.part "$mitr/Mitr-SemiBold.ttf"
echo "$MITR_SHA256  Mitr-SemiBold.ttf.part" | shasum -a 256 -c -
mv Mitr-SemiBold.ttf.part Mitr-SemiBold.ttf
curl -fsSL -o Mitr-OFL.txt.part "$mitr/OFL.txt"
echo "$MITR_LICENSE_SHA256  Mitr-OFL.txt.part" | shasum -a 256 -c -
mv Mitr-OFL.txt.part Mitr-OFL.txt
