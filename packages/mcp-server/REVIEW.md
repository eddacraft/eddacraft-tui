# MCP Server Code Review

**Package**: `@eddacraft/anvil-mcp-server` **Reviewed commit**: `8ec79e4` (feat:
MCP Server Implementation #237) **Date**: 2026-02-04 **Updated**: 2026-02-05
(verification and fixes)

---

## Overview

The MCP server exposes Anvil's architecture validation and quality gate
functionality as tools, resources, and prompts for AI code editors. It supports
both stdio and HTTP (Streamable HTTP) transports and provides config generation
for Claude Code, Cursor, Windsurf, and VS Code.

**Surface area**: 6 tools, 8 resources, 4 prompts, 2 transports, 4 config
generators, 11 test files.

---

## Strengths

### Architecture & Design

- **Clean separation of concerns**: Tools, resources, prompts, transports, and
  config generators are each in their own directory with barrel exports.
  Registration functions are isolated and composable.
- **Server factory pattern**: `createAnvilMcpServer()` cleanly wires up all
  capabilities and accepts overridable options. The `getWorkspaceRoot()` closure
  pattern for write-tools is a well-chosen approach for pinning the workspace
  root at server-creation time rather than trusting client input.
- **Dynamic imports**: Using `await import(...)` for `@eddacraft/anvil-runtime`
  and `@eddacraft/anvil-core` in tool/resource callbacks avoids loading heavy
  dependencies at module-load time. This is the right pattern for an MCP server
  that may be invoked via `npx`.
- **Dual transport support**: Stdio and Streamable HTTP transports are cleanly
  separated. The HTTP transport correctly manages per-session MCP server
  instances via a `Map` (not a plain object — addressing prototype pollution).

### Security

- **Path traversal protection**: `anvil_fix`, `anvil_suppress`, and
  `file-warnings` all implement two-layer path validation: logical check via
  `resolve`/`relative` plus symlink resolution via `realpathSync`. This is
  thorough.
- **Write tools use server-pinned workspace root**: `anvil_fix` and
  `anvil_suppress` take `getWorkspaceRoot` from the server constructor rather
  than accepting `workspaceRoot` from client input. This prevents untrusted HTTP
  clients from targeting arbitrary filesystem locations.
- **Session management**: HTTP transport uses `Map<string, Transport>` instead
  of a plain object, avoiding prototype pollution via crafted `mcp-session-id`
  headers.
- **Input validation**: All tools use Zod schemas for input validation. The CLI
  validates `--transport` and `--port` ranges before generating config.

### Test Coverage

- Comprehensive test suite across all layers: integration tests
  (server.test.ts), individual tool tests, resource tests, prompt tests,
  transport tests, and config tests.
- Tests use the MCP SDK's `InMemoryTransport` and `Client` for realistic
  end-to-end verification of the MCP protocol flow.
- The fix and suppress tool tests use real filesystem operations in temp
  directories, verifying actual file modifications.
- Edge cases are well-covered: path traversal, out-of-range lines, non-existent
  tools, non-Error thrown values, idempotency.

### Tool Annotations

- Correct annotation of `readOnlyHint`, `destructiveHint`, and `idempotentHint`
  across tools. Write tools (fix, suppress) are correctly marked
  `destructiveHint: true`, and `anvil_suppress` is correctly marked
  `idempotentHint: false` since it adds lines.

---

## Issues Found

### P1 — High Priority

#### 1. `anvil_check` and `anvil_gate` accept `workspaceRoot` from client input

**Files**: `check.tool.ts:19`, `gate.tool.ts:20`, `query-boundary.tool.ts:24`,
`status.tool.ts:18`

The read-only tools (`anvil_check`, `anvil_gate`, `anvil_status`,
`anvil_query_boundary`) all accept `workspaceRoot` as a client-provided
parameter. While the write tools correctly pin `workspaceRoot` via
`getWorkspaceRoot()`, the read tools allow a client to point them at arbitrary
filesystem paths.

For stdio transport, this is low-risk (the client already has local access). For
HTTP transport, an untrusted client could use `anvil_check` or `anvil_gate` to
probe file existence and contents on the server's filesystem. The
`GateRunner.analyzeFiles()` call reads file contents to detect anti-patterns, so
error messages may leak path information.

**Recommendation**: Consider having read-only tools also use the server-pinned
`getWorkspaceRoot()`, or at minimum validate that the provided `workspaceRoot`
is within an allowed set of roots.

#### ~~2. `anvil_fix` AP-003 pattern may produce incorrect code~~ ✅

**File**: `fix.tool.ts:21`

The AP-003 fix uses the regex `/:\s*any\b/g` to replace `: any` with
`: unknown`. This regex will also match inside string literals and comments. For
example:

```ts
const msg = 'type: any value here'; // false positive
// TODO: Replace: any type with proper type  // false positive
```

The regex also doesn't distinguish between `any` as a type annotation vs. in
other syntactic positions. While the tool description says "deterministic
mechanical transforms," false positives in string/comment contexts could
introduce bugs.

**Recommendation**: Document this limitation clearly in the tool description, or
restrict the replacement to only operate when the line appears to contain a
TypeScript type annotation context.

**Fixed:** AP-003 now skips comment lines and strips string literals before
checking for `: any`, then uses a character-by-character parser to only replace
occurrences outside of string contexts.

#### 3. Hardcoded version `'0.1.0'` in multiple places

**Files**: `server.ts:40`, `status.tool.ts:70`

The version string `'0.1.0'` is hardcoded in:

- `server.ts` (default server version for MCP handshake)
- `status.tool.ts` (returned in status response)
- `package.json` (package version)

These can drift. If the package version changes, the status tool and server
handshake will still report `0.1.0`.

**Recommendation**: Import the version from `package.json` or use a build-time
constant to keep these in sync.

### P2 — Medium Priority

#### 4. `gate.tool.ts` uses `{} as unknown as PlanData` for full gate runs

**File**: `gate.tool.ts:71`

```ts
const emptyPlan = {} as unknown as PlanData;
const result = await runner.runGate(emptyPlan, config, workspaceRoot, { ... });
```

The double type assertion (`as unknown as PlanData`) to pass an empty object as
plan data is a code smell. The comment says "the runner tolerates missing fields
and falls back to 'no-hash' when plan.hash is absent," but this relies on
undocumented internal behavior. If `runGate`'s implementation changes, this will
silently break.

**Recommendation**: Either create a proper "empty plan" factory in
`@eddacraft/anvil-runtime`, or add a `runGateWithoutPlan()` method that doesn't
require a plan argument.

#### 5. HTTP transport does not validate `Content-Type` on POST

**File**: `streamable-http.ts:41`

The POST `/mcp` handler doesn't validate that the incoming request has
`Content-Type: application/json`. Express 5's `express.json()` middleware will
silently pass through requests with non-JSON content types, leaving `req.body`
as `undefined`. The `isInitializeRequest(req.body)` check would then return
`false`, causing the server to return a 400 error, but with a generic message
rather than a clear "unsupported content type" error.

**Recommendation**: Add content-type validation or handle undefined `req.body`
explicitly.

#### 6. `bin-http.ts` doesn't validate `ANVIL_MCP_PORT` environment variable

**File**: `bin-http.ts:15`

```ts
const port = parseInt(process.env.ANVIL_MCP_PORT ?? '3000', 10);
```

If `ANVIL_MCP_PORT` is set to a non-numeric string like `"abc"`, `parseInt`
returns `NaN`. This `NaN` is passed to `startHttpServer`, which passes it to
`app.listen()`. Node.js will throw at bind time, but the error message won't be
descriptive.

The CLI's `mcp-config` command correctly validates port ranges (1-65535), but
the direct `bin-http.ts` entry point does not.

**Recommendation**: Add port validation in `bin-http.ts`, matching the CLI's
validation logic.

#### 7. `suppress.tool.ts` — expiry date calculation uses local timezone

**File**: `suppress.tool.ts:64-66`

```ts
const expiry = new Date();
expiry.setDate(expiry.getDate() + days);
const expiryStr = expiry.toISOString().split('T')[0];
```

`setDate()` operates in local time, but `toISOString()` outputs UTC. Near
midnight UTC, this can produce an off-by-one day in the expiry date relative to
local time. For example, at 11:30 PM UTC-5, `new Date()` is 6:30 AM the next day
in UTC, so adding 30 days and taking the ISO date could be off by one.

The test (`suppress.tool.test.ts:78`) sets fake timers to midnight UTC which
masks this edge case.

**Recommendation**: Use UTC-based arithmetic (`setUTCDate`/`getUTCDate`) to be
timezone-independent, or document the UTC behavior.

### P3 — Low Priority / Nits

#### 8. `check.tool.ts` strips `explanation` field from warnings

**File**: `check.tool.ts:47-56`

The `anvil_check` tool maps warnings to include `suggestion` but excludes
`explanation`. The `explanation` field provides additional context about _why_ a
pattern is problematic. Meanwhile, the `anvil://patterns` resource includes
both. There's an inconsistency — an AI consuming the check results would benefit
from having `explanation` available alongside the suggestion.

**Recommendation**: Consider including `explanation` in the check tool output,
or document the omission rationale.

#### 9. VS Code config uses `type: "sse"` for HTTP transport

**File**: `vscode.ts:17`

```ts
type: 'sse',
```

VS Code's MCP support now uses Streamable HTTP transport (as of late 2025). The
`type: "sse"` value refers to the older SSE transport. While many
implementations auto-negotiate between SSE and Streamable HTTP, this could cause
compatibility issues with newer VS Code versions.

**Recommendation**: Verify against current VS Code MCP documentation. The type
may need to be `"http"` or `"streamable-http"` depending on VS Code's expected
schema.

#### 10. Resources don't return HTTP status codes for errors

**Files**: All resource handlers

All resource handlers return errors as JSON in the response body with `200` HTTP
semantics (MCP resources always return `contents[]`). This is standard MCP
behavior, but it means the consumer must inspect the response JSON to detect
errors rather than relying on HTTP status codes. This is fine for MCP protocol
compliance but worth noting.

> **Note (2026-02-15):** This is standard MCP protocol behavior. The MCP
> specification requires that tool and resource errors are conveyed in the
> JSON-RPC response body, not via HTTP status codes. The HTTP layer is only a
> transport; the application-level success/failure semantics live in the
> JSON-RPC `result` or `error` fields. No change needed.

#### 11. `server.test.ts` — tests 1 and 2 in `createAnvilMcpServer` describe block are redundant

**File**: `server.test.ts:87-114`

Tests "returns an McpServer instance with default options" and "returns an
McpServer instance when called with no arguments" both call
`createAnvilMcpServer()` with no arguments and assert
`toBeInstanceOf(McpServer)`. These are duplicates.

#### 12. Missing SIGTERM handler in stdio `bin.ts`

**File**: `bin.ts`

The HTTP entry point (`bin-http.ts`) handles both `SIGINT` and `SIGTERM` for
graceful shutdown. The stdio entry point (`bin.ts`) handles neither. If the MCP
client sends SIGTERM, the server will terminate without cleanup. While stdio
servers typically rely on the parent process lifecycle, adding signal handlers
would improve robustness for containerized deployments.

#### 13. No rate limiting or request size limits on HTTP transport

**File**: `streamable-http.ts`

The HTTP transport uses `express.json()` without a `limit` option. Express 5
defaults to a 100KB body limit, which should be sufficient, but there's no
explicit rate limiting. For a local-only MCP server this is acceptable, but
worth noting if the server is ever exposed beyond localhost.

---

## Summary

| Category     | Found  | Fixed | Remaining |
| ------------ | ------ | ----- | --------- |
| P1 (High)    | 3      | 1     | 2         |
| P2 (Medium)  | 4      | 2     | 2         |
| P3 (Low/Nit) | 6      | 2     | 4         |
| **Total**    | **13** | **5** | **8**     |

The implementation is well-structured, thoroughly tested, and demonstrates good
security awareness. The main areas for improvement are:

1. **Read-tool workspace root validation** for HTTP transport scenarios
2. **Regex false-positive risk** in the AP-003 fix pattern
3. **Version string synchronization** across the package

Overall, this is solid work for a v0.1.0 release. The security fixes applied
during the PR review process (path traversal, symlink resolution, session Map,
write-tool workspace pinning) show good iterative improvement.

---

## Fixes Applied (2026-02-05)

The following issues were fixed during review verification:

| Issue                                      | Status   | Fix                                                                          |
| ------------------------------------------ | -------- | ---------------------------------------------------------------------------- |
| P2-6: `bin-http.ts` port validation        | ✅ Fixed | Added validation for `ANVIL_MCP_PORT` env var with descriptive error message |
| P2-7: `suppress.tool.ts` timezone issue    | ✅ Fixed | Changed to use `setUTCDate()`/`getUTCDate()` for consistent UTC arithmetic   |
| P3-11: Duplicate test in `server.test.ts`  | ✅ Fixed | Merged redundant tests into single test case                                 |
| P3-12: Missing signal handlers in `bin.ts` | ✅ Fixed | Added SIGINT and SIGTERM handlers for graceful shutdown                      |

### Remaining Open Issues

The following issues remain open for team discussion:

- **P1-1**: Read-tool workspace root validation — Design decision for HTTP
  transport security model
- ~~**P1-2**: AP-003 regex false positives~~ ✅ — Fixed with string-aware parser
- **P1-3**: Version string synchronization — Requires build-time tooling
- **P2-4**: Empty plan double-cast — Requires changes to
  `@eddacraft/anvil-runtime`
- **P2-5**: Content-Type validation — Low impact, optional improvement
- **P3-8**: Missing explanation field — Design decision
- **P3-9**: VS Code SSE type — Needs verification against current VS Code MCP
  docs
- **P3-10**: Resources HTTP status codes — MCP protocol standard behavior
- **P3-13**: Rate limiting — Only needed if exposed beyond localhost
