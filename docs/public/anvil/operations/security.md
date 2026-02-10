---
id: security
title: Security Model
description: Understanding Anvil's security considerations and best practices.
sidebar_position: 2
---

# Security Model

Anvil is a security-conscious tool. This page covers its security model and best
practices.

## Threat Model

### What Anvil Protects Against

| Threat                    | How Anvil Helps             |
| ------------------------- | --------------------------- |
| Accidental secret commits | Pattern + entropy detection |
| Architecture violations   | Import boundary enforcement |
| Quality erosion           | Anti-pattern detection      |
| Audit gaps                | Evidence trail              |
| AI drift                  | Real-time validation        |

### What Anvil Does NOT Protect Against

| Threat                   | Why Not                                |
| ------------------------ | -------------------------------------- |
| Malicious insiders       | Anvil is a tool, not access control    |
| Zero-day vulnerabilities | Anvil validates patterns, not exploits |
| Supply chain attacks     | Use npm audit, Snyk, etc.              |
| Runtime attacks          | Anvil is static analysis only          |

## Secret Detection

### How It Works

Anvil uses two detection methods:

1. **Pattern matching** — regex for known secret formats
2. **Entropy analysis** — Shannon entropy for random strings

### Built-in Patterns

```
api[_-]?key
secret[_-]?key
password
token
credential
private[_-]?key
bearer
auth
```

### Entropy Threshold

High-entropy strings (random-looking) trigger alerts:

- Threshold: 4.5 bits/character (configurable)
- Example: `sk_live_51H...` → entropy ~4.8 → flagged

### Limitations

- **False positives** — test data, example configs
- **False negatives** — low-entropy passwords, encoded secrets
- **Git history** — optional scan, not default

### Best Practices

1. **Use `.env` files** — gitignored, not scanned
2. **Use secret managers** — Vault, AWS Secrets, etc.
3. **Review alerts** — don't just suppress
4. **Check history** — enable `checkGitHistory` for initial audit

## Evidence Security

### Integrity

Evidence is cryptographically signed:

```json
{
  "evidence_hash": "sha256:abc123...",
  "signed_at": "2024-01-15T10:30:00Z"
}
```

Tampering changes the hash, invalidating the evidence.

### Storage

Default: local `.anvil/evidence/`

For compliance:

- Use remote storage (S3, GCS)
- Enable encryption at rest
- Configure retention policies

```json
{
  "evidence": {
    "remote": {
      "type": "s3",
      "bucket": "company-anvil-evidence",
      "encryption": "AES256"
    }
  }
}
```

### Access Control

Evidence files are read-only after creation. Control access via:

- File permissions (local)
- IAM policies (remote)
- RBAC (enterprise)

## Configuration Security

### Sensitive Config

Never put secrets in `anvil.config.json`:

```json
// ❌ Wrong
{
  "remote": {
    "apiKey": "sk_live_..."
  }
}

// ✓ Correct - use environment
{
  "remote": {
    "apiKey": "${ANVIL_API_KEY}"
  }
}
```

### Config Versioning

Version config in git for auditability:

- Who changed what rules
- When rules were added/removed
- Why (via commit messages)

## Suppression Governance

Suppressions bypass checks—govern them carefully.

### Require Explanations

Anvil warns on unexplained suppressions:

```typescript
// ⚠️ Warning: suppression without explanation
// @anvil-ignore AP-003

// ✓ Acceptable
// @anvil-ignore AP-003 Third-party API returns untyped data
```

### Review Suppressions

Use CODEOWNERS:

```
# .github/CODEOWNERS
anvil.config.json @security-team
```

### Track Suppressions

List all suppressions:

```bash
anvil suppressions list
```

Review periodically—are they still needed?

## CI Security

### GitHub Token Scope

Minimum required scopes:

- `contents: read` — read code
- `checks: write` — create check runs
- `pull-requests: write` — comment on PRs

### Secrets in CI

Use GitHub Secrets, not hardcoded values:

```yaml
env:
  ANVIL_API_KEY: ${{ secrets.ANVIL_API_KEY }}
```

### Output Sanitisation

CI output may be visible. Anvil:

- Never logs detected secrets
- Redacts sensitive file contents
- Uses generic error messages

## Enterprise Considerations

### Audit Requirements

For SOC 2, ISO 27001, etc.:

- Enable evidence collection
- Configure remote storage
- Set retention to required period
- Export evidence on demand

### Multi-Tenant

For platforms serving multiple customers:

- Isolate evidence per tenant
- Configure boundaries per project
- Use separate configs per environment

### Compliance Reporting

Generate compliance reports:

```bash
anvil evidence export \
  --since 30d \
  --format compliance \
  --output audit-report.json
```

---

**Next:** [Troubleshooting →](/anvil/operations/troubleshooting)
