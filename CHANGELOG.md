# Changelog

## [0.2.0] - 2026-10-04

### Added

- `sqlite` feature: `SqliteMemory`, a durable implementation of both `EpisodicStore`
  and `SemanticStore` on one SQLite file (SQLite compiled in via `rusqlite` with
  `bundled`; calls run on Tokio's blocking pool). Memory now survives restarts.
- `SqliteMemory::search`: full-text search over episode content with SQLite FTS5,
  ranked by BM25; free text or FTS5 syntax (phrases, OR/NOT, prefixes).
  Re-recording an episode replaces its indexed text; `forget` removes it.
- Tests: survival across reopening the file, the same answers as the in-memory store
  for every trait method, search ranking and updates, 200 concurrent writers.
- Every Rust example in the README is compiled by `cargo test --doc`.

### Changed

- crates.io metadata (description, keywords, categories, repository) is in the
  repository's `Cargo.toml` (0.1.1 was published with it, but it was never committed).
- Removed the stale `tokio-memory/` copy of the crate from the repository.
- `rust-version` stays 1.79 for the default features (checked with an MSRV-aware
  lockfile); the `sqlite` feature needs 1.85.

## [0.1.1]

- First crates.io release.
