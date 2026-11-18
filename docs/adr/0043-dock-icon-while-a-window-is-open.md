# 0043. The Dock icon shows while a window is open

## Context

PimSiang is a menu bar app (`LSUIElement`), so it had no Dock icon and no Cmd+Tab entry, even with Setup, Settings or History open. On a crowded menu bar the status item can sit behind the overflow control, leaving no visible sign that the app is running and no way back to an open window.

## Decision

The event loop switches the activation policy at runtime between Regular (Dock icon, Cmd+Tab) and Accessory. A pure rule, `dock_visible(ShowInDock, any_window_open)`, decides which one applies, and `DockState` reports a change only when the result differs, so `setActivationPolicy` is called only on a real transition. `ShowInDock` has three values (always, while a window is open, never) and defaults to while a window is open. Only Setup, Settings and History count as windows; the Pill and the status item never do. Opening a window shows the icon and brings that window to the front; closing the last one hides it without moving anything. Clicking the Dock icon focuses the most recent window, through `applicationShouldHandleReopen:hasVisibleWindows:` added to winit's application delegate class, because winit 0.30 does not expose it. `LSUIElement` stays in the bundle so launch is unchanged. The preference itself comes in a later ticket; until then the app passes the default.

The default is "while a window is open" because the app's normal life is in the menu bar, and a permanent Dock icon would be clutter for a tool used through a key.

## Considered options

- **Always shown.** Easy to find, but it puts a permanent icon in the Dock for an app that is meant to stay out of the way.
- **Never shown (status quo).** Leaves the problem: an open window behind other apps cannot be reached, and a hidden status item gives no sign the app runs.
- **A separate toggle only, with no window-based default.** Users who never open Settings would never find it, so the new behaviour would not help the people who hit the problem.
