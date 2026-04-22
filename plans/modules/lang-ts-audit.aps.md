<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TypeScript Audit & T3 Calibration (Track 1 anchor zero)

| ID     | Owner | Status |
| ------ | ----- | ------ |
| LANGTS | —     | Draft  |

## Purpose

Audit TypeScript's current tier and produce the **T3 acceptance checklist**
that Rust and Python anchors must pass. TS is the language Anvil already
partially supports (`crates/anvil-kernel/src/parser/languages.rs`); without an
explicit calibration pass, "T3" is a made-up label. This module is the literal
first work item in the language-and-coverage plan set per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§7.3, §8.1, §17.3.

The audit also folds in **Zod-creep rules** (`z.any()`, `z.unknown()`,
`.passthrough()`) into the TS T2 anti-pattern catalogue alongside existing
rules for `any`, `as any`, `@ts-ignore`. Zod is cross-cutting infrastructure
used by every TS framework — language-level concern, not pack concern.

This module is the single point of failure flagged by council finding C-007 —
five Track 4 packs gate on TS → T3. Audit must complete before any pack
module starts execution.

## In Scope

- Enumerate current TS capabilities against the nine T3 dimensions
  (grammar, symbol/import extraction, anti-pattern catalogue, suppression,
  entry-point detection, layer/boundary enforcement, policy hook integration,
  drift baseline, `architecture-validate` inclusion) per spec §7.3.
- Identify and close TS-specific gaps surfaced during the audit.
- Publish a **T3 acceptance checklist** as a checked-in artefact (location
  TBD — likely `docs/architecture/anvil-t3-acceptance.md`).
- Add Zod-creep rules to TS T2 catalogue.
- Optional: split into a sub-module for the kernel prerequisite work
  identified by council §16.5 #3 (extractor refactor, grammar version in
  cache key, parser thread-safety, panic removal, grammar maturity audit) —
  this becomes Track 1 item 0.5 if the work is large enough to justify a
  module boundary.

## Out of Scope

- Any other anchor language (Rust, Python — separate modules).
- Pack work (substrate-gated on this module's completion).
- Tooling churn unrelated to T3 calibration.
- Type checker replacement (Anvil is governance, not typing).

## Interfaces

**Depends on:**

- Existing kernel parser (`crates/anvil-kernel/src/parser/`).
- Existing architecture analysis (`core/src/architecture/`).
- Existing OPA policy pipeline.
- Existing drift-baseline mechanism.

**Exposes:**

- T3 acceptance checklist (load-bearing for Track 1 items 1 and 2 plus all
  Track 4 packs).
- Updated TS extraction layer if gaps identified.
- Updated TS T2 anti-pattern catalogue including Zod-creep rules.
- (Possibly) extractor trait refactor underpinning subsequent anchors.

## Prerequisites

None — this is anchor item zero.

## Ready Checklist

Change status to **Ready** when:

- [ ] Audit owner named (single accountable owner for the T3 checklist).
- [ ] Re-scoring gate run per
      [docs/guides/anchor-rescoring-process.md](../../docs/guides/anchor-rescoring-process.md);
      session owner named for this invocation.
- [ ] Decision recorded on whether kernel prerequisite work (council §16.5 #3)
      is in-scope here or split into LANGTS-prereq submodule.
- [ ] Decision recorded on T3 checklist artefact location.

## Tasks

Tasks will be defined when this module moves to Ready. Anticipated shape (each
to be authored as a proper task with Intent + Validation when promoted):

- LANGTS-001: Enumerate current TS capability state across the seven T3
  dimensions.
- LANGTS-002: Close identified TS gaps (extraction completeness, layer
  enforcement reach, policy hook reachability, drift baseline default).
- LANGTS-003: Publish T3 acceptance checklist artefact.
- LANGTS-004: Add Zod-creep rules (`z.any()`, `z.unknown()`,
  `.passthrough()`) to TS T2 anti-pattern catalogue.
- LANGTS-005: Kernel prerequisite work — extractor trait, grammar version in
  AST cache key, parser thread-safety strategy, panic removal in
  `Parser::get_parser()`, grammar maturity audit (or split into
  LANGTS-prereq-* if scope justifies).

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Audit reveals months of TS gaps; pack ROI argument collapses (council C-007) | High | Re-score whole spec before committing to Track 4; surface gap depth inside this module before opening pack modules |
| Kernel prerequisite work entangles anchor zero indefinitely | Medium | Split into LANGTS-prereq submodule the moment scope grows beyond a sprint |
| T3 checklist becomes a moving target as Rust/Python uncover edge cases | Medium | Version the checklist; treat changes as ADR-level decisions |

## Open Questions

- [ ] Is LANGTS a single module or LANGTS + LANGTS-prereq? Decide before
      moving to Ready.
- [ ] Where does the T3 acceptance checklist live in the docs tree?
- [ ] Should the Zod-creep rules ship in a separate task, or fold into
      LANGTS-002 gap-close?
