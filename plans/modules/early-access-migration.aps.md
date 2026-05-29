<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Early Access Migration

| ID | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| EAMIG | —     | Medium   | In Progress |

**Last reviewed:** 2026-05-26

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

## Work Items

### Phase 1 — Kernel Types (slice 1)

#### EAMIG-001 — NodeId newtype for SymbolNode.id

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Introduce `pub struct NodeId(pub u64)` to prevent type confusion
  between node IDs, sequence numbers, and other u64 values. Change
  `SymbolNode.id`, `SymbolEdge.from`, and `SymbolEdge.to` to `NodeId`. Wire
  format is identical (serde transparent).
- **Files:** `crates/anvil-kernel-types/src/graph.rs`

#### EAMIG-002 — Enforce EventType/EventPayload consistency

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** Either remove `EventType` as a separate field (payload variant
  encodes the type) or add a `From<&EventPayload> for EventType` impl and
  enforce consistency via a constructor. Currently callers can construct
  mismatched events.
- **Files:** `crates/anvil-kernel-types/src/events.rs`

---

### Phase 2 — Checks (slice 2)

#### EAMIG-003 — Surface invalid custom pattern errors in secret scanning

- **Status:** Done
- **Priority:** High
- **Confidence:** High
- **Intent:** `compile_secret_patterns` silently drops invalid custom regex
  patterns. Return errors so misconfigured patterns are visible to the user.
- **Files:** `crates/anvil-checks/src/secret/patterns.rs`,
  `crates/anvil-checks/src/secret/check.rs`,
  `crates/anvil-checks/src/secret/types.rs`
- **Notes:** `compile_custom_patterns` now returns
  `(Vec<CompiledPattern>, Vec<String>)`. `SecretCheckResult` gained a
  `pattern_errors: Vec<String>` field (`#[serde(default)]`, wire-compatible).
  Regression test `compile_custom_patterns_separates_invalid_from_valid`
  locks the behaviour.

#### EAMIG-004 — Expand git scanner file extension coverage

- **Status:** Merged 2026-05-26 via PR #1994
- **Priority:** High
- **Confidence:** High
- **Intent:** Git history scanning covers only JS/TS/JSON/YAML/env — far
  narrower than on-disk scanning. Expand to match the working-tree scan
  extensions or remove the glob filter entirely.
- **Files:** `crates/anvil-checks/src/secret/git_scanner.rs`

#### EAMIG-005 — Credit card pattern false-positive mitigation

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** Current pattern matches any 16-digit number. Add contextual
  anchoring (require a label like `card:`, `cc:`) or make the pattern opt-in
  rather than default.
- **Files:** `crates/anvil-checks/src/secret/patterns.rs`

#### EAMIG-006 — Dedup key collision on same-line multi-match

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** `deduplicate_findings` uses `file:line:type:pattern_name` as the
  key, collapsing distinct matches on the same line. Include column offset or
  redacted match in the key.
- **Files:** `crates/anvil-checks/src/secret/check.rs`

#### EAMIG-007 — Pre-compile command safety arg patterns

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `match_args` compiles rule arg-pattern regex on every call. Pre-
  compile when rules are loaded and store alongside the `CommandRule`.
- **Files:** `crates/anvil-checks/src/command_safety/matcher.rs`

#### EAMIG-008 — Expand rm-rf-home pattern to absolute paths

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `rm-rf-home` rule only matches `~` and `$HOME`, missing
  `/home/user` and `/Users/user` paths.
- **Files:** `crates/anvil-checks/src/command_safety/rules/filesystem_rules.rs`

#### EAMIG-009 — Empty catch regex multiline support

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** AP-006 (empty catch) regex is single-line only. Misses catch
  blocks with multiline comments.
- **Files:** `crates/anvil-checks/src/antipattern/patterns.rs`

---

### Phase 3 — Policy (slice 3)

#### EAMIG-010 — Distinguish OPA error from empty evaluation result

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** `evaluate()` conflates OPA execution errors and empty results via
  the `success` flag. Add `execution_error: Option<String>` to `OpaResult` to
  differentiate the two states.
- **Files:** `crates/anvil-policy/src/opa.rs`, `crates/anvil-policy/src/evaluator.rs`

#### EAMIG-011 — Restrict load_bundle visibility

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `load_bundle()` is public with no workspace-root boundary
  enforcement. Either make it `pub(crate)` or add a workspace_root validation
  parameter.
- **Files:** `crates/anvil-policy/src/bundle.rs`

