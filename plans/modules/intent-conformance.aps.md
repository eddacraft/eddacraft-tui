# Intent Conformance

| ID   | Owner  | Status   | Progress |
| ---- | ------ | -------- | -------- |
| CONF | @aneki | Proposed | 0/9      |

**Last reviewed:** 2026-06-11

> **Origin (2026-06-11):** Product direction set during the graphify gap
> analysis: plan gates return as a conformance lint — "did the agent build
> what you were planning and what it said it did." This was Anvil's original
> use case (see the ILGOV audit note: "all Anvil was going to be a system for
> proving the plan was followed"). The scope guard's "knowledge-graph doc
> search → Out" ruling (DocGraph assessment, 2026-05-31) does **not** apply
> here: this surface is detection + evidence at gate/closeout time, not
> retrieval — the lane the scope guard marks **In**. CONF-001 records that
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

All tiers normalise into one canonical conformance contract; policy predicates
("touched outside declared scope", "claimed X, delta shows Y") operate on the
contract, never on a source format. APS is the richest producer, not a
requirement — planless-first (ADR-001) is preserved because Tier 0 delivers
value with no plan at all.

## In Scope

- Canonical conformance contract: declared scope, claimed changes, acceptance
  assertions, source provenance with evidence grading
  ([ADR-062](../decisions/062-policy-evidence-drift-as-evidence.md))
- Tier-0 claim extraction from conventional commits and PR bodies
- Conformance evaluation against GV2 `GraphDelta` (file-level first;
  symbol-level as GV2 edge coverage grows)
- Correlation join: plan/work-item ID ↔ commit trailer ↔ PR ↔ delta ↔ capsule
  ([ADR-072](../decisions/072-git-native-governance-substrate.md),
  [ADR-074](../decisions/074-review-capsule-v0-format.md))
- Closeout-class conformance check (ADR-042 carve-out; warnings-first per
  ADR-002, baselined per ADR-003)
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

- GV2 — `GraphDelta`, `SymbolIdentity`, file/symbol deltas
- ILGOV — Tier-1 intent records and the `IntentLedgerRecord` canonical schema
  (ILGOV rescope item 2); CONF-002 must not fork that schema
- GITGOV — capsule evidence attachment (ADR-074)
- `packages/adapters/` — Tier-2 plan-format normalisation (TS, consumed via
  artifact, never linked)
- `anvil-kernel-types` — contract type home (SCHEMA precedent)

**Exposes:**

- Conformance contract type (Rust) + JSON artifact schema for Tier-2 producers
- Conformance findings consumable by gate/closeout and capsule evidence
- Policy predicates over the contract for L4/policy-engine composition

## Ready Checklist

Change status to **Ready** when:

- [ ] CONF-001 ADR accepted (product decision: conformance gating in-lane;
      tier model; retrieval distinction; naming)
- [ ] GV2 delta surface confirmed sufficient for Tier-0 file-level checks
- [ ] ILGOV rescope item 2 (Rust `IntentLedgerRecord`) landed or co-designed
      so CONF-002 extends rather than forks it

## Work Items

### CONF-001: Product-decision ADR for conformance gating

- **Status:** Proposed
- **Intent:** Record the decision that intent/plan-conformance gating is
  in-lane, with the tier model and the retrieval-surface distinction.
- **Expected Outcome:** Accepted ADR in `plans/decisions/`; DECISION-LOG row;
  scope-guard borderline table cites it so the DocGraph "Out" ruling is not
  re-applied to this surface.
- **Validation:** `pnpm adr:check`
- **Dependencies:** —
- **Confidence:** high

### CONF-002: Canonical conformance contract

- **Status:** Proposed
- **Intent:** One Rust contract all tiers normalise into: declared scope,
  claimed changes, acceptance assertions, source provenance + evidence grade.
- **Expected Outcome:** Contract type in `anvil-kernel-types` with serde JSON
  schema for external producers; extends (not forks) ILGOV's record schema.
- **Validation:** `cargo test -p anvil-kernel-types`
- **Dependencies:** CONF-001
- **Confidence:** medium

### CONF-003: Tier-0 claim extraction — conventional commits

- **Status:** Proposed
- **Intent:** Deterministically parse commit `type(scope)` and trailers into
  conformance contract claims.
- **Expected Outcome:** Commit messages yield typed scope/kind claims;
  malformed messages degrade to "no claim", never error.
- **Validation:** `cargo test -p anvil-checks`
- **Dependencies:** CONF-002
- **Confidence:** high

### CONF-004: Tier-0 conformance check — claims vs delta

- **Status:** Proposed
- **Intent:** Evaluate Tier-0 claims against the change delta and emit
  advisory findings.
- **Expected Outcome:** `docs:`-typed commit touching code, or scoped commit
  touching outside its scope, produces a warning finding; exit 0 by default;
  baselined new-edges-only.
- **Validation:** `cargo test -p anvil-checks` + dogfood on this repo's history
- **Dependencies:** CONF-003
- **Confidence:** high

### CONF-005: Tier-0 claim extraction — PR bodies

- **Status:** Proposed
- **Intent:** Extract deterministic claim patterns ("test-only", "no behavior
  change", "refactor only") from PR descriptions as weak-graded claims.
- **Expected Outcome:** PR-body claims enter the contract with low evidence
  grade; unrecognised prose yields no claim.
- **Validation:** `cargo test -p anvil-checks`
- **Dependencies:** CONF-002
- **Confidence:** medium

### CONF-006: Correlation join across the git substrate

- **Status:** Proposed
- **Intent:** Join work-item IDs to their realised changes: plan/item ID ↔
  commit trailer ↔ PR ↔ `GraphDelta` ↔ capsule.
- **Expected Outcome:** Given a work-item ID, the join returns its commits,
  PRs, and touched files/symbols deterministically from local git state.
- **Validation:** `cargo test -p anvil-checks` + join replay against a known
  merged item in this repo
- **Dependencies:** CONF-002, GV2 delta surface
- **Confidence:** medium

### CONF-007: Closeout conformance check

- **Status:** Proposed
- **Intent:** Run conformance evaluation at closeout as an ADR-042-class
  integrity check with opt-in enforcement.
- **Expected Outcome:** Closeout reports conformance findings; non-zero exit
  only under explicit opt-in; findings attach to capsule evidence.
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
  state what strength of intent they were checked against.
- **Expected Outcome:** Findings distinguish "conformant against plan
  acceptance criteria" from "conformant against commit-type claim only".
- **Validation:** `cargo test -p anvil-checks`
- **Dependencies:** CONF-004, CONF-008
- **Confidence:** medium
