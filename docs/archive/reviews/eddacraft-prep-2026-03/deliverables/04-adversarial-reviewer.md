# Adversarial Reviewer — Challenge Assumptions, Edge Cases, Security

**Reviewer:** adversarial-reviewer
**Scope:** f0d34257..HEAD (7 commits, RCLI-015a/015b/022/023/030)
**Verdict:** PASS with findings

## Findings

### MAJOR-AR-001: TOCTOU race in credential migration
- **File:** `crates/anvil-cli/src/auth/credentials.rs:73,82-89`
- **Category:** race condition
- **Severity:** major (low likelihood, high impact)
- **Detail:** `resolve_credentials` checks `xdg_path.exists()` at line 73, then
  if not found, reads a legacy path and calls `migrate_to_xdg` which writes to
  xdg_path. If two CLI processes start simultaneously (e.g. parallel CI jobs),
  both may find the XDG path missing, both read legacy, and both write to XDG.
  The last writer wins, which is benign here (same content), but the migration
  notice prints twice. More importantly, if the credential file is partially
  written by one process while the other reads it, the reader gets corrupt JSON.
  **Mitigation:** Use an advisory lock (`flock`) or atomic rename (write to
  temp file, then `rename`). The atomic rename pattern is simpler and sufficient.

### MINOR-AR-002: Secret scan patterns are incomplete
- **File:** `crates/anvil-cli/src/commands/gate.rs:178-185`
- **Category:** false sense of security
- **Severity:** minor
- **Detail:** Only 3 secret patterns are checked (AWS access key, OpenAI key,
  GitHub PAT). Missing patterns include: Slack tokens (`xoxb-`, `xoxp-`),
  Stripe keys (`sk_live_`), generic high-entropy strings, private keys
  (`-----BEGIN`). The check passes with "No hardcoded secrets found" which may
  give users false confidence. Consider renaming to "No common secret patterns
  found" or expanding the pattern set.

### MINOR-AR-003: walkdir traversal has no depth limit
- **File:** `crates/anvil-cli/src/commands/gate.rs:187`
- **Category:** edge case / DoS
- **Severity:** minor
- **Detail:** `WalkDir::new(&root)` with no `max_depth` will traverse the entire
  filesystem tree from the workspace root. In monorepos with deep node_modules
  trees or symlink loops (despite the ignore list), this could be very slow.
  Consider setting a reasonable depth limit (e.g. 20) or adding a timeout.

### MINOR-AR-004: evaluate_auth swallows the error message
- **File:** `crates/anvil-cli/src/main.rs:138-140`
- **Category:** edge case
- **Severity:** minor
- **Detail:** When `credentials::load()` returns `Err(e)`, `evaluate_auth`
  prints "Authentication required" but discards the actual error. If the error
  is "permission denied reading credentials file" or "corrupt JSON", the user
  gets no diagnostic information. At minimum, log the error in verbose mode.

### MINOR-AR-005: wants_json() scans all args including values
- **File:** `crates/anvil-cli/src/main.rs:156-157`
- **Category:** edge case
- **Severity:** minor
- **Detail:** `std::env::args().any(|a| a == "--json")` would match `--json`
  even if it appeared as a value argument (e.g. `anvil export --format --json`).
  This is unlikely but technically incorrect. The function is only used for
  pre-parse error formatting so the impact is cosmetic.

### MINOR-AR-006: first_run_marker_path is relative
- **File:** `crates/anvil-cli/src/commands/welcome.rs:165-167`
- **Category:** assumption
- **Severity:** minor
- **Detail:** `first_run_marker_path()` returns `PathBuf::from(".anvil/first-run")`
  which is relative to CWD, not the project root. If the user runs `anvil welcome`
  from a subdirectory, the marker is written to the wrong location. Should use
  `workspace_root().join(".anvil/first-run")` or similar.

## Summary

One major finding (TOCTOU in migration) that is low-likelihood in practice but
should be addressed with atomic file writes. The remaining findings are edge
cases and hardening opportunities.
