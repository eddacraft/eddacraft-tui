<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Observability Foundation

| ID  | Owner      | Status   | Progress                                                  |
| --- | ---------- | -------- | --------------------------------------------------------- |
| OBS | @eddacraft | Proposed | 0/5 (was 0/6 before OBS-006 migrated to TRACE-001 on 2026-04-30) |

**Last reviewed:** 2026-05-28 — OBS-001..005 fleshed out (Status / Files /
Dependencies / Confidence added; archived upstreams re-sourced to live modules).
Module stays **Proposed**, not Ready: this is post-launch domain-ops hardening
deliberately deferred behind the `v0.7.x` daemon-working slate, and the live
dashboard surface it depends on (`dashboard-ops-views`, DASHOPS, Ready 0/7) has
not started, so execution authority is not yet open. Promote to Ready only when
the Ready Checklist below is satisfied. Note: DASHOPS currently defers real-time
SSE/WebSocket updates ("not needed for local dev tool"), which directly bounds
OBS-003 — reconcile the live-feed premise with DASHOPS before promoting OBS-003.

> **Scope reduction (2026-04-30,
> [ADR-034](../decisions/034-cross-cutting-modules-as-aps-primitive.md)):**
> Cross-cutting tracing scope (originally OBS-006) migrated to
> [`tracing-foundation.aps.md`](./tracing-foundation.aps.md) (TRACE) on
> 2026-04-30 per [ADR-035](../decisions/035-three-pipe-observability-rule.md)
> / Planning Council session plan-b00c16c7. OBS-001..005 remain Draft and
> deferred to post-launch hardening. The new
> [observability-export](./observability-export.aps.md) module (EXPORT)
> owns the deferred production sink choice. See
> [ADR-035](../decisions/035-three-pipe-observability-rule.md) for the
> three-pipe rule (Kindling = governance facts; Notification =
> user-visible state; tracing/OTEL = ephemeral debugging) that pins each
> pipe's role.

> **Audit note (2026-04-26):** Several upstream dependencies named below
> (`kindling-integration`, `edda-stack-integration`, `cli-hardening`) now live
> in `plans/archive/modules/`. They predate the TS→Rust migration and are not
> part of active scope. Re-derive concrete observability requirements from
> the live Rust crates (`anvil-cli`, `anvil-kernel`, `anvil-tui`) and the
> `dashboard-ops-views` module before promoting OBS to Ready. The
> tracing-baseline portion of that re-derivation moved to TRACE on
> 2026-04-30 (see scope reduction note above); OBS-001..005 remain
> domain-ops scope and stay Draft.

## Purpose

Unify Anvil observability into one executable foundation so incidents can be detected, diagnosed, and resolved quickly. This module aligns telemetry emission, Neon database health visibility, dashboard/live ops views, and runbook-driven operations.

## In Scope

- Observability event contract across API/CLI/website paths
- Core health signals: uptime, error rate, latency, queue depth, email delivery outcomes
- Neon operational visibility: connectivity, query latency, transaction failures, capacity pressure
- Dashboard data contract for operational views (including real-time update feed)
- Alert thresholds + severity mapping for production support
- Runbooks for common operational failures and recovery actions
- ~~Runtime tracing baseline across Rust services, CLI entry points, and hosted
  API paths~~ — **migrated to TRACE on 2026-04-30** (see scope reduction note
  at top); now owned by [`tracing-foundation`](./tracing-foundation.aps.md)

## Out of Scope

- Full enterprise SIEM integration
- Multi-region failover orchestration
- Team-wide on-call rota tooling

## Interfaces

**Depends on:**

- `dashboard-ops-views` (DASHOPS, Ready 0/7) — operational UI surfaces that
  consume the OBS-003 data contract; OBS-003 must reconcile with the DASHOPS
  "real-time SSE/WebSocket deferred" Out-of-Scope decision before promotion.
