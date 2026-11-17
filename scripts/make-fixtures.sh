#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$root/crates/engine/tests/fixtures"
voice="${PIMSIANG_FIXTURE_VOICE:-Kanya}"

while IFS=$'\t' read -r name text; do
  [ -z "$name" ] && continue
  say -v "$voice" -o "$dir/$name.wav" --data-format=LEI16@16000 "$text"
done < "$dir/transcripts.tsv"

say -v "$voice" -o "$dir/weather-44k.wav" --data-format=LEI16@44100 \
  "$(awk -F'\t' '$1=="weather"{print $2}' "$dir/transcripts.tsv")"

echo "fixtures written to $dir"
