---
id: quickstart
title: Quickstart
description: Scan your project and see what anvil finds in under 5 minutes.
sidebar_position: 3
---

# Quickstart

Install anvil, scan your project, and fix your first issue -- all in under 5
minutes.

## Prerequisites

- A TypeScript or JavaScript project
- A terminal (macOS, Linux, or Windows)

## Install

:::info Early access

anvil is still in early access. The install flow below is the fresh-start path
for the current Rust CLI. If your team has gated beta access, use the GitHub
account tied to that access when prompted by anvil or the docs site.

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

anvil is a single native binary available for macOS, Linux, and Windows. Your
project still needs Node.js and a package manager for lint and test gate checks,
but anvil itself has no runtime dependencies.

:::tip Windows users

If the installer doesn't add `anvil` to your PATH automatically, add
`%USERPROFILE%\.eddacraft\bin` to your system PATH or run:

```powershell
$env:Path = "$env:USERPROFILE\.eddacraft\bin;$env:Path"
```

:::

## Authenticate

Start the default device-code login flow:

```bash
anvil auth login
```

anvil prints a short code and verification URL. Open the URL in your browser,
enter the code, and the CLI will finish the login automatically.

If you need email OTP instead, run:

```bash
anvil auth login --otp
```

## Initialise

Run the setup wizard in your project root:

```bash
anvil init
```

anvil detects your project type, creates an `.anvilrc` configuration file, and
sets up the `.anvil/` directory:

```
Initialising anvil in current project...

Detected environment:
  Project: my-app
  Package Manager: pnpm
  Git: yes
  TypeScript: yes

anvil initialised successfully!

Created files:
  .anvilrc
  .anvil/
```

## Scan Your Project

This is the moment you see what anvil catches. Run the fast source scan first:

```bash
anvil check --all
```

`anvil check` is the targeted source-analysis command. In the current Rust CLI it
scans source artefacts for Anvil's registry-backed anti-pattern rules, including
architecture-category findings emitted by that scanner. Use `anvil gate` when you
want the full workflow judgement across architecture, policy, secrets, and other
gate checks.

Most projects have something. Here is typical `check` output:

```
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

If everything passes, you will see:

```
Checked 12 file(s)

Checking anti-patterns... done

No warnings found
```

To run the broader gate surface, use:

```bash
anvil gate --profile dev
```

## Turn On Watch Mode

Start anvil so it validates on every save:

```bash
anvil watch --source
```

```
Anvil Watch

Watching for changes...
Press Ctrl+C to stop.
```

Save a file and see anvil catch it. Every change is validated in milliseconds,
not minutes.

:::tip

Run watch mode in a dedicated terminal pane or use the VS Code extension for
in-editor diagnostics.

:::

## Fix Your First Issue

Take one of the warnings from the scan -- say AP-003, the explicit `any` in
`src/utils/parser.ts`:

**Before:**

```typescript
export function parse(input: any): Record<string, unknown> {
  // ...
}
```

**After:**

```typescript
export function parse(input: string): Record<string, unknown> {
  // ...
}
```

Save the file. If watch mode is running you will see immediate confirmation:

```
Change detected: src/utils/parser.ts

Checking anti-patterns... done

All gates passed.
```

One warning down. Repeat for the rest at your own pace.

## Next Steps

- **Interactive tutorial** -- run `anvil tutorial` for a guided walk-through
  inside your terminal
- [Set up your first project](/anvil/first-project) -- architecture boundaries,
  suppressions, and CI
- [Understand gates](/anvil/concepts/gates) -- what anvil validates and why
- [Configuration reference](/anvil/operations/config) -- customise checks,
  patterns, and watch behaviour

**Feature tutorials:**

- [Custom policies](/anvil/tutorials/policies) -- write OPA/Rego rules for your
  team's standards
- [Architecture boundaries](/anvil/tutorials/architecture) -- define and enforce
  module boundaries
- [Drift detection](/anvil/tutorials/drift) -- capture snapshots and track
  architectural drift
- [CI integration](/anvil/tutorials/ci) -- add anvil to your pipeline

---

**Next:** [Set up your first project →](/anvil/first-project)

**Need help?** Check [Troubleshooting](/anvil/operations/troubleshooting) or
[open an issue](https://github.com/eddacraft/anvil/issues).
