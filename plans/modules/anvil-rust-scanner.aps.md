<!--
APS Module: Anvil Rust Scanner
===============================
Ports the registry-driven scanner from TypeScript into the Rust CLI
(`crates/anvil-checks`). Makes the Rust scanner authoritative per
ADR-026. Closes the deferred ANVFMT-013 work. See: plans/aps-rules.md
-->

# Anvil Rust Scanner

| ID    | Owner | Status      |
| ----- | ----- | ----------- |
| RSCAN | —     | In Progress |

## Purpose

Make the Rust scanner in `crates/anvil-checks/src/antipattern/`
authoritative, registry-driven, and capable of scanning every artifact
type the `.anvil` format supports. Delete the hardcoded
`PATTERN_DEFS` array. Match the TS scanner's capability surface so
`anvil check` and `anvil scan` fire the current 18-rule family-based
catalogue, emit family provenance on warnings, and scale to tens of
concurrent artifact scans via `rayon`.

Authoritative ADR: [ADR-026](../decisions/026-rust-scanner-authoritative.md).

## Background

### Current state (2026-04-21)

The Rust scanner is the engine that ships in the `anvil` binary — when a
user runs `anvil check`, this is what executes. It contains a hardcoded
`PATTERN_DEFS: &[PatternDef]` array of 13 rules (AP-001..AP-013),
predating the `.anvil` format. It has:

- No connection to `patterns/compiled/registry.json`
- No `ArtifactKind` concept — everything is a source file on disk
- No `family` / `definition_ref` / `spectrum_position` metadata
- No RL-* / DD-* / GS-* rules
- Still includes retired AP-008..AP-013 HTML/CSS rules

Meanwhile the TS scanner in `packages/anvil/core/src/antipattern/` has
been refactored through ANVFMT-006..015 to read the compiled registry,
support artifact kinds, emit family provenance, and carry the current
catalogue. But it runs only from in-process TS surfaces (VSCode
extension, MCP server, embedded analysis).

This module closes that gap. After it lands, rule authors maintain
exactly one source — the `.anvil` files under `patterns/` — and both
scanners consume the same compiled `registry.json`.

### Performance requirement

The real CLI workloads involve tens of parallel artifact scans
(multi-PR gate checks, watch-mode fan-out, full-repo scans in CI). The
throughput target rules out Node as the primary engine and motivates
the `rayon::par_iter` parallelism in the scan loop. This is the
non-negotiable reason the Rust scanner is the authoritative one.

## Scope

**In scope:**

- `crates/anvil-checks/src/antipattern/registry_loader.rs` (new)
- Rewrite of `crates/anvil-checks/src/antipattern/patterns.rs` to read
  from registry
- Rust `AntiPattern` / `Warning` struct additions for family metadata
- Rust `ArtifactKind` enum and `scan_artifact` API
- `rayon`-driven parallel scan loop
- New CLI command or flag for pr-description / commit-message artifacts
- Shared scanner-parity fixture suite
- Deletion of hardcoded AP-008..AP-013 (retired by registry absence)

**Out of scope:**

- Retiring the TS scanner (separate follow-up module; requires napi-rs
  / WASM migration for VSCode + MCP surfaces)
- Changes to the `.anvil` format itself (frozen per ANVFMT)
- Changes to the compiler pipeline (TS-owned, stays TS-owned for now)
- Rust ownership of `scripts/compile-patterns`

## Interfaces

**Depends on:**

- `patterns/compiled/registry.json` — produced by the TS compiler
- `anvil-checks` crate — existing antipattern module (being rewritten)
- `anvil-cli` crate — CLI entry point
- `regex`, `rayon`, `serde_json` crates (already in the workspace)

**Exposes:**

- `anvil_checks::antipattern::scan_artifact(artifact, options)`
- `anvil_checks::antipattern::ArtifactKind`
- `anvil scan --artifact <kind> <path>` CLI subcommand (or equivalent)
- Rust `Warning` carrying `family` / `definition_ref` /
  `spectrum_position`

## Tasks

### RSCAN-001: Rust registry loader

- **Intent:** Rust can read and validate `patterns/compiled/registry.json`
- **Expected Outcome:** A `registry_loader` module reads the JSON file,
  validates shape with `serde_json`, caches by resolved path, and
  exposes `load_compiled_registry(opts)` and
  `load_registry_patterns(opts)`. Four-tier resolution: explicit path →
  `ANVIL_REGISTRY_PATH` env → cwd upward walk → executable directory
  upward walk. Graceful fallback (empty catalogue + warning diagnostic)
  when the registry is missing.
