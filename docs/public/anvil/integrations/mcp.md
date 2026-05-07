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
primary MCP surface, backed by the local Anvil daemon over owner-only IPC. It
exposes the `anvil_validate_write` tool for pre-write validation — see
[Available Tools](#available-tools) below. The daemon-backed path is Unix-first
in this release; the embedded scanner is the correctness-equivalent fallback
when the daemon is not reachable.

The legacy Node.js MCP server (`@eddacraft/anvil-mcp-server`, last published at
`0.4.0-beta`) is retained as a compatibility surface for the broader legacy
tool catalogue (`anvil_check`, `anvil_gate`, `anvil_fix`, `anvil_suppress`,
`anvil_status`, `anvil_query_boundary`). Use it only when you specifically
need one of those tools — see [Legacy Node MCP path](#legacy-node-mcp-path).

:::

## What is MCP?

MCP is a protocol for providing context to AI models. anvil's MCP server
exposes:

- Pre-write validation via `anvil_validate_write` (Rust shim, daemon-backed
  on Unix; embedded fallback otherwise)
- Current project configuration and status (legacy Node tools)
- Architecture boundaries and drift snapshots (legacy Node tools)
- Prompts for architecture review and violation fixes (legacy Node tools)

## What `anvil start` Does to Your MCP Config

For Cursor or Claude Code, the easiest path is `anvil start`. The activator
calls into the same `mcp install` machinery internally for the supported
clients, writing `~/.cursor/mcp.json` and `~/.claude.json` (Claude Code's
canonical config location), then probes whether the editor's MCP transport
can reach the shim. Pass `--verify` for a read-only probe that prints the
diagnostic without writing anything:

```bash
anvil start            # activate Cursor + Claude Code if installed
anvil start --verify   # probe state, no writes
```

If you only need to re-run the install in isolation, use `anvil mcp install`
directly (next section). For HTTP transport or workspace-scoped paths against
Cursor or Claude Code, use `anvil mcp-config` further down. Windsurf and VS
Code are not currently `mcp-config` targets in v0.6.0-beta — see the manual
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

The installer is restricted to Cursor and Claude Code. `anvil mcp-config`
below covers the same two clients with extra knobs (HTTP transport,
workspace-scoped paths). For Windsurf or VS Code, hand-write the
configuration using the [Manual Configuration](#manual-configuration) section
— the previous `mcp-config` Windsurf and VS Code targets were removed in
LAUNCH-009.5 (Windsurf was never protocol-verified; the VS Code emitter
wrote to the pre-1.99 file shape and silently no-op'd on current builds).

## Generate Configuration with `anvil mcp-config`

`anvil mcp-config` generates the right configuration shape for each editor,
supports stdio and HTTP transports, and can write or verify the on-disk file
directly.

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

Pass `--transport http` to switch a client to HTTP transport when the editor
supports it; stdio remains the default.

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

The legacy Node.js MCP server remains available for the broader legacy tool
surface described below, but it is not what `anvil mcp-config` writes for the
Rust stdio launch path.

For HTTP transport (e.g. remote or multi-client setups), start the HTTP server
and configure your client to connect via URL:

```json
{
  "mcpServers": {
    "anvil": {
      "url": "http://localhost:3000/mcp"
    }
  }
}
```

Start the server separately:

```bash
ANVIL_MCP_PORT=3000 npx --package @eddacraft/anvil-mcp-server anvil-mcp-server-http
```

:::note Node.js version

The MCP server requires **Node.js 18+** and **npm 7+** (for the `--package` flag
used by `npx`). Run `node --version` and `npm --version` to verify.

:::

Configure the port and host with `ANVIL_MCP_PORT` (default: 3000) and
`ANVIL_MCP_HOST` (default: localhost).

## Available Tools

### anvil_validate_write

Served by the Rust `anvil mcp serve --stdio` shim. Validates a proposed file
write before the agent applies it; the response carries a `decision` (`allow`
or `block`) and the same `anvil.diagnostic.v1` envelope used by the gate
output.

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

The response includes a `correlation` envelope. The
`correlation.daemonStatus` field reports whether the daemon-backed validation
path is live:

| Value         | Meaning                                                           |
| ------------- | ----------------------------------------------------------------- |
| `available`   | Daemon reachable; tool ran via the daemon-backed path             |
| `unavailable` | Daemon-backed client probed but not reachable; embedded fallback  |
| `not-wired`   | Daemon validation client not compiled in (Windows in v0.6.0-beta) |

:::caution Windows daemon-backed path

In `v0.6.0-beta`, `correlation.daemonStatus` is always `not-wired` on Windows
because the validation client is `cfg(unix)`-gated. The embedded fallback runs
the same checks; the daemon-backed correlation envelope is part of the
follow-up Windows named-pipe work.

:::

The remaining tools (`anvil_check`, `anvil_gate`, `anvil_fix`, `anvil_suppress`,
`anvil_status`, `anvil_query_boundary`) live on the legacy Node MCP server —
see [Legacy Node MCP path](#legacy-node-mcp-path) below.

## Legacy Node MCP path

The legacy Node MCP server (`@eddacraft/anvil-mcp-server`) remains available
for the broader legacy tool surface. The Rust shim does not expose these
tools today; reach for the legacy server only when you specifically need one
of them.

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
