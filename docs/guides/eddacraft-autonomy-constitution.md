# eddacraft Autonomy Constitution (v0)

| Type  | Authority     | Owner   | Status | Freshness                                                                 |
| ----- | ------------- | ------- | ------ | ------------------------------------------------------------------------- |
| Guide | Authoritative | HARNESS | Draft  | Last updated 2026-03-09; metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream                                 | Downstream                              |
| ---------------------------------------- | --------------------------------------- |
| Harness engineering operating principles | Agent autonomy and governance workflows |

Status: Draft for operational use  
Owner: Harness Engineering  
Last updated: 2026-03-09

## 1) Purpose

Define how autonomy is safely exercised in eddacraft systems.

This constitution exists to ensure:

- autonomy increases delivery throughput,
- without degrading trust, quality, or governance,
- and with explicit human control at decision boundaries.

## 2) Core distinction

- **Agentic Engineering**: philosophy and capability posture (what agents can
  do).
- **Harness Engineering**: runtime governance, controls, and operating model
  (how we reliably direct what agents do).

**Policy:** eddacraft prioritises Harness Engineering as the production
discipline.

## 3) Decision rights by autonomy tier

### Tier 0 — Autonomous (pre-approved)

Agents may act without approval when work is:

- reversible,
- low blast radius,
- internal-only,
- and policy-compliant.

Examples:

- docs edits,
- formatting/lint-only fixes,
- non-breaking test updates,
- issue/label hygiene.

### Tier 1 — Bounded autonomy (approve once)

Agents may execute within a pre-approved scope window.

Requirements:

- scope and constraints declared up front,
- explicit success criteria,
- stop conditions,
- evidence checkpoint before merge.

Examples:

- scoped refactors,
- reliability improvements,
- planned feature slices with clear acceptance criteria.

### Tier 2 — Human-gated (always approve)

Human approval is mandatory before execution and before merge.

Examples:

- architecture boundary changes,
- security posture changes,
- external/public actions,
- spending or irreversible operations,
- policy/governance rule changes.

## 4) Escalation triggers (mandatory)

Autonomous execution must pause and escalate when any trigger is hit:

- confidence drops below declared threshold,
- unresolved blocker > 5 minutes in active run,
- repeated failure loop (same class twice),
- dependency/assumption mismatch,
- potential production/security impact,
- ambiguous requirement affecting scope or safety.

Escalation update must include:

- what blocked,
- what was attempted,
- what decision/input is required.

## 5) Verification contract

No completion claim without evidence.

Required before “done”:

- relevant checks/tests executed,
- outputs captured,
- acceptance criteria mapped to evidence,
- residual risks explicitly listed.

Rule: **evidence before assertion**.

## 6) APS ↔ GitHub (GH) orchestration contract

### APS is canonical for

- task decomposition,
- dependencies,
- readiness semantics,
- acceptance criteria intent.

### GitHub is canonical for

- execution flow state,
- assignment/review lifecycle,
- merge and CI outcomes,
- delivery telemetry.

### Orchestrator loop responsibilities

- select next APS-ready work,
- ensure GH execution artifact exists,
- keep APS↔GH linkage current,
- detect and reconcile drift,
- escalate when state conflicts persist.

## 7) Safety and rollback

Every autonomous change path must maintain:

- recoverable rollback path,
- bounded retries,
- explicit kill switch.

If rollback confidence is low, escalate before apply.

## 8) Audit minimums

Every meaningful autonomous action should record:

- actor/session id,
- intent + scope,
- inputs and tool operations,
- outputs/artifacts,
- decision rationale,
- verification evidence,
- final state (complete/escalated/aborted).

## 9) Guardrail against autonomy theatre

Autonomy is not judged by volume of autonomous actions.

Primary scorecard:

- throughput with stable quality,
- low rework,
- low incident rate,
- predictable escalation quality,
- operator trust.

## 10) Adoption policy

Default rollout model:

1. start at Tier 0,
2. graduate to Tier 1 per stable lane,
3. keep Tier 2 human-gated unless explicitly redesigned and approved.

This constitution is a living governance artifact owned by Harness Engineering.
