# 0036. Paste the last transcript with a global hotkey, and menu key equivalents

## Context

The menu bar menu lists "Paste last transcript" with the shortcut Control Option V, plus Command comma, Command Y and Command Q for Settings, History and Quit. The Trigger key is watched by a listen-only event tap on its own thread; Trigger key handling must not change.

## Decision

Control Option V is registered as a system-wide hotkey with the `global-hotkey` crate (Carbon `RegisterEventHotKey` underneath) on the main thread when the app starts, and unregistered when the app exits. A press is matched by a pure function (`shortcuts::is_paste_last`) and forwarded to the event loop as the same action as the menu item. If the combination is already taken, registration fails, a warning is logged and the menu item still works.

The menu items carry the key equivalents Command comma, Command Y and Command Q (and Control Option V for display). macOS only dispatches these while the menu is open, so they are discoverable shortcuts rather than global ones.

## Considered options

- **Extend the listen-only event tap.** No new dependency, but a listen-only tap cannot swallow the key, so Control Option V would also reach the focused app, and the shortcut would depend on the Input Monitoring permission that Setup may still be waiting for. Rejected.
- **Make the tap active (default options).** It could swallow the key but would put the Trigger key path on a tap that can block all keyboard input if it stalls. Rejected.
- **Direct Carbon FFI.** Same mechanism as the crate with a large unsafe surface to maintain. Rejected.
