# Rust MCP Launch Shim Contract

**Status:** Accepted for RMCP-001  
**Date:** 2026-04-28  
**APS:** RMCP-001  

## Purpose

Define the launch-sized Rust MCP contract for the current release.

The current release needs one credible path:

```text
anvil mcp install
  -> editor launches anvil mcp serve --stdio
  -> MCP client calls one pre-write validation tool
  -> Rust validation returns canonical diagnostics
  -> client warns or blocks before writing content
```

This is a launch shim, not a port of `archive/anvil-mcp-server`.

## Contract Summary

- Binary: the shipped Rust `anvil` binary.
- Launch command: `anvil mcp serve --stdio`.
- Install command: `anvil mcp install --client cursor|claude-code`.
- Transport: MCP stdio using JSON-RPC 2.0 messages on stdin/stdout.
- Capability: tools only; no resources, no prompts, no HTTP transport.
- Tool: `anvil_validate_write`.
- Validation mode: `pre-write` / ADR-031 `mode = preWrite`.
- Diagnostics: canonical `anvil.diagnostic.v1` payloads from
  `crates/anvil-kernel-types/src/diagnostics.rs`.
- Backend order: daemon validation RPC when available, embedded Rust rule
  pipeline fallback when the daemon path is not ready.

## Non-Goals

The Rust launch shim must not re-open full MCP parity. These existing TypeScript
MCP server surfaces stay out of scope until RMCPF:

| Existing TS surface | Examples | RMCP status |
| --- | --- | --- |
| Tools | `anvil_check`, `anvil_fix`, `anvil_gate`, `anvil_status`, `anvil_suppress`, `anvil_query_boundary` | Not ported |
| Resources | `baseline`, `boundaries`, `config`, `constraints`, `drift`, `file-warnings`, `patterns`, `suppressions` | Not advertised |
| Prompts | `architecture-review`, `fix-violation`, `pre-generation`, `suppress-violation` | Not advertised |
| Transports | streamable HTTP | Not implemented |

The launch shim also does not provide graph context resources, auto-fix,
suppression editing, gate running, status queries, or boundary queries.

## CLI Surface

### `anvil mcp serve --stdio`

Starts the Rust stdio MCP server.

Requirements:

- Reserve stdout exclusively for MCP protocol frames.
- Send all logs, diagnostics about server startup, and panic summaries to stderr.
- Exit cleanly on EOF and on the JSON-RPC `shutdown` / `exit` lifecycle.
- Never require Node.js, pnpm, or `archive/anvil-mcp-server`.
- Load enough workspace context to validate supplied paths, but do not scan the
  workspace at startup.

Signal-specific shutdown handling is deferred until the server owns long-running
tool execution to unwind. The RMCP-002 server must still avoid writing anything
other than JSON-RPC frames to stdout when the process is externally terminated.

### Auth and licence policy

`anvil mcp serve --stdio` is not licence-gated at process startup. Editor MCP
clients launch this command non-interactively, so startup must not emit auth
prompts or licence errors on stdout. Tool-level entitlement and policy decisions
must be returned as structured MCP tool responses once RMCP adds tool execution.

### `anvil mcp install --client cursor|claude-code`

Writes an MCP server entry named `anvil` that launches:

```json
{
  "command": "anvil",
  "args": ["mcp", "serve", "--stdio"],
  "env": {}
}
```

`--verify` confirms the configured entry uses command `anvil` and args
`["mcp", "serve", "--stdio"]`. RCLI3-016 already established the lower-level
config generation shape; RMCP-007 owns the wrapper command and client-specific
verification.

## MCP Protocol Subset

The server implements only enough MCP for Cursor and Claude Code to discover and
call the launch tool.

