# Operations Reviewer — Deploy, Rollback, Monitoring

**Reviewer:** operations-reviewer
**Scope:** f0d34257..HEAD (7 commits, RCLI-015a/015b/022/023/030)
**Verdict:** PASS

## Findings

### MINOR-OP-001: No deprecation warning for legacy credential paths
- **File:** `crates/anvil-cli/src/auth/credentials.rs:161-164`
- **Category:** migration UX
- **Severity:** minor
- **Detail:** When credentials are migrated from legacy paths, the code prints
  "Migrated credentials -> {path}" but does not advise the user to remove the
  old files. Over time, stale credentials in `~/.anvil/auth.json` or
  `~/.anvil/license` could confuse users or create a security concern. Consider
  adding: "You can safely remove the old credential file at {legacy_path}."

### MINOR-OP-002: Archival preserves full git history but no .gitkeep
- **Files:** `archive/anvil-cli-node/`, `archive/anvil-tui-ink/`
- **Category:** operational clarity
- **Severity:** minor
- **Detail:** The archive directories contain the full source trees. There is no
  README or marker indicating these are archived and should not be modified.
  A simple `archive/README.md` stating "These directories contain archived
  Node.js implementations. Do not modify." would prevent confusion.

### MINOR-OP-003: Exit codes are well-defined but undocumented externally
- **File:** `crates/anvil-cli/src/main.rs:11-15`
- **Category:** observability
- **Severity:** minor
- **Detail:** Five exit codes are defined (0-4) which is excellent for CI
  integration. These should be documented in the CLI's help output or README
  so CI pipeline authors can branch on specific codes (e.g. exit 2 = gate
  failure vs exit 3 = auth required).

### MINOR-OP-004: nx.json archive exclusion is correct
- **File:** `nx.json`
- **Category:** positive
- **Severity:** positive (no issue)
- **Detail:** All three Nx plugins (`@nx/js/typescript`, `@nx/eslint/plugin`,
  `@nx/vite/plugin`) correctly exclude `archive/**`. This ensures archived
  code does not affect build times, type checking, or test runs.

### MINOR-OP-005: pnpm-lock.yaml shrinks by ~1000 lines
- **File:** `pnpm-lock.yaml`
- **Category:** positive
- **Severity:** positive (no issue)
- **Detail:** The archival removes the Node.js CLI's dependencies from the
  lockfile, reducing install times and attack surface. Good operational hygiene.

## Rollback Assessment

Rollback is straightforward: the archival is a rename operation, and all Rust
code is additive. `git revert` of the archival commit would restore the Node.js
CLI to its original location. The Rust CLI additions are independent and do not
modify existing Rust crate APIs.

## Summary

Clean operational posture. The archival is well-executed with proper build
system updates. Exit codes are defined for CI integration. Minor suggestions
around migration UX and documentation.
