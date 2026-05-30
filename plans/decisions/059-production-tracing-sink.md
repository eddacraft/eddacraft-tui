# ADR-059: Production tracing sink — OTLP-neutral export to a managed backend

## Status

Proposed

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
"motivate the sink choice." That trigger is a business gate, but it has also
left the underlying architecture undecided: EXPORT-001 currently has no
validation defined and a NEW-ADR requirement baked into its expected outcome.
This ADR resolves the **architecture and constraints** of the sink so EXPORT-001
becomes specifiable, and recommends a vendor while marking the vendor selection
itself as the one remaining ratification point (a cost / contract decision owned
by the founder).

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
   sits behind Azure Front Door ([ADR-032](032-edge-architecture-afd.md)).

Once that split is explicit, the sink choice is much smaller than "pick one of
five vendors": it is "pick an export path and backend for the operator-hosted
surfaces, keep the local-first surfaces local, and stay vendor-neutral at the
instrumentation layer so the backend is swappable."

## Decision

Adopt an **OTLP-neutral export architecture** with four parts:

1. **Instrumentation stays vendor-neutral OTLP.** Spans are emitted over OTLP
   (the TRACE baseline already speaks it). No vendor SDK is linked into either
   the Rust crates or the TS packages. The exporter endpoint, headers, and
   sampling are configuration, not code. This keeps the backend swappable and
   makes the vendor choice below reversible without touching instrumentation.

2. **Only operator-hosted surfaces export.** The production exporter is wired
   into `apps/anvil-api` (and any future eddacraft-operated Rust service),
   behind config that is **off by default**. The end-user Rust CLI / daemon
   keep the formatter + local file sink and never auto-export. An opt-in OTLP
   exporter for self-hosted operators may follow, but is not part of the default
   path and not this decision.

3. **Ingestion path for the Vercel-hosted API is the Vercel OTEL drain**
   (`@vercel/otel` / the OTLP collector Vercel exposes), forwarding to the
   backend below. Vercel's own observability retention is too short and its
   trace query surface too limited to be the terminal store for a
   trust-relevant diagnostic pipe, so the Vercel OTEL drain is treated as a
   *collector*, not a *sink*.

4. **Backend (the actual sink): a managed, OTLP-native trace backend.**
   **Recommended: Honeycomb** as the initial backend — OTLP-native ingest, a
   trace-query surface (BubbleUp) that is materially better than Tempo/Jaeger
   for the "why was this request slow / wrong" question this pipe exists to
   answer, a free tier that covers low-volume hosted-API traffic, and near-zero
   operational burden. **Documented fallback: Grafana Cloud (managed Tempo)** if
   consolidating traces with logs and metrics under one vendor becomes the
   priority. Self-hosting (Tempo or Jaeger) is rejected for the default path:
   there is no team to operate an observability cluster, and the storage backend
   (object store / Cassandra / Elasticsearch) is ops weight the three-pipe rule
   says a *debugging* pipe does not justify.

Cross-cutting constraints that bind whatever backend is ratified:

- **The TRACE-003 redaction layer wraps the exporter.** The sampled output MUST
  honour the `SENSITIVE_FIELDS` deny-list; the exporter must not be a redaction
  bypass.
- **Sampling is conservative and config-driven** — tail-based at the collector
  where available, otherwise a low-rate head sample — sized to a stated cost
  budget, not "export everything."
- **Credentials flow through the existing Pulumi-managed secrets path**, never
  inlined.
- **Local development stays formatter-only** even for the hosted-API code, so a
  developer running `apps/anvil-api` locally does not ship traces anywhere.

The one item left for founder ratification is the **vendor** in part 4
(Honeycomb vs Grafana Cloud vs another OTLP-native backend). Everything else —
OTLP-neutral instrumentation, hosted-only export, local-stays-local,
redaction-wrapped, config-gated-off-by-default — is decided here.

## Rationale

The candidate list mixed three different layers (instrumentation, collection,
storage) and one privacy boundary (local vs hosted) into a single five-way
"pick a vendor." Separating them collapses most of the apparent choice:

- Vendor-neutral OTLP at the instrumentation layer makes the storage vendor a
  late, cheap, reversible decision — which is exactly the posture a *non*
  source-of-truth pipe warrants (ADR-035: spans are breadcrumbs).
- The local-first boundary removes end-user trace export from scope entirely,
  which is both the right privacy answer and a large cost reduction — the only
  traffic that reaches the sink is the operator's own hosted API.
- A managed backend is the only option consistent with "no team to run an
  observability cluster." Between managed options, Honeycomb wins the *query*
  dimension the pipe exists for; Grafana Cloud wins the *consolidation*
  dimension, which is not yet a need.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **OTLP-neutral → managed backend (Honeycomb), hosted-only, local-stays-local** (chosen) | Swappable backend; best trace query for the cost; zero ops; respects local-first privacy; free tier covers current volume | A vendor relationship to manage; query value is soft lock-in (mitigated by OTLP-in) |
| Grafana Cloud (managed Tempo) | One vendor for traces+logs+metrics; OTLP-native; free tier | Weaker trace search than Honeycomb; consolidation not yet a need; more moving parts |
| Self-hosted Tempo or Jaeger | No vendor; full control | Real ops burden + a storage backend to run; contradicts "no observability team"; over-weight for a debugging pipe |
| Vercel OTEL drain as the terminal sink | Already on Vercel; minimal wiring | Retention too short, query surface too limited to be the store of record for diagnostics; better used as the collector/drain |
| Export end-user CLI/daemon traces too | "Full" visibility | Privacy violation for a local-first tool; large cost; out of scope for a debugging pipe |

## Consequences

- **Positive:** EXPORT-001 becomes specifiable — its validation can now be
  written (an integration test that an emitted span reaches the configured
  backend with sampling applied, plus a redaction-deny-list assertion on the
  sampled output). The backend stays swappable. End-user privacy is preserved by
  construction. Cost is bounded to operator-hosted traffic.
- **Negative:** A vendor relationship (Honeycomb or Grafana Cloud) must be
  provisioned and its credentials managed. The "query value" of the chosen
  backend is a soft lock-in even though ingest is neutral.
- **Risks:** Sampling mis-sized against the free-tier ceiling could either drop
  useful traces or incur cost; the exporter could regress the redaction layer if
  wired around it rather than through it.
- **Mitigations:** Config-gated, off by default, conservative sampling tied to a
  stated budget; the redaction layer wraps the exporter and is covered by the
  EXPORT-001 validation; OTLP-neutral instrumentation means a vendor switch is a
  config change, not a re-instrumentation.

## References

- Related ADRs: [ADR-035](035-three-pipe-observability-rule.md) (three-pipe
  rule — pins this pipe as never-source-of-truth),
  [ADR-032](032-edge-architecture-afd.md) (Azure Front Door edge),
  [ADR-019](019-flags-observability-alignment.md) (OTEL metrics vs Kindling
  rows)
- APS modules: EXPORT-001 (resolves OQ1), TRACE-001 / TRACE-002 / TRACE-003
  (subscriber init, TS wiring, redaction layer this exporter must honour)
- External: OpenTelemetry OTLP exporter spec; `@vercel/otel`; Honeycomb and
  Grafana Cloud OTLP ingest documentation
