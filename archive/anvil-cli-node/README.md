# Anvil (Legacy Node.js Package)

> **Deprecated:** This Node.js package (`@eddacraft/anvil-cli`) is deprecated as
> of 0.3.0-beta. Install the native Rust binary instead — see below.

Deterministic development automation that makes AI-generated code changes safe
for production. Validate plans through quality gates before execution, ensuring
changes meet your team's standards.

## Install (Recommended)

Use the native binary for all platforms:

```bash
# macOS / Linux
curl -fsSL https://install.eddacraft.ai | sh

# Windows (PowerShell)
irm https://install.eddacraft.ai/windows | iex

# Homebrew (macOS / Linux)
brew install eddacraft/tap/anvil
```

See [The Switch to Rust](../../docs/public/anvil/releases/rust-rewrite.md) for
migration details.

## Quick Start (Legacy)

```bash
# Try without installing (legacy npm package)
npx @eddacraft/anvil-cli login
npx @eddacraft/anvil-cli tutorial

# Or install globally (legacy)
npm install -g @eddacraft/anvil-cli
anvil login
anvil tutorial
```

For the current beta, authenticate first with your beta token. The interactive
tutorial takes about 5 minutes and walks you through scanning, watching, and
fixing issues.

## What Anvil Does

- **Quality gates** - validate code changes against architecture rules,
  anti-patterns, and team conventions
- **Plan validation** - check planning documents (APS, SpecKit, BMAD) before
  execution
- **Architecture enforcement** - define boundaries, layers, and dependency rules
- **Real-time watch mode** - validate as you code with instant feedback
- **AI authorship tracking** - trace which changes were AI-generated via Git
  Notes
- **OPA/Rego policies** - write custom rules for your organisation

## Commands

| Command                | Description                     |
| ---------------------- | ------------------------------- |
| `anvil tutorial`       | Interactive guided tutorial     |
| `anvil init`           | Set up Anvil in a project       |
| `anvil check --all`    | Scan codebase for issues        |
| `anvil watch --source` | Real-time validation            |
| `anvil gate`           | Run quality gates               |
| `anvil doctor`         | Diagnostics and troubleshooting |
| `anvil explain <rule>` | Understand a warning            |
| `anvil status`         | Show workspace status           |
| `anvil --help`         | See all commands                |

## Requirements

For the native binary: no runtime dependencies. Works on macOS, Linux, and
Windows (x86_64 and aarch64).

For the legacy Node.js package:

- Node.js 20.19.0 or later
- A package manager: **pnpm**, **npm**, **yarn**, or **bun**
- Git

## Beta

This is an early beta release. We welcome bug reports and feedback:

- [Report a bug](https://github.com/EddaCraft/anvil-001/issues/new?template=bug_report.md)
- [Request a feature](https://github.com/EddaCraft/anvil-001/issues/new?template=feature_request.md)
- [Share feedback](https://github.com/EddaCraft/anvil-001/issues/new?template=feedback.md)

## Documentation

- [Beta Quickstart](https://eddacraft.ai/beta)
- [CLI Command Reference](https://github.com/EddaCraft/anvil-001/blob/main/apps/anvil-cli/DEVELOPMENT.md)

## Licence

Apache-2.0
