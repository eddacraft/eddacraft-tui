---
id: install
title: Install
description: Install the kindling binary and initialise project memory.
sidebar_position: 1
owner: DOCSYNC
---

# Install Kindling

Kindling ships as a single binary, `kindling`, that provides the CLI, the
background daemon, and Claude Code hook support. Install it through whichever
channel suits you.

## Install the binary

### Prebuilt binary (recommended)

The one-line installer downloads the prebuilt `kindling` binary for your
platform — no Node.js or Rust toolchain required (Linux / macOS):

```bash
curl -fsSL https://raw.githubusercontent.com/eddacraft/kindling/main/install.sh | sh
```

### Rust / Cargo

The CLI is published as the
[`eddacraft-kindling`](https://crates.io/crates/eddacraft-kindling) crate (the
bare `kindling` name on crates.io is taken). The installed binary is still
`kindling`:

```bash
cargo install eddacraft-kindling
```

### Homebrew (macOS / Linux)

```bash
brew install eddacraft/tap/kindling
```

Same tap as anvil:
[eddacraft/homebrew-tap](https://github.com/eddacraft/homebrew-tap). On
musl/Alpine Linux, use the install script instead.

### Node.js

The canonical CLI is the Rust binary above. For Node applications, the npm
package `@eddacraft/kindling` is a **thin client library** that talks to the
same Rust daemon — it is not a global CLI. At publish time it declares optional
per-platform binary dependencies so `npm install` pulls a matching prebuilt
`kindling` binary for your OS/arch:

```bash
npm install @eddacraft/kindling
```

> The older standalone CLI package `@eddacraft/kindling-cli` is **deprecated**;
> use the prebuilt installer or `cargo install eddacraft-kindling` for the CLI.
> The embedded TypeScript stack (`@eddacraft/kindling-core`, `-store-*`) is also
> deprecated — use the thin client instead.

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

| Flag            | Description                                                                           |
| --------------- | ------------------------------------------------------------------------------------- |
| `--db <path>`   | Use an explicit database path instead of the per-project default.                     |
| `--claude-code` | Detect `~/.claude/` and print plugin install next steps. Does not install the plugin. |
| `--skip-db`     | Configure integration only; do not create the database.                               |
| `--json`        | Emit machine-readable JSON instead of human output.                                   |

## Verify the setup

```bash
kindling log "First observation"
kindling search "first"
kindling status
```

`kindling status` reports the database path, size, and counts of capsules,
observations, and pins.

## Try before you init

You can explore kindling without creating a project database:

```bash
kindling demo
kindling search "JWT" --db ~/.kindling/demo/kindling.db
kindling browse
```

See [Quickstart without Claude Code](/kindling/quickstart/without-claude-code).

## Next steps

- [Capture and search your first memory →](/kindling/quickstart/first-memory)
- [Set up automatic capture →](/kindling/quickstart/automatic-capture)
- [Configuration & data locations →](/kindling/reference/config)
