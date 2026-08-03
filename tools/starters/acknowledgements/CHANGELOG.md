# Changelog

All notable changes to the acknowledgements starter kit are recorded here. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the
kit uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

- **major** — a breaking change to a consumer contract: the `attribution.toml`
  schema, a driver's CLI, the marker-splice format, or the freshness-gate exit
  **table** (what exit codes 0, 1 and 2 mean).

  Note the narrow reading of that last one. Tightening a gate so that input
  which previously exited 0 now exits 1 is **not** a major change, provided the
  meaning of the exit codes themselves is unchanged. Such input was producing
  silently wrong output — a stale block, a truncated path, deleted prose — and
  reporting it is a fix. Gate fixes of that kind ship as minor, and every one
  is called out under its own heading in the entry so an upgrade never
  surprises you. See 1.1.0 for the first instance.

- **minor** — additive and backwards-compatible (a new ecosystem driver, a new
  optional field), plus gate fixes as described above.
- **patch** — fixes, docs, or determinism work that changes no behaviour a
  correct configuration could observe.

## [1.1.0] - 2026-08-03

A hardening release. The kit gains no new capability; it stops failing quietly
in the places where it previously reported success.

### Configurations that used to pass and now fail

Both cases below exited **0** before this release while producing wrong output.
If your build goes red on upgrade it is one of these, and the file was already
wrong:

- **A marker pair in the wrong order** (`END` above its `BEGIN`) is now
  refused. Previously the generator printed `Updated <file>`, exited 0, and
  deleted every line from the `BEGIN` marker to the end of the file — silently
  discarding the hand-curated content this kit promises to preserve. *Fix:* put
  the markers in the right order, and check your history if content has already
  gone missing.
- **A marker pair with no matching block** in `attribution.toml` is now
  reported by name. Previously a block you deleted or renamed kept its last
  generated content indefinitely and `--check` reported green over it — stale
  licence attribution passing a freshness gate. *Fix:* delete the orphaned
  marker pair, or re-declare the block.

Markers inside fenced code blocks are ignored, so a target that documents the
marker syntax in its own prose is unaffected.

### Fixed

- A `#` inside a quoted `attribution.toml` value is data, not the start of a
  comment. `manifest_path = "vendor/c#sharp/Cargo.toml"` previously reached the
  driver truncated to `vendor/c` and resolved somewhere else entirely.
- The Node driver escapes `|` in every cell it renders, so a package name,
  licence expression, or repository URL containing one can no longer split a
  row into extra columns. The Go and Python renders are **not** covered: their
  rows come from an upstream template and from `pip-licenses` itself, so the
  driver never builds those cells.

### Added

- `tests/run-all.sh` — one entry point for the kit's self-tests, so the suite
  is runnable without reconstructing it from a CI file. `--require-all` turns a
  skipped test into a failure; `--list` prints the list.
- `kit-tests.yml.snippet` — a workflow to copy into your own
  `.github/workflows/`, carrying the external scanner versions the driver tests
  expect.

### Changed

- The self-test suite runs on macOS as well as Linux. The kit needed no changes
  to pass — its portability had simply never been exercised.

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
