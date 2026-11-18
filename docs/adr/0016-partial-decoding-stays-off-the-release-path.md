# Partial decoding must not delay the release

Partial text is a re-decode of the open segment (ADR 0008), run synchronously inside `Engine::push_audio` on the controller thread, so a slow decode left the Trigger key release and the paste queued behind a backlog of audio blocks, each of which could trigger another decode. Two changes bound this without moving the Engine to another thread. The controller coalesces consecutive queued audio blocks into one `push_audio` call, so a backlog costs at most one Partial decode and stale ticks are skipped. When the key is released, audio still queued behind the release goes through the new `Engine::push_tail`, which feeds the VAD and closes segments but never decodes a Partial, so the only decode after release is the one that produces Final text. The worst remaining delay is one Partial decode already in progress when the key is released.

## Considered options

- **Engine on its own worker thread:** the release would never wait for a decode, but the controller would lose its synchronous Engine and every test seam, and Final text would need a second channel back; revisit if one Partial decode proves too slow on long segments.
- **Cap the open segment length:** bounds decode time but changes segmenting (ADR 0012) and was left to a later decision.
