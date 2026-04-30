---
id: mcp
title: MCP Integration
description: Using anvil with Model Context Protocol servers.
sidebar_position: 3
---

# MCP Integration

anvil provides an MCP (Model Context Protocol) server for AI agent integration.

:::info Two MCP server paths

As of 0.5.0-beta the Rust CLI ships `anvil mcp serve --stdio`, the launch path
for the new write-validation surface. `anvil mcp install --client cursor` (or
`--client claude-code`) wires the editor up in one step. Today the Rust shim
exposes the `anvil_validate_write` tool — see
[Available Tools](#available-tools) below.

The legacy Node.js MCP server (`@eddacraft/anvil-mcp-server`, last published at
`0.4.0-beta`) still provides the broader tool surface — `anvil_check`,
`anvil_gate`, `anvil_fix`, `anvil_suppress`, `anvil_status`,
`anvil_query_boundary`, plus resources and prompts — and is the right choice
when you need any of those today.

:::

## What is MCP?

MCP is a protocol for providing context to AI models. anvil's MCP server
exposes:

- Current project configuration and status
- Architecture boundaries and drift snapshots
- Validation tools for files, gates, and boundaries
- Prompts for architecture review and violation fixes

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

The installer is restricted to Cursor and Claude Code. For Windsurf, VS Code, or
any setup that needs HTTP transport or workspace-scoped paths, use
`anvil mcp-config` below.

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
| Windsurf    | `windsurf`    | stdio             |
| VS Code     | `vscode`      | stdio             |

Pass `--transport http` to switch a client to HTTP transport when the editor
supports it; stdio remains the default.

Use `--workspace <path>` to override the project root that anvil records in the
generated config. If `--write` would overwrite an existing config, anvil prompts
before performing an atomic write so you can review the change. Use `--verify`
in CI or pre-commit to fail when a checked-in config has drifted from what
`anvil mcp-config` would generate today.

## Manual Configuration

If you'd rather wire anvil up by hand, the configuration shapes below are what
`anvil mcp-config` writes for each client.

Add anvil to your MCP configuration:

```json
{
  "mcpServers": {
    "anvil": {
      "command": "npx",
      "args": ["@eddacraft/anvil-mcp-server"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

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

Served by the Rust `anvil mcp serve --stdio` shim shipped with the CLI in
0.5.0-beta. Validates a proposed file write before the agent applies it; the
response carries a `decision` (`allow` or `block`) and the same
`anvil.diagnostic.v1` envelope used by the gate output.

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

The remaining tools (`anvil_check`, `anvil_gate`, `anvil_fix`, `anvil_suppress`,
`anvil_status`, `anvil_query_boundary`) are served by the legacy Node MCP server
(`@eddacraft/anvil-mcp-server`). The Rust shim does not expose them yet.

### anvil_check

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

Optional `checks` parameter limits which checks run (default: all):

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

### anvil_gate

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

### anvil_fix

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

### anvil_suppress

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

### anvil_status

Get the current workspace validation status:

```json
{
  "tool": "anvil_status",
  "arguments": {
    "workspaceRoot": "/absolute/path/to/project"
  }
}
```

### anvil_query_boundary

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

## Resources

The MCP server exposes read-only resources:

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

## Example: Agent Loop

An AI agent using anvil MCP:

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

## Prompts

The MCP server provides helpful prompts:

| Prompt                | Description                                   |
| --------------------- | --------------------------------------------- |
| `architecture-review` | Review a file's architecture boundary context |
| `fix-violation`       | Explain a violation and suggest a fix         |
| `pre-generation`      | Provide constraints before generating code    |
| `suppress-violation`  | Guide suppression with a proper explanation   |

---

**Next:** [Configuration reference →](/anvil/operations/config)
