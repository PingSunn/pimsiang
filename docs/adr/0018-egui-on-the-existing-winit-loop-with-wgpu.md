# egui windows run on the existing winit loop through `egui-winit` and `egui-wgpu`

The menu bar icon, the Pill and the controller events already live on one winit event loop, and `eframe` insists on owning its own loop. The setup window (and later History and settings) therefore uses `egui-winit` for input and `egui-wgpu`'s `Painter` (Metal only) for drawing, created from the same `ApplicationHandler`. The Anuphan variable font (OFL, pinned to a google/fonts commit with its sha256 in `scripts/fetch-font.sh`, licence text beside it) is embedded with `include_bytes!` and put first in every egui font family; egui's text shaping handles Thai vowels and tone marks. The UI language is Thai when the first system preferred language is Thai, otherwise English.

## Considered options

- **`eframe`:** least code, but cannot share the winit loop with `tray-icon` and the Pill.
- **`egui_glow` with glutin:** more windowing code than wgpu, and OpenGL is deprecated on macOS.
- **A native AppKit setup window:** would not exercise the egui and Anuphan path the later windows need.
