# Cut Utterances at silences with Silero VAD inside the streaming Engine

The Engine feeds every 32 ms window of incoming audio to Silero VAD v5.1.2 (commit `6478567951ae5c9979ad7b234185b5515f4be7a1`, `silero_vad.onnx`, sha256 `2623a2953f6ff3d2c1e61740c6cdb7168133479b267dfef114a4a3cc5bdd788f`), run through the same `ort` environment as the Model and stored beside it as `silero_vad.onnx`. A segment opens when speech probability reaches 0.5 and closes after 500 ms below 0.35, with 300 ms of padding kept before and after the speech. A closed segment is decoded once and its text is locked; audio before the first speech and after a closed segment is dropped, so silence is never decoded and silence-only input gives empty Final text. Partial text is the locked segments plus a re-decode of the open segment every 500 ms; Final text joins all segments with a space. The detector sits behind a `SpeechDetector` trait, like the `Recognizer`, so segmenting is tested with a scripted detector and the real VAD is only exercised by the opt-in real-Model tests. `ort` allows one environment per process, so the Model and the VAD share one, kept for the life of the process.

A single segment is still decoded whole, so a very long stretch of speech without a 500 ms pause still grows memory and time per tick with its length; ADR 0031 caps it.

## Considered options

- **Bundling `silero_vad.onnx` in the repository:** no download step, but it is a 2.3 MB binary in git; it is pinned and fetched by `scripts/fetch-vad.sh` instead, and the Model download ticket will carry it with the other Model files.
- **A newer Silero VAD release:** not tried; v5.1.2 is the release the Engine's 512-sample window with 64 samples of context was checked against.
