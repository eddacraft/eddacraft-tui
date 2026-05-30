---
id: beta-testing-guide
title: Beta Testing Guide
description:
  Everything you need to get started testing anvil during early access,
  including setup, what to test, and how to report useful feedback.
sidebar_position: 6
---

# Beta Testing Guide

Welcome to the anvil beta. Thank you for putting real projects through the tool:
the best feedback comes from normal development work, not from perfect demo
repos.

**Current version:** 0.7.2-beta

anvil is a single native binary that analyses your codebase for architectural
drift, AI-generated anti-patterns, and project convention violations. It is
designed to catch issues before the AI's write lands -- in front of an MCP
client like Cursor or Claude Code -- and to fall back to save-time signal when
pre-write attachment is not available.

## What's New to Focus Testing On

The current tagged cut is `0.7.2-beta`; a `0.7.3-beta` candidate ("Surfacing the
Signal") is in flight on dev builds. These are the highest-leverage flows. If
you only have a short session, these are the right places to spend it.

- **`anvil watch` now runs code-quality checks by default (`0.7.2-beta`).** A
  bare `anvil watch` previously watched architecture and dependency edges only
  and ran no code-quality scan, while the dashboard still read "100% pass". It
  now runs `anvil check --all` on each save. Confirm a save with a real
  antipattern actually surfaces a finding. Run `anvil watch --action none` to
  restore the architecture/dependency-only watch.
- **Wow-start activation (`anvil start`).** `install → cd repo → anvil start` is
  the canonical first minute. The command runs init + first-scan + MCP install
  in one go and ends with a literal protection state (`protecting`,
  `ready_restart_required`, `watching`, `needs_action`, `unsupported`, or
  `error`). The state is the contract -- trust it.
- **Daemon-backed MCP validation in Cursor and Claude Code.** The
  `anvil_validate_write` MCP tool is backed by the local daemon over owner-only
  IPC on Unix and Windows, with the embedded path as a correctness-equivalent
  fallback. Restart the editor after `anvil start` and verify `anvil` shows in
  the MCP list, then ask the AI to make a wrong rewrite and watch the daemon
  refuse it.
- **Repo language profile honesty.** Activation now names detected languages and
  their coverage tier (TS supported, SQL and Markdown partial, Python and Rust
  unsupported). Language-specific antipattern checks honour the profile;
  cross-language checks (e.g. secrets) still run on every file.
- **Foreground daemon ops.** The daemon runs in foreground only in v1
  (`anvil intercept start --foreground`). Fences survive restart;
  `anvil intercept unblock --worktree <PATH>` clears a fenced worktree on Unix,
  while Windows users should stop and restart the daemon if every surface is
  quarantined. `anvil intercept status` is available on every supported target.
- **`anvil version` install-method awareness.** Reports current and latest
  version and prints the upgrade command for your install method (Homebrew,
  Scoop, WinGet, installer, dev build).

### Coming in `0.7.3-beta` — Surfacing the Signal

These features are landing on dev builds ahead of the `0.7.3-beta` tag. If you
run a dev build, they are the freshest surfaces to exercise; if you are on the
`0.7.2-beta` tag, they are a preview of what is next.

- **`anvil dashboard` — read-only TUI dashboards.** Live views over persisted
  `.anvil/` state: **Architecture Health**, **Drift Snapshots**, and
  **Suppressions**. Run `anvil dashboard` for a picker or
  `anvil dashboard <architecture|drift|suppressions>` to open one directly. See
  the [dashboards guide](guides/dashboard.md).
- **`anvil insights` — surface accumulated signal.** A weekly activity summary
  by default; `--suppressions` for a suppression health view that lists stale
  suppressions first; `--drift` for an 8-week new-edge sparkline. All support
  `--json`. See the [insights guide](guides/insights.md).
- **SARIF 2.1.0 export.** `anvil check`, `anvil gate`, and `anvil audit` accept
  `--format sarif` and emit the GitHub Code Scanning subset, so findings upload
  to Code Scanning without a per-command adapter. Emission is exit-code-neutral.
  See the [GitHub integration guide](integrations/github.md).
- **Working-tree secret scanning.** The secret scan now inspects on-disk
  content, not just git history, and reports the count of oversize lines it
  skipped — a "0 findings" result can no longer silently hide unscanned content.
- **Clean `--json` output.** CLI diagnostics (warnings, errors, `ANVIL_LOG` /
  `RUST_LOG` output) now route to stderr, leaving stdout reserved for command
  output, so `anvil … --json` is safe to pipe into `jq`. `anvil --json watch` is
  now a stable NDJSON stream (`anvil.watch.event.v1`).

## What We Need From You

Run anvil on a real project and tell us where it helps, where it gets in the
way, and where the output is unclear.

The most useful feedback answers these questions:

- Did `anvil start` honestly describe what protection is live in your repo? When
  the printed state was `protecting`, was it actually catching writes?
- Did Cursor or Claude Code show `anvil` in the MCP list after restart? Did an
  AI rewrite get refused before the write landed, or did it slip through?
- Did the language profile in the activation summary match your repo? If your
  repo is mostly Python or Rust, did the summary name the gap instead of
  pretending coverage?
- Did install, sign-in (if you used it), and project setup work without help?
- Were warnings accurate, actionable, and easy to triage?
- When MCP couldn't attach, did the watch fallback (`anvil start --watch` /
  `anvil watch --source`) produce useful save-time signal?
