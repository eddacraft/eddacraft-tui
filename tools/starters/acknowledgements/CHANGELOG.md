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
  reporting it is a fix. Gate fixes of that kind ship as minor, and every one is
  called out under its own heading in the entry so an upgrade never surprises
  you. See 1.1.0 for the first instance.

- **minor** — additive and backwards-compatible (a new ecosystem driver, a new
  optional field), plus gate fixes as described above.
- **patch** — fixes, docs, or determinism work: no change to behaviour that a
  correct configuration could observe.

## [1.2.1] - 2026-08-14

The documented first-copy path for a Node-only consumer now works. The generator
core in 1.2.0 was fine; the templates, expander, Node driver lookup, and CI
snippet were not.

### Fixed

- **Copying the shipped template and example config is no longer an immediate
  orphan error.** The acknowledgements template uses `{{BLOCK_NAME}}` in the
  marker pair. Replace it with the block `name`.
- **A Node-only repo can populate its allow-list.** The kit ships
  `licences.toml.template`. The expander no longer requires `about.toml` or
  `deny.toml`.
- **`license-checker` installed as a devDependency is found.** The Node driver
  walks `node_modules/.bin` from the manifest. You do not need it on `PATH`.
- **A published package is not attributed to itself.** The driver always
  excludes the manifest's own `name@version`.

### Changed

- `ci-freshness.yml.snippet` is a generator `--check` job with commented
  per-ecosystem setup. It no longer installs cargo-about or runs a Rust fixture.

### Documentation

- The adoption checklist covers Node-only bootstrap, `{{BLOCK_NAME}}`, and
  putting `ACKNOWLEDGEMENTS.md` in `package.json` `"files"` so `npm pack`
  includes it.

## [1.2.0] - 2026-08-14

Reproduces every distinct copyright notice instead of one per licence family.
The Rust template previously dropped most attributions — **if you generate a
file you redistribute, upgrade and regenerate.** This is also the first cut that
ships a licence grant; pin this release (or later), not `v1.0.0` / `v1.1.x`.

### Added

- **The kit ships an Apache License 2.0 grant.** The published mirror previously
  had no `LICENSE`, so GitHub reported `NOASSERTION` and a third party following
  the adoption instructions had no licence to copy the kit under. `LICENSE` now
  travels with the subtree.

### Documentation

- The public pin example now uses `v1.2.0`, not `v1.0.0`. The older tag still
  carries the silent splice-delete and stale-`--check` defects; pinning it is
  not a safe starting point.
- The kit README lists `LICENSE`, `VERSION`, and `CHANGELOG.md` among what
  ships, and states the Apache-2.0 grant.

### Fixed

- **The Rust template reproduced one licence text per licence _family_, so every
  other copyright holder's notice was dropped.** `about.hbs.template` iterated
  `overview`, which carries a single representative text per SPDX id. A project
  with 361 MIT crates emitted exactly one MIT block — one crate's copyright
  line, standing in for all of them — under a heading claiming each licence was
  "reproduced in full below". MIT and the BSD licences condition redistribution
  on retaining _the above copyright notice_, so the generated file did not
  satisfy the terms it asserted. The template now iterates `licenses`, which
  cargo-about keys by distinct licence text, emitting one block per notice and
  naming the crates that share it, each as `name version` on its own line so the
  list stays diffable and two versions of the same crate are told apart.

  Regenerating will grow the generated block substantially — that growth is the
  attributions that were previously missing.

- **Crates that publish no licence file are now labelled rather than presented
  as attributed.** Where a crate ships no licence file, cargo-about substitutes
  the canonical SPDX text, whose copyright fields are the literal placeholders
  `<year> <owner>`. Those blocks are now marked _(canonical SPDX text)_ so the
  file does not present a placeholder as a holder's notice.

