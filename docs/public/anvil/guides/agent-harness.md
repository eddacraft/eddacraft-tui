---
id: agent-harness
title: Protect AI-assisted writes
description: Connect a supported AI client to local pre-write validation.
owner: MCPX
upstream:
  - crates/anvil-cli/src/commands/mcp_installer.rs
  - crates/anvil-cli/src/activation/agent_registry.rs
  - crates/anvil-cli/src/commands/start.rs
verified_against: 0.9.0-beta
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

## 1. Activate and connect a client

```text
anvil start
```

On a real terminal this opens the consent-first activation surface. The MCP list
includes every supported client; nothing is written unless you select it.

To install or verify one client without a full interactive run, use a client id
from your binary's help:

```text
anvil mcp install --help
anvil mcp install --client cursor
anvil mcp install --client cursor --verify
```

See [Model Context Protocol integration](../integrations/mcp.md) for
multi-client options such as `--all-mcp-clients` and `--no-mcp`. Always confirm
ids with `anvil mcp install --help` on the binary you installed.

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

## 4. Day two without reinstalling

After activation, bare `anvil` turns protection on for the project (daemon +
already-configured MCP). It does not open the client picker. Use `anvil start`
only when you need to add a client or repair configuration.

## Corporate or restricted environments

If client configuration is not allowed, activate without MCP changes:

```text
anvil start --no-mcp
```

Then use [save-time validation](save-time-validation.md) and terminal gates.

## Next step

Run the [first finding tutorial](../first-gate.md) from the same project.
