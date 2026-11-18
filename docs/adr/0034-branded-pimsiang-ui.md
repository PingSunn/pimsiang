# 0034: A branded PimSiang look instead of plain shadcn zinc

Superseded by 0035 (the look: palette, logo and mascot; the layout and verification stay).

The shadcn zinc theme from ADR 0033 was correct but anonymous: black and white cards that looked like a default toolkit. The user wanted something in the spirit of Linear or Arc, a crafted product with its own identity. The theme module (`crates/app/src/theme/`) now carries a PimSiang identity on top of the same egui `Style` and `Visuals` approach.

Identity: an indigo-violet accent (`#6a5af9` in dark, `#5b45ee` in light, with lighter text variants for use on dark surfaces) used only for primary actions, focus rings, selection and the active sidebar item. A logo mark drawn with egui shapes (a rounded square with a five-bar waveform) sits next to the wordmark "PimSiang พิมพ์เสียง" in every window header, which has a faint accent gradient. Surfaces are layered (background, sidebar, raised card) with low-contrast borders, and the greys are tinted towards the accent. Dark is the primary look and light is tuned separately. Headings use the variable Anuphan font at weight 650 and controls at 500, through `RichText::variation`.

Layout: Settings has a left sidebar (General, Trigger, Sounds & Clipboard) with painted icons and a content pane of setting rows grouped in cards. History has a header band, a toolbar with the search box and Clear all, and hover-highlighted rows with Copy and Delete actions; the clear-all confirmation sits in the toolbar so it never scrolls out of view. Setup is a hero (logo, title, intro) followed by numbered steps with a segmented progress bar. Icons are drawn with painter shapes (`theme/icons.rs`) because the bundled fonts do not cover symbol glyphs. The Pill stays a dark capsule with the same positioning and gains an accent dot at its left edge.

Verification without opening windows: `cargo test -p app --bin pimsiang ui_snapshots -- --ignored` renders every window to `target/ui-snapshots/*.png` in Thai and English, dark and light, using `egui_kittest` with its wgpu renderer. These tests need a GPU and are ignored so that `scripts/check.sh` stays headless. Behaviour tests in `ui_tests.rs` run on the CPU and are part of `check.sh`.

## Considered options

- **Follow native macOS controls.** Familiar, but egui cannot draw real AppKit controls and an imitation would look uncanny. Rejected.
- **Keep shadcn zinc and polish spacing only.** Cheaper, but the user's complaint was missing identity, not spacing. Rejected.
- **Bundle an icon font.** Crisp icons, but another asset and licence for about ten simple shapes. Rejected in favour of painter shapes.
