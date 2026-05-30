<!--
APS Module: Observability Export
================================
Production telemetry sink — sampling, retention, and exporter wiring for
the tracing pipe. Stays Draft until a paying customer or production
incident motivates the sink choice. Owns OQ1 from Planning Council session
plan-b00c16c7.

See plans/aps-rules.md#cross-cutting-modules and
plans/decisions/035-three-pipe-observability-rule.md.
-->

# Observability Export

| ID     | Owner      | Status | Progress |
| ------ | ---------- | ------ | -------- |
| EXPORT | @eddacraft | Draft  | 0/1      |

**Last reviewed:** 2026-04-30

> **Provenance:** Planning Council session plan-b00c16c7 (2026-04-30) split
> observability into three modules — OBS (domain ops, deferred), TRACE
> (cross-cutting tracing baseline, launch-blocker), and EXPORT (this
> module, production sink choice, deferred). See
> [ADR-035](../decisions/035-three-pipe-observability-rule.md) for the
> three-pipe rule that places EXPORT downstream of the tracing pipe.

## Purpose

Choose, wire, and operate a production sink for the tracing pipe. EXPORT
covers sampling strategy, retention, exporter configuration, and any
sink-specific dashboards or alert plumbing that comes with the chosen
backend. EXPORT does **not** introduce a new pipe — per ADR-035, tracing
is ephemeral debugging context, not source-of-truth.

## In scope

- Sink choice and the rationale ADR (Tempo / Honeycomb / Grafana Cloud /
  self-hosted Jaeger / OTLP-to-Vercel-OTel are the candidates surfaced at
  Council).
- Sampling strategy (head / tail / hybrid, per-binary defaults).
- Retention policy and cost-of-tracing budget.
- Exporter wiring in `anvil-observability` (Rust) and
  `@anvil/observability` (TS, after TRACE-002).
- Sink-specific dashboards or correlation-with-Kindling views, if the
  chosen sink has them.

## Out of scope

- The tracing baseline itself (owned by
  [tracing-foundation](./tracing-foundation.aps.md)).
- Anything Kindling-shaped or notification-shaped — per ADR-035 those
  pipes are not EXPORT's concern.
- The `observability-triage.md` runbook (owned by OBS-005).

## Interfaces

**Depends on:**

- `anvil-observability` Rust crate from TRACE-001 — EXPORT wires its
  exporter into the existing subscriber init.
- `@anvil/observability` from TRACE-002 — required before any TS-side
  sink is wired.
- ADR-035 (three-pipe rule) — pins the role this module is allowed to
  play.

**Coordinates with:**

- TRACE-003 (redaction-layer hardening) — sampled exporters MUST honour
  the redaction layer; EXPORT inherits that constraint, does not weaken
  it.
- OBS module (post-launch domain ops work) — alert thresholds and
  runbook material that depend on sink-specific telemetry land here as
  cross-references when both modules are active.

**Exposes:**

- The chosen sink, its credentials story, its sampling defaults, and
  the operational runbook for it.

## Work Items

> Status: Draft. The sink is chosen and ratified — Azure Monitor +
> Application Insights ([ADR-059](../decisions/059-production-tracing-sink.md),
> Accepted 2026-05-30) — and EXPORT-001 is now fully specified with concrete
> validation. EXPORT execution (wiring the exporter) stays deferred until a
> paying customer or first production incident justifies it; the architecture
> and acceptance criteria are settled, so promotion to Ready is a timing call,
> not a design one.

### EXPORT-001: Choose, ratify, and wire the production tracing sink

- **Intent:** Anvil's tracing pipe has its ratified production sink
  (Azure Monitor + Application Insights per ADR-059) wired for the
  operator-hosted TS API, with documented sampling and exporter config;
  the local-first Rust CLI/daemon stay formatter-only and never export.
- **Expected Outcome:** The sink choice + trade-offs are recorded in
  [ADR-059](../decisions/059-production-tracing-sink.md) (Azure Monitor +
  Application Insights, Accepted). `@eddacraft/anvil-observability` and
  `apps/anvil-api` carry the **Azure Monitor OpenTelemetry exporter**,
  gated behind config that is **off by default** so local development —
  and the Rust binaries — stay formatter-only. The App Insights
  connection string is wired through the existing Pulumi-managed secrets
  path. Sampling defaults match the cost budget ADR-059 agreed to.
- **Coordinates with:** [ADR-059](../decisions/059-production-tracing-sink.md)
  (sink decision + binding constraints), TRACE-001 (subscriber init is the
  integration point), TRACE-002 (TS-side wiring), TRACE-003 (redaction layer
  must not be bypassed by the exporter), USAGE (feature-flagged usage is
  Kindling-of-record, not this pipe — App Insights breadcrumbs only).
- **Validation:** CI-runnable deterministic tests at the exporter
  boundary (`pnpm --filter @eddacraft/anvil-observability test`, plus the
  `apps/anvil-api` tracing-init test), with a single documented manual
  end-to-end check against a staging Application Insights resource:
  - **V1 — exporter wiring + gating:** with the export config enabled,
    tracing init attaches the Azure Monitor OTel trace exporter built from
    the App Insights connection string; with the config off (the default),
    no exporter is attached and spans stay formatter-only. Asserted with a
    captured/in-memory exporter double — no live Azure call.
  - **V2 — redaction not bypassed (TRACE-003):** a span carrying an
    attribute whose name matches `SENSITIVE_FIELDS`, run through the
    exporter pipeline, yields an exported payload in which the denied
    attribute is redacted/absent. Proves the redaction layer wraps the
    Azure Monitor exporter rather than the exporter shipping raw spans.
  - **V3 — sampling applied:** the configured head/parent sampler is
    honoured — a seeded/deterministic sampling decision keeps and drops as
    configured, and the default ratio matches the documented cost budget.
  - **V4 — secrets, no silent default:** a missing or blank connection
    string disables export with a clear log line and a non-export fall back
    to formatter-only — never a panic and never a silent
    export-to-nowhere (per the operator-config propagation rule).
  - **V5 — end-to-end ingest (manual, staging):** with the exporter
    enabled against a staging App Insights resource, an emitted span from
    `apps/anvil-api` is queryable in App Insights within ingestion latency
    — KQL `union traces, dependencies, requests | where operation_Id == '<traceparent-trace-id>'`
    returns the span, with sampling applied and no `SENSITIVE_FIELDS`
    attribute present. Recorded in the EXPORT runbook as the release check.
- **Confidence:** medium — the sink is decided (ADR-059) and V1–V4 are
  deterministic CI tests with no live-Azure dependency; the only external
  variable is provisioning a staging Application Insights resource for the
  V5 manual ingest check.
- **Status:** Draft

## Open questions

- **OQ1 (from Planning Council session plan-b00c16c7) — RESOLVED
  2026-05-30 by [ADR-059](../decisions/059-production-tracing-sink.md):**
  Production sink choice is **Azure Monitor + Application Insights** (the
  original candidates were Tempo / Honeycomb / Grafana Cloud / self-hosted
  Jaeger / OTLP-to-Vercel-OTel). The founder ratified the sink ahead of
  the original "first paying customer / production incident" trigger, so
  the choice is no longer the gate. The module stays Draft only on
  **execution timing** — wiring the exporter — not on any open design
  question.

## Risks

- **Risk:** EXPORT staying Draft indefinitely silently leaves the
  tracing pipe ephemeral-only. This is acceptable per the three-pipe
  rule (tracing is not source-of-truth) and is documented in TRACE's
  Known Gaps section. The risk is only material if a downstream
  consumer starts treating spans as durable; the ADR-035 matrix is
  the prevention.
