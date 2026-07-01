---
id: mcp
title: MCP Integration
description: Using anvil with Model Context Protocol servers.
sidebar_position: 3
---

# MCP Integration

anvil provides an MCP (Model Context Protocol) server for AI agent integration.

:::info Rust shim is the primary surface

As of `v0.6.0-beta`, the Rust CLI's `anvil mcp serve --stdio` shim is the
primary MCP surface, backed by the local Anvil daemon over owner-only IPC for
validation. It exposes `anvil_validate_write` for pre-write validation,
`anvil_status` for read-only workspace health, and read-only graph-context tools
for assistant planning — see [Available Tools](#available-tools) below. The
daemon-backed validation path uses Unix sockets on Linux/macOS and owner-only
named pipes on Windows as of `v0.7.1-beta`; the embedded scanner is the
correctness-equivalent fallback when the daemon is not reachable. As of
`v0.8.0-beta`, `anvil watch` routes its save-time checks through the same daemon
validation path by default when the daemon is live (`ANVIL_WATCH_DAEMON=0` opts
out; see the [save-time validation guide](../guides/save-time-validation.md) for
the full routing story), so editor/agent MCP writes and terminal watch converge
on one warm verdict path instead of two separate scanners. As of `v0.8.2-beta`,
an interactive `anvil start` can auto-start that daemon and `anvil watch` can
offer to start one, so the daemon-backed path is the normal one rather than
something you launch by hand — see the
[daemon lifecycle](../guides/save-time-validation.md#daemon-lifecycle).

The legacy Node.js MCP server (`@eddacraft/anvil-mcp-server`, last published at
`0.4.0-beta`) is no longer the recommended runtime path. Its mutation-oriented
tool and prompt catalogue is frozen historical compatibility material; the
active Rust surface now carries validation, status, and graph context.

:::

## What is MCP?

MCP is a protocol for providing context to AI models. anvil's MCP server
exposes:

- Pre-write validation via `anvil_validate_write` (Rust shim, daemon-backed on
  every supported OS when the daemon is reachable; embedded fallback otherwise)
- Workspace health summaries via `anvil_status` (Rust shim, local read-only
  status with explicit no-daemon provenance)
- Assistant graph context via identity-only tools and `graph://` resources for
  symbols, dependency/caller traversal, impact reports, affected-test hints, and
  bounded symbol context

## What `anvil start` Does to Your MCP Config

For Cursor or Claude Code, the easiest path is `anvil start`. The activator
calls into the same `mcp install` machinery internally for supported clients it
detects on the host, writing the matching editor config (`~/.cursor/mcp.json`
for Cursor or `~/.claude.json` for Claude Code), then probes whether the
editor's MCP transport can reach the shim. Pass `--all-mcp-clients` or set
`ANVIL_ALL_MCP_CLIENTS=1` to opt into configuring every supported client even if
it was not detected. Pass `--verify` for a read-only probe that prints the
diagnostic without writing anything:

```bash
anvil start            # activate detected Cursor / Claude Code installs
anvil start --verify   # probe state, no writes
```

If you only need to re-run the install in isolation, use `anvil mcp install`
directly (next section). For workspace-scoped paths against Cursor or Claude
Code, use `anvil mcp-config` further down. Windsurf and VS Code are not
currently `mcp-config` targets in the current Rust CLI — see the manual
configuration section.

## One-Step Install with `anvil mcp install`

For Cursor or Claude Code, the simplest path is the built-in installer in the
Rust binary. It writes the editor config and points it at the bundled
`anvil mcp serve --stdio` shim:

```bash
# Configure Cursor
anvil mcp install --client cursor

# Configure Claude Code
anvil mcp install --client claude-code

# Verify an existing entry instead of writing
anvil mcp install --client cursor --verify
```

The installer is restricted to Cursor and Claude Code. `anvil mcp-config` below
covers the same two clients with workspace-scoped path overrides. For Windsurf
or VS Code, hand-write the configuration using the
[Manual Configuration](#manual-configuration) section — the previous
`mcp-config` Windsurf and VS Code targets were removed in LAUNCH-009.5 (Windsurf
was never protocol-verified; the VS Code emitter wrote to the pre-1.99 file
shape and silently no-op'd on current builds).

## Generate Configuration with `anvil mcp-config`

`anvil mcp-config` generates the right stdio configuration shape for each
editor, and can write or verify the on-disk file directly.

```bash
# Print the generated config for inspection
anvil mcp-config --target claude-code

# Write it to the client's expected path (with a path-safety prompt)
anvil mcp-config --target claude-code --write

# Verify the on-disk config matches what anvil expects
anvil mcp-config --target claude-code --verify
```

Supported targets:

| Client      | `--target`    | Default transport |
| ----------- | ------------- | ----------------- |
| Claude Code | `claude-code` | stdio             |
| Cursor      | `cursor`      | stdio             |

Use `--workspace <path>` to override the project root that anvil records in the
generated config. If `--write` would overwrite an existing config, anvil prompts
before performing an atomic write so you can review the change. Use `--verify`
in CI or pre-commit to fail when a checked-in config has drifted from what
`anvil mcp-config` would generate today.

## Manual Configuration

If you'd rather wire anvil up by hand, the configuration shapes below match the
current Rust stdio path that `anvil mcp-config` writes for each client.

Add anvil to your MCP configuration:

```json
{
  "mcpServers": {
    "anvil": {
      "command": "anvil",
      "args": ["mcp", "serve", "--stdio"],
      "env": {}
    }
  }
}
```

HTTP transport for the Rust MCP server is not shipped in this release. RMCPF-021
owns the decision on whether Streamable HTTP returns with the Rust full port. If
you maintain a private legacy Node MCP deployment, configure your client to
connect to that service directly; this is frozen compatibility, not the active
Anvil setup path.

```json
{
  "mcpServers": {
    "anvil": {
      "url": "http://localhost:3000/mcp"
    }
  }
}
```

## Available Tools

The Rust MCP server has two active halves:

- **Validation** — `anvil_validate_write` is the launch gate for proposed
  writes. It can allow or block a change before an assistant writes to disk.
- **Graph context** — the graph tools and resources below are read-only planning
  aids. They return context, never an enforcement decision.

### anvil_validate_write

Served by the Rust `anvil mcp serve --stdio` shim. Validates a proposed file
write before the agent applies it; the response carries a `decision` (`allow` or
`block`) and the same `anvil.diagnostic.v1` envelope used by the gate output.

```json
{
  "tool": "anvil_validate_write",
  "arguments": {
    "workspaceRoot": "/absolute/path/to/project",
    "path": "src/auth/login.ts",
    "operation": "create",
    "proposedContent": "export const login = …"
  }
}
```

The response includes a `correlation` envelope. The `correlation.daemonStatus`
field reports whether the daemon-backed validation path is live:

| Value         | Meaning                                                          |
| ------------- | ---------------------------------------------------------------- |
| `available`   | Daemon reachable; tool ran via the daemon-backed path            |
| `unavailable` | Daemon-backed client probed but not reachable; embedded fallback |
| `not-wired`   | Daemon validation client not compiled in for this runtime        |

:::info Windows daemon-backed path

In `v0.7.1-beta`, Windows MCP validation reaches the daemon through owner-only
named pipes and can return the same `protection_claim` as Unix. If
`correlation.daemonStatus` is `unavailable`, the embedded fallback still runs
the same checks; use `anvil intercept status` to inspect daemon health directly.

:::

### anvil_status

Read-only workspace health summary. Its response keeps the legacy fields
(`status`, `workspaceRoot`, `availableChecks`, `config`, `hasBaseline`, and
`version`) but redacts path values to workspace-relative forms and includes
local backend/daemon provenance.

### Assistant graph-context tools

These tools query the daemon's resident graph so an assistant can understand the
codebase before it edits. They are identity-only by default: names, kinds,
workspace-relative paths, visibility, and edges can cross the boundary; source
text, absolute paths, secrets, operator identity, and raw trust levels do not.

| Tool                     | Use                                                                          |
| ------------------------ | ---------------------------------------------------------------------------- |
| `anvil_search_symbols`   | Find symbols by name, kind, file, language, or visibility.                   |
| `anvil_find_dependents`  | List files that import a file, with bounded hop distance.                    |
| `anvil_find_callers`     | List symbols that call a target symbol, marked heuristic when approximate.   |
| `anvil_impact_of_change` | Report affected symbols, dependent files, and known tests for changed paths. |
| `anvil_affected_tests`   | Suggest test files importing changed files and call out coverage gaps.       |
| `anvil_symbol_context`   | Return bounded neighbourhood context around a symbol or file.                |

`anvil_symbol_context` is the only graph tool that can return source snippets.
Snippets require both workspace/operator consent and `includeSource: true` on
the request. Use `anvil gctx egress enable`, `status`, and `disable` to manage
persisted workspace consent; `ANVIL_GCTX_EGRESS=0` disables the whole graph
context surface for the process, while leaving it unset keeps the safe
identity-only default unless consent has been recorded.

### Graph resources

For MCP clients that prefer resources, the same graph is exposed as:

| Resource          | Use                                                         |
| ----------------- | ----------------------------------------------------------- |
| `graph://stats`   | Symbol, edge, and file counts.                              |
| `graph://symbols` | Paged symbol identity summaries; optionally scoped by file. |
| `graph://edges`   | Paged graph edges with bounded-result signalling.           |

Graph tools and resources can return named outcomes instead of data: `ready`,
`not_ready`, `unavailable`, `disabled`, or `invalid_query`. A `not_ready` result
means the daemon is warming the graph and a retry is usually enough;
`unavailable` means the daemon is not reachable.

## Frozen legacy Node MCP catalogue

The archived Node MCP server (`@eddacraft/anvil-mcp-server`) carried the broader
legacy tool surface below. Treat this section as a compatibility inventory for
RMCPF and existing private deployments, not as active setup guidance. Except for
the Rust `anvil_status` port noted above, the Rust shim does not expose these
tools today.

### Legacy Node MCP Tools

#### anvil_check

Validate files against architecture rules and anti-patterns:

```json
{
  "tool": "anvil_check",
  "arguments": {
    "files": ["src/auth/login.ts"],
    "workspaceRoot": "/absolute/path/to/project"
  }
}
```

Optional `checks` parameter limits which checks run (default: all). These are
legacy tool parameters, not the canonical Rust CLI check names:

```json
{
  "tool": "anvil_check",
  "arguments": {
    "files": ["src/auth/login.ts"],
    "checks": ["architecture", "antipattern"],
    "workspaceRoot": "/absolute/path/to/project"
  }
}
```

#### anvil_gate

Run the full gate pipeline (lint, test, coverage, architecture, policy):

```json
{
  "tool": "anvil_gate",
  "arguments": {
    "workspaceRoot": "/absolute/path/to/project"
  }
}
```

Optional parameters: `targetFiles` (specific files), `skipChecks` (checks to
skip), `failFast` (stop on first failure).

#### anvil_fix

Auto-fix a specific violation:

```json
{
  "tool": "anvil_fix",
  "arguments": {
    "filePath": "src/auth/login.ts",
    "warningId": "AP-003",
    "line": 42
  }
}
```

#### anvil_suppress

Suppress a warning with an explanation:

```json
{
  "tool": "anvil_suppress",
  "arguments": {
    "filePath": "src/auth/login.ts",
    "warningId": "AP-003",
    "line": 42,
    "reason": "Third-party API returns untyped data"
  }
}
```

Optional `expiryDays` parameter sets when the suppression expires (default: 30).

#### anvil_status

Get the current workspace validation status:

```json
{
  "tool": "anvil_status",
  "arguments": {
    "workspaceRoot": "/absolute/path/to/project"
  }
}
```

#### anvil_query_boundary

Check whether an import between two files is allowed by architecture rules:

```json
{
  "tool": "anvil_query_boundary",
  "arguments": {
    "sourceFile": "src/api/handlers/user.ts",
    "targetFile": "src/repositories/user.repo.ts",
    "workspaceRoot": "/absolute/path/to/project"
  }
}
```

### Legacy Node MCP Resources

The legacy Node MCP server exposes read-only resources:

| Resource                       | Description                                   |
| ------------------------------ | --------------------------------------------- |
| `anvil://config`               | Current gate configuration and enabled checks |
| `anvil://baseline`             | Architecture baseline snapshot                |
| `anvil://boundaries`           | Architecture boundary rules                   |
| `anvil://constraints`          | Aggregated constraints for AI consumption     |
| `anvil://drift`                | Current architecture drift status             |
| `anvil://file/{path}/warnings` | Warnings for a specific file (template)       |
| `anvil://patterns`             | Anti-pattern catalogue                        |
| `anvil://suppressions`         | Active suppressions with expiry dates         |

These resource names describe the legacy Node surface. Where old resources are
ported, their data source is the active Rust surface: constraints from
`anvil export --format mcp-resource`, drift from `anvil drift`, and suppressions
from the Rust `.anvil/suppressions.json` readers. The archived TypeScript
`runtime/export` pipeline is historical fixture material only.

### Legacy Node MCP Agent Loop

An AI agent using the legacy Node MCP server:

```python
# Pseudocode — adapt to your agent framework
from mcp import Client

anvil = Client("anvil")
root = "/absolute/path/to/project"

# 1. Read boundary rules before generating code
boundaries = anvil.read("anvil://boundaries")

# 2. Generate code respecting boundaries
code = generate_code(boundaries)

# 3. Write to file
write_file("src/auth/login.ts", code)

# 4. Validate against architecture and anti-patterns
result = anvil.call("anvil_check", {
    "files": ["src/auth/login.ts"],
    "workspaceRoot": root
})

if result["status"] != "pass":
    # Retry with feedback from validation warnings
    warnings = result["warnings"]
    code = regenerate_code(warnings)
    # ...
```

### Legacy Node MCP Prompts

The legacy Node MCP server provides helpful prompts:

| Prompt                | Description                                   |
| --------------------- | --------------------------------------------- |
| `architecture-review` | Review a file's architecture boundary context |
| `fix-violation`       | Explain a violation and suggest a fix         |
| `pre-generation`      | Provide constraints before generating code    |
| `suppress-violation`  | Guide suppression with a proper explanation   |

---

**Next:** [Configuration reference →](/anvil/operations/config)
