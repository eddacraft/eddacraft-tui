---
id: uninstall
title: Uninstalling Anvil
description:
  How to remove Anvil from a project or machine using anvil uninstall.
sidebar_position: 5
---

# Uninstalling Anvil

`anvil uninstall` removes Anvil's files and configuration from your project or
machine in one step. It is useful when you want a clean slate before
reinstalling, when removing Anvil from a project permanently, or when
troubleshooting a stuck or half-installed state.

The command is **project-scoped by default** — it only touches the current
repository. Adding `--global` also removes your user-level state, credentials,
and MCP entries.

## Project uninstall (default)

Running `anvil uninstall` in a repository removes:

- `.anvil/` — project state directory
- `.anvilrc` — project configuration file
- Anvil-managed Git hooks (`pre-commit`, `pre-push`) in both file mode
  (`.git/hooks/`, `.husky/`) and config mode (`hook.<name>.command` entries
  added by Anvil)

Non-Anvil hooks and your own project files are never touched.

```bash
# Preview what would be removed (no changes applied)
anvil uninstall --dry-run

# Remove with interactive confirmation
anvil uninstall

# Remove without prompting
anvil uninstall --yes
```

## Full uninstall (--global)

`--global` extends the project uninstall with user-level cleanup:

- `~/.anvil/` — user state directory (project caches, activation markers)
- MCP server entries from `~/.claude.json` (Claude Code) and
  `~/.cursor/mcp.json` (Cursor) — Anvil's own entry only; all other entries are
  preserved
- Stored authentication credentials
- The running `anvil-intercept` daemon (SIGTERM, then SIGKILL after one second
  if it does not exit cleanly)

```bash
# Preview full uninstall, including user state
anvil uninstall --global --dry-run

# Full uninstall with confirmation
anvil uninstall --global

# Full uninstall without touching MCP config files
anvil uninstall --global --keep-mcp
```

## Removing the binary

`anvil uninstall` does not remove the `anvil` binary itself — use the tool you
installed with:

| Install method | How to remove the binary             |
| -------------- | ------------------------------------ |
| Homebrew       | `brew uninstall eddacraft/tap/anvil` |
| Cargo          | `cargo uninstall anvil`              |
| curl installer | `rm ~/.eddacraft/bin/anvil`          |
| Winget         | `winget uninstall eddacraft.anvil`   |
| Scoop          | `scoop uninstall anvil`              |

As of `v0.7.1-beta`, uninstall recognises Scoop and WinGet install roots when it
plans cleanup on Windows and keeps removal bounded to the detected install root.

Run `anvil uninstall --global` first so any running daemon is stopped before you
remove the binary.

## Options reference

| Option          | Short | Description                                                        |
| --------------- | ----- | ------------------------------------------------------------------ |
| `--dry-run`     | `-n`  | Show what would be removed; make no changes                        |
| `--yes`         | `-y`  | Skip the interactive confirmation prompt                           |
| `--global`      |       | Also remove user-level state, credentials, MCP entries, and daemon |
| `--keep-mcp`    |       | Skip MCP config edits even when `--global` is set                  |
| `--keep-daemon` |       | Do not attempt to stop the running daemon                          |
| `--force`       |       | Continue past per-step errors instead of stopping                  |
| `--json`        |       | Output results as JSON (requires `--yes` for non-dry-run)          |
| `--verbose`     | `-v`  | Enable verbose logging                                             |
| `--no-tui`      |       | Plain-text output; disables TUI rendering                          |

## Automation and scripting

Use `--json --yes` for non-interactive environments. `--json` requires `--yes`
on non-dry-run invocations to prevent a confirmation prompt from blocking a
script.

```bash
# Dry run — no confirmation needed
anvil uninstall --dry-run --json

# Unattended project-scope removal
anvil uninstall --yes --json

# Unattended full removal
anvil uninstall --global --yes --json
```

The JSON envelope contains a `plan` array describing each action and an
`outcomes` array with the result of each step (`Removed`, `NotPresent`, or
`Failed`).

## Reinstalling after uninstall

After a project uninstall, run `anvil init` to re-initialise the project. After
a full uninstall, re-run the installer first:

```bash
# macOS / Linux
curl -fsSL https://install.eddacraft.ai | sh

# Windows (PowerShell)
irm https://install.eddacraft.ai/windows | iex
```

Then `anvil init` in each project you want to bring back.

## Troubleshooting

### Uninstall stopped mid-way through

If an error stops uninstall before all steps complete, rerun with `--force` to
continue past the failing step:

```bash
anvil uninstall --force
```

Check `anvil doctor` afterwards to confirm the state is clean.

### Symlink in .anvil/ or .anvilrc

`anvil uninstall` refuses to remove symlinks and reports the path so you can
handle it manually. Remove or resolve the symlink, then rerun.

### Daemon did not stop

If the daemon was not running or the PID file is stale, the daemon step exits
cleanly with a `NotPresent` outcome. If the daemon is running but unresponsive,
send SIGKILL directly:

```bash
kill -9 "$(cat ~/.anvil/daemon.pid)"
```

Then rerun `anvil uninstall --global --keep-daemon` to clean up the remaining
state.

## Further reading

- [Git hook setup](./git-hooks.md) — file mode vs config mode hooks
- [Troubleshooting](./troubleshooting.md) — stuck daemon and other runtime
  issues
- [Quickstart](../quickstart.md) — reinstalling from scratch
