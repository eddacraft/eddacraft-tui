---
id: storage
title: Storage
description: How and where Kindling stores memory — per-project SQLite with WAL and FTS5.
sidebar_position: 4
---

# Storage

Kindling stores everything locally in SQLite. There is no server to run, no
account, and no network dependency — memory is just files on your machine.

## Per-project databases

Each project gets its own database, so memory never bleeds between repositories.
Databases live under your **kindling home** (`~/.kindling` by default), keyed by
a stable hash of the project root path:

```
~/.kindling/
├── projects/
│   ├── f33aa9244af5/
│   │   └── kindling.db      # one project
│   └── 9b1c0e7a42d1/
│       └── kindling.db      # another project
├── kindling.sock            # daemon Unix domain socket
└── kindling.pid             # daemon PID file
```

The `<hash>` is the first 12 hex characters of the SHA-256 of the project root
path. The CLI, the daemon, and the Claude Code hooks all derive the same hash
for the same project, so they share one database.

## SQLite with WAL and FTS5

Each `kindling.db` is a standard SQLite database using:

- **WAL (write-ahead logging)** for safe concurrent access — multiple tools
  (and the daemon) can read and write at once.
- **FTS5 full-text search** over observation and summary content, which powers
  the ranked candidate tier of [retrieval](/kindling/concepts/retrieval).

Because it is plain SQLite, a database is portable: copy the file to back it up,
move it between machines, or inspect it with any SQLite tool.

## The daemon

For concurrent, multi-tool access, Kindling runs a background **daemon** that
owns the databases and serves requests over a Unix domain socket (HTTP/1 over
UDS):

```bash
kindling serve
```

```
kindling home: /home/you/.kindling
listening on /home/you/.kindling/kindling.sock
```

The daemon routes each request to the right per-project database using the
project root supplied by the caller. Clients (including
[`kindling-client`](/kindling/reference/crates)) auto-spawn the daemon on first
use if it is not already running, and it shuts itself down after an idle
timeout (default 30 minutes).

CLI verbs run **in-process** against the database by default. Pass
`--via-daemon` to route the daemon-backed verbs (`log`, `capsule`, `search`,
`pin`, `unpin`, `forget`) through the running daemon instead — useful when
several tools touch the same project at once.

## Choosing the database path

Kindling resolves which database to use in this order:

1. an explicit `--db <path>` flag;
2. the `KINDLING_DB_PATH` environment variable;
3. the per-project default under the kindling home.

See [Configuration](/kindling/reference/config) for all environment variables
and paths.

## Backup, restore, and migration

The portable option is the export bundle:

```bash
kindling export ./backup.json --pretty   # whole store (or --session / --repo)
kindling import ./backup.json
```

Or copy the SQLite file directly:

```bash
cp ~/.kindling/projects/f33aa9244af5/kindling.db ~/backups/
```

Schema migrations run automatically when a database is opened (including on
`kindling init`), so upgrading the binary keeps existing databases current.

## Next

- [Configuration & environment variables →](/kindling/reference/config)
- [Export / import format →](/kindling/reference/formats)
