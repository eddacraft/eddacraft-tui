# Security Analyst — Vulnerabilities, Credential Handling, Input Validation

**Reviewer:** security-analyst
**Scope:** f0d34257..HEAD (7 commits, RCLI-015a/015b/022/023/030)
**Verdict:** PASS with findings

## Findings

### CRITICAL-SA-001: None found
No critical security vulnerabilities identified.

### MAJOR-SA-001: Credential file written without atomic rename
- **File:** `crates/anvil-cli/src/auth/credentials.rs:144-153, 182-189`
- **Category:** credential security
- **Severity:** major
- **Detail:** Both `migrate_to_xdg` and `save` write credentials by opening the
  target file directly and writing content. If the process is interrupted
  mid-write (SIGKILL, power loss), the credential file could be left in a
  partially-written state containing a truncated JSON blob. This is both a
  reliability issue (credentials lost) and a potential security concern (partial
  token could be logged in error messages if parsing fails). **Recommendation:**
  Write to a temporary file in the same directory, then `rename()` atomically.
  This is the standard pattern for credential files.

### MINOR-SA-002: File permissions set correctly (0o600)
- **File:** `crates/anvil-cli/src/auth/credentials.rs:148, 186`
- **Category:** credential security
- **Severity:** positive (no issue)
- **Detail:** Both `migrate_to_xdg` and `save` correctly set mode 0o600 on Unix,
  restricting read/write to the file owner. The test at line 455-468 verifies
  this. Good security practice.

### MINOR-SA-003: HTTPS enforcement on API URL is correct
- **File:** `crates/anvil-cli/src/auth/mod.rs:18-27`
- **Category:** transport security
- **Severity:** positive (no issue)
- **Detail:** `api_url()` correctly rejects non-HTTPS URLs unless they target
  localhost. The localhost exception covers `localhost`, `127.0.0.1`, and `[::1]`.
  This is a well-implemented security control.

### MINOR-SA-004: Env var credential not persisted to disk
- **File:** `crates/anvil-cli/src/auth/credentials.rs:112-123`
- **Category:** credential security
- **Severity:** positive (no issue)
- **Detail:** `ANVIL_LICENSE` env var credentials are correctly returned without
  calling `migrate_to_xdg`, ensuring ephemeral CI tokens are never written to
  disk. Test at line 348-356 verifies this invariant. Excellent.

### MINOR-SA-005: Token logged in error context
- **File:** `crates/anvil-cli/src/auth/credentials.rs:75-77`
- **Category:** information disclosure
- **Severity:** minor
- **Detail:** If JSON parsing fails at line 77, the anyhow context includes the
  file path but the error message from serde_json may include a snippet of the
  file content (e.g. "expected value at line 1 column 5" with partial content).
  This is generally safe since the file is local, but in CI environments where
  stderr is captured to logs, a malformed credential file could leak partial
  token content in error output. Low risk.

### MINOR-SA-006: Secret scan reads file content into memory
- **File:** `crates/anvil-cli/src/commands/gate.rs:216`
- **Category:** resource consumption
- **Severity:** minor
- **Detail:** `std::fs::read_to_string(path)` reads entire files. A large binary
  file with a text extension (e.g. a `.json` fixture containing megabytes of
  data) would be fully read into memory. Consider capping file size (e.g. skip
  files > 1MB) or reading line-by-line with BufReader.

### MINOR-SA-007: Non-Unix credential write has no permission restriction
- **File:** `crates/anvil-cli/src/auth/credentials.rs:156-159, 197-199`
- **Category:** credential security
- **Severity:** minor
- **Detail:** On non-Unix platforms (Windows), `std::fs::write` is used without
  setting restrictive ACLs. The file will inherit the directory's default
  permissions, which may be world-readable. For a beta targeting Linux/macOS
  this is acceptable, but should be addressed before Windows support.

## Summary

Credential handling is well-designed with correct permission setting, HTTPS
enforcement, and env-var-only credential isolation. The main recommendation is
adopting atomic file writes for credential persistence (write-to-temp + rename).
No critical vulnerabilities found.
