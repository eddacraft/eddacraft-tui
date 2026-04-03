<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Early Access Migration

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| EAMIG | —     | Medium   | Ready  |

## Purpose

Track all outstanding and deferred council findings, design improvements, and
migration items identified during the v0.3.x release review councils. These are
not release blockers — the critical and should-fix items were resolved inline —
but they represent genuine improvements that should be addressed before GA.

## In Scope

- Deferred council findings from slices 1–4 review
- Design improvements flagged but not actioned during release
- Dependency migrations (serde_yaml, dead code removal)
- Documentation gaps in public APIs
- Items from RCLI Phase 9–11 that affect the shipped crates

## Out of Scope

- New features (covered by RCLI2, RCLI3)
- Test gaps (covered by EATEST)
- Distribution pipeline items (covered by DIST)

## Interfaces

**Depends on:** RCLI (Tier 1 complete), release slices 1–10 merged

**Exposes:** Cleaner crate APIs, better error messages, reduced dependency surface

---

## Phase 1 — Kernel Types (slice 1)

### EAMIG-001 — NodeId newtype for SymbolNode.id

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Introduce `pub struct NodeId(pub u64)` to prevent type confusion
  between node IDs, sequence numbers, and other u64 values. Change
  `SymbolNode.id`, `SymbolEdge.from`, and `SymbolEdge.to` to `NodeId`. Wire
  format is identical (serde transparent).
- **Files:** `crates/anvil-kernel-types/src/graph.rs`

### EAMIG-002 — Enforce EventType/EventPayload consistency

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** Either remove `EventType` as a separate field (payload variant
  encodes the type) or add a `From<&EventPayload> for EventType` impl and
  enforce consistency via a constructor. Currently callers can construct
  mismatched events.
- **Files:** `crates/anvil-kernel-types/src/events.rs`

---

## Phase 2 — Checks (slice 2)

### EAMIG-003 — Surface invalid custom pattern errors in secret scanning

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** `compile_secret_patterns` silently drops invalid custom regex
  patterns. Return errors so misconfigured patterns are visible to the user.
- **Files:** `crates/anvil-checks/src/secret/patterns.rs`

### EAMIG-004 — Expand git scanner file extension coverage

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** Git history scanning covers only JS/TS/JSON/YAML/env — far
  narrower than on-disk scanning. Expand to match the working-tree scan
  extensions or remove the glob filter entirely.
- **Files:** `crates/anvil-checks/src/secret/git_scanner.rs`

### EAMIG-005 — Credit card pattern false-positive mitigation

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** Current pattern matches any 16-digit number. Add contextual
  anchoring (require a label like `card:`, `cc:`) or make the pattern opt-in
  rather than default.
- **Files:** `crates/anvil-checks/src/secret/patterns.rs`

### EAMIG-006 — Dedup key collision on same-line multi-match

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** `deduplicate_findings` uses `file:line:type:pattern_name` as the
  key, collapsing distinct matches on the same line. Include column offset or
  redacted match in the key.
- **Files:** `crates/anvil-checks/src/secret/check.rs`

### EAMIG-007 — Pre-compile command safety arg patterns

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `match_args` compiles rule arg-pattern regex on every call. Pre-
  compile when rules are loaded and store alongside the `CommandRule`.
- **Files:** `crates/anvil-checks/src/command_safety/matcher.rs`

### EAMIG-008 — Expand rm-rf-home pattern to absolute paths

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `rm-rf-home` rule only matches `~` and `$HOME`, missing
  `/home/user` and `/Users/user` paths.
- **Files:** `crates/anvil-checks/src/command_safety/rules/filesystem_rules.rs`

### EAMIG-009 — Empty catch regex multiline support

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** AP-006 (empty catch) regex is single-line only. Misses catch
  blocks with multiline comments.
- **Files:** `crates/anvil-checks/src/antipattern/patterns.rs`

---

## Phase 3 — Policy (slice 3)

### EAMIG-010 — Distinguish OPA error from empty evaluation result

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** `evaluate()` conflates OPA execution errors and empty results via
  the `success` flag. Add `execution_error: Option<String>` to `OpaResult` to
  differentiate the two states.
- **Files:** `crates/anvil-policy/src/opa.rs`, `crates/anvil-policy/src/evaluator.rs`

### EAMIG-011 — Restrict load_bundle visibility

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `load_bundle()` is public with no workspace-root boundary
  enforcement. Either make it `pub(crate)` or add a workspace_root validation
  parameter.
- **Files:** `crates/anvil-policy/src/bundle.rs`

### EAMIG-012 — Remove dead _find_opa_binary and which dependency

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `_find_opa_binary()` is dead code. Remove it and the `which`
  dependency from Cargo.toml, or wire it up in `OpaExecutor::new()`.
- **Files:** `crates/anvil-policy/src/opa.rs`, `crates/anvil-policy/Cargo.toml`

