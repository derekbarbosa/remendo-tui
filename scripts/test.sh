#!/usr/bin/env bash
# Reproduce the CI test job locally.
# Mirrors: .github/workflows/ci.yml → test
set -euo pipefail

echo "==> Build"
cargo build --all-targets

echo "==> Unit + integration tests"
cargo test --all-targets --no-fail-fast

echo "==> Doc tests"
cargo test --doc

echo "==> All tests passed"