- **Scope:** `crates/anvil-checks/src/antipattern/registry_loader.rs`
- **Validation:** `cargo test -p anvil-checks -- registry_loader`
- **Confidence:** high
- **Status:** Complete

### RSCAN-002: Artifact model and API

- **Intent:** Rust mirrors the TS `Artifact` / `ArtifactKind` / `scan_artifact` surface
- **Expected Outcome:** `ArtifactKind` enum (source / pr-description / commit-message / agent-output), `Artifact` struct carrying `kind`, `ref`, `content`, and `scan_artifact(&Artifact, &ScanOptions)` that honours `pattern.targets` when present. `scan_file` becomes a wrapper building a source `Artifact`.
- **Scope:** `crates/anvil-checks/src/antipattern/types.rs`, `scanner.rs`
- **Dependencies:** RSCAN-001
- **Validation:** `cargo test -p anvil-checks -- scan_artifact`
- **Confidence:** high
- **Status:** Ready

### RSCAN-003: Family provenance on AntiPattern and Warning

- **Intent:** Rust emits the same warning metadata the TS scanner does
- **Expected Outcome:** Rust `AntiPattern` gains `family` / `definition_ref` / `spectrum_position` / `targets` fields; Rust `Warning` gains `family` / `definition_ref` / `spectrum_position`; JSON output shape stays backward-compatible (additive optional fields).
- **Scope:** `crates/anvil-checks/src/antipattern/types.rs`
- **Dependencies:** RSCAN-001
- **Validation:** `cargo test -p anvil-checks` + snapshot of `anvil check --json` output against a fixture
- **Confidence:** high
- **Status:** Ready

### RSCAN-004: Replace PATTERN_DEFS with registry-backed catalogue

- **Intent:** Delete the hardcoded `PATTERN_DEFS` array; `PATTERNS` comes from the registry
- **Expected Outcome:** `crates/anvil-checks/src/antipattern/patterns.rs` becomes a thin `LazyLock<Vec<AntiPattern>>` wrapper over `load_registry_patterns()`. The retired AP-008..AP-013 drop out because they aren't in the registry. `get_default_patterns` / `get_enabled_patterns` / `get_pattern_ids` / `get_pattern` still work, backed by the new source.
- **Scope:** `crates/anvil-checks/src/antipattern/patterns.rs`
- **Dependencies:** RSCAN-001, RSCAN-003
- **Validation:** `cargo test -p anvil-checks`; `anvil check` on a sample project fires only registry-sourced rules (no AP-008..AP-013)
- **Confidence:** high
- **Status:** Ready

### RSCAN-005: Parallel scan loop

- **Intent:** The scan path uses `rayon` to scan artifacts concurrently
- **Expected Outcome:** `run_antipattern_check` and `scan_artifacts` iterate with `par_iter`. Pattern compilation moves to a one-time `LazyLock` to avoid per-call regex construction. Throughput target: linear speedup on CPU-bound workloads up to physical core count.
- **Scope:** `crates/anvil-checks/src/antipattern/scanner.rs`, `check.rs`
- **Dependencies:** RSCAN-002, RSCAN-004
- **Validation:** `cargo bench -p anvil-bench --bench antipattern_scan` (or equivalent) shows multi-core scaling; existing `cargo test` suite stays green
- **Confidence:** medium
- **Status:** Ready

### RSCAN-006: CLI entry point for non-source artifacts

- **Intent:** Expose pr-description / commit-message scanning through the CLI
- **Expected Outcome:** `anvil scan --artifact <kind> <path>` (or `anvil check --artifact <kind> <file>`) routes to `scan_artifact` with the appropriate `ArtifactKind`. Exit code and JSON output match the existing `anvil check` shape. Closes the original ANVFMT-013 intent.
- **Scope:** `crates/anvil-cli/src/commands/` (new command or extension of `check.rs`)
- **Dependencies:** RSCAN-002, RSCAN-004
- **Validation:** `cargo test -p anvil-cli`; e2e fixture scanning a PR description triggers RL-* warnings
- **Confidence:** medium
- **Status:** Ready

### RSCAN-007: Shared scanner-parity fixture suite

