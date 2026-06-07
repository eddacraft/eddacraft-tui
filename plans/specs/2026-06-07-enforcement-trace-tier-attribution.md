# Enforcement Trace — Deterministic Tier-Attribution Receipt — Design Spec

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | TBD (proposed; `CEWS` on adoption) | Draft | Last reviewed 2026-06-07 against `crates/anvil-policy-engine/src/result.rs`, `plans/decisions/062-policy-evidence-drift-as-evidence.md` |

| Upstream | Downstream |
| -------- | ---------- |
| `plans/decisions/037-witness-chain-and-l4-policy.md`, `plans/decisions/058-sarif-shared-emitter-no-finding-model.md`, `plans/decisions/062-policy-evidence-drift-as-evidence.md`, `plans/modules/compliance-evidence-workspace.aps.md` | Future Enforcement Trace ADR and APS plan (none live yet) |

**Status:** Draft — pending Planning Council review (cross-boundary).
**Date:** 2026-06-07
**Owner:** TBD (proposed)
**APS module:** TBD — proposed home is the Compliance Evidence Workspace
  (`CEWS`, currently Draft); see §8 (Home Decision). Not yet APS-planned.
**ADR:** New ADR to be drafted under the owning work item (tier vocabulary +
  model-authority exclusion). Builds on ADR-002, ADR-003, ADR-031, ADR-037,
  ADR-038, ADR-058, ADR-062.
**Provenance:** Anvil Opportunity Assessment of GhostGuard
  (`joemunene-by/ghostguard`, MIT, checked 2026-06-05), promoted from the
  2026-05-07 radar entry. Borrow classified **Inspiration Only**; no source
  code adopted.

## 1. Context

Anvil already makes tiered enforcement decisions across distinct layers:

- `anvil_hook::verdict::Verdict` — hook-surface closure (Pass / Warn / Block),
  ADR-038.
- `anvil_intercept::enforcement::EnforcementDecision` — intercept-lane
  Allow / Interrupt, pure computation (no trace emission).
- `anvil_l4::decide::CommitDecision` — commit-level Allow / Block /
  NeedsL4Validation, routed by `OnNoWitness` policy (ADR-037).
- `anvil_policy_engine` — `Finding`, `Trace`, `Coverage`; surfaced via
  `anvil policy eval --explain` / `--why <finding-id>` (POLENG-006).

What Anvil lacks is a single, exportable record that *attributes* a binding
verdict across those tiers: which tier produced the decision, which rule or
constraint matched, what each tier cost, and — the differentiator — proof that
the winning tier was **deterministic and reproducible**, not model-adjudicated.
Today that information is scattered across per-layer decisions and internal
telemetry, none of which a customer can hand to an auditor as one artefact.

GhostGuard (`joemunene-by/ghostguard`) is an early-stage AI-agent security
proxy (1 star, 2 commits, MIT — design inspiration only). Its useful shape is a
four-tier, ordered, first-match pipeline — static rules → pattern scan →
anomaly/rate-limit → optional LLM judge — with a per-decision audit record
(`tool_name`, `arguments`, `verdict`, `reason`, `tier`, timestamp). The borrow
is **not** the proxy and **not** the tier inventory. The borrow is the idea of
an explicit, exportable, tier-attributed decision receipt.

This spec adapts that idea into Anvil's deterministic, warnings-over-blocks,
new-edges-only model, and deliberately **excludes** GhostGuard's anomaly and
LLM-judge tiers from the binding vocabulary (§6, §7).

## 2. Goals

1. Define an **Enforcement Trace**: a per-decision evidence record that names
   the winning tier, the matched rule/constraint, the verdict, the explanation
   reference, and the per-tier cost, for one governed enforcement decision.
2. Make **determinism attestation** a first-class, required field: the receipt
   proves the binding verdict came from a deterministic, reproducible tier.
3. Pin a **closed-set tier vocabulary** (§6) of deterministic tiers only, with
   `EscalateToReview` as the terminal for ambiguity — human/deferred review,
   never model-as-authority.
4. Reference, never re-implement, existing provenance: the trace links to the
   relevant `WitnessLine` (ADR-037) and existing per-layer decisions rather
   than copying their fields.
5. Be exportable as evidence (JSONL/CSV), aligned with CEWS export goals, and
   deterministically testable (injected `AsOf` time, no live clock — ADR-031).

## 3. Non-goals

1. **Not** an enforcement engine. The trace records decisions other layers
   already make; it does not adjudicate. `hook`, `intercept`, and `gate`
   remain the sole enforcement authorities.
