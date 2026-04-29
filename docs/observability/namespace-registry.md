<!-- placeholder; TRACE-001 will expand this with field-naming rules and validation hooks -->

# Anvil Observability Namespace Registry

> **Status:** Stub created as part of Planning Council session
> [plan-b00c16c7](../../plans/decisions/034-cross-cutting-modules-as-aps-primitive.md)
> (2026-04-30). Populated under TRACE-001 with full field-naming rules,
> attribute shape conventions, and validation hooks.
>
> Normative references: [ADR-019](../../plans/decisions/019-namespace-freedom.md)
> (namespace freedom) and [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md)
> (three-pipe rule).

## Purpose

This registry records every `anvil.<domain>.*`, `kindling.*`, and partner
namespace that contributes attributes to Anvil's observability pipes. Per
ADR-019, namespaces are free to contribute. This document is the durable
record of which namespaces exist, who owns them, and which pipe each
attribute lands on per the ADR-035 three-pipe matrix.

## Initial namespace entries

| Namespace        | Owner / origin                           | Pipe(s)                          | Notes                                                                 |
| ---------------- | ---------------------------------------- | -------------------------------- | --------------------------------------------------------------------- |
| `anvil.flags.*`  | FLAGS module / [ADR-019](../../plans/decisions/019-namespace-freedom.md) | Tracing (per-eval), Kindling (gate-affecting outcomes) | Routine evaluations on tracing; only gate-affecting outcomes earn a Kindling row. |
| `kindling.*`     | Kindling system (Edda Stack)             | Kindling                         | Governance facts only; write-once, query-shaped. Source-of-truth for governance. |
| `anvil.rtai.*`   | RTAI module (provisional)                | Tracing                          | Provisional namespace pending RTAI promotion; ratified via TRACE-001 registry stub. |

## How to add a namespace

New `anvil.<domain>.*` contributions follow ADR-019:

1. The owning module proposes the namespace in its module spec.
2. The contribution lands as a PR that adds a row to the table above.
3. The PR is reviewed by the founder before merge (founder PR-review gate).
4. Pipe allocation must comply with the ADR-035 three-pipe matrix.

## Known Gaps

- **Day-one limitation:** dashboard cannot join traces across producers
  until TS-side `traceparent` parsing lands (tracked under TRACE-002).
- **Secret-redaction across binary boundaries** is not yet hardened on
  the tracing pipe. Tracked under TRACE-003; closed when both TRACE-003
  and INTD-015 land. Until then, treat `notification.context` payloads
  as potentially secret-bearing (DA-OBS-004 risk acceptance, see
  [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md)).
