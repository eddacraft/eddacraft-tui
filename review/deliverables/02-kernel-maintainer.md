# Kernel Maintainer — Simplicity, Correctness, API Design

**Reviewer:** kernel-maintainer
**Scope:** f0d34257..HEAD (7 commits, RCLI-015a/015b/022/023/030)
**Verdict:** PASS with one major and minor findings

## Findings

### MAJOR-KM-001: gate command calls std::process::exit() instead of returning error
- **File:** `crates/anvil-cli/src/commands/gate.rs:759`
- **Category:** API design / correctness
- **Severity:** major
- **Detail:** `run()` calls `std::process::exit(EXIT_GATE_FAIL)` directly when
  gate checks fail. This bypasses the structured error handling in `main()` and
  prevents cleanup (destructors, terminal teardown). The `main()` function already
  has infrastructure to convert errors to exit codes. `run()` should return an
  error or a typed result that `main()` can map to the appropriate exit code.
  This is the only command that calls `process::exit` directly.

### MINOR-KM-002: anvil-architecture validate() is YAML-parse-only
- **File:** `crates/anvil-architecture/src/lib.rs:26-47`
- **Category:** API completeness
- **Severity:** minor
- **Detail:** The `validate()` function only checks YAML parseability and always
  returns `valid: true` for parseable files. The `ValidationResult` struct has a
  `violations` field that is never populated. The doc comment explains this is
  intentional for beta, which is fine, but the architecture gate check
  (`gate.rs:417-451`) presents this as full validation. Consider making the skip
  explicit in the gate check message (e.g. "Architecture YAML is parseable
  (boundary checking deferred)").

### MINOR-KM-003: workspace_root() called multiple times per gate run
- **File:** `crates/anvil-cli/src/commands/gate.rs:88-99, 101, 133, 175`
- **Category:** simplicity
- **Severity:** minor
- **Detail:** `workspace_root()` shells out to `git rev-parse --show-toplevel`
  and is called independently by `run_check_lint`, `run_check_test`,
  `run_check_secret`, and indirectly via `run_single_check`. It should be
  resolved once and threaded through as a parameter.

### MINOR-KM-004: OutputMode::Tui falls through to Plain in gate command
- **File:** `crates/anvil-cli/src/commands/gate.rs:725-726`
- **Category:** API design
- **Severity:** minor
- **Detail:** `OutputMode::Plain | OutputMode::Tui` are combined in the match arm
  with a comment "TUI surface for gate is not yet implemented". This is acceptable
  for now but should be tracked -- the welcome command already has a TUI gate
  surface (`collect_gate_data` + `GateState`), so the standalone `anvil gate`
  command could use it too.

## Summary

The API design is generally clean. The major finding (process::exit bypassing
main's error handling) should be addressed before merge. The architecture crate
is a reasonable beta stub. OutputMode integration is well-designed with a clear
priority chain.
