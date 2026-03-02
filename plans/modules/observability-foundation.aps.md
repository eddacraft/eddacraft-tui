<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Observability Foundation

| ID  | Owner      | Status |
| --- | ---------- | ------ |
| OBS | @eddacraft | Draft  |

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

- `kindling-integration` — event/observation baseline and provenance links
- `dashboard-ops-views` — operational UI surfaces
- `edda-stack-integration` — telemetry and stack-level metrics requirements
- `cli-hardening` — API/DB resilience and health endpoint expectations

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

Steps: [../execution/OBS.steps.md](../execution/OBS.steps.md)
