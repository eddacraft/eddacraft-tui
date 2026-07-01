<!--
APS Module: Tracing Foundation
==============================
Cross-cutting tracing baseline across anvil-intercept (Rust daemon),
anvil-cli (Rust), anvil-api (TS), and the dashboard ops surface. Owns its
own work items but coordinates with INTD, RTAI, and (post-launch) the
dashboard live feed.

Cross-cutting convention: see plans/aps-rules.md#module-types-vertical-and-conductor.
-->

# Tracing Foundation

| ID    | Owner      | Status      | Progress |
| ----- | ---------- | ----------- | -------- |
| TRACE | @eddacraft | In Progress | 2/4      |

**Last reviewed:** 2026-05-11

> **Provenance:** Architecture decision record
> [ADR-034](../decisions/034-cross-cutting-modules-as-aps-primitive.md)
> (2026-04-30) split observability into three modules (OBS / TRACE / EXPORT)
> and promoted the cross-cutting module convention to a first-class APS
> primitive. This module is the **second trial** of that convention. See
> [ADR-035](../decisions/035-three-pipe-observability-rule.md).

## Cross-cutting convention

This module is the **second trial** of the cross-cutting module convention.
It does **not** restate the convention inline — the normative spec lives in
[`plans/aps-rules.md#module-types-vertical-and-conductor`][rules] and is cited by anchor
link wherever a callout is used in a task body.

[rules]: ../aps-rules.md#module-types-vertical-and-conductor

The first trial (`launch-flow-readiness`, LAUNCH) ring-fenced its convention
to itself until a second author was tempted to copy. TRACE is that second
author; rather than copy LAUNCH's block, the convention has been promoted
to `aps-rules.md` per ADR-034. As LAUNCH archives, learnings from its close
sweeps flow back into `aps-rules.md` (in particular, the still-provisional
`Blocks on:` clause hardens once exercised through a real close).

> **Anti-drift hook (per ADR-034):** changes to the
> `## Module Types: Vertical and Conductor` section of `aps-rules.md` update this module's
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

