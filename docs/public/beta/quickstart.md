---
id: quickstart
title: Beta Quickstart
description: Get up and running with the Anvil beta in 10 minutes.
sidebar_position: 1
slug: /
---

# Beta Quickstart

Install Anvil, run the tutorial, scan your project, and start giving feedback --
all in about 10 minutes.

:::info Beta release

This is **pre-release software** (0.3.0-beta). The CLI is now a native Rust
binary — no Node.js required. APIs and behaviour may change between releases.
Your feedback directly shapes the product before public launch.

:::

## Prerequisites

- A TypeScript or JavaScript project to test with
- **macOS**, **Linux**, or **Windows** (x86_64 or aarch64)

## Install

:::info Sign up first

Don't have access yet? [Request an invite](https://eddacraft.ai/#waitlist) to
join the next beta cohort.

:::

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/EddaCraft/anvil/releases/latest/download/anvil-cli-installer.sh | sh

# Windows (PowerShell)
powershell -ExecutionPolicy ByPass -c "irm https://github.com/EddaCraft/anvil/releases/latest/download/anvil-cli-installer.ps1 | iex"
```

Verify the installation:

```bash
anvil --version
```

## Step 1 -- Run the Interactive Tutorial

The fastest way to learn Anvil. It takes about 5 minutes and walks you through
scanning, watching, and fixing issues in a sandbox project.

```bash
anvil tutorial
```

The tutorial covers:

1. **Scan** -- analyse code for issues
2. **Watch** -- monitor files in real-time
3. **Fix** -- address identified problems
4. **Next steps** -- where to go from here

```bash
anvil tutorial --reset     # Start fresh if you have run it before
```

For deeper dives into specific features, see the
[written tutorials](/anvil/tutorials) (policies, architecture, drift, CI).

## Step 2 -- Log In

Authenticate with your beta access token (provided with your invite):

```bash
anvil login
```

You will be prompted for your token. Once authenticated, all CLI commands are
available.

## Step 3 -- Initialise Your Project

Try Anvil on a real codebase:

```bash
cd your-project
anvil init
```

The setup wizard will:

- Detect your project type, package manager, and tooling
- Create an `.anvilrc` configuration file
- Set up the `.anvil/` directory
- Optionally install Git hooks

```
Initialising Anvil in current project...

Detected environment:
  Project: my-app
  Package Manager: pnpm
  Git: yes
  TypeScript: yes

Anvil initialised successfully!

Created files:
  .anvilrc
  .anvil/
```

## Step 4 -- Scan Your Codebase

Run a full scan to see what Anvil catches:

```bash
anvil check --all
```

Most projects have something. Here is typical output:

```
Checking architecture... done
Checking anti-patterns...
  [AP-003] Explicit any type detected
    src/utils/parser.ts:42
    Using 'any' defeats type safety
    Fix: Define a proper type or use 'unknown'

  [AP-006] Empty catch block
    src/services/auth.ts:87
    Empty catch blocks hide errors
    Fix: Log the error or re-throw

2 warnings found.
```

**Other scan options:**

```bash
anvil check --changed            # Only changed files (git-aware)
anvil check --changed --staged   # Only staged files
anvil check --verbose            # Detailed explanations
```

**Understand a policy:**

```bash
anvil policy explain <policy-id>       # Explain a specific policy rule
anvil policy list                      # List all available policies
```

## Step 5 -- Watch Mode

Start real-time validation as you code:

```bash
anvil watch --source
```

Save a file and see Anvil catch it immediately.

```bash
anvil watch --plans        # Watch planning documents only
anvil watch --all          # Watch both source and plans
```

Press `Ctrl+C` to stop.

:::tip

Run watch mode in a dedicated terminal pane or use the
[VS Code extension](/anvil/integrations/vscode) for in-editor diagnostics.

:::

## Step 6 -- Run Diagnostics

If something is not working, run the doctor:

```bash
anvil doctor
```

This checks Git configuration, Anvil configuration validity, and hook
installation status.

The doctor checks Git configuration, `.anvilrc` validity, and hook installation.

## More Commands Worth Testing

```bash
anvil status              # Current configuration and state
anvil gate                # Full codebase scan via quality gates
anvil gate myplan.md      # Validate a specific plan file
anvil gate --profile dev  # Development mode
anvil start               # Welcome screen with guided options
anvil --help              # See all commands
```

---

## What to Test

We are especially interested in feedback on these areas:

| Area                    | What to try                                           |
| ----------------------- | ----------------------------------------------------- |
| **Tutorial experience** | Is it clear? Does it work smoothly?                   |
| **Init wizard**         | Does it detect your project correctly?                |
| **Scan results**        | Are warnings accurate and actionable?                 |
| **Watch mode**          | Is it fast enough? Does it catch changes?             |
| **Error messages**      | Are they helpful when something goes wrong?           |
| **TUI (terminal UI)**   | Does the interactive interface work in your terminal? |

## Known Limitations

- **Gate checks** -- some gates (policy, OPA/Rego) require external tools
- **Adapters** -- SpecKit and BMAD adapters are complete; others are in progress
- **VS Code extension** -- basic functionality only; advanced features coming
- **First-run performance** -- initial scan may be slower while caches are built
- **Large monorepos** -- gate execution may be slower on very large codebases

**Tested on:** Linux (Ubuntu 22.04+), macOS 13+, Windows 11.

## Reporting Issues

Found a bug or have feedback?

- [Report a bug](https://github.com/EddaCraft/anvil-001/issues/new?template=bug_report.md)
- [Request a feature](https://github.com/EddaCraft/anvil-001/issues/new?template=feature_request.md)
- [Share general feedback](https://github.com/EddaCraft/anvil-001/issues/new?template=feedback.md)

**When reporting, include:**

- The commands you ran and what happened
- Your environment (OS, terminal, `anvil --version` output)
- Steps to reproduce the issue

## Quick Reference

| Command                | Purpose                         |
| ---------------------- | ------------------------------- |
| `anvil tutorial`       | Interactive guided tutorial     |
| `anvil init`           | Set up Anvil in a project       |
| `anvil check --all`    | Scan entire codebase            |
| `anvil watch --source` | Real-time validation            |
| `anvil doctor`         | Diagnostics and troubleshooting |
| `anvil policy explain`  | Understand a policy rule        |
| `anvil status`         | Check configuration and state   |
| `anvil gate`           | Run quality gates               |
| `anvil --help`         | See all commands                |

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
