# remendo-tui Makefile
# Local equivalents for every CI job. Run `make ci` to reproduce the full pipeline.

.PHONY: all lint test coverage crap crap-baseline crap-diff crap-pr docs audit ci clean help

all: lint test  ## Default: lint + test

# ── CI jobs (mirrors .github/workflows/ci.yml) ───────────────

lint:  ## Format check + clippy (ci.yml → lint)
	@scripts/lint.sh

test:  ## Build + unit/integration/doc tests (ci.yml → test)
	@scripts/test.sh

coverage:  ## Generate lcov.info coverage report (ci.yml → coverage)
	@scripts/coverage.sh

crap: coverage  ## CRAP score report (ci.yml → crap)
	@scripts/crap.sh report

crap-baseline: coverage  ## Save CRAP baseline JSON (ci.yml → crap-baseline)
	@scripts/crap.sh baseline

crap-diff: coverage  ## CRAP delta vs saved baseline
	@scripts/crap.sh diff

crap-pr: coverage  ## CRAP PR-comment markdown with delta
	@scripts/crap.sh pr

# ── Docs (mirrors .github/workflows/docs.yml) ────────────────

docs:  ## Build Pages site: rustdoc + coverage HTML (docs.yml)
	@scripts/docs.sh

# ── Security ─────────────────────────────────────────────────

audit:  ## Dependency security audit
	cargo audit

# ── Compound targets ─────────────────────────────────────────

ci: lint test coverage crap  ## Full CI pipeline locally

# ── Cleanup ──────────────────────────────────────────────────

clean:  ## Remove build artifacts
	cargo clean
	rm -f lcov.info crap-baseline.json

# ── Help ─────────────────────────────────────────────────────

help:  ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'
