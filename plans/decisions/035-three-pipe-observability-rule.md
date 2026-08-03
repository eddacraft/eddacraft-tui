# ADR-035: Three-pipe observability rule (Kindling / Notification / tracing)

## Status

Accepted. Amended 2026-08-03 by
[ADR-116](116-kindling-product-profiles-and-governance-record.md): "write-once"
means append-only during normal operation; its authenticated, explicit,
receipted governance-prune transaction is the sole removal exception.

## Date

2026-04-30

## Context

Anvil now carries three telemetry-shaped pipes that have grown up at different
times for different reasons:

1. **Kindling** — the system of record for governance facts. ADR-116 defines
   Anvil's closed six-kind governance inventory and keeps `command.invoked` as a
   separate usage envelope; generic standalone-memory kinds are not silently
   governance facts. Rows are append-only outside ADR-116's explicit,
   receipted governance-prune transaction. The SQLite-backed mechanism is
   established under the Edda Stack and already cited by ADR-019 as the
   source-of-truth pipe for governance.
2. **Notification envelope** — the user-visible state-change feed surfaced by
   the dashboard live ops views and consumed across the CLI / TUI / dashboard
   surfaces. NOTIFY-008 published the canonical telemetry contract; INTD-013
   (mirror-decisions) is the latest mid-flight extension and INTD-015 is the
   queued redaction hardening.
3. **Tracing / OTEL spans** — the in-flight runtime tracing baseline that
   OBS-006 carved out as a placeholder, now lifted into a dedicated TRACE
   module under the Planning Council session plan-b00c16c7 split.

Without an explicit rule about which pipe owns what, drift is predictable:
a span attribute starts being read as a fact (it is not — spans are
ephemeral); a notification starts carrying ad-hoc instrumentation
(it should not — notifications are user-visible state); Kindling starts
absorbing per-evaluation noise that breaks its query usefulness (ADR-019
already pushed back on that for FLAGS, but the wider rule is still
unstated).

The Planning Council session plan-b00c16c7 reached consensus on a
**three-pipe rule** that pins each pipe's role and is explicit about what is
and is not source-of-truth.

