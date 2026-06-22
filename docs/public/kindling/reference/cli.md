---
id: cli
title: CLI Reference
description: Complete reference for the kindling command-line interface.
sidebar_position: 1
---

# CLI Reference

The `kindling` binary is the CLI, the daemon, and the Claude Code hook runner in
one artefact. This page documents every command, flag, and default.

## Global options

| Option         | Description                                                                                                                                                                                          |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--via-daemon` | Route the daemon-backed verbs (`log`, `capsule`, `search`, `pin`, `unpin`, `forget`) through the running [daemon](/kindling/concepts/storage#the-daemon) instead of opening the database in-process. |
| `--help`       | Show help.                                                                                                                                                                                           |
| `--version`    | Show the version.                                                                                                                                                                                    |

Most verbs also accept these shared options:

| Option        | Description                                                              |
| ------------- | ------------------------------------------------------------------------ |
| `--db <path>` | Database path. Overrides `KINDLING_DB_PATH` and the per-project default. |
| `--json`      | Emit machine-readable JSON instead of human-readable output.             |

### In-process vs daemon-backed verbs

This table covers data-path verbs. Lifecycle hooks (`hook`) are documented
separately — they always shell out from the binary and do not use the daemon
routing table.

| In-process only                                                         | Daemon-backed (or `--via-daemon`)                    |
| ----------------------------------------------------------------------- | ---------------------------------------------------- |
| `init`, `status`, `list`, `export`, `import`, `demo`, `browse`, `serve` | `log`, `capsule`, `search`, `pin`, `unpin`, `forget` |

## init

Initialise kindling: create the database (running migrations) and optionally
check for Claude Code.

```bash
kindling init [--db <path>] [--claude-code] [--skip-db] [--json]
```

| Flag            | Description                                                                                                          |
| --------------- | -------------------------------------------------------------------------------------------------------------------- |
| `--db <path>`   | Use an explicit database path instead of the per-project default.                                                    |
| `--claude-code` | Detect `~/.claude/` and print next steps for the plugin. Does **not** install or configure the plugin automatically. |
| `--skip-db`     | Skip database creation (integration check only).                                                                     |
| `--json`        | JSON output (`{ directory, database, claudeCode }`).                                                                 |

## demo

Load bundled sample memory so you can try search and browse without capturing
anything first. Writes to `~/.kindling/demo/kindling.db` by default.

```bash
kindling demo [--reset] [--db <path>] [--json]
```

| Flag          | Description                                              |
| ------------- | -------------------------------------------------------- |
| `--reset`     | Replace an existing demo database before importing.      |
| `--db <path>` | Use a custom database path instead of the demo default.  |
| `--json`      | Emit machine-readable JSON with suggested next commands. |

See [Quickstart without Claude Code](/kindling/quickstart/without-claude-code).

## log

Log an observation to memory.

```bash
kindling log <content> [--kind <kind>] [--session <id>] [--repo <id>] [--capsule <id>]
```

| Argument / flag  | Description                                                                                                                                                 |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<content>`      | The observation content (positional, required).                                                                                                             |
| `--kind <kind>`  | Observation kind. Default `message`. One of `tool_call`, `command`, `file_diff`, `error`, `message`, `node_start`, `node_end`, `node_output`, `node_error`. |
| `--session <id>` | Session scope ID.                                                                                                                                           |
| `--repo <id>`    | Repository scope ID.                                                                                                                                        |
| `--capsule <id>` | Attach to an existing capsule.                                                                                                                              |

```bash
kindling log "JWT tokens expire after 15 minutes, not 1 hour"
kindling log --kind error "segfault in auth middleware after the upgrade"
```

## capsule

Manage capsules. Two subcommands: `open` and `close`.

### capsule open

```bash
kindling capsule open --intent <text> [--type <type>] [--session <id>] [--repo <id>]
```

| Flag              | Description                        |
| ----------------- | ---------------------------------- |
| `--intent <text>` | Purpose of the capsule (required). |
| `--type <type>`   | Capsule type. Default `session`.   |
| `--session <id>`  | Session scope ID.                  |
| `--repo <id>`     | Repository scope ID.               |

### capsule close

```bash
kindling capsule close <id> [--summary <text>]
```

| Argument / flag    | Description                                 |
| ------------------ | ------------------------------------------- |
| `<id>`             | Capsule ID to close (positional, required). |
| `--summary <text>` | Summary text attached to the capsule.       |

## status

Show database status and statistics (path, size, and counts of capsules,
observations, summaries, pins, redacted items, and open capsules).

```bash
kindling status [--db <path>] [--json]
```

## search

Search for relevant context. Returns the three retrieval tiers — pins, current
summary, and ranked candidates. See [Retrieval](/kindling/concepts/retrieval).

