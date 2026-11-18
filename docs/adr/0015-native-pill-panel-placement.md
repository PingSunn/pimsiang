# Pill panel placement and threading

The Pill is a borderless `NSPanel` with the non-activating style mask, floating at status window level, joining all Spaces and allowed over full-screen apps, ignoring the mouse and never becoming key, so the target app keeps keyboard focus and the paste lands there. It is created lazily on the main thread (winit's event loop); the controller thread sends `PillState` values through the event loop proxy. The screen is the one containing the mouse cursor, falling back to the main screen; the focused window's display is not consulted because the cursor display needs no Accessibility call and covers the common case. On a notched display (`safeAreaInsets.top > 0`, notch width from the two auxiliary top areas) the panel is flush with the physical top edge, as wide as the notch at least, with its text row below the notch, so it reads as the notch growing downward. Elsewhere it sits just below the menu bar. Text is a single `NSTextField` label truncated at the head so the newest words stay visible; the system font cascade renders Thai. The black background is an `NSBox` with a custom fill, which avoids Core Animation layers and `unsafe`.

The controller owns the Pill timing: it emits `Hidden` after a short linger (400 ms, 3 s for errors) via its tick deadline, so the sequence is testable with the fake Platform. Error messages are a closed `PillError` enum with English text for now.

## Considered options

- **Focused window's display via Accessibility:** matches the wording "cursor or focused window" more literally, but adds a permission-dependent call on every Utterance for little gain.
- **Pill hides itself in the macOS layer:** simpler controller, but the hide timing could not be asserted in tests.
- **Layer-backed view with masked corners:** exact notch shape, but needs Core Animation and Core Graphics bindings for a cosmetic gain.
