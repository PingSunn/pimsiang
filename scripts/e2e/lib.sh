#!/usr/bin/env bash
# Helpers for scripts/e2e/run.sh. Sourced, not executed.

export PYTHONUTF8=1
E2E_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
E2E_DIR="$E2E_ROOT/target/e2e"
KEYS="$E2E_DIR/keys"
DB="$HOME/Library/Application Support/PimSiang/history.sqlite"
SETTINGS="$HOME/Library/Application Support/PimSiang/settings"
RESULT_FILE="$E2E_DIR/.result"
INFO_FILE="$E2E_DIR/.info"
CER_MAX="${E2E_CER_MAX:-0.25}"
PASTE_TIMEOUT="${E2E_PASTE_TIMEOUT:-30}"

SAVED_CLIPBOARD="$E2E_DIR/.clipboard"
SAVED_SOURCE=""

die() { echo "FAIL $*" > "$RESULT_FILE"; exit 1; }
skip() { echo "SKIP $*" > "$RESULT_FILE"; exit 0; }
info() { echo "$*" >> "$INFO_FILE"; }

now_ms() { perl -MTime::HiRes=time -e 'printf "%d\n", time * 1000'; }

build_keys() {
  mkdir -p "$E2E_DIR"
  if [ ! -x "$KEYS" ] || [ "$E2E_ROOT/scripts/e2e/keys.swift" -nt "$KEYS" ]; then
    swiftc -O "$E2E_ROOT/scripts/e2e/keys.swift" -o "$KEYS"
  fi
}

keys() { "$KEYS" "$@"; }

tap() { keys down:ropt sleep:0.08 up:ropt; }
double_tap() { keys down:ropt sleep:0.06 up:ropt sleep:0.12 down:ropt sleep:0.06 up:ropt; }

log_file() { echo "$HOME/Library/Logs/PimSiang/pimsiang.log.$(date -u +%Y-%m-%d)"; }
log_mark() {
  local file
  file="$(log_file)"
  [ -f "$file" ] || : > "$file"
  echo "$file:$(wc -l < "$file" | tr -d ' ')"
}
log_since() {
  local file="${1%:*}" lines="${1##*:}" current
  current="$(log_file)"
  if [ "$current" != "$file" ]; then file="$current"; lines=0; fi
  [ -f "$file" ] || return 0
  tail -n +"$((lines + 1))" "$file" | grep -v 'ort::logging' || true
}
log_has() { log_since "$1" | grep -qE -- "$2"; }
wait_log() {
  local mark="$1" pattern="$2" timeout="${3:-$PASTE_TIMEOUT}" waited=0
  while [ "$waited" -lt $((timeout * 10)) ]; do
    log_has "$mark" "$pattern" && return 0
    sleep 0.1
    waited=$((waited + 1))
  done
  return 1
}
assert_log_has() { log_has "$1" "$2" || die "${3:-log line missing}: /$2/"; }
assert_log_lacks() { ! log_has "$1" "$2" || die "${3:-unexpected log line}: /$2/"; }

textedit_new() {
  osascript -e 'tell application "TextEdit" to activate' \
    -e 'tell application "TextEdit" to make new document' \
    -e 'delay 0.6' > /dev/null
}
textedit_text() { osascript -e 'tell application "TextEdit" to get text of front document'; }
textedit_close() {
  osascript -e 'tell application "TextEdit" to close front document saving no' > /dev/null 2>&1 || true
}
finder_front() { osascript -e 'tell application "Finder" to activate' -e 'delay 0.5' > /dev/null; }

clipboard_save() { pbpaste > "$SAVED_CLIPBOARD" 2> /dev/null || : > "$SAVED_CLIPBOARD"; }
clipboard_restore() { [ -f "$SAVED_CLIPBOARD" ] && pbcopy < "$SAVED_CLIPBOARD"; }
clipboard_set() { printf '%s' "$1" | pbcopy; }

db_count() { sqlite3 -readonly "$DB" 'select count(*) from entries'; }
db_last() { sqlite3 -readonly "$DB" "select $1 from entries order by created desc, id desc limit 1"; }
setting() { sed -n "s/^$1=//p" "$SETTINGS" 2> /dev/null | head -n 1; }

say_th() { say -v Kanya -r 170 "$1"; }

cer() {
  python3 -I -c '
import sys
ref = "".join(sys.argv[1].split())
hyp = "".join(sys.argv[2].split())
prev = list(range(len(hyp) + 1))
for i, r in enumerate(ref, 1):
    cur = [i]
    for j, h in enumerate(hyp, 1):
        cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (r != h)))
    prev = cur
print("%.3f" % (prev[-1] / max(len(ref), 1)))
' "$1" "$2"
}
assert_cer() {
  local spoken="$1" heard="$2" label="${3:-text}" value
  value="$(cer "$spoken" "$heard")"
  info "$label CER $value (max $CER_MAX)"
  python3 -I -c 'import sys; sys.exit(0 if float(sys.argv[1]) <= float(sys.argv[2]) else 1)' "$value" "$CER_MAX" \
    || die "$label CER $value above $CER_MAX; spoke \"$spoken\", heard \"$heard\""
}

failure_screenshot() { screencapture -x "$E2E_DIR/$1-fail.png" 2> /dev/null || true; }

save_state() {
  clipboard_save
  SAVED_SOURCE="$(keys source-get 2> /dev/null || true)"
}
restore_state() {
  keys releaseall > /dev/null 2>&1 || true
  clipboard_restore || true
  if [ -n "$SAVED_SOURCE" ]; then keys "source-set:$SAVED_SOURCE" > /dev/null 2>&1 || true; fi
}

preflight() {
  [ "$(uname)" = Darwin ] || { echo "macOS only" >&2; return 1; }
  local tool
  for tool in swiftc python3 sqlite3 say osascript screencapture pbcopy perl; do
    command -v "$tool" > /dev/null || { echo "missing tool: $tool" >&2; return 1; }
  done
  say -v '?' | grep -q '^Kanya' || { echo "the Thai voice Kanya is not installed" >&2; return 1; }
  pgrep -x pimsiang > /dev/null || { echo "PimSiang is not running; start it with scripts/run-dev.sh" >&2; return 1; }
  [ -f "$DB" ] || { echo "no History database at $DB" >&2; return 1; }
  [ "$(setting trigger_key)" = right-option ] \
    || { echo "the Trigger key must be Right Option (settings say '$(setting trigger_key)'); change it yourself" >&2; return 1; }
}
