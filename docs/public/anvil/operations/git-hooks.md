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
verified_against: 0.9.0-beta
---

# Add Git hooks

**For:** Git repositories that already pass a manual anvil gate

**Time:** 5 minutes

**Outcome:** local commits run the quality gate and L3 witness; pushes run L4
validation

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

If the repository uses Husky:

```text
anvil hooks install --husky
```

Do not use `--force` until you have inspected existing hooks and know what would
be replaced.

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
