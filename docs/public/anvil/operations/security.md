---
id: security
title: Security Model
description: Understanding anvil's security considerations and best practices.
sidebar_position: 2
---

# Security Model

anvil is a security-conscious tool. This page covers its security model and best
practices.

:::info Local trust boundary

The Rust daemon and MCP server use a **same-UID, local-IPC trust model**.
Owner-only Unix domain sockets (`0700` parent directory, `0600` socket file) and
an owner-only Windows named pipe with remote clients rejected enforce the
transport boundary. There is no public network listener. Workspace confinement
can narrow a daemon further to explicitly admitted repository roots.

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
| Supply chain attacks     | Use a dedicated dependency scanner     |
| Runtime attacks          | anvil is static analysis only          |

## Secret Detection

### How It Works

anvil uses two detection methods:

1. **Pattern matching** — regex for known secret formats
2. **Entropy analysis** — Shannon entropy for random strings

### Built-in Patterns

The current Rust registry has 21 built-in patterns. It recognises generic API
keys, secrets, passwords, private keys, database URLs, and credit-card-shaped
values, plus provider-specific AWS, GitHub, Slack, Stripe, Google, Heroku,
SendGrid, Twilio, npm, Anthropic, and OpenAI key formats.

### Entropy Threshold

High-entropy strings (random-looking) trigger alerts:

- Threshold: 4.5 bits/character (configurable)
- Example: `sk_live_51H...` → entropy ~4.8 → flagged

### Limitations

- **False positives** — test data, example configs
- **False negatives** — low-entropy passwords, encoded secrets
- **Git history** — source scanning covers the working tree, not every
  historical blob

### Best Practices

1. **Use `.env` files carefully** — gitignore real secret-bearing files, and
   expect Anvil to scan `.env`, `.envrc`, and `.env.*` content when those files
   are present
2. **Use secret managers** — Vault, AWS Secrets, etc.
3. **Review alerts** — don't just suppress
4. **Check history separately** — use your established secret-history scanner;
   `anvil audit-chain` checks witness coverage, not historical secret content

## Evidence Security

### Integrity

The witness chain binds governance events to commits. Review capsules package a
bounded commit range with a manifest and collected digests;
`anvil capsule verify` checks the closed-state package and re-collects
repository digests when the source repository is present.

Anvil does not silently create a remote evidence store.

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

- Run `anvil audit-chain` on the commit range your control covers
- Create and verify a review capsule for the reviewed range
- Store the resulting JSON or capsule in your existing controlled artefact
  system
- Apply the retention and access policy required by your organisation

### Multi-Tenant

For platforms serving multiple customers:

- Isolate evidence per tenant
- Configure boundaries per project
- Use separate configs per environment
- Confine the intercept daemon to admitted workspace roots

By default the intercept daemon adopts each repository on first touch (open
mode). On a shared or multi-tenant machine, switch it to allow-list confinement
so it only serves roots you explicitly admit:

```bash
anvil workspace mode allowlist
anvil workspace allow /srv/tenant-a --prefix
```

This is admission control on top of the same-UID transport boundary, not a
replacement for it — confinement narrows _which_ workspaces a same-UID daemon
will serve. See [Workspace confinement](./config.md#workspace-confinement).

### Compliance Reporting

Audit witness coverage and create a portable review package with the shipped
Rust commands:

```bash
anvil audit-chain --json
anvil capsule create --range <base>..<head> --out ../review-capsule
anvil capsule verify --json ../review-capsule
```

Store the audit output or capsule in your organisation's existing artefact
system, with its normal access and retention policy. See
[Audit Trail](../concepts/audit-trail.md) for the evidence boundaries and exit
semantics.

---

**Next:** [Troubleshooting →](/anvil/operations/troubleshooting)
