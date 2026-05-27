# Code Reviewer — Code Quality, Patterns, Naming

**Reviewer:** code-reviewer
**Scope:** f0d34257..HEAD (7 commits, RCLI-015a/015b/022/023/030)
**Verdict:** PASS with minor findings

## Findings

### MINOR-CR-001: Duplicated file-write logic in credentials.rs
- **File:** `crates/anvil-cli/src/auth/credentials.rs:130-166` and `168-202`
- **Category:** DRY violation
- **Severity:** minor
- **Detail:** `migrate_to_xdg()` and `save()` both contain nearly identical
  platform-gated file-write logic (OpenOptions with mode 0o600 on Unix,
  plain write on non-Unix). Extract a shared `write_credentials_file(path, content)`
  helper to avoid drift between the two code paths.

### MINOR-CR-002: Redundant set_permissions in save()
- **File:** `crates/anvil-cli/src/auth/credentials.rs:192`
- **Category:** redundancy
- **Severity:** minor
- **Detail:** `save()` opens the file with `.mode(0o600)` then immediately calls
  `std::fs::set_permissions` with the same mode. The `mode()` on OpenOptions
  already sets permissions at creation time. The extra `set_permissions` call is
  redundant (though harmless). `migrate_to_xdg()` correctly omits it.

### MINOR-CR-003: `create_first_run_marker` takes `&PathBuf` not `&Path`
- **File:** `crates/anvil-cli/src/commands/welcome.rs:169`
- **Category:** idiomatic Rust
- **Severity:** minor
- **Detail:** Function signature is `fn create_first_run_marker(path: &PathBuf)`.
  Idiomatic Rust prefers `&Path` as the parameter type since `PathBuf` auto-derefs
  to `Path`. This is a Clippy lint (`clippy::ptr_arg`).

### MINOR-CR-004: Magic string "default" in collect_gate_data
- **File:** `crates/anvil-cli/src/commands/gate.rs:624`
- **Category:** naming
- **Severity:** minor
- **Detail:** `plan_id: "default".to_string()` is a magic string. Consider a
  constant or deriving the plan ID from the actual plan argument.

### MINOR-CR-005: Inconsistent error output channel
- **File:** `crates/anvil-cli/src/main.rs:165,176,208`
- **Category:** consistency
- **Severity:** minor
- **Detail:** JSON error output uses `eprintln!` (stderr) in all three locations,
  which is correct. However, the clap error path (line 167) calls `err.print()`
  which may write to stdout for help/version messages. This is standard clap
  behaviour but worth documenting for CI consumers that expect all errors on stderr.

## Summary

Code quality is solid overall. The credential module is well-structured with
clear separation of concerns, testable pure functions, and good test coverage.
The OutputMode enum is clean and well-tested. No critical or major issues found.
