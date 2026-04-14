---
id: rust-rewrite
title: The Switch to Rust
description:
  Why we rewrote the anvil CLI in Rust, what changed, and how to migrate from
  the Node.js package.
sidebar_position: 10
---

# The Switch to Rust

Starting with `0.3.0-beta`, the anvil CLI is a native binary written in Rust.
The Node.js package (`@eddacraft/anvil-cli`) is deprecated and will not receive
further updates.

## Why Rust

anvil watches your codebase and validates changes at save-time. That means the
CLI sits in a hot loop: parse files, walk dependency graphs, evaluate policies,
and render results — all within the time it takes you to glance at the terminal
after pressing save.

The Node.js implementation was good enough for small projects, but it hit walls:

- **Cold start** — Node.js takes 200-400ms just to load the runtime and parse
  the dependency tree. The Rust binary starts in under 10ms.
- **Memory** — A 5,000-file monorepo consumed 400MB+ of RSS in watch mode. The
  Rust watcher sits at 30-50MB for the same project.
- **Parse throughput** — Tree-sitter bindings in Rust parse TypeScript at
  ~15,000 files/second. The Node.js equivalent managed ~2,000 files/second with
  the same grammar.
- **Concurrency** — File watching, parsing, graph updates, and policy evaluation
  now run on separate threads with zero-copy message passing. Node.js required
  worker threads with serialisation overhead.
- **Distribution** — A single static binary with no runtime dependencies. No
  more "which Node.js version?", no more `node_modules`, no more npm registry
  authentication for private packages.

The result: anvil is 5-10x faster on typical projects and uses around 80% less
memory in watch mode.

## What Changed

### Installation

**Before (Node.js):**

```bash
pnpm add -D @eddacraft/anvil-cli
# or: npm install -D @eddacraft/anvil-cli
```

**Now (native binary):**

```bash
# macOS / Linux
curl -fsSL https://install.eddacraft.ai | sh

# Windows
irm https://install.eddacraft.ai/windows | iex

# Or via Homebrew (macOS/Linux)
brew install eddacraft/tap/anvil
```

The install script detects your platform and architecture (x86_64, aarch64) and
drops the binary into `~/.eddacraft/bin/` (macOS/Linux) or
`%USERPROFILE%\.eddacraft\bin\` (Windows). Add it to your PATH if the installer
doesn't do so automatically.

**Supported platforms:**

| OS      | Architecture          | Binary      |
| ------- | --------------------- | ----------- |
| macOS   | x86_64, Apple Silicon | `anvil`     |
| Linux   | x86_64, aarch64       | `anvil`     |
| Windows | x86_64, aarch64       | `anvil.exe` |

### Commands

Most commands remain the same. `anvil watch`, `anvil init`, and
`anvil tutorial` still work as before from a user's perspective.

:::note Command changes

Both the legacy Node.js CLI and the Rust CLI expose separate `anvil check` and
`anvil gate` commands:

- **`anvil check`** — static analysis: scans files for anti-patterns and
  architecture violations. Use for quick file-level scanning.
- **`anvil gate`** — quality gate: runs all check categories (lint, test,
  coverage, dependency, secret, architecture, policy) with configurable profiles
  (`dev`, `ci`, `production`).

CI workflows that used `anvil check --all --ci` should migrate to
`anvil gate --profile ci`.

:::

### Configuration

`.anvilrc` files are fully compatible. No configuration changes are needed.

### CI Integration

**Before:**

```yaml
- run: pnpm install
- run: pnpm anvil check --all --ci
```

**Now (Linux/macOS):**

```yaml
- name: Install anvil
  run: curl -fsSL https://install.eddacraft.ai | sh

- name: Run anvil
  run: anvil gate --profile ci
```

**Now (Windows):**

```yaml
- name: Install anvil
  shell: pwsh
  run: irm https://install.eddacraft.ai/windows | iex

- name: Run anvil
  run: anvil gate --profile ci
```

The anvil binary itself requires no Node.js runtime. However, some gate checks
(lint, test) shell out to your project's package manager, so your CI workflow
should still install project dependencies if those checks are enabled.

### Terminal UI

The interactive surfaces (tutorial, watch, wizard, and status) have been
rebuilt using Ratatui with the eddacraft Terminal Standard design system. The
experience is smoother, more responsive, and more consistent across terminal
emulators.

### What's New in Rust

Features that were difficult or impractical in the Node.js version:

- **Kernel engine** — a persistent daemon mode with incremental parsing and a
  semantic dependency graph that updates in real time as files change
- **Policy evaluation** — policy configuration and rule loading are handled
  natively; OPA is still required for Rego evaluation
- **Structured exit codes** — `0` (pass), `1` (error), `2` (gate failure), `3`
  (auth required), `4` (config error) for precise CI integration
- **Cross-platform auth** — device-flow authentication with secure credential
  storage via the OS keychain

## Migration Guide

### Step 1: Install the native binary

**macOS / Linux:**

```bash
curl -fsSL https://install.eddacraft.ai | sh
```

**Windows (PowerShell):**

```powershell
irm https://install.eddacraft.ai/windows | iex
```

### Step 2: Verify

```bash
anvil --version
# anvil 0.3.0-beta
```

### Step 3: Remove the Node.js package

```bash
# If installed globally
npm uninstall -g @eddacraft/anvil-cli

# If installed as a project dependency
pnpm remove @eddacraft/anvil-cli
# or: npm uninstall @eddacraft/anvil-cli
```

### Step 4: Authenticate

```bash
anvil login
```

If you were previously authenticated with the Node.js CLI, your credentials
migrate automatically on first run. Run `anvil login` only if prompted or if you
see exit code 3 (auth required).

### Step 5: Update CI

Replace any `pnpm anvil` or `npx anvil` invocations with direct `anvil` calls.
Remove the Node.js install step if anvil was the only reason it was there.

### Step 6: Test

```bash
anvil gate
```

Your `.anvilrc` and `.anvil/` directory work without changes.

## Reporting Issues

The Rust CLI is in beta. If you find something that worked in the Node.js
version but does not work in Rust, please
[open an issue](https://github.com/eddacraft/anvil/issues) and mention
`rust-migration` in the title or body.

### What to include

- anvil version (`anvil --version`)
- Operating system and architecture:
  - macOS / Linux: `uname -a`
  - Windows (PowerShell): `[System.Environment]::OSVersion` and
    `$env:PROCESSOR_ARCHITECTURE`
- Steps to reproduce
- Expected behaviour vs actual behaviour
