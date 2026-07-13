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

This is **pre-release software** (`v0.9.0-beta`). The CLI is a self-contained
Rust binary. APIs and behaviour may change between releases. Your feedback
directly shapes the product before public launch.

:::

## Prerequisites

- A real repository you know well (TypeScript, JavaScript, Python, or Rust is
  supported; SQL and Markdown get partial coverage)
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

## Step 2 -- Discover with `anvil welcome` (optional, no login)

Before authenticating, you can see what anvil finds in your own repo:

```bash
cd your-project
anvil welcome
```

On first run it lands on a real local finding (or an honest clean result), shows
a one-line fix preview, and requires explicit consent before applying anything.
When you are ready for ongoing protection, authenticate and run `anvil start`.

## Step 3 -- Authenticate and `anvil start`

Durable surfaces (`start`, `watch`, `check`, `gate`, …) require beta
authentication. Sign in, then activate:

```bash
anvil auth login
anvil start
```

`anvil start` is the activation entrypoint. It runs `anvil init` if needed,
baselines existing findings, detects your repo's language profile, and wires MCP
entries so Cursor or Claude Code can call `anvil_validate_write` before each AI
write. On Linux and macOS an interactive terminal also auto-starts the per-user
save-time daemon (pass `--no-daemon` to suppress). On a healthy,
already-activated repo a repeat `anvil start` collapses to a short confidence
check.

The activation summary ends with a literal protection state -- one of
`protecting`, `ready_restart_required`, `watching`, `needs_action`,
`unsupported`, or `error`. Trust that literal: if `anvil start` reports
`needs_action`, pre-write protection is **not** live yet.

`anvil start` writes (only for clients you consent to install):

- `~/.cursor/mcp.json` (Cursor)
- `~/.claude.json` (Claude Code)

Install pickers start **unticked** — nothing selected means nothing written.
Restart the editor after activation. When the daemon is reachable, MCP
validation is daemon-backed on every supported OS (named pipes on Windows as of
`v0.7.1-beta`); the embedded path is the correctness-equivalent fallback.

**Useful flags:**

```bash
anvil start --verify   # Read-only probe; no init, no scan, no MCP write
anvil start --watch    # After activation, fall back to save-time watch mode
anvil start --tui      # Opt-in activation TUI (consent-first)
```

`--watch` is a save-time **fallback** -- it reports findings on save. It is not
pre-write interception.

The first watch pass builds baseline/readiness state. Existing findings in the
repository are not reported as new save-time violations until a later file
change introduces or re-surfaces them.

## Step 4 -- Try the MCP catch

With `anvil start` reporting `protecting` and your editor restarted, ask the AI
inside Cursor or Claude Code to make a change you know is wrong (e.g. "add an
`any` type to this function" or "swallow this error in a try/catch"). The MCP
tool `anvil_validate_write` is called before the write lands; the daemon refuses
the write and the AI sees the rejection.

For a guided walk-through, see the
[wow-start demo](/anvil/guides/wow-start-demo).

## Step 5 -- Run the Tutorial

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

## Step 6 -- Diagnostics

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

## Step 7 -- Watch Fallback

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

## Sign In

```bash
anvil auth login           # Device-code flow (default)
anvil auth login --otp     # Email OTP
```

`anvil welcome` is ungated for discovery. Daily protection (`anvil start` and
siblings) requires beta authentication. Unauthenticated action commands exit
`3`.

---

## What to Test

We are especially interested in feedback on these areas in `v0.9.0-beta`:

| Area                                       | What to try                                                                                                 |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| **`anvil welcome` first win**              | Real local finding (or honest clean), consent-gated apply, no write on decline                              |
| **Wow-start activation (`anvil start`)**   | Does the first minute land? Does the printed protection state match what is actually wired?                 |
| **Quiet repeat start**                     | Second `anvil start` on a healthy repo collapses rather than replaying onboarding                           |
| **MCP catch via Cursor / Claude Code**     | After restart, does `anvil` show in the MCP list? Does an AI rewrite get refused before the write lands?    |
| **Assistant graph context**                | Do graph tools / `graph://` resources answer identity-only without unintended source snippets?              |
| **Python + infrastructure hygiene**        | Are Python files analysed? Do Dockerfile / GHA / shell / SQL checks fire usefully?                          |
| **Activation states copy (no over-claim)** | If activation reports `needs_action` or `unsupported`, is the explanation specific and the next step real?  |
| **Tutorial experience (`ProtectionLoop`)** | Is the protection-loop walk clear? Does it leave you in a useful state?                                     |
| **`anvil version`**                        | Does it correctly identify your install method and print the right upgrade command?                         |
| **Watch fallback**                         | When MCP can't attach, does `anvil watch --source` / `anvil start --watch` produce useful save-time signal? |

## Known Limitations

- **MCP install is Cursor and Claude Code only.** Windsurf, VS Code MCP install,
  and Copilot / Codex CLI integration are explicitly out of scope for automated
  install; hand-write config if needed (see the MCP guide).
- **Daemon auto-start is Linux and macOS.** Interactive `anvil start` /
  `anvil watch` manage the per-user daemon on Unix; Windows still uses
  foreground daemon launch for operator/debug, while MCP can reach the daemon
  over named pipes when it is running (`v0.7.1-beta`+).
- **macOS interrupt ladder is fence-first.** Interrupt decisions on macOS fence
  the worktree rather than running the SIGINT/SIGTERM/SIGKILL ladder. Recover by
  stopping the daemon and clearing fence state (see the security / intercept
  docs).
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
