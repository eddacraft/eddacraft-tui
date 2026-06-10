---
id: git-hooks
title: Git hook setup
description:
  How Anvil interacts with file-based and Git 2.54 native config-based hooks.
sidebar_position: 4
---

# Git hook setup

Anvil supports two hook installation modes: the long-standing **file mode**
(writing scripts into `.husky/` or `.git/hooks/`) and an opt-in **config mode**
that drives Git 2.54's native `[hook.<name>]` config blocks. This page
summarises which mode applies when. The detailed compatibility baseline lives in
the contributor-facing
[Git hook compatibility policy](https://github.com/eddacraft/anvil/blob/main/docs/guides/git-hook-compatibility.md)
(in the source repository).

## Which mode should I use?

| Situation                                 | Recommended mode | Why                                        |
| ----------------------------------------- | ---------------- | ------------------------------------------ |
| Existing repo with Husky / lint-staged    | File mode        | No migration required; Anvil auto-detects  |
| Plain `.git/hooks/` setup                 | File mode        | Same script that you would write by hand   |
| You want multiple commands per hook event | Config mode      | Native composition without wrapper scripts |
| You share hooks across worktrees / clones | Config mode      | Hooks travel with `git config`, not files  |

Config mode requires Git **2.54 or newer** on the machine that runs the hook.
File mode works on every Git version Anvil officially supports (2.30+). See the
[compatibility policy](https://github.com/eddacraft/anvil/blob/main/docs/guides/git-hook-compatibility.md#compatibility-baseline)
for the full version matrix.

## Default behaviour

`anvil hooks install` installs file-mode hooks today. The `--config` flag opts
into native config-mode without changing the default.

For a hand-written file-mode hook, the staged-only check keeps pre-commit fast:

```bash
# .husky/pre-commit (or .git/hooks/pre-commit)
anvil check --changed --staged
```

The `--changed --staged` flags restrict analysis to staged files, and the
managed config-mode hook installs `ANVIL_HOOK=1 anvil gate --progress` instead —
keep the manual script if you want staged-only checks.

When both file hooks and config-hook entries exist for the same event, Git runs
**both**, so choose one mode per event to avoid duplicate execution. Anvil
surfaces this as a structured warning from `anvil hooks install --config`,
`anvil hooks uninstall --config`, and `anvil hooks status` — see
[Coexistence](#coexistence) below for the full behaviour. Anvil never edits
`.git/hooks/` to wire native hooks; config-mode goes through
`git config --add hook.<name>.command`.

## Verifying your Git version

```bash
git --version
```

If the output is `git version 2.54.0` or higher, both modes are available. If it
is older, file mode still works; install or update Git before opting into config
mode.

## Coexistence

Anvil keeps file-mode and config-mode hooks side by side without ever editing,
removing, or refusing entries it does not own. The behaviour is deliberately
warning-first.

### How Git resolves multiple hook sources

| Situation                                         | What Git does                                                   |
| ------------------------------------------------- | --------------------------------------------------------------- |
| `.git/hooks/<event>` only                         | Runs the script.                                                |
| `hook.<event>.command` config entry only          | Runs the command (Git 2.54+).                                   |
| Both `.git/hooks/<event>` AND a config-mode entry | Runs **both**. Anvil flags this as a duplicate-execution risk.  |
| `core.hooksPath` is set                           | Resolves file-mode hooks from that path; `.git/hooks/` ignored. |
| `core.hooksPath` set AND a config-mode entry      | Runs the configured `hooksPath` script AND the config entry.    |

`core.hooksPath` only redirects file-mode lookup. It does not disable
config-mode entries — those are independent.

### What Anvil does on coexistence

- **Detects** every signal at install, uninstall, and status time. The detection
  produces a structured report covering: file-mode paths found
  (`.git/hooks/<event>`, `.husky/<event>`), third-party hook managers (Husky,
  Lefthook, the pre-commit framework), the count of foreign
  `hook.<event>.command` entries, and the value of `core.hooksPath` if set.
- **Warns** when both modes are present for the same event so you know Git will
  run both. The warning lists each source explicitly.
- **Never edits foreign entries.** A `hook.<event>.command` value that does not
  start with Anvil's marker (`ANVIL_HOOK=1 anvil gate`) is treated as foreign
  and is not touched by `install --config` or `uninstall --config`. The
  uninstall path uses Git's value-pattern filter so only Anvil-owned entries are
  removed.
- **Adds alongside.** `install --config` appends Anvil's entry via
  `git config --add hook.<event>.command`. Multi-valued config keys are
  preserved — running `git config --get-all` after install returns every entry,
  in the order they were added.

### Status output, explained

`anvil hooks status` prints:

1. The familiar file-mode rows (`.git/hooks/<event>`, `.husky/<event>`) with
   their installed/Anvil-managed/missing label.
2. A **Coexistence** block per event when there is anything to report. This
   block surfaces:
   - Duplicate-execution risk when a file-mode hook and any config-mode entry
     are both present, naming each source.
   - Other hook managers detected (`Husky`, `Lefthook`, `pre-commit framework`)
     so you see the full picture even when Anvil is not running them.
   - Foreign config-mode entries (count) — entries Anvil did not install and
     will never edit.
   - `core.hooksPath` value when set.

The same data is available under the `coexistence` key in
`anvil hooks status --json` for automation.

### What Anvil never does

- Override Git's own precedence rules. `core.hooksPath` is yours; we report it,
  we do not change it.
- Remove or rewrite a hook script Anvil did not create. File-mode hooks are
  identified by the `# @anvil-managed` marker; config-mode entries are
  identified by the `ANVIL_HOOK=1 anvil gate` prefix.
- Refuse `install --config` because another hook manager is present. The flag
  always emits a warning and proceeds — see the
  [scope-guard "warnings over blocks" rule](https://github.com/eddacraft/anvil/blob/main/docs/vision/anvil-scope-guard.md).

## Further reading

- [Git hook compatibility policy](https://github.com/eddacraft/anvil/blob/main/docs/guides/git-hook-compatibility.md)
  — the canonical contributor-facing compatibility baseline and rollout policy.
- [GitHub integration guide](../integrations/github.md) — CI pipelines, SARIF
  upload, and branch protection.
- [Agent Harness guide](../guides/agent-harness.md) — using Anvil as a
  pre-commit guardrail for AI-assisted workflows.
