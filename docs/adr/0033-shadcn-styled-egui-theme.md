# 0033: shadcn-styled egui theme

Partly superseded by 0034: the colours, layout and identity changed, the egui theme-module approach did not.

The setup, Settings and History windows used egui's default look, which the user found ugly. We give every egui window one shared theme module (`crates/app/src/theme.rs`) that carries the shadcn/ui zinc design tokens (colours, 8 px radius, spacing and type scales) for light and dark, applied through `egui::Style` and `Visuals`, plus small widget helpers (buttons, card, input, switch, select, badge, progress). The windows follow the system appearance through winit's theme events, which egui-winit feeds to egui. The Pill keeps a dark zinc-950 surface with a 1 px zinc-800 border and a fully rounded shape in both appearances.

We chose an egui theme over a webview running real shadcn because it adds no JS toolchain, no IPC layer and no extra RAM for a menu bar app that should stay small. The look is a close imitation of shadcn rather than the real components.

## Considered options

- **Webview with real shadcn/ui.** Pixel-exact, but needs a Node build, a bridge between Rust and the page, and a web engine resident in memory. Rejected.
- **Tweak egui's default visuals only.** Cheap, but does not give cards, switches, badges or button variants. Rejected.
- **A third-party egui theme crate.** Adds a dependency for tokens we can express in one file. Rejected.
