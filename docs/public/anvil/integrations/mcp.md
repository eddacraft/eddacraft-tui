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

For HTTP transport (e.g. remote or multi-client setups):

```json
{
  "mcpServers": {
    "anvil": {
      "command": "npx",
      "args": ["anvil-mcp-server-http"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

## Available Tools

### anvil_check

Validate files against architecture rules and anti-patterns:

```json
{
  "tool": "anvil_check",
  "arguments": {
    "files": ["src/auth/login.ts"]
  }
}
```

### anvil_gate

Run the full gate pipeline (lint, test, coverage, architecture, policy):

```json
{
  "tool": "anvil_gate",
  "arguments": {}
}
```

### anvil_fix

Auto-fix a specific violation:

```json
{
  "tool": "anvil_fix",
  "arguments": {
    "file": "src/auth/login.ts",
    "code": "AP-003"
  }
}
```

### anvil_suppress

Suppress a warning with an explanation:

```json
{
  "tool": "anvil_suppress",
  "arguments": {
    "file": "src/auth/login.ts",
    "code": "AP-003",
    "reason": "Third-party API returns untyped data"
  }
}
```

### anvil_status

Get the current workspace validation status:

```json
{
  "tool": "anvil_status",
  "arguments": {}
}
```

### anvil_query_boundary

Query architecture boundary rules for a file or module:

```json
{
  "tool": "anvil_query_boundary",
  "arguments": {
    "file": "src/api/handlers/user.ts"
  }
}
```

## Resources

The MCP server exposes read-only resources:

| Resource                | Description                       |
| ----------------------- | --------------------------------- |
| `anvil://config`        | Current `.anvilrc` configuration  |
| `anvil://status`        | Last validation status            |
| `anvil://baseline`      | Current baseline snapshot         |
| `anvil://boundaries`    | Architecture boundary definitions |
| `anvil://constraints`   | Active task constraints and scope |
| `anvil://drift`         | Drift snapshots                   |
| `anvil://file-warnings` | Per-file warning list             |
| `anvil://patterns`      | Anti-pattern definitions          |
| `anvil://suppressions`  | Active suppressions               |

## Example: Agent Loop

An AI agent using Anvil MCP:

```python
from mcp import Client

anvil = Client("anvil")

# 1. Get task constraints
constraints = anvil.call("anvil_get_constraints", task="AUTH-001")

# 2. Generate code within constraints
code = generate_code(constraints)

# 3. Write to file
write_file("src/auth/login.ts", code)

# 4. Validate
result = anvil.call("anvil_validate", files=["src/auth/login.ts"])

if result["status"] != "pass":
    # Retry with feedback
    issues = result["checks"]
    code = regenerate_code(constraints, issues)
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