| Method | Direction | Behaviour |
| --- | --- | --- |
| `initialize` | client -> server | Return server metadata and tools capability only. |
| `notifications/initialized` | client -> server | Acknowledge readiness without response. |
| `tools/list` | client -> server | Return exactly `anvil_validate_write`. |
| `tools/call` | client -> server | Execute `anvil_validate_write`; reject all other tool names. |
| `shutdown` | client -> server | Return success and prepare for `exit`. |
| `exit` | client -> server notification | Terminate cleanly. |
| `ping` | either direction | Return an empty success response when called as a request. |

For JSON-RPC requests (messages with an `id`), unsupported methods return
JSON-RPC `-32601` (`Method not found`). Malformed JSON returns `-32700`.
Invalid params return `-32602`. Internal operational failures return `-32603`
only when the server cannot construct a tool result. JSON-RPC notifications
(messages without an `id`) do not receive responses, including error responses,
and are ignored as required by JSON-RPC.

The server does not advertise resource or prompt capabilities. If a client
sends `resources/list`, `resources/read`, `prompts/list`, or `prompts/get` as
requests, the server returns `-32601` rather than a partial compatibility shim.
If they are sent as notifications, the server ignores them without response.

## Tool Contract

### Name

`anvil_validate_write`

### Intent

Validate a proposed file write before the MCP client applies it.

### Input

```json
{
  "workspaceRoot": "/absolute/path/to/workspace",
  "path": "src/example.ts",
  "operation": "create",
  "proposedContent": "export const value = 1;\n",
  "patch": null,
  "contentEncoding": "utf-8",
  "client": {
    "name": "cursor",
    "sessionId": "optional-client-session-id"
  }
}
```

| Field | Required | Notes |
| --- | :---: | --- |
| `workspaceRoot` | no | Defaults to server cwd. Must resolve to an existing directory that is the server cwd, a path inside it, or a registered linked Git worktree of the same repository (ADR-125). |
| `path` | yes | Workspace-relative path or absolute path inside `workspaceRoot`. Escapes are rejected. |
| `operation` | yes | `create`, `update`, `delete`, or `rename`. |
| `proposedContent` | conditional | Full proposed UTF-8 content after the operation. Required for `create`/`update` unless `patch` is present. |
| `patch` | conditional | Unified diff or client patch payload. If supplied without `proposedContent`, the server may read the current file and synthesise proposed content. |
| `contentEncoding` | no | `utf-8` by default. Other encodings are reserved for future binary-aware handling and are not advertised by the A1 tool schema. |
| `client` | no | Best-effort client/session metadata for correlation. It is not an auth factor. |

Exactly one of `proposedContent` or `patch` should be supplied for normal
`create` and `update` calls. If both are supplied, `proposedContent` is
authoritative and `patch` is retained only for correlation metadata.

### Output

The tool returns one JSON text content item. `allow` and `warn` are normal tool
results. `block` sets MCP `isError: true` so clients do not silently proceed,
while still returning a structured Anvil payload.

```json
{
  "schema": "anvil.mcp.validate-write.v1",
  "decision": "block",
  "summary": {
    "total": 1,
    "bySeverity": {
      "error": 1,
      "warning": 0,
      "info": 0
    }
  },
  "diagnostics": [
    {
      "schema_version": "anvil.diagnostic.v1",
      "id": "diag_prewrite_src_example_ts_4_secret-detection",
      "severity": "error",
      "summary": "Potential secret detected (AWS Access Key)",
      "location": {
        "file": "src/example.ts",
        "line": 4
      },
      "category": "secret",
      "source": {
        "rule_id": "secret-detection",
        "source_module": "anvil-checks::secret"
      },
      "remediation_hint": "Use a placeholder or environment variable instead.",
      "mode": "pre-write"
    }
  ],
  "correlation": {
    "id": "corr_01HW8K6Q4P0X7N9TJ4YA3S0V",
    "surface": "mcp",
    "mode": "preWrite",
    "backend": "embedded",
    "path": "src/example.ts"
  },
  "safeDefault": "do-not-write"
}
```

`decision` values:

| Decision | Meaning | MCP `isError` |
| --- | --- | :---: |
| `allow` | No blocking or warning diagnostics. | false |
| `warn` | Diagnostics exist, but enforcement config allows the write with warning. | false |
| `block` | Client must not apply the write. | true |

