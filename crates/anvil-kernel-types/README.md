# anvil-kernel-types

Shared types for the Anvil Rust kernel — events, graph nodes, and trust levels.

## Modules

- **`events`** — kernel event types (file changes, parse results, policy
  violations)
- **`graph`** — graph node and edge type definitions
- **`trust`** — trust level enums and scoring
- **`diagnostics`** — the canonical `anvil.diagnostic.v1` envelope (see below)

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
`anvil-checks`, and the MCP server. It contains no logic — only type definitions
and serialisation derives.

## Part of

[eddacraft Anvil](../../README.md) monorepo (`crates/anvil-kernel-types`).
