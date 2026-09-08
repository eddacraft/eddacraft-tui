---
id: git-hooks
title: Add Git hooks
description:
  Install, inspect, and remove anvil-managed pre-commit, post-commit, and
  pre-push checks.
owner: GHOOK
upstream:
  - crates/anvil-cli/src/commands/hooks.rs
  - crates/anvil-hook/src/lib.rs
  - crates/anvil-hook/src/coexistence.rs
  - crates/anvil-cli/src/policy_load.rs
  - crates/anvil-cli/src/commands/status.rs
verified_against: 0.9.0-beta
---

# Add Git hooks

**For:** Git repositories that already pass a manual anvil gate

**Time:** 5 minutes

**Outcome:** local commits run the quality gate and L3 witness; pushes run L4
validation when a parseable `anvil/policy.*` is present

## Before you begin

Run the intended gate manually. Do not install a hook that the team cannot
reproduce or recover from.

## Install managed hooks

```text
anvil hooks install
```

This installs the default pre-commit, post-commit, and pre-push hooks.
Pre-commit still runs `anvil gate --progress` and also runs
`anvil hook pre-commit` (L3 witness append). Post-commit runs
`anvil hook post-commit` so HEAD is SHA-bound. Pre-push stays on
`anvil hook pre-push`. `--config` mode installs the same verbs.
`--pre-commit-only` installs the commit-side pair (pre-commit + post-commit):

```text
anvil hooks install --pre-commit-only
anvil hooks install --pre-push-only
```

Pre-push L4 validation runs only when `anvil/policy.*` is present and parseable.
Fresh `anvil init` writes a default. `anvil status` reports L4 `on` only when
that file and an active pre-push hook are both present. To force the L4 engine
to run, see [Exercise L4](../concepts/policy-model.md#exercise-l4).

If the repository uses Husky:

```text
anvil hooks install --husky
```

Do not use `--force` until you have inspected existing hooks and know what would
be replaced.

## Silent pre-push pass

`git push` can succeed with no anvil output when any of these hold:

1. **No policy (or no project-id).** L4 is a no-op until `anvil/policy.*` and
   `anvil/project-id` exist. This is designed, not a missed block.
2. **Clean allowed range.** The pushed commits satisfy policy, so the hook stays
   silent.
3. **`command -v anvil` guard.** The installed wrapper exits 0 when Git's hook
   PATH does not contain `anvil`. Git's hook environment often has a different
   PATH from an interactive shell. On Windows this commonly shows up as:
   `Get-Command anvil` works in PowerShell, but `git push` still proceeds
   because Git for Windows ran the hook with a PATH that does not include
   `anvil`. The skip is always exit 0; output depends on the wrapper: bootstrap
   `shell_template` and the Husky managed block print nothing;
   `anvil hooks install` file hooks print
   `anvil not found on PATH, skipping hook`.

Git invokes the hook _script_ as `pre-push <remote> <url>` and writes the ref
lines to stdin. Wrappers that forward `"$@"` pass those two positionals to
`anvil hook pre-push`; `anvil hooks install` file hooks do not. The positionals
are informational. The stdin contract is unchanged.

## Inspect status

```text
anvil hooks status
```

Success means each intended hook is present and anvil identifies whether it owns
it. If multiple hook managers are present, avoid running the same gate twice.

## Remove managed hooks

```text
anvil hooks uninstall
```

anvil should leave hooks it does not own untouched. Confirm with
`anvil hooks status`.

## Native Git hook configuration

Some versions can use Git's native hook configuration with `--config`. Check
`anvil hooks install --help` and your Git version before choosing that mode. Use
one mode per event.

## Next step

Add [continuous integration](../integrations/github.md) as the shared authority.

## Related definitions

- [How anvil evaluates a project](../concepts/evaluation-model.md)
- [Use save-time validation](../guides/save-time-validation.md)
- [CLI command reference](../reference/cli.md)
