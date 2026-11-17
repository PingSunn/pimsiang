# Native AppKit Pill, egui for the History and settings windows

The Pill must float above every app without taking focus and sit in or under the notch, which no Rust GUI toolkit does well, so on macOS it is a non-activating `NSPanel` driven through `objc2`. It appears at the top centre of the screen holding the mouse cursor or focused window, coming out of the notch where there is one. The History and settings windows use egui, which runs on every target platform, with the Anuphan font (OFL) embedded because egui has no Thai glyphs. The menu bar icon uses `tray-icon`. UI strings are in Thai and English, following the system language.

## Considered options

- **egui (or iced/Slint) for the Pill too:** one toolkit, but cannot make a non-activating, notch-aware panel.
- **AppKit for every window:** the most native look, but the History and settings UI would be rewritten for each later platform.
- **System Thonburi font instead of embedding one:** smaller app, but macOS-only and read from a system path.
