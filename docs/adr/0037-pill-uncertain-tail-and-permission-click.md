# 0037. The Pill fades the unsettled tail and opens System Settings on a permission error

## Context

While listening, the Engine re-decodes the open segment on every Partial text, so its last words change; closed segments never do. The design shows settled words in full colour and the unsettled ones muted, and makes the Pill clickable when a permission is missing.

## Decision

`Engine::settled_len` reports the byte length of the closed-segment prefix of the text most recently returned by `push_audio` (0 by default, so other engines show everything as unsettled). The controller carries it in `PillState::Partial(PartialText)`; `PartialText::split` cuts at a character boundary. The text is split in the Language layer into `PillContent` (settled, tentative, caret, dots, badge) which the AppKit Pill draws as one attributed string.

The Pill stays a non-activating panel that ignores the mouse, except while the state is `Error(PermissionMissing)`. Then it accepts mouse events and a local `NSEvent` monitor for left mouse down, filtered by window number, opens the System Settings pane for that permission and swallows the event. No window subclass is needed and the app never activates. `platform::pill_click` decides, purely, which states are clickable.

## Considered options

- Subclassing `NSView` or `NSPanel` for `mouseDown`: needs `define_class!` and more unsafe code for the same effect.
- Changing `push_audio` to return the split: breaks every Engine implementation and test for a display detail.
