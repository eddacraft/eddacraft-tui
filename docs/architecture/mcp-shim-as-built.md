# anvil MCP Shim — Compatibility Record

| Type     | Authority | Owner   | Status     | Freshness                                                                                          |
| -------- | --------- | ------- | ---------- | -------------------------------------------------------------------------------------------------- |
| As-built | Derived   | CLI/MCP | Deprecated | Component truth moved 2026-08-20 to `crates/anvil-cli/ARCHITECTURE.md` under DOCRB-005 and ADR-123 |

| Upstream                      | Downstream                                                                                                                                                                                                             |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADR-106, ADR-113, and ADR-123 | [anvil CLI architecture](../../crates/anvil-cli/ARCHITECTURE.md#mcp-shim), [MCP server specification](rust-mcp-server-spec.md), [intercept map](intercept-as-built.md), and [driver map](driver-framework-as-built.md) |

## Current authority

The in-binary MCP process model, fourteen-tool registry, validation routing,
authentication posture, workspace containment, redaction, graph egress, and
failure invariants are maintained in the
[anvil CLI architecture](../../crates/anvil-cli/ARCHITECTURE.md#mcp-shim).
Implementation remains under
[`crates/anvil-cli/src/mcp/`](../../crates/anvil-cli/src/mcp).

The [MCP server specification](rust-mcp-server-spec.md) retains protocol design
intent. The [intercept map](intercept-as-built.md) retains the daemon boundary,
and the [driver map](driver-framework-as-built.md) retains cross-component
protocol and capability relationships.

## Decisions and history

[ADR-106](../../plans/decisions/106-agent-integration-registry-and-managed-installers.md)
governs client integration.
[ADR-113](../../plans/decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md) governs
the dual-era protocol boundary.
[ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
governs component-local placement.

This path remains useful to inbound links but no longer asserts live component
authority. For the detailed pre-migration map and dated gap register, run:

```bash
git log --follow -- docs/architecture/mcp-shim-as-built.md
```

Earlier tool counts, line references, rollout summaries, and resolved gaps are
historical evidence; current source and registry tests are authoritative.
