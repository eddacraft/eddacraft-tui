<!--
APS Module: MCP Server Hardening
================================
Addresses remaining issues from the 2026-02-05 code review.
See: packages/mcp-server/REVIEW.md
-->

# MCP Server Hardening

| ID   | Owner | Status |
| ---- | ----- | ------ |
| MCPH | —     | Draft  |

## Purpose

Address the remaining 9 issues identified in the MCP server code review
(2026-02-05). These range from security hardening (P1) to code quality
improvements (P3). This module tracks the work needed to resolve each finding
and ensure the MCP server is production-ready.

**Source:** [packages/mcp-server/REVIEW.md](../packages/mcp-server/REVIEW.md)

## In Scope

- P1 security and correctness issues (3 items)
- P2 code quality and robustness issues (2 items)
- P3 nits and improvements (4 items)

## Out of Scope

- New MCP features (handled by MCP module)
- Performance optimization beyond the identified issues

## Interfaces

**Depends on:**

- `@eddacraft/anvil-mcp-server` — Package being hardened
- `@eddacraft/anvil-runtime` — For P2-4 (empty plan factory)

**Exposes:**

- Hardened MCP server with improved security and robustness

## Ready Checklist

Change status to **Ready** when:

- [ ] All P1 issues have clear implementation paths
- [ ] Team has reviewed security implications of P1-1
- [ ] At least one task defined

## Tasks

### MCPH-001: Read-tool workspace root validation

- **Intent:** Prevent untrusted HTTP clients from probing arbitrary filesystem
  paths via read-only tools
- **Expected Outcome:** Read tools (`anvil_check`, `anvil_gate`, `anvil_status`,
  `anvil_query_boundary`) validate that `workspaceRoot` is within allowed roots,
  or use server-pinned root like write tools do
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="workspace"`
- **Files:** `packages/mcp-server/src/tools/check.tool.ts`,
  `packages/mcp-server/src/tools/gate.tool.ts`,
  `packages/mcp-server/src/tools/status.tool.ts`,
  `packages/mcp-server/src/tools/query-boundary.tool.ts`,
  `packages/mcp-server/src/server.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P1
- **Notes:** Design decision needed: (a) pin all tools to server root like write
  tools, (b) validate against allowed roots list, or (c) accept risk for
  localhost-only HTTP deployments. For stdio transport this is low-risk since
  the client already has local access.

### MCPH-002: Document AP-003 regex limitations

- **Intent:** Prevent false positives from AP-003 fix pattern matching inside
  strings/comments
- **Expected Outcome:** Tool description clearly documents that the fix applies
  line-by-line regex and may match `: any` in strings or comments; users
  understand to review changes
- **Validation:** Manual review of tool description
- **Files:** `packages/mcp-server/src/tools/fix.tool.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Notes:** Full AST-level analysis would require TypeScript compiler
  integration, which is out of scope for deterministic mechanical fixes.
  Documentation is the pragmatic solution.

### MCPH-003: Synchronize version strings

- **Intent:** Ensure version reported in MCP handshake and status tool matches
  package.json
- **Expected Outcome:** Server version comes from a single source of truth;
  changing package.json version automatically updates all version strings
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="version"`
- **Files:** `packages/mcp-server/src/server.ts`,
  `packages/mcp-server/src/tools/status.tool.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Notes:** Options: (a) import version from package.json at runtime, (b) use
  build-time constant via esbuild/rollup define, (c) generate version.ts during
  build. Option (a) is simplest for ESM packages.

### MCPH-004: Add empty plan factory to anvil-runtime

- **Intent:** Remove `{} as unknown as PlanData` double-cast in gate tool
- **Expected Outcome:** `@eddacraft/anvil-runtime` exports `createEmptyPlan()`
  or `runGate` accepts optional plan parameter
- **Validation:** `pnpm -F anvil-runtime test && pnpm -F mcp-server test`
- **Files:** `packages/anvil/runtime/src/gate-runner.ts`,
  `packages/mcp-server/src/tools/gate.tool.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P2
- **Notes:** Requires coordinated change across two packages. The runtime
  currently tolerates missing fields but this is undocumented internal behavior.

### MCPH-005: Add Content-Type validation to HTTP transport

- **Intent:** Return clear error when POST /mcp receives non-JSON content type
- **Expected Outcome:** Server returns 415 Unsupported Media Type with helpful
  message when Content-Type is not application/json
- **Validation:**
  `pnpm -F mcp-server test -- --testNamePattern="content.type|415"`
- **Files:** `packages/mcp-server/src/transports/streamable-http.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Notes:** Express 5's `express.json()` silently passes non-JSON requests.
  Add explicit middleware check before JSON parsing.

### MCPH-006: Include explanation field in check tool output

- **Intent:** Provide consistent warning context between tools and resources
- **Expected Outcome:** `anvil_check` tool includes `explanation` field in
  warning output, matching `anvil://patterns` resource
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="check"`
- **Files:** `packages/mcp-server/src/tools/check.tool.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Notes:** One-line change to include `explanation` in the warning mapping.

### MCPH-007: Verify VS Code MCP config type

- **Intent:** Ensure VS Code config generator uses correct transport type
- **Expected Outcome:** VS Code HTTP config uses correct `type` value per
  current VS Code MCP documentation
- **Validation:** Manual test with VS Code MCP extension
- **Files:** `packages/mcp-server/src/config/vscode.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P3
- **Notes:** Current config uses `type: "sse"` which may be outdated. Need to
  check VS Code MCP extension docs for current schema. May need to be `"http"`
  or `"streamable-http"`.

### MCPH-008: Document MCP protocol error semantics

- **Intent:** Clarify that resource errors are returned in JSON body per MCP
  protocol
- **Expected Outcome:** REVIEW.md or code comments document that 200 status
  with error in JSON is intentional MCP behavior
- **Validation:** Manual review
- **Files:** `packages/mcp-server/REVIEW.md` or resource files
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Notes:** This is standard MCP behavior, not a bug. Documentation prevents
  future confusion.

### MCPH-009: Add rate limiting for non-localhost HTTP deployments

- **Intent:** Protect HTTP transport from abuse when exposed beyond localhost
- **Expected Outcome:** HTTP transport supports optional rate limiting
  configuration; defaults off for localhost, warns if enabled for non-localhost
  without rate limits
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="rate.limit"`
- **Files:** `packages/mcp-server/src/transports/streamable-http.ts`
- **Dependencies:** MCPH-005
- **Confidence:** low
- **Priority:** P3
- **Notes:** Only needed if server is exposed beyond localhost. Current scope
  is localhost-only, so this is low priority. Consider express-rate-limit
  middleware.

## Risks

| Risk                        | Impact | Mitigation                              |
| --------------------------- | ------ | --------------------------------------- |
| P1-1 breaks existing setups | Medium | Make validation opt-in or localhost-exempt |
| Version sync adds build complexity | Low | Use runtime import (simplest approach) |
| Runtime changes affect other consumers | Medium | Ensure backward compatibility |
