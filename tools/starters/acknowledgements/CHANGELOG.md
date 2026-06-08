# Changelog

All notable changes to the acknowledgements starter kit are recorded here. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the
kit uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

- **major** — a breaking change to a consumer contract (the `attribution.toml`
  schema, a driver's CLI, the marker-splice format, or the freshness-gate exit
  semantics).
- **minor** — additive, backwards-compatible (a new ecosystem driver, a new
  optional field).
- **patch** — fixes, docs, or determinism work with no contract change.

## [1.0.0] - 2026-06-08

First versioned release. The kit's contract is considered stable.

### Added

- A dispatcher that reads a `[[blocks]]` array from `attribution.toml` and
  routes each block to an ecosystem-specific driver, splicing the rendered
  output between `BEGIN`/`END` marker comments in a target markdown file
  (typically `ACKNOWLEDGEMENTS.md`). Hand-curated content outside the markers is
  preserved verbatim.
- Ecosystem drivers for **Rust** (`cargo-about`), **Node** (`license-checker`),
  **Go** (`go-licenses`), and **Python** (`pip-licenses`), plus a
  hand-maintained **bundled-binaries** driver for artifacts that appear in no
  lockfile.
- A single-source licence allow-list: `licences.toml` expands into each consumer
  file (`about.toml`, `deny.toml`, and the per-ecosystem allow-list fragments),
  with a `--check` drift gate.
- Strict per-driver licence gates, an idempotent and atomic marker splice, a
  deterministic (coreutils-independent) note wrapper, and an empty-output guard.
- A back-compatible flat `[rust]` configuration shim for single-ecosystem
  consumers.

### Notes

- This is the first entry to carry an explicit version. Earlier history is in
  the upstream repository's commit log.
