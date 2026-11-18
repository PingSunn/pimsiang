#!/usr/bin/env bash
# Test cases for scripts/e2e/run.sh. Sourced, not executed. Each case runs in
# a subshell with errexit; assertions call die (FAIL) or skip (SKIP).

SENTENCE_A="วันนี้อากาศดีมากเราไปเดินเล่นที่สวนสาธารณะกัน"
SENTENCE_B="ฉันอยากสั่งกาแฟร้อนหนึ่งแก้วและขนมปังปิ้งหนึ่งชิ้น"
SENTENCE_NOTE="จดไว้ว่าพรุ่งนี้ต้องส่งรายงานให้เสร็จก่อนเที่ยง"
LONG_SENTENCES=(
  "เมื่อเช้านี้ฉันตื่นนอนแต่เช้าแล้วออกไปวิ่งที่สวนสาธารณะใกล้บ้าน"
  "หลังจากนั้นฉันแวะซื้อกาแฟและขนมปังที่ร้านประจำก่อนกลับมาทำงาน"
  "ทีมของเรากำลังพัฒนาโปรแกรมพิมพ์ตามคำพูดสำหรับภาษาไทยบนเครื่องแมค"
  "การประชุมสัปดาห์หน้าจะพูดถึงแผนงานและกำหนดส่งงานของทุกคนในทีม"
  "ฉันคิดว่าเราควรทดสอบโปรแกรมกับเสียงจริงหลายแบบเพื่อให้แน่ใจว่าใช้งานได้ดี"
  "ตอนเย็นฝนตกหนักมากทำให้การจราจรติดขัดไปทั่วทั้งเมือง"
  "ขอบคุณทุกคนที่ช่วยกันตรวจสอบและให้ความเห็นเกี่ยวกับเอกสารฉบับนี้"
  "พรุ่งนี้ฉันจะส่งรายงานสรุปผลให้หัวหน้าตรวจก่อนเวลาเที่ยงวัน"
)

hold_and_speak() {
  keys down:ropt sleep:1.2
  say_th "$1"
  keys sleep:0.4 up:ropt
}

case_ptt() {
  textedit_new
  local mark count
  mark="$(log_mark)"
  count="$(db_count)"
  hold_and_speak "$SENTENCE_A"
  wait_log "$mark" 'pasted Final text' || die "no 'pasted Final text' line within ${PASTE_TIMEOUT}s"
  sleep 0.8
  assert_cer "$SENTENCE_A" "$(textedit_text)" "pasted text"
  [ "$(db_count)" -eq $((count + 1)) ] || die "History did not gain exactly one entry"
  [ "$(db_last kind)" = pasted ] || die "newest History entry has kind '$(db_last kind)', expected pasted"
}

case_quick_tap() {
  textedit_new
  local mark count
  mark="$(log_mark)"
  count="$(db_count)"
  keys down:ropt sleep:0.1 up:ropt
  sleep 3
  assert_log_lacks "$mark" 'pasted Final text' "a short tap pasted"
  [ -z "$(textedit_text)" ] || die "TextEdit has text after a short tap"
  [ "$(db_count)" -eq "$count" ] || die "History changed after a short tap"
}

case_hands_free() {
  textedit_new
  local mark
  mark="$(log_mark)"
  double_tap
  sleep 1.2
  say_th "$SENTENCE_A"
  sleep 0.4
  tap
  wait_log "$mark" 'pasted Final text' || die "no 'pasted Final text' line within ${PASTE_TIMEOUT}s"
  sleep 0.8
  assert_cer "$SENTENCE_A" "$(textedit_text)" "pasted text"
}

case_esc_ptt() {
  textedit_new
  local mark count
  mark="$(log_mark)"
  count="$(db_count)"
  keys down:ropt sleep:1.2
  say_th "$SENTENCE_A"
  keys esc sleep:0.5 up:ropt
  sleep 3
  assert_log_has "$mark" 'cancelled the Utterance' "Esc while holding did not cancel"
  assert_log_lacks "$mark" 'pasted Final text' "Esc while holding still pasted"
  [ -z "$(textedit_text)" ] || die "TextEdit has text after Esc"
  [ "$(db_count)" -eq "$count" ] || die "History changed after Esc"
}

case_esc_hands_free() {
  textedit_new
  local mark count cancelled=0 pasted=0 text recovery_mark
  mark="$(log_mark)"
  count="$(db_count)"
  double_tap
  sleep 1.2
  say_th "$SENTENCE_A"
  keys esc
  sleep 3
  log_has "$mark" 'cancelled the Utterance' && cancelled=1
  log_has "$mark" 'pasted Final text' && pasted=1
  text="$(textedit_text)"
  recovery_mark="$(log_mark)"
  if [ "$cancelled" -eq 0 ]; then
    tap
    wait_log "$recovery_mark" 'pasted Final text|Utterance produced no text|kept' 10 || true
  fi
  [ "$cancelled" -eq 1 ] || die "Esc in Hands-free did not cancel the Utterance (suspected bug)"
  [ "$pasted" -eq 0 ] || die "Esc in Hands-free still pasted"
  [ -z "$text" ] || die "TextEdit has text after Esc in Hands-free"
  [ "$(db_count)" -eq "$count" ] || die "History changed after Esc in Hands-free"
}

