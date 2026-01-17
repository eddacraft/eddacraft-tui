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
# Using pnpm (recommended)
pnpm add -D @anvil/cli

# Using npm
npm install -D @anvil/cli

# Using yarn
yarn add -D @anvil/cli
```

Or install globally:

```bash
pnpm add -g @anvil/cli
```

## Initialise Anvil

Run the interactive setup wizard:

```bash
anvil init
```

This will:

1. Detect your project type
2. Create an `anvil.config.json` file
3. Set up default gate checks
4. Optionally create an initial plan

**Expected output:**

```
🔨 Anvil Init

Detected: TypeScript project with pnpm
Creating configuration...

✓ Created anvil.config.json
✓ Configured default gates
✓ Ready to run

Next steps:
  anvil status    View current configuration
  anvil watch     Start watching for changes
  anvil run       Run gates once
```

## Your First Run

Run Anvil once to see current issues:

```bash
anvil run
```

**Expected output (clean project):**

```
🔨 Anvil Run

Checking architecture... ✓
Checking anti-patterns... ✓
Checking secrets... ✓

All gates passed.
```

**Expected output (issues found):**

```
🔨 Anvil Run

Checking architecture... ✓
Checking anti-patterns...
  ⚠️  AP-003: Explicit 'any' type
      src/utils/parser.ts:42:10
  ⚠️  AP-006: Empty catch block
      src/services/api.ts:87:5

2 warnings found.
Gate status: WARN
```

## Start Watch Mode

For the best experience, run Anvil in watch mode:

```bash
anvil watch
```

Anvil will now validate your code every time you save a file.

**Tip:** Run this in a dedicated terminal pane or use the VS Code extension.

## Configuration

Your `anvil.config.json` controls which checks run:

```json
{
  "version": "1.0",
  "gates": {
    "architecture": {
      "enabled": true,
      "boundaries": []
    },
    "antiPatterns": {
      "enabled": true,
      "patterns": ["AP-001", "AP-003", "AP-004", "AP-006"]
    },
    "secrets": {
      "enabled": true
    }
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
