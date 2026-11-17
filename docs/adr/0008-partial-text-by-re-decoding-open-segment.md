# Partial text by re-decoding the open speech segment; latency fixed at 560 ms

The published ONNX export of OpenThai 2.0 ASR Streaming is not cache-aware: the encoder takes the whole audio of a call and its attention context is fixed at [56, 6] (560 ms), so ADR 0002's latency choice (80/320/560/1120 ms) does not exist in practice and is dropped. To still show Partial text, the Engine re-decodes the open speech segment about every 500 ms while the user speaks; once Silero VAD closes a segment at a silence, that segment's text is locked and never decoded again, so the work per tick stays bounded by one segment instead of growing with the Utterance. Final text is the locked segments joined in order. Whether the shipped Model is int8 or fp32 is decided by measuring both in ticket 01 (character error rate, speed, memory on real Thai audio), because the int8 export's accuracy is unmeasured by its authors.

## Considered options

- **No Partial text in the MVP:** simplest and cheapest, but loses the live text in the Pill that sets PimSiang apart.
- **Exporting a cache-aware streaming ONNX ourselves from the NeMo checkpoint:** true streaming and selectable latency, but needs NeMo and a GPU, a self-hosted export to maintain, and a licence check for redistribution.
