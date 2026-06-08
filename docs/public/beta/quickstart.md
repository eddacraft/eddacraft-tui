---
id: quickstart
title: Beta Quickstart
description: Get up and running with the Anvil beta in 10 minutes.
sidebar_position: 1
slug: /
---

# Beta Quickstart

Install Anvil, activate protection in a real repo, watch an AI write get
blocked, and start giving feedback -- all in about 10 minutes.

:::info Beta release

This is **pre-release software** (`0.6.0-beta`). The CLI is a single native
binary -- no Node.js required. APIs and behaviour may change between releases.
Your feedback directly shapes the product before public launch.

:::

## Prerequisites

- A real repository you know well (TypeScript, JavaScript, or Rust gets the
  strongest coverage; SQL and Markdown get partial coverage; Python is
  unsupported in v1)
- **macOS**, **Linux**, or **Windows** (x86_64 or aarch64)
- Cursor or Claude Code installed if you want to test the MCP catch path

## Step 1 -- Install

:::info Sign up first

Don't have access yet? [Request an invite](https://eddacraft.ai/#waitlist) to
join the next cohort.

:::

```bash
# macOS / Linux
curl -fsSL https://install.eddacraft.ai | sh

# Windows (PowerShell)
irm https://install.eddacraft.ai/windows | iex

# Or via Homebrew (macOS / Linux)
brew install eddacraft/tap/anvil

# Or via WinGet (Windows)
winget install eddacraft.anvil

# Or via Scoop (Windows)
scoop bucket add eddacraft https://github.com/eddacraft/scoop-bucket
scoop install anvil
```

Verify the install:

```bash
anvil --version
```

If Anvil is already installed through Homebrew, the macOS/Linux curl installer
now exits without replacing it and prints the package-manager command instead:

```bash
brew upgrade eddacraft/tap/anvil
```

Use one install method per machine. To switch from Homebrew to the standalone
installer, uninstall the Homebrew formula first.

## Step 2 -- `anvil start` (the wow-start)

From inside a real repository, `anvil start` is the activation entrypoint. It
runs `anvil init`, scans the repo to baseline existing findings, detects your
repo's language profile, and writes MCP entries so Cursor or Claude Code can
call `anvil_validate_write` before each AI write.

```bash
cd your-project
anvil start
```

The activation summary ends with a literal protection state -- one of
`protecting`, `ready_restart_required`, `watching`, `needs_action`,
`unsupported`, or `error`. Trust that literal: if `anvil start` reports
`needs_action`, pre-write protection is **not** live yet.

`anvil start` writes:

- `~/.cursor/mcp.json` (Cursor)
- `~/.claude.json` (Claude Code)

Restart the editor after activation. On Unix, the MCP path is daemon-backed when
the local daemon is running; the embedded path is a correctness-equivalent
fallback otherwise. On Windows, the daemon path is currently `not-wired` from
the MCP correlation envelope (this is documented v1 scope, not a regression).

**Useful flags:**

```bash
anvil start --verify   # Read-only probe; no init, no scan, no MCP write
anvil start --watch    # After activation, fall back to save-time watch mode
```

`--watch` is a save-time **fallback** -- it spawns the kernel watcher and
reports findings on save. It is not pre-write interception, and `anvil start`
will refuse to spawn it when MCP pre-write validation is already live (it would
just be redundant noise).

The first watch pass builds baseline/readiness state. Existing findings in the
repository are not reported as new save-time violations until a later file
change introduces or re-surfaces them.

## Step 3 -- Try the MCP catch

With `anvil start` reporting `protecting` and your editor restarted, ask the AI
inside Cursor or Claude Code to make a change you know is wrong (e.g. "add an
`any` type to this function" or "swallow this error in a try/catch"). The MCP
tool `anvil_validate_write` is called before the write lands; the daemon refuses
the write and the AI sees the rejection.

For a guided walk-through, see the
[wow-start demo](/anvil/guides/wow-start-demo).

## Step 4 -- Run the Tutorial

For a guaranteed-value path (especially when your repo doesn't trip anything
straight away), run the protection-loop tutorial:

```bash
anvil tutorial
```

The default path (`ProtectionLoop`) is a five-step, value-first walk that ends
with `anvil start --verify` so you finish back in the activation surface.

```bash
anvil tutorial --reset     # Start fresh if you have run it before
```

For deeper dives into specific features, see the
[written tutorials](/anvil/tutorials) (policies, architecture, drift, CI).

## Step 5 -- Diagnostics

```bash
anvil doctor              # Environment, config, and hook checks
anvil status --verify     # Read-only activation probe (same backend as `anvil start --verify`)
anvil version             # Current and latest version + the upgrade command for your install method
```

`anvil version` is install-method aware -- it knows whether you used Homebrew,
Scoop, WinGet, the install script, or a dev build, and prints the upgrade
command for that path.

Need to reset after a beta test? Use `anvil uninstall --dry-run` to preview the
project cleanup, `anvil uninstall --yes` to remove project state, and
`anvil uninstall --global` when you also want user-level state and Anvil MCP
entries removed. The command does not remove the binary itself.

## Step 6 -- Watch Fallback

When MCP can't attach (no Cursor / Claude Code, or the editor refused to load
the server), watch mode is the save-time fallback:

```bash
anvil watch --source       # Watch source files
anvil start --watch        # Activate, then drop into the watch fallback
```

Save a file and Anvil reports findings after the save. This is **not** pre-write
protection -- the write already happened. Press `Ctrl+C` to stop.

Watch mode prints startup feedback immediately so a large repository does not
look hung while the initial scan warms up. If stdin or stdout is not a terminal,
Anvil automatically falls back to plain output instead of opening the TUI.

