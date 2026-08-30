# anvil-kernel-types

Shared types for the anvil Rust kernel — events, graph nodes, trust levels, and
cross-surface contracts.

## Modules

- **`events`** — kernel event types (file changes, parse results, policy
  violations)
- **`graph`** — graph node and edge type definitions; the resident graph lives
  in
  [`../anvil-graph-cache/ARCHITECTURE.md`](../anvil-graph-cache/ARCHITECTURE.md)
- **`trust`** — trust level enums and scoring
- **`diagnostics`** — the canonical `anvil.diagnostic.v1` envelope (see below)
- **`conformance`** — the canonical `anvil.intent-conformance.v1` input and
  verdict types, binding validation, and external-producer JSON Schema

## Canonical Diagnostic Envelope (`anvil.diagnostic.v1`)

`anvil-kernel-types` owns the `anvil.diagnostic.v1` diagnostic shape used by
gate, save-time, watch, and mid-edit validation surfaces. The AI guardrail
profile (`anvil gate --profile ai`), the RTAI-001 mid-edit secret-detection
loop, and the MCP `validate_write` tool all emit diagnostics in this envelope so
agent and editor consumers can parse results without bespoke per-surface
plumbing.

The envelope coordination spec records how AIGUARD, RTAI, INTD, and DRVR share
it and how the schema version is rolled forward. New diagnostic producers must
depend on this crate rather than re-deriving a parallel shape.

## Usage

This crate is a dependency of `anvil-kernel`, `anvil-tui`, `anvil-cli`,
`anvil-checks`, and the MCP server. It contains shared type definitions,
serialisation derives, and contract-only structural validation; source parsing
and policy evaluation stay with their owning crates.

External conformance producers can consume
[`intent-conformance-v1.schema.json`](schema/intent-conformance-v1.schema.json)
or the same schema through `conformance_json_schema()`. The runtime contract
keeps intent provenance, Git and graph evidence bindings, outcome, and evidence
strength distinct so incomplete evidence cannot serialise as a conformant
verdict.

Verdicts also separate exact raw Git change records from canonical per-path
coverage. Coverage entries reference their backing raw-record indices and carry
the path's policy/evidence disposition, so a rename keeps one raw record while
its old and new endpoints are evaluated independently. Output validation
requires the exact endpoint-to-record map in both directions and accepts legal
repository-relative UTF-8 Git paths independently of the stricter
authority-prefix grammar.

## Part of

[eddacraft anvil](../../README.md) monorepo (`crates/anvil-kernel-types`).