#### EAMIG-012 — Remove dead _find_opa_binary and which dependency

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `_find_opa_binary()` is dead code. Remove it and the `which`
  dependency from Cargo.toml, or wire it up in `OpaExecutor::new()`.
- **Files:** `crates/anvil-policy/src/opa.rs`, `crates/anvil-policy/Cargo.toml`

#### EAMIG-013 — Validate exception glob patterns on add

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `glob_matches()` silently returns false for invalid patterns.
  Validate in `ExceptionStore::add()` and return an error on invalid syntax.
- **Files:** `crates/anvil-policy/src/exceptions.rs`

#### EAMIG-014 — Fix fingerprint hash collision

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `compute_fingerprint` concatenates field bytes without separators,
  causing collisions across different (rule, policy) pairs. Insert null byte
  separator between fields.
- **Files:** `crates/anvil-policy/src/opa.rs`

#### EAMIG-015 — Normalise bundle error handling in list_bundles

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** Parse errors are warned and skipped but I/O errors abort the
  listing. Both should be treated the same (skip and warn, or collect all).
- **Files:** `crates/anvil-policy/src/bundle.rs`

#### EAMIG-016 — Use OsStr for OPA paths instead of to_string_lossy

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `run_tests()` converts policy_dir via `to_string_lossy()`,
  mangling non-UTF-8 paths. Use `Command::arg()` with `&Path` directly.
- **Files:** `crates/anvil-policy/src/opa.rs`

---

### Phase 4 — Architecture (slice 4)

#### EAMIG-017 — Migrate from deprecated serde_yaml 0.9

- **Status:** Ready
- **Priority:** High
- **Confidence:** Medium
- **Intent:** `serde_yaml` 0.9 is deprecated and unmaintained with known panic
  vectors. Migrate to a maintained alternative (e.g., `serde_yml` community
  fork, or `figment` with YAML support).
- **Files:** `crates/anvil-architecture/Cargo.toml`, all YAML parsing code
- **Dependencies:** Evaluate impact on anvil-policy (also uses serde_yaml)

#### EAMIG-018 — Merge_with_template additive layer merge

- **Status:** Draft
- **Priority:** Medium
- **Confidence:** Low
- **Intent:** Currently all-or-nothing: if user defines one layer, all template
  defaults are discarded. Consider additive merge where template layers fill
  gaps. Design decision needed.
- **Files:** `crates/anvil-architecture/src/yaml_parser.rs`

#### EAMIG-019 — Monorepo template meaningful layer separation

- **Status:** Draft
- **Priority:** Medium
- **Confidence:** Low
- **Intent:** The monorepo template puts all apps/packages/libs in one layer,
  defeating cross-app boundary enforcement. Split into meaningful layers.
- **Files:** `crates/anvil-architecture/src/yaml_parser.rs`

#### EAMIG-020 — Auto-update baseline updated_at on save

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `save_baseline` does not stamp `updated_at`. Either update it
  automatically or document that callers must do so.
- **Files:** `crates/anvil-architecture/src/baseline.rs`

---

### Phase 5 — RCLI Deferred Items

#### EAMIG-021 — Deduplicate ANVIL_DIR constant (RCLI-047)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** 141 occurrences of `.anvil` across Rust crates with 5 separate
  constant definitions. Extract to a shared constants module.
- **Files:** Multiple crates

#### EAMIG-022 — Deduplicate file-tree walks in gate command (RCLI-053)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** 3 independent walkdir traversals in gate.rs; walks 2 and 3 are
  near-identical. Consolidate into single walk, saving ~300-500ms per run.
- **Files:** `crates/anvil-cli/src/commands/gate.rs`

#### EAMIG-023 — Preserve underlying error in evaluate_auth (RCLI-041)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** `device_flow.rs` discards OTP response with `let _`, losing
  server feedback. Capture and surface the reason.
- **Files:** `crates/anvil-cli/src/auth/device_flow.rs`

#### EAMIG-024 — Improve secret scan robustness (RCLI-040)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** Various robustness improvements to the secret scanning pipeline
  identified during the parity rework council.
- **Files:** `crates/anvil-checks/src/secret/`

#### EAMIG-025 — Document exit codes for CI consumers (RCLI-042)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Exit codes 0/1/2/3/4 exist but are not documented in a user-
  facing location. Add to CLI help and docs.
- **Files:** `crates/anvil-cli/src/main.rs`, docs

#### EAMIG-026 — Deprecation notice for old credential files (RCLI-043)

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** Emit a notice when reading credentials from legacy paths
  (`~/.anvil/auth.json`, `~/.anvil/license`).
