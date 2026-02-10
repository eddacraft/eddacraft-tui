---
id: config
title: Configuration
description: Kindling configuration reference.
sidebar_position: 3
---

# Configuration

Complete configuration reference for Kindling.

## Configuration File

Location: `~/.kindling/config.json`

```json
{
  "version": "1.0",
  "dataDir": "~/.kindling",
  "defaultCapsule": "default",
  "storage": {
    "type": "sqlite"
  },
  "search": {
    "defaultLimit": 20,
    "highlightMatches": true
  },
  "adapters": {},
  "aliases": {}
}
```

## Core Settings

### dataDir

Directory for Kindling data.

```json
{ "dataDir": "~/.kindling" }
```

Or use environment:

```bash
export KINDLING_DATA_DIR=/path/to/data
```

### defaultCapsule

Default capsule for operations.

```json
{ "defaultCapsule": "my-project" }
```

### version

Config schema version.

```json
{ "version": "1.0" }
```

## Storage Settings

### sqlite (default)

```json
{
  "storage": {
    "type": "sqlite",
    "options": {
      "journalMode": "wal",
      "synchronous": "normal"
    }
  }
}
```

### json

```json
{
  "storage": {
    "type": "json",
    "options": {
      "pretty": true
    }
  }
}
```

## Search Settings

```json
{
  "search": {
    "defaultLimit": 20,
    "highlightMatches": true,
    "fuzzyMatch": false,
    "caseSensitive": false
  }
}
```

| Option             | Type    | Default | Description           |
| ------------------ | ------- | ------- | --------------------- |
| `defaultLimit`     | number  | 20      | Default result limit  |
| `highlightMatches` | boolean | true    | Highlight matches     |
| `fuzzyMatch`       | boolean | false   | Enable fuzzy matching |
| `caseSensitive`    | boolean | false   | Case-sensitive search |

## Adapter Settings

Per-adapter configuration:

```json
{
  "adapters": {
    "opencode": {
      "enabled": true,
      "capsule": "coding-sessions",
      "capture": {
        "decisions": true,
        "discoveries": true
      }
    },
    "pocketflow": {
      "enabled": true,
      "defaultCapsule": "workflows"
    }
  }
}
```

## Aliases

Command shortcuts:

```json
{
  "aliases": {
    "/m": "memory",
    "/ma": "memory add",
    "/ms": "memory search",
    "obs": "observe",
    "s": "search"
  }
}
```

## Display Settings

```json
{
  "display": {
    "dateFormat": "relative",
    "colors": true,
    "maxContentLength": 200,
    "showTags": true,
    "showSource": true
  }
}
```

| Option             | Values                     | Description      |
| ------------------ | -------------------------- | ---------------- |
| `dateFormat`       | `relative`, `iso`, `local` | Timestamp format |
| `colors`           | boolean                    | ANSI colors      |
| `maxContentLength` | number                     | Truncate content |
| `showTags`         | boolean                    | Display tags     |
| `showSource`       | boolean                    | Display source   |

## Export Settings

```json
{
  "export": {
    "defaultFormat": "markdown",
    "includeMetadata": true,
    "dateFormat": "iso"
  }
}
```

## Environment Variables

| Variable                   | Description      |
| -------------------------- | ---------------- |
| `KINDLING_DATA_DIR`        | Data directory   |
| `KINDLING_CONFIG`          | Config file path |
| `KINDLING_DEFAULT_CAPSULE` | Default capsule  |
| `KINDLING_NO_COLOR`        | Disable colors   |
| `KINDLING_VERBOSE`         | Verbose output   |

## Project-Local Config

Create `.kindling/config.json` in project root:

```json
{
  "defaultCapsule": "project-local",
  "storage": {
    "type": "sqlite",
    "path": ".kindling/observations.db"
  }
}
```

This overrides global settings for the project.

## Config Validation

Validate configuration:

```bash
kindling config validate
```

Reset to defaults:

```bash
kindling config reset
```

---

**Back to:** [Kindling Overview →](/kindling/overview)
