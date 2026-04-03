---
id: install
title: Install
description: Installing Kindling on your system.
sidebar_position: 1
---

# Install Kindling

Get Kindling running on your system.

## Requirements

- **Node.js** >= 20.19.0
- A package manager: **pnpm**, **npm**, **yarn**, or **bun**

## Installation

:::info Closed beta

Kindling is part of the Edda stack and currently in early access.
[Request access](https://eddacraft.ai/#waitlist) if you don't have an invite
yet.

:::

### Global Install (Recommended)

```bash
# Using pnpm
pnpm add -g @eddacraft/kindling

# Using npm
npm install -g @eddacraft/kindling

# Using yarn
yarn global add @eddacraft/kindling

# Using bun
bun add -g @eddacraft/kindling

# Or run without installing
npx @eddacraft/kindling --help
```

Verify installation:

```bash
kindling --version
```

### Project Install

Add to a project:

```bash
pnpm add -D @eddacraft/kindling
```

Run via:

```bash
pnpm kindling --help
```

## Initial Setup

After installation, initialise Kindling:

```bash
kindling init
```

This creates:

- `~/.kindling/` — data directory
- `~/.kindling/config.json` — configuration
- Default capsule — for immediate use

**Output:**

```
Kindling initialised.

Data directory: ~/.kindling
Default capsule: default

Next steps:
  kindling capsule create my-project
  kindling observe "First observation"
  kindling search "observation"
```

## Verify Installation

Test that everything works:

```bash
# Create a test observation
kindling observe "Test observation"

# Search for it
kindling search "test"
```

**Expected output:**

```
[2024-01-15T10:30:00Z] Test observation
  Capsule: default
  Source: manual
```

## Configuration

View current config:

```bash
kindling config show
```

Default configuration:

```json
{
  "dataDir": "~/.kindling",
  "defaultCapsule": "default",
  "storage": {
    "type": "sqlite"
  }
}
```

### Change Data Directory

```bash
kindling config set dataDir /path/to/data
```

### Change Default Capsule

```bash
kindling config set defaultCapsule my-project
```

## Uninstall

```bash
# Remove the package
pnpm remove -g @eddacraft/kindling

# Optionally remove data
rm -rf ~/.kindling
```

---

**Next:** [Create a capsule →](/kindling/quickstart/create-capsule)
