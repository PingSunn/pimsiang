# `unsafe` is denied workspace-wide and allowed only in `platform-macos`

The workspace forbids `unsafe_code`. `platform-macos` must read Core Foundation run loop mode constants, which are `extern` statics, so it lowers the lint to `deny` for itself and each use carries a narrow `#[allow(unsafe_code)]` on a tiny function. Every other crate keeps `forbid`. Wrapper crates (`core-graphics`, `arboard`, `cpal`) are used for the event tap, paste and microphone so the allowed surface stays as small as possible.

## Considered options

- **Keep `forbid` in `platform-macos` and avoid the statics:** not possible with the run loop APIs needed to keep an event tap alive and re-enable it.
- **Drop the workspace lint:** removes the guard for crates that never need `unsafe`.
