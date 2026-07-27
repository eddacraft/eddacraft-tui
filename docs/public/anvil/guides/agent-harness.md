---
id: agent-harness
title: Protect AI-assisted writes
description: Connect a supported AI client to local pre-write validation.
---

# Protect AI-assisted writes

**For:** users of a supported AI coding client

**Time:** about 10 minutes

**Outcome:** the client can ask anvil to validate a proposed write before it
lands

MCP means **Model Context Protocol**, a standard way for an AI client to call a
local tool.

## Before you begin

Complete the [quickstart](../quickstart.md), including sign-in.

## 1. Activate guided clients

```text
anvil start
```

On a real terminal this opens the consent-first activation surface. By default
the guided path configures **Cursor** and **Claude Code** when they are
detected. Nothing is written unless you select it.

To prepare both guided clients even when one is not currently detected:

```text
anvil start --all-mcp-clients
```

To install a guided client explicitly:

```text
anvil mcp install --client cursor
```

See [Model Context Protocol integration](../integrations/mcp.md) for verify
options and for how newer betas expand the client registry beyond the guided
pair. Always confirm ids with:

```text
anvil mcp install --help
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

Optionally install a managed [agent skill](../integrations/skills.md) when your
binary exposes the skill surface (`anvil --help` lists `skill`). Skills are
complementary to MCP and do not prove protection alone.

## Corporate or restricted environments

If client configuration is not allowed, activate without MCP changes:

```text
anvil start --no-mcp
```

Then use [save-time validation](save-time-validation.md) and terminal gates.

## Next step

Run the [first finding tutorial](../first-gate.md) from the same project.
