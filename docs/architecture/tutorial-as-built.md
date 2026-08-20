# Tutorial Subsystem — Compatibility Record

| Type     | Authority | Owner      | Status     | Freshness                                                                                          |
| -------- | --------- | ---------- | ---------- | -------------------------------------------------------------------------------------------------- |
| As-built | Derived   | TUI/LAUNCH | Deprecated | Component truth moved 2026-08-20 to `crates/anvil-tui/ARCHITECTURE.md` under DOCRB-005 and ADR-123 |

| Upstream                      | Downstream                                                                                                                                                                                                |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADR-092, ADR-115, and ADR-123 | [anvil TUI architecture](../../crates/anvil-tui/ARCHITECTURE.md#tutorial-engine), [CLI activation and runner](../../crates/anvil-cli/ARCHITECTURE.md), and [public tutorials](../public/anvil/tutorials/) |

## Current authority

Tutorial discovery, path and step state, fix/verify/watch-demo flow, rendering,
copy honesty, reset behaviour, and snapshots are maintained in the
[anvil TUI architecture](../../crates/anvil-tui/ARCHITECTURE.md#tutorial-engine).
Implementation remains under
[`crates/anvil-tui/src/surfaces/tutorial/`](../../crates/anvil-tui/src/surfaces/tutorial).

Terminal sessions, file-change delivery, command effects, and evidence-backed
activation labels belong to the
[CLI architecture](../../crates/anvil-cli/ARCHITECTURE.md). Reader-facing
tutorial content is a separate authority under
[`docs/public/anvil/tutorials/`](../public/anvil/tutorials/); this record does
not restate or refresh that corpus.

## Decisions and history

[ADR-092](../../plans/decisions/092-mcp-optional-activation-spine.md) governs
the activation vocabulary taught by the ProtectionLoop path.
[ADR-115](../../plans/decisions/115-eddacraft-tui-surface-trait-evolution.md)
governs the shared surface contract.
[ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
governs component-local placement.

This path remains for inbound links and historical review, not as a second live
tutorial-engine authority. For the detailed pre-migration flow and dated gap
register, run:

```bash
git log --follow -- docs/architecture/tutorial-as-built.md
```

Resolved gaps, rollout prose, and old snapshot counts remain historical.
