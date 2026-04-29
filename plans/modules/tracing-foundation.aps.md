<!--
APS Module: Tracing Foundation
==============================
Cross-cutting tracing baseline across anvil-intercept (Rust daemon),
anvil-cli (Rust), anvil-api (TS), and the dashboard ops surface. Owns its
own work items but coordinates with INTD, RTAI, and (post-launch) the
dashboard live feed.

Cross-cutting convention: see plans/aps-rules.md#cross-cutting-modules.
-->

# Tracing Foundation

| ID    | Owner      | Status | Progress |
| ----- | ---------- | ------ | -------- |
| TRACE | @eddacraft | Ready  | 0/3      |

**Last reviewed:** 2026-04-30

> **Provenance:** Architecture decision record
> [ADR-034](../decisions/034-cross-cutting-modules-as-aps-primitive.md)
> (2026-04-30) split observability into three modules (OBS / TRACE / EXPORT)
> and promoted the cross-cutting module convention to a first-class APS
> primitive. This module is the **second trial** of that convention. See
> [ADR-035](../decisions/035-three-pipe-observability-rule.md).

## Cross-cutting convention

This module is the **second trial** of the cross-cutting module convention.
It does **not** restate the convention inline — the normative spec lives in
[`plans/aps-rules.md#cross-cutting-modules`][rules] and is cited by anchor
link wherever a callout is used in a task body.

[rules]: ../aps-rules.md#cross-cutting-modules

The first trial (`launch-flow-readiness`, LAUNCH) ring-fenced its convention
to itself until a second author was tempted to copy. TRACE is that second
author; rather than copy LAUNCH's block, the convention has been promoted
to `aps-rules.md` per ADR-034. As LAUNCH archives, learnings from its close
sweeps flow back into `aps-rules.md` (in particular, the still-provisional
`Blocks on:` clause hardens once exercised through a real close).

> **Anti-drift hook (per ADR-034):** changes to the
> `## Cross-Cutting Modules` section of `aps-rules.md` update this module's
> header reference and `launch-flow-readiness.aps.md`'s header reference in
> the same PR.

## Purpose

Give Anvil a runtime tracing baseline that:

- Lets Rust binaries (`anvil-intercept`, `anvil-cli`) and the TS API
  (`anvil-api`) emit structured spans with W3C Trace Context propagation.
- Stops every binary inventing its own subscriber init.
- Pins tracing as **debugging context, not source-of-truth** (per ADR-035)
  so future modules know which observability pipe to use.
- Turns ADR-019's `anvil.flags.*` precedent into a registry-based
  contribution model without designing every domain's attributes for them.

The narrow launch-blocker scope is **TRACE-001 only**. Everything else
(TS-side mirror, redaction hardening, EXPORT sink choice, the OBS module's
domain ops work) is post-launch.

## In scope

- A new `anvil-observability` Rust crate housing the `TraceContext` type,
  W3C `traceparent` parse/generate, propagation helpers, subscriber init
  via `init_tracing(BinaryKind)`, JSON formatter, redaction layer, and
  EnvFilter defaults.
- Threading `traceparent` through the JSON-RPC envelope, the HTTP header
  surface, and the notification envelope per ADR-035.
- A namespace registry stub that domain modules append to when they
  contribute their `anvil.<domain>.*` attributes, using ADR-019's
  `anvil.flags.*` convention as the first registered precedent.
