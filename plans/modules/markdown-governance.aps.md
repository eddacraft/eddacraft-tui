<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Markdown Governance (Track 5)

| ID    | Owner | Status |
| ----- | ----- | ------ |
| MDGOV | —     | Draft  |

**Last reviewed:** 2026-04-26

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

- [x] ADR-028 Accepted — `Accepted (2026-04-26)`.
- [x] ADR-029 Accepted — `Accepted (2026-04-26)`.
- [x] OPSUP check-registry slice landed — OPSUP-001 Done (stable check-ID
      registry on `check_catalog.rs`); MDGOV-001 registers through it.
- [x] Acceptance bar wording agreed and aligned with the existing
      cross-reference rot in `plans/` — "all findings reviewed and
      fixed-or-suppressed" per spec §8.5 + council C-016; pinned in MDGOV-006.
- [ ] Anvil's own `plans/` directory inventoried for known stale
      references (this is the baseline drift event) — owned by MDGOV-005;
      done as the first execution step, not a pre-Ready gate.
- [ ] Owner named.

Remaining gate to flip Ready: name an owner. All ADR/OPSUP/acceptance-bar
prerequisites are satisfied; the `plans/` inventory is MDGOV-005's deliverable,
not a precondition.

## Work Items

M1 target: APS wellformedness + cross-reference integrity + decision-record
hygiene, delivered as a standalone Rust crate per
[ADR-028](../decisions/028-markdown-governance-crate.md). Acceptance is "all
findings reviewed and fixed-or-suppressed", not "clean run".

### MDGOV-001: Land `crates/anvil-markdown-governance/` skeleton

- **Status:** Ready
- **Intent:** Stand up the standalone markdown-governance crate and register
  its checks through the OPSUP check-ID registry.
- **Expected Outcome:** A new `crates/anvil-markdown-governance/` crate exists
  using `pulldown-cmark`, exposing a check entry point that registers `ANV-MD-*`
  IDs through the OPSUP-001 registry (consumed via `crates/anvil-checks`). No
  markdown logic lands in `crates/anvil-kernel` (council C-017). The crate
  compiles and has at least one smoke test parsing a fixture markdown file.
- **Scopes:** new crate skeleton; OPSUP registry registration; suppression
  parser consumption via `crates/anvil-checks`.
- **Non-scope:** any concrete check rule (MDGOV-002/-003/-004 own those); any
  edit to `crates/anvil-kernel`.
- **Files:**
  - `crates/anvil-markdown-governance/` (NEW crate)
  - `crates/anvil-checks/src/` (registry + suppression-parser hand-off seam)
- **Validation:**
  - `cargo test -p eddacraft-anvil-markdown-governance` (skeleton smoke test)
  - New test asserts the crate registers its check IDs through the OPSUP registry
- **Dependencies:** OPSUP-001 (Done), ADR-028, ADR-029
- **Confidence:** high

### MDGOV-002: APS wellformedness checks

- **Status:** Ready
- **Intent:** Promote the `aps-planning` skill's structural rules into a
  deterministic check over `plans/**/*.aps.md`.
- **Expected Outcome:** The check flags missing required headers, work-item IDs
  that do not match the `PREFIX-NNN` shape, duplicated work-item IDs within a
  module, and invalid status values per `plans/aps-rules.md`. Findings carry the
  file + line and an `ANV-MD-*` ID, and are suppressible via the standard
  `@anvil-ignore` syntax. Rules derive from a single source shared with the APS
  schema where practical (see Open Questions on schema sharing).
- **Scopes:** APS structural rules; finding emission through the crate.
- **Non-scope:** cross-reference resolution (MDGOV-003); prose-quality checks.
- **Files:**
  - `crates/anvil-markdown-governance/src/` (APS wellformedness rules)
- **Validation:**
  - `cargo test -p eddacraft-anvil-markdown-governance aps_wellformedness`
  - Fixtures: a well-formed module passes; modules with a missing header, a
    malformed ID, a duplicate ID, and an invalid status each flag exactly once
- **Dependencies:** MDGOV-001
- **Confidence:** high

### MDGOV-003: Cross-reference integrity check

- **Status:** Ready
- **Intent:** Verify markdown `[link](path)` references in governance docs
  resolve to existing files, distinguishing archived targets from broken ones.
- **Expected Outcome:** The check resolves relative links in `plans/**` and
  flags references whose target file does not exist. References into
  `./archive/modules/...` resolve normally (archived ≠ broken). The check
  reports the source file + line and is suppressible. This is the same class of
  signal the `docs:check` `links` surface provides today, scoped to governance
  markdown and emitted through the unified check pipeline.
- **Scopes:** relative-link resolution; archived-vs-broken disambiguation.
- **Non-scope:** external URL liveness; anchor-fragment resolution beyond file
  existence.
- **Files:**
  - `crates/anvil-markdown-governance/src/` (cross-reference resolver)
