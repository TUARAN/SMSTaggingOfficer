#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "[selftest] running Rust selftest binary..."
cargo run --quiet --manifest-path src-tauri/Cargo.toml --bin selftest

echo "[selftest] done"
