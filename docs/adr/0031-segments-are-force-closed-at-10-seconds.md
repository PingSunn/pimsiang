# Segments are force-closed at 10 seconds

ADR 0012 left a segment unbounded: a clip of near-continuous speech kept one open segment that every Partial text tick re-decoded whole. A 33.5 s clip built from the concatenated fixtures with no gaps peaked at 9.6 GB resident (12.3 GB footprint) and took 117 s in the `transcribe` release build.

An open segment is now closed when it reaches `MAX_SEGMENT_SAMPLES` (10 s, since the cost of a decode grows steeply with length; the first cap was 15 s and was lowered after measuring). The Engine keeps the speech probability of the last 1.5 s of windows (47 windows) and cuts at the lowest of them, at the middle of that window, so a word is unlikely to be split. The cut audio is decoded and its text locked; the next segment starts exactly at the cut with the audio after it, so no audio is lost or repeated. A tick that force-closes a segment decodes the capped segment and then only the short remainder after the cut for the Partial, so the Partial text never shrinks and one tick decodes at most one capped segment. A cut that falls inside a pause leaves the remainder outside speech, so silence is still never decoded. A forced cut is mid-phrase, and Thai has no spaces between words, so its text is joined to the next segment without a space; a segment closed by silence is still joined with one.

Partial text is re-decoded every 500 ms while the open segment is 5 s or shorter and every 1 s once it is longer, because a Partial tick on a long segment already takes close to real time. A forced cut leaves a short remainder, so the 500 ms cadence returns after it.

Measured with the `transcribe` release build, int8 Model, M-series Mac, on a 37.6 s clip (the six fixtures weather, meeting, food, news, greeting, weather concatenated with no gaps) and a 75 s clip (the same audio repeated), Partials on, same clips for both caps:

| Clip | Cap | Peak resident | Peak footprint | Wall time | Final decode |
| --- | --- | --- | --- | --- | --- |
| 37.6 s | 15 s | 5.7 GB | 5.4 GB | 37.2 s | 0.38 s |
| 37.6 s | 10 s | 4.4 GB | 4.1 GB | 23.3 s | 0.51 s |
| 75 s | 15 s | 5.6 GB | 5.7 GB | 71.2 s | 0.18 s |
| 75 s | 10 s | 4.4 GB | 4.1 GB | 38.9 s | 0.39 s |

Before any cap, a 33.5 s clip peaked at 9.6 GB resident (12.3 GB footprint) and took 117 s. Memory stays flat in the clip length (37.6 s to 75 s adds 0.02 GB) and the Final text after the last Key release decodes in under 0.6 s instead of 3.3 s. The 10 s cap cuts peak resident memory by about 1.2 GB and wall time by about 45 % against 15 s, at a cost in accuracy: the CER of the long continuous clip rose from 0.008 to 0.032, still under the 0.10 threshold, because more forced cuts land inside words.

## Considered options

- **Cutting at the cap exactly:** simplest, but splits a word whenever speech is continuous.
- **A 15 s or 20 s cap:** fewer cuts and a lower CER, but decode memory and per-tick time grow with the cap.
- **A 500 ms Partial cadence at any length:** simpler, but each tick on a long segment costs close to real time.
- **Joining forced cuts with a space:** would insert a space inside Thai words and phrases.
