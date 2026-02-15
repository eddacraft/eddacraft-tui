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

This is **pre-release software** (0.1.0-beta). APIs and behaviour may change
between releases. Your feedback directly shapes the product before public
launch.

:::

## Prerequisites

- **Node.js** 20.0.0 or later
- **pnpm**, **npm**, or **yarn**
- A TypeScript or JavaScript project to test with

## Install

```bash
# Using pnpm (recommended)
pnpm add -D @eddacraft/anvil-cli

# Using npm
npm install -D @eddacraft/anvil-cli
```

Verify the installation:

```bash
npx anvil --version
```

## Step 1 -- Run the Interactive Tutorial

The fastest way to learn Anvil. It takes about 5 minutes and walks you through
scanning, watching, and fixing issues in a sandbox project.

```bash
npx anvil tutorial
```

The tutorial covers:

1. **Scan** -- analyse code for issues
2. **Watch** -- monitor files in real-time
3. **Fix** -- address identified problems
4. **Next steps** -- where to go from here

```bash
npx anvil tutorial --list      # See all available tutorials
npx anvil tutorial --reset     # Start fresh if you have run it before
```

**Advanced tutorials** (after the core tutorial):

```bash
npx anvil tutorial policies       # Write custom OPA/Rego rules
npx anvil tutorial architecture   # Define architecture boundaries
npx anvil tutorial drift          # Track architecture drift over time
npx anvil tutorial ci             # Set up CI integration
```

## Step 2 -- Log In

Authenticate with your beta access token (provided with your invite):

```bash
npx anvil login
```

You will be prompted for your token. Once authenticated, all CLI commands are
available.

## Step 3 -- Initialise Your Project

Try Anvil on a real codebase:

```bash
cd your-project
npx anvil init
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
npx anvil check --all
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
npx anvil check --changed            # Only changed files (git-aware)
npx anvil check --changed --staged   # Only staged files
npx anvil check --verbose            # Detailed explanations
```

**Understand a warning:**

```bash
npx anvil explain AP-003                    # What does AP-003 mean?
npx anvil explain AP-003-src/file.ts:42     # Explain with file context
npx anvil explain --rules                   # List all explainable rules
```

## Step 5 -- Watch Mode

Start real-time validation as you code:

```bash
npx anvil watch --source
```

Save a file and see Anvil catch it immediately.

```bash
npx anvil watch --plans        # Watch planning documents only
npx anvil watch --all          # Watch both source and plans
npx anvil watch --profile dev  # Development mode (skips coverage checks)
```

Press `Ctrl+C` to stop.

:::tip

Run watch mode in a dedicated terminal pane or use the
[VS Code extension](/anvil/integrations/vscode) for in-editor diagnostics.

:::

## Step 6 -- Run Diagnostics

If something is not working, run the doctor:

```bash
npx anvil doctor
```

This checks Node.js version, Git configuration, Anvil configuration validity,
and hook installation status.

```bash
npx anvil doctor --fix    # Auto-fix common issues
```

## More Commands Worth Testing

```bash
npx anvil status              # Current configuration and state
npx anvil gate                # Full codebase scan via quality gates
npx anvil gate myplan.md      # Validate a specific plan file
npx anvil gate --profile dev  # Development mode
npx anvil start               # Welcome screen with guided options
npx anvil --help              # See all commands
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
- Your environment (OS, Node version, terminal)
- Steps to reproduce the issue

## Quick Reference

| Command                    | Purpose                         |
| -------------------------- | ------------------------------- |
| `npx anvil tutorial`       | Interactive guided tutorial     |
| `npx anvil init`           | Set up Anvil in a project       |
| `npx anvil check --all`    | Scan entire codebase            |
| `npx anvil watch --source` | Real-time validation            |
| `npx anvil doctor`         | Diagnostics and troubleshooting |
| `npx anvil explain <rule>` | Understand a warning            |
| `npx anvil status`         | Check configuration and state   |
| `npx anvil gate`           | Run quality gates               |
| `npx anvil --help`         | See all commands                |

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
