#!/usr/bin/env bash
# End-to-end harness: drives the running PimSiang with synthetic keys and Thai
# TTS through the room microphone. See docs/adr/0041-e2e-harness-synthetic-events-and-tts.md.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=scripts/e2e/lib.sh
source "$here/lib.sh"
# shellcheck source=scripts/e2e/cases.sh
source "$here/cases.sh"

ALL_CASES=(ptt quick_tap hands_free esc_ptt esc_hands_free option_combo quick_note
  clipboard_restore paste_last finder_note long_hands_free)

usage() {
  cat << USAGE
usage: scripts/e2e/run.sh --yes [case...]
       scripts/e2e/run.sh --list

Runs the named cases, or all of them. The app must already be running
(scripts/run-dev.sh) with the Trigger key set to Right Option.

This takes over the keyboard, mouse and speakers and needs Accessibility for
the terminal. Do not touch the machine while it runs.

cases: ${ALL_CASES[*]}
Screenshots on failure, and the long_hands_free series, go to target/e2e/.
USAGE
}

yes=0
selected=()
for arg in "$@"; do
  case "$arg" in
    --yes) yes=1 ;;
    --list) printf '%s\n' "${ALL_CASES[@]}"; exit 0 ;;
    -h | --help) usage; exit 0 ;;
    -*) echo "unknown option $arg" >&2; usage >&2; exit 2 ;;
    *) selected+=("$arg") ;;
  esac
done
[ "${#selected[@]}" -gt 0 ] || selected=("${ALL_CASES[@]}")

for name in "${selected[@]}"; do
  declare -F "case_$name" > /dev/null || { echo "unknown case: $name" >&2; usage >&2; exit 2; }
done

if [ "$yes" -ne 1 ]; then
  echo "WARNING: this harness takes over the keyboard, mouse and speakers while it runs." >&2
  echo "Do not use the machine meanwhile. Re-run with --yes to proceed." >&2
  exit 2
fi

build_keys || exit 1
preflight || exit 1
rm -f "$RESULT_FILE" "$INFO_FILE"
save_state
trap restore_state EXIT

passed=0
failed=0
skipped=0
for name in "${selected[@]}"; do
  rm -f "$RESULT_FILE" "$INFO_FILE"
  ( set -e; "case_$name" )
  code=$?
  keys releaseall > /dev/null 2>&1 || true
  if [ -f "$RESULT_FILE" ]; then
    status="$(cut -d' ' -f1 "$RESULT_FILE")"
    reason="$(cut -d' ' -f2- "$RESULT_FILE")"
  elif [ "$code" -eq 0 ]; then
    status=PASS
    reason="$(paste -sd';' "$INFO_FILE" 2> /dev/null || true)"
  else
    status=FAIL
    reason="case exited with status $code"
  fi
  case "$status" in
    PASS) passed=$((passed + 1)) ;;
    SKIP) skipped=$((skipped + 1)) ;;
    *) failed=$((failed + 1)); failure_screenshot "$name" ;;
  esac
  printf '%-4s %-18s %s\n' "$status" "$name" "$reason"
  textedit_close
  sleep 1
done

echo "passed $passed, failed $failed, skipped $skipped"
[ "$failed" -eq 0 ]
