#!/usr/bin/env bash
# Run CRAP analysis locally.
# Mirrors: .github/workflows/ci.yml → crap / crap-baseline
# Requires: cargo-crap, cargo-llvm-cov, llvm-tools-preview
#
# Usage:
#   scripts/crap.sh              # human-readable report
#   scripts/crap.sh baseline     # save JSON baseline to crap-baseline.json
#   scripts/crap.sh diff         # compare against crap-baseline.json
#   scripts/crap.sh pr           # PR-comment markdown with delta
set -euo pipefail

MODE="${1:-report}"

# Generate fresh coverage if lcov.info is stale or missing
if [ ! -f lcov.info ] || [ "$(find lcov.info -mmin +5 2>/dev/null)" ]; then
    echo "==> Generating fresh lcov.info"
    cargo llvm-cov --all-targets --no-fail-fast --lcov --output-path lcov.info
fi

case "$MODE" in
    baseline)
        echo "==> Saving CRAP baseline"
        cargo crap --lcov lcov.info --format json --output crap-baseline.json
        echo "==> Baseline saved to crap-baseline.json"
        ;;
    diff)
        if [ ! -f crap-baseline.json ]; then
            echo "error: crap-baseline.json not found. Run 'scripts/crap.sh baseline' first." >&2
            exit 1
        fi
        echo "==> CRAP delta vs baseline"
        cargo crap --lcov lcov.info --baseline crap-baseline.json --allow 'tests/**'
        ;;
    pr)
        if [ ! -f crap-baseline.json ]; then
            echo "error: crap-baseline.json not found. Run 'scripts/crap.sh baseline' first." >&2
            exit 1
        fi
        echo "==> CRAP PR comment"
        cargo crap --lcov lcov.info --format pr-comment --baseline crap-baseline.json --allow 'tests/**'
        ;;
    report|*)
        echo "==> CRAP report"
        cargo crap --lcov lcov.info --allow 'tests/**'
        ;;
esac
