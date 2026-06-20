---
id: install
title: Install
description: Install the kindling binary and initialise project memory.
sidebar_position: 1
---

# Install Kindling

Kindling ships as a single binary, `kindling`, that provides the CLI, the
background daemon, and Claude Code hook support. Install it through whichever
channel suits you.

## Install the binary

### Rust / Cargo (canonical)

```bash
cargo install kindling
```

This builds and installs the `kindling` binary from
[crates.io](https://crates.io/crates/kindling).

### One-line installer (Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/eddacraft/kindling/main/install.sh | sh
```

### npm

A prebuilt-binary npm package is also published. Node.js >= 20 is required.

```bash
npm install -g @eddacraft/kindling-cli
# or: pnpm add -g @eddacraft/kindling-cli
# or: yarn global add @eddacraft/kindling-cli
# or: bun add -g @eddacraft/kindling-cli
```

### Verify

```bash
kindling --version
```

## Initialise a project

From your project directory, run:

```bash
kindling init
```

This creates the per-project database (running its migrations) under your
kindling home:

```
Kindling Setup
==============

Created directory /home/you/.kindling/projects/f33aa9244af5
Created database  /home/you/.kindling/projects/f33aa9244af5/kindling.db

Kindling is ready!

Next steps:
  kindling status     - Check database status
  kindling search     - Search your memory
  kindling serve      - Start the daemon
```

Each project gets its own database, keyed by a hash of the project root path, so
memory never leaks between repositories. See
[Storage](/kindling/concepts/storage) for the full layout.

### Options

| Flag             | Description                                              |
| ---------------- | -------------------------------------------------------- |
| `--db <path>`    | Use an explicit database path instead of the per-project default. |
| `--claude-code`  | Detect and (when available) configure Claude Code integration.    |
| `--skip-db`      | Configure integration only; do not create the database.  |
| `--json`         | Emit machine-readable JSON instead of human output.      |

## Verify the setup

```bash
kindling log "First observation"
kindling search "first"
kindling status
```

`kindling status` reports the database path, size, and counts of capsules,
observations, and pins.

## Next steps

- [Capture and search your first memory →](/kindling/quickstart/first-memory)
- [Set up automatic capture →](/kindling/quickstart/automatic-capture)
- [Configuration & data locations →](/kindling/reference/config)
