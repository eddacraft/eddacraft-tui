<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Context Compiler

| ID   | Owner       | Status | Progress |
| ---- | ----------- | ------ | -------- |
| CCTX | @joshuaboys | Ready | 1/3      |

**Status:** **Ready** (2026-09-08). Owner authorised the first spike. Spec
§12/§13 is frozen enough to run (CCTX-001). Spec §9 Decision Brief
contract is frozen as an advisory eval-fixture contract (CCTX-002). Spec
§15 open decisions are **parked until after the spike** — not resolved,
not ADRs. Trust boundary remains binding: synthesis is advisory and
never allow / warn / block. Enforcement must keep working if Context
Compiler is unavailable.

CCTX-001 is **Merged 2026-09-08 via PR #4456**. CCTX-002 is **Done** on
this branch (Decision Brief contract frozen; Merged after land).
CCTX-003 remains Ready behind CCTX-001 and CCTX-002.

**Spec:**
[`plans/specs/2026-09-08-context-compiler.md`](../specs/2026-09-08-context-compiler.md).

**Affinity, not membership:** this work sits next to
[graph-answer-attestation](./graph-answer-attestation.aps.md) (GATT),
[change-evidence-graph](./change-evidence-graph.aps.md) (CEG), and the
[Graph Trust Surfaces](../index.aps.md#graph-trust-surfaces) programme in
intent — it wants graph-backed answers that agents can trust. It is **not** a
sixth track of that operator-approved shortlist. Its home is Graph Substrate.
It must not duplicate GATT's disclosure contract or CEG's revision-delta
predicates.

**Trust boundary:** synthesis is advisory context only. It is never authority
for an Anvil allow, warn, or block decision. Enforcement must keep working if
Context Compiler is unavailable, stale, or wrong.

**Last reviewed:** 2026-09-08 — CCTX-002 Done on this branch (Decision
Brief contract frozen; §15.6 still parked).

## Purpose

Give a coding agent the smallest evidence-backed understanding required for a
task, before it acts, while preserving an exact path back to canonical source
and keeping deterministic enforcement on a separate path.

Agents today reconstruct the same repository understanding by searching,
reading, and re-deriving constraints. GCTX already improves *selection*.
Context Compiler is meant to perform stable synthesis ahead of time, then
select task-specific knowledge in one bounded **Decision Brief**. Until that
value is measured, this module owns only the spec and an internal evaluation
spike.

## In Scope

- Ingest source, symbols, dependency relationships, tests, repository
  metadata, relevant documentation, ADRs, policy, and Git change
  relationships — using the graph and surfaces Anvil already has.
- Produce versioned, evidence-linked synthesis cards for stable concepts
  (responsibilities, invariants, workflows, ownership, known risks,
  architectural boundaries).
- Track evidence, confidence, source hashes, synthesis version, and
  invalidation dependencies for every card and claim.
- Select relevant cards and deterministic graph facts for a task.
- Produce one bounded Decision Brief, with exact-source drill-down and an
  explanation of why material was included.
- Incrementally invalidate and rebuild affected knowledge after source
  changes.
- Emit metrics for cost, freshness, retrieval quality, and task outcomes.
- For this Ready spike: freeze the first-spike evaluation protocol and the
  advisory Decision Brief contract, then compare baselines against brief
  variants internally. No product compiler.

## Out of Scope

- Acting as the authority for allow, warn, or block decisions.
- Replacing Anvil's source graph, policy engine, or deterministic checks.
- Replacing or re-implementing GCTX projections, GATT disclosures, or CEG
  revision-delta predicates.
- A general-purpose enterprise knowledge product.
- Automatic code changes.
- Whole-repository natural-language summaries.
- Cross-customer learning from private source.
- Fully autonomous conflict resolution between contradictory sources.
- Membership in the Graph Trust Surfaces five-track shortlist.
- A product compiler, crate, or feature flag while the first spike is
  incomplete. Ready authorises the eval protocol, not a compiler.

## Interfaces

**Depends on:**

- Graph Substrate — archived
  [GV2](../archive/modules/graph-v2-foundation.aps.md) and
  [GCTX](../archive/modules/graph-context-delivery.aps.md) contracts; the
  resident graph in `crates/anvil-graph-cache`; sealed egress DTOs in
  `crates/anvil-gctx-types`.
- [GCTX delivery contract](../../docs/architecture/graph-context-delivery-spec.md)
  and [AI context delivery](../../docs/guides/ai-context-delivery.md) — the
  advisory projection surface CCTX selects over, including the existing
  split from launch validation.
- [GATT](./graph-answer-attestation.aps.md) — affinity for in-band honesty
  about graph-answer limits. Evidence-format reuse is parked in spec §15.6
  (deferred to post-spike), not a dependency that authorises GATT work
  from here.
- [CEG](./change-evidence-graph.aps.md) — affinity for Git-backed change
  evidence. CCTX does not consume CEG and must not wait on it.
- Anvil's existing local-first data boundary
  ([Local data and security](../../docs/public/anvil/operations/security.md)).

**Exposes:**

- Decision Brief contract — **advisory only**. Never allow / warn / block.
  Never a substitute for `anvil_validate_write` or the policy / check engine.

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one work item defined
- [x] Owner confirms the spec is validated and the first spike is the right
      next grain (authorised 2026-09-08 via eddacraft dev coordinator /
      @joshuaboys)
- [x] Open decisions in spec §15 are resolved, parked, or given an ADR —
      **parked** 2026-09-08 as deferred-to-post-spike; no ADRs invented
- [x] Trust boundary remains explicit: synthesis is never enforcement
      authority

## Work Items

Draft spike only. Aligned with spec §12 (internal eval: baselines versus
brief variants). No implementation detail; none of these items authorise a
compiler, crate, or flag.

| ID       | Task                                              | Status | Depends on |
| -------- | ------------------------------------------------- | ------ | ---------- |
| CCTX-001 | Freeze the internal evaluation protocol           | Merged 2026-09-08 via PR #4456 | —          |
| CCTX-002 | Freeze the Decision Brief as an advisory contract | Done   | CCTX-001   |
| CCTX-003 | Compare baselines against brief variants          | Ready  | CCTX-001, CCTX-002 |

### CCTX-001: Freeze the internal evaluation protocol

- **Status:** Merged 2026-09-08 via PR #4456
- **Intent:** Make the first spike comparable before anyone spends synthesis
  cost.
- **Expected Outcome:** The spec's §12 protocol is frozen enough to run: one
  Anvil revision, current GCTX, selected docs/ADRs/tests/policy, twenty
  representative real tasks, four variants, six task classes, and the §13
  metrics and initial gates. Numeric thresholds remain unset until baselines
  exist.
- **Validation:** `pnpm docs:check` — validation passed 2026-09-08 (15/15
  surfaces). `pnpm aps:index:check` and `pnpm aps:drift` exit 0 (pre-existing
  DPO 2/6 vs 4/8 advisory only). `pnpm aps:active-lint` could not run (`aps`
  not on PATH in this environment).
- **Non-scope:** No product compiler. No GCTX, GATT, or CEG behaviour change.
  No EVALCI / policy-regression work.
- **Files:** `plans/specs/2026-09-08-context-compiler.md`,
  `plans/modules/context-compiler.aps.md`, `plans/index.aps.md`
- **Claim:** private GitHub issue #4455
- **PR:** Merged 2026-09-08 via
  [#4456](https://github.com/eddacraft/anvil-001/pull/4456)
  (`07db4dd636b20330c24206601ebb398632fc9ce3` on `main`; rebase-merge of
  `9b19e98a9`)
- **Confidence:** medium

### CCTX-002: Freeze the Decision Brief as an advisory contract

- **Status:** Done
- **Intent:** Fix what one task-shaped brief must contain, and what it must
  never be allowed to mean.
- **Expected Outcome:** The Decision Brief contract in spec §9 is the
  declared shape: required fields, claim-level evidence, freshness, and the
  fact / interpretation / uncertainty / recommendation split. The contract
  states that a brief is advisory and is never allow / warn / block.
  Frozen enough for CCTX-003 to author V3/V4 fixtures. §15.6 remains
  parked (GATT versus distinct brief evidence — not forked, not decided).
- **Validation:** `pnpm docs:check` — pending this PR's evidence gate.
  `pnpm aps:index:check` and `pnpm aps:drift` also in the gate. Stored
  progress stays 1/3 (ADR-053; feature PRs do not bump `N/M`).
- **Non-scope:** No runtime schema, crate, or MCP tool. No change to GCTX
  DTOs or GATT's attestation block. How brief evidence relates to GATT
  remains an open decision (§15.6 parked). CCTX-003 not started.
- **Files:** `plans/specs/2026-09-08-context-compiler.md`,
  `plans/modules/context-compiler.aps.md`, `plans/index.aps.md`
- **Dependencies:** CCTX-001
- **Claim:** private GitHub issue #4458
- **Confidence:** medium

### CCTX-003: Compare baselines against brief variants

- **Status:** Ready
- **Intent:** Learn whether a brief beats ordinary exploration and GCTX
  before building a compiler.
- **Expected Outcome:** A dated internal eval report records the four §12
  variants against the frozen corpus and §13 metrics. It distinguishes
  measured baselines from unmeasured brief variants, reports each initial
  gate as passed, failed, or unmeasured, and records zero hidden stale use
  on whatever was actually run. No product ships.
- **Validation:** `pnpm docs:check`
- **Non-scope:** No compiler, knowledge store, or feature flag. No change to
  enforcement. Does not replace the GCTX-031 `token_reduction` bench.
- **Dependencies:** CCTX-001, CCTX-002
- **Confidence:** low

## Related, not this module

Recorded so they are not pulled in as CCTX work:

| Thread | Owner |
| ------ | ----- |
| In-band graph-answer limits (caps, per-edge fidelity, cost self-report) | GATT |
| Exact committed-revision evidence for CONF | CEG |
| Policy eval-regression CI gate | EVALCI |
| Operator-approved Graph Trust Surfaces tracks | CGBDG, CONF, POLCAP, SCA, LSPNAV |
| GCTX projection contract and identity-only default | Archived GCTX / delivery spec |