case_option_combo() {
  textedit_new
  local mark count
  mark="$(log_mark)"
  count="$(db_count)"
  keys down:ropt sleep:0.3 e sleep:0.5 up:ropt
  sleep 3
  assert_log_lacks "$mark" 'pasted Final text|kept a Quick note|Utterance produced no text|cancelled|as a Note' "Right Option + e started an Utterance"
  [ "$(db_count)" -eq "$count" ] || die "History changed after Right Option + e"
}

case_quick_note() {
  textedit_new
  local mark count sentinel="e2e-sentinel-quick-note"
  clipboard_set "$sentinel"
  mark="$(log_mark)"
  count="$(db_count)"
  keys down:shift down:ropt sleep:1.2
  say_th "$SENTENCE_NOTE"
  keys sleep:0.4 up:ropt up:shift
  wait_log "$mark" 'kept a Quick note' || die "no 'kept a Quick note' line within ${PASTE_TIMEOUT}s"
  sleep 1.5
  [ "$(pbpaste)" = "$sentinel" ] || die "the clipboard changed during a Quick note"
  [ "$(db_count)" -eq $((count + 1)) ] || die "History did not gain exactly one entry"
  [ "$(db_last kind)" = note ] || die "newest History entry has kind '$(db_last kind)', expected note"
  assert_cer "$SENTENCE_NOTE" "$(db_last text)" "note text"
  [ -z "$(textedit_text)" ] || die "a Quick note was pasted into TextEdit"
}

case_clipboard_restore() {
  [ "$(setting clipboard_restore)" = on ] || skip "the clipboard_restore setting is not on"
  textedit_new
  local mark sentinel="e2e-sentinel-clipboard-restore"
  clipboard_set "$sentinel"
  mark="$(log_mark)"
  hold_and_speak "$SENTENCE_B"
  wait_log "$mark" 'pasted Final text' || die "no 'pasted Final text' line within ${PASTE_TIMEOUT}s"
  sleep 2
  [ "$(pbpaste)" = "$sentinel" ] || die "the clipboard was not restored (holds '$(pbpaste)')"
}

paste_last_with() {
  local second="$1" last="$2" mark text waited=0
  textedit_new
  mark="$(log_mark)"
  keys down:ctl "down:$second" sleep:0.2 v sleep:0.2 "up:$second" up:ctl
  while [ "$waited" -lt 50 ]; do
    text="$(textedit_text)"
    [ "$text" = "$last" ] && break
    sleep 0.1
    waited=$((waited + 1))
  done
  [ "$text" = "$last" ] || die "Control + $second + V pasted '$text', expected the last entry"
  sleep 1
  assert_log_lacks "$mark" 'pasted Final text|kept a Quick note|Utterance produced no text|cancelled|as a Note' "Control + $second + V started an Utterance"
  textedit_close
}

case_paste_last() {
  local last
  last="$(db_last text)"
  [ -n "$last" ] || skip "History is empty; run ptt first"
  paste_last_with lopt "$last"
  paste_last_with ropt "$last"
}

case_finder_note() {
  finder_front
  local mark count
  mark="$(log_mark)"
  count="$(db_count)"
  hold_and_speak "$SENTENCE_NOTE"
  wait_log "$mark" 'keeping the Final text as a Note' || die "no 'as a Note' line within ${PASTE_TIMEOUT}s"
  sleep 1
  [ "$(db_count)" -eq $((count + 1)) ] || die "History did not gain exactly one entry"
  [ "$(db_last kind)" = note ] || die "newest History entry has kind '$(db_last kind)', expected note"
  [ "$(pbpaste)" = "$(db_last text)" ] || die "the Note was not copied to the clipboard"
  assert_cer "$SENTENCE_NOTE" "$(db_last text)" "note text"
}

case_long_hands_free() {
  textedit_new
  local mark spoken="" start stop_ms done_ms index=0 shots_pid latency
  mark="$(log_mark)"
  rm -f "$E2E_DIR"/long-*.png
  double_tap
  sleep 1.2
  (
    n=0
    while :; do
      screencapture -x "$E2E_DIR/long-$(printf '%02d' "$n").png" 2> /dev/null || true
      n=$((n + 1))
      sleep 10
    done
  ) &
  shots_pid=$!
  start="$(now_ms)"
  while [ $(( $(now_ms) - start )) -lt 60000 ]; do
    sentence="${LONG_SENTENCES[$((index % ${#LONG_SENTENCES[@]}))]}"
    say_th "$sentence"
    spoken="$spoken$sentence"
    index=$((index + 1))
  done
  kill "$shots_pid" 2> /dev/null || true
  wait "$shots_pid" 2> /dev/null || true
  stop_ms="$(now_ms)"
  tap
  wait_log "$mark" 'pasted Final text' 20 || die "no 'pasted Final text' line within 20s of stopping"
  done_ms="$(now_ms)"
  latency=$((done_ms - stop_ms))
  info "spoke $(( (stop_ms - start) / 1000 ))s, paste ${latency}ms after stop, screenshots in target/e2e/long-NN.png"
  [ "$latency" -le 3000 ] || die "paste arrived ${latency}ms after stopping (limit 3000ms)"
  sleep 0.8
  local want got
  want="$(printf '%s' "$spoken" | tr -d '[:space:]' | python3 -I -c 'import sys; print(len(sys.stdin.read()))')"
  got="$(textedit_text | tr -d '[:space:]' | python3 -I -c 'import sys; print(len(sys.stdin.read()))')"
  [ "$((got * 100))" -ge "$((want * 80))" ] || die "pasted $got characters, under 80% of the $want spoken"
}
