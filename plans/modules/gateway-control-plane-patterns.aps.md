# Gateway Control Plane Patterns

| ID   | Owner  | Status | Progress |
| ---- | ------ | ------ | -------- |
| GATE | @aneki | Draft  | 0/3      |

**Last reviewed:** 2026-04-26

> **Audit note (2026-04-26):** Tier B (queued — Enterprise Readiness
> constellation). Status demoted Ready → Draft pending an enterprise
> consumer asking for gateway deployment guidance.
>
> Council C recommended converting to an ADR on the basis that GATE has
> "no code scope." That was partially wrong — GATE-001 is doc-only
> (topology spec) but GATE-002 (enforcement contract) and GATE-003
> (observability event model) have real Rust contract scopes in
> `crates/anvil-kernel-types` and `crates/anvil-policy`. The 3-task
> structure is correct; only the Ready status was wrong (no consumer).
>
> **Strategic context:** Enterprise readiness is moving up the priority
> stack. GATE is part of a coherent "Enterprise Readiness" constellation
> alongside POLFED (multi-repo federation), ORGHIER (org-level policy
> hierarchy), POLLC (lifecycle / canary / grace), COMPLY (SOC 2 / ISO /
> NIST framework support), CEWS (auditor surfaces), TRUST (public trust
> artefacts). These should be sequenced together when the first
> enterprise prospect surfaces.
>
> **Promotion gate to Ready:**
> - First enterprise prospect or design-partner request for gateway
>   deployment guidance, OR
> - Internal decision to ship Anvil's own deployment topology as a
>   reference (e.g. for the docs site / api gateway in `infra/`).
>
> **Followup work pending** (tracked separately):
> 1. Coordinate GATE-002 enforcement contract with INTD-002 (daemon RPC
>    schema) and DRVR-002 (driver protocol) — these are sister contracts.
> 2. Coordinate GATE-003 observability event model with INTD-013 /
>    RTAI-007 telemetry envelope — single shape across surfaces.
> 3. Confirm GATE-001 reference topologies match what `infra/` actually
>    deploys (Vercel + Azure DNS today; future enterprise modes).

## Purpose

Define deployable gateway control-plane patterns for central policy enforcement, routing visibility, and topology guidance in enterprise environments.

## In Scope

- Reference deployment topologies
- Control-plane policy enforcement points
- Observability and audit signal contracts

## Work Items

<!-- Audit 2026-04-26: Tasks not zero-padded per APS convention; convention is GATE-001 etc. — IDs already correct. Validation commands updated for Rust crates per ADR-026. -->

### GATE-001: Define reference topologies
- **Intent:** Capture supported gateway deployment patterns and trust boundaries.
- **Expected Outcome:** Topology definitions include traffic flow and policy points.
- **Validation:** Manual doc review in `docs/`

### GATE-002: Define control-plane enforcement contract
- **Intent:** Specify policy interception and decision contracts at gateway boundaries.
- **Expected Outcome:** Enforcement points have consistent request/decision schema.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- gateway_enforcement`
- **Dependencies:** GATE-001

### GATE-003: Define observability event model
- **Intent:** Provide standard events for routing, denials, and policy outcomes.
- **Expected Outcome:** Gateways emit auditable event streams for operations.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- gateway_events`
- **Dependencies:** GATE-002

## Execution

Steps: [../execution/GATE.steps.md](../execution/GATE.steps.md)
