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

**Current version:** 0.5.0

anvil is a native CLI that analyses your codebase for architectural drift,
AI-generated anti-patterns, and project convention violations. It is designed to
catch issues at save time, before they reach review or CI.

:::info Native binary

As of 0.3.0-beta, anvil is a native Rust binary. The deprecated Node.js package
(`@eddacraft/anvil-cli`) is no longer the recommended path. See
[The Switch to Rust](./releases/rust-rewrite.md) for details.

:::

## What We Need From You

Run anvil on a real TypeScript or JavaScript project and tell us where it helps,
where it gets in the way, and where the output is unclear.

The most useful feedback answers these questions:

- Did install, login, and project setup work without help?
- Did the first scan find anything useful?
- Were warnings accurate, actionable, and easy to triage?
- Did watch mode feel fast enough to leave running while coding?
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

```bash
brew upgrade eddacraft/tap/anvil
```

```powershell
winget upgrade eddacraft.anvil
scoop update anvil
```

For 0.5.0 upgrade notes, see [Upgrade Notes](./releases/upgrade-notes.md).

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

### 1. Try the Tutorial

```bash
anvil tutorial
```

Record whether the tutorial explains the product clearly and whether any step is
confusing, too slow, or broken in your terminal.

### 2. Initialise a Real Project

```bash
cd your-project
anvil init
```

The setup flow creates `.anvilrc`, creates `.anvil/`, and now runs a first
sample analysis so you see useful signal immediately.

Record:

- Whether project type, package manager, Git state, and TypeScript detection
  were correct.
- Whether the generated `.anvilrc` makes sense for your project.
- Whether the first scan found useful warnings or produced noise.

If you have already initialised the project and want to retest setup, run:

```bash
anvil init --force
```

### 3. Run the Main Scan

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

### 4. Leave Watch Mode Running

Start source-file watch mode:

```bash
anvil watch --source
```

Save a TypeScript or JavaScript file. Watch should print the active scope and
respond when files change.

Try the watch filters introduced in 0.4.0-beta and still active in 0.5.0:

```bash
anvil watch --patterns "src/**/*.ts,src/**/*.tsx"
anvil watch --exclude "dist/**,coverage/**"
```

Record:

- Whether the startup banner makes the active watch scope clear.
- Whether save-time feedback feels fast enough.
- Whether any files are missed or unexpectedly included.
- Whether `--exclude` glob behaviour is clear. Bare names such as `dist` only
  match that exact path; use `dist/**` to exclude contents.

Press `Ctrl+C` to stop watch mode.

### 5. Run Diagnostics and Status

```bash
anvil doctor
anvil status
```

Record whether remediation steps are specific enough when something is missing,
misconfigured, or skipped.

### 6. Try a Gate Run

```bash
anvil gate --profile dev
```

If your project has CI-like dependencies available locally, also try:

```bash
anvil gate --profile ci
```

Record whether gate failures clearly explain what failed and what to do next.

### 7. Try the 0.5.0 AI Guardrail and MCP Surfaces

These are the headline 0.5.0 surfaces and the most useful test focus this cycle.

```bash
# AI guardrail profile — strict config, JSON envelope by default
anvil gate --profile ai

# Generate (and verify) editor MCP configuration
anvil mcp-config --client claude-code --verify
anvil mcp-config --client cursor --write
```

Record:

- Whether the AI guardrail run produces actionable output, including when
  governance config is missing or invalid.
- Whether `anvil mcp-config` produces correct config for your editor, and
  whether `--verify` cleanly diffs against an existing setup.
- Whether `--write` path-safety prompts behave the way you'd expect when a
  config already exists.

Optionally try the new config-mode Git hooks on Git 2.54+:

```bash
anvil hooks install --config
anvil hooks status
anvil hooks uninstall --config
```

Record whether the install/uninstall flow leaves Husky and any third-party hook
manager untouched, and whether `anvil doctor` correctly flags coexistence or
`core.hooksPath` overrides.

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
anvil policy explain AP-003
```

Useful feedback includes whether policy names, severity, and explanations match
the issue you saw in the scan output.

### Integrations

Try the integrations that match your workflow:

- [GitHub Actions](./integrations/github.md)
- [VS Code](./integrations/vscode.md)
- [MCP / AI editor configuration](./integrations/mcp.md)

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

- **Primary language coverage is TypeScript and JavaScript.** Other language
  surfaces are expanding, but the beta is strongest on TS/JS projects.
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