- **Validation:**
  - `cargo test -p eddacraft-anvil-markdown-governance cross_reference`
  - Fixtures: a resolving link passes; a broken link flags; an
    `./archive/modules/...` link resolves without a finding
- **Dependencies:** MDGOV-001
- **Confidence:** high

### MDGOV-004: Decision-record-hygiene check

- **Status:** Ready
- **Intent:** Check that `plans/decisions/NNN-*.md` records are numbered
  contiguously, dated, and carry a status.
- **Expected Outcome:** The check flags non-contiguous ADR numbering, an ADR
  missing a `## Status` section, and an ADR missing a date. Findings are
  per-file with an `ANV-MD-*` ID and are suppressible. Mirrors the existing
  `adr:check` invariant so the two do not disagree.
- **Scopes:** ADR numbering/date/status hygiene.
- **Non-scope:** ADR content quality; the DECISION-LOG index (owned by DOCGOV).
- **Files:**
  - `crates/anvil-markdown-governance/src/` (decision-record rules)
- **Validation:**
  - `cargo test -p eddacraft-anvil-markdown-governance decision_record`
  - Fixtures: a contiguous, dated, statused ADR set passes; a gap in numbering,
    a missing status, and a missing date each flag
- **Dependencies:** MDGOV-001
- **Confidence:** high

### MDGOV-005: Baseline drift event for `plans/`

- **Status:** Ready
- **Intent:** Record Anvil's own current cross-reference and wellformedness
  state as the starting baseline so M1 warns on new drift, not legacy rot.
- **Expected Outcome:** Running the MDGOV-002/-003/-004 checks against `plans/`
  produces a recorded baseline of current findings (the "drift baseline
  established" event per new-edges-only posture, D-003). Pre-existing findings
  are baselined; only new findings warn after this point. The baseline lives
  with the drift baseline mechanism (coordinate with OPSUP-003 schema
  versioning).
- **Scopes:** the first-run baseline capture for governance markdown.
- **Non-scope:** fixing the legacy findings (acceptance bar is reviewed +
  fixed-or-suppressed over time, not a clean run gate).
- **Files:**
  - `crates/anvil-markdown-governance/src/` (baseline integration)
  - drift baseline storage (per OPSUP-003)
- **Validation:**
  - `cargo test -p eddacraft-anvil-markdown-governance baseline`
  - The baseline run over `plans/` completes and records current findings
    without failing the gate
- **Dependencies:** MDGOV-002, MDGOV-003, MDGOV-004; coordinates with OPSUP-003
- **Confidence:** medium

### MDGOV-006: Validation against Anvil's own `plans/` and `docs/`

- **Status:** Ready
- **Intent:** Prove M1 on Anvil's own corpus and confirm the acceptance bar.
- **Expected Outcome:** The full M1 check set runs over `plans/` and `docs/`
  with every finding reviewed and either fixed or explicitly suppressed (the
  council C-016 acceptance bar), not a clean-run requirement. A short report
  records the finding count, how many were fixed, and how many were suppressed
  with reasons.
- **Scopes:** end-to-end validation run + acceptance report.
- **Non-scope:** M2 stale-claim detection; M3 capability-aware checks.
- **Files:**
  - `crates/anvil-markdown-governance/` (integration test over the real corpus)
  - a validation report under `plans/` or the crate's test fixtures
- **Validation:**
  - `cargo test -p eddacraft-anvil-markdown-governance --test corpus`
  - All findings reviewed and fixed-or-suppressed; report committed
- **Dependencies:** MDGOV-005
- **Confidence:** medium

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Tries to be a documentation linter (council C-016) | High | Strict scope: M1 = wellformedness + cross-ref only; no prose-quality checks |
| Lives in the Rust kernel by accident (council C-017) | High | ADR up front; reject any task that puts markdown logic in `crates/anvil-kernel` |
| Pre-existing cross-reference rot blocks first-run acceptance (council C-016) | High | Acceptance bar is "reviewed and fixed-or-suppressed", not "clean run" |
| M1 scope creeps into M2 stale-claim detection | Medium | M2 is explicitly out of scope; defer demand to a follow-up module |

## Open Questions

- [x] Crate location: standalone Rust crate, TS layer, or new tooling
      package? **Resolved by [ADR-028](../decisions/028-markdown-governance-crate.md)
      (Accepted 2026-04-26): standalone `crates/anvil-markdown-governance/`
      using `pulldown-cmark`.** MDGOV-001 lands it.
- [ ] Should APS wellformedness rules (MDGOV-002) be derived from a single
      schema definition shared with the `aps-planning` skill, or implemented
      independently against `plans/aps-rules.md`? Implementation detail, not a
      blocker — decide at MDGOV-002 start.
- [ ] How are explicitly-archived modules (`./archive/modules/...`)
      distinguished from broken references in MDGOV-003? Treated as resolvable
      targets (archived ≠ broken); confirm the resolver honours the archive
      path convention.
