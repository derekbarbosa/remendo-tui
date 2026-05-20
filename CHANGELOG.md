# Changelog

All notable changes to remendo are documented here.

## [0.1.0] — 2026-05-20

### Bug Fixes

- README: fix double license badge -s
- Fix nightly workflow for immutable releases
- Add Makefile, CI scripts, and CRAP PR comment workflow
- Rename strip_email_quoting to strip_review_quoting, fix test definitions
- Fix domain language in docs and design specs
- Fix domain language in code comments: Sashiko API, not LKML/email
- Fix critical bug: editor launch corrupts terminal state
- Restore ConfigError::Validation and ConfigWarning::ParseWarning
- Fix API query params and detail endpoint per Sashiko docs

### CI/CD

- Set CRAP regression epsilon to 1.0 to ignore coverage noise
- README: adjust link to CRAP badge
- Document series7 features: sorting, filtering, diff highlighting, indentation
- Add GitHub Actions release pipeline + Dependabot

### Documentation

- Add LLM acknowledgement to README
- Add config.example.toml and reference it from documentation

### Features

- Add cargo-release and git-cliff configuration
- Add Esc key to clear search filter in list view
- Prepare Cargo.toml for crates.io publishing
- Add CONTRIBUTING.md and Cargo.toml repository metadata
- Update README: roadmap reference, correct test count, remove stale upcoming list
- Distinguish patch metadata from reviewer commentary in inline reviews
- Add cross-feature integration tests for series7 sort+filter compose
- Fix diff syntax highlighting in patchset detail inline reviews
- Add Tier 1 features: bookmark-filtering, diff-syntax-highlighting, thread-indentation
- Add series7 integration test definitions (103 steps across 9 TOML files)
- Add column-sorting feature: cycle sort columns with s/S keybindings
- Add message browser view with m key toggle
- Add patchset metadata display and baseline logs viewer
- Fix stale docs: bookmarks, comment-nav, and caching are implemented
- Fix PrevComment (N) keybinding not matching crossterm events
- Add loading dialog when selecting a patchset for detail view
- Implement CachingClient decorator with in-memory TTL cache
- Add bookmark system, comment navigation, and cache layer infrastructure
- Add test coverage for audit gaps: detail scroll, sidebar select, search, ViewRawLog
- Fix documentation accuracy: pagination keys, planned features
- Fix empty patches in detail view and implement view raw log
- Add pagination, search filter, and mailing list sidebar
- Add detail view, help overlay, stats display, and back navigation
- Wire Enter/Select to fetch and display patchset detail
- Fix post-tier review findings: panics, test gaps, and cross-feature hardening
- Add patchset-list-view and mailbox-sidebar interactive TUI features
- Add cmd-effect-runtime: wire API client into TEA event loop
- Add project documentation: README, user guide, configuration guide
- Add testing-framework: CI pipeline, integration tests, snapshots
- Add api-client module: async Sashiko HTTP client
- Add core-architecture: async TEA event loop and terminal lifecycle
- Add configuration module: TOML config with XDG paths
- Add data-models module: Sashiko domain types

### Miscellaneous

- Initial Commit

### Other

- Distinguish reviewer commentary from quoted diff in inline reviews

### Refactoring

- Remove nightly workflow: incompatible with immutable releases
- Move Pages assets to static/, redesign index with denim aesthetic
- Add GitHub Pages deployment for rustdoc
- Clean up code quality: simplify deser error, remove dead variants
- Add docs/REDHAT.md to .gitignore
- Add project scaffold: Cargo manifest, gitignore, and policies

### Testing

- Rename crate from remendo-tui to remendo
- Track insta snapshot files for CI
- Pin Rust toolchain to 1.95.0, fix duration_suboptimal_units lint
- Add CRAP badge, HTML report page, coverage theme, and docs audit fixes
- Add 48 tests to reduce CRAP scores below threshold
- Add llvm-cov coverage to CI and GitHub Pages deployment
- Remove agent config files from tracking, clean gitignore
- Revert thread-indentation: restore flat thread rendering
- Add rendering test verifying thread indentation at depths 0, 1, 2
- Fix diff highlighting: handle email-quoted diff lines in inline reviews
- Add file-based debug logging via REMENDO_LOG env var
- Gitignore: exclude insta test snapshots from tracking
- Fix config loading visibility and terminal init ordering


