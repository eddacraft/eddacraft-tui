<!--
APS Module: MCP Server
======================
Exposes Anvil validation as MCP tools for AI assistants.
See: plans/aps-rules.md
-->

# MCP Server

| ID  | Owner | Status |
| --- | ----- | ------ |
| MCP | —     | Ready  |

## Purpose

Enable AI assistants (Claude Code, Cursor, VS Code Copilot) to validate code
during generation by exposing Anvil's check, gate, and architecture analysis as
MCP tools. This shifts validation left — catching issues before code is accepted
rather than after commit.

## In Scope

- MCP server package (`@eddacraft/anvil-mcp-server`)
- Tools: `anvil_check`, `anvil_gate`, `anvil_status`, `anvil_explain`
- Resources: baseline, config, patterns, suppressions
- Prompts: warning explanation, suppression guidance
- Stdio transport (primary), HTTP transport (secondary)
- Configuration generators for Claude Desktop and Cursor

## Out of Scope

- Watch mode with streaming (future enhancement)
- Remote server deployment/hosting
- Authentication/authorisation (MCP doesn't define this)
- Custom tool creation API (users extend via adapters)

## Interfaces

**Depends on:**

- `@eddacraft/anvil-core` — CheckRunner, GateRunner, ArchitectureService
- `@eddacraft/anvil-cli` — Configuration loading, project detection
- `@modelcontextprotocol/sdk` — MCP server implementation

**Exposes:**

- `@eddacraft/anvil-mcp-server` — Standalone MCP server binary
- `anvil_check` tool — File validation with warnings
- `anvil_gate` tool — Full quality gate execution
- `anvil_status` tool — Project health summary
- `anvil_explain` tool — Warning deep-dive with fixes
- `anvil://` resources — Baseline, config, patterns access

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined

## Tasks

### MCP-001: Package scaffold and basic server

- **Intent:** Establish MCP server package with working stdio transport
- **Expected Outcome:** `npx @eddacraft/anvil-mcp-server` starts and responds to MCP
  handshake
- **Validation:** `pnpm -F mcp-server test`
- **Files:** `packages/mcp-server/src/index.ts`,
  `packages/mcp-server/src/server.ts`
- **Dependencies:** None (foundational)
- **Confidence:** high

### MCP-002: anvil_check tool implementation

- **Intent:** Expose core validation as MCP tool
- **Expected Outcome:** AI can invoke `anvil_check` with file paths, receives
  warnings with locations and explanations
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="check tool"`
- **Files:** `packages/mcp-server/src/tools/check.tool.ts`
- **Dependencies:** MCP-001
- **Confidence:** high

### MCP-003: anvil_gate and anvil_status tools

- **Intent:** Expose gate runner and status summary as tools
- **Expected Outcome:** AI can run full gate or get quick health check
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="gate|status"`
- **Files:** `packages/mcp-server/src/tools/gate.tool.ts`,
  `packages/mcp-server/src/tools/status.tool.ts`
- **Dependencies:** MCP-002
- **Confidence:** high

### MCP-004: Resources and prompts

- **Intent:** Expose baseline, config, patterns as readable resources; add
  prompt templates
- **Expected Outcome:** AI can read project context and use guided prompts
- **Validation:**
  `pnpm -F mcp-server test -- --testNamePattern="resource|prompt"`
- **Files:** `packages/mcp-server/src/resources/`,
  `packages/mcp-server/src/prompts/`
- **Dependencies:** MCP-003
- **Confidence:** medium

### MCP-005: HTTP transport and configuration generators

- **Intent:** Support remote server mode and generate config for AI tools
- **Expected Outcome:** Server runs via HTTP; `anvil mcp-config` outputs valid
  JSON for Claude/Cursor
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="http|config"`
- **Files:** `packages/mcp-server/src/transports/http.ts`,
  `cli/src/commands/mcp-config.ts`
- **Dependencies:** MCP-004
- **Confidence:** medium

## Execution

Steps: [../execution/MCP-001.steps.md](../execution/MCP-001.steps.md)

## Risks

| Risk                         | Impact | Mitigation                            |
| ---------------------------- | ------ | ------------------------------------- |
| MCP SDK breaking changes     | High   | Pin version, monitor releases         |
| Slow response on large repo  | Medium | Incremental check, caching            |
| AI misinterprets tool output | Medium | Structured JSON, clear error messages |

## Open Questions

- [ ] Should `anvil_watch` be a tool (polling) or resource (subscription)?
- [ ] Include `anvil_fix` tool for auto-remediation? (risky for AI to modify)
- [ ] MCP registry listing requirements?
