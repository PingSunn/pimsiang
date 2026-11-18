#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
