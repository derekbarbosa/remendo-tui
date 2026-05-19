#!/usr/bin/env bash
# Build the Pages site locally.
# Mirrors: .github/workflows/docs.yml → build
# Requires: cargo-llvm-cov, cargo-crap, llvm-tools-preview, python3
set -euo pipefail

echo "==> Build rustdoc"
RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo doc --no-deps --document-private-items

echo "==> Build coverage HTML + LCOV"
cargo llvm-cov --all-targets --no-fail-fast --html --output-dir target/llvm-cov
cargo llvm-cov --all-targets --no-fail-fast --lcov --output-path lcov.info --no-clean

echo "==> Generate CRAP report"
cargo crap --lcov lcov.info --format json --output target/crap-full.json

echo "==> Generate CRAP badge + HTML report"
python3 scripts/crap_html.py target/crap-full.json target/crap-badge.json target/crap-report.html

echo "==> Assemble Pages site"
mkdir -p target/pages/rustdoc
mkdir -p target/pages/coverage
mkdir -p target/pages/crap
cp -r static/* target/pages/
cp -r target/doc/* target/pages/rustdoc/
cp -r target/llvm-cov/html/* target/pages/coverage/
cp target/crap-badge.json target/pages/crap-badge.json
cp target/crap-report.html target/pages/crap/index.html

# Append denim theme to llvm-cov stylesheets
echo "==> Apply coverage theme"
cat static/coverage-theme.css >> target/pages/coverage/style.css
for css in target/pages/coverage/coverage/**/style.css; do
  [ -f "$css" ] && cat static/coverage-theme.css >> "$css"
done

echo "==> Pages site assembled in target/pages/"
echo "    Serve with: python3 -m http.server 8787 --directory target/pages"
