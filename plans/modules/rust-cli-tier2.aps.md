<!--
APS Module: Rust CLI Tier 2
=========================
Port Tier 2 (utility & operational) commands to crates/anvil-cli/.
Depends on RCLI (Tier 1) foundation being complete.

Scopes: RCLI2 (main)
-->

# Rust CLI — Tier 2

| ID    | Owner | Status   |
| ----- | ----- | -------- |
| RCLI2 | —     | Proposed |

## Purpose

Port Tier 2 utility and operational commands from the Node.js CLI
(`apps/anvil-cli/`) to the Rust binary (`crates/anvil-cli/`). These commands
extend the core workflow shipped in RCLI Tier 1 with single-file checking,
plan validation, drift tracking, gate configuration, and policy tooling.

**Why:** Tier 1 covers the primary workflow loop (init → watch → gate → status)
but omits operational commands needed for CI pipelines (`check`, `pr-comment`),
incremental debugging (`policy-debug`, `drift compare`), and configuration
management (`gate-config`, `validate`). Without these, users must fall back to
the Node.js CLI for common tasks, defeating the single-binary goal.

**ADR:** [012-rust-cli-replacement](../decisions/012-rust-cli-replacement.md)
**Spec:** [2026-03-18-rust-cli-design](../specs/2026-03-18-rust-cli-design.md) §6 Tier 2

## In Scope

- 8 commands (or command groups) classified as Tier 2 in the design spec
- Integration with existing Rust crates: anvil-kernel (gate/check), anvil-policy
  (policy-debug, policy-watch), anvil-architecture (drift)
- JSON and plain-text output modes for all commands
- Kindling provenance integration where applicable (check, gate-config)

## Out of Scope

- Tier 3 commands (subsystems — see rust-cli-tier3.aps.md)
- New features not present in the Node.js CLI
- TUI surfaces (Tier 2 commands are headless; no new Ratatui surfaces)
- OPA binary distribution (uses system-installed OPA or bundled Wasm — decided
  in OPAE module)

## Interfaces

**Depends on:**

- RCLI — Foundation crate structure, clap entry point, output formatters,
  auth middleware
- KERN — Kernel watcher and event emission (for policy-watch)
- OPAE — Policy commands policy-debug, policy-watch, pr-comment, exception
  require OPAE work items to be implemented in TypeScript first (or ported
  from spec directly). Items marked with ‡ below depend on OPAE completion

**Exposes:**

- 8 additional subcommands on the `anvil` binary
- Drift snapshot storage compatible with Node.js CLI format

## Constraints

- Output parity with Node.js CLI (same JSON schema, same exit codes)
- Drift snapshots interchangeable between Rust and Node.js CLIs during
  transition
- Commands that depend on unimplemented OPAE items are deferred until the
  domain logic exists (in Rust or TypeScript)
- Same error handling conventions as RCLI: `anyhow` for application code,
  `thiserror` for library errors

## Ready Checklist

Change status to **Ready** when:

- [x] RCLI Tier 1 foundation complete (Phases 1–4)
- [ ] RCLI-017 (anvil-policy crate) has evaluator implemented
- [ ] RCLI-019 (anvil-architecture crate) has validation logic implemented
- [ ] OPAE status reviewed — identify which Tier 2 commands can proceed

---

## Tasks

#### Phase 1 — Check & Validate
### RCLI2-001: check command

- **Status:** Proposed
- **Intent:** Port `anvil check` (planless file analysis). Supports file
  selection modes: explicit paths, `--all`, `--changed`, `--staged`,
  `--since <ref>`. Runs GateRunner in planless mode. Supports interactive
  review with nudge coaching, suppression filtering, and caching
- **Expected Outcome:** `anvil check --changed` analyses modified files and
  reports warnings with severity levels; `--json` produces machine-readable
  output; `--interactive` prompts per-warning
- **Validation:** Warning counts and severities match Node.js CLI for same
  project state; exit code 0 on clean, 1 on warnings
- **Files:** `crates/anvil-cli/src/commands/check.rs`
- **Confidence:** medium (664 LOC in Node.js; interactive mode needs crossterm
  prompts)
- **Priority:** High
- **Dependencies:** RCLI (foundation), KERN (gate runner)

---

### RCLI2-002: validate command

- **Status:** Proposed
- **Intent:** Port `anvil validate <plan>`. Auto-detects plan format (APS,
  SpecKit, BMAD), validates structure, verifies content hash integrity.
  Supports `--format` override and `--no-validate-hash`
- **Expected Outcome:** `anvil validate plan.aps.md` reports validation issues
  with line numbers; shows detected format and confidence
- **Validation:** Validation results match Node.js CLI for same plan files;
  hash verification produces identical pass/fail
- **Files:** `crates/anvil-cli/src/commands/validate.rs`
- **Confidence:** high (192 LOC in Node.js; straightforward file parsing)
- **Priority:** Medium
- **Dependencies:** RCLI (foundation)

---

## Phase 2 — Drift & Gate Config

### RCLI2-003: drift command

- **Status:** Proposed
- **Intent:** Port `anvil drift` with four subcommands: `snapshot` (capture
  baseline), `compare` (diff two snapshots), `report` (longitudinal analysis),
  `list` (enumerate snapshots). Reads/writes `.anvil/snapshots/` directory.
  Supports `--json` output for all subcommands