- **Intent:** Prove the Rust scanner and TS scanner emit the same warnings for the same inputs, so the two-engine transition period doesn't cause UX drift
- **Expected Outcome:** `tests/scanner-parity/` (or similar) contains one fixture per rule ID: an input sample plus an expected warning list. A harness runs both engines against every fixture and asserts identical warning IDs and locations. Runs in CI.
- **Scope:** `tests/scanner-parity/`, a small runner invoking both engines
- **Dependencies:** RSCAN-004, RSCAN-006
- **Validation:** `pnpm test:scanner-parity` (or equivalent) passes with zero diffs
- **Confidence:** medium
- **Status:** Ready

### RSCAN-008: Documentation refresh

- **Intent:** Docs describe the authoritative Rust scanner and the parity story
- **Expected Outcome:** `docs/guides/anvil-rule-authoring.md` notes that `registry.json` is the contract and both engines consume it; `docs/public/anvil/` mentions the parallel scan throughput; `docs/architecture/` describes the two-engine state and the eventual napi-rs migration path.
- **Scope:** `docs/`
- **Dependencies:** RSCAN-004, RSCAN-006
- **Validation:** Grep finds no stale references to Rust AP-008..AP-013 or "Rust scanner has its own patterns"
- **Confidence:** high
- **Status:** Ready

## Risks

- **Regex engine differences.** Rust's `regex` crate is RE2-style (no
  backtracking, no lookaround) while V8 regex is PCRE-ish. Some patterns
  in the registry may not compile in Rust without rewrites.
  *Mitigation:* RSCAN-007 parity tests surface this at compile time;
  ANVFMT-001 schema already rejects JS-only flags, which is a partial
  check.
- **Registry path discovery under `cargo install`-ed binaries.** The
  four-tier resolution order relies on reaching the monorepo from the
  binary location. When `anvil` is installed globally, that walk fails.
  *Mitigation:* `ANVIL_REGISTRY_PATH` env + graceful fallback diagnostic;
  longer term, embed the registry via `include_str!` for the shipped
  binary.
- **Parallel warning ordering.** `rayon::par_iter` produces
  non-deterministic interleaving; existing tests may rely on insertion
  order.
  *Mitigation:* sort warnings by `(file, line, column, id)` before
  return; most consumers already do.
- **Performance regression from registry parse on cold start.** Parsing
  hundreds of patterns through `serde_json` at every CLI invocation
  could dominate short-lived `anvil check --staged` runs.
  *Mitigation:* `LazyLock` caches the parsed registry per process; for
  multi-invocation scenarios, shells should reuse the registry path via
  the env var.

## Milestones

- **M1 (RSCAN-001, RSCAN-003):** Rust can load and represent the
  registry. No runtime change yet.
- **M2 (RSCAN-002, RSCAN-004):** `anvil check` fires the full 18-rule
  catalogue with family metadata; AP-008..AP-013 retire from the CLI.
- **M3 (RSCAN-005, RSCAN-006):** Parallel scan + pr-description CLI
  command live. Original ANVFMT-013 closed.
- **M4 (RSCAN-007, RSCAN-008):** Parity guaranteed in CI; docs
  reconciled.

## Interfaces with Other Modules

- **anvil-file-format (ANVFMT):** This module consumes the artifact that
  ANVFMT produces. ANVFMT-013 is formally reparented here as RSCAN-006.
- **Future "retire TS scanner" module:** Depends on M4 landing. Not in
  scope here.

## Progress Log

- **2026-04-21 — RSCAN-001 landed.** New
  `crates/anvil-checks/src/antipattern/registry_loader.rs` loads
  `patterns/compiled/registry.json`, validates schema_version=1 with
  `serde_json`, caches per resolved path, and implements the four-tier
  path resolution (explicit → `ANVIL_REGISTRY_PATH` → cwd walk → exe-dir
  walk). Exposes `load_compiled_registry`, `load_registry_patterns`,
  `compiled_to_antipattern`, and `reset_registry_cache`. Extended
  `AntiPatternCategory` with `TypeEvasion` / `Accountability` /
  `DeferredDebt` variants so the mapper round-trips family categories
  faithfully. AST-detection rules are skipped pending future scanner
  AST support. 10 loader unit tests cover load/cache/schema rejection
  paths plus regex-mapping fidelity and the AST skip. Clippy clean at
  `-D warnings`.