- **Files:** `crates/anvil-cli/src/auth/credentials.rs`

#### EAMIG-027 — Restrict credential permissions on non-Unix (RCLI-044)

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** Credential file permissions are only enforced on Unix. Add
  Windows ACL restriction or document the limitation.
- **Files:** `crates/anvil-cli/src/auth/credentials.rs`

#### EAMIG-028 — Deduplicate credential file-write logic (RCLI-037)

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** Multiple credential-write paths share duplicated logic.
  Consolidate.
- **Files:** `crates/anvil-cli/src/auth/`

---

### Phase 6 — Kernel (slice 5)

#### EAMIG-029 — Use StableGraph for symbol graph removals

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** `remove_file` swap-index repair loop is not provably correct for
  3+ interleaved removals. Switch to `StableGraph` or `retain_nodes` so
  `NodeIndex` values are never invalidated on removal.
- **Files:** `crates/anvil-kernel/src/graph/symbol_graph.rs`

#### EAMIG-030 — Fix import edge sourcing for multi-symbol files

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** `update_file` always sources import edges from the first symbol
  in a file. Multi-export files have incomplete dependency edges. Associate
  edges with a per-file representative node or the Module node.
- **Files:** `crates/anvil-kernel/src/graph/incremental.rs`

#### EAMIG-031 — Surface GraphDelta errors to emitter

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** `GraphDelta.errors` from failed symbol insertions are silently
  dropped. Emit `ErrorCode::Internal` events for each error.
- **Files:** `crates/anvil-kernel/src/watch.rs`

#### EAMIG-032 — Key previously_public/privileged on (file, name)

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `PublicApiExpansion` and `PrivilegeExpansion` invariants key on
  bare symbol name, causing false-negative suppression for common names
  across files. Key on `(file, name)` pair.
- **Files:** `crates/anvil-kernel/src/graph/incremental.rs`,
  `crates/anvil-kernel/src/policy/invariants/privilege_expansion.rs`

#### EAMIG-033 — Evaluate cross-layer invariants on reverse edges

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** `CrossLayerViolation` only examines the changed file's delta.
  Files that import the changed file are not re-evaluated, missing violations
  when `infra/db.ts` changes but `domain/user.ts` imports it.
- **Files:** `crates/anvil-kernel/src/watch.rs`

#### EAMIG-034 — Use filter_entry for initial_scan directory pruning

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `initial_scan` in `watch.rs` does not prune ignored directories
  at the walk level (unlike `embedded.rs`), so it descends into
  `node_modules` and `target`.
- **Files:** `crates/anvil-kernel/src/watch.rs`

#### EAMIG-035 — Monotonic next_id counter instead of max recomputation

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `next_id` recomputes `max(all node IDs) + 1` per file (O(n) per
  file). Replace with a monotonic counter incremented by symbol count.
- **Files:** `crates/anvil-kernel/src/watch.rs`, `crates/anvil-kernel/src/embedded.rs`

#### EAMIG-036 — Replace hand-rolled now_iso8601 with time crate

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** Custom Gregorian calendar algorithm in `emitter.rs` may have
  off-by-one errors near century-year boundaries. Use `time` or `chrono`.
- **Files:** `crates/anvil-kernel/src/protocol/emitter.rs`

#### EAMIG-037 — Consolidate duplicate POOL_INIT rayon statics

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** Two `POOL_INIT: Once` statics in `watch.rs` and `embedded.rs`
  both call `build_global()`. Extract to a shared `crate::pool` module.
- **Files:** `crates/anvil-kernel/src/watch.rs`, `crates/anvil-kernel/src/embedded.rs`

#### EAMIG-038 — Remove or gate unimplemented WatchConfig patterns

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** `include_patterns` and `exclude_patterns` on `WatchConfig` are
  documented as not consumed. Either remove from the public API or emit a
  warning when non-empty.
- **Files:** `crates/anvil-kernel/src/watch.rs`

---

### Phase 7 — TUI (slice 6)

#### EAMIG-039 — Fix LogPanel next_match/prev_match index semantics

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** `next_match`/`prev_match` treat `selected_index` as a match-list
  index while render treats it as a filtered-entries index. These diverge when
  the match set is a strict subset of filtered entries.
- **Files:** `archive/eddacraft-tui-local/src/widgets/log_panel.rs` [archived local copy from before `eddacraft-tui` was extracted to a published crate (workspace dep `eddacraft-tui = "0.1.0"`); re-target before working — the live widget set in `crates/anvil-tui/src/widgets/` is Anvil-specific]

