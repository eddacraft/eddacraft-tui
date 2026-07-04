# Trust Center Automation

| ID    | Owner  | Status  |
| ----- | ------ | ------- |
| TRUST | @aneki | Blocked |

**Last reviewed:** 2026-07-04 (status correction — see below)

> **Status correction (2026-07-04):** Demoted Ready → Blocked. TRUST's
> Purpose depends on compliance outputs, sourced from
> `compliance-evidence-workspace` (CEWS, Draft since 2026-04-26) and
> `compliance-reporting` (COMPLY, reset 2026-07-04 by POLRESET-010 to
> "post-first-slice expansion... Later" — explicitly not a near-term
> prerequisite). TRUST being Ready while both compliance sources it depends
> on are Draft was backwards, same class of drift as the CEWS correction it
> depends on. Promote back to Ready when COMPLY/CEWS land, or rescope
> TRUST-001 to a policy+eval-only artifact (both already available via
> EVAL, Done) if a compliance-free trust summary is viable sooner.

## Purpose

Automate trust-centre evidence publishing from Anvil policy, eval, and
compliance outputs to reduce manual due-diligence overhead.

## In Scope

- Trust artifact model and publication workflow
- Evidence freshness checks and ownership routing
- Export channels for buyer-facing trust summaries

## Work Items

<!-- Audit 2026-04-26: Validation commands updated for Rust crates per ADR-026. "artifact" left as-is (US-spelled identifier in task IDs and existing terminology). -->

### TRUST-001: Define trust artifact model

- **Status:** Blocked (compliance source Draft — see module status correction)
- **Intent:** Create canonical trust artifact schema for publishable evidence.
- **Expected Outcome:** Artifacts can be assembled from policy, eval, and compliance sources.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- trust_artifact_model`

### TRUST-002: Build publishing pipeline

- **Status:** Blocked (depends on TRUST-001)
- **Intent:** Automate generation and update of trust summaries.
- **Expected Outcome:** Publish pipeline emits dated, traceable trust outputs.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- trust_publishing`
- **Dependencies:** TRUST-001

### TRUST-003: Add freshness and ownership controls

- **Status:** Blocked (depends on TRUST-002)
- **Intent:** Ensure stale evidence is flagged and routed to owners.
- **Expected Outcome:** Trust artifacts include freshness state and escalation metadata.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- trust_freshness`
- **Dependencies:** TRUST-002

## Execution

Action plan: [../execution/TRUST.actions.md](../execution/TRUST.actions.md)
