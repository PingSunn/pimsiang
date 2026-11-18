# Segments are force-closed at 15 seconds

ADR 0012 left a segment unbounded: a clip of near-continuous speech kept one open segment that every Partial text tick re-decoded whole. A 33.5 s clip built from the concatenated fixtures with no gaps peaked at 9.6 GB resident (12.3 GB footprint) and took 117 s in the `transcribe` release build.

An open segment is now closed when it reaches `MAX_SEGMENT_SAMPLES` (15 s, the low end of the 15-20 s range, since the cost of a decode grows steeply with length). The Engine keeps the speech probability of the last 1.5 s of windows (48 entries) and cuts at the lowest of them, at the middle of that window, so a word is unlikely to be split. The cut audio is decoded and its text locked; the next segment starts exactly at the cut with the audio after it, so no audio is lost or repeated. A tick that force-closes a segment skips the Partial re-decode, so one tick decodes at most one capped segment. A forced cut is mid-phrase, and Thai has no spaces between words, so its text is joined to the next segment without a space; a segment closed by silence is still joined with one.

Measured with the `transcribe` release build, int8 Model, M-series Mac, same clips before and after:

| Clip | Peak resident | Peak footprint | Wall time |
| --- | --- | --- | --- |
| 33.5 s, before | 9.6 GB | 12.3 GB | 117 s |
| 33.5 s, after | 5.9 GB | 5.4 GB | 45 s |
| 75 s, after | 6.0 GB | 5.6 GB | 94 s |

Memory is now flat in the clip length (33.5 s to 75 s adds 0.1 GB) and the Final text after the last Key release decodes in 0.2-0.3 s instead of 3.3 s. It is bounded by the cap rather than small: decoding 15 s of audio still costs about 5 GB, and the Partial ticks still take close to real time on a long segment, so a lower cap is the lever if that proves too heavy. The 75 s "before" run was not attempted because the 33.5 s one already used 12 GB.

## Considered options

- **Cutting at the cap exactly:** simplest, but splits a word whenever speech is continuous.
- **A 20 s cap:** fewer cuts, but decode memory and per-tick time grow with the cap.
- **Joining forced cuts with a space:** would insert a space inside Thai words and phrases.
