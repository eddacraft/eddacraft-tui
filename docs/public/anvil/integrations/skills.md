---
id: agent-skills
title: Agent skills
description:
  Install and verify the managed anvil developer-functions skill in supported
  clients when your binary exposes it.
---

# Agent skills

Managed **agent skills** teach a supported AI client how to call anvil's
developer functions safely. They do not replace MCP configuration or prove
protection on their own.

The bundled skill name is `anvil-developer-functions`.

## Check whether your binary supports skills

```text
anvil --help
```

If `skill` is not listed, the managed-skill surface is not in your installed
version. The public 0.9.0-beta binary does not ship it; newer betas after that
release add `anvil skill install`. Upgrade, then re-check help before following
the rest of this page.

## Install the managed skill (when available)

When `skill` appears in top-level help, open that subcommand's help on the same
binary for client ids, scope, verify, and dry-run flags. Typical shape after the
skill surface ships:

- interactive install prompts for scope and detected clients;
- scripts pass explicit `--client` values (repeatable) and optional
  `--scope global|project`;
- `--verify` checks an existing managed install without writing;
- `--dry-run` previews destinations without writing.

Do not copy install flags from an older or newer release note without checking
your binary.

## Check freshness

After upgrades, run:

```text
anvil doctor
```

When managed skills are present, doctor can report freshness (for example fresh,
stale, dirty, unmanaged, absent, or broken). Reinstall through the skill command
when the report says the managed copy is stale or broken. Do not hand-edit
managed skill directories if you want doctor to keep treating them as managed.

## Relationship to MCP

Skills and MCP are complementary:

1. Configure the client with [MCP integration](mcp.md) so it can call anvil.
2. Install the skill (when available) so the client has a maintained procedure
   for those tools.
3. Verify protection with `anvil start --verify`. On later days, bare `anvil`
   turns protection on without reinstalling.

A skill alone does not activate pre-write protection.

## Next step

Use [AI-assisted write protection](../guides/agent-harness.md) for the full
client workflow.