The launch-blocker scope is **TRACE-001 (shipped) + TRACE-004 (shipped,
2026-05-11)**. TRACE-004 landed call-path instrumentation,
`traceparent`-to-span binding, and a local-only dev sink so developers can
debug a request through the daemon end-to-end before the first external user.
Everything else (TS-side mirror, redaction hardening, EXPORT sink choice, the
OBS module's domain ops work) is post-launch.

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
- USAGE module — TRACE-004's span-binding helper is the source of the
  `traceparent` USAGE-001 stamps on every Kindling row. The two land in
  either order, but both must be in place before the first end-to-end
  "trace ↔ usage" join works.

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

## Work Items

> Status: In Progress. TRACE-001 Complete 2026-04-30. TRACE-004 Complete
> 2026-05-11 via PR #1435.
> TRACE-002 authorised by operator request on 2026-05-25. A partial slice
> landed the reusable TS mirror and `anvil-api` ingress, then blocked on
> concrete dashboard live-feed consumer ownership. TRACE-003 also received
> a partial Rust tracing-formatter redaction slice. As of 2026-06-24, INTD-015 is
> Complete and ADR-059 has decided the sink, so the redaction-parity slice is
> actionable; only sampled-exporter behaviour remains blocked on EXPORT-001's
> deferred exporter wiring.

### TRACE-001: Tracing baseline crate, propagation, and namespace registry

> **Status update (2026-04-30):** Complete. `anvil-observability` crate
> shipped with `TraceContext` (W3C `traceparent` v00 parse/generate),
> `BinaryKind`, `init_tracing`, JSON formatter, and an advisory-only
> redaction deny-list (TRACE-003 wires the layer). Both binary
> entrypoints call `init_tracing` once and surface install errors to
> stderr. The JSON-RPC envelope validates `traceparent` on every
> request and round-trips it on every response (success and error);
> the INTD-014 conformance fixture pins the contract with two new
> assertions. Council session `council-666d6e65` converged
> (4 critical, 10 major, 14 minor, 0 nit; 22 fixed, 4 deferred,
> 1 dismissed). ADR-035 + namespace registry doc updated with
> known-gaps callouts so operators are not misled about redaction
> enforcement.

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
  `cargo test -p eddacraft-anvil-observability -p eddacraft-anvil-intercept && rg -n "traceparent" crates/anvil-intercept/tests/jsonrpc_conformance.rs`
- **Confidence:** medium
- **Status:** Done

---

### TRACE-002: TypeScript `@anvil/observability` mirror

> **Status update (2026-05-25):** Blocked after partial implementation.
> Operator request "complete as much of TRACE as you can before getting
> blocked" authorised lifting this post-launch item out of Draft. Completed
> slice: new `@eddacraft/anvil-observability` package with Rust-compatible
> `traceparent` parse/format/envelope helpers, plus `anvil-api` request ingress
> middleware. Remaining blocker: no concrete dashboard live-feed consumer
> surface exists in this repo to wire the notification join or smoke-test the
> joined dashboard view without fabricating a surface.

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
- **Validation:** Partial slice passed 2026-05-25:
  `pnpm --filter @eddacraft/anvil-observability build`,
  `pnpm --filter @eddacraft/anvil-observability test`,
  `pnpm --filter @eddacraft/anvil-observability typecheck`,
  `pnpm --filter @eddacraft/anvil-api test -- trace-context`, and
  `pnpm --filter @eddacraft/anvil-api typecheck`. Full task still needs a
  dashboard joined-view smoke test once that surface exists.
- **Confidence:** medium
- **Status:** Blocked

---

### TRACE-003: Redaction-layer hardening across binary boundaries

> **Status update (2026-05-25):** Blocked after partial implementation.
> Completed slice: Rust `init_tracing` now uses redacting JSON field and event
> formatters that replace sensitive attributes (`SENSITIVE_FIELDS`) with
> `<redacted>` before stderr or local file-sink output, including exact
> `notification.context` aliases. The namespace registry reflects the narrowed
> gap at that time: shared INTD-015 notification redaction policy and
> sampled-exporter refusal/handling under EXPORT's sink decision.
>
> **Blocker update (2026-06-24):** one of the two blockers has cleared.
> **INTD-015 is `Complete`** (merged 2026-05-06 via PR #1305 —
> `crates/anvil-intercept/src/fanout.rs` provides the daemon-side cross-session
> redaction policy this item must mirror), so the notification-redaction-parity
> slice of TRACE-003 is now **actionable**. The **EXPORT sink decision is also
> made** — [ADR-059](../decisions/059-production-tracing-sink.md) (Accepted
> 2026-05-30) selects Azure Monitor + Application Insights. The **only residual
> blocker** is therefore the *sampled-exporter behaviour* half, which needs a
> wired exporter to test against: that wiring is **EXPORT-001**, deliberately
> deferred by timing (not design) until a paying customer or first production
> incident. Net: the INTD-015-dependent redaction-parity work can proceed now;
> the sampled-exporter refusal test waits on EXPORT-001. Consider splitting
> TRACE-003 along that seam via `planning-workflow` if the parity slice is
> scheduled ahead of EXPORT.

- **Intent:** Close the DA-OBS-004 risk-accepted gap on the tracing pipe
  so secret-bearing fields cannot transit spans even before the first
  secret-detection rule ships.
- **Concrete failure mode (DA-OBS-004):** A `notification.context` payload
  carrying a secret could transit an unredacted span attribute to any tracing
  subscriber or sampled exporter. The INTD-015 daemon-side redaction policy has
  now landed; TRACE-003 must mirror it on the tracing pipe and still define the
  sampled-exporter refusal/handling behaviour once EXPORT-001 wires the exporter.
- **Expected Outcome:** `anvil-observability`'s redaction layer carries
  a default deny-list for known sensitive field names, integrates with
  the (future) shared redaction policy used by the notification
  envelope, and refuses to forward sensitive content to any sampled
  exporter selected by EXPORT. Documented in the namespace registry's
  Known Gaps section as the closed-out side of the DA-OBS-004 risk.
- **Coordinates with:** INTD-015 (**Complete** — merged 2026-05-06 via PR #1305;
  the daemon-side cross-session redaction policy now lives in
  [`../archive/modules/intercept-daemon.aps.md`](../archive/modules/intercept-daemon.aps.md)
  / `crates/anvil-intercept/src/fanout.rs`, so the parity contract this item must
  mirror is fixed, not pending).
- **Validation:** Partial slice passed 2026-05-25:
  `cargo test -p eddacraft-anvil-observability` and `cargo fmt --check`.
  The INTD-015 policy-parity validation is now actionable (the contract is
  Complete). Sampled-exporter validation still waits on a wired exporter
  (EXPORT-001, deferred by timing; sink decided in ADR-059).
- **Confidence:** low — the residual depends on EXPORT-001's deferred exporter
  wiring; the INTD-015-parity slice is now unblocked.
- **Status:** Blocked (residual: sampled-exporter behaviour via EXPORT-001; the
  INTD-015 redaction-parity slice is unblocked as of 2026-06-24)

---

### TRACE-004: Instrument call paths and bind incoming `traceparent` to span context — Complete

> **Status update (2026-05-11):** Complete via PR #1435. Shippable cut landed:
> `anvil-observability` exposes current-span and explicit-span binding helpers;
> JSON-RPC dispatch records valid incoming `traceparent` as correlation fields on
> handler spans; scan-buffer and CLI entry spans are instrumented;
> `ANVIL_TRACE_SINK=file=<path>` writes local JSON-line output with restrictive
> Unix create permissions and hardened existing-file checks; OTLP remains
> deferred to EXPORT to avoid SDK dependency churn. Fuller exporter-backed parent
> propagation / local collector walkthrough is explicitly deferred to EXPORT.

- **Intent:** Anvil's daemon and CLI emit spans on the call paths
  developers actually need to debug, and an incoming `traceparent`
  is recorded as correlation fields on the local dispatch span so a
  single trace ID joins the JSON-RPC envelope to the work it triggered.
- **Concrete failure mode (today):** A developer reproducing a JSON-RPC
  bug runs the daemon under `RUST_LOG=debug`, sees flat log lines with
  no span tree, and cannot distinguish concurrent requests beyond
  timing. The `traceparent` is parsed and echoed but never bound to the
  work the daemon does after parsing. Full OpenTelemetry parent propagation
  remains EXPORT scope.
- **Expected Outcome:**
  - `anvil-observability` exposes a
    `bind_traceparent_to_current_span(&TraceContext)` helper that
    records `trace_id`, `parent_id`, and `trace_flags` as fields on the
    enclosing `tracing::Span`, so subscriber output exposes a stable
    correlation key. It does not create an OpenTelemetry parent relationship;
    EXPORT owns exporter-backed propagation.
  - `#[instrument]` (or equivalent `info_span!`) on: the JSON-RPC
    dispatch loop, scan-buffer handlers, and CLI command entrypoints.
    Kernel work-surface breadth remains follow-up scope. Span attributes follow the
    [namespace registry](../../docs/observability/namespace-registry.md)
    conventions; new attributes added by this pass are recorded in
    the registry in the same PR.
  - A local-only dev sink behind `ANVIL_TRACE_SINK`: unset (default) =
    formatter-only as today; `=file=<path>` = JSON-line file sink.
    `=otlp[=<endpoint>]` and production sinks remain EXPORT's call.
  - `docs/observability/local-tracing.md` (new) — short developer-facing
    doc for local file tracing. Local Jaeger / OTLP collector walkthrough
    remains deferred to EXPORT.
  - The INTD-014 conformance fixture's `traceparent_round_trips_*`
    test extended (or a sibling test added) asserting that the
    daemon's handler span carries the matching `trace_id` /
    `parent_id` fields.
- **Coordinates with:** TRACE-001 — consumes `TraceContext` and the
  existing subscriber init; TRACE-001 added no instrumentation,
  TRACE-004 is the rest of the iceberg.
- **Coordinates with:** TRACE-002 — TS mirror inherits the span-binding
  contract; whatever `bind_traceparent_to_current_span` does in Rust,
  the TS mirror does in TS.
- **Coordinates with:** TRACE-003 — any new span attributes flow
  through the future redaction layer; until TRACE-003 lands, producers
  honour the advisory `SENSITIVE_FIELDS` deny-list and treat sensitive
  attribute values as a known gap (DA-OBS-004 risk acceptance per
  ADR-035 R1).
- **Coordinates with:** EXPORT-001 — `ANVIL_TRACE_SINK=otlp` is the
  dev-time half; EXPORT's production sink choice is independent and
  remains Draft until its trigger fires.
- **Coordinates with:** USAGE-001 — every usage observation Kindling
  records carries the active `traceparent` so a usage row joins to the
  spans that produced it. USAGE-001 is the writer; TRACE-004
  guarantees the trace context exists at write time.
- **Files (best-effort):** `crates/anvil-observability/src/lib.rs` (new
  binding helper + `ANVIL_TRACE_SINK` plumbing),
  `crates/anvil-observability/Cargo.toml` (optional OTLP exporter dep
  behind a feature flag), `crates/anvil-intercept/src/ipc.rs`
  (instrument dispatch; bind incoming `traceparent`),
  `crates/anvil-intercept/src/main.rs` (sink wiring at init),
  `crates/anvil-cli/src/main.rs` (instrument command entrypoints),
  `crates/anvil-intercept/tests/jsonrpc_conformance.rs` (extend the
  round-trip / handler-span test), `docs/observability/local-tracing.md` (new),
  `docs/observability/namespace-registry.md` (record any new
  attributes added by the instrumentation pass).
- **Validation:** Passed 2026-05-11: `cargo fmt --check`,
  `pnpm format:check`,
  `cargo clippy -p eddacraft-anvil-intercept -p eddacraft-anvil-observability --all-targets -- -D warnings`,
  `cargo test -p eddacraft-anvil-observability`, and
  `cargo test -p eddacraft-anvil-intercept --test jsonrpc_conformance`. PR #1435
  CI passed before merge. Jaeger / connected OTLP trace verification is deferred
  to EXPORT.
- **Confidence:** medium-high — the missing pieces are well-shaped
  (helper + attribute pass + opt-in sink) and TRACE-001's plumbing is
  the integration point. Risk lives in the breadth of the
  instrumentation pass.
- **Status:** Done

## Risks

- **R1 (accepted pre-launch, see ADR-035):** A `notification.context`
  payload carrying a secret could transit an unredacted span attribute to any
  tracing subscriber or sampled exporter. **Revisit condition met 2026-06-24:**
  INTD-015 is Complete and secret-detection has shipped, so TRACE-003 is now the
  tracing-pipe mitigation; only sampled-exporter behaviour remains deferred to
  EXPORT-001.
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
- **R5 (TRACE-004 breadth):** The instrumentation pass touches every
  command surface and the kernel, so a missed call path silently drops
  out of the trace tree. Mitigation: TRACE-004's INTD-014 fixture
  extension asserts the handler span carries the matching IDs, and the
  USAGE-001 contract test (which iterates the registered command list)
  fails if a command emits no span context.

## Open questions

- **OQ1 (deferred to EXPORT):** Production sink choice — Tempo /
  Honeycomb / Grafana Cloud / self-hosted Jaeger / OTLP-to-Vercel-OTel
  — to be decided when first paying customer or first production
  incident motivates it. Tracked in
  [observability-export](./observability-export.aps.md). EXPORT stays
  Draft until then.