- An ADR ratifying the three-pipe observability rule (ADR-035, landed as
  part of TRACE-001's PR).
- Updating INTD-014's JSON-RPC conformance fixture so `traceparent`
  round-trips through the envelope.

## Out of scope

- `@anvil/observability` TypeScript mirror for `anvil-api` and the
  dashboard parser (deferred to TRACE-002).
- Production redaction-layer hardening across binary boundaries (deferred
  to TRACE-003; revisit gates DA-OBS-004 risk acceptance).
- EXPORT module's sink choice — Tempo / Honeycomb / Grafana Cloud /
  self-hosted Jaeger / OTLP-to-Vercel-OTel — deferred to
  [observability-export](./observability-export.aps.md). EXPORT stays
  Draft until a paying customer or production incident forces the
  decision.
- OBS-001..005 (signal inventory, Neon health, dashboard real-time data
  contract, alert thresholds, runbook pack). Those remain in
  [observability-foundation](./observability-foundation.aps.md), Draft and
  deferred to post-launch.
- The `observability-triage.md` runbook. Belongs to OBS-005, not TRACE-001.
- Any OTEL SDK dependency. v1 ships structured logs + correlation-ID + W3C
  `traceparent` only; OTEL exporter wiring is EXPORT's call when EXPORT
  comes off Draft.

## Interfaces

**Depends on:**

- INTD-014 (JSON-RPC conformance, Committed) — TRACE-001 adds an assertion
  to its conformance fixture (`crates/anvil-intercept/tests/jsonrpc_conformance.rs`);
  the fixture itself is the integration point.
- ADR-019 (flags observability alignment) — TRACE-001 keeps tracing naming
  and emitted observability fields aligned with the repository's approved
  observability/flags conventions.
- ADR-035 (three-pipe rule) — the tracing-pipe role TRACE codifies.

**Coordinates with:**

- RTAI module — RTAI proceeds with provisional `anvil.rtai.*` and is
  **not** blocked on TRACE. TRACE-001 ships in the same release wave and
  ratifies the namespace through the registry stub.
- INTD-013 (mirror-decisions, Committed) — the redaction risk note in
  ADR-035 stems from `notification.context`; TRACE-003 is the
  cross-binary mitigation for the tracing pipe.
- INTD-015 (queued redaction hardening) — when INTD-015 reaches Ready or
  the first secret-detection rule ships, revisit DA-OBS-004 risk; TRACE-003
  acts on the tracing-pipe side of that mitigation.
- TUIDASH / dashboard-ops-views — they consume `traceparent` from the
  notification envelope **after** TRACE-002 lands the TS-side parser.

**Exposes:**

- `anvil-observability` Rust crate: `TraceContext`, `init_tracing`, the
  redaction layer, the JSON formatter, EnvFilter defaults.
- `docs/observability/namespace-registry.md` — the per-namespace
  contribution doc with founder-reviewed PR-to-add instructions.
- The `traceparent` round-trip assertion in
  `crates/anvil-intercept/tests/jsonrpc_conformance.rs` (the existing INTD-014
  fixture file; TRACE-001 adds one assertion to it).

## Ready Checklist

This module is **Ready** when:

- [x] Scope and ownership confirmed (Planning Council session
      plan-b00c16c7, 2026-04-30; owner @eddacraft).
- [x] Three-pipe rule ratified (ADR-035).
- [x] Cross-cutting convention promoted to `aps-rules.md` (ADR-034).
- [x] **Precondition (resolved 2026-04-30):** LAUNCH-003's open
      `Coordinates with: TUIDASH-009` callout was swept per ADR-034 rule 3.
      LAUNCH-003 shipped first; the conditional "Superseded by" branch did
      not fire and is closed. The named `WatchStats` contract remains the
      inheritance TUIDASH-009 will consume when the dashboard surface lands.
      This was the first real exercise of rule 3 against a live cross-
      reference, satisfying the "tried in anger" bar adversarial-reviewer
      raised during planning council session plan-b00c16c7.

## Tasks

> Status: Ready (LAUNCH-003 callout sweep closed 2026-04-30). TRACE-001 is
> launch-blocker scope; TRACE-002 and TRACE-003 are post-launch hardening and
> stay Draft until picked up.

### TRACE-001: Tracing baseline crate, propagation, and namespace registry

- **Intent:** Anvil's Rust binaries share one tracing crate, propagate
  `traceparent` across the three observability surfaces, and publish a
  namespace registry domain modules can append to.
- **Expected Outcome:** A new `anvil-observability` Rust crate exists with
  a `TraceContext` type, W3C `traceparent` parse/generate helpers,
  propagation utilities for the JSON-RPC envelope, the HTTP header
  surface, and the notification envelope, a `init_tracing(BinaryKind)`
  subscriber initialiser, a JSON formatter, a redaction layer, and
  EnvFilter defaults. Each binary's `main()` (`anvil-cli`, every
  `anvil-intercept` binary entrypoint) calls `init_tracing` once. Library
  crates emit spans but never initialise the global subscriber. ADR-035
  is published in the same PR. A namespace registry stub exists at
  `docs/observability/namespace-registry.md` listing `anvil.flags.*`,
  `kindling.*`, `anvil.rtai.*` entries, the founder-reviewed PR-to-add
  instruction, and a **Known Gaps** subsection that reads: *"Day-one
  limitation: dashboard cannot join traces across producers until
  TS-side `traceparent` parsing lands (tracked under TRACE-002)."* The
  INTD-014 JSON-RPC conformance fixture gains one assertion that
  `traceparent` round-trips through the envelope unchanged.
- **Coordinates with:** INTD-014 (the conformance fixture is the
  integration point — TRACE-001 adds one round-trip assertion, does not
  re-shape the fixture).
