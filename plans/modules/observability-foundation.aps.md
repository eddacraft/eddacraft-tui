<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Observability Foundation

| ID  | Owner      | Status | Progress |
| --- | ---------- | ------ | -------- |
| OBS | @eddacraft | Draft  | 0/5      |

**Last reviewed:** 2026-04-26

> **Audit note (2026-04-26):** Several upstream dependencies named below
> (`kindling-integration`, `edda-stack-integration`, `cli-hardening`) now live
> in `plans/archive/modules/`. They predate the TS→Rust migration and are not
> part of active scope. Re-derive concrete observability requirements from
> the live Rust crates (`anvil-cli`, `anvil-kernel`, `anvil-tui`) and the
> `dashboard-ops-views` module before promoting OBS to Ready.

## Purpose

Unify Anvil observability into one executable foundation so incidents can be detected, diagnosed, and resolved quickly. This module aligns telemetry emission, Neon database health visibility, dashboard/live ops views, and runbook-driven operations.

## In Scope

- Observability event contract across API/CLI/website paths
- Core health signals: uptime, error rate, latency, queue depth, email delivery outcomes
- Neon operational visibility: connectivity, query latency, transaction failures, capacity pressure
- Dashboard data contract for operational views (including real-time update feed)
- Alert thresholds + severity mapping for production support
- Runbooks for common operational failures and recovery actions

## Out of Scope

- Full enterprise SIEM integration
- Multi-region failover orchestration
- Team-wide on-call rota tooling

## Interfaces

**Depends on:**

- `dashboard-ops-views` — operational UI surfaces (active module)
- ~~`kindling-integration`~~ — archived; provenance/event baseline assumptions
  must be re-sourced from active modules before OBS goes Ready
- ~~`edda-stack-integration`~~ — archived; replace with concrete telemetry
  contract scoped to anvil-cli / anvil-kernel
- ~~`cli-hardening`~~ — archived; API/DB resilience expectations must be
  redocumented against current Rust CLI surfaces

**Exposes:**

- Canonical observability contract (signals + payload expectations)
- Neon health checklist + query diagnostics model
- Dashboard/live-feed data requirements for ops pages
- Runbook set for support + engineering responders

## Ready Checklist

Change status to **Ready** when:

- [ ] Scope and ownership confirmed (who owns alerts, dashboards, runbooks)
- [ ] Initial telemetry contract draft reviewed
- [ ] Dependencies mapped to active APS modules
- [ ] At least one execution task approved for sprint

## Tasks

### OBS-001: Observability signal inventory and contract

- **Intent:** Define one agreed list of production signals and payload semantics.
- **Expected Outcome:** Shared contract document covering metrics, event fields, log levels, and ownership.
- **Validation:** `rg -n "OBS-001|signal inventory|ownership" plans/modules/observability-foundation.aps.md docs/guides/runbooks/*.md`

### OBS-002: Neon operational health instrumentation baseline

- **Intent:** Make Neon failure modes visible before customer impact.
- **Expected Outcome:** Baseline checks and telemetry for connection failures, slow queries, and transaction degradation.
- **Validation:** `rg -n "Neon|DATABASE_URL|latency|transaction" docs/guides/runbooks/*.md`

### OBS-003: Dashboard operations real-time data contract

- **Intent:** Finalise the minimum real-time contract needed by dashboard ops views.
- **Expected Outcome:** Defined event feed schema and reconnection/fallback expectations for live operations pages.
- **Validation:** `test -f docs/public/anvil/operations/realtime-feed-contract.md && rg -n "event feed schema|SSE|WebSocket|reconnect|fallback" docs/public/anvil/operations/realtime-feed-contract.md`

### OBS-004: Alert thresholds and incident severity matrix

- **Intent:** Standardise when to page, when to warn, and when to watch.
- **Expected Outcome:** Threshold table mapped to severity levels with explicit responder actions.
- **Validation:** `rg -n "severity|threshold|page|warn" docs/guides/runbooks/*.md`

### OBS-005: Operations runbook pack v1

- **Intent:** Ensure common incidents have fast, repeatable playbooks.
- **Expected Outcome:** Published runbooks for Neon DB ops, waitlist email delivery, and observability triage.
- **Validation:** `test -f docs/guides/runbooks/neon-db-operations.md && test -f docs/guides/runbooks/observability-triage.md && test -f docs/guides/waitlist-email-operations.md`

## Execution

Steps: [../execution/OBS.steps.md](../execution/OBS.steps.md) *(file not yet
created — produce when module reaches Ready)*

> **Audit note (2026-04-26):** validation commands above reference
> `docs/guides/runbooks/*.md` and `docs/public/anvil/operations/realtime-feed-contract.md`,
> neither of which exist yet. Treat the validation rg/test invocations as
> placeholders the work items must produce, not as already-passing checks.
