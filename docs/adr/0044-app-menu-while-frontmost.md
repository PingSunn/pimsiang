# 0044. The app menu routes Edit commands into egui itself

## Context

With a Dock icon, PimSiang is a normal frontmost app and the menu bar must offer Quit, Close and Copy/Paste. Its windows are egui surfaces on winit, so no AppKit responder implements `copy:`, `paste:` or `performClose:` for them. A native menu key equivalent such as Cmd+C is taken by the menu before winit sees the key, so a predefined Copy item would either be disabled or swallow the key without reaching egui. The egui-winit `clipboard` feature is also off, so egui's own Cmd+C and Cmd+V only ever reached an in-app buffer, never the system pasteboard.

## Decision

`main_menu::install` builds the main menu with `Menu::init_for_nsapp` after winit's default menu, using custom items with stable ids for Settings, Quit, Close and every Edit command, and muda's predefined items only where AppKit already routes correctly (About, Hide, Minimize). `app::app_menu` maps an id to an action and turns an Edit command into the egui events for it: `Event::Cut`, `Event::Copy`, `Event::Paste` with the pasteboard text, and Cmd+A, Cmd+Z and Shift+Cmd+Z key strokes. The event loop pushes them into the key window's egui input, so the History search box handles them exactly as typed input. Because the menu owns the key equivalents, each shortcut reaches the app once, as a menu event, and never also as a key event; the Esc and Cmd+W handling inside the windows stays for the cases where no menu is in play. egui's `CopyText` commands are also written to the system pasteboard, so Copy in the menu and Cmd+C leave real clipboard content. Quit goes through the same `event_loop.exit()` as the status item, not `terminate:`. Close closes the window that last became key.

The menu is installed once at startup and stays in every Show in Dock mode; AppKit keeps honouring its key equivalents even while the app is an Accessory, so the shortcuts behave the same without a Dock icon.

## Considered options

- **Predefined Copy, Paste and Select All items.** No AppKit responder in a winit view implements them, so they would be disabled or eat the keys.
- **Edit items without key equivalents.** Keys would reach egui natively, but the menu would show no shortcuts and still need the same injection for mouse clicks.
- **Enabling the egui-winit `clipboard` feature.** Adds the arboard dependency to fix a gap that a small pasteboard read in `platform-macos` closes.
- **Predefined Quit and Close Window.** They call `terminate:` and `performClose:`, bypassing the app's own quit and window bookkeeping.
