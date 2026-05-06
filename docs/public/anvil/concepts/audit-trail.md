---
id: audit-trail
title: Audit Trail
description: Understanding anvil's provenance and trust model.
sidebar_position: 3
---

# Audit Trail

:::caution Planned feature

The `anvil evidence` CLI commands described on this page are planned for a
future release. Evidence is currently generated internally during validation
runs but is not yet exposed via dedicated CLI commands.

:::

Anvil's validation pipeline is designed around auditable records. Dedicated
evidence commands are not yet public, so this page explains the model and shows
the planned operator surface.

## Why Audit Trails?

When AI generates code, you need to answer:

- **What was validated?** — which checks ran on which files
- **When was it validated?** — timestamps for compliance
- **What was the result?** — pass, fail, warnings
- **Can I reproduce it?** — same inputs, same outputs

anvil's audit trail answers all of these.

## Provenance Model

Provenance tracks the origin and lineage of every artefact:

```
┌─────────────────────────────────────────────────────┐
│                     Evidence                         │
├─────────────────────────────────────────────────────┤
│ What: run_abc123 validated src/auth/login.ts        │
│ When: 2024-01-15T10:30:00Z                          │
│ Who: developer@example.com                          │
│ How: anvil check (watch mode)                       │
│ Config: sha256:config123                            │
│ Result: PASS (all 4 checks)                         │
│ Signature: sha256:evidence789                       │
└─────────────────────────────────────────────────────┘
```

### Provenance Fields

| Field           | Description                         |
| --------------- | ----------------------------------- |
| `subject`       | What was validated (files, commits) |
| `timestamp`     | When validation occurred            |
| `actor`         | Who triggered validation            |
| `method`        | How validation was triggered        |
| `configuration` | Config hash at time of validation   |
| `result`        | Validation outcome                  |
| `signature`     | Cryptographic integrity hash        |

## Trust Model

anvil's trust model is based on **verifiable claims**:

### 1. Configuration is Versioned

Every run records the configuration hash:

```json
{
  "config_version": "1.0",
  "config_hash": "sha256:abc123",
  "gates_enabled": ["architecture", "anti-patterns", "secrets"]
}
```

If configuration changes, the hash changes. You can prove what rules were
active.

### 2. Inputs are Recorded

Every run records input file hashes:

```json
{
  "inputs": [
    { "path": "src/auth/login.ts", "hash": "sha256:file123" },
    { "path": "src/auth/types.ts", "hash": "sha256:file456" }
  ]
}
```

### 3. Results are Signed

Evidence is cryptographically signed:

```json
{
  "evidence_hash": "sha256:evidence789",
  "signature_algorithm": "sha256",
  "signed_at": "2024-01-15T10:30:00Z"
}
```

Tampering with evidence changes the hash, invalidating the signature.

### 4. Chain of Custody

Sessions link runs together:

```
Session: session_abc123
├── Run 1: run_001 (10:30:00) → PASS
├── Run 2: run_002 (10:35:00) → FAIL (fixed)
├── Run 3: run_003 (10:36:00) → PASS
└── Commit: abc123def456
```

## Planned Evidence Commands

The examples below show the intended command shape. They are not available in
the current public CLI.

### List Recent Evidence

```bash
anvil evidence list --since 7d
```

Output:

```
ID              RUN         TIMESTAMP              STATUS  FILES
evidence_001    run_abc123  2024-01-15T10:30:00Z   PASS    3
evidence_002    run_def456  2024-01-15T11:00:00Z   FAIL    1
evidence_003    run_ghi789  2024-01-15T11:05:00Z   PASS    1
```

### View Evidence Details

```bash
anvil evidence show evidence_001
```

### Verify Evidence Integrity

```bash
anvil evidence verify evidence_001
```

Output:

```
Verifying evidence_001...
  ✓ Hash matches content
  ✓ Inputs recorded
  ✓ Configuration referenced
  ✓ Signature valid

Evidence is valid.
```

### Export for Compliance

```bash
anvil evidence export --session session_abc123 --format json > audit.json
```

## Integration with Git

Today, use a normal Git trailer if your team wants to record a CI artefact or
validation report ID manually:

```bash
# Include evidence reference in commit
git commit -m "feat: add login endpoint" \
  --trailer "anvil-evidence: evidence_001"
```

Anvil's own commit helper (`anvil commit`) is planned for a future release.

The evidence ID in the commit trailer links to the full audit record.

## Retention and Storage

### Planned Default Storage

The planned local evidence store is `.anvil/evidence/`:

```
.anvil/
└── evidence/
    ├── 2024-01/
    │   ├── evidence_001.json
    │   └── evidence_002.json
    └── 2024-02/
        └── evidence_003.json
```

### Retention Policy

Planned retention configuration:

```json
{
  "evidence": {
    "retention": {
      "duration": "90d",
      "keep_failures": true,
      "compress_after": "7d"
    }
  }
}
```

### Remote Storage

For compliance, a future release is expected to support remote evidence storage:

```json
{
  "evidence": {
    "remote": {
      "type": "s3",
      "bucket": "company-anvil-evidence"
    }
  }
}
```

---

**Next:** [Solo dev workflow →](/anvil/guides/solo-dev-flow)
