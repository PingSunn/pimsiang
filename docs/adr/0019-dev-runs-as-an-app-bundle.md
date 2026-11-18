# Development runs PimSiang as a signed `.app` bundle

macOS lists a program under Privacy and Security only when it has a bundle identity, and attributes a bare binary's permissions to the app that launched it (the terminal, or an agent shell). Run from a terminal, PimSiang never appeared in the Microphone, Accessibility or Input Monitoring panes. `scripts/bundle-dev.sh` now builds `target/PimSiang.app` (identifier `com.pingsunn.pimsiang`, `LSUIElement`, `NSMicrophoneUsageDescription`) around the debug binary and signs it; `scripts/run-dev.sh` builds and `open`s it, passing the model directory with `--env`. `run-dev.sh --plain` keeps the old direct run. Signing is ad-hoc by default; set `PIMSIANG_SIGN_IDENTITY` to a stable identity so grants survive rebuilds (ADR 0007: the grant is tied to the signature, and an ad-hoc signature changes every build). Before asking, the controller calls the request APIs (`AXIsProcessTrustedWithOptions` with the prompt option, `CGRequestListenEventAccess`, `AVCaptureDevice requestAccess`) so macOS adds the entry itself, then opens the pane. When the executable is not inside a `.app`, the setup window says the permissions belong to the launching terminal.

## Considered options

- **Keep running the bare binary:** grants attach to the terminal, and PimSiang is never listed.
- **Wait for the release bundle script (ADR 0007):** blocks onboarding work on packaging that comes later.