```bash
anvil watch --plans        # Watch planning documents only
anvil watch --all          # Watch both source and plans
```

Audit and watch use the same built-in local-noise ignore policy. Agent/tool
state and generated directories such as `.claude`, `.opencode`, `.gemini`,
`.serena`, `.worktrees`, `node_modules`, `target`, `dist`, and cache folders are
skipped by default so first-run scans do not spend time on local machinery.

## Sign In (optional)

```bash
anvil auth login           # Device-code flow
anvil auth login --otp     # Email OTP
```

Anvil's local protection works without sign-in; auth is for online-only features
such as update checks.

---

## What to Test

We are especially interested in feedback on these areas in the current beta:

| Area                                       | What to try                                                                                                 |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| **Wow-start activation (`anvil start`)**   | Does the first minute land? Does the printed protection state match what is actually wired?                 |
| **MCP catch via Cursor / Claude Code**     | After restart, does `anvil` show in the MCP list? Does an AI rewrite get refused before the write lands?    |
| **Activation states copy (no over-claim)** | If activation reports `needs_action` or `unsupported`, is the explanation specific and the next step real?  |
| **Language profile honesty**               | If your repo is mostly Python, does the activation summary name the gap instead of pretending?              |
| **Tutorial experience (`ProtectionLoop`)** | Is the protection-loop walk clear? Does it leave you in a useful state?                                     |
| **`anvil version`**                        | Does it correctly identify your install method and print the right upgrade command?                         |
| **Watch fallback**                         | When MCP can't attach, does `anvil watch --source` / `anvil start --watch` produce useful save-time signal? |

## Known Limitations

- **MCP install is Cursor and Claude Code only in v1.** Windsurf, VS Code MCP
  install, and Copilot / Codex CLI integration are explicitly out of scope.
- **`anvil intercept status` works on every supported target.** The Unix path
  speaks the UDS IPC; the Windows path drives the same wire shape over the named
  pipe and `--json` returns the same `DaemonStatusV1` on either OS. The
  remaining Windows gap is in the MCP correlation envelope only:
  `correlation.daemonStatus` returned by `anvil_validate_write` is always
  `not-wired` on Windows in this cut, tracked under `chore/windows-status`.
- **Daemon runs in foreground only.** Use `anvil intercept start --foreground`
  -- backgrounding is not a v1 surface. Operators running under systemd /
  launchd should run foreground under the manager's supervision.
- **Fences survive daemon restart.** The `anvil intercept stop` and
  `anvil intercept unblock` CLI subcommands are not wired in v1 (a follow-up
  INTD task tracks the front-end). Recovery is: stop the foreground daemon
  (Ctrl-C, or SIGTERM by PID), then
  `rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/anvil"` to clear fence state,
  then re-launch.
- **macOS interrupt ladder is fence-first.** Interrupt decisions on macOS fence
  the worktree rather than running the SIGINT/SIGTERM/SIGKILL ladder. Recover
  the same way as above: stop the daemon and remove the fence directory.
- **Windows CI runs only on `main` syncs.** A dev-branch build's CI green does
  not mean the Windows target was tested for that change. File Windows bugs with
  that caveat noted.
- **Gate checks** -- some gates (policy, OPA/Rego) require external tools.
- **First-run performance** -- the initial scan may be slower while caches are
  built.

**Tested on:** Linux (Ubuntu 22.04+), macOS 13+, Windows 11.

## Reporting Issues

Found a bug or have feedback?

- [Report a bug](https://github.com/eddacraft/anvil/issues/new?template=bug_report.md)
- [Request a feature](https://github.com/eddacraft/anvil/issues/new?template=feature_request.md)
- [Share general feedback](https://github.com/eddacraft/anvil/issues/new?template=feedback.md)

**When reporting, include:**

- The commands you ran and what happened
- Your environment (OS, terminal, `anvil --version` output)
- Steps to reproduce the issue

## Quick Reference

| Command                                  | Purpose                                                       |
| ---------------------------------------- | ------------------------------------------------------------- |
| `anvil start`                            | Activate protection: init + scan + MCP install                |
| `anvil start --verify`                   | Read-only activation probe (no writes)                        |
| `anvil start --watch`                    | Activate, then run the save-time watch fallback               |
| `anvil mcp install --client cursor`      | Install MCP entry for Cursor only                             |
| `anvil mcp install --client claude-code` | Install MCP entry for Claude Code only                        |
| `anvil status --verify`                  | Read-only activation probe (same backend as `start --verify`) |
| `anvil version`                          | Current and latest version, plus install-aware upgrade hint   |
| `anvil tutorial`                         | Interactive protection-loop walk-through                      |
| `anvil watch --source`                   | Save-time watch fallback                                      |
| `anvil check --all`                      | Scan entire codebase                                          |
| `anvil doctor`                           | Diagnostics and troubleshooting                               |
| `anvil policy explain <id>`              | Understand a policy rule                                      |
| `anvil gate`                             | Run quality gates                                             |
| `anvil --help`                           | See all commands                                              |

## Next Steps

Once you are comfortable with the basics:

- [Set up your first project](/anvil/first-project) -- architecture boundaries,
  suppressions, and CI
- [Understand gates](/anvil/concepts/gates) -- what Anvil validates and why
- [Configuration reference](/anvil/operations/config) -- customise checks,
  patterns, and watch behaviour
- [Custom policies](/anvil/tutorials/policies) -- write OPA/Rego rules for your
  team's standards

---

Thank you for testing Anvil. Your feedback shapes the product.
