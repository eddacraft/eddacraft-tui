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

> Status: Draft. EXPORT remains Draft until a paying customer or first
> production incident motivates the sink choice. Pre-launch this module
> is a placeholder that captures OQ1; post-launch the founder picks it
> up when triggered.

### EXPORT-001: Choose, ratify, and wire the production tracing sink

- **Intent:** Anvil's tracing pipe has one chosen production sink with
  documented sampling, retention, and exporter wiring across both Rust
  binaries and the TS API.
- **Expected Outcome:** A new ADR records the sink choice and the
  trade-offs it accepts (cost, vendor lock-in, sampling shape,
  ingestion shape). `anvil-observability` and `@anvil/observability`
  carry the corresponding exporter wiring, gated behind config so local
  development still uses the formatter-only subscriber. The sink's
  credentials story is wired through the existing secrets path
  (Pulumi-managed where appropriate). Sampling defaults match the
  cost budget the sink choice agreed to.
- **Coordinates with:** TRACE-001 (subscriber init is the integration
  point), TRACE-002 (TS-side wiring), TRACE-003 (redaction layer must
  not be bypassed by the chosen exporter).
- **Validation:** TBD when picked up — at minimum an integration test
  that the chosen sink receives an emitted span end-to-end with the
  configured sampling, and a manual verification that the redaction
  layer's deny-list is honoured by the sampled output.
- **Confidence:** low — entire scope is a sink choice that has not been
  made.
- **Status:** Draft

## Open questions

- **OQ1 (verbatim from Planning Council session plan-b00c16c7):**
  Production sink choice — Tempo / Honeycomb / Grafana Cloud /
  self-hosted Jaeger / OTLP-to-Vercel-OTel — to be decided when first
  paying customer or first production incident motivates it. EXPORT
  module stays Draft until then.

## Risks

- **Risk:** EXPORT staying Draft indefinitely silently leaves the
  tracing pipe ephemeral-only. This is acceptable per the three-pipe
  rule (tracing is not source-of-truth) and is documented in TRACE's
  Known Gaps section. The risk is only material if a downstream
  consumer starts treating spans as durable; the ADR-035 matrix is
  the prevention.