2. **Not** a unified cross-command finding model. ADR-058 stands: per-command
   finding shapes remain; the trace is a *receipt over* a decision, attached to
   an existing record home (§8), not a new universal engine.
3. **Not** an adoption of an anomaly-detection tier. GhostGuard's sliding-window
   anomaly/rate-limit tier is out of scope; if rate-limit signal ever exists in
   Anvil it is referenced, not defined here.
4. **Not** an adoption of an LLM-judge tier as binding authority. Model
   assistance, if ever added, is advisory-only and lives outside the binding
   verdict path (§7).
5. **Not** a proxy. Anvil does not sit on the OpenAI/Anthropic base URL. The
   GhostGuard deployment model is explicitly not borrowed.

## 4. Scope-guard fit (`docs/vision/anvil-scope-guard.md`)

1. **Prevention capability?** Indirect — the trace is evidence *about*
   prevention decisions, strengthening the provenance chain rather than adding a
   new block.
2. **Pre-execution?** Yes — it records decisions taken before the governed
   action proceeds, alongside the witness chain.
3. **Deterministic?** Yes, by construction — deterministic-only binding tiers,
   injected time, reproducible attribution (ADR-031).
4. **Enforces or just informs?** Informs/proves. Enforcement stays with
   `hook`/`intercept`/`gate`; the trace is non-authoritative evidence.
5. **New edges only?** Yes — the trace inherits the `is_new_edge` annotation of
   the decision it records (ADR-003); baselined decisions are marked as such.

## 5. The primitive

**Name:** Enforcement Trace (`EnforcementTrace`) — a Deterministic
Tier-Attribution Receipt.

**Definition:** an append-once evidence record describing one governed
enforcement decision, proving which deterministic tier produced the binding
verdict and which rule/constraint matched, with the determinism attestation and
provenance links required to reproduce and audit it later.

Indicative shape (illustrative — field set is ratified by the owning ADR, not by
this draft). Optional fields use `#[serde(default, skip_serializing_if =
"Option::is_none")]`, per the house convention:

```json
{
  "schema_version": "anvil.enforcement-trace.v1",
  "decision_ref": "string",            // stable id of the recorded decision
  "normalized_action_ref": "string",   // ref to the normalised action under eval
  "tool_name": "string",
  "policy_ref": "string",
  "policy_digest": "string",           // hash of the effective policy
  "tier_order": ["StaticRule", "PatternScan", "RateLimit"],
  "winning_tier": "StaticRule",        // closed-set, §6
  "matched_rule_ref": "string|null",
  "matched_constraint_ref": "string|null",
  "pattern_scan_result": "Clean|Matched|Skipped|null",
  "rate_limit_state_ref": "string|null",   // referenced, not defined here (§3.3)
  "determinism": "Deterministic|EscalateToReview", // required attestation (§7)
  "advisory_refs": [],                 // non-binding model/heuristic notes (§7)
  "verdict": "Pass|Warn|Block|EscalateToReview",
  "explanation_ref": "string",         // stable reason, not free text
  "tier_latency_ms": { "StaticRule": 0.1 },  // optional observability
  "total_latency_ms": 0.1,             // optional observability
  "witness_line_ref": "string|null",   // link to provenance (ADR-037)
  "audit_event_ref": "string|null",
  "is_new_edge": true,                 // inherited (ADR-003)
  "as_of_unix": 0                      // injected, never live clock (ADR-031)
}
```

### Divergence from Morgan's proposed field set

Morgan's radar entry proposed an 18-field `EnforcementPipelineTrace` /
`TieredPolicyDecision` fixture. This spec **retains** the attribution core
(`decision_ref`, `normalized_action_ref`, `tool_name`, `policy_ref`,
`policy_digest`, `tier_order`, `winning_tier`, `matched_rule_ref`,
`matched_constraint_ref`, `pattern_scan_result`, `tier_latency_ms`,
`total_latency_ms`, `verdict`, `explanation_ref`, `audit_event_ref`) and
**diverges** as follows:

- **Drops `model_judge_used` and `model_judge_result`** from the canonical
  shape. Including them normalises the LLM-judge tier as part of Anvil's binding
  vocabulary — the exact thing the source note warns against. Model assistance,
  if ever present, is represented as non-binding `advisory_refs` (§7).
- **Adds `determinism`** as a required attestation — the durable differentiator
  Morgan's latency framing under-weighted (§7).
- **Demotes `anomaly_signal_ref`** to an out-of-scope referenced signal
  (`rate_limit_state_ref`, nullable), since Anvil has no anomaly tier (§3.3).
- **Adds `witness_line_ref` and `is_new_edge`** to bind the receipt to existing
  provenance and baseline semantics rather than standing alone.