### EAMIG-013 — Validate exception glob patterns on add

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `glob_matches()` silently returns false for invalid patterns.
  Validate in `ExceptionStore::add()` and return an error on invalid syntax.
- **Files:** `crates/anvil-policy/src/exceptions.rs`

### EAMIG-014 — Fix fingerprint hash collision

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `compute_fingerprint` concatenates field bytes without separators,
  causing collisions across different (rule, policy) pairs. Insert null byte
  separator between fields.
- **Files:** `crates/anvil-policy/src/opa.rs`

### EAMIG-015 — Normalise bundle error handling in list_bundles

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** Parse errors are warned and skipped but I/O errors abort the
  listing. Both should be treated the same (skip and warn, or collect all).
- **Files:** `crates/anvil-policy/src/bundle.rs`

### EAMIG-016 — Use OsStr for OPA paths instead of to_string_lossy

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `run_tests()` converts policy_dir via `to_string_lossy()`,
  mangling non-UTF-8 paths. Use `Command::arg()` with `&Path` directly.
- **Files:** `crates/anvil-policy/src/opa.rs`

---

## Phase 4 — Architecture (slice 4)

### EAMIG-017 — Migrate from deprecated serde_yaml 0.9

- **Status:** Ready
- **Priority:** High
- **Confidence:** Medium
- **Intent:** `serde_yaml` 0.9 is deprecated and unmaintained with known panic
  vectors. Migrate to a maintained alternative (e.g., `serde_yml` community
  fork, or `figment` with YAML support).
- **Files:** `crates/anvil-architecture/Cargo.toml`, all YAML parsing code
- **Dependencies:** Evaluate impact on anvil-policy (also uses serde_yaml)

### EAMIG-018 — Merge_with_template additive layer merge

- **Status:** Draft
- **Priority:** Medium
- **Confidence:** Low
- **Intent:** Currently all-or-nothing: if user defines one layer, all template
  defaults are discarded. Consider additive merge where template layers fill
  gaps. Design decision needed.
- **Files:** `crates/anvil-architecture/src/yaml_parser.rs`

### EAMIG-019 — Monorepo template meaningful layer separation

- **Status:** Draft
- **Priority:** Medium
- **Confidence:** Low
- **Intent:** The monorepo template puts all apps/packages/libs in one layer,
  defeating cross-app boundary enforcement. Split into meaningful layers.
- **Files:** `crates/anvil-architecture/src/yaml_parser.rs`

### EAMIG-020 — Auto-update baseline updated_at on save

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `save_baseline` does not stamp `updated_at`. Either update it
  automatically or document that callers must do so.
- **Files:** `crates/anvil-architecture/src/baseline.rs`

---

## Phase 5 — RCLI Deferred Items

### EAMIG-021 — Deduplicate ANVIL_DIR constant (RCLI-047)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** 141 occurrences of `.anvil` across Rust crates with 5 separate
  constant definitions. Extract to a shared constants module.
- **Files:** Multiple crates

### EAMIG-022 — Deduplicate file-tree walks in gate command (RCLI-053)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** 3 independent walkdir traversals in gate.rs; walks 2 and 3 are
  near-identical. Consolidate into single walk, saving ~300-500ms per run.
- **Files:** `crates/anvil-cli/src/commands/gate.rs`

### EAMIG-023 — Preserve underlying error in evaluate_auth (RCLI-041)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** `device_flow.rs` discards OTP response with `let _`, losing
  server feedback. Capture and surface the reason.
- **Files:** `crates/anvil-cli/src/auth/device_flow.rs`

### EAMIG-024 — Improve secret scan robustness (RCLI-040)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** Various robustness improvements to the secret scanning pipeline
  identified during the parity rework council.
- **Files:** `crates/anvil-checks/src/secret/`

### EAMIG-025 — Document exit codes for CI consumers (RCLI-042)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Exit codes 0/1/2/3/4 exist but are not documented in a user-
  facing location. Add to CLI help and docs.
- **Files:** `crates/anvil-cli/src/main.rs`, docs

### EAMIG-026 — Deprecation notice for old credential files (RCLI-043)

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** Emit a notice when reading credentials from legacy paths
  (`~/.anvil/auth.json`, `~/.anvil/license`).
- **Files:** `crates/anvil-cli/src/auth/credentials.rs`

### EAMIG-027 — Restrict credential permissions on non-Unix (RCLI-044)

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** Credential file permissions are only enforced on Unix. Add
  Windows ACL restriction or document the limitation.
- **Files:** `crates/anvil-cli/src/auth/credentials.rs`

### EAMIG-028 — Deduplicate credential file-write logic (RCLI-037)

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** Multiple credential-write paths share duplicated logic.
  Consolidate.
- **Files:** `crates/anvil-cli/src/auth/`