- **Generated blocks could leave the freshness gate permanently red.** Licence
  texts are copied verbatim from upstream packages and some ship CRLF files, so
  a generated block could carry mixed line endings. In a repo whose
  `.gitattributes` normalises the target to LF — the usual case for `*.md` — the
  checked-out file and the freshly generated block then differed on every run,
  failing `--check` over a difference that could not be committed away. The
  dispatcher now normalises driver output to LF before splicing, stripping only
  a trailing CR so that a bare CR inside a line — which is reproduced content,
  not a line ending — is preserved. Line endings are not part of a licence's
  meaning; no notice is altered.

## [1.1.1] - 2026-08-04

Repairs the marker gates 1.1.0 introduced. Independent review found that those
gates both rejected valid files and could be silently switched off — **if you
are on 1.1.0, upgrade.**

### Fixed

- **The orphaned-marker gate could be disabled by a valid document, leaving
  stale attribution undetected.** Fence tracking was a simple on/off toggle, so
  a `~~~` line inside a ` ``` ` block — ordinary content in CommonMark, not a
  fence — flipped it on and left it on. Every marker after that point went
  unseen, and `--check` reported green over a retired block's frozen content. An
  unbalanced fence anywhere in the file did the same. Fences are now tracked per
  CommonMark, by fence character and length.
- **Prose that mentioned a marker was treated as markup.** A migration note
  citing a retired block's marker, an inline code span, or link text containing
  one was reported as an orphaned block, failing the build over hand-curated
  content the README promises is left alone. A marker is now recognised only
  when it is the entire line.
- **Two markers sharing one line counted as one.** The count gate counted
  matching _lines_, so a duplicated marker on a single line passed a gate the
  README says catches duplicates.
- **A `marker_begin` / `marker_end` override containing a backslash disabled the
  orphaned-marker gate entirely.** The marker text was passed to `awk` in a way
  that applied escape processing, so `C:\temp` became `C:<tab>emp` and matched
  nothing.
- The gates and the splice now read one shared scan of the target instead of
  each re-matching the text, so they can no longer disagree about which lines
  are markers.

### Documentation

- The README now documents the marker-order and orphaned-marker gates, and
  defines exactly what counts as a marker. 1.1.0 shipped both gates without
  documenting either.
- 1.1.0's note that "markers inside fenced code blocks are ignored" was true
  only of the orphaned-marker gate. The count gate did not skip fences, so
  documenting the markers you actually use still failed. Both now share the same
  rule.

## [1.1.0] - 2026-08-03

A hardening release. The kit gains no new capability; it stops failing quietly
in the places where it previously reported success.

### Configurations that used to pass and now fail

Both cases below exited **0** before this release while producing wrong output.
If your build goes red on upgrade it is one of these, and the file was already
wrong:

- **A marker pair in the wrong order** (`END` above its `BEGIN`) is now refused.
  Previously the generator printed `Updated <file>`, exited 0, and deleted every
  line from the `BEGIN` marker to the end of the file — silently discarding the
  hand-curated content this kit promises to preserve. _Fix:_ put the markers in
  the right order, and check your history if content has already gone missing.
- **A marker pair with no matching block** in `attribution.toml` is now reported
  by name. Previously a block you deleted or renamed kept its last generated
  content indefinitely and `--check` reported green over it — stale licence
  attribution passing a freshness gate. _Fix:_ delete the orphaned marker pair,
  or re-declare the block.

Markers inside fenced code blocks are ignored, so a target that documents the
marker syntax in its own prose is unaffected.

### Fixed

- A `#` inside a quoted `attribution.toml` value is data, not the start of a
  comment. `manifest_path = "vendor/c#sharp/Cargo.toml"` previously reached the
  driver truncated to `vendor/c` and resolved somewhere else entirely.
- The Node driver escapes `|` in every cell it renders, so a package name,
  licence expression, or repository URL containing one can no longer split a row
  into extra columns. The Go and Python renders are **not** covered: their rows
  come from an upstream template and from `pip-licenses` itself, so the driver
  never builds those cells.

### Added

- `tests/run-all.sh` — one entry point for the kit's self-tests, so the suite is
  runnable without reconstructing it from a CI file. `--require-all` turns a
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