## 6. Tier vocabulary (closed-set)

Binding tiers are deterministic-only. Following the house pattern of
schema-versioned closed-set vocabularies (`WorktreeClaimState`, `BlockReason`,
`ErrorClass`), the initial set is:

| Tier | Cost class | Notes |
| --- | --- | --- |
| `StaticRule` | cheap | exact/glob tool + constraint match |
| `PatternScan` | cheap | regex/blocklist scan over normalised args |
| `RateLimit` | cheap | references existing rate-limit signal if present; not defined here |
| `EscalateToReview` | terminal | ambiguity routed to human/deferred review |

Deliberately **excluded** from the binding set: `AnomalyModel`, `LlmJudge`.
These are GhostGuard tiers Anvil has not chosen; admitting them to the
closed-set vocabulary would commit Anvil to capabilities it lacks and (for the
judge) to model authority it has rejected. Adding a tier is a schema-version +
ADR event, not an additive field.

## 7. Determinism attestation and the model-authority boundary

The receipt's central guarantee, and the reframe of Morgan's "latency/safety
trade-off":

- The binding `verdict` is always produced by a deterministic tier (§6) or is
  `EscalateToReview`. `determinism` attests which.
- An ambiguous case **escalates to review** with a named human/deferred owner.
  It does **not** fall through to a model that becomes binding authority. This
  preserves warnings-over-blocks (ADR-002): escalation is a routing outcome, not
  a silent allow.
- Any model or heuristic second opinion, if ever introduced, is recorded only in
  `advisory_refs` — explicitly non-binding, never the source of `verdict`.

Buyer language this enables: *"Every block is attributable to a named
deterministic rule at a known tier, and we can prove no model made the call."*

## 8. Home decision (open — blocks APS planning)

The trace must attach to an existing record home, not float as a standalone
fixture (ADR-058). Two candidate homes, to be resolved by Planning Council:

1. **CEWS `EvidenceRecord`** (preferred) — the trace becomes an enforcement
   evidence record, exported with other compliance evidence. **Blocker:** CEWS
   was demoted Ready → Draft on 2026-04-26 pending upstream COMPLY; planning the
   trace now would front-run its home.
2. **Policy-engine result annotation** — the trace is computed as an annotation
   on `anvil_policy_engine` results and surfaced through the existing
   `--explain` / `--why` path, with CEWS consuming it later. Lower coupling to
   COMPLY; narrower export story initially.

Until this is decided, the primitive stays at **Specification** disposition and
does not enter APS planning.

## 9. Fixtures (artefact of this spec, not a standalone commitment)

Two golden fixtures demonstrate the contract, mirroring the existing
`status_v1` golden-fixture pattern:

1. **Fast deterministic deny** — `StaticRule` winning tier, matched rule ref,
   `determinism: Deterministic`, `verdict: Block`, sub-millisecond
   `total_latency_ms`. Proves cheap-first attribution.
2. **Ambiguous escalation** — no deterministic tier produces a definitive
   verdict; `winning_tier: EscalateToReview`, `verdict: EscalateToReview`,
   `advisory_refs` may carry a non-binding note. Proves the model-authority
   boundary: ambiguity goes to review, not to a binding judge.

## 10. Open questions

1. Home decision (§8) — CEWS record vs policy-engine annotation.
2. Whether `RateLimit` belongs in the v1 binding set or is deferred until a
   real rate-limit signal exists in Anvil.
3. Relationship between `explanation_ref` and existing finding `message` fields
   — reuse vs new stable-reason registry.
4. Whether `tier_latency_ms` survives the determinism fence as a recorded field
   or stays out-of-band telemetry (ADR-031 / ADR-035 three-pipe rule).
5. Export surface ownership — CEWS export vs a dedicated enforcement-receipt
   export, and overlap with SARIF (2026-05-29 SARIF output design).

## 11. References

- ADR-002 Warnings over blocks; ADR-003 New edges only; ADR-031 Validation
  latency rubric; ADR-035 Three-pipe observability; ADR-037 Witness chain and
  L4 policy; ADR-038 Hook surface and noise discipline; ADR-058 Per-command
  finding shapes; ADR-062 Policy/evidence drift as evidence.
- `plans/modules/compliance-evidence-workspace.aps.md` (CEWS, Draft).
- `crates/anvil-hook/src/verdict.rs`, `crates/anvil-intercept/src/enforcement.rs`,
  `crates/anvil-l4/src/decide.rs`, `crates/anvil-policy-engine/src/{result,trace,coverage}.rs`.
- GhostGuard: `https://github.com/joemunene-by/ghostguard` (MIT, inspiration only).
