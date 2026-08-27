# Intent Conformance

| ID   | Owner  | Status   | Progress |
| ---- | ------ | -------- | -------- |
| CONF | @aneki | Proposed | 1/9      |

**Last reviewed:** 2026-08-27 — CONF-001 Merged via
[#4174](https://github.com/eddacraft/anvil-001/pull/4174).
[ADR-134](../decisions/134-intent-conformance-gating.md) is accepted and pins
the deterministic Git, evidence-binding, and fail-honest outcome contract. The
existing GV2 per-file channel is **not** implementation clearance until
revision/schema/generation binding is proved. Tier 0 does not wait on the full
ILGOV rescope. The module remains **Proposed** until CONF-002..004 are
separately promoted with implementation files and Rust validation commands.

> **Origin (2026-06-11):** Product direction set during the graphify gap
> analysis: plan gates return as a conformance lint — "did the agent build
> what you were planning and what it said it did." This was anvil's original
> use case (see the ILGOV audit note: "all Anvil was going to be a system for
> proving the plan was followed"). The scope guard's "knowledge-graph doc
> search → Out" ruling (DocGraph assessment, 2026-05-31) does **not** apply
> here: this surface is detection + evidence at gate/closeout time, not
> retrieval — the lane the scope guard marks **In**. ADR-134 records that
> decision formally.
>
> **Naming:** deliberately "conformance", not "drift" —
> [ADR-052](../decisions/052-automated-drift-snapshots.md) owns code-edge
> drift and the DocGraph assessment §6 warns against overloading the term.

## Purpose

Check that a change matches its declared intent — what was planned, what was
asked, and what the author (human or agent) claimed was done — and attach the
result as gate/closeout evidence. Intent sources are tiered so the check is
generically useful with zero planning format and gets richer when one exists:

| Tier | Intent source                                        | Availability      |
| ---- | ---------------------------------------------------- | ----------------- |
| 0    | Commit `type(scope)`, PR-body claims                 | Universal         |
| 1    | Session intent events (Kindling, via ILGOV)          | Any wired agent   |
| 2    | Plan documents via adapters (APS first; OpenSpec, BMAD, SpecKit, issues) | Opt-in |

Conventional Commit extraction/evaluation is the first Tier-0 implementation
slice (CONF-003/004). Deterministic PR-body patterns remain Tier 0 but land
later under CONF-005.

All tiers normalise into one canonical conformance contract; policy predicates
("touched outside declared scope", "claimed X, delta shows Y") operate on the
contract, never on a source format. APS is the richest producer, not a
requirement — planless-first (ADR-001) is preserved because Tier 0 delivers
value with no plan at all.

## In Scope

- Canonical conformance contract: declared scope, claimed changes, acceptance
  assertions, source provenance with evidence grading
  ([ADR-062](../decisions/062-policy-evidence-drift-as-evidence.md))
- Tier-0 claim extraction staged as Conventional Commits first (CONF-003/004)
  and deterministic PR-body patterns later (CONF-005)
- Conformance evaluation over the ADR-134 per-commit Git coverage contract:
  Git evidence for closed path/file-class claims; revision-bound GV2 evidence
  for graph-semantic claims
- Correlation join: plan/work-item ID ↔ commit trailer ↔ PR ↔ delta ↔ capsule
  ([ADR-072](../decisions/072-git-native-governance-substrate.md),
  [ADR-074](../decisions/074-review-capsule-v0-format.md))
- Gate or closeout evaluation/attachment timing; warnings-first and non-zero
  only under explicit ADR-002 opt-in, baselined per ADR-003
- Tier-2 adapter artifact contract so the TS adapters layer
  (`packages/adapters/`) can emit the canonical contract as a build-time JSON
  artifact (ADR-049 cross-language style) without a Rust port of the adapters

## Out of Scope

- Knowledge-graph search, retrieval, indexing, or FTS over docs/plans (scope
  guard: Out; unchanged by this module)
- Session intent capture and ledger integrity — owned by ILGOV
- Plan-format parsing itself — owned by the adapters layer (OPENSPEC, BMAD4)
  and the external `anvil-plan-spec` toolchain
- GV2 schemas, deltas, identities — owned by GV2
- LLM-inferred claim extraction; all extraction here is deterministic
- Blocking by default — exit-0 advisory posture until opt-in enforce

## Interfaces

**Depends on:**

- GV2 — per-file `GraphDelta`, `SymbolIdentity`, and file/symbol deltas;
  graph-semantic use requires repository/worktree, evaluated revision/blob,
  schema, generation, and run binding
- ILGOV — Tier-1 intent records and the `IntentLedgerRecord` canonical schema
  (ILGOV rescope item 1); CONF-002 must not fork that schema
- GITGOV — later capsule evidence attachment under ADR-134's minimisation and
  privacy-review gate
- `packages/adapters/` — Tier-2 plan-format normalisation (TS, consumed via
  artifact, never linked)
- `anvil-kernel-types` — contract type home (SCHEMA precedent)

**Exposes:**

- Conformance contract type (Rust) + JSON artifact schema for Tier-2 producers
- Conformance findings consumable by gate/closeout; later minimised capsule
  evidence owned by CONF-006/009
- Policy predicates over the contract for L4/policy-engine composition

## Ready Checklist

Change status to **Ready** when:

- [x] CONF-001 ADR accepted as
      [ADR-134](../decisions/134-intent-conformance-gating.md) (product lane,
      tier model, retrieval distinction, naming)
- [x] ADR-134 pins per-commit Git extraction, claim-appropriate evidence,
      per-member dispositions, monotonic outcome/evidence-strength aggregation,
      explicit `path:` scope authority, base-tree mappings, and capsule privacy
- [x] Tier 0 sequenced ahead of ILGOV rescope item 1; CONF-002 keeps a
      co-design seam and must not fork the future Rust `IntentLedgerRecord`
- [ ] CONF-002..004 implement and prove exact Git extraction plus
      repository/worktree, revision/blob, schema, generation, and run binding;
      then promote them separately with implementation files and Rust validation
      commands
- [x] CONF-005..009 remain Proposed for later waves

## Work Items

### CONF-001: Product-decision ADR for conformance gating

- **Status:** Merged 2026-08-27 via PR #4174
- **Intent:** Record the decision that intent/plan-conformance gating is
  in-lane, with the tier model and the retrieval-surface distinction.
- **Expected Outcome:** Accepted ADR in `plans/decisions/`; DECISION-LOG row;
  scope-guard borderline table cites it so the DocGraph "Out" ruling is not
  re-applied to this surface; module/programme/index truth records the cleared
  Tier-0 decision without promoting implementation work.
- **Files:** `plans/decisions/134-intent-conformance-gating.md`,
  `plans/decisions/DECISION-LOG.md`, `docs/vision/anvil-scope-guard.md`,
  this module, `plans/specs/2026-07-28-graph-trust-surfaces.md`, and
  `plans/index.aps.md`
- **Evidence:** ADR-134 pins base/head and per-commit Git extraction,
  first-parent merge footprints, raw status/rename/mode/type/gitlink and path
  handling, fixed extraction and rename budgets with fail-honest overflow,
  replacement-disabled sanitised Git execution, option-safe revision
  resolution, claim-appropriate evidence binding, per-member dispositions,
  monotonic outcomes, explicit `path:` authority, base-tree mapping
  authority, anti-self-authorisation, Conventional-Commit-first staging,
  ADR-002 posture, capsule minimisation/privacy ownership, and the Wave
  boundary. Implementation evidence remains with CONF-002..004; CONF-005..009
  remain Proposed.
- **Validation:** `pnpm adr:check`, `pnpm format:check`,
  `pnpm docs:check`, `pnpm aps:active-lint`, `pnpm aps:index:check`
- **Dependencies:** —
- **Confidence:** high

### CONF-002: Canonical conformance contract

- **Status:** Proposed
- **Intent:** One Rust contract all tiers normalise into: declared scope,
  claimed changes, acceptance assertions, source provenance + evidence grade.
- **Expected Outcome:** Contract type in `anvil-kernel-types` with serde JSON
  schema for external producers; binds repository/worktree, evaluation run,
  revision/blob and graph schema/generation; extends (not forks) ILGOV's record
  schema.
- **Validation:** `cargo test -p anvil-kernel-types`
- **Dependencies:** CONF-001
- **Confidence:** medium

### CONF-003: Tier-0 claim extraction — conventional commits

- **Status:** Proposed
- **Intent:** Deterministically parse commit `type(scope)` and trailers into
  conformance contract claims as the first executable Tier-0 source.
- **Expected Outcome:** Each selected commit is bound to its own exact raw Git
  footprint and typed scope/kind claim under ADR-134's bounded,
  replacement-disabled Git contract; malformed, unknown, inapplicable, or
  option-shaped revision inputs yield reason-coded not-evaluated, never
  conformant. Tests cover option-shaped inputs, active replacement refs,
  legacy graft state, ambient Git overrides, and every budget reason.
- **Validation:** `cargo test -p anvil-checks`
- **Dependencies:** CONF-002
- **Confidence:** high

### CONF-004: Tier-0 conformance check — claims vs delta

- **Status:** Proposed
- **Intent:** Evaluate Tier-0 claims against the change delta and emit
  advisory findings.
- **Expected Outcome:** Closed path/file-class claims evaluate from Git;
  graph-semantic claims require bound GV2 evidence; every coverage member has
  a disposition; direct paths require `path:<prefix>`; mappings come only from
  versioned base-tree authority and mapping changes are `policy-change`;
  proven violations remain non-conformant under partial evidence; warnings exit
  0 by default; baselined new-edges-only.
- **Validation:** `cargo test -p anvil-checks` + dogfood on this repo's history
- **Dependencies:** CONF-003
- **Confidence:** high

### CONF-005: Tier-0 claim extraction — PR bodies

- **Status:** Proposed
- **Intent:** Extract deterministic claim patterns ("test-only", "no behaviour
  change", "refactor only") from PR descriptions as weak-graded claims.
- **Expected Outcome:** PR-body claims enter the contract with low evidence
  grade under ADR-134's source-reference+digest privacy rule; unrecognised prose
  yields no claim. This is later Tier 0 and is not part of CONF-003/004.
- **Validation:** `cargo test -p anvil-checks`
- **Dependencies:** CONF-002
- **Confidence:** medium

### CONF-006: Correlation join across the git substrate

- **Status:** Proposed
- **Intent:** Join work-item IDs to their realised changes: plan/item ID ↔
  commit trailer ↔ PR ↔ `GraphDelta` ↔ capsule.
- **Expected Outcome:** Given a work-item ID, the join returns its commits,
  PRs, and touched files/symbols deterministically from local git state. Any
  later capsule attachment carries normalised claims and source reference +
  digest, never raw bodies, and waits for privacy review.
- **Validation:** `cargo test -p anvil-checks` + join replay against a known
  merged item in this repo
- **Dependencies:** CONF-002, GV2 delta surface
- **Confidence:** medium

### CONF-007: Closeout conformance check

- **Status:** Proposed
- **Intent:** Run or attach conformance evaluation at closeout as a timing
  surface, without creating a separate enforcement class.
- **Expected Outcome:** Closeout reports conformance findings; non-zero exit is
  available only through explicit ADR-002 enforcement opt-in; attachment
  preserves outcome/evidence separation and the privacy gate.
- **Validation:** dogfood closeout run on this repo
- **Dependencies:** CONF-004, CONF-006
- **Confidence:** medium

### CONF-008: Tier-2 plan-adapter artifact contract

- **Status:** Proposed
- **Intent:** Let the TS adapters layer emit the canonical contract as a JSON
  artifact the Rust gate consumes; APS adapter first.
- **Expected Outcome:** An APS module/work item round-trips to a conformance
  contract artifact; contract versioned; Rust side validates schema and never
  links TS.
- **Validation:** `pnpm --filter @eddacraft/anvil-adapters test` +
  `cargo test -p anvil-checks`
- **Dependencies:** CONF-002
- **Confidence:** low

### CONF-009: Intent-source evidence grading in verdicts

- **Status:** Proposed
- **Intent:** Carry source evidence grades through to findings so verdicts
  state what strength of intent they were checked against, and own the closed
  minimised capsule allowlist with CONF-006.
- **Expected Outcome:** Findings distinguish "conformant against plan
  acceptance criteria" from "conformant against commit-type claim only";
  capsule evidence excludes raw bodies, absolute paths, `GraphDelta.errors`,
  and graph baseline sets until a privacy review approves schema extension.
- **Validation:** `cargo test -p anvil-checks`
- **Dependencies:** CONF-004, CONF-008
- **Confidence:** medium