`safeDefault` is always `do-not-write` for `block` and for operational errors.

## Diagnostic and Decision Mapping

The tool does not invent a diagnostic schema. It returns canonical
`anvil.diagnostic.v1` diagnostics and adds only MCP-tool wrapper fields:

- `schema` for the tool response version.
- `decision` for the enforcement outcome.
- `summary` derived from diagnostics.
- `correlation` for log lookup and demo evidence.
- `safeDefault` for agent-readable safety behaviour.

`Diagnostic.severity` is not the same as `decision`. Severity comes from the
rule. The daemon, or embedded fallback using the same policy defaults, maps
severity to `allow`, `warn`, or `block` according to the configured enforcement
mode.

For the current release:

- Secret findings default to `block`.
- Reasoning-pattern findings default to the configured AI guardrail policy; if
  unavailable, they default to `warn` unless explicitly configured to block.
- Empty diagnostics return `allow`.
- Workspace escape, binary content, unsupported encoding, and oversize content
  return `block` with no secret excerpts.

All secret-bearing excerpts must be redacted before they enter `summary`,
`remediation_hint`, logs, or MCP responses. Secret diagnostics may include the
rule id, file path, line number, and a redacted line or match. They must not
include raw secret values.

## Validation Backend Selection

Backend selection is deterministic:

1. If a future daemon pre-write validation RPC is available and compatible,
   call it through the adapter seam.
2. For the launch slice, no concrete daemon pre-write RPC exists yet, so the
   default daemon client reports `Unavailable` and MCP uses the embedded Rust
   validation fallback.
3. Return the same tool response schema from both paths.

The response includes `correlation.backend = "daemon"` or `"embedded"`.

The concrete daemon client belongs to RTAI-002 now that the INTD-002 IPC
listener is pinned. RMCP-005 only ships the adapter seam and the embedded
fallback path.

The embedded fallback exists only for the launch slice. It may call the shared
Rust rule pipeline directly, starting with secret detection and any available A1
reasoning-pattern rules. It must not shell out to Node.js or call
`archive/anvil-mcp-server`.

Operational backend failures return a structured MCP tool error:

```json
{
  "schema": "anvil.mcp.validate-write.v1",
  "error": {
    "code": "validation-backend-unavailable",
    "message": "Anvil could not validate the proposed write.",
    "retriable": true
  },
  "safeDefault": "do-not-write",
  "correlation": {
    "surface": "mcp",
    "mode": "preWrite"
  }
}
```

Clients should treat this as a failed validation and avoid applying the write.

## Stdio Safety

Stdio framing is load-bearing because MCP clients parse stdout as protocol.

Requirements:

- stdout contains only MCP JSON-RPC frames.
- stderr carries human logs and crash summaries.
- JSON-RPC response ids echo request ids exactly.
- Notifications never receive responses.
- Batch requests are rejected for A1 unless a client requires them during
  compatibility testing.
- The server exits successfully on normal EOF after all in-flight responses are
  flushed.

## Validation Evidence for RMCP-001

RMCP-001 is complete when review confirms:

- The only advertised tool is `anvil_validate_write`.
- The only supported MCP capability is tools.
- Existing TypeScript MCP tools, resources, prompts, and HTTP transport are
  explicitly out of scope.
- The response uses canonical diagnostics and separates severity from decision.
- Backend selection is daemon-first with an embedded Rust fallback.
- Secret content is redacted by default.

## References

- `plans/modules/rust-mcp-launch-shim.aps.md`
- `plans/modules/rust-mcp-full-port.aps.md`
- `plans/modules/realtime-ai-validation.aps.md` (`RTAI-006`, `RTAI-008`)
- `plans/specs/2026-04-26-diagnostic-envelope-coordination.md`
- `plans/decisions/031-validation-latency-rubric.md`
- `plans/decisions/030-surface-drivers-supersede-napi-cutover.md`
- `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
