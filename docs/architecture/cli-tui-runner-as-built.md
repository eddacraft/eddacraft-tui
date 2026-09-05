# CLI TUI Runner — Compatibility Record

| Type     | Authority | Owner | Status     | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| -------- | --------- | ----- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | CLI   | Deprecated | Last reviewed 2026-09-06 (repository local date; 2026-09-05 UTC) for CONV-002's internal CLI/MCP scan extraction; topology, protocol, review scope and historical evidence here are unaffected. Unaffected review 2026-09-05: RIO-001/002 enforce existing I/O bounds; protocol, topology and historical evidence here remain unchanged. Unaffected review 2026-08-31: CIB-385 through CIB-389 re-date `crates/anvil-cli/ARCHITECTURE.md`; no TUI path added. Also unaffected 2026-08-30: CONF-011 is plain/structured output and adds no TUI path; component truth remains in `crates/anvil-cli/ARCHITECTURE.md` under DOCRB-005 and ADR-123 |

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