- Did `anvil version` correctly identify your install method and print the right
  upgrade command?
- Did any command fail, hang, produce noisy output, or ask for unclear input?
- What would stop you from using this on every save?

Use a project you know well. A small production app, internal tool, or active
side project is better than a toy repo because you can judge whether findings
are real.

## Before You Start

You will need:

- A macOS, Linux, or Windows machine.
- A TypeScript or JavaScript project you are comfortable testing against.
- Git installed, ideally with a clean working tree or a disposable branch.
- Beta access tied to the email or GitHub account we invited.
- Node.js and your project package manager if you want gate checks to run your
  existing lint or test commands. anvil itself does not require Node.js.

:::tip Use a disposable branch

`anvil init` creates `.anvilrc` and `.anvil/` in your project. Run it on a
branch you can discard if you only want to test the setup flow.

:::

## Install or Upgrade

If anvil is not installed yet, use one of these install methods.

**macOS / Linux installer:**

```bash
curl -fsSL https://install.eddacraft.ai | sh
```

**Windows PowerShell installer:**

```powershell
irm https://install.eddacraft.ai/windows | iex
```

**Homebrew (macOS / Linux):**

```bash
brew install eddacraft/tap/anvil
```

**WinGet (Windows):**

```powershell
winget install eddacraft.anvil
```

**Scoop (Windows):**

```powershell
scoop bucket add eddacraft https://github.com/eddacraft/scoop-bucket
scoop install anvil
```

If you already have anvil installed, upgrade before each beta test session.

```bash
anvil update
anvil --version
```

If you installed with Homebrew, WinGet, or Scoop, use that package manager
instead when the built-in updater tells you to.

The macOS/Linux curl installer also detects an existing Homebrew Anvil binary.
In that case it exits successfully without replacing the Homebrew-managed binary
and prints the Homebrew command instead.

```bash
brew upgrade eddacraft/tap/anvil
```

```powershell
winget upgrade eddacraft.anvil
scoop update anvil
```

For per-release upgrade notes, see [Upgrade Notes](./releases/upgrade-notes.md).

## Sign In

Start the default device-code login flow:

```bash
anvil auth login
```

anvil prints a short code and a verification URL. Open the URL, enter the code,
and return to the terminal.

If you need email OTP instead, run:

```bash
anvil auth login --otp
```

Check the stored identity:

```bash
anvil auth whoami
```

## Run a 30-Minute Test Pass

This is the recommended beta path if you only have one short session.

### 1. Activate Protection (`anvil start`)

```bash
cd your-project
anvil start
```

This runs init + first-scan + MCP install for Cursor and Claude Code in one go.
The activation summary ends with a literal protection state. Record:

- Which state was reported (`protecting`, `ready_restart_required`, `watching`,
  `needs_action`, `unsupported`, or `error`).
- Whether the language profile in the summary matched your repo.
- Whether the explanation of what is and isn't protected matched what you
  expected.
- Whether the MCP entries (`~/.cursor/mcp.json`, `~/.claude.json`) were written
  and whether your editor picked them up after restart.

If you only want a read-only check (no init, no scan, no MCP write):

```bash
anvil start --verify
```

### 2. Try the MCP Catch in Your Editor

With `anvil start` reporting `protecting` and your editor restarted, ask the AI
to make a change you know is wrong (e.g. add an `any` type, swallow an error,
introduce a hardcoded secret). Record:

- Whether `anvil` shows in the MCP list after the restart.
- Whether the AI's rewrite was refused before the write landed.
- Whether the rejection message was specific enough for the AI to recover.
- If a write slipped through, what kind of finding was missed.

### 3. Try the Tutorial

```bash
anvil tutorial
```

The default path (`ProtectionLoop`) is the protection-loop walk-through. Record
whether it explains the product clearly and whether any step is confusing, too
slow, or broken in your terminal.

### 4. Re-run Init in Isolation (optional)

If you want to retest the setup flow on its own:

```bash
anvil init --force
```

Record:

- Whether project type, package manager, Git state, and TypeScript detection
  were correct.
- Whether the generated `.anvilrc` makes sense for your project.

### 5. Run the Main Scan

