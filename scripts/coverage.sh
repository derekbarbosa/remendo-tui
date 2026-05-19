#!/usr/bin/env bash
# Reproduce the CI coverage job locally.
# Mirrors: .github/workflows/ci.yml → coverage
# Requires: cargo-llvm-cov, llvm-tools-preview
set -euo pipefail

echo "==> Coverage summary"
cargo llvm-cov --all-targets --no-fail-fast

echo "==> LCOV report"
cargo llvm-cov --all-targets --no-fail-fast --lcov --output-path lcov.info

echo "==> Coverage report saved to lcov.info"
