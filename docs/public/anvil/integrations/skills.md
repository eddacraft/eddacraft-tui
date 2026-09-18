---
id: agent-skills
title: Agent skills
description:
  Install and verify the managed anvil skills in supported clients when your
  binary exposes them.
owner: SKPKG
upstream:
  - crates/anvil-cli/src/commands/skill.rs
  - crates/anvil-cli/src/commands/skill_state.rs
verified_against: 0.11.1-beta
---

# Agent skills

Managed **agent skills** teach a supported AI client how to operate anvil. They
do not replace MCP configuration or prove protection on their own.

`anvil skill install` writes two skills into each selected client root:

- `anvil-developer-functions` — in-session graph-context tools and the pre-write
  validation gate
- `using-anvil` — setup, `anvil check` / `anvil gate`, doctor, CI, and light
  config

The edit-loop skill names `using-anvil` as its companion; both must be present
for that hand-off to load.

## Check whether your binary supports skills

```text
anvil --help
```

`0.9.1-beta` and later list `skill` in top-level help. If your installed binary
does not, upgrade and re-check help before following the rest of this page.

## Install the managed skills

`anvil start` installs these skills for every MCP client you tick (or pass with
`--mcp-client`). Clients with no documented skill location are skipped without
failing activation. Repeat or repair with:

Open the subcommand help on the same binary for the current client ids, scope,
verify, and dry-run flags:

```text
anvil skill install --help
```

The current surface supports:

- interactive install prompts for scope and detected clients;
- scripts pass explicit `--client` values (repeatable) and optional
  `--scope global|project`;
- `--verify` checks an existing managed install without writing;
- `--dry-run` previews destinations without writing.

Additional flags (including managed-copy refresh when your binary ships it)
appear in `anvil skill install --help` on that same binary.

For a non-interactive scripted fleet, enumerate every destination by repeating
`--client`. Omitting it is an error when clients are detected, so a script
cannot silently write to every detected harness:

```text
anvil skill install --client claude-code --client codex --client opencode
```

Do not copy install flags from an older or newer release note without checking
your binary.

## Check freshness

`anvil update` refreshes existing managed skill copies after a successful
upgrade (and when the binary is already current). Dirty or unmanaged directories
are skipped. `--check` does not write.

```text
anvil update
```

A package-manager upgrade that never runs `anvil update` (for example a bare
`brew upgrade`) does not refresh skills. Repair those by running `anvil update`,
or — when your binary lists it under `anvil skill install --help` — refreshing
managed copies in place. You can also run:

```text
anvil doctor
```

When managed skills are present, doctor can report freshness (for example fresh,
stale, dirty, unmanaged, absent, or broken). Reinstall through the skill command
when the report says the managed copy is stale or broken. If install refuses an
unmanaged skill directory or entry, move that content outside the relevant
skills directory tree (or to another path the client does not scan). Do not
hand-edit managed skill directories if you want doctor to keep treating them as
managed.

## Relationship to MCP

Skills and MCP are complementary:

1. Configure the client with [MCP integration](mcp.md) so it can call anvil.
   Interactive `anvil start` also installs the managed skills for the MCP
   clients you choose.
2. Repair or refresh later with `anvil update` (or the managed-refresh flag
   listed by `anvil skill install --help` on your binary) if doctor reports
   stale copies. Unmanaged directories still need a manual move before a normal
   install.
3. Verify protection with `anvil start --verify`. On later days, bare `anvil`
   turns protection on without reinstalling.

A skill alone does not activate pre-write protection.

## Next step

Use [AI-assisted write protection](../guides/agent-harness.md) for the full
client workflow.

## Related definitions

- [How anvil evaluates a project](../concepts/evaluation-model.md)
- [CLI command reference](../reference/cli.md)