```bash
anvil check --all
```

Then try the changed-file path:

```bash
anvil check --changed
anvil check --changed --staged
```

Record:

- Warnings that are definitely real.
- Warnings that are false positives.
- Real problems you expected anvil to catch but it missed.
- Output that is too vague to act on.

### 6. Try the Watch Fallback

When MCP can't attach, watch mode is the save-time fallback. It is **not**
pre-write protection -- the write already happened by the time watch reports the
finding.

```bash
anvil watch --source
```

Save a file. Watch should print the active scope and respond when files change.
On large repos it should print startup feedback immediately while warm-up runs,
not sit at a blank terminal. When stdin or stdout is not a terminal, it should
fall back to plain output instead of opening the TUI.

The initial watch scan is baseline/readiness state: existing repo contents
should not appear as new save-time violations until a later file change
introduces or re-surfaces the issue.

Audit and watch share a local-noise ignore policy. Tool state, local agent
worktrees, generated directories, and caches such as `.claude`, `.opencode`,
`.gemini`, `.serena`, `.worktrees`, `node_modules`, `target`, and `dist` should
be skipped by default.

Try the watch filters:

```bash
anvil watch --patterns "src/**/*.ts,src/**/*.tsx"
anvil watch --exclude "dist/**,coverage/**"
```

Record:

- Whether the startup banner makes the active watch scope clear.
- Whether save-time feedback feels fast enough.
- Whether any files are missed or unexpectedly included.
- Whether local tool/agent/cache directories were skipped as expected.
- Whether `--exclude` glob behaviour is clear. Bare names such as `dist` only
  match that exact path; use `dist/**` to exclude contents.

Press `Ctrl+C` to stop watch mode.

### 7. Run Diagnostics, Status, and Version

```bash
anvil doctor
anvil status --verify
anvil version
```

Record:

- Whether remediation steps are specific enough when something is missing,
  misconfigured, or skipped.
- Whether `anvil status --verify` matches what `anvil start --verify` reported.
- Whether `anvil version` correctly identifies your install method (Homebrew /
  Scoop / WinGet / installer / dev) and prints the right upgrade command.

### 8. Try a Gate Run

```bash
anvil gate --profile dev
```

If your project has CI-like dependencies available locally, also try:

```bash
anvil gate --profile ci
```

Record whether gate failures clearly explain what failed and what to do next.

### 9. Try the AI Guardrail Profile

```bash
anvil gate --profile ai
```

Record whether the AI guardrail run produces actionable output, including when
governance config is missing or invalid.

Optionally try the config-mode Git hooks on Git 2.54+:

```bash
anvil hooks install --config
anvil hooks status
anvil hooks uninstall --config
```

Record whether the install/uninstall flow leaves Husky and any third-party hook
manager untouched, and whether `anvil doctor` correctly flags coexistence or
`core.hooksPath` overrides.

### 10. Reset Cleanly After Testing

If you want to remove Anvil from the test repo or reset a stuck install, use the
uninstall command rather than manually deleting files:

```bash
anvil uninstall --dry-run   # Preview project cleanup
anvil uninstall --yes       # Remove project state and Anvil-managed hooks
```

Use `--global` only when you also want to remove user-level Anvil state,
credentials, MCP entries, and the running daemon:

```bash
anvil uninstall --global --dry-run
anvil uninstall --global --yes
```

`anvil uninstall` does not remove the Anvil binary. Use Homebrew, WinGet, Scoop,
or the installer path cleanup for that after the Anvil state is removed.

## Optional Deeper Areas

If you have more time, test the areas below.

### Architecture Boundaries

If your project has clear layers, create `.anvil/architecture.yaml` and validate
it:

```bash
anvil architecture validate
anvil architecture show
anvil check --all
```

Useful feedback includes whether layer names, glob patterns, and violation
output map to how you think about the project.

For a walkthrough, see [First Project](./first-project.md).

### Drift Snapshots

Capture and compare dependency snapshots:

```bash
anvil drift snapshot --name before-test
anvil drift list
anvil drift report
```

After making a small change or finishing a test task, capture a second snapshot
and compare them:

```bash
anvil drift snapshot --name after-test
anvil drift compare before-test after-test
```

Useful feedback includes whether the report helps you understand how
architecture changes over time.

### Policies

Explore available policies:

```bash
anvil policy list
anvil policy explain ARCH-001
```

Useful feedback includes whether policy names, severity, and explanations match
the issue you saw in the scan output. AP-\* anti-pattern explanations are not a
policy surface in this release; use the scan output and rule catalogue details
until the Rust explain command lands.

### Integrations

Try the integrations that match your workflow:

- [GitHub Actions](./integrations/github.md)
- [MCP / AI editor configuration](./integrations/mcp.md) -- Cursor and Claude
  Code only in v1

