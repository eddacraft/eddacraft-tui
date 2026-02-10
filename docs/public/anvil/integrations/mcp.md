---
id: mcp
title: MCP Integration
description: Using Anvil with Model Context Protocol servers.
sidebar_position: 3
---

# MCP Integration

Anvil provides an MCP (Model Context Protocol) server for AI agent integration.

## What is MCP?

MCP is a protocol for providing context to AI models. Anvil's MCP server
exposes:

- Current project configuration
- Active gates and their status
- Task constraints and scope
- Validation endpoints

## Starting the MCP Server

```bash
anvil mcp serve
```

The server listens on stdin/stdout for MCP messages.

## Configuration

Add Anvil to your MCP configuration:

```json
{
  "mcpServers": {
    "anvil": {
      "command": "anvil",
      "args": ["mcp", "serve"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

## Available Tools

### anvil_validate

Validate a file or set of files:

```json
{
  "tool": "anvil_validate",
  "arguments": {
    "files": ["src/auth/login.ts"]
  }
}
```

Response:

```json
{
  "status": "pass",
  "checks": [
    { "name": "architecture", "status": "pass" },
    { "name": "anti-patterns", "status": "pass" }
  ]
}
```

### anvil_get_constraints

Get constraints for a task:

```json
{
  "tool": "anvil_get_constraints",
  "arguments": {
    "task": "AUTH-001"
  }
}
```

Response:

```json
{
  "task": "AUTH-001",
  "outcome": "Users can log in with email/password",
  "allowed_files": ["src/auth/**"],
  "forbidden_patterns": ["src/payments/**"],
  "validation_command": "pnpm test src/auth/"
}
```

### anvil_check_scope

Check if a file is within task scope:

```json
{
  "tool": "anvil_check_scope",
  "arguments": {
    "task": "AUTH-001",
    "file": "src/auth/login.ts"
  }
}
```

Response:

```json
{
  "in_scope": true,
  "reason": "File matches pattern src/auth/**"
}
```

### anvil_get_issues

Get current issues in the project:

```json
{
  "tool": "anvil_get_issues",
  "arguments": {}
}
```

Response:

```json
{
  "issues": [
    {
      "file": "src/utils/parser.ts",
      "line": 42,
      "code": "AP-003",
      "message": "Explicit 'any' type"
    }
  ]
}
```

## Resources

The MCP server exposes resources:

### anvil://config

Current Anvil configuration:

```json
{
  "uri": "anvil://config",
  "content": {
    "version": "1.0",
    "gates": { ... }
  }
}
```

### anvil://status

Current validation status:

```json
{
  "uri": "anvil://status",
  "content": {
    "last_run": "2024-01-15T10:30:00Z",
    "status": "pass",
    "issues_count": 0
  }
}
```

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

### anvil_explain_issue

Explain an issue code:

```json
{
  "prompt": "anvil_explain_issue",
  "arguments": {
    "code": "AP-003"
  }
}
```

Response includes explanation, examples, and fix suggestions.

---

**Next:** [Configuration reference →](/anvil/operations/config)
