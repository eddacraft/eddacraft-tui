# eddacraft-anvil-settings

| Type  | Authority     | Owner  | Status | Freshness                                                               |
| ----- | ------------- | ------ | ------ | ----------------------------------------------------------------------- |
| Crate | Authoritative | SETCON | Live   | Last reviewed 2026-08-25 against ADR-132 and SETCON-001..011 acceptance |

| Upstream                                                                               | Downstream                                  |
| -------------------------------------------------------------------------------------- | ------------------------------------------- |
| [ADR-132](../../plans/decisions/132-settings-truth-contract.md), `crates/anvil-config` | SETINS, SETPREF, SETGOV, `crates/anvil-cli` |

Settings truth service for Anvil (SETCON /
[ADR-132](../../plans/decisions/132-settings-truth-contract.md)).

This crate is the only settings read model. Surfaces (CLI, TUI, MCP) consume it;
they do not read `.anvil.<ext>` for settings purposes and they do not write
configuration except through the settings service (SETPREF / SETGOV extend that
later).

## What it owns

- typed catalogue
- precedence / composite resolution with provenance
- post-resolution policy constraints
- runtime-state classification (`unknown` / `stale` / `failed` / `drift` /
  `active`)
- health aggregation
- redacted `anvil.settings.v1` envelope
- mapping of settings semantic outcomes onto the global CLI exit registry

File discovery stays in `eddacraft-anvil-config`. Attestation transport is the
existing intercept daemon RPC.

## Tests

```text
cargo test -p eddacraft-anvil-settings
```