- **Expected Outcome:** `anvil drift snapshot --name release-1.0` captures
  current state; `anvil drift compare s1 s2` shows deltas in violations,
  antipatterns, and suppressions
- **Validation:** Snapshot JSON format is identical to Node.js CLI; comparison
  metrics (net_change, trend, violation deltas) match for same input
- **Files:** `crates/anvil-cli/src/commands/drift.rs`
- **Confidence:** medium (324 LOC in Node.js; requires SnapshotCaptureService
  port or Rust equivalent)
- **Priority:** High
- **Dependencies:** RCLI (foundation), RCLI-019 (anvil-architecture crate)

---

### RCLI2-004: gate-config command

- **Status:** Proposed
- **Intent:** Port `anvil gate-config`. List, enable, disable, and
  interactively configure gate checks and thresholds. Reads/writes
  `.anvil/gate-config.json`
- **Expected Outcome:** `anvil gate-config --list` shows current config;
  `--enable policy` enables a check; `--interactive` walks through all
  settings
- **Validation:** Config file format is identical to Node.js CLI; enable/disable
  produces same JSON mutations
- **Files:** `crates/anvil-cli/src/commands/gate_config.rs`
- **Confidence:** high (131 LOC in Node.js; straightforward JSON CRUD)
- **Priority:** Medium
- **Dependencies:** RCLI (foundation)

---

## Phase 3 — Policy Utilities

### RCLI2-005: policy-debug command ‡

- **Status:** Proposed
- **Intent:** Port `anvil policy-debug`. Interactive policy debugging with
  step-through evaluation, variable inspection, and rule tracing. Requires
  OPA integration
- **Expected Outcome:** `anvil policy-debug <rule>` shows evaluation trace
  with variable bindings at each step
- **Validation:** Debug trace matches OPA's native `--explain` output format
- **Files:** `crates/anvil-cli/src/commands/policy_debug.rs`
- **Confidence:** low (not yet implemented in Node.js; depends on OPAE-013)
- **Priority:** Medium
- **Dependencies:** RCLI-017 (anvil-policy crate), OPAE-013
- **Notes:** ‡ Blocked on OPAE implementation. Can proceed if OPAE-013 lands
  in TypeScript and a clean interface exists, or if ported directly from spec

---

### RCLI2-006: policy-watch command ‡

- **Status:** Proposed
- **Intent:** Port `anvil policy-watch`. Watches policy files (`.rego`,
  `.yaml`) for changes and re-evaluates on save. Uses kernel file watcher
  with policy-specific event handling
- **Expected Outcome:** `anvil policy-watch` monitors policy directory and
  reports validation errors in real time
- **Validation:** File change triggers re-evaluation within 500ms; errors
  reported correctly
- **Files:** `crates/anvil-cli/src/commands/policy_watch.rs`
- **Confidence:** low (not yet implemented in Node.js; depends on OPAE-015)
- **Priority:** Low
- **Dependencies:** RCLI-017, KERN (watcher), OPAE-015

---

## Phase 4 — CI Integration

### RCLI2-007: pr-comment command ‡

- **Status:** Proposed
- **Intent:** Port `anvil pr-comment`. Generates PR annotations from gate
  results. Formats warnings as GitHub PR review comments with file/line
  context, severity badges, and summary table
- **Expected Outcome:** `anvil pr-comment --gate-output results.json` posts
  annotations to the current PR via GitHub API
- **Validation:** Annotations appear on correct lines in PR; summary table
  counts match gate results
- **Files:** `crates/anvil-cli/src/commands/pr_comment.rs`
- **Confidence:** low (not yet implemented in Node.js; depends on OPAE-028,
  OPAE-029)
- **Priority:** Medium
- **Dependencies:** RCLI (foundation), OPAE-028, OPAE-029

---

### RCLI2-008: exception command ‡

- **Status:** Proposed
- **Intent:** Port `anvil exception`. Manage policy exceptions — create,
  list, approve, revoke. Supports time-bounded exceptions with approval
  workflows
- **Expected Outcome:** `anvil exception create --rule ARCH-001 --reason "..."
  --expires 30d` creates a scoped exception; `anvil exception list` shows
  active exceptions
- **Validation:** Exception storage format matches Node.js CLI; expired
  exceptions are correctly filtered
- **Files:** `crates/anvil-cli/src/commands/exception.rs`
- **Confidence:** low (not yet implemented in Node.js; depends on OPAE-027)
- **Priority:** Low
- **Dependencies:** RCLI (foundation), OPAE-027

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| OPAE items not implemented (‡ commands) | High | Medium | Defer ‡ commands; ship Tier 2 without them; they join Tier 2 when OPAE lands |
| SnapshotCaptureService port complexity | Medium | Medium | Share analysis logic with gate command; avoid duplicating check infrastructure |
| Interactive mode (check --interactive) | Low | Low | Use crossterm raw mode with simple Y/N/S prompts; no full TUI needed |
| Drift snapshot format divergence | Low | Medium | Write format compatibility test; snapshot header includes schema version |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Check & Validate | 2 | Proposed |
| 2 — Drift & Gate Config | 2 | Proposed |
| 3 — Policy Utilities | 2 | Proposed (blocked on OPAE) |
| 4 — CI Integration | 2 | Proposed (blocked on OPAE) |
| **Total** | **8** | — |