- `tracing-foundation` (TRACE, In Progress 2/4) — the `anvil-observability`
  Rust crate, `traceparent` propagation, and namespace registry already own the
  span/correlation/redaction surface OBS once scoped under OBS-006. OBS signals
  reference TRACE namespaces rather than re-defining tracing conventions.
- `apps/anvil-api/` — the live hosted API surface that emits the API/DB health
  signals (replaces the archived `cli-hardening` resilience assumptions).
- ~~`kindling-integration`~~ — archived; provenance/event baseline now emitted by
  the Rust kernel/CLI (per the `dashboard-ops-views` provenance-source REVIEW
  note). Re-source provenance event fields from the active kernel emitter, not
  from this archived module.
- ~~`edda-stack-integration`~~ — archived; the concrete telemetry contract is
  now scoped against `anvil-cli` / `anvil-kernel` and the `apps/anvil-api`
  routes, captured in OBS-001.

**Exposes:**

- Canonical observability contract (signals + payload expectations)
- Neon health checklist + query diagnostics model
- Dashboard/live-feed data requirements for ops pages
- Runbook set for support + engineering responders
- ~~Tracing conventions for spans, correlation IDs, redaction, and exporter
  boundaries~~ — **migrated to TRACE on 2026-04-30**; tracing conventions are
  now exposed by [`tracing-foundation`](./tracing-foundation.aps.md) (the
  `anvil-observability` Rust crate, the `traceparent` propagation surface,
  and the namespace registry stub)

## Ready Checklist

Change status to **Ready** when:

- [x] Dependencies mapped to active APS modules (DASHOPS, TRACE, `apps/anvil-api`;
      archived upstreams re-sourced — 2026-05-28)
- [ ] Scope and ownership confirmed (who owns alerts, dashboards, runbooks for
      the hosted surface)
- [ ] OBS-003 live-feed premise reconciled with the DASHOPS "real-time
      SSE/WebSocket deferred" decision (drop, defer, or re-scope OBS-003)
- [ ] Initial telemetry contract draft (OBS-001) reviewed
- [ ] At least one execution task approved for the active release window — OBS is
      post-launch hardening and is not currently in a release wave

## Tasks

### OBS-001: Observability signal inventory and contract

- **Intent:** Define one agreed list of production signals and payload semantics.
- **Expected Outcome:** Shared contract document covering metrics, event fields, log levels, and ownership, scoped to the live `apps/anvil-api` routes, the `anvil-cli` / `anvil-kernel` emitters, and Neon DB signals. References TRACE namespaces rather than redefining tracing conventions.
- **Files:**
  - `docs/runbooks/observability-triage.md` (extend with the signal inventory) or a new `docs/public/anvil/operations/signal-inventory.md`
- **Dependencies:** TRACE-001 (namespace registry; Done) for namespace alignment.
- **Confidence:** medium — the signal set must be enumerated from live API/CLI surfaces; the document shape is clear.
- **Validation:** `rg -n "OBS-001|signal inventory|ownership" plans/modules/observability-foundation.aps.md docs/runbooks/*.md`

### OBS-002: Neon operational health instrumentation baseline

- **Intent:** Make Neon failure modes visible before customer impact.
- **Expected Outcome:** Baseline checks and telemetry for connection failures, slow queries, and transaction degradation, wired against the `apps/anvil-api/src/db/` client and extending the existing `docs/runbooks/neon-db-operations.md` runbook with the live-signal mapping.
- **Files:**
  - `docs/runbooks/neon-db-operations.md` (exists — extend with health signals + thresholds)
  - `apps/anvil-api/src/db/` (instrumentation hooks, when implemented)
- **Dependencies:** OBS-001 (signal contract).
- **Confidence:** medium — Neon client surface and runbook exist; the failure-mode telemetry wiring is new.
- **Validation:** `rg -n "Neon|DATABASE_URL|latency|transaction" docs/runbooks/*.md`

