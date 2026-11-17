# Local ONNX Engine with Silero VAD, Model downloaded and pinned at first launch

The Engine runs OpenThai 2.0 ASR Streaming (`iapp/OpenThai2.0-ASR-Streaming-ONNX`, int8) on the CPU through `ort`, so audio never leaves the machine and no Python runtime is shipped. The model drops words after long pauses, so Silero VAD (also through `ort`) cuts audio at silences before decoding. The Model is downloaded from Hugging Face on first launch rather than bundled in the cask, pinned to a repository revision with a sha256 per file, resumable, and stored under `~/Library/Application Support/PimSiang/models`. Defaults are 560 ms latency and `th-mix`, changeable in settings (latency choice and int8 superseded in part by ADR 0008). The Engine sits behind one interface so another model (an earlier internal bake-off found Pathumma FC more accurate and faster) can be swapped in later.

## Considered options

- **iApp's hosted Realtime WebSocket API:** no local RAM cost, but free only during launch, needs a network, and sends audio off the machine.
- **Mirroring the Model on our own GitHub Releases:** survives a Hugging Face outage, but redistribution under OpenMDW-1.1 would need checking; not needed for v0.1.
