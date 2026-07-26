---
id: agent-skills
title: Agent skills
description:
  Install and verify the managed anvil developer-functions skill in supported
  clients.
---

# Agent skills

anvil can install a managed **agent skill** into supported AI clients so those
clients know how to call anvil's developer functions safely.

The bundled skill is `anvil-developer-functions`. It teaches the client when to
use local graph and write-validation tools; it does not replace MCP
configuration or prove protection on its own.

## Install the managed skill

```text
anvil skill install
```

In an interactive terminal, anvil prompts for scope and detected clients. For
scripts, name them explicitly:

```text
anvil skill install --client claude-code --scope global
anvil skill install --client cursor --client codex --scope project
```

Useful flags:

- `--client <id>` — repeat to select more than one client.
- `--scope global|project` — global is the interactive default.
- `--verify` — check an existing managed install without writing.
- `--dry-run` — preview destinations without writing.

```text
anvil skill install --client claude-code --dry-run
anvil skill install --client cursor --verify
```

If `skill` is not listed in `anvil --help`, your installed binary is older than
this surface. Upgrade, then re-check.

## Check freshness

After upgrades, run:

```text
anvil doctor
```

Doctor reports managed-skill state (for example fresh, stale, dirty, unmanaged,
absent, or broken). Reinstall with `anvil skill install` when the report says
the managed copy is stale or broken. Do not hand-edit managed skill directories
if you want doctor to keep treating them as managed.

## Relationship to MCP

Skills and MCP are complementary:

1. Configure the client with [MCP integration](mcp.md) so it can call anvil.
2. Install the skill so the client has a maintained procedure for those tools.
3. Verify protection with `anvil start --verify`.

A skill alone does not activate pre-write protection.

## Next step

Use [AI-assisted write protection](../guides/agent-harness.md) for the full
client workflow.
