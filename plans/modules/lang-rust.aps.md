<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Rust Language Anchor (Track 1)

| ID     | Owner | Status |
| ------ | ----- | ------ |
| RSTLAN | —     | Proposed |

**Last reviewed:** 2026-05-14

> **Priority update (2026-05-14):** RSTLAN is now part of the first Language &
> Coverage target set because Anvil's primary implementation surface is Rust and
> founder priority is to get Rust coverage sorted before later packs expand. The
> module remains `Proposed` until the readiness gates below close; this priority
> change does not authorise implementation before LANGTS/kernel prerequisites and
> the Rust T3 architecture-enforcement ADR are ready.

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
- Layer/boundary enforcement reaching Rust crates and modules — this
  requires the council §16.5 #5 decision (TS shim vs Rust-native enforcement
  location) to be made and recorded as an ADR before the work item lands.
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

- [`lang-ts-audit`](./lang-ts-audit.aps.md) — T3 acceptance checklist; no
  start until LANGTS publishes it.
- Kernel prerequisite work from `lang-ts-audit` (extractor trait, grammar
  version in cache key, parser thread-safety, panic removal).
- Existing `crates/anvil-kernel/src/parser/`, architecture analysis,
  policy pipeline, drift baseline, suppression parser.

**Exposes:**

- Rust at T3 — first-phase dogfood coverage for Anvil's own crates and
  substrate-tier prerequisite for `pack-tokio` and (Phase 3) `pack-axum`.
- Rust portion of T3 acceptance evidence — calibration data for Python anchor.

## Prerequisites

- `lang-ts-audit` complete (T3 acceptance checklist exists).
- ADR recorded for council §16.5 #5 (T3 architecture enforcement location).
- Re-scoring gate run per
  [docs/guides/anchor-rescoring-process.md](../../docs/guides/anchor-rescoring-process.md)
  before this module starts.

## Ready Checklist

Change status to **Ready** when:

- [ ] LANGTS complete and T3 acceptance checklist published.
- [ ] ADR for Rust T3 architecture enforcement location recorded.
- [ ] Re-scoring gate snapshot recorded; Rust still anchor #2 after TS.
- [ ] Owner named for the anchor work.

## Work Items

Tasks will be defined when this module moves to Ready. Anticipated shape:

- RSTLAN-001: Tree-sitter-rust grammar wired through extractor trait.
- RSTLAN-002: Rust symbol/import extraction (mod/use/crate/super shapes).
- RSTLAN-003: Rust T2 anti-pattern catalogue including serde-hygiene rules.
- RSTLAN-004: Entry-point detection (`Cargo.toml`, `fn main`, workspace bins).
- RSTLAN-005: Layer/boundary enforcement reaches Rust per ADR.
- RSTLAN-006: Drift baseline default-on for `.rs`.
- RSTLAN-007: `architecture-validate` includes Rust crates/modules.
- RSTLAN-008: Validate against Anvil's own kernel — zero panics, full graph
  inclusion, FP rate < N% per council §16.5 #9.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| `use` paths need crate context | Medium | Read `Cargo.toml` for crate name mapping |
| `unsafe` false positives in FFI crates | Medium | Allowlist for FFI crates; severity: info |
| Macro-generated imports invisible | Low | Document limitation; revisit if proc-macro analysis arrives |
| Workspace vs single-crate layouts confuse extraction | Medium | Detect via `Cargo.toml` `[workspace]`; validate against Anvil's own monorepo first |
| Architecture-enforcement decision (council §16.5 #5) deferred indefinitely | High | ADR is a Ready prerequisite; module cannot move past Proposed without it |

## Open Questions

- [ ] Should `unsafe fn` and `unsafe {}` blocks be flagged separately?
- [ ] How are workspace-internal vs public crate boundaries declared?
- [ ] Should `#[allow(...)]` and `#[clippy::allow(...)]` be treated like
      `@anvil-ignore` for suppression accounting?
- [ ] Re-scoring owner — who runs the gate before each anchor commits?
