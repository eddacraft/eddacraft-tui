---
id: quickstart
title: Quickstart
description: Scan your project and see what Anvil finds in under 5 minutes.
sidebar_position: 3
---

# Quickstart

Install Anvil, scan your project, and fix your first issue -- all in under 5
minutes.

## Prerequisites

- **Node.js** 20.0.0 or later
- A package manager: **pnpm**, **npm**, **yarn**, or **bun**
- A TypeScript or JavaScript project

## Install

:::info Closed beta

Anvil is currently in closed beta. You need to
[request access](https://eddacraft.ai/#waitlist) before you can install.

:::

```bash
# Using pnpm (recommended)
pnpm add -D @eddacraft/anvil-cli

# Using npm
npm install -D @eddacraft/anvil-cli

# Using yarn
yarn add -D @eddacraft/anvil-cli

# Using bun
bun add -D @eddacraft/anvil-cli

# Or run without installing
npx @eddacraft/anvil-cli --help
```

## Authenticate

Log in with the beta token from your invite email:

```bash
anvil login
```

You will be prompted for your token. All CLI commands require authentication.

## Initialise

Run the setup wizard in your project root:

```bash
anvil init
```

Anvil detects your project type, creates an `.anvilrc` configuration file, and
sets up the `.anvil/` directory:

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

## Scan Your Project

This is the moment you see what Anvil catches. Run a full scan:

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

If everything passes, you will see:

```
Checking architecture... done
Checking anti-patterns... done
Checking secrets... done

All gates passed.
```

## Turn On Watch Mode

Start Anvil in the background so it validates on every save:

```bash
anvil watch --source
```

```
Anvil Watch

Watching for changes...
Press Ctrl+C to stop.
```

Save a file and see Anvil catch it. Every change is validated in milliseconds,
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
- [Understand gates](/anvil/concepts/gates) -- what Anvil validates and why
- [Configuration reference](/anvil/operations/config) -- customise checks,
  patterns, and watch behaviour

**Feature tutorials:**

- [Custom policies](/anvil/tutorials/policies) -- write OPA/Rego rules for your
  team's standards
- [Architecture boundaries](/anvil/tutorials/architecture) -- define and enforce
  module boundaries
- [Drift detection](/anvil/tutorials/drift) -- capture snapshots and track
  architectural drift
- [CI integration](/anvil/tutorials/ci) -- add Anvil to your pipeline

---

**Need help?** Check [Troubleshooting](/anvil/operations/troubleshooting) or
[open an issue](https://github.com/EddaCraft/anvil-001/issues).
