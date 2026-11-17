# Self-signed builds, own Homebrew tap, released from the developer's machine

PimSiang is installed with `brew install --cask PingSunn/tap/pimsiang`. Builds are Apple Silicon only (macOS 14+; Universal later) and signed with one self-signed certificate reused for every release: an ad-hoc signature changes each build, and macOS ties the Accessibility grant to the signature, so users would have to grant it again after every update. Users click "Open Anyway" once because the app is not notarized (Homebrew 7 has no `--no-quarantine`). A local script builds, signs, zips, publishes the GitHub release and bumps the cask in `PingSunn/homebrew-tap`, so the certificate never leaves the machine. The app checks GitHub Releases once a day and tells the user to run `brew upgrade`; it never replaces itself. The repo stays private until v0.1, so tests and lint run locally until then, and GitHub Actions comes when the repo is public and macOS runners are free.

## Considered options

- **Apple Developer Program ($99/year) with notarization:** no Gatekeeper prompt and eligible for the main homebrew-cask repo; deferred until it is worth paying for.
- **Release from GitHub Actions on tag:** hands-off, but the signing certificate would live in CI secrets and private-repo macOS minutes cost 10x.
- **In-app self-update:** works without Homebrew, but fights `brew upgrade` over who owns the installed app.
