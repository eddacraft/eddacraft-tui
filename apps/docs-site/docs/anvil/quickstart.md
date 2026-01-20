---
id: quickstart
title: Quickstart
description: Get Anvil running in your project in under 5 minutes.
sidebar_position: 3
---

# Quickstart

Get Anvil running in your project in under 5 minutes.

## Prerequisites

- **Node.js** 20.0.0 or later
- **pnpm**, **npm**, or **yarn**
- A TypeScript or JavaScript project

## Installation

```bash
# Clone and build (pre-release)
git clone https://github.com/EddaCraft/anvil-001.git
cd anvil-001 && pnpm install && pnpm build

# Link CLI globally
pnpm link:cli
```

Once published to npm, installation will be:

```bash
# Using pnpm (recommended)
pnpm add -D @eddacraft/anvil-cli

# Using npm
npm install -D @eddacraft/anvil-cli
```

## Initialise Anvil

Run the interactive setup wizard:

```bash
cd /path/to/your/project
anvil init
```

This will:

1. Detect your project type
2. Create an `.anvilrc` configuration file
3. Set up default gate checks
4. Create the `.anvil/` directory

**Expected output:**

```
🔨 Initialising Anvil in current project...

Detected environment:
  Project: my-app
  Package Manager: pnpm
  Git: ✓
  TypeScript: ✓

✓ Anvil initialised successfully!

Created files:
  ✓ .anvilrc
  ✓ .anvil/

Next steps:
  anvil status    View current configuration
  anvil check     Run checks on changed files
  anvil watch     Start watching for changes
```

## Your First Check

Run Anvil once to see current issues:

```bash
anvil check --changed
```

**Expected output (clean project):**

```
Checking architecture... ✓
Checking anti-patterns... ✓
Checking secrets... ✓

All gates passed.
```

**Expected output (issues found):**

```
Checking architecture... ✓
Checking anti-patterns...
  ⚠ [AP-003] Explicit any type detected
    src/utils/parser.ts:42
    Using 'any' defeats type safety
    Fix: Define a proper type or use 'unknown'

1 warning found.
```

## Start Watch Mode

For the best experience, run Anvil in watch mode:

```bash
anvil watch --source
```

Anvil will now validate your code every time you save a file.

**Tip:** Run this in a dedicated terminal pane or use the VS Code extension.

## Configuration

Your `.anvilrc` controls which checks run:

```json
{
  "checks": {
    "architecture": {
      "enabled": true,
      "baseline": ".anvil/baseline.json"
    },
    "antipattern": {
      "enabled": true,
      "patterns": ["AP-001", "AP-003", "AP-004", "AP-006"]
    },
    "secrets": {
      "enabled": true
    }
  },
  "watch": {
    "patterns": ["src/**/*.ts"],
    "debounceMs": 300
  }
}
```

See [Configuration](/docs/anvil/operations/config) for full options.

## Next Steps

- [Set up your first project →](/docs/anvil/first-project)
- [Understand gates →](/docs/anvil/concepts/gates)
- [Configure architecture boundaries →](/docs/anvil/concepts/plans)

---

**Need help?** Check [Troubleshooting](/docs/anvil/operations/troubleshooting)
or [open an issue](https://github.com/EddaCraft/anvil-001/issues).
