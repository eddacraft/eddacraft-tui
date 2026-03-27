---
id: beta-testing-guide
title: Beta Testing Guide
description:
  Everything you need to get started testing Anvil during the closed beta,
  including setup, what to test, and how to report issues.
sidebar_position: 2
---

# Beta Testing Guide

Welcome to the Anvil closed beta. Thank you for helping us shape the tool — your
feedback directly influences what we build next.

**Current version:** 0.3.0-beta

Anvil is a developer tool that analyses your codebase for architectural drift,
anti-patterns, and convention violations, then helps you maintain consistency as
your project evolves.

:::info Native binary

As of 0.3.0-beta, Anvil is a native Rust binary. The Node.js package
(`@eddacraft/anvil-cli`) is deprecated. See
[The Switch to Rust](./releases/rust-rewrite.md) for details and migration
instructions.

:::

## Getting Started

### Install

**macOS / Linux:**

```bash
curl -fsSL https://install.eddacraft.ai | sh
```

**Windows:**

```powershell
irm https://install.eddacraft.ai/windows | iex
```

**Homebrew (macOS / Linux):**

```bash
brew install eddacraft/tap/anvil
```

No Node.js or npm required. The installer downloads a single static binary for
your platform.

### Authenticate

```bash
anvil login
```

### Set up a project

```bash
cd your-project
anvil init
```

The init wizard will detect your project type and suggest a default
configuration.

For a full walkthrough, see the [Quickstart](./quickstart.md).

## How to Upgrade

```bash
curl -fsSL https://install.eddacraft.ai | sh
```

Or via Homebrew:

```bash
brew upgrade eddacraft/tap/anvil
```

Verify your version:

```bash
anvil --version
```

We recommend upgrading before each testing session to ensure you have the latest
fixes and features.

## What to Test

The following areas are organised by feature rather than release. For each area,
we have listed the key commands to try and the kind of feedback that is most
useful.

### Core Scanning

Anvil's primary function: analysing your codebase for issues.

**Commands to try:**

```bash
anvil check              # Run a full scan
anvil watch              # Watch mode — re-scans on file changes
```

**What we are looking for:**

- False positives (findings that are not real issues)
- Missed patterns (real issues that Anvil should have caught)
- Performance on large projects (scan time, memory usage)
- Accuracy of architecture detection and anti-pattern identification

### Project Setup

The initial configuration experience.

**Commands to try:**

```bash
anvil init               # Interactive setup wizard
```

**What we are looking for:**

- Edge cases with different project structures (nested packages, non-standard
  layouts)
- Package manager detection issues (npm, pnpm, yarn, bun)
- Monorepo handling
- Whether the generated `.anvilrc` makes sense for your project

### Interactive Tutorial

A guided introduction to Anvil's features.

**Commands to try:**

```bash
anvil tutorial
```

**What we are looking for:**

- Clarity of explanations
- Pacing (too fast, too slow, about right)
- Anything confusing or unclear
- Steps that do not work as described

### Project Memory

_New in 0.2.x._ Anvil can learn patterns from your codebase over time using the
Edda memory system and Ember proposal engine.

**Commands to try:**

```bash
anvil edda               # View and manage project memory
anvil ember              # View pattern proposals and candidates
anvil stack              # Inspect the full Edda/Ember stack state
```

**What we are looking for:**

- Usefulness of detected patterns
- Accuracy of pattern recognition
- Command UX and output readability
- Whether the memory system surfaces genuinely helpful insights

### Architecture Tools

Tools for understanding and enforcing your project's architecture.

**Commands to try:**

```bash
anvil drift              # Detect architectural drift
anvil architecture       # View detected architecture boundaries
```

**What we are looking for:**

- Boundary detection accuracy
- Drift detection usefulness
- Whether the output helps you understand your project's structure

### CI and Integrations

Anvil integrates with your existing development workflow.

**Available integrations:**

- GitHub Action
- VS Code extension

**What we are looking for:**

- Setup friction (was it easy to get running?)
- Reliability (does it work consistently?)
- Whether the output is useful in CI contexts

## Reporting Issues

Please report issues on GitHub:

**[github.com/EddaCraft/anvil-001/issues](https://github.com/EddaCraft/anvil-001/issues)**

### What to include

- Anvil version (`anvil --version`)
- Operating system, version, and architecture (`uname -a` or equivalent)
- Steps to reproduce the issue
- Expected behaviour vs actual behaviour
- Any relevant output or error messages

### Suggested labels

| Label           | Use for                                     |
| --------------- | ------------------------------------------- |
| `beta-feedback` | General feedback or observations            |
| `bug`           | Something is broken or behaves incorrectly  |
| `enhancement`   | Feature requests or improvement suggestions |

## Known Limitations

- **TypeScript and JavaScript projects only** — support for other languages is
  planned but not yet available.
- **False positives on unconventional structures** — some anti-pattern detectors
  may flag valid code in projects with non-standard layouts.
- **Memory system is new** — pattern detection accuracy improves over time as
  Anvil observes more of your project. Early results may be noisy.

## FAQ

**Do I need to be online?** Authentication requires an internet connection.
After initial setup, scanning works entirely offline.

**Can I use this on private or proprietary code?** Yes. Anvil runs locally on
your machine. No source code is sent to external services.

**Do I need Node.js?** The Anvil binary itself has no runtime dependencies.
However, some gate checks (lint, test) shell out to your project's package
manager, so Node.js and pnpm/npm are still needed if those checks are enabled.
If you previously used the npm package, see
[The Switch to Rust](./releases/rust-rewrite.md) for migration steps.

**How do I reset my project configuration?** Run `anvil init --force` to
regenerate your configuration from scratch.

**Where is my data stored?** In the `.anvil/` directory in your project root.
This directory contains your configuration, baselines, suppressions, and memory
data.

**How often should I upgrade?** We recommend upgrading before each testing
session. Beta releases are frequent and often include fixes for issues reported
by testers.
