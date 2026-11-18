# 0041. End-to-end harness with synthetic events and TTS through the room microphone

## Context

Unit tests cannot cover the Trigger key, Hands-free, Esc, Quick note and paste paths end to end: they depend on the event tap, the real microphone, the ASR Model and the focused app.

## Decision

`scripts/e2e/run.sh` drives a running PimSiang (started with `scripts/run-dev.sh`). It posts synthetic key and click events from a small Swift tool (`keys.swift`, compiled on demand to `target/e2e/keys`), with the device-specific modifier bits the tap checks, and speaks Thai through `say -v Kanya` so the audio reaches the real microphone through the room. Each case asserts on observable output: TextEdit text, log lines, the clipboard and the History database. Spoken text is compared by character error rate (threshold 0.25), because speaker-to-microphone audio is noisy. The terminal needs Accessibility permission to post events. The harness takes over the keyboard, mouse and speakers, so it requires `--yes`, restores the clipboard and the keyboard input source, never changes settings and never deletes History rows. It is run by hand and is not part of `scripts/check.sh`.

## Considered options

- **Inject audio and events inside the process.** Deterministic, but it skips the event tap, the microphone and the paste path, which are the parts that break. Rejected.
- **Run it in `check.sh`.** It needs a running app, audio hardware and an idle machine, so it would make the gate flaky and intrusive. Rejected.
