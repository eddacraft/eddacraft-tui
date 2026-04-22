<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Markdown Governance (Track 5)

| ID    | Owner | Status |
| ----- | ----- | ------ |
| MDGOV | —     | Draft  |

## Purpose

Per [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.4, §8.5. Markdown is its own track because it fits none of the other axes
— not a programming language (no symbol graph), not a typical governance
surface (pattern catalogues alone miss the point), not a pack (no substrate).
Markdown in Anvil's world is **governance artefacts written in prose** —
APS plans, decision records, agent capability declarations, public
documentation. 762 markdown files in Anvil's own repo, ~173k LoC, almost
all load-bearing.

Initial target: **M1 (Structural)** — APS wellformedness + cross-reference
integrity. M2 (claim hygiene) and M3 (capability-aware) queue for later.

Phase 2 deliverable (spec §9 step 5; can slot earlier if bandwidth allows
because it has no dependencies on other tracks).

**Crate location**: per council finding C-017, this module does **NOT** live
in the Rust kernel. Decision recorded in
[ADR-028](../decisions/028-markdown-governance-crate.md) — standalone Rust
crate `crates/anvil-markdown-governance/` using `pulldown-cmark`.

## In Scope

**M1 (this module's target):**

- APS plan wellformedness: `plans/modules/*.aps.md` conform to the APS
  schema. Missing headers, broken status transitions, orphaned work-item
  IDs, duplicated IDs, cross-module reference drift. Effectively the
  `aps-planning` skill logic promoted to a check.
- Cross-reference integrity: markdown `[link](path)` references resolve
  to existing files. `plans/index.aps.md` references modules that exist
  (or are explicitly archived).
- Decision record hygiene: `plans/decisions/NNN-*.md` numbered
  contiguously, dated, statused.
- Markdown-fenced secrets pickup: existing secret scanner already
  covers this — explicitly hand off, do not duplicate.

**Acceptance for M1** (per spec §8.5 and council C-016 softening):
"All findings reviewed and fixed-or-suppressed" rather than "clean run
required". §3.2 of the spec itself notes the stale references this design
is replacing — the bar must reflect that reality.

## Out of Scope (M1)

- Stale-claim detection in public docs (M2).
- Agent capability-manifest integration (M3, depends on AGOV-007).
- Grammar, spelling, style — that is editorial, not governance.
- Markdown rendering correctness.
- Markdown-as-source / literate programming.
- Natural-language understanding of prose content.

## Interfaces

**Depends on:**

- Existing APS schema (`plans/aps-rules.md`).
- Existing secret scanner (hand-off only).
- [ADR-028](../decisions/028-markdown-governance-crate.md) — crate location
  (standalone Rust crate, not kernel).
- [ADR-029](../decisions/029-suppression-parser-authority.md) — Rust
  suppression parser is authoritative; this crate consumes it via
  `crates/anvil-checks`.
- [OPSUP](./operational-supplement.aps.md) check registry — this crate
  registers its checks through OPSUP like any other source.

**Exposes:**

- APS wellformedness checker.
- Cross-reference resolver.
- Decision-record-hygiene checker.

## Prerequisites

- [ADR-028](../decisions/028-markdown-governance-crate.md) advanced from
  Proposed → Accepted.
- [ADR-029](../decisions/029-suppression-parser-authority.md) Accepted.
- [OPSUP](./operational-supplement.aps.md) check-registry slice landed (or
  agreed to land before MDGOV's first task).
- Acceptance bar wording agreed (council C-016).

## Ready Checklist

Change status to **Ready** when:

- [ ] ADR-028 Accepted.
- [ ] ADR-029 Accepted.
- [ ] OPSUP check-registry slice landed.
- [ ] Acceptance bar wording agreed and aligned with the existing
      cross-reference rot in `plans/`.
- [ ] Anvil's own `plans/` directory inventoried for known stale
      references (this is the baseline drift event).
- [ ] Owner named.

## Tasks

Tasks will be defined when this module moves to Ready. Anticipated:

- MDGOV-001: Land `crates/anvil-markdown-governance/` skeleton per
  [ADR-028](../decisions/028-markdown-governance-crate.md); register
  through OPSUP.
- MDGOV-002: APS wellformedness checks (schema, status transitions,
  ID uniqueness).
- MDGOV-003: Cross-reference integrity check (markdown links resolve).
- MDGOV-004: Decision-record-hygiene check (`NNN-*.md` numbering).
- MDGOV-005: Baseline drift event for `plans/` (record current
  cross-reference state as the starting point).
- MDGOV-006: Validation against Anvil's own `plans/` and `docs/`.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Tries to be a documentation linter (council C-016) | High | Strict scope: M1 = wellformedness + cross-ref only; no prose-quality checks |
| Lives in the Rust kernel by accident (council C-017) | High | ADR up front; reject any task that puts markdown logic in `crates/anvil-kernel` |
| Pre-existing cross-reference rot blocks first-run acceptance (council C-016) | High | Acceptance bar is "reviewed and fixed-or-suppressed", not "clean run" |
| M1 scope creeps into M2 stale-claim detection | Medium | M2 is explicitly out of scope; defer demand to a follow-up module |

## Open Questions

- [ ] Crate location: standalone Rust crate, TS layer, or new tooling
      package?
- [ ] Should APS wellformedness rules be derived from a single schema
      definition shared with the `aps-planning` skill?
- [ ] How are explicitly-archived modules (`./archive/modules/...`)
      distinguished from broken references?
