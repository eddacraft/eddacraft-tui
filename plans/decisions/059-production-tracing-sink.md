# ADR-059: Production tracing sink — OTLP-neutral export to Azure Monitor + Application Insights

## Status

Accepted. A **partial supersession is proposed** by
[ADR-124](124-operator-plane-cloud-placement.md) (2026-08-17): §Decision-3
(Azure Monitor OTel exporter) and §Decision-4 (App Insights + Grafana) are
reopened because this ADR's vendor rationale rests on Azure holding the
billing/credits relationship, which Cloudflare and AWS credits have since made
non-exclusive. **§Decision-1 (vendor-neutral OTLP instrumentation) and
§Decision-2 (only operator-hosted surfaces export; the local-first Rust
CLI/daemon never auto-export) are preserved and re-affirmed by ADR-124**, and
are not reopened. This ADR remains in force in full until ADR-124 is Accepted.

## Date

2026-05-30

## Context

[ADR-035](035-three-pipe-observability-rule.md) established the three-pipe rule:
Kindling owns durable governance facts, the notification envelope owns
user-visible state, and the tracing / OTEL pipe is **ephemeral debugging
context that is never source-of-truth**. The
[tracing-foundation](../modules/tracing-foundation.aps.md) module (TRACE) wired
the baseline subscriber, `traceparent` correlation, and a redaction layer
(TRACE-003), and explicitly deferred the production sink to the
[observability-export](../modules/observability-export.aps.md) module (EXPORT).

EXPORT-001's single open question (OQ1, from Planning Council session
plan-b00c16c7) is the sink choice itself, surfaced as five candidates with no
decision criteria recorded:

> Tempo · Honeycomb · Grafana Cloud · self-hosted Jaeger · OTLP-to-Vercel-OTel

EXPORT has stayed Draft pending a paying customer or production incident to
"motivate the sink choice." That trigger is a business gate, but it had also
left the underlying architecture undecided: EXPORT-001 had no validation defined
and a NEW-ADR requirement baked into its expected outcome. This ADR resolves the
**architecture and constraints** of the sink, and records the founder's vendor
ratification: **Azure Monitor + Application Insights**, with Grafana for
dashboards.

The decisive force the candidate list obscures: **Anvil emits traces from two
fundamentally different places.**

1. **The Rust CLI and intercept daemon run on end-user machines.** Anvil is
   planless-first and local-first; these surfaces already use a
   formatter-only subscriber plus a local hardened file sink (TRACE-001 /
   TRACE-004). Auto-shipping an end user's local traces to a vendor backend
   would be a privacy violation and is out of scope for a debugging pipe.
2. **The hosted TypeScript API (`apps/anvil-api`, on Vercel) is
   operator-controlled.** This is where a production sink earns its keep: it is
   eddacraft-run, its traces are eddacraft's own operational data, and its edge
   already sits behind Azure Front Door ([ADR-032](032-edge-architecture-afd.md)).

Once that split is explicit, the sink choice is much smaller than "pick one of
five vendors": it is "pick an export path and backend for the operator-hosted
surfaces, keep the local-first surfaces local, and stay vendor-neutral at the
instrumentation layer so the backend is swappable."

## Decision

Adopt an **OTLP-neutral export architecture** terminating at **Azure Monitor +
Application Insights**, in four parts:

1. **Instrumentation stays vendor-neutral OTLP/OTel.** Spans are emitted via the
   OpenTelemetry API (the TRACE baseline already speaks it). No vendor-proprietary
   tracing SDK is linked into application code. The exporter, endpoint, headers,
   and sampling are configuration, not instrumentation — so the backend remains
   swappable even though the chosen exporter (below) is Azure-specific.

2. **Only operator-hosted surfaces export.** The production exporter is wired
   into `apps/anvil-api` (and any future eddacraft-operated service), behind
   config that is **off by default**. The end-user Rust CLI / daemon keep the
   formatter + local file sink and never auto-export. An opt-in exporter for
   self-hosted operators may follow, but is not part of the default path and not
   this decision.

3. **Ingestion path: the Azure Monitor OpenTelemetry exporter.** `apps/anvil-api`
   exports OTel spans to Application Insights via the Azure Monitor OTel exporter
   (directly, or through an OpenTelemetry Collector carrying the Azure Monitor
   exporter if a collection hop is wanted). The Vercel OTEL drain (`@vercel/otel`)
   may sit in front as a collection convenience, but it is a *collector*, not the
   sink — Vercel's own observability retention and query surface are too limited
   to be the store of record for a diagnostic pipe.

4. **Backend (the sink): Azure Monitor + Application Insights, dashboards on
   Grafana.** App Insights is the trace store and the trace-debugging surface
   (end-to-end transaction details, application map, dependency tracking, KQL
   over traces). Dashboards are **hand-rolled first** — KQL queries surfaced
   through an Azure Workbook or a small custom view — with **Azure Managed
   Grafana** (or self-managed Grafana over the Azure Monitor data source) adopted
   later, once dashboard needs justify its per-active-user cost. This keeps the
   launch surface cheap and planless-first.

Cross-cutting constraints:

- **The TRACE-003 redaction layer wraps the exporter.** The sampled output MUST
  honour the `SENSITIVE_FIELDS` deny-list; the exporter must not be a redaction
  bypass.
- **Sampling is conservative and config-driven** — tail-based at a collector
  where available, otherwise a low-rate head sample — sized to a stated cost
  budget, not "export everything."
- **Credentials (the App Insights connection string) flow through the existing
  Pulumi-managed secrets path**, never inlined.
- **Local development stays formatter-only** even for the hosted-API code, so a
  developer running `apps/anvil-api` locally does not ship traces anywhere.
