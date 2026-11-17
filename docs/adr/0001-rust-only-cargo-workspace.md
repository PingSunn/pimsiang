# Rust only, as a Cargo workspace with per-platform crates

PimSiang ships on macOS first and on Windows and Linux later, so the whole app is written in Rust and split into a Cargo workspace: `engine` (VAD + speech recognition), `audio`, `store` (History), `platform-macos` (everything that calls AppKit, Accessibility or event taps) and `app` (wiring). OS calls live only in a `platform-*` crate so a later `platform-windows` adds a port without touching the rest, and `engine` can be tested with no UI.

## Considered options

- **Tauri v2 (Rust core + web UI):** cross-platform UI for free, but the Pill still needs native code and a web runtime adds weight for what is mostly a background app.
- **Swift app + shared Rust core:** best Mac UI, but every platform would need its own UI codebase.
- **Electron + Python sidecar running the model's bundled server:** fastest prototype, but a multi-GB install that is hard to ship through Homebrew.
