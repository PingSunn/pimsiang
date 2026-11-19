# Contributing to PimSiang

Thanks for helping. Bug reports, recognition examples that go wrong, and pull requests are all welcome.

## Before you start

- For anything larger than a small fix, open an issue first so we can agree on the approach.
- Read `CONTEXT.md` for the words the code uses (Utterance, Pill, Note, Trigger key, and so on) and `docs/adr/` for the decisions behind the design.
- Security issues go through [SECURITY.md](SECURITY.md), not public issues.

## Setup

You need an Apple Silicon Mac on macOS 14 or newer and a stable Rust toolchain (`rustup`).

```sh
scripts/make-signing-cert.sh   # once, so macOS keeps the permissions between builds
scripts/run-dev.sh             # builds, bundles and launches the app
```

The app downloads the Model on first launch. To use a Model already on disk, set `PIMSIANG_MODEL_DIR`, or put it in `models/int8` (`scripts/fetch-vad.sh models/int8` adds the Silero VAD file).

## Checks

`scripts/check.sh` runs `cargo fmt --check`, build, clippy with warnings as errors, and the tests. CI runs the same script on every pull request.

Some tests are ignored by default because they need the Model or a GPU:

```sh
PIMSIANG_MODEL_DIR=models/int8 cargo test -p engine -- --ignored
cargo test -p app ui_snapshots -- --ignored
```

`scripts/e2e/run.sh --yes` drives the real app with synthetic keys and Thai text-to-speech. It takes over the keyboard and speakers, so run it only when you can leave the Mac alone.

## Pull requests

- Keep each pull request to one change. Pull requests are squash-merged, so the title becomes the commit on `main`.
- Write the title as a [Conventional Commit](https://www.conventionalcommits.org/), for example `fix(app): keep the Pill on the focused screen`.
- Record a new design decision as an ADR in `docs/adr/NNNN-slug.md`: a short paragraph with context, decision and why, plus the options you rejected when they matter.
- User-visible text needs both English and Thai (`crates/app/src/i18n.rs`).

By contributing you agree that your work is released under the [MIT licence](LICENSE) and that you follow the [Code of Conduct](CODE_OF_CONDUCT.md).
