# Widget Catalogue — Compatibility Record

| Type     | Authority | Owner | Status     | Freshness                                                                   |
| -------- | --------- | ----- | ---------- | --------------------------------------------------------------------------- |
| As-built | Derived   | TUI   | Deprecated | Anvil-specific component truth moved 2026-08-20 under DOCRB-005 and ADR-123 |

| Upstream            | Downstream                                                                                                                                                                  |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADR-115 and ADR-123 | [anvil TUI architecture](../../crates/anvil-tui/ARCHITECTURE.md#anvil-specific-widgets-and-shared-widgets) and [eddacraft-tui README](../../crates/eddacraft-tui/README.md) |

## Current authority

Anvil-specific widget composition, status/finding presentation, surface
navigation, and rendered copy are maintained in the
[anvil TUI architecture](../../crates/anvil-tui/ARCHITECTURE.md#anvil-specific-widgets-and-shared-widgets).
Implementation lives under
[`crates/anvil-tui/src/widgets/`](../../crates/anvil-tui/src/widgets) and the
owning surface modules.

The shared `Theme`, semantic roles, keyboard mapping, generic widgets, shell
branding, animation, lifecycle option, and snapshot utilities are not anvil TUI
contracts. They remain owned and documented by
[`eddacraft-tui`](../../crates/eddacraft-tui/README.md), with implementation
under [`crates/eddacraft-tui/src/`](../../crates/eddacraft-tui/src).

## Decisions and history

[ADR-115](../../plans/decisions/115-eddacraft-tui-surface-trait-evolution.md)
governs the shared downstream-implemented `Surface` trait.
[ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
governs placement and discovery.

This path remains for inbound links and historical review. It no longer owns a
live combined catalogue. For the pre-migration catalogue and dated gap
narratives, run:

```bash
git log --follow -- docs/architecture/widgets-as-built.md
```

Historic inventory counts are not a substitute for the current module exports
and crate documentation.
