#!/usr/bin/env bash
# Reproduce the CI lint job locally.
# Mirrors: .github/workflows/ci.yml → lint
# Lint levels are configured in Cargo.toml [lints.clippy].
# RUSTFLAGS=-Dwarnings promotes warnings to errors (matching CI).
set -euo pipefail

export RUSTFLAGS="${RUSTFLAGS:--Dwarnings}"

echo "==> Format check"
cargo fmt --all -- --check

echo "==> Clippy"
cargo clippy --all-targets

echo "==> Lint passed"
