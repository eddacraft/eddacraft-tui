# Beta Tester Quickstart

> Get up and running with Anvil in 10 minutes

Welcome, beta tester! This guide walks you through the key features we'd love
you to test. Your feedback helps shape Anvil before public release.

## Prerequisites

- **Node.js** 20.0.0 or later
- **pnpm**, **npm**, or **yarn**
- A TypeScript or JavaScript project to test with

## Installation

<!-- TODO: Finalise npm package availability -->

```bash
# Installation instructions coming soon
# Check with the team for the current install method
```

---

## 1. Run the Interactive Tutorial (Recommended First Step)

The best way to learn Anvil is the built-in interactive tutorial. It takes about
5 minutes and walks you through scanning, watching, and fixing issues.

```bash
anvil tutorial
```

This runs the **core tutorial** which covers:

1. **Scan** - Analyse code for issues
2. **Watch** - Monitor files in real-time
3. **Fix** - Address identified problems
4. **Next steps** - Guidance for further exploration

**Options:**

```bash
anvil tutorial --list      # See all available tutorials
anvil tutorial --reset     # Start fresh if you've run it before
```

**Advanced tutorials** (after completing the core tutorial):

```bash
anvil tutorial policies       # Write custom OPA/Rego rules
anvil tutorial architecture   # Define architecture boundaries
anvil tutorial drift          # Track architecture drift over time
anvil tutorial ci             # Set up CI integration
```

---

## 2. Initialise Anvil in Your Own Project

After the tutorial, try Anvil on a real project:

```bash
cd your-project
anvil init
```

The setup wizard will:

- Detect your project type, package manager, and tooling
- Create an `.anvilrc` configuration file
- Set up the `.anvil/` directory
- Optionally install Git hooks

**Options:**

```bash
anvil init --force           # Overwrite existing configuration
anvil init --non-interactive # Use defaults without prompts
```

---

## 3. Scan Your Codebase

Run a full scan to see what Anvil catches:

```bash
anvil check --all
```

This analyses your entire codebase for:

- Anti-pattern violations (AP-\*)
- Architecture boundary violations
- Common code quality issues

**Other scan options:**

```bash
anvil check --changed            # Check only changed files (git-aware)
anvil check --changed --staged   # Check only staged files
anvil check --verbose     # Show detailed explanations
```

**Want to understand a warning?**

```bash
anvil explain AP-003                    # Explain what AP-003 means
anvil explain AP-003-src/file.ts:42     # Explain with file context
anvil explain --list                    # List all explainable rules
```

---

## 4. Watch Mode

Start real-time validation as you code:

```bash
anvil watch --source
```

This monitors your source files and validates on every save.

**Watch options:**

```bash
anvil watch --plans        # Watch planning documents only
anvil watch --all          # Watch both source and plans
anvil watch --profile dev  # Development mode (skips coverage checks)
```

Press `Ctrl+C` to stop watching.

---

## 5. Run Diagnostics

If something isn't working, run the doctor:

```bash
anvil doctor
```

This checks:

- Node.js version
- Git configuration
- Anvil configuration validity
- Hook installation status

**Auto-fix common issues:**

```bash
anvil doctor --fix
```

---

## Other Commands Worth Testing

### Check Project Status

```bash
anvil status              # Show current Anvil configuration and state
```

### Quality Gates (Plan-Based Validation)

```bash
anvil gate                # Full codebase scan via gates
anvil gate myplan.md      # Validate a specific plan file
anvil gate --profile dev  # Development mode
```

### Getting Started Screen

```bash
anvil start               # Welcome screen with tutorial, init, doctor, and help options
```

---

## What to Test

We're especially interested in feedback on:

| Area                    | What to try                                           |
| ----------------------- | ----------------------------------------------------- |
| **Tutorial experience** | Is it clear? Does it work smoothly?                   |
| **Init wizard**         | Does it detect your project correctly?                |
| **Scan results**        | Are warnings accurate and actionable?                 |
| **Watch mode**          | Is it fast enough? Does it catch changes?             |
| **Error messages**      | Are they helpful when something goes wrong?           |
| **TUI (terminal UI)**   | Does the interactive interface work in your terminal? |

---

## Reporting Issues

Found a bug or have feedback? We want to hear it!

- **Bugs**:
  [Report a bug](https://github.com/EddaCraft/anvil-001/issues/new?template=bug_report.md)
- **Feature requests**:
  [Request a feature](https://github.com/EddaCraft/anvil-001/issues/new?template=feature_request.md)
- **General feedback**:
  [Share feedback](https://github.com/EddaCraft/anvil-001/issues/new?template=feedback.md)

**When reporting, include:**

- Commands you ran and what happened
- Your environment (OS, Node version, terminal)
- Steps to reproduce the issue

---

## Quick Reference

| Command                | Purpose                         |
| ---------------------- | ------------------------------- |
| `anvil tutorial`       | Interactive guided tutorial     |
| `anvil init`           | Set up Anvil in a project       |
| `anvil check --all`    | Scan entire codebase            |
| `anvil watch --source` | Real-time validation            |
| `anvil doctor`         | Diagnostics and troubleshooting |
| `anvil explain <rule>` | Understand a warning            |
| `anvil status`         | Check configuration and state   |
| `anvil gate`           | Run quality gates               |
| `anvil --help`         | See all commands                |

---

Thank you for testing Anvil! Your feedback directly shapes the product.
