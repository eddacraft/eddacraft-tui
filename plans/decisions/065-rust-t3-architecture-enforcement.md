# ADR-065: Rust T3 Architecture Enforcement Location — Rust-native

## Status

Accepted

## Date

2026-06-03

## Context

The 2026-04-08 Language and Coverage Design (§8.1, council finding C-019 / §16.5 #5) left the T3 architecture enforcement location for Rust as an explicit open decision: "TS shim vs Rust-native".

- `packages/anvil/core/src/architecture/` (TypeScript) contains the original layer detector, edge detector (regex-based for JS/TS + web), baseline, rego-generator, and `anvil architecture` surfaces. It hardcodes JS/TS include patterns and import shapes.
- `crates/anvil-architecture/` (Rust) is a stub that already mirrors the types, baseline, definition, validator, yaml parser, and `collect_source_files` (note: already includes "rs" in include_extensions). The CLI commands (`anvil gate`, dashboard architecture views, `anvil architecture *`) are implemented against the Rust crate (see crates/anvil-cli/src/commands/gate.rs, commands/architecture.rs, mcp/tools/query_boundary.rs).
- `crates/anvil-kernel/src/parser/` now owns tree-sitter extraction (post LANGTS-005: extractor trait, grammar_version cache key, panic removal). `extract_import_edges` in gate.rs currently filters to only JS/TS even when architecture file lists include .rs.
- `anvil-architecture` validator already accepts `ImportEdge[]` supplied by the caller; the kernel's `FileSymbols.imports` + resolution will supply Rust shapes once `tree-sitter-rust` and a Rust extractor impl are wired.
- ADR-012 (Rust CLI), ADR-026/033 (Rust scanner authoritative, TS scanner parked), ADR-040 (regorus policy), ADR-061 (daemon delta validation) and the v0.7 daemon-working slate have established Rust as the native home for hot governance surfaces (checks, intercept, witness, policy, graph). Keeping architecture enforcement in TS would require a shim (serialising Rust edges to TS, or a hybrid call) for any Rust file that participates in layer/boundary rules.

Rust crate/module semantics (mod declarations, `use`, `pub use`, `crate::`, `super::`, `#[path]`, workspace members, lib.rs/bin.rs, extern crate) do not map to the JS-shaped import extractor or the TS regex edge detector. Layer assignment via globs is language-agnostic, but edge extraction and "import" resolution for boundaries are not.

The NBI "RSTLAN re-eval" (triggered by LANGTS 6/6) requires this decision before RSTLAN can be promoted from Proposed to Ready (see lang-rust.aps.md Ready Checklist and Risks table).

## Decision

**Rust-native enforcement in the `anvil-architecture` crate is authoritative for T3.**

- Layer/boundary validation, baseline management, definition parsing, and violation detection for Rust (and future anchors) live in `crates/anvil-architecture` and are driven by edges supplied by the kernel parser (`anvil_kernel::parser`).
- The TS `packages/anvil/core/src/architecture/` analyser becomes a legacy/compatibility surface: it continues to support existing TS/JS-only architecture documents and `anvil architecture` examples in docs for the short term, but is not extended for new language substrates. New Rust-specific rules, resolvers, and extractors are not back-ported to it.
- `crates/anvil-cli/src/commands/gate.rs` (and equivalent call sites) will be updated (under RSTLAN) to stop hard-filtering to JS/TS when invoking architecture validation; the pre-collected source list from `anvil_architecture::collect_source_files` (which already accepts .rs) + kernel-driven edge extraction will supply the edges.
- Rust mod/use shapes, crate-root resolution, workspace member mapping, and re-export handling are implemented in the kernel extractor (new `extract/rust.rs` or unified extractor) and fed to `anvil_architecture::ImportEdge` (or a small Rust-native equivalent if the TS-shaped `ImportEdge` proves too narrow).
- The T3 acceptance checklist (§1 parser, §5 layer/boundary enforcement, §8 `architecture-validate` inclusion) is satisfied for Rust only when the Rust-native path produces the same observable outcomes (layer assignments, new-vs-existing violation classification, baseline round-trips) as the prior TS path did for its languages.
- No TS "shim" (data bridge, FFI, or process call from TS architecture into Rust) is introduced for enforcement. The Rust crate is the implementation; TS surfaces that need architecture data after the cutover consume it via CLI/MCP/JSON or are archived.

This decision is recorded in the RSTLAN Ready Checklist as the §16.5 #5 gate.

## Rationale

Rust is the implementation language of the protection surface (daemon, kernel, intercept, witness, policy engine, new graph V2). Authoritative enforcement must live where the hot path and the durable model live, not behind a translation layer that would duplicate extraction logic, introduce serialisation boundaries, and slow the "save-time <2s cached" target.

### Reconciliation with prior ADRs and architecture principles (addressing review feedback)
This choice was cross-checked against the existing decision log before drafting:
- ADR-012 / 033 / 026 / 040 / 061 / 064: establish the Rust CLI/daemon/kernel as the primary runtime and hot path; TS surfaces are either parked (scanner/MCP IDE) or thin clients. "No TS shim" for enforcement is consistent — the authoritative path moves with the engine.
- ADR-014 (language allocation TS vs Rust): directional; this ADR supplies the concrete T3 enforcement location that was left open.
- Core principles (AGENTS + architecture overview): deterministic (kernel + tree-sitter already provide it), warnings-over-blocks (baseline + new-edges-only already wired in anvil-architecture), new-edges-only (the validator already classifies via baseline). The decision does not alter those invariants; it merely chooses the implementation substrate for Rust inputs.
- No conflict with hybrid monorepo reality (pnpm/nx + cargo workspaces) because layer assignment is glob-driven (language-agnostic) and edge supply is pluggable via `validate_with_edges`. Mixed TS/Rust repos continue to work; only Rust-origin edges gain full native resolution.
- Scope guard and zero-config posture preserved: no new user config required; Rust participation is "by presence of .rs + grammar" just as TS was.

Evidence grounding: re-score snapshot 2026-06-03 (dogfood demand from Anvil's own Rust crates), current state of `crates/anvil-architecture/validator.rs` (already accepts .rs and edges) and `gate.rs` (documented filter + comment calling out the exact gap this NBI closes). If future parity issues surface on TS surfaces, they will be new-edges and handled under the existing baseline contract (no silent drift).

The Rust architecture crate was deliberately stubbed with the full surface (baseline, validator, collect, yaml) in anticipation of this; LANGTS kernel prereqs (extractor trait + grammar_version) were built to enable exactly this hand-off for subsequent anchors.

A TS shim would contradict the "Rust for CPU-bound hot paths" (ADR-014 direction) and the post-ADR-033 retirement of TS scanner surfaces. It would also make Rust T3 acceptance depend on a parked TS codebase, violating the "no partials" rule in the language design ("Rust is at T3 but architecture enforcement is not wired yet" is not T3).

Alternatives table below captures the trade-off analysis.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Chosen: Rust-native (`anvil-architecture` + kernel edges) | Single source of truth for governance; reuses existing Rust graph/extractor work; consistent with daemon/CLI/ADR-061/ADR-064; no duplication of import resolution; globs + edges already partially wired; enables dogfood on Anvil's own Rust crates immediately. | Requires wiring Rust-specific extractor + resolver (mod/use/crate/super) as part of RSTLAN; TS architecture surfaces become legacy (docs/examples only). |
| TS shim (keep TS analyser authoritative; Rust supplies JSON edges or NAPI calls) | Minimal change to existing `packages/anvil/core` architecture code; TS packs can continue using in-process TS shapes. | Duplicates extraction logic (kernel already parses for checks/drift); adds serialisation/FFI boundary on hot save-time path; contradicts Rust-migration ADRs and parked TS scanner; Rust T3 would be "second class" (enforcement still TS); maintenance burden on two codebases for the same contract. |
| Hybrid (TS for TS files, Rust for .rs files, union at baseline) | Appears pragmatic. | Splits the baseline and violation model; "new edge" classification becomes cross-implementation; baseline format would need versioning or two stores; `anvil architecture validate` would have to orchestrate two engines; defeats the point of a unified `.anvil/architecture.json`. |

## Consequences

- **Positive:**
  - Rust T3 is first-class and dogfoodable on Anvil itself (kernel, crates, workspace layout claims).
  - Enforcement shares the same parser, cache, and graph substrate as checks, drift, and intercept — deterministic, one warm model.
  - Simplifies future pack substrate work (Tokio etc. will consume the same edges/layers).
  - Removes a deferred "month-scale decision" blocker; RSTLAN can now be promoted to Ready with concrete items.
  - TS architecture code can be left as-is or gradually thinned (no forced rewrite for Rust shapes).

- **Negative:**
  - RSTLAN work items must now include the Rust extractor wiring + architecture integration (previously assumed to be "just grammar + symbols"; the enforcement side is now explicit).
  - Existing TS-only architecture tutorials/docs that assume the TS analyser remain accurate only for JS/TS projects; docs will need "Rust uses the native path" notes (tracked under DOCSYNC/DOCGOV).
  - Short-term: `anvil architecture` surfaces for mixed-language repos will only enforce boundaries on files whose edges are supplied (JS/TS today; Rust once wired).

- **Risks:**
  - Rust mod resolution complexity (workspaces, `#[path]`, proc-macro-hidden mods) may surface FP or missed edges; mitigated by dogfooding on Anvil first + explicit exclusion notes in T3 checklist.
  - Baseline migration or format drift between legacy TS-generated and new Rust-generated baselines — use the existing `anvil drift migrate` pattern + schema versioning (OPSUP).
  - Some TS architecture surfaces (e.g. rego-generator used by policy?) may need to consume Rust-produced baselines or be re-expressed.

- **Mitigations:**
  - Phase the wiring: RSTLAN-001 grammar + basic symbols, RSTLAN-002 extraction of mod/use shapes, then explicit RSTLAN-005 layer/boundary integration using the already-present `validate_with_edges`.
  - Add `architecture` to the T3 checklist acceptance section for the anchor (already referenced).
  - Keep the TS analyser compiling and tested for its supported languages until a later deprecation wave (no immediate archive).
  - Record the decision in lang-rust.aps.md, index.aps.md §16.5 table, and the T3 checklist if a "location" note is warranted.

## References

- Related ADRs: [ADR-012](012-rust-cli-replacement.md), [ADR-026](026-rust-scanner-authoritative.md), [ADR-033](033-park-ide-mcp-retire-ts-scanner.md), [ADR-040](040-rust-policy-engine-regorus.md), [ADR-061](061-save-time-daemon-delta-validation.md), [ADR-064](064-intercept-graph-cache-crate-boundary.md), [ADR-014](014-language-allocation-tree-ts-vs-rust.md) (directional)
- APS modules: [lang-rust](../archive/modules/lang-rust.aps.md) (RSTLAN), [lang-ts-audit](../archive/modules/lang-ts-audit.aps.md) (LANGTS), [multilayer-protection-v2](../modules/multilayer-protection-v2.aps.md) (context for enforcement surfaces)
- Specs: [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md) §8.1, §16.5 #5 (C-019), T3 checklist
- Code: `crates/anvil-architecture/src/validator.rs` (collect + edges), `crates/anvil-cli/src/commands/gate.rs` (the `extract_import_edges` function, `include_extensions` list and `if !include_extensions.contains(&ext)` guard in the JS/TS filter), `packages/anvil/core/src/architecture/`
- Process: NBI row in `plans/index.aps.md`; anchor re-scoring snapshot 2026-06-03