#### EAMIG-040 — Remove render-time filter.search overwrite in LogPanel

- **Status:** Done
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `LogPanel::render` overwrites `filter.search` with
  `search_input` every frame, making external writes to `filter.search`
  ineffective.
- **Files:** `archive/eddacraft-tui-local/src/widgets/log_panel.rs` [archived local copy of the LogPanel implementation; the live `eddacraft-tui` is a published workspace dep, not a local crate]

---

### Phase 8 — Distribution (slice 11)

#### EAMIG-041 — Add checksum verification to install.sh

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** `install.sh` downloads and executes the cargo-dist installer
  without any checksum or signature verification. Either add SHA256
  verification before execution, or redirect users to the cargo-dist
  installer directly which has its own verification.
- **Files:** `install.sh`

#### EAMIG-042 — Scope ANVIL_RELEASES_TOKEN to step-level env

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** The PAT is exposed as a job-level env variable to all steps in
  the host job. Move to step-level env for only the steps that need it.
  Confirm the token is a fine-grained PAT scoped to only eddacraft/anvil
  and eddacraft/homebrew-tap.
- **Files:** `.github/workflows/release.yml`

#### EAMIG-043 — Add artefact attestation to release workflow

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Release binaries have no provenance attestation. Add
  `actions/attest-build-provenance` to generate SLSA provenance for free.
- **Files:** `.github/workflows/release.yml`

#### EAMIG-044 — Add tag protection rules

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Any contributor with push access can trigger a release by
  creating a version tag. Configure GitHub tag protection rules.
- **Files:** Repository settings (not code)

#### EAMIG-045 — Verify cargo-dist bootstrap integrity

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** cargo-dist is installed by piping curl to sh with no checksum.
  Add a verification step after install.
- **Files:** `.github/workflows/release.yml`

---

### Phase 9 — Bench (slice 12)

#### EAMIG-046 — Add publish = false to anvil-bench

- **Status:** Done
- **Priority:** High
- **Confidence:** High
- **Intent:** Missing `publish = false` means the dev-only crate could be
  accidentally published. Its deps (tempfile, rand) would ship as production
  dependencies.
- **Files:** `crates/anvil-bench/Cargo.toml`
- **Notes:** Already shipped — `publish = false` is present in
  `crates/anvil-bench/Cargo.toml:7`. APS entry was stale.

#### EAMIG-047 — Fix graph_memory per-step measurement

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** Graphs are dropped between steps so RSS deltas are unreliable.
  Either accumulate graphs or annotate metrics as net-of-dealloc.
- **Files:** `crates/anvil-bench/src/scenarios/graph_memory.rs`

#### EAMIG-048 — Fix cold_start warm-cache measurement

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** Measures discovery after freshly generating the repo (warm
  cache). Rename metric or drop caches before measurement.
- **Files:** `crates/anvil-bench/src/scenarios/cold_start_scaling.rs`

#### EAMIG-049 — Fix watcher_saturation settle_time double-count

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Settle time is added to the reported duration, double-counting
  overhead in the wall-clock metric.
- **Files:** `crates/anvil-bench/src/scenarios/watcher_saturation.rs`

#### EAMIG-050 — Rename policy_scaling violations to matches

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** The metric counts rule-symbol matches, not violations. The
  naming is inverted compared to policy engine semantics.
- **Files:** `crates/anvil-bench/src/scenarios/policy_scaling.rs`

#### EAMIG-051 — Stream git-history secret scan output

- **Status:** Proposed
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** `scan_git_history` buffers the entire `git log -p` output into
  memory via `Command::output()` before parsing. EAMIG-004 broadened the
  pathspec from a 6-extension allowlist to "all files except `skip_extensions`",
  increasing that buffered volume; on a large monorepo at high
  `git_history_depth` this is an OOM/latency risk (bounded today by the depth
  clamp of 1000 and `--diff-filter=AM`, so not urgent). Replace `.output()`
  with a streaming `BufReader` over `Stdio::piped()` stdout, processing lines as
  they arrive. While here, thread a typed `git_error: Option<String>` through
  `GitScanOutput` so a non-zero git exit is distinguishable from a clean history
  at the type level rather than only via a stderr warning.
- **Files:** `crates/anvil-checks/src/secret/git_scanner.rs`,
  `crates/anvil-checks/src/secret/check.rs`
- **Source:** PR #1994 (EAMIG-004) council + Copilot review — operations and
  adversarial reviewers flagged the buffering; deferred as non-blocking.
