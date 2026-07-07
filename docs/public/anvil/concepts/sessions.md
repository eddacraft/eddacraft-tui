---
id: sessions
title: Sessions and Runs
description: Understanding anvil's execution model and artefact management.
sidebar_position: 2
---

# Sessions and Runs

:::caution Planned feature

Session and run tracking commands (`anvil session start`, `anvil session end`,
etc.) are planned for a future release. The conceptual model described here
reflects the intended design. Currently, anvil tracks runs internally but does
not expose session management via the CLI.

:::

anvil organises work into **sessions** (bounded development periods) and
**runs** (individual validation executions).

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

### Planned Commands

These commands describe the intended session-management surface and are not yet
available in the public CLI.

```bash
# Start a new session
anvil session start --task AUTH-001

# View current session
anvil session status

# End session (generates summary)
anvil session end
```

## Runs

A run is a single execution of anvil validation.

### When Runs Happen

- **Watch mode** — automatic on file save
- **Manual** — `anvil check --all`
- **CI** — as part of pipeline

### Run Output

Each run produces:

```
Run ID: run_abc123
Timestamp: 2024-01-15T10:30:00Z
Files checked: 3
Duration: 245ms

Results:
  ✓ import-boundaries (23ms)
  ✓ antipattern-scan (12ms)
  ✓ secret-detection (8ms)

Status: PASS
```

The run output uses the canonical check names accepted by
`anvil gate --only-checks` and `.anvilrc#checks`. Legacy aliases such as
`architecture` and `secret` may still parse, but public examples use
`import-boundaries` and `secret-detection`.

### Run Modes

| Mode        | Trigger                   | Output                |
| ----------- | ------------------------- | --------------------- |
| Watch       | File save                 | Inline terminal       |
| Interactive | `anvil check --all`       | Full terminal UI      |
| CI          | `anvil gate --profile ci` | Structured exit codes |

## Artefacts

Runs produce artefacts—files and data for later reference.

### Types of Artefacts

| Artefact  | Purpose                         |
| --------- | ------------------------------- |
| Evidence  | Immutable validation record     |
| Reports   | Human-readable summaries        |
| Coverage  | Code coverage data              |
| Snapshots | Pre-change state (for rollback) |

### Planned Artefact Storage

The intended session artefact layout is:

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

### Planned Access Commands

These commands are part of the intended evidence/session surface and are not yet
available in the public CLI.

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
      "name": "import-boundaries",
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

**Next:** [Audit trail and trust model →](/anvil/concepts/audit-trail)
