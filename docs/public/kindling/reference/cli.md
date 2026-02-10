---
id: cli
title: CLI Reference
description: Complete Kindling CLI command reference.
sidebar_position: 2
---

# CLI Reference

Complete reference for the Kindling command-line interface.

## Global Options

```
--help, -h       Show help
--version, -v    Show version
--verbose        Verbose output
--quiet, -q      Suppress output
--config PATH    Config file path
```

## observe

Record an observation.

```bash
kindling observe <content> [options]
```

### Options

| Option          | Short | Description            |
| --------------- | ----- | ---------------------- |
| `--kind`        | `-k`  | Observation kind       |
| `--tag`         | `-t`  | Add tag (repeatable)   |
| `--capsule`     | `-c`  | Target capsule         |
| `--source`      | `-s`  | Source identifier      |
| `--context`     |       | File/line context      |
| `--file`        | `-f`  | Read content from file |
| `--clipboard`   |       | Read from clipboard    |
| `--interactive` | `-i`  | Interactive mode       |

### Examples

```bash
kindling observe "API uses OAuth2"
kindling observe "Rate limit" --kind gotcha --tag api
kindling observe --file notes.md --capsule my-project
kindling observe --interactive
```

## search

Search observations.

```bash
kindling search [query] [options]
```

### Options

| Option           | Short | Description         |
| ---------------- | ----- | ------------------- |
| `--capsule`      | `-c`  | Search in capsule   |
| `--all-capsules` | `-a`  | Search all capsules |
| `--tag`          | `-t`  | Filter by tag       |
| `--kind`         | `-k`  | Filter by kind      |
| `--since`        |       | After date/duration |
| `--until`        |       | Before date         |
| `--limit`        | `-n`  | Max results         |
| `--export`       | `-e`  | Export format       |

### Examples

```bash
kindling search "authentication"
kindling search --tag api --since 7d
kindling search "error" --kind gotcha --limit 10
kindling search "api" --export markdown
```

## capsule

Manage capsules.

```bash
kindling capsule <command> [options]
```

### Subcommands

| Command          | Description          |
| ---------------- | -------------------- |
| `list`           | List capsules        |
| `create <name>`  | Create capsule       |
| `use <name>`     | Set default capsule  |
| `show <name>`    | Show capsule details |
| `archive <name>` | Archive capsule      |
| `delete <name>`  | Delete capsule       |
| `export <name>`  | Export capsule       |

### Examples

```bash
kindling capsule list
kindling capsule create my-project
kindling capsule use my-project
kindling capsule show my-project
kindling capsule archive old-project
```

## export

Export observations.

```bash
kindling export [options]
```

### Options

| Option      | Short | Description       |
| ----------- | ----- | ----------------- |
| `--capsule` | `-c`  | Capsule to export |
| `--format`  | `-f`  | Output format     |
| `--since`   |       | After date        |
| `--until`   |       | Before date       |
| `--output`  | `-o`  | Output file       |

### Formats

- `json` — Full JSON export
- `markdown` — Readable markdown
- `context` — LLM context format
- `csv` — Spreadsheet format

### Examples

```bash
kindling export --capsule my-project --format markdown
kindling export --format json --output backup.json
kindling export --since 7d --format context
```

## import

Import observations.

```bash
kindling import <file> [options]
```

### Options

| Option         | Description               |
| -------------- | ------------------------- |
| `--capsule`    | Target capsule            |
| `--kind`       | Default kind              |
| `--parse-tags` | Extract inline tags       |
| `--dry-run`    | Preview without importing |

### Examples

```bash
kindling import backup.json
kindling import notes.md --capsule my-project --parse-tags
kindling import data.csv --dry-run
```

## recent

Show recent observations.

```bash
kindling recent [options]
```

### Options

| Option      | Short | Description                  |
| ----------- | ----- | ---------------------------- |
| `--capsule` | `-c`  | From capsule                 |
| `--limit`   | `-n`  | Number to show (default: 10) |
| `--source`  |       | Filter by source             |

### Examples

```bash
kindling recent
kindling recent -n 20 --capsule my-project
kindling recent --source opencode
```

## show

Show observation details.

```bash
kindling show <id>
```

### Examples

```bash
kindling show obs_abc123
```

## stats

Show statistics.

```bash
kindling stats [options]
```

### Options

| Option      | Short | Description  |
| ----------- | ----- | ------------ |
| `--capsule` | `-c`  | For capsule  |
| `--all`     | `-a`  | All capsules |

### Examples

```bash
kindling stats
kindling stats --capsule my-project
```

## config

Manage configuration.

```bash
kindling config <command> [options]
```

### Subcommands

| Command             | Description        |
| ------------------- | ------------------ |
| `show`              | Show configuration |
| `set <key> <value>` | Set value          |
| `get <key>`         | Get value          |
| `reset`             | Reset to defaults  |

### Examples

```bash
kindling config show
kindling config set defaultCapsule my-project
kindling config get dataDir
```

## adapter

Manage adapters.

```bash
kindling adapter <command> [options]
```

### Subcommands

| Command          | Description       |
| ---------------- | ----------------- |
| `list`           | List installed    |
| `install <name>` | Install adapter   |
| `remove <name>`  | Remove adapter    |
| `<name> config`  | Configure adapter |

### Examples

```bash
kindling adapter list
kindling adapter install opencode
kindling adapter opencode config --capsule my-project
```

---

**Next:** [Configuration reference →](/kindling/reference/config)
