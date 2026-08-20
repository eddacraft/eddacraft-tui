# anvil-tui — Compatibility Record

| Type     | Authority | Owner | Status     | Freshness                                                                                          |
| -------- | --------- | ----- | ---------- | -------------------------------------------------------------------------------------------------- |
| As-built | Derived   | TUI   | Deprecated | Component truth moved 2026-08-20 to `crates/anvil-tui/ARCHITECTURE.md` under DOCRB-005 and ADR-123 |

| Upstream            | Downstream                                                                                                                                        |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADR-115 and ADR-123 | [anvil TUI architecture](../../crates/anvil-tui/ARCHITECTURE.md) and [CLI terminal runner](../../crates/anvil-cli/ARCHITECTURE.md#cli-tui-runner) |

## Current authority

Anvil-specific surface dispatch, state/event adapters, rendering invariants,
navigation, failure behaviour, tutorial integration, and snapshots are
maintained in the
[anvil TUI architecture](../../crates/anvil-tui/ARCHITECTURE.md). Implementation
remains under [`crates/anvil-tui/src/`](../../crates/anvil-tui/src).

Terminal lifecycle and external event polling belong to the
[CLI runner](../../crates/anvil-cli/ARCHITECTURE.md#cli-tui-runner). Shared
theme, keyboard, generic widget, lifecycle, and snapshot contracts belong to
[`eddacraft-tui`](../../crates/eddacraft-tui/README.md). Dashboard/operator
relationships remain in the central [architecture overview](overview.md).

## Decisions and history

[ADR-115](../../plans/decisions/115-eddacraft-tui-surface-trait-evolution.md)
governs shared surface evolution.
[ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
governs component-local placement.

This path remains as a compatibility and history record, not a duplicate live
component map. For its pre-migration detail, run:

```bash
git log --follow -- docs/architecture/tui-as-built.md
```

Earlier surface and snapshot counts, rollout status, and resolved gaps are
historical evidence.
