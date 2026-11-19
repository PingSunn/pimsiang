# OpenThai settings follow the model card: Recognition mode, a choice of Precision, two symbols per frame

The OpenThai 2.0 ASR Streaming model card lists what its reference server lets a caller set: `lang` (`th-TH` or `th-mix`), `latency_ms` (80, 320, 560 or 1120) and `mode`. PimSiang offers the ones the published ONNX export can honour and nothing of its own invention.

**Recognition mode.** Standard (the default) and Conversation are the card's `standard` and `conversation` modes, decoded exactly as the card describes: Standard decodes with a blank offset of −1.0, decodes a clip under 2 s again at −2.5, and decodes an empty result again at −4.0; Conversation decodes at −2.0 and an empty result again at −4.0. The encoder runs once per clip and only the greedy decode is repeated, so a retry costs decoder passes only. The mode is stored as `openthai.mode=` (namespaced because it belongs to this Model, ADR 0011), is chosen under the Language mode in the General settings, and applies from the next Utterance through the existing preferences flow (ADR 0025). The card's `meeting` mode is left out: it needs a separate diarization model.

**Latency stays at 560 ms.** The ONNX export at `iapp/OpenThai2.0-ASR-Streaming-ONNX` is built with the attention context fixed at [56, 6] and has no input to change it (ADR 0008). Other latencies would need someone to export and host more files from the `.nemo` checkpoint; the user declined to host model files. The card's CER gain from 560 to 1120 ms is about 0.1 points.

**Precision: Light by default, Accurate on request.** Settings offers Light (int8, about 1 GB) and Accurate (fp32, about 2.6 GB), both downloaded from iApp's repository at the pinned revision with a sha256 per file (ADR 0023). Light stays the default (ADR 0009 stands): on the synthetic fixtures fp32 gave 1.39 % CER against 1.58 % for int8, with each missing about one character on a different clip, at the same speed (RTF 0.049 against 0.051 on an M5 Pro) but with about 1.4 GB more peak memory (5.2 GB against 3.8 GB) and a slower load. Switching downloads the other build in the background while the current one keeps working, switches the Engine between Utterances once every file verifies, and only then deletes the old build; a failed download leaves the current build in use.

**Two symbols per frame.** The greedy decode emits at most two symbols per encoder frame, as the card recommends against rare repetition loops. On the fixtures this changed no fp32 output and one already-wrong word in the int8 37.6 s clip (same 12/372 edits), at no cost in speed.

**ONNX Runtime threads stay at the default.** The card advises one CPU thread on a Mac, measured with PyTorch. With ONNX Runtime one thread made decoding 57 % slower for int8 and 16 % slower for fp32, with identical text, so the sessions keep ORT's default.

The model's licence, OpenMDW-1.1, permits redistribution as long as the licence and notices travel with the files, which settles the question ADR 0002 left open; PimSiang still downloads from iApp and redistributes nothing.

## Considered options

- **User-tunable blank offsets or a "word capture" preset:** more control, but values of PimSiang's own invention that the card does not define or measure. Rejected for the card's two modes.
- **Exporting and hosting our own 1120 ms ONNX:** a small accuracy gain with no added delay in PimSiang (Partial text re-decodes the open segment, ADR 0008), but PimSiang would become the distributor of model files. Rejected.
- **Accurate (fp32) as the default:** chosen first for accuracy, reverted after the measurements showed no clear gain on the fixtures for 1.6 GB more download and about 1.4 GB more memory.
- **One ONNX Runtime thread:** slower here; rejected on the measurements.
