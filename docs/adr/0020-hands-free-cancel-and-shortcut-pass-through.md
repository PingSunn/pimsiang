# Hands-free starts on the second tap's release; Esc and shortcuts are read from a listen-only tap

Hands-free starts when the second tap of a double-tap is released within 400 ms of the first tap's release, and the next press of the Trigger key ends it. If the second press is held past the hold threshold it is Push-to-talk as before, so a tap followed by a hold never opens the microphone twice. The event tap (ADR 0013) now also receives key-down events: Esc cancels the current Utterance (during the hold threshold, Push-to-talk, Hands-free or transcribing), and any other key pressed while the Trigger key is down marks the press as part of a shortcut, which drops it before the microphone opens and cancels an Utterance already started by a slow hold. Esc and the shortcut key still reach the focused app because the tap stays listen-only, so Esc also acts on whatever app is in front.

While the previous Utterance is transcribing, Trigger key events are discarded rather than queued, so a new Utterance cannot start until the Final text is delivered and a key held through the transcription must be pressed again. Start and stop sounds are the system sounds Tink and Pop played through the platform trait; they are switched by a controller setting.

## Considered options

- **Start Hands-free on the second press:** a few tens of milliseconds sooner, but a tap followed by a hold would be ambiguous and the microphone would open for what the user means as Push-to-talk.
- **Active tap to swallow Esc while an Utterance is running:** stops Esc reaching other apps, but contradicts ADR 0013 and a slow callback would stall system-wide input.
- **Queue Trigger key presses during transcription:** nothing is lost, but a stale press would open the microphone after the text is pasted, which the spec forbids.