- **Coordinates with:** RTAI module — the registry's `anvil.rtai.*`
  entry ratifies the namespace RTAI is already using provisionally.
- **Files (best-effort):** `crates/anvil-observability/` (new),
  `crates/anvil-cli/src/main.rs`, `crates/anvil-intercept/src/main.rs`,
  `crates/anvil-intercept/tests/jsonrpc_conformance.rs` (existing INTD-014
  fixture; TRACE-001 adds one assertion), `apps/anvil-api/` (no init —
  TS-side mirror is TRACE-002),
  `docs/observability/namespace-registry.md`,
  `plans/decisions/035-three-pipe-observability-rule.md`.
- **Validation:**
  `cargo test -p anvil-observability && rg -n "traceparent" crates/anvil-intercept/tests/jsonrpc_conformance.rs`
- **Confidence:** medium
- **Status:** Draft

---

### TRACE-002: TypeScript `@anvil/observability` mirror

- **Intent:** `anvil-api` and the dashboard parse `traceparent` and join
  traces across producers without re-implementing the propagation logic.
- **Expected Outcome:** A `@anvil/observability` TS package mirrors the
  Rust crate's parse/generate helpers and propagation surface, exposes a
  consumer for the JSON-RPC envelope and the notification envelope, and
  is wired into `anvil-api`'s request entry path and the dashboard's
  notification consumer. The Known Gaps note in the namespace registry
  closes when this lands.
- **Coordinates with:** dashboard-ops-views (the live feed consumer is
  the first non-Rust reader of `traceparent`).
- **Coordinates with:** TRACE-003 — TS-side consumer inherits the
  DA-OBS-004 redaction gap; treat `notification.context` as potentially
  secret-bearing until INTD-015 and TRACE-003 both land.
- **Validation:** TBD when picked up — at minimum a TS unit test that
  parses a Rust-emitted `traceparent` and a smoke test that the
  dashboard renders the joined view.
- **Confidence:** medium
- **Status:** Draft

---

### TRACE-003: Redaction-layer hardening across binary boundaries

- **Intent:** Close the DA-OBS-004 risk-accepted gap on the tracing pipe
  so secret-bearing fields cannot transit spans even before the first
  secret-detection rule ships.
- **Concrete failure mode (DA-OBS-004):** A `notification.context` payload
  carrying a secret can transit an unredacted span attribute to any
  tracing subscriber or sampled exporter if a secret-detection rule fires
  before INTD-015 lands.
- **Expected Outcome:** `anvil-observability`'s redaction layer carries
  a default deny-list for known sensitive field names, integrates with
  the (future) shared redaction policy used by the notification
  envelope, and refuses to forward sensitive content to any sampled
  exporter selected by EXPORT. Documented in the namespace registry's
  Known Gaps section as the closed-out side of the DA-OBS-004 risk.
- **Coordinates with:** INTD-015 (queued redaction hardening on the
  notification envelope).
- **Validation:** TBD when picked up — at minimum a unit test that a
  span attribute matching the deny-list is replaced with a redaction
  marker before formatting.
- **Confidence:** low — depends on INTD-015's shape and EXPORT's sink.
- **Status:** Draft

## Risks

- **R1 (accepted pre-launch, see ADR-035):** A `notification.context`
  payload carrying a secret can transit an unredacted span attribute to
  any tracing subscriber or sampled exporter if a secret-detection rule
  fires before INTD-015 lands. Revisit when INTD-015 reaches Ready OR the
  first secret-detection rule ships, whichever first. TRACE-003 is the
  tracing-pipe side of the mitigation.
- **R2:** `anvil.<domain>.*` namespace fragmentation if multiple modules
  contribute attributes with conflicting shapes (units, plurals, naming
  case). Mitigation: namespace registry doc + founder PR review gate.
- **R3:** TRACE-002 deferred → dashboard cannot join traces across
  producers on day one. Mitigation: documented in Known Gaps section of
  the namespace registry.
- **R4:** `Blocks on:` callout type promoted into `aps-rules.md`
  speculatively (it has not yet been exercised through a real close).
  Mitigation: ADR-034 carries an explicit follow-up contract — when a
  TRACE task with an open `Blocks on:` callout reaches Complete and the
  closer sweeps it, the spec's "provisional" flag is removed in the same
  edit.

## Open questions

- **OQ1 (deferred to EXPORT):** Production sink choice — Tempo /
  Honeycomb / Grafana Cloud / self-hosted Jaeger / OTLP-to-Vercel-OTel
  — to be decided when first paying customer or first production
  incident motivates it. Tracked in
  [observability-export](./observability-export.aps.md). EXPORT stays
  Draft until then.
