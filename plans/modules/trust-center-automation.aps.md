# Trust Center Automation

| ID    | Owner  | Status |
| ----- | ------ | ------ |
| TRUST | @aneki | Ready  |

**Last reviewed:** 2026-05-25 (APSCAN-010 canonical-heading migration)

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

- **Status:** Ready
- **Intent:** Create canonical trust artifact schema for publishable evidence.
- **Expected Outcome:** Artifacts can be assembled from policy, eval, and compliance sources.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- trust_artifact_model`

### TRUST-002: Build publishing pipeline

- **Status:** Ready
- **Intent:** Automate generation and update of trust summaries.
- **Expected Outcome:** Publish pipeline emits dated, traceable trust outputs.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- trust_publishing`
- **Dependencies:** TRUST-001

### TRUST-003: Add freshness and ownership controls

- **Status:** Ready
- **Intent:** Ensure stale evidence is flagged and routed to owners.
- **Expected Outcome:** Trust artifacts include freshness state and escalation metadata.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- trust_freshness`
- **Dependencies:** TRUST-002

## Execution

Action plan: [../execution/TRUST.actions.md](../execution/TRUST.actions.md)
