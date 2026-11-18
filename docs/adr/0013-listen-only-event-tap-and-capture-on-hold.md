# Listen-only event tap; the microphone opens when the hold threshold passes

The Trigger key is watched through a listen-only `CGEventTap` on `FlagsChanged` events, matching Right Option by key code and its device-specific flag bit. A listen-only tap cannot swallow or delay keys, so the key keeps its normal behaviour for every other app, and a stalled PimSiang cannot freeze the user's keyboard; if macOS disables the tap it is re-enabled and a held key is reported as released. The Utterance controller treats a press as Push-to-talk only after a short hold threshold (250 ms by default); only then does it open the microphone and start the Engine's Utterance. A release before the threshold is an ordinary tap and leaves no trace.

## Considered options

- **Open the microphone on key down and discard the audio on a short tap:** no lost first syllable, but the microphone indicator would flash on every Right Option tap, which people use for typing symbols.
- **Active (filtering) event tap:** could hide the key from other apps, but needs more trust and a slow callback would stall system-wide input.
