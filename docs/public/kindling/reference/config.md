---
id: config
title: Configuration
description:
  Environment variables, data locations, and how kindling resolves them.
sidebar_position: 2
owner: DOCSYNC
verified_against: 0.2.0
---

# Configuration

Kindling is configured by **flags and environment variables**, not a config
file. There is no `config.json` to manage — the defaults are derived from your
home directory and the current project.

## Database path resolution

When a command needs a database, kindling resolves the path in this order:

1. an explicit **`--db <path>`** flag;
2. the **`KINDLING_DB_PATH`** environment variable;
3. the **per-project default** under the kindling home —
   `<kindling_home>/projects/<hash>/kindling.db`, where `<hash>` is the first 12
   hex characters of the SHA-256 of the project root path.

The kindling home is `~/.kindling` by default (resolved from `HOME`, or
`USERPROFILE` on Windows). See [Storage](/kindling/concepts/storage) for the
full directory layout.

## Environment variables

| Variable               | Default                     | Effect                                                                                                                                     |
| ---------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `KINDLING_DB_PATH`     | per-project default         | Explicit database path. Overridden only by `--db`.                                                                                         |
| `KINDLING_SOCK`        | `~/.kindling/kindling.sock` | Daemon Unix domain socket path (used by clients and hooks).                                                                                |
| `KINDLING_REPO_ROOT`   | detected from cwd           | Override the project root used to pick the database. Applied only when the working directory is under it (prevents cross-project leakage). |
| `KINDLING_MAX_CONTEXT` | `10`                        | Maximum recent observations injected on Claude Code `SessionStart`.                                                                        |
| `KINDLING_BIN`         | `kindling` on `PATH`        | Override the daemon binary path used by npm/Rust clients and auto-spawn.                                                                   |

`HOME` (or `USERPROFILE` on Windows) is used to locate the kindling home when no
explicit path is given.

### Installer overrides

The [install script](https://github.com/eddacraft/kindling/blob/main/install.sh)
honours these when set before running:

| Variable               | Default              | Effect                            |
| ---------------------- | -------------------- | --------------------------------- |
| `KINDLING_INSTALL_DIR` | `~/.local/bin`       | Where the `kindling` binary lands |
| `KINDLING_VERSION`     | latest release       | Pin a specific release version    |
| `KINDLING_REPO`        | `eddacraft/kindling` | GitHub repo for release assets    |

## Schema version

Each database carries SQLite `user_version` **5** (FTS tokenizer: porter
unicode61). Clients check the daemon's reported schema version on connect and
refuse mismatched pairs.

## Daemon configuration

The [daemon](/kindling/concepts/storage#the-daemon) is configured via `serve`
flags rather than environment:

| Flag                     | Default                     |
| ------------------------ | --------------------------- |
| `--socket <path>`        | `~/.kindling/kindling.sock` |
| `--idle-timeout <secs>`  | `1800` (30 minutes)         |
| `--kindling-home <path>` | `~/.kindling`               |

By convention the daemon also writes:

- a PID file at `~/.kindling/kindling.pid` (Unix)
- a port file at `~/.kindling/kindling.port` when using the Windows TCP loopback
  transport

## Choosing in-process vs daemon

By default the CLI opens the database **in-process**. To route the daemon-backed
verbs (`log`, `capsule`, `search`, `pin`, `unpin`, `forget`) through the running
daemon instead — for safe concurrent access from multiple tools — pass the
global `--via-daemon` flag:

```bash
kindling --via-daemon search "auth"
```

## Next

- [Storage layout →](/kindling/concepts/storage)
- [Full CLI reference →](/kindling/reference/cli)
