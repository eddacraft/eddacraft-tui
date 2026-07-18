---
id: agent-harness
title: Protect AI-assisted writes
description: Connect a supported AI client to local pre-write validation.
---

# Protect AI-assisted writes

**For:** Cursor or Claude Code users

**Time:** about 10 minutes

**Outcome:** the client can ask anvil to validate a proposed write before it
lands

MCP means **Model Context Protocol**, a standard way for an AI client to call a
local tool.

## Before you begin

Complete the [quickstart](../quickstart.md), including sign-in.

## 1. Activate detected clients

```text
anvil start
```

By default, anvil configures supported clients it detects. To prepare both
supported clients even when one is not currently detected, use:

```text
anvil start --all-mcp-clients
```

## 2. Restart when asked

If the final state is `ready_restart_required`, fully quit and reopen the named
client. Then run:

```text
anvil start --verify
```

Success is `protecting`. A `watching` result means save-time validation is
available but pre-write protection is not proven.

## 3. Confirm the connection

Inspect the available MCP operations without changing configuration:

```text
anvil mcp --help
```

The current CLI provides `install` and `serve`. Connection readiness is reported
by `anvil start --verify`, not by a separate MCP status command.

## Corporate or restricted environments

If client configuration is not allowed, activate without MCP changes:

```text
anvil start --no-mcp
```

Then use [save-time validation](save-time-validation.md) and terminal gates.

## Next step

Run the [first finding tutorial](../first-gate.md) from the same project.