- **Alerting uses Azure Monitor alert rules** (billed per rule, no hard cap),
  added only as real needs appear — there is no fixed-trigger ceiling to design
  around.
- **Feature-flagged usage analytics is NOT this pipe.** Per ADR-035 usage facts
  are governance-shaped and durable, so they live on **Kindling** (the
  [usage-analytics](../archive/modules/usage-analytics.aps.md) module is the source of
  record). App Insights `customEvents` MAY carry usage *breadcrumbs* to feed a
  convenience dashboard, but Kindling stays the system of record — the dashboard
  must never become the store.

## Rationale

The candidate list mixed three layers (instrumentation, collection, storage) and
one privacy boundary (local vs hosted) into a single five-way "pick a vendor."
Separating them collapses most of the choice; the remaining vendor call was made
on **stack consolidation**:

- Vendor-neutral OTel at the instrumentation layer makes the storage vendor a
  late, reversible decision — the right posture for a *non* source-of-truth pipe
  (ADR-035: spans are breadcrumbs).
- The local-first boundary removes end-user trace export from scope entirely —
  the right privacy answer and a large cost reduction (only the operator's hosted
  API reaches the sink).
- **Why Azure over Honeycomb / Grafana Cloud:** eddacraft is already on Azure —
  Front Door edge (ADR-032), Azure DNS, Entra identity, Azure billing/credits.
  Azure Monitor + App Insights + Grafana is one consolidated stack covering
  traces, metrics, logs, dashboards, and alerting, with no extra vendor
  relationship and no per-feature trigger ceiling (Honeycomb's free tier caps
  Triggers at ~2). App Insights' trace debugging is a step below Honeycomb's
  high-cardinality "BubbleUp" exploration — that gap is the accepted trade. For a
  small team, on a *non*-source-of-truth debugging pipe, consolidation +
  uncapped alerting + Azure-native identity/billing outweigh best-in-class trace
  querying.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Azure Monitor + App Insights (store/debug), Grafana dashboards, hand-rolled first** (chosen) | One consolidated Azure stack (traces+metrics+logs+dashboards+alerting); Entra identity + Azure billing/credits; uncapped alert rules; no new vendor; dashboards deferrable to control cost | Trace-debugging exploration below Honeycomb; ingest via the Azure Monitor exporter (not a bare OTLP endpoint); the hosted workload is on Vercel, so it's cross-cloud egress until/unless compute moves to Azure |
| Honeycomb | Best-in-class high-cardinality trace debugging (BubbleUp); OTLP-native; free tier | Separate vendor; free tier caps Triggers at ~2; soft query lock-in; no metrics/logs/dashboard consolidation |
| Grafana Cloud (managed Tempo) | Traces+logs+metrics one vendor; OTLP-native; free tier | Another vendor alongside Azure; weaker trace search than Honeycomb; no Azure-native identity/billing benefit |
| Self-hosted Tempo or Jaeger | No vendor; full control | Real ops burden + a storage backend to run; contradicts "no observability team"; over-weight for a debugging pipe |
| Vercel OTEL drain as the terminal sink | Already on Vercel; minimal wiring | Retention too short, query surface too limited to be the store of record; better used as a collector in front of App Insights |
| Export end-user CLI/daemon traces too | "Full" visibility | Privacy violation for a local-first tool; large cost; out of scope for a debugging pipe |

## Consequences

- **Positive:** EXPORT-001 becomes specifiable — validation can now be written
  (an integration test that an emitted span reaches Application Insights with
  sampling applied, plus a redaction-deny-list assertion on the sampled output).
  Observability consolidates on one stack the team already runs. Alerting has no
  trigger ceiling. End-user privacy is preserved by construction; cost is bounded
  to operator-hosted traffic and the launch dashboard is hand-rolled (no AMG cost
  up front).
- **Negative:** App Insights' trace-debugging exploration is weaker than
  Honeycomb's. The Azure Monitor exporter is Azure-specific, so a future backend
  switch is an exporter change (instrumentation stays OTel, but it is not a pure
  endpoint-URL swap). The traced workload is on Vercel, so traces egress
  cross-cloud to Azure until compute consolidates.
- **Risks:** Sampling mis-sized could drop useful traces or incur cost; the
  exporter could regress the redaction layer if wired around it rather than
  through it; usage analytics could drift onto App Insights and bypass Kindling
  as the record.
- **Mitigations:** Config-gated, off by default, conservative budgeted sampling;
  the redaction layer wraps the exporter and is covered by EXPORT-001 validation;
  the usage-of-record boundary (Kindling, breadcrumbs only on App Insights) is
  stated above and owned by the USAGE module.

## References

- Related ADRs: [ADR-035](035-three-pipe-observability-rule.md) (three-pipe
  rule — pins this pipe as never-source-of-truth),
  [ADR-032](032-edge-architecture-afd.md) (Azure Front Door edge — the existing
  Azure footprint this consolidates onto),
  [ADR-019](019-flags-observability-alignment.md) (OTEL metrics vs Kindling rows)
- APS modules: EXPORT-001 (resolves OQ1), TRACE-001 / TRACE-002 / TRACE-003
  (subscriber init, TS wiring, redaction layer this exporter must honour),
  [USAGE](../archive/modules/usage-analytics.aps.md) (feature-flagged usage is
  Kindling-of-record, not this pipe)
- External: OpenTelemetry OTLP exporter spec; Azure Monitor OpenTelemetry
  exporter / Application Insights OTel ingestion; Azure Managed Grafana + the
  Azure Monitor Grafana data source; `@vercel/otel`