```bash
kindling search <query> [--session <id>] [--repo <id>] [--max <n>]
```

| Argument / flag  | Description                                 |
| ---------------- | ------------------------------------------- |
| `<query>`        | Query string (positional, required).        |
| `--session <id>` | Filter by session ID.                       |
| `--repo <id>`    | Filter by repository ID.                    |
| `--max <n>`      | Maximum candidates to return. Default `10`. |

Candidate scores are normalised to the range **0.0–1.0**.

## list

List entities.

```bash
kindling list <entity> [--session <id>] [--repo <id>] [--limit <n>]
```

| Argument / flag  | Description                                |
| ---------------- | ------------------------------------------ |
| `<entity>`       | One of `capsules`, `pins`, `observations`. |
| `--session <id>` | Filter by session ID.                      |
| `--repo <id>`    | Filter by repository ID.                   |
| `--limit <n>`    | Maximum results. Default `20`.             |

There is no `--status` or `--kind` filter on `list` today — filter results in
your shell or use `search` for ranked retrieval.

## browse

Export the database to a self-contained offline HTML viewer and optionally open
it in your default browser.

```bash
kindling browse [--output <path>] [--no-open] [--db <path>] [--json]
```

| Flag              | Description                                                              |
| ----------------- | ------------------------------------------------------------------------ |
| `--output <path>` | Output HTML path. Default: temp file `kindling-browse-<timestamp>.html`. |
| `--no-open`       | Print the output path only; do not launch a browser.                     |
| `--db <path>`     | Database to export. Default: per-project resolution.                     |
| `--json`          | Emit `{ path }` as JSON.                                                 |

## pin

Pin an observation or a summary so it is always returned first and never
evicted.

```bash
kindling pin <type> <id> [--note <text>] [--ttl <ms>]
```

| Argument / flag | Description                                                      |
| --------------- | ---------------------------------------------------------------- |
| `<type>`        | Target type: `observation` or `summary`.                         |
| `<id>`          | Target ID.                                                       |
| `--note <text>` | Note describing why this is pinned (stored as `reason` in JSON). |
| `--ttl <ms>`    | Time-to-live in **milliseconds**, after which the pin expires.   |

## unpin

Remove a pin by ID.

```bash
kindling unpin <id>
```

## forget

Redact (forget) an observation by its exact ID. Masks the content; the record is
retained.

```bash
kindling forget <id>
```

## export

Export memory to a JSON file.

```bash
kindling export [output] [--session <id>] [--repo <id>] [--pretty]
```

| Argument / flag  | Description                                              |
| ---------------- | -------------------------------------------------------- |
| `[output]`       | Output file. Default `kindling-export-<timestamp>.json`. |
| `--session <id>` | Export only a specific session.                          |
| `--repo <id>`    | Export only a specific repository.                       |
| `--pretty`       | Pretty-print the JSON.                                   |

See [Formats](/kindling/reference/formats) for the bundle shape.

## import

Import memory from an export file. Conflicting IDs are skipped
(`INSERT OR IGNORE`).

```bash
kindling import <file> [--dry-run]
```

| Argument / flag | Description                 |
| --------------- | --------------------------- |
| `<file>`        | Bundle file to import.      |
| `--dry-run`     | Validate without importing. |

## serve

Start the kindling daemon (HTTP/1 over a Unix domain socket on Linux/macOS; TCP
loopback on Windows).

```bash
kindling serve [--socket <path>] [--idle-timeout <secs>] [--kindling-home <path>] [--daemonize]
```

| Flag                     | Description                                                                         |
| ------------------------ | ----------------------------------------------------------------------------------- |
| `--socket <path>`        | Unix domain socket to bind. Default `~/.kindling/kindling.sock`.                    |
| `--idle-timeout <secs>`  | Seconds idle before the daemon shuts itself down. Default `1800`.                   |
| `--kindling-home <path>` | Root of per-project databases. Default `~/.kindling` (or the parent of `--socket`). |
| `--daemonize`            | Suppress the human-readable startup banner (used when auto-spawned).                |

## hook

Run a Claude Code lifecycle hook. Reads the hook context as JSON on stdin and
always exits 0. You normally don't call this directly — the
[Claude Code plugin](/kindling/adapters/claude-code) wires it up.

```bash
kindling hook <type>
```

`<type>` is one of: `session-start`, `post-tool-use`, `post-tool-use-failure`,
`user-prompt-submit`, `subagent-stop`, `stop`, `pre-compact`. The binary also
responds to `<type>` when invoked under the program name `kindling-hook`.

## Next

- [Configuration & environment variables →](/kindling/reference/config)
- [Export / import format →](/kindling/reference/formats)
