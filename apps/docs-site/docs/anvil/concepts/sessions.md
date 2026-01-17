---
id: sessions
title: Sessions and Runs
description: Understanding Anvil's execution model and artefact management.
sidebar_position: 3
---

# Sessions and Runs

Anvil organises work into **sessions** (bounded development periods) and **runs** (individual validation executions).

## Sessions

A session is a bounded period of development work with a specific goal.

### What Defines a Session?

- **Start** — `anvil session start` or first change in watch mode
- **End** — `anvil session end` or explicit commit
- **Scope** — typically one feature, bug fix, or task

### Session Lifecycle

```
┌──────────────────────────────────────────────────┐
│                    Session                        │
│                                                   │
│  Start ──▶ Run ──▶ Run ──▶ Run ──▶ End          │
│              │       │       │                   │
│              ▼       ▼       ▼                   │
│           Evidence Evidence Evidence             │
│                                                   │
└──────────────────────────────────────────────────┘
```

### Session Context

Sessions track:

- **Files modified** — what changed during the session
- **Runs executed** — all validation runs
- **Evidence generated** — audit trail
- **Active task** — if working within a plan

### Commands

```bash
# Start a new session
anvil session start --task AUTH-001

# View current session
anvil session status

# End session (generates summary)
anvil session end
```

## Runs

A run is a single execution of Anvil validation.

### When Runs Happen

- **Watch mode** — automatic on file save
- **Manual** — `anvil run`
- **CI** — as part of pipeline

### Run Output

Each run produces:

```
Run ID: run_abc123
Timestamp: 2024-01-15T10:30:00Z
Files checked: 3
Duration: 245ms

Results:
  ✓ architecture (23ms)
  ✓ anti-patterns (12ms)
  ✓ secrets (8ms)

Status: PASS
```

### Run Modes

| Mode | Trigger | Output |
|------|---------|--------|
| Watch | File save | Inline terminal |
| Interactive | `anvil run` | Full terminal UI |
| CI | `anvil run --ci` | Machine-readable JSON |

## Artefacts

Runs produce artefacts—files and data for later reference.

### Types of Artefacts

| Artefact | Purpose |
|----------|---------|
| Evidence | Immutable validation record |
| Reports | Human-readable summaries |
| Coverage | Code coverage data |
| Snapshots | Pre-change state (for rollback) |

### Artefact Storage

```
.anvil/
├── sessions/
│   └── session_abc123/
│       ├── evidence/
│       │   ├── run_001.json
│       │   └── run_002.json
│       └── summary.json
└── cache/
    └── file_hashes.json
```

### Accessing Artefacts

```bash
# List sessions
anvil session list

# View session details
anvil session show session_abc123

# Export evidence
anvil evidence export session_abc123 --format json
```

## Evidence

Evidence is the immutable record of what was validated.

### Evidence Structure

```json
{
  "id": "evidence_xyz789",
  "run_id": "run_abc123",
  "session_id": "session_abc123",
  "timestamp": "2024-01-15T10:30:00Z",
  "inputs": {
    "files": ["src/auth/login.ts"],
    "config_hash": "sha256:config123"
  },
  "checks": [
    {
      "name": "architecture",
      "status": "pass",
      "duration_ms": 23
    }
  ],
  "signature": "sha256:result456"
}
```

### Evidence Properties

- **Immutable** — cannot be modified after creation
- **Signed** — cryptographic hash prevents tampering
- **Linked** — references inputs and configuration
- **Timestamped** — precise creation time

### Using Evidence

Evidence enables:

- **Audit** — prove what was checked and when
- **Compliance** — demonstrate validation occurred
- **Debugging** — understand why something passed/failed
- **Reproducibility** — re-run with same inputs

---

**Next:** [Audit trail and trust model →](/docs/anvil/concepts/audit-trail)
