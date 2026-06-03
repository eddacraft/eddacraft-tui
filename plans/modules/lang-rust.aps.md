<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Rust Language Anchor (Track 1)

| ID     | Owner   | Status | Done |
| ------ | ------- | ------ | ---- |
| RSTLAN | @aneki | In Progress | 2/8  |

**Last reviewed:** 2026-06-03 — NBI "RSTLAN re-eval — Rust anchor scoping" completed. ADR-065 (Rust T3 architecture enforcement location — Rust-native) Accepted; anchor re-scoring snapshot 2026-06-03 recorded (sequence unchanged, Rust elevated to #2 for dogfood); LANGTS 6/6 + kernel prereqs complete; module promoted Proposed → Ready with executable work items. Owner named. All Ready Checklist items now checked.

> **Priority update (2026-05-14, reaffirmed 2026-06-03):** RSTLAN is the first Language &
> Coverage Track 1 anchor after TS because Anvil's primary implementation surface (kernel, daemon, intercept, witness, CLI, graph) is Rust. Self-governance of the shipped substrate is a credibility requirement before wider pack expansion. The module is now Ready; implementation authorised.

## Purpose

Bring Rust to **T3 (Governed)** per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.1, §8.1. Rust is the credibility test for "Anvil governs systems code" —
two confirmed demand points (Anvil's own kernel + User B), and Anvil currently
cannot see its own primary implementation language. T3 means: tree-sitter
grammar wired, full symbol/import extraction, anti-pattern catalogue,
suppression syntax, entry-point detection, layer/boundary enforcement, policy
hook integration, drift baseline, included in `architecture-validate`.

This module **rewrites** the previous regex-era Rust placeholder. The previous
content assumed regex-based parsing and an `HTMLCSS-001` prerequisite that has
since been archived. Tree-sitter-based reality changes the implementation
shape entirely.

## In Scope

- `tree-sitter-rust` grammar wired in `crates/anvil-kernel/src/parser/`
  through whatever extractor abstraction `LANGTS` produces.
- File detection: `.rs`.
- Symbol/import extraction handling Rust shapes that do not map to the
  current JS-shaped extractor: `mod`, `use`, `pub`, `pub use`, `crate::`,
  `super::`, `self::`, `extern crate`, namespaced `use foo::{a, b}`,
  re-exports, `#[path]`, workspace-relative paths.
- T2 anti-pattern catalogue (per spec §8.1):
  - `unwrap()` / `expect()` in non-test code
  - `unsafe` blocks without safety comment
  - `.clone()` in hot loops (flag, do not block)
  - `todo!()` / `unimplemented!()` shipped
  - `panic!()` in library code
  - **Serde deserialisation hygiene** — `#[serde(deny_unknown_fields)]`
    missing on external-input structs, `#[serde(flatten)]` without
    validation, `Deserialize` on types containing secret fields, custom
    `deserialize_with` without bounds. Serde folded into the Rust anchor
    rather than its own pack — too ubiquitous to not be a language concern.
- Suppression syntax via `// @anvil-ignore <ID>: <reason>` (already supported
  for `//` comments).
- Entry-point detection: `fn main()` in `src/main.rs`, `Cargo.toml` `[[bin]]`
  targets, workspace member crates.
- Layer/boundary enforcement reaching Rust crates and modules (realised by
  ADR-065: Rust-native in `anvil-architecture` + kernel edges; see RSTLAN-005).
- Drift baseline default-on for `.rs` files.
- `architecture-validate` includes Rust crates and module graphs.

## Out of Scope

- Cargo dependency-graph analysis (lives in `config-intelligence`).
- Macro expansion, lifetime/borrow-checker analysis.
- proc-macro crate analysis.
- Tokio-specific async patterns (lives in `pack-tokio`).
- Axum / other framework patterns (Phase 3 packs).

## Interfaces

**Depends on:**

- [`lang-ts-audit`](./lang-ts-audit.aps.md) — T3 acceptance checklist + kernel
  prerequisites (LANGTS-005) — satisfied (6/6, checklist published, extractor
  trait + grammar_version live).
- Kernel prerequisite work from `lang-ts-audit` (extractor trait, grammar
  version in cache key, parser thread-safety, panic removal) — satisfied.
- Existing `crates/anvil-kernel/src/parser/`, architecture analysis
  (`crates/anvil-architecture`), policy pipeline, drift baseline, suppression
  parser.
- ADR-065 (Rust T3 architecture enforcement location) — satisfied.

**Exposes:**

- Rust at T3 — first-phase dogfood coverage for Anvil's own crates and
  substrate-tier prerequisite for `pack-tokio` and (Phase 3) `pack-axum`.
- Rust portion of T3 acceptance evidence — calibration data for Python anchor.

## Prerequisites

- `lang-ts-audit` complete (T3 acceptance checklist exists) — satisfied.
- ADR recorded for council §16.5 #5 (T3 architecture enforcement location) — satisfied (ADR-065 Accepted 2026-06-03).
- Re-scoring gate run per
  [docs/guides/anchor-rescoring-process.md](../../docs/guides/anchor-rescoring-process.md)
  before this module starts — satisfied (snapshot 2026-06-03 recorded; sequence unchanged).

## Ready Checklist

Change status to **Ready** when:

- [x] LANGTS complete and T3 acceptance checklist published. (LANGTS 6/6 via PR #2125; checklist at `plans/specs/2026-04-26-t3-acceptance-checklist.md`; LANGTS-005 kernel prereqs Merged #2096 unblock extractor wiring.)
- [x] ADR for Rust T3 architecture enforcement location recorded. (ADR-065 Accepted 2026-06-03 — Rust-native in `anvil-architecture` + kernel edges; no TS shim.)
- [x] Re-scoring gate snapshot recorded; Rust still anchor #2 after TS. (Snapshot at `plans/decisions/anchor-rescore-2026-06-03.md`; demand/strategic elevated by Anvil dogfood + Rust migration; sequence TS → Rust → Python unchanged.)
- [x] Owner named for the anchor work. (@aneki; planning agent handoff under NBI completion.)

## Work Items

All items are Ready (module promoted 2026-06-03 after NBI re-eval, ADR-065, re-score snapshot, and LANGTS/kernel gates). Items are sequenced for minimal dependency fan-out but several can run in parallel waves once the grammar + extractor base lands (see waves in execution plan if filed).

### RSTLAN-001: Wire tree-sitter-rust grammar and Language variant — Merged

- **Status:** Merged 2026-06-04 via PR #2303
- **Intent:** Add `tree-sitter-rust` support to the kernel parser so `.rs` files are recognised and produce a tree-sitter AST under the unified `LanguageExtractor` contract (K1 from LANGTS-005).
- **Expected Outcome:** `Language` enum gains `Rust` variant; `from_path` returns it for `.rs`; `ts_language()` binds the grammar; `grammar_version()` produces a stable discriminator; `cargo test -p eddacraft-anvil-kernel -- parser::languages` and new Rust-specific tests pass; Cargo.toml pins the grammar crate.
- **Scope:** `crates/anvil-kernel/Cargo.toml` (add tree-sitter-rust dep), `crates/anvil-kernel/src/parser/languages.rs` (enum + match arms + tests), parser mod re-exports if needed.
- **Non-scope:** Symbol extraction logic (RSTLAN-002); anti-patterns; architecture edges.
- **Dependencies:** LANGTS-005 (extractor trait + grammar_version landed); LANGTS complete.
- **Validation:** `cargo test -p eddacraft-anvil-kernel` passes (runs the unit tests inside `parser/languages.rs` including grammar_version distinctness); extend the tests for the new Rust variant + from_path + smoke parse of a real .rs; `cargo check -p eddacraft-anvil-kernel` succeeds after adding the grammar dep. Reproducible via `cargo test -p eddacraft-anvil-kernel` (no new test binary required unless the item adds one).
- **Confidence:** high — grammar crate is mature; pattern follows existing TS/JS wiring.
- **Files:** crates/anvil-kernel/Cargo.toml, crates/anvil-kernel/src/parser/languages.rs

### RSTLAN-002: Implement Rust symbol and import extraction (mod/use shapes) — Merged

- **Status:** Merged 2026-06-04 via PR #2303
- **Intent:** Provide a `rust.rs` extractor (implementing `LanguageExtractor`) that walks the tree-sitter-rust AST and emits `SymbolNode`s + `ImportEdge`s for Rust module shapes so the symbol graph and downstream consumers (architecture, drift, checks) see Rust crates.
- **Expected Outcome:** `FileSymbols` for `.rs` files contains module symbols, use/import edges (relative crate:: super:: self::, pub use, namespaced uses, #[path] re-exports resolved at least to the declaring file), extern crate; dispatch arm in `extract_symbols` routes Language::Rust to the new extractor; existing TS paths unaffected; round-trip tests on representative Rust idioms pass.
- **Scope:** New `crates/anvil-kernel/src/parser/extract/rust.rs` (or query-driven), update `extract/mod.rs` dispatch + Language match, `FileSymbols`/`ImportEdge` if Rust needs additive fields (keep minimal), tests/fixtures under parser/extract or integration.
- **Non-scope:** Full re-export name tracking beyond what's needed for boundaries (defer per T3 checklist); proc-macro expansion; lifetime analysis.
- **Dependencies:** RSTLAN-001 (grammar must be wired first).
- **Validation:** `cargo test -p eddacraft-anvil-kernel` passes and covers the new extractor (add unit tests in the new module or existing parser tests); the dispatch and shapes for mod/use etc. are exercised; additionally run a smoke using the kernel's own source: `cargo test -p eddacraft-anvil-kernel -- --quiet` (or a specific test filter for extract). Workspace resolution smoke via `cargo metadata` + manual parse check as described. Reproducible with `cargo test -p eddacraft-anvil-kernel` today (no new `--test parser` binary required for this item).
- **Confidence:** medium — Rust module system is richer than JS; workspace-relative resolution may need Cargo.toml reading (see Risks).
- **Files:** crates/anvil-kernel/src/parser/extract/rust.rs, crates/anvil-kernel/src/parser/extract/mod.rs, tests

### RSTLAN-003: Rust T2 anti-pattern catalogue (unwrap, unsafe, serde, etc) — Ready

- **Status:** Ready
- **Intent:** Add the language-level T2 anti-pattern rules from the design spec so Rust code triggers the same governance surface as TS `any` / `as any`.
- **Expected Outcome:** New patterns under `patterns/` (or Rust family) for `unwrap()`/`expect()` (non-test), `unsafe` without safety comment, `todo!`/`unimplemented!` shipped, `panic!` in lib, `.clone()` in hot (flag), and Serde hygiene rules; rules appear in compiled registry; scanner detects them on `.rs` files; suppression via `// @anvil-ignore` works.
- **Scope:** Pattern sources (new `rust-*.anvil` or equivalent), family registration, `crates/anvil-checks` scanner or registry loader updates if language dispatch needed, build.rs / snapshot tests.
- **Non-scope:** Framework packs (Tokio etc.); changing severity defaults without ADR.
- **Dependencies:** RSTLAN-001 (to have Rust files reach the scanner in the first place).
- **Validation:** `cargo test -p eddacraft-anvil-checks` (or equivalent) + fixture pairs asserting the named patterns fire on representative bad Rust and are clean/suppressible on good equivalents; end-to-end `anvil check` on a temp Rust file with the patterns.
- **Confidence:** high for basic rules; medium for precise "non-test" and "hot loop" heuristics.
- **Files:** patterns/, crates/anvil-checks/src/
- **Detection-mechanism finding (2026-06-04, during -003 scoping):** the
  `anvil-checks` antipattern scanner is **regex + same-line post-filter only**
  (`scanner.rs::rewrite_spec`; `FilterSpec::Negative`/`TrailingByteOrEol` act on
  the matched line, with no adjacent-line or AST context). This splits the
  catalogue by feasibility:
  - **Regex-clean, low-FP (ship as-is):** `todo!()` / `unimplemented!()` shipped
    — rare in tests, no context needed.
  - **Needs `#[cfg(test)]`-module awareness:** `unwrap()` / `expect()` (non-test)
    and `panic!` (lib) — a path-based allowlist cannot see inline
    `#[cfg(test)] mod tests`, so a pure-regex version false-positives heavily on
    Anvil's own inline unit tests and would fail the §16.5 #9 FP bar at -008.
  - **Needs adjacent-line context:** `unsafe` without a `// SAFETY:` comment
    (the comment is on the preceding line).
  - **Needs real AST:** serde hygiene (`deny_unknown_fields`, `flatten`,
    secret-field `Deserialize`) and `.clone()`-in-hot-loop.
  Implication: RSTLAN-003 wants a detection-context decision before build —
  accept-FP-with-aggressive-allowlists, or invest in cfg(test)/AST-aware
  detection (the new RSTLAN-002 kernel Rust extractor is the natural home for an
  AST detection path). Recommend splitting: **-003** = regex-clean rules now,
  **-003b** (new) = the context/AST-dependent rules, gated on that decision.
  Owner steer requested before implementing.

### RSTLAN-004: Entry-point detection for Rust binaries and workspaces — Ready

- **Status:** Ready
- **Intent:** Ensure `anvil` knows the entry points of a Rust workspace (bins, lib roots, [[bin]] targets) so baseline + layer assignment + "what to protect" are correct for multi-crate Rust projects.
- **Expected Outcome:** Entry-point detector (or kernel equivalent) surfaces `src/main.rs` `fn main`, `Cargo.toml` [[bin]]/[[example]], workspace member roots; these appear in architecture baselines and `anvil architecture` surfaces for mixed or pure-Rust repos.
- **Scope:** Extension of entry-detector (TS or Rust arch crate), or new Rust-side collector consumed by `anvil-architecture`; integration in gate / baseline creation paths; tests on the Anvil monorepo itself.
- **Non-scope:** Full Cargo metadata parsing beyond bin targets (dep graph is config-intelligence).
- **Dependencies:** RSTLAN-001/002 (need to parse the entry files); may coordinate with RSTLAN-005.
- **Validation:** Baseline created on a workspace with multiple bins + lib produces the expected entry points; `anvil architecture` or internal calls list them.
- **Confidence:** medium — entry detection currently lives in TS analyser; Rust path needs the hand-off point clarified in implementation.
- **Files:** crates/anvil-architecture/, crates/anvil-cli/src/commands/gate.rs (or shared), packages/anvil/core/src/architecture/entry-detector.ts (legacy updates only)

### RSTLAN-005: Layer/boundary enforcement reaches Rust crates and modules (per ADR-065) — Ready

- **Status:** Ready
- **Intent:** Make `anvil architecture validate`, `anvil gate`, baseline diff, and new-edge classification work for cross-crate Rust imports and Rust↔TS boundaries inside a mixed workspace, using the Rust-native path.
- **Expected Outcome:** `collect_source_files` already includes .rs; `extract_import_edges` (gate) and architecture validator consume kernel-supplied Rust edges (from RSTLAN-002); Rust `use` / `mod` resolved to actual files for layer matching; violations emitted for new cross-layer Rust edges; baseline round-trips include .rs files; "new edges only" contract holds for Rust changes.
- **Scope:** Updates to `crates/anvil-cli/src/commands/gate.rs` (remove JS/TS hard filter, use kernel for .rs or all when available), `crates/anvil-architecture` resolver extensions if ImportEdge shape insufficient for Rust, any TS analyser parity notes (legacy), integration tests, Anvil monorepo architecture.yaml if needed for dogfood.
- **Non-scope:** Changing the layer model itself (user-owned in .anvil/architecture.yaml); full Cargo graph (config-intelligence).
- **Dependencies:** RSTLAN-001, RSTLAN-002 (edges), ADR-065 (this item realises the decision), RSTLAN-004 (entry points affect some boundaries).
- **Validation:** On the Anvil monorepo (or a test workspace with .ts + .rs cross-layer), after wiring: `cargo run --bin anvil -- gate --architecture . 2>&1 | grep -E '\.rs:'` shows Rust files participating (or clean run with baseline); `cargo test -p eddacraft-anvil-architecture --test validator` + gate integration tests pass with Rust edges; `cargo test -p eddacraft-anvil-cli --test gate` (or the architecture smoke in gate tests) asserts Rust-origin ImportEdge produces layer/boundary findings consistent with the baseline (new vs existing classification). Also exercises workspace member resolution.
- **Confidence:** medium-high — the validator already accepts edges; the work is primarily in the extractor + call-site plumbing.
- **Files:** crates/anvil-cli/src/commands/gate.rs, crates/anvil-architecture/src/validator.rs (resolver), crates/anvil-kernel/src/parser/extract/rust.rs

### RSTLAN-006: Drift baseline default-on for `.rs` files — Ready

- **Status:** Ready
- **Intent:** Ensure that once a workspace has a baseline, `.rs` files participate in drift detection by default (no opt-in per-language flag).
- **Expected Outcome:** `anvil drift` and baseline machinery include .rs files in the scanned set and edge set when the grammar+extractor are present; a pure-Rust or mixed workspace gets drift coverage for Rust without extra config.
- **Scope:** Verify / extend the include logic in drift and baseline paths (likely already generic via architecture collect or kernel scan); update any docs or defaults; tests.
- **Non-scope:** New drift UI or policies for Rust (those are separate).
- **Dependencies:** RSTLAN-001/002 (so Rust files produce symbols/edges that drift can consume).
- **Validation:** `anvil drift` on a repo with .rs changes reports Rust edges; baseline JSON contains Rust file entries.
- **Confidence:** high — the substrate is shared; mostly a "remove filter" + test item.
- **Files:** crates/anvil-cli/src/commands/drift.rs, crates/anvil-architecture/src/baseline.rs, related kernel scan sites

### RSTLAN-007: `architecture-validate` surface includes Rust crates/modules — Ready

- **Status:** Ready
- **Intent:** The `anvil architecture validate`, `anvil gate` (architecture mode), dashboard architecture views, and MCP query tools all report Rust layer assignments and boundary findings when Rust support is active.
- **Expected Outcome:** Running the surfaces on Anvil's own source (or a test Rust workspace) produces layer assignments for .rs files and boundary violations where rules are breached; output (text/JSON/SARIF) contains Rust paths; no "Rust ignored" silent paths.
- **Scope:** CLI command surfaces, TUI dashboard architecture widget, any MCP tools, docs examples; ensure they go through the Rust `anvil-architecture` path for .rs.
- **Non-scope:** New TUI widgets or SARIF extensions (those are separate modules).
- **Dependencies:** RSTLAN-005 (enforcement), RSTLAN-004.
- **Validation:** End-to-end: `cargo run --bin anvil -- architecture validate` (or built equivalent) on the monorepo emits Rust findings; snapshot or insta tests updated if output shape changes for Rust.
- **Confidence:** high once enforcement wired.
- **Files:** crates/anvil-cli/src/commands/architecture.rs, crates/anvil-cli/src/commands/gate.rs, crates/anvil-cli/src/commands/dashboard/architecture.rs, eddacraft-tui surfaces if any

### RSTLAN-008: Dogfood T3 acceptance on Anvil's own kernel (validate + FP bar) — Ready

- **Status:** Ready
- **Intent:** Run the full T3 bar (grammar, extraction, anti-patterns, suppression, entry, layer/boundary, drift baseline, `architecture-validate`) against Anvil's Rust crates as the primary acceptance evidence; demonstrate FP rate acceptable per council §16.5 #9 and at least the spirit of "≥1 external codebase".
- **Expected Outcome:** Anvil monorepo baseline created + validated with Rust paths included; zero panics during parse/extract on the kernel; new Rust-originated boundary violations are either pre-existing (baselined) or intentionally introduced+suppressed with reason; FP classification matches the revised acceptance bar; evidence captured (baseline diff, run output, count of findings).
- **Scope:** Running the tools on the live repo, capturing artefacts under plans/ or a review note, updating any self-architecture.yaml if gaps found, closing the item with explicit "passes T3 checklist + bar" statement.
- **Non-scope:** Fixing all pre-existing architecture debt in Anvil (that's adoption work); changing the bar itself.
- **Dependencies:** All prior RSTLAN-001..007; the module's own Rust substrate must be governed by the end.
- **Validation:** Explicit checklist pass + FP classification evidence recorded (e.g. in the commit or `plans/reviews/` note): create baseline + run `anvil architecture validate` (or `gate --architecture`) over the monorepo; count Rust-originated findings, classify pre-existing vs new, record rate (target per §16.5 #9: FP rate < N% AND evidence of external or Anvil dogfood run). `cargo test -p eddacraft-anvil-architecture --test validator` + kernel parser tests green; any new Rust baseline violations are either suppressed with reason or pre-baselined. The landing PR/commit includes the run log snippet + classification summary.
- **Confidence:** medium — depends on the quality of the earlier items; Anvil is a complex workspace (multiple crates, mixed TS/Rust, build.rs, etc.) so real edge cases will appear.
- **Files:** The monorepo source (self-governed), plans/ review artefacts, .anvil/architecture.yaml (if adjustments), continuous-improvement or post-merge notes.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| `use` / `mod` paths need crate + workspace context for correct resolution | Medium | Read nearest `Cargo.toml` for `[package] name` + `[workspace] members`; start with Anvil monorepo as the validation target (see RSTLAN-008) |
| `unsafe` false positives in FFI / bindgen / low-level crates | Medium | Severity: info for `unsafe` by default; allowlist pattern for generated FFI dirs; document in T2 catalogue |
| Macro-generated / proc-macro-hidden imports and symbols invisible to static extract | Low | Document limitation in T3 checklist and user runbook; the graph is conservative (missed edges = missed drift, not false violations); revisit if a future macro-expansion surface arrives |
| Workspace vs single-crate layouts + virtual manifests confuse file-to-crate mapping | Medium | Detect via `Cargo.toml` presence + workspace metadata; fall back to directory heuristics; dogfood on Anvil first |
| Complex re-exports and `#[path]` attributes produce incomplete or surprising edges | Low-Medium | Conservative resolution (best-effort file target); explicit note in acceptance evidence; tests on real Rust idioms in the kernel itself |
| FP rate on first dogfood exceeds the §16.5 #9 bar (or external codebase signal is thin) | Medium | Capture rate + classification in RSTLAN-008 evidence; use "info" for borderline rules; do not block the anchor on perfect cleanliness — the bar revision allows <N% + external run |

## Open Questions

- [ ] Should `unsafe fn` and `unsafe {}` blocks be flagged separately (distinct IDs or one with sub-reason)?
- [ ] How are workspace-internal vs public crate boundaries best declared in `.anvil/architecture.yaml` (or inferred)?
- [ ] Should `#[allow(...)]` / `#[clippy::allow(...)]` and `#[allow(clippy::...)]` be treated as equivalent to `@anvil-ignore` for suppression accounting and baseline purposes?
- [ ] Re-scoring owner — permanent named owner still open (this snapshot used NBI owner + review expectation); promote to standing role per §17.3?
- [ ] Exact shape of Rust `ImportEdge` resolution for `extern crate` and renamed uses — does the graph need an explicit "crate name" field, or is file target sufficient for boundaries?
