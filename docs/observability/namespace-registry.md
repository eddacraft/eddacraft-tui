# Anvil Observability Namespace Registry

> **Status:** Live as of 2026-04-30 (TRACE-001 landed). Records every
> `anvil.<domain>.*` namespace contributed to the tracing pipe and which
> binary owns the contribution. New rows are added by founder-reviewed
> PR per the procedure below.
>
> Normative references:
> [ADR-019](../../plans/decisions/019-flags-observability-alignment.md) (feature
> flag telemetry alignment),
> [ADR-034](../../plans/decisions/034-cross-cutting-modules-as-aps-primitive.md)
> (cross-cutting module primitive), and
> [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md)
> (three-pipe rule).

## Purpose

This registry records every `anvil.<domain>.*`, `kindling.*`, and partner
namespace that contributes attributes to Anvil's observability pipes. ADR-019
established the first domain-owned namespace precedent with `anvil.flags.*`;
this document is the durable record of which namespaces exist, who owns them,
and which pipe each attribute lands on per the ADR-035 three-pipe matrix.

## Initial namespace entries

| Namespace       | Owner / origin                                                                       | Pipe(s)                                                | Notes                                                                               |
| --------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| `anvil.flags.*` | FLAGS module / [ADR-019](../../plans/decisions/019-flags-observability-alignment.md) | Tracing (per-eval), Kindling (gate-affecting outcomes) | Routine evaluations on tracing; only gate-affecting outcomes earn a Kindling row.   |
| `kindling.*`    | Kindling system (Edda Stack)                                                         | Kindling                                               | Governance facts only; write-once, query-shaped. Source-of-truth for governance.    |
| `anvil.rtai.*`  | RTAI module (provisional)                                                            | Tracing                                                | Provisional namespace pending RTAI promotion; ratified via TRACE-001 registry stub. |

## Field-naming rules

All `anvil.<domain>.*` attributes obey the conventions established by
ADR-019's `anvil.flags.*` precedent:

1. **Lower snake_case** for the leaf segment: `anvil.flags.gate_id`, not
   `anvil.flags.gateId` or `anvil.flags.GateId`.
2. **Singular nouns** unless the attribute is genuinely a list:
   `anvil.flags.value` (singular), `anvil.flags.values` (only if a list
   is the contract).
3. **Dotted hierarchy** with the domain immediately after `anvil.`:
   `anvil.<domain>.<subject>.<attribute>`. Two segments after the
   domain is the maximum; deeper hierarchies indicate the domain should
   own a nested namespace.
4. **Units in the name** when ambiguous: `anvil.intercept.scan_duration_ms`
   beats `anvil.intercept.scan_duration`. SI units lower-cased.

## Pipe allocation (ADR-035)

A new attribute MUST live on exactly one pipe. If a fact must persist to
two pipes (e.g. an outcome that is both a notification and a Kindling
row) the producer emits to each pipe via that pipe's contract — it does
NOT smuggle the same attribute name across both.

| Question | Answer | Pipe |
|---|---|---|
| Does the dashboard live feed need to render this state change? | Yes | Notification envelope |
| Is this a governance fact that must be cited later? | Yes | Kindling |
| Otherwise — debugging breadcrumbs, in-flight context | — | Tracing |

## How to add a namespace

1. The owning module proposes the namespace in its module spec.
2. The contribution lands as a PR that adds a row to the table above.
3. The PR is reviewed by the founder before merge (founder PR-review gate).
4. Pipe allocation must comply with the ADR-035 three-pipe matrix.
5. Field names match the ADR-019 conventions documented above.

## Validation hooks

- **Rust producers:** the [`anvil-observability`](../../crates/anvil-observability/)
  crate exposes [`TraceContext`](../../crates/anvil-observability/src/traceparent.rs)
  for parsing/generating W3C `traceparent` headers and an
  **advisory-only** redaction
  [deny-list](../../crates/anvil-observability/src/redaction.rs)
  (`SENSITIVE_FIELDS`) producers may consult before adding a new span
  attribute. **The deny-list is NOT enforced** by the installed
  subscriber — see Known Gaps.
- **JSON-RPC envelope:** the daemon validates `traceparent` on every
  incoming request and round-trips the validated header on the matching
  response. The
  [INTD-014 conformance fixture](../../crates/anvil-intercept/tests/jsonrpc_conformance.rs)
  carries a regression test pinning that contract.
- **Subscriber init:** `init_tracing(BinaryKind)` is the only entry
  point. Library crates emit spans via the global `tracing` macros and
  MUST NOT install their own subscriber.

> **Same-UID peers can supply any valid `traceparent`:** the daemon
> does not mint trace IDs and cannot detect ID-fixation. Trace
> integrity for exported spans is the exporter's deduplication concern,
> not the envelope's. Documented per ADR-035 R1 risk acceptance.

## Known Gaps

- **Redaction is NOT active.** `SENSITIVE_FIELDS` is a future deny-list,
  not an enforced subscriber layer. Span attributes named `password`,
  `token`, `api_key` etc. will appear in plain text in JSON log output
  until TRACE-003 wires the layer. Producers MUST NOT emit secret-bearing
  values into spans before TRACE-003 ships. Reviewed pre-launch under
  ADR-035 R1 risk acceptance.
- **Day-one limitation:** dashboard cannot join traces across producers until
  TS-side `traceparent` parsing lands (tracked under TRACE-002).
- **Secret-redaction across binary boundaries** is not yet hardened on the
  tracing pipe. Tracked under TRACE-003; closed when both TRACE-003 and INTD-015
  land. Until then, treat `notification.context` payloads as potentially
  secret-bearing (DA-OBS-004 risk acceptance, see
  [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md)).
