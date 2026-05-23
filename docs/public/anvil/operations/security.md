---
id: security
title: Security Model
description: Understanding anvil's security considerations and best practices.
sidebar_position: 2
---

# Security Model

anvil is a security-conscious tool. This page covers its security model and best
practices.

:::info Trust boundary in v0.6.0-beta

The Anvil daemon and driver framework run a **same-UID, local-IPC trust model**
in `v0.6.0-beta`. Owner-only Unix domain sockets (`0700` parent directory,
`0600` socket file) and an owner-only Windows named pipe with remote clients
rejected enforce that boundary at the transport layer. There is no remote /
cross-UID surface, no TLS, and no signed manifests in v1. For the four HIGH
trade-offs the release council surfaced inside that boundary — allowlist
file-mode verification, unsalted SHA-256 redaction hash, spec-only §4.4
redaction filter for non-`validate_write` MCP tools, and the Linux PID-reuse
TOCTOU window / macOS fence-on-uncertainty interrupt ladder — see the
[v0.6.0-beta security note](https://github.com/eddacraft/anvil-001/blob/main/docs/archive/runbooks/v0.6.0-beta-security-note.md).

:::

## Threat Model

### What anvil Protects Against

| Threat                    | How anvil helps             |
| ------------------------- | --------------------------- |
| Accidental secret commits | Pattern + entropy detection |
| Architecture violations   | Import boundary enforcement |
| Quality erosion           | Anti-pattern detection      |
| Audit gaps                | Evidence trail              |
| AI drift                  | Real-time validation        |

### What anvil Does NOT Protect Against

| Threat                   | Why Not                                |
| ------------------------ | -------------------------------------- |
| Malicious insiders       | anvil is a tool, not access control    |
| Zero-day vulnerabilities | anvil validates patterns, not exploits |
| Supply chain attacks     | Use npm audit, Snyk, etc.              |
| Runtime attacks          | anvil is static analysis only          |

## Secret Detection

### How It Works

anvil uses two detection methods:

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

1. **Use `.env` files carefully** — gitignore real secret-bearing files, and
   expect Anvil to scan `.env`, `.envrc`, and `.env.*` content when those files
   are present
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

### Planned Evidence Storage

Dedicated evidence export commands and remote evidence storage are planned, not
part of the current public CLI.

For compliance in the current beta:

- Store CI logs and JSON gate output in your existing artefact system
- Restrict access with your CI provider's permissions
- Keep retention aligned with your organisation's normal build-log policy

### Access Control

Control access to captured Anvil output via:

- File permissions for local artefacts
- CI artefact permissions
- Your organisation's normal RBAC and retention controls

## Configuration Security

### Sensitive Config

Never put secrets in `.anvilrc`:

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

anvil warns on unexplained suppressions:

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
.anvilrc @security-team
```

### Track Suppressions

Search for `@anvil-ignore` comments and review any exported suppression reports
your team keeps for governance:

```bash
grep -rn "@anvil-ignore" src/
```

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

CI output may be visible. anvil:

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

:::caution Planned

Evidence export commands are planned for a future release. For now, copy the
`.anvil/evidence/` directory directly for compliance archival.

:::

---

**Next:** [Troubleshooting →](/anvil/operations/troubleshooting)
