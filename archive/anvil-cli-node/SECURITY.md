# Anvil CLI Security

## Token Storage

Anvil stores authentication tokens in `~/.anvil/auth.json` with file permissions
set to `0600` (owner read/write only). This is standard practice for CLI tools
that manage credentials locally, consistent with how `npm`, `gh`, and `gcloud`
handle token storage.

### Token Format

Tokens are opaque hex strings. They are never logged, echoed to stdout, or
included in error messages.

### File Permissions

| File                 | Permissions | Purpose          |
| -------------------- | ----------- | ---------------- |
| `~/.anvil/auth.json` | `0600`      | Auth credentials |

## Reporting Vulnerabilities

If you discover a security vulnerability, please report it privately via GitHub
Security Advisories on this repository. Do not open a public issue.
