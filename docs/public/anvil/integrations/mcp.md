---
id: mcp
title: MCP Integration
description: Using Anvil with Model Context Protocol servers.
sidebar_position: 3
---

# MCP Integration

Anvil provides an MCP (Model Context Protocol) server for AI agent integration.

:::info Node.js package required

The MCP server is a separate Node.js package (`@eddacraft/anvil-mcp-server`),
not part of the Rust CLI binary. You need Node.js and npx (or a package manager)
to run it. A built-in `anvil mcp serve` command is planned for a future release
of the Rust CLI.

:::

## What is MCP?

MCP is a protocol for providing context to AI models. Anvil's MCP server
exposes:

- Current project configuration and status
- Architecture boundaries and drift snapshots
- Validation tools for files, gates, and boundaries
- Prompts for architecture review and violation fixes

## Configuration

Add Anvil to your MCP configuration:

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

An AI agent using Anvil MCP:

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