This ADR records that rule and ratifies it as a decision the next domain
modules (TRACE, EXPORT, the eventual OBS Ready promotion, future RTAI tracing
attributes, and any new `anvil.<domain>.*` namespace contribution following
ADR-019's `anvil.flags.*` precedent) cite by reference instead of
re-litigating.

## Decision

Anvil has three observability pipes. Each pipe has one role, one consumer
shape, and one durability commitment. The matrix below is normative for any
new module that emits or consumes any of them.

### The matrix

| Pipe | Role | Consumer | Durability | Source of truth? |
|------|------|----------|------------|------------------|
| **Kindling** | Durable governance facts (gate evaluations, action observations, constraint applications). Append-only outside ADR-116's authenticated explicit prune, query-friendly. | Provenance queries, audit trails, policy evidence, Edda canonical memory. | Persistent (SQLite, retained per Kindling retention policy). | **Yes** — for governance outcomes that need to be cited later. |
| **Notification envelope** | User-visible state changes that the dashboard live feed and surface UIs render. Carries `notification.context` for mid-flight state (e.g. INTD-013 decision-mirror) and is the contract NOTIFY-008 / INTD-014 ratify. | Dashboard live ops views, CLI/TUI live surfaces, anything that has to render a current state change to a human. | Buffered for the live feed; not retained as a long-term archive. | **Yes** — for user-visible state. The dashboard reads this as the authoritative live view. |
| **Tracing / OTEL spans** | Ephemeral debugging context: spans, attributes, baggage, `traceparent` propagation. Diagnostic instrumentation only. | Developer-facing tracing UIs, sampled exporters (chosen under EXPORT module, deferred), local `tracing-subscriber` formatters. | Ephemeral by default; sampled / sink-driven retention if EXPORT chooses one. | **No.** Spans MUST NOT be the only place a fact is recorded. If a fact matters for governance, it goes to Kindling. If it matters for the dashboard live view, it goes to the notification envelope. Spans are debugging breadcrumbs. |

### Concrete rules that fall out of the matrix

1. **Tracing is never source-of-truth.** A code path that emits an OTEL
   span attribute and nothing else has lost the fact the moment retention
   expires. If a downstream consumer (dashboard, audit, governance check)
   reads from spans, that is a bug to be fixed by routing the fact to
   Kindling or to the notification envelope. The TRACE module's runbook and
   the namespace registry both make this explicit.
2. **Kindling does not absorb per-request instrumentation.** ADR-019 already
   set this for FLAGS-006: routine flag evaluations go to OTEL metrics, only
   gate-affecting outcomes go to Kindling. The three-pipe rule generalises
   that: high-frequency, low-stakes signals belong on tracing; only
   governance-relevant outcomes earn a Kindling row.
3. **Notification envelope does not carry debug breadcrumbs.** Spans are for
   debugging; notifications are for user-visible state. A new dashboard
   view that needs span-level context should consume traces directly via
   the (deferred) EXPORT pipeline — it should not hijack
   `notification.context` to ship debug data to the dashboard.
4. **`traceparent` is the cross-pipe correlation key.** The TRACE module's
   subscriber init (TRACE-001) puts a W3C `traceparent` on every span, on
   the JSON-RPC envelope (per INTD-014's conformance fixture), and on the
   notification envelope. A consumer that wants to join a notification to
   its underlying spans uses `traceparent`. This makes the cross-pipe story
   correlatable without making any pipe authoritative for the others'
   facts.

### Redaction risk note

INTD-013 introduced `notification.context`, which can carry arbitrary
state-change payloads. INTD-015 is the queued redaction hardening for that
field. Until INTD-015 is at least Ready, secret-detection rule output that
fires before INTD-015 lands could in principle transit
`notification.context` unredacted. The founder (Planning Council session
plan-b00c16c7, 2026-04-30) accepted this gap **pre-launch** with explicit
revisit triggers (INTD-015 reaching Ready, OR the first secret-detection
rule shipping, whichever comes first). The
TRACE-003 task carries the corresponding redaction-layer hardening for the
tracing pipe; until both land, every new contribution under this ADR's
matrix should treat secret-bearing payloads as a known gap, not an
unknown one.

## Rationale

### Alternatives considered

| Option | Pros | Cons |
|--------|------|------|
| Three-pipe rule with explicit matrix (chosen) | One short reference each domain module cites; pins durability, role, source-of-truth per pipe; aligns with ADR-019's domain-owned `anvil.flags.*` precedent and inverts FLAGS' "ratify, not design" pattern at the pipe level | Codifies a hard line that may have edge cases (e.g. dashboard wanting span-level data for a power-user view) — handled by routing through EXPORT, not by mixing pipes |
| Single observability pipe | Maximum simplicity | Conflates governance facts, live UI state, and ephemeral debugging; erases ADR-019's separation between routine flag telemetry and Kindling governance facts; would force re-architecture of NOTIFY and Kindling |
| Two pipes (Kindling + tracing) and put live-state on tracing | One fewer pipe shape | Notifications are not ephemeral debugging context; the dashboard cannot tolerate sampled retention; Kindling does not have the live-feed shape needed by the dashboard |
| Defer the rule until an incident forces it | Zero upfront writing | Drift is invisible right up to the incident; the cost of pulling a fact back from a stale span when audit asks for it is exactly the cost this rule avoids |

### Why this option

Each pipe already exists for a reason that the others do not satisfy:

- Kindling is the only append-only-by-default, query-shaped, retained store;
  ADR-116's authenticated explicit prune is the sole removal exception.
- The notification envelope is the only contract the dashboard live feed
  consumes.
- Tracing/OTEL is the only place developer debugging breadcrumbs land.

A rule that pins those roles in writing is cheap to publish and expensive to
violate later. ADR-019 demonstrates that a domain can contribute a narrow
`anvil.flags.*` convention without waiting for OBS to design every field; this
ADR says where each kind of attribute lives once a domain contributes one.

The redaction-risk acceptance is documented here rather than buried in the
TRACE module body so that future contributors, including the people writing
INTD-015, see one canonical statement of the gap.

## Consequences

- **Positive:** Domain modules adding new attributes under their
  `anvil.<domain>.*` namespace know which pipe to put them on without a new
  Council round. The TRACE module's namespace registry can cite this ADR by
  anchor link rather than restating the policy.
- **Positive:** The dashboard cannot drift into reading from spans by
  accident; that path is now an explicit rule violation, surfaced in
  review.
- **Positive:** Aligns with and generalises ADR-019's FLAGS precedent:
  domain modules may contribute small namespaced conventions, and this ADR
  sets the pipe-allocation pattern those conventions must follow.
- **Negative:** Edge cases that genuinely need span-level data on the
  dashboard now require an EXPORT-driven pathway rather than a quick
  notification-envelope shortcut. Acceptable — that is the path that scales.
- **Risks (R1, accepted pre-launch by the founder during Planning Council
  session plan-b00c16c7, 2026-04-30):** Secret content may transit via
  INTD-013 `notification.context` if a secret-detection rule fires before
  INTD-015 lands. Trigger to revisit: INTD-015 reaches Ready, OR first
  secret-detection rule ships, whichever first.
- **Risks (R2):** `anvil.<domain>.*` namespace fragmentation if multiple
  modules contribute attributes with conflicting shapes (units, plurals,
  naming case). Mitigation: namespace registry doc (TRACE-001 stub) +
  founder PR review gate for each new namespace contribution.
- **Risks (R3):** TRACE-002 deferred — dashboard cannot join traces across
  producers on day one. Mitigation: documented in the Known Gaps section of
  the namespace registry; the day-one limitation does not block the
  three-pipe rule from being usable.
- **Mitigations:** When INTD-015 reaches Ready, this ADR's redaction-risk
  note moves from "accepted gap" to "closed". When EXPORT chooses a sink,
  the matrix gains a fourth column for sampled-retention behaviour.

## References

- Related ADRs:
  - ADR-019 (feature flag telemetry alignment — established the
    domain-owned `anvil.flags.*` precedent and Kindling boundary)
  - ADR-034 (cross-cutting modules — TRACE is the cross-cutting module
    that ratifies this rule)
- Related modules: `plans/modules/tracing-foundation.aps.md` (TRACE),
  `plans/modules/observability-export.aps.md` (EXPORT, deferred),
  `plans/modules/observability-foundation.aps.md` (OBS, post-launch
  hardening)
- Related work items for the redaction risk: INTD-013 (mirror-decisions,
  Committed), INTD-014 (JSON-RPC conformance, Committed), INTD-015 (queued
  redaction hardening), TRACE-003 (post-launch redaction-layer hardening)
- Planning Council session: plan-b00c16c7 (2026-04-30)
