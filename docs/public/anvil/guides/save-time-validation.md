---
id: save-time-validation
title: Use save-time validation
description:
  Run anvil as a foreground watcher and understand the save-time assurance
  level.
owner: DSV
upstream:
  - crates/anvil-cli/src/commands/watch.rs
  - crates/anvil-cli/src/commands/watch_save_time.rs
  - crates/anvil-cli/src/ast_followup.rs
  - crates/anvil-cli/src/intercept_policy_evaluator.rs
  - crates/anvil-intercept/src/validate_paths.rs
  - plans/decisions/149-save-time-policy-evaluator-hook.md
verified_against: 0.11.1-beta
---

# Use save-time validation

**For:** users whose editor cannot provide pre-write validation, or who want a
visible save-time loop

**Time:** 5 minutes

**Outcome:** changed files get a save-time regex-and-secrets pass after each
save. Installed policy packs can also emit diagnostics when the CLI-started
daemon injected the policy hook. That is not an AST pass, not a merge gate, and
not an interrupt before the write lands.

## Start the watcher

For a project that has completed activation:

```text
anvil watch
```

Wait for the ready message before editing a file.

## Prove it works

1. Start the watcher.
2. Edit a supported source file.
3. Save it.
4. Confirm the watcher names the file and reports findings or an explicit clean
   result. A clean save is regex plus secrets. When packs were evaluated, daemon
   scope shows as `antipattern+policy` (`check_families` includes `policy`);
   `coverage: certified` remains graph plus antipattern.

Use the [ten-minute tutorial](../first-gate.md) for a safe deliberate finding.

## Assurance boundary

Save-time validation runs **after** the editor writes the file. It is useful
fallback protection, but it is not equivalent to stopping an unsafe AI write
before it reaches disk.

The save-time catalogue is regex anti-patterns and secrets. **No AST.** When
`anvil start` / `anvil intercept start` injected the policy evaluator **and an
enabled pack was actually evaluated**, packs can add diagnostics and
`check_families` includes `policy` (daemon scope `antipattern+policy`). No
packs, discovery failure, or `ANVIL_POLICY_ENFORCEMENT=off` returns
`evaluated: false` and withholds the policy family. `coverage: certified`
remains graph plus antipattern. Save-time does not interrupt; the file is
already on disk. MCP `anvil_validate_write` still owns interrupt-before-write. A
daemon started without that CLI hook stays regex and secrets only.

Default `anvil watch` action is `check`, not `gate`. After an allow, watch may
print one stderr line if a background AST follow-up finds something; that line
does not block the write. Run `anvil check --changed` when you want regex and
AST anti-patterns, plus secrets, on demand, and `anvil gate` when you need a
merge judgement.

The evaluation layers are in
[How anvil evaluates a project](../concepts/evaluation-model.md).

## Common problems

- **No event appears:** confirm the file extension is supported and the watcher
  reports ready.
- **The terminal is non-interactive:** add `--no-tui` for plain output.
- **The daemon cannot start:** run `anvil doctor`, then use
  [troubleshooting](../operations/troubleshooting.md).
- **You need to stop:** press Ctrl-C once and wait for the command to exit.

## Next step

Add [Git hooks](../operations/git-hooks.md) or
[continuous integration](../integrations/github.md) as a later safety net.

## Related definitions

- [How anvil evaluates a project](../concepts/evaluation-model.md)
- [Policy model](../concepts/policy-model.md)
- [Watch output reference](../integrations/watch-output.md)
- [Catch a saved change](../tutorials/first-save-caught.md)
