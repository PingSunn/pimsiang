# 0041. The Pill can float where the user puts it

## Context

The Pill was always drawn out of the notch (ADR 0015). Some users run apps that live in the notch, such as Vorssaint, and the Pill covered them. Others do not want a Pill at the top at all.

## Decision

There are two Pill placements. Notch is the old behaviour. Floating is a fully rounded capsule, with the same content and states, at a position the user chose. Floating is the default for new installs and for existing users with no saved value, at bottom centre, because it never collides with a notch app.

The position is stored as a fraction of the visible frame of the screen (x from the left, y from the bottom, for the Pill's centre), for example `pill_position=0.5,0.12`, next to `pill_placement=floating` in the plain `settings` file (ADR 0025). A fraction lands in the same relative place on any display and survives a change of resolution or Dock size. It is applied to the screen that has the mouse pointer, as before. The frame is clamped so the Pill is always fully on screen; as Partial text widens it, it grows around the saved centre and shifts only when it would cross an edge. A bad stored value falls back to the default.

The user sets the position in Settings > General, "Pill position", with a thumbnail that follows the mode and the position. "Position…" enters a placement mode: a sample Pill appears on the pointer's screen, accepts the mouse and can be dragged, with a small hint and a Done button beside it. Done saves; Esc saves if the user dragged and cancels otherwise. Dragging snaps to the horizontal centre within 12 points. Outside the placement mode the Pill still ignores the mouse, except for the missing-permission click of ADR 0037. The panel stays non-activating and never becomes key, so focus is not taken.

The frame arithmetic, the placement state machine and the mouse rule are pure functions in the `platform` crate so they are tested without AppKit.

## Considered options

- **An always-draggable Pill:** simpler to explain, but it would catch clicks meant for the app behind it during dictation and make it easy to move by accident. Placement mode keeps the Pill click-through the rest of the time.
- **Absolute screen coordinates or an offset from an edge:** wrong after a display change and meaningless on a second screen of another size.
- **Notch as the default:** keeps today's look but collides with notch apps, which is the reason for this change.
