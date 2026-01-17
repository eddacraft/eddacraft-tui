---
id: storage
title: Storage
description: How Kindling stores observations.
sidebar_position: 3
---

# Storage

Kindling uses simple, portable storage by default.

## SQLite Storage

Each capsule is a SQLite database:

```
~/.kindling/
├── capsules/
│   ├── project-alpha.db
│   ├── auth-refactor.db
│   └── default.db
└── config.json
```

### Why SQLite?

- **No server required** — just files
- **Portable** — copy/move databases freely
- **Reliable** — battle-tested, ACID compliant
- **Queryable** — SQL when you need it
- **Compact** — efficient storage

### Database Schema

```sql
CREATE TABLE observations (
  id TEXT PRIMARY KEY,
  content TEXT NOT NULL,
  kind TEXT NOT NULL,
  tags TEXT,  -- JSON array
  source TEXT,  -- JSON object
  context TEXT,  -- JSON object
  created_at TEXT NOT NULL,
  updated_at TEXT
);

CREATE INDEX idx_observations_kind ON observations(kind);
CREATE INDEX idx_observations_created ON observations(created_at);

-- Full-text search
CREATE VIRTUAL TABLE observations_fts USING fts5(
  content,
  tags,
  content='observations'
);
```

## File Locations

### Global Data

```
~/.kindling/
├── capsules/           # Capsule databases
├── config.json         # Global configuration
└── cache/              # Search index cache
```

### Project-Local Data

```
project/
├── .kindling/
│   └── local.db        # Project capsule
└── ...
```

### Custom Location

Configure in `config.json`:

```json
{
  "dataDir": "/path/to/kindling/data"
}
```

Or via environment:

```bash
export KINDLING_DATA_DIR=/path/to/data
```

## Backup

### Manual Backup

```bash
# Backup single capsule
cp ~/.kindling/capsules/my-project.db ~/backups/

# Backup all
cp -r ~/.kindling ~/backups/kindling-$(date +%Y%m%d)
```

### Export Backup

```bash
kindling export --all --format json > kindling-backup.json
```

### Restore

```bash
kindling import kindling-backup.json
```

## Migration

### Between Machines

```bash
# On source machine
cp ~/.kindling/capsules/my-project.db /transfer/

# On target machine
cp /transfer/my-project.db ~/.kindling/capsules/
kindling capsule list  # Verify
```

### Version Upgrades

Kindling handles schema migrations automatically:

```bash
kindling capsule migrate my-project
```

## Data Integrity

### Verification

```bash
kindling capsule verify my-project
```

Output:
```
Verifying my-project...
  ✓ Database integrity
  ✓ Schema version: 2
  ✓ Observation count: 42
  ✓ FTS index synced

Capsule is healthy.
```

### Repair

If corruption occurs:

```bash
kindling capsule repair my-project
```

## Size Management

### Check Size

```bash
kindling capsule size my-project
```

```
my-project: 2.4 MB
  Observations: 42
  Average size: 57 KB
```

### Compact

Remove deleted records:

```bash
kindling capsule compact my-project
```

### Prune Old Data

```bash
# Remove observations older than 1 year
kindling capsule prune my-project --older-than 365d
```

## Alternative Storage

### JSON Files

For simplicity or version control:

```json
{
  "storage": {
    "type": "json",
    "path": ".kindling/observations.json"
  }
}
```

### Remote Storage (Future)

Planned support for:
- S3/GCS buckets
- PostgreSQL
- Custom backends

---

**Next:** [Retrieval philosophy →](/docs/kindling/concepts/retrieval)