Useful feedback includes setup friction, unclear permissions, and whether the
same findings appear consistently across CLI, editor, and CI surfaces.

## Reporting Feedback

Report issues on GitHub:

**[github.com/eddacraft/anvil/issues](https://github.com/eddacraft/anvil/issues)**

Use the `beta-feedback` label for general observations, `bug` for broken
behaviour, and `enhancement` for improvement suggestions.

### Include This Information

- anvil version from `anvil --version`.
- Operating system, terminal, and architecture.
- Installation method: installer, Homebrew, WinGet, Scoop, or manual.
- Command you ran.
- Expected behaviour.
- Actual behaviour.
- Full error output or the smallest useful excerpt.
- Whether you were online, offline, behind VPN, or behind a corporate proxy.
- Whether the project is a monorepo, package workspace, or single-package repo.

Helpful environment commands:

```bash
anvil --version
anvil doctor
```

```bash
# macOS / Linux
uname -a
```

```powershell
# Windows PowerShell
[System.Environment]::OSVersion
$env:PROCESSOR_ARCHITECTURE
```

### Feedback Template

```markdown
## Summary

One sentence describing what happened.

## Environment

- anvil version:
- OS / terminal:
- Install method:
- Project type:

## Steps

1. Ran `...`
2. Expected `...`
3. Saw `...`

## Notes

- Was the output actionable?
- Did this block you or just feel confusing?
- Can you share a reduced repro or screenshot?
```

## Known Limitations

- **MCP install is Cursor and Claude Code only in v1.** Windsurf, VS Code MCP
  install, and Copilot / Codex CLI integration are explicitly out of scope. No
  process auto-attach.
- **`anvil intercept status` is available on every supported target.** The Unix
  path speaks the UDS IPC; the Windows path drives the same wire shape over the
  named pipe via `connect_owner_only_pipe_client`. `--json` returns the same
  `DaemonStatusV1` on either OS.
- **Windows MCP daemon-status correlation requires `v0.7.1-beta` or newer.** In
  `v0.7.0-beta`, Windows MCP responses reported `daemonStatus: not-wired`; the
  `v0.7.1-beta` patch adds owner-only named-pipe parity so
  `anvil_validate_write` can return `available`, `unavailable`, and
  `protection_claim` on Windows too.
- **Daemon runs in foreground only.** Use `anvil intercept start --foreground`
  -- backgrounded launches are not a v1 surface.
- **Fences survive daemon restart.** On Unix, use
  `anvil intercept unblock --worktree <PATH>` for worktree-scoped recovery. On
  Windows, worktree-scoped unblock is not supported yet; stop and restart the
  foreground daemon if every surface is quarantined. If daemon state is
  corrupted, remove `${XDG_DATA_HOME:-$HOME/.local/share}/anvil` before
  restarting; that clears all fence state for the user.
- **macOS interrupt ladder is fence-first.** Interrupt decisions on macOS fence
  the worktree rather than running the SIGINT/SIGTERM/SIGKILL ladder. Recovery
  is the same Unix `anvil intercept unblock --worktree <PATH>` path as above;
  reserve fence-directory removal for full reset or corrupt daemon state.
- **Windows CI runs only on `main` syncs.** A dev-branch build's CI green does
  not mean the Windows target was tested for that change.
- **Primary language coverage is TypeScript and JavaScript.** SQL and Markdown
  are partial; Python and Rust are unsupported in v1. The activation summary
  names the gap.
- **Gate checks may call your existing tools.** If lint, test, OPA, or other
  project tools are missing locally, `anvil gate` may skip or fail those checks.
- **Architecture checks need an architecture definition.** Use
  `.anvil/architecture.yaml` when you want boundary enforcement.
- **Some legacy or unconventional projects may be noisy.** False-positive
  reports are especially useful when you can explain why the code is valid.
- **Windows ARM is available but less exercised.** Please report install and
  PATH issues if you test on Windows ARM hardware.

## FAQ

**Do I need to be online?** Authentication and update checks need an internet
connection. Local scans continue to work after setup.

**Is source code sent to EddaCraft?** No. anvil runs analysis locally. Issue
reports should only include code snippets you are allowed to share.

**Do I need Node.js?** Not for anvil itself. Your project may still need Node.js
and a package manager when gate checks run your existing lint or test commands.

**How do I reset project configuration?** Run `anvil init --force` to regenerate
configuration from scratch.

**Where is project data stored?** anvil stores project configuration, snapshots,
cache data, and suppressions under `.anvil/` in your project root.

**How often should I upgrade?** Upgrade before each beta test session. Beta
releases are frequent and often include fixes from tester reports.

---

**Next:** [Set up your first project →](/anvil/first-project) | **See also:**
[Quickstart](/anvil/quickstart), [Changelog](/anvil/releases/changelog)
