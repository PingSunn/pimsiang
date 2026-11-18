# PimSiang brand assets

PimSiang's identity is the hill myna (นกขุนทอง), the Thai bird famous for repeating speech: here it listens and writes back what you said. It replaces the indigo look of ADR 0034 (see ADR 0035). Use the provided SVGs as the source of truth; all lettering is vector paths.

## Colours

| Name | Hex | Use |
| --- | --- | --- |
| Feather | `#1C1A17` | Bird, text, primary button, Pill surface |
| Wattle | `#FFD23F` | App icon, accent, selection, text on the primary button |
| Beak | `#F28C28` | Beak, question mark, warnings |
| Paper | `#FBF7EE` | Light background |
| Live | `#E5484D` | Recording only: sound arcs, the listening dot |
| Ink-2 | `#6B645A` | Secondary text |
| Rule | `#E8E0CF` | Dividers, borders |
| Side | `#F6F1E4` | Sidebar, card tint, dark-variant rim |
| Select | `#FFE89A` | Selected row |

Dark mode uses Feather-dark backgrounds (`#16140F`, `#1F1C17`), Paper text and Wattle for accents and selection. The primary button is Feather with Wattle text on light, and Wattle with Feather text on dark.

## Typography

Both fonts are free under the SIL Open Font License and available from Google Fonts.

| Use | Font | Weight |
| --- | --- | --- |
| Titles and wordmark | Mitr | 600 |
| Interface text | Anuphan | 400 to 650 |

Never set "พิมพ์เสียง" in a font in place of the logo; use the wordmark SVG.

## Files

| File | Use |
| --- | --- |
| `pimsiang-wordmark.svg`, `pimsiang-wordmark-dark.svg` | "พิมพ์เสียง" (Mitr 600, outlined) with PIMSIANG below; light and dark backgrounds. |
| `pimsiang-icon.svg`, `PimSiang.icns` | macOS app icon: yellow squircle with the myna head. The `.icns` holds 16 to 512 px at 1x and 2x. |
| `pimsiang-icon-maskable.svg` | Full-bleed variant; the head sits inside the central 80% safe zone. |
| `pimsiang-menubar.svg` | Menu bar template glyph (18 x 18, monochrome, eye and wattle cut out). |
| `pimsiang-menubar-listening.svg`, `-transcribing.svg`, `-attention.svg` | State variants: a dot, three dots, or a `!` badge at the corner. Still monochrome templates. |
| `mascot/pimsiang-myna-<pose>.svg`, `...-dark.svg` | Nine poses, viewBox 120 x 120; see Mascot. |
| `mascot/pimsiang-myna-head-<state>.svg`, `...-dark.svg` | Head only, viewBox 64 x 64, for 16 to 32 px (default, listening, writing, confused). |
| `exports/` | PNG renders of all of the above. |
| `guidelines.html` | One-page guide for designers (Thai). |

## Mascot

| Pose | Use it for |
| --- | --- |
| `default` | General illustration, no particular message. |
| `listening` | Recording: head tilt and red sound arcs. |
| `writing` | Transcribing and the model download: beak open, Thai letters. |
| `note` | Quick note saved. |
| `confused` | Missing permission: a `?` and a padlock. |
| `sleeping` | Idle unload; empty History. |
| `waving` | Welcome and Setup: the wing raised. |
| `oops` | Microphone lost, History unavailable: a microphone with an unplugged cable. |
| `searching` | History search with no match: a magnifier. |

- **Light and dark.** `-dark.svg` adds a Side (`#F6F1E4`) rim around the feather so it does not vanish on a dark surface, and lightens the wing, shadow, `z` letters and the microphone. Pick the variant by the background, never the other way round. The Pill is always a Feather capsule, so it uses the dark heads.
- **Minimum size.** About 64 px tall for the full body; below that use the head. Heads go down to 16 px. The menu bar glyph is drawn for 18 pt and must stay legible at 18 px.
- **Decorative.** The mascot never carries meaning alone: the text beside it says what happened. Mark it `aria-hidden`.
- **Don'ts.** Do not recolour, stretch, rotate or redraw the bird, do not substitute an emoji, do not put the light variant on a dark background, do not add effects to the SVGs.
- **Motion.** Gentle and slow: a 3 px float over 2.4 s, or a 1.4 s opacity pulse for the listening dot. Any animation must stop under `prefers-reduced-motion: reduce`. The files here are still.

## Menu bar

macOS template images are monochrome, so the system tints them for light and dark menu bars. State is therefore a small shape at the corner of the same glyph (a dot while listening, three dots while transcribing, a `!` badge when a permission is missing) and never a recolour of the whole glyph.

## Rendering

`source/render.sh` regenerates everything:

1. builds `source/render` (a small Rust tool on `resvg`, with its own `Cargo.lock`, outside the app workspace);
2. runs `source/mascot.py`, which writes every SVG above (Thai letters and the wordmark are converted to paths with Mitr through `brand-render textpath`);
3. renders the PNGs in `exports/`, builds `PimSiang.icns` with `iconutil`, and writes the PNGs the app embeds: `crates/app/assets/mascot/`, `crates/app/assets/menubar/` and `crates/platform-macos/assets/`.

Fonts for the render are read from `crates/app/assets/` (`scripts/fetch-font.sh` downloads them). Change the art in `source/mascot.py` and rerun the script; do not edit the SVGs by hand.
