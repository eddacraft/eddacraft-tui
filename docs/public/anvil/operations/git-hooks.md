---
id: git-hooks
title: Git hook setup
description: How Anvil interacts with file-based and Git 2.54 native config-based hooks.
sidebar_position: 4
---

# Git hook setup

Anvil supports two hook installation modes: the long-standing **file
mode** (writing scripts into `.husky/` or `.git/hooks/`) and an opt-in
**config mode** that drives Git 2.54's native `[hook.<name>]` config
blocks. This page summarises which mode applies when. The detailed
compatibility baseline lives in
[`docs/guides/git-hook-compatibility.md`](../../guides/git-hook-compatibility.md).

## Which mode should I use?

| Situation                                  | Recommended mode | Why                                            |
| ------------------------------------------ | ---------------- | ---------------------------------------------- |
| Existing repo with Husky / lint-staged     | File mode        | No migration required; Anvil auto-detects      |
| Plain `.git/hooks/` setup                  | File mode        | Same script that you would write by hand       |
| You want multiple commands per hook event  | Config mode      | Native composition without wrapper scripts     |
| You share hooks across worktrees / clones  | Config mode      | Hooks travel with `git config`, not files      |

Config mode requires Git **2.54 or newer** on the machine that runs
the hook. File mode works on every Git version Anvil officially
supports (2.30+). See the [compatibility
policy](../../guides/git-hook-compatibility.md#compatibility-baseline)
for the full version matrix.

## Default behaviour

`anvil hooks install` installs file-mode hooks today. The future
`--config` flag (delivered by GHOOK-002) opts into native config-mode
without changing the default.

When both file hooks and config-hook entries exist for the same event,
Git runs **both** — Anvil's status and doctor surfaces flag this as a
duplicate-execution risk and recommend choosing one. Anvil never edits
`.git/hooks/` to wire native hooks; config-mode work goes through
`git config --add hook.<name>.command`.

## Verifying your Git version

```bash
git --version
```

If the output is `git version 2.54.0` or higher, both modes are
available. If it is older, file mode still works; install or update Git
before opting into config mode.

## Further reading

- [Git hook compatibility policy](../../guides/git-hook-compatibility.md)
  — the canonical compatibility baseline and rollout policy.
- [CI Integration tutorial](../tutorials/ci.md) — pre-commit and CI
  examples.
- [Agent Harness guide](../guides/agent-harness.md) — using Anvil as a
  pre-commit guardrail for AI-assisted workflows.
