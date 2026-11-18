# 0040: The approved UI design is built in egui with painted key glyphs

The user approved a set of HTML artboards (Setup, Settings with General and Trigger key, History, menu bar and Pill) for the hill myna brand. This ADR records how the egui windows follow them; ADR 0035 still owns the palette and fonts.

Setup, Settings and History are drawn with the theme in `crates/app/src/theme`. The Wattle hero band, speech bubble, segmented progress bars, stepped card, sidebar with status chip, language cards, keyboard strip, segmented trigger control and History rows are theme components, not per-window code, and dark is derived from the same structure with the existing dark tokens (the keyboard strip stays a dark surface in both modes).

The Setup model row has no time estimate. The download reports `done` and `total` bytes only, so the line reads "412 จาก 1,000 MB" and nothing is invented. Adding "about N minutes left" needs a measured rate first.

The Settings status chip is a pure function of the model download status and of whether the Engine is loading (`app::model_chip`). "Loading" comes from the Pill's loading state, the only place the controller reports it. The chip names the model "OpenThai", the only model in the catalog; it needs the catalog's display name once a second model exists.

History groups entries by local calendar day (today, yesterday, then a date with the Buddhist year only when it is not the current year) and highlights search matches with Select. Matching is case-insensitive by character and widened to whole Thai syllables, so a highlight never separates a consonant from its vowel or tone mark. The footer counts come from the store (`History::count` and `History::total`), not from the loaded page, so "เจอ n จาก m" stays right while the list is paged. The "pasted into <app>" text from the design is left out on purpose: the user declined it.

Key symbols (command, option, control, shift) are painted as vector shapes. The bundled Anuphan has no glyph for them and egui panics on a missing glyph, and a second font only for four symbols is not worth the weight. They are used in the keyboard strip, the segmented control, the search hint and the Setup footer.

## Considered options

- **Bundle a symbols font.** Gives exact glyphs, but adds a binary and a licence for four shapes. Rejected for painted shapes.
- **Allow tofu for missing glyphs.** Hides the problem and ships boxes. Rejected.
- **Show a download rate and ETA estimated from progress events.** The numbers would jump and cannot be tested against real networks here. Rejected until a measured rate exists.
