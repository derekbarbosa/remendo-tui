# Contributing to remendo

Thanks for your interest in contributing to remendo — a terminal interface for
[Sashiko](https://sashiko.dev/) agentic patch review instances.

## Getting started

```sh
git clone https://github.com/derekbarbosa/remendo-tui.git
cd remendo-tui
make help    # list all available targets
make ci      # run the full local CI pipeline
```

### Prerequisites

- **Rust stable toolchain** (1.85+, edition 2024)
- **cargo-llvm-cov** — `cargo install cargo-llvm-cov`
- **cargo-crap** — `cargo install cargo-crap`
- **cargo-audit** (optional) — `cargo install cargo-audit`

## Development workflow

### Before every commit

```sh
make lint test
```

This runs `cargo fmt --check` + `cargo clippy` (with warnings as errors), then
builds and runs all 316 tests (unit, integration, doc).

### After adding or modifying tests

```sh
make crap
```

This regenerates `lcov.info` and reports CRAP (Change Risk Anti-Patterns)
scores. Functions exceeding the threshold of 30 need either more test coverage
or lower cyclomatic complexity.

### Full local CI pipeline

```sh
make ci    # lint → test → coverage → crap
```

### Available targets

| Target | Description |
|--------|-------------|
| `make lint` | Format check + clippy |
| `make test` | Build + all tests |
| `make coverage` | Generate `lcov.info` |
| `make crap` | CRAP score report |
| `make crap-baseline` | Save CRAP baseline for delta comparisons |
| `make crap-diff` | Compare against saved baseline |
| `make crap-pr` | PR-comment markdown with delta |
| `make docs` | Build Pages site (rustdoc + coverage + CRAP report) |
| `make audit` | Dependency security audit |
| `make ci` | Full pipeline |
| `make clean` | Remove build artifacts |

## Code style

- **Lint config**: All clippy lint levels live in `Cargo.toml` under `[lints.clippy]`.
  Do not pass `-W`/`-D` flags on the CLI or add `#![warn]` attributes in source.
- **Formatting**: `cargo fmt` with default rustfmt settings. CI enforces this.
- **No `unwrap()` in production code**: The crate denies `clippy::unwrap_used`.
  Use `expect()` with a descriptive message, or handle the error. Test code uses
  `#[allow(clippy::unwrap_used)]` or `#[allow(clippy::expect_used)]` on the test
  module.

## Testing

- **Test fixtures**: Use the `::fixture()` constructors in `src/models/fixtures.rs`
  to build test data. Don't hand-construct model types in tests.
- **CRAP threshold**: Functions with a CRAP score above 30 are flagged in CI.
  The score combines cyclomatic complexity and code coverage — either add tests
  or reduce branching to bring the score down.
- **Integration tests**: Live in `tests/`. API integration tests use
  [wiremock](https://crates.io/crates/wiremock) for HTTP mocking.

## Pull requests

- **CI checks**: All PRs run lint, test, coverage, and CRAP analysis. A bot
  posts a CRAP delta comment showing regressions, improvements, and new
  functions. Regressions (increased CRAP score vs. main baseline) fail the
  check.
- **Commit messages**: Use imperative mood, 1-2 sentence summary of *why*.
  Add `Assisted-by: <tool> <email>` trailer if AI-assisted.
- **One concern per PR**: Keep PRs focused. Large refactors should be separate
  from feature work.

## Releases

Releases are managed with [cargo-release](https://github.com/crate-ci/cargo-release)
and [git-cliff](https://git-cliff.org/). Only maintainers cut releases.

```sh
cargo release patch --execute   # 0.1.0 → 0.1.1
cargo release minor --execute   # 0.1.0 → 0.2.0
```

This generates a changelog from commit history, commits it, publishes to
crates.io, creates a signed tag, and pushes — triggering a GitHub Release
with binary assets.

**Commit message tips for good changelogs**: Start commit messages with a
keyword like `Add`, `Fix`, `Refactor`, `Remove`, or use conventional commit
prefixes (`feat:`, `fix:`, `docs:`, `test:`, `ci:`). The changelog generator
(`cliff.toml`) categorizes commits by these patterns.

## Domain language

remendo interacts with the **Sashiko API** — an agentic patch review platform.
Use "review instance", "patchset", "Sashiko" in code and docs. Not "mailing
list archive", "email client", or "LKML". The term "mailing list" is valid
when referring to tracked kernel lists within the Sashiko data model.

## License

By contributing, you agree that your contributions will be licensed under the
[AGPL-3.0-or-later](LICENSE) license.
