#!/usr/bin/env bash
# Build the Pages site locally.
# Mirrors: .github/workflows/docs.yml → build
# Requires: cargo-llvm-cov, llvm-tools-preview
set -euo pipefail

echo "==> Build rustdoc"
RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo doc --no-deps --document-private-items

echo "==> Build coverage HTML"
cargo llvm-cov --all-targets --no-fail-fast --html --output-dir target/llvm-cov

echo "==> Assemble Pages site"
mkdir -p target/pages/rustdoc
mkdir -p target/pages/coverage
cp -r static/* target/pages/
cp -r target/doc/* target/pages/rustdoc/
cp -r target/llvm-cov/html/* target/pages/coverage/

echo "==> Pages site assembled in target/pages/"
echo "    Serve with: python3 -m http.server 8787 --directory target/pages"