### OBS-003: Dashboard operations real-time data contract

- **Intent:** Finalise the minimum real-time contract needed by dashboard ops views.
- **Expected Outcome:** Defined event feed schema and reconnection/fallback expectations for live operations pages.
- **Blocked-on (design):** DASHOPS lists "Real-time WebSocket/SSE updates — deferred (not needed for local dev tool)" as Out of Scope. This item cannot proceed until that decision is reconciled — either DASHOPS reopens live updates, OBS-003 is re-scoped to a polled snapshot contract, or OBS-003 is dropped. Do not produce the contract doc until the premise is settled.
- **Files:**
  - `docs/public/anvil/operations/realtime-feed-contract.md` (new, if the live-feed premise survives reconciliation)
- **Dependencies:** OBS-001 (signal contract); coordination with DASHOPS Out-of-Scope decision.
- **Confidence:** low — premise conflicts with the consuming module's current scope.
- **Validation:** `test -f docs/public/anvil/operations/realtime-feed-contract.md && rg -n "event feed schema|SSE|WebSocket|reconnect|fallback" docs/public/anvil/operations/realtime-feed-contract.md`

### OBS-004: Alert thresholds and incident severity matrix

- **Intent:** Standardise when to page, when to warn, and when to watch.
- **Expected Outcome:** Threshold table mapped to severity levels with explicit responder actions, covering the OBS-001 signals (API health, Neon, email delivery) and cross-linked from the triage runbook.
- **Files:**
  - `docs/runbooks/observability-triage.md` (exists — add the threshold/severity matrix)
- **Dependencies:** OBS-001 (signals to threshold against), OBS-002 (Neon signal baseline).
- **Confidence:** medium — runbook exists; threshold values need owner sign-off.
- **Validation:** `rg -n "severity|threshold|page|warn" docs/runbooks/*.md`

### OBS-005: Operations runbook pack v1

- **Intent:** Ensure common incidents have fast, repeatable playbooks.
- **Expected Outcome:** Published runbooks for Neon DB ops, waitlist email delivery, and observability triage, each cross-linked to the OBS-001 signals and OBS-004 thresholds and validated as executable.
- **Files:**
  - `docs/runbooks/neon-db-operations.md` (exists)
  - `docs/runbooks/observability-triage.md` (exists)
  - `docs/runbooks/waitlist-email-operations.md` (exists)
- **Dependencies:** OBS-001 (signals), OBS-004 (thresholds). The three target runbooks already exist; this item validates and cross-links them rather than authoring from scratch.
- **Confidence:** high — all three runbook targets are already present; the work is reconciliation and cross-linking.
- **Validation:** `test -f docs/runbooks/neon-db-operations.md && test -f docs/runbooks/observability-triage.md && test -f docs/runbooks/waitlist-email-operations.md`

### ~~OBS-006~~ — superseded by TRACE-001

Originally scoped as the runtime tracing baseline. Migrated to
[`tracing-foundation`](./tracing-foundation.aps.md) on 2026-04-30 per
Planning Council session plan-b00c16c7 and
[ADR-035](../decisions/035-three-pipe-observability-rule.md). The full
launch-blocker scope (subscriber init, W3C `traceparent` propagation,
redaction layer, namespace registry stub, INTD-014 fixture update) now
lives under TRACE-001. The deferred follow-ups (TS mirror, redaction
hardening) are TRACE-002 and TRACE-003. The deferred production sink
choice is the EXPORT module.

## Execution

Action plan: `../execution/OBS.actions.md` *(not yet created — produce when the
module reaches Ready, using the canonical `.actions.md` suffix per
`plans/project-context.md`)*

> **Audit note (2026-04-26):** validation commands above reference
> `docs/runbooks/*.md` and `docs/public/anvil/operations/realtime-feed-contract.md`.
> The runbook directory exists, but treat missing target files or contents as
> placeholders the work items must produce, not as already-passing checks.
