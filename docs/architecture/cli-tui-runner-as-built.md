# CLI TUI Runner — Compatibility Record

| Type     | Authority | Owner | Status     | Freshness                                                                                          |
| -------- | --------- | ----- | ---------- | -------------------------------------------------------------------------------------------------- |
| As-built | Derived   | CLI   | Deprecated | Component truth moved 2026-08-20 to `crates/anvil-cli/ARCHITECTURE.md` under DOCRB-005 and ADR-123 |

| Upstream            | Downstream                                                                                                                                           |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADR-115 and ADR-123 | [anvil CLI architecture](../../crates/anvil-cli/ARCHITECTURE.md#cli-tui-runner) and [anvil TUI architecture](../../crates/anvil-tui/ARCHITECTURE.md) |

## Current authority

Terminal setup and restoration, panic handling, standard and specialised event
loops, animation/dirty redraws, `SurfaceExit`, and channel-failure behaviour are
maintained in the
[anvil CLI architecture](../../crates/anvil-cli/ARCHITECTURE.md#cli-tui-runner).
Implementation remains in
[`crates/anvil-cli/src/tui.rs`](../../crates/anvil-cli/src/tui.rs).

Surface state, rendering, tutorial flow, widgets, and snapshots belong to the
[anvil TUI architecture](../../crates/anvil-tui/ARCHITECTURE.md). Shared
terminal-widget primitives belong to
[`eddacraft-tui`](../../crates/eddacraft-tui/README.md).

## Decisions and history

[ADR-115](../../plans/decisions/115-eddacraft-tui-surface-trait-evolution.md)
governs the shared surface extension boundary.
[ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
governs component-local placement.

This path remains for old links and review history; it is not a second live
runner authority. For the pre-migration implementation map, run:

```bash
git log --follow -- docs/architecture/cli-tui-runner-as-built.md
```

Earlier call-site and line counts and resolved gap narratives are historical,
not current behaviour.
