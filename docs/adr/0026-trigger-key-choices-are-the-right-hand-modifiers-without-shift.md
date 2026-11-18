# The Trigger key is one of three right-hand modifiers, never Shift or Fn

The settings offer Right Option (default), Right Command and Right Control. The event tap stays listen-only (ADR 0013) and keeps watching `flagsChanged`; the key to watch lives in a shared atomic that `MacPlatform::set_trigger_key` writes and the tap callback reads on every event, so a change applies to the running tap without stopping or recreating it and without a restart. Each key is told apart by its key code (`0x3D`, `0x36`, `0x3E`) and its device-dependent flag bit (`0x40`, `0x10`, `0x2000`), so the left-hand twin of the same modifier is ignored. If the key changes while the old one is held, the tap emits a release so the controller cannot be left holding.

Shift is deliberately not an option: Shift held with the Trigger key is how a Quick note starts (ADR 0005), so a Trigger key that is itself Shift could not be combined with it. Because the tap reports the Trigger key independently of Shift, the Quick note rule is the same whichever of the three keys is chosen. Fn/🌐 stays excluded for the reasons in ADR 0005, and the left-hand modifiers are excluded because they are part of everyday shortcuts. Any other key pressed while the Trigger key is down still cancels the Utterance and passes through (ADR 0020), which is what keeps Right Command usable as a Trigger key for shortcuts.

## Considered options

- **Any key, recorded by pressing it:** the most flexible, but needs key capture UI, and ordinary keys would type into the focused field while held.
- **Including Right Shift:** conflicts with the Shift rule for Quick note.
- **Recreating the event tap on change:** works, but briefly leaves no tap and needs the same permission handshake again.
