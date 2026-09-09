# Activation Orchestrator — Compatibility Record

| Type     | Authority | Owner      | Status     | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| -------- | --------- | ---------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| As-built | Derived   | CLI/LAUNCH | Deprecated | Last reviewed 2026-09-10 JREL-009 preserve explicit MCP launch choices Prior review 2026-09-06 (repository local date; 2026-09-05 UTC) for CONV-002's internal CLI/MCP scan extraction; topology, protocol, review scope and historical evidence here are unaffected. Unaffected review 2026-09-05: RIO-001/002 enforce existing I/O bounds; protocol, topology and historical evidence here remain unchanged. Unaffected review 2026-08-31: CIB-385 through CIB-389 re-date `crates/anvil-cli/ARCHITECTURE.md`; activation orchestration unchanged. Also unaffected 2026-08-30: CONF-011 adds a command adapter outside activation; component truth remains in `crates/anvil-cli/ARCHITECTURE.md` under DOCRB-005 and ADR-123 |

| Upstream                      | Downstream                                                                                                                                                                                                             |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADR-092, ADR-106, and ADR-123 | [anvil CLI architecture](../../crates/anvil-cli/ARCHITECTURE.md#activation-orchestration), [architecture overview](overview.md), [auth map](auth-as-built.md), and [MCP server specification](rust-mcp-server-spec.md) |

## Current authority

Activation orchestration, the evidence-based `ProtectionState` vocabulary,
client detection/installation, fallback behaviour, and component failure
invariants are maintained in the
[anvil CLI architecture](../../crates/anvil-cli/ARCHITECTURE.md#activation-orchestration).
Implementation remains under
[`crates/anvil-cli/src/activation/`](../../crates/anvil-cli/src/activation).

The central [architecture overview](overview.md) retains cross-system placement.
The [auth map](auth-as-built.md) retains authentication relationships, and the
[MCP server specification](rust-mcp-server-spec.md) retains MCP design intent.
Operational fallback belongs to the
[no-MCP activation runbook](../runbooks/anvil-no-mcp-activation.md).

## Decisions and history

[ADR-092](../../plans/decisions/092-mcp-optional-activation-spine.md) governs
the optional-MCP activation spine.
[ADR-106](../../plans/decisions/106-agent-integration-registry-and-managed-installers.md)
governs client registry and managed installation.
[ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
governs the move to component-local authority.

This path is retained for inbound links and historical review. It no longer
asserts live component authority. To inspect the detailed pre-migration
implementation map and dated gap register, run:

```bash
git log --follow -- docs/architecture/activation-as-built.md
```

Dated rollout summaries, source line counts, and resolved gaps in earlier
versions are historical evidence, not current behaviour.
