# Ship the int8 Model

ADR 0008 left the choice between the int8 and fp32 exports of OpenThai 2.0 ASR Streaming (revision `9f142a43e4d35a422c1c7dac40772ef707387ebc`) to measurement. Both were run through the Rust Engine on an Apple Silicon Mac against the five Thai fixtures in `crates/engine/tests/fixtures` (synthetic speech, 1.3 s to 16 s, `th-mix`), and their output matched a numpy + ONNX Runtime reference token for token. Accuracy is the same within noise, while int8 needs about half the memory and less than half the download, so PimSiang ships int8; fp32 stays loadable for comparison.

| | int8 | fp32 |
|---|---|---|
| Mean CER over the fixtures | 1.6 % (worst 4.8 %) | 1.4 % (worst 4.8 %) |
| Real-time factor, Final text only | 0.05–0.08 | 0.05–0.08 |
| Model load | 6–12 s | 7–8 s |
| Peak memory, 1.3 s clip | 1.4 GB | 3.0 GB |
| Peak memory, 16 s clip with Partial text every 500 ms | 5.7 GB | 6.9 GB |
| Download | 0.99 GB | 2.60 GB |

The 16 s figure grows because Partial text re-decodes the whole buffered audio until VAD segmenting (ticket 04) bounds it to one open segment.

## Considered options

- **fp32:** the only export its authors checked against NeMo, but twice the memory and download for no accuracy gain we could measure.
