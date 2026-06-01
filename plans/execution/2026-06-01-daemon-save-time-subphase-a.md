# Daemon Save-time Validation — Sub-phase A Implementation Plan

**Goal:** Ship the frozen `validate_paths` wire + watch client + MCP re-point, backed by an interim per-`WorktreeKey` `SymbolGraph` cache (rebuild-on-restart, no persistence), behind the §8 correctness bar.
**Architecture:** Three new verdict-shaped JSON-RPC verbs on the existing intercept daemon reuse the `scan_buffer` handshake/transport/envelope. A per-path identity table classifies FS events; a bounded reverse-impact closure over the existing `DependencyGraph.reverse` index decides certified-vs-Stale; `run_antipattern_check` runs against the changed-path set over warm `SymbolGraph` state. `watch` and MCP become thin daemon clients with a scoped (never `--all`) fallback.
**Tech Stack:** Rust (anvil-intercept, anvil-intercept-proto, anvil-checks, anvil-kernel, anvil-cli), tokio, rayon, JSON-RPC 2.0 NDJSON, criterion (ipc_roundtrip).

Spec: [`plans/specs/2026-06-01-daemon-save-time-validation-contract.md`](../specs/2026-06-01-daemon-save-time-validation-contract.md) · ADR: [`plans/decisions/061-...md`](../decisions/061-save-time-daemon-delta-validation.md)

**Out of scope (do NOT implement here):** sub-phase A′ (GV2 hot-read slice — blocked on the GV2 hot-/non-hot-path boundary gate) and sub-phase B (GV2-021 persistence/warm-start). The backing in this plan is the interim `SymbolGraph` cache only.

---

## APS mapping

| Task group | APS item(s) |
|---|---|
| Wire + daemon verbs + backing | INTD `validate_paths` method; MLP2-067 (folded in as the interim backing) |
| MCP re-point | DRVR |
| Resource model + benches/SLO | RLB-002 (real default bench), RLB-008 (SLO + CI gate), RLB-005 (concurrent multi-process) |

Mark INTD/DRVR/MLP2-067 **In Progress** before starting; reconcile counts in `plans/index.aps.md` per APS rules. (APS status edits land in a *separate* reconciliation PR per the hot-file-collision rule — do not bundle them into code commits here.)

---

## File Map

| File | Create/Modify | Responsibility |
|---|---|---|
| `crates/anvil-intercept-proto/src/protocol.rs` | Modify | Method consts + `ValidatePathsRequest/Response`, `ChangeDescriptor`, `WorkspaceAssurance`, `StaleReason`, `Coverage`, `WorkspaceStatus*`, `RequestFullScan*` |
| `crates/anvil-intercept/src/path_safety.rs` | Create | `openat2` dirfd read-safety + lstat-ladder fallback; root-relative path normalisation + escape rejection |
| `crates/anvil-intercept/src/change_class.rs` | Create | Per-path `(inode,mtime,size)` table; `classify()` → canonical change-class; case-sensitivity probe; stat-on-validate drift |
| `crates/anvil-intercept/src/assurance.rs` | Create | Per-workspace assurance state machine + `StaleReason` taxonomy mapping (default-deny) |
| `crates/anvil-intercept/src/kernel_cache.rs` | Create | Per-`WorktreeKey` `SymbolGraph` cache (LRU + generation-guard + unregister-hook), delta application via `graph::incremental` |
| `crates/anvil-intercept/src/certify.rs` | Create | Bounded reverse-impact closure; `coverage` decision |
| `crates/anvil-intercept/src/validate_paths.rs` | Create | `validate_paths` orchestration: classify → certify → run check → coalesce → assurance |
| `crates/anvil-intercept/src/workspace_pool.rs` | Create | Two cooperating rayon pools + chunked-yield background scan; per-workspace in-flight admission token; DoS caps |
| `crates/anvil-intercept/src/confinement.rs` | Create | Admission mode (`open`/`allowlist`), operator-level allowlist load (fail-closed), `workspace-not-admitted` |
| `crates/anvil-intercept/src/ipc.rs` | Modify | Dispatch arms for the three verbs; auth (handshake + `validate_workspace_roots`, growable root set) |
| `crates/anvil-intercept/src/auth.rs` | Modify | Growable per-connection workspace-root set; drop any cwd path |
| `crates/anvil-cli/src/commands/watch.rs` | Modify | Default action routes to `validate_paths`; scoped fallback (never `--all`) inheriting read-safety |
| `crates/anvil-cli/src/commands/workspace.rs` | Create | `anvil workspace allow\|deny\|list\|mode` CLI |
| `crates/anvil-cli/src/mcp/tools/validate_write.rs` + `crates/anvil-cli/src/mcp/validation.rs` | Modify | `anvil_validate_write` re-point: in-process scan in `validation.rs` routes to daemon `validate_paths`, in-process fallback |
| `crates/anvil-cli/src/commands/status.rs` (or TUI surface) | Modify | Render assurance state incl `unavailable` + `confined: N` |
| `crates/anvil-intercept/benches/ipc_roundtrip.rs` | Modify | `validate_paths` warm-read latency + concurrency SLO case |
| `crates/anvil-intercept/tests/diagnostic_parity.rs` | Create | Order-normalised golden parity across the 4 paths |

> **MCP write-gate location (fact-checked):** the tool is `crates/anvil-cli/src/mcp/tools/validate_write.rs` (declared `mod.rs:10`, registered `tools/registry.rs:24`); the live in-process scan + timeout logic is in `crates/anvil-cli/src/mcp/validation.rs`. That `validation.rs` scan call is the Task-13 re-point site.

---

## Task 1: Freeze the wire types in the protocol crate

**Files:**
- Modify: `crates/anvil-intercept-proto/src/protocol.rs`
- Test: same file (`#[cfg(test)] mod tests`)

Add method constants beside `ANVIL_SCAN_BUFFER`:
```rust
pub const ANVIL_VALIDATE_PATHS: &str = "anvil/validate_paths";
pub const ANVIL_WORKSPACE_STATUS: &str = "anvil/workspace_status";
pub const ANVIL_REQUEST_FULL_SCAN: &str = "anvil/request_full_scan";
```
Add types (serde, `#[serde(rename_all = "snake_case")]`, all unknown-field-tolerant for forward-compat):
```rust
#[derive(Serialize, Deserialize, ...)]
#[serde(rename_all = "snake_case", tag = "change")]
pub enum ChangeKindWire {
    Created, Modified, Deleted,
    Renamed { from: String },          // root-relative, slash-normalised
}
pub struct ChangeDescriptor {
    pub path: String,
    #[serde(flatten)] pub change: ChangeKindWire,
    #[serde(skip_serializing_if = "Option::is_none")] pub content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub mtime: Option<i64>,
}
pub struct ValidatePathsRequest { pub workspace_root: String, pub paths: Vec<ChangeDescriptor> }
#[serde(rename_all = "snake_case")] pub enum Coverage { Certified, Partial }
#[serde(rename_all = "snake_case")] pub enum AssuranceState { Clean, Stale, Pending, Running, Unavailable }
#[serde(rename_all = "kebab-case")] pub enum StaleReason {
    CrossFileResolutionNeeded, Deleted, Renamed, SymlinkRetarget,
    ConfigBoundaryPolicyEdit, GitignoreScopeChange, ImpactSetOverflow,
    WarmStateEvicted, ScanTimeout, DaemonAbsent, UnknownClass,
}
pub struct WorkspaceAssurance {
    pub state: AssuranceState,
    #[serde(skip_serializing_if="Option::is_none")] pub reason: Option<StaleReason>,
    pub generation: u64,
    #[serde(skip_serializing_if="Option::is_none")] pub last_full_scan: Option<String>,
}
pub struct EvaluatedPath { pub path: String, pub content_hash: String }
pub struct ValidatePathsResponse {
    pub diagnostics: ScanDiagnostics,   // reuse the scan_buffer envelope type verbatim
    pub evaluated: Vec<EvaluatedPath>,
    pub workspace_assurance: WorkspaceAssurance,
    pub coverage: Coverage,
}
// WorkspaceStatusRequest/Response, RequestFullScanRequest/Response similarly.
```

- [ ] Write failing tests: `validate_paths_method_const`, `change_descriptor_roundtrip_all_variants`, `response_tolerates_unknown_additive_field` (MLP2-052 forward-compat style — deserialize a JSON with an extra field, assert Ok), `stale_reason_kebab_wire_strings`, `no_graph_version_field` (assert serialized `WorkspaceAssurance`/`WorkspaceStatus` JSON has no `graph_version` key).
- [ ] Run: `cargo test -p eddacraft-anvil-intercept-proto` — verify fail.
- [ ] Implement the types.
- [ ] Run: `cargo test -p eddacraft-anvil-intercept-proto` — verify pass.
- [ ] Commit: `feat(intercept-proto): freeze validate_paths verdict-shaped wire (ADR-061)`

## Task 2: Authorisation — growable per-connection workspace-root set; drop cwd

**Files:**
- Modify: `crates/anvil-intercept/src/auth.rs`, `crates/anvil-intercept/src/ipc.rs`
- Test: `crates/anvil-intercept/src/auth.rs` tests

Reuse `DriverManifest::validate_workspace_roots` against active sessions. Add a per-connection `BTreeSet<PathBuf>` of admitted (canonicalised) roots that grows on first contact (mode-gated by Task 14). Authorise a verb iff its `workspace_root` is in the set (or admissible under `open` mode). No `/proc/<pid>/cwd` anywhere.

- [ ] Failing tests: `validate_paths_authorised_for_session_root`, `validate_paths_refused_for_unrelated_root_in_allowlist_mode`, `root_set_grows_on_first_touch_in_open_mode`, `no_cwd_in_auth_path` (grep-guard test or absence assertion).
- [ ] Run `cargo test -p eddacraft-anvil-intercept auth` — fail → implement → pass.
- [ ] Commit: `feat(intercept): growable per-connection workspace-root auth, drop cwd gate (ADR-061)`

## Task 3: Read-safety — openat2 dirfd + lstat-ladder fallback

**Files:**
- Create: `crates/anvil-intercept/src/path_safety.rs`
- Test: same file

`open_workspace_dirfd(root) -> OwnedFd` (`O_PATH|O_DIRECTORY`); `read_under(dirfd, rel_path) -> io::Result<Vec<u8>>` via `openat2(RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH)`; lstat-ladder fallback where `openat2` unavailable (reuse the INTD-002 ladder). `normalise_rel(root, path) -> Result<RelPath, Escape>` (slash-normalise, reject `..`/absolute/symlink-escape). Applies to `path` AND `renamed.from`.

- [ ] Failing tests: `read_under_reads_real_file`, `read_under_rejects_symlink_escape` (create a symlink to `/etc/hostname`, assert refused), `normalise_rel_rejects_parent_escape`, `normalise_rel_rejects_absolute`.
- [ ] Run `cargo test -p eddacraft-anvil-intercept path_safety` — fail → implement → pass.
- [ ] `cargo clippy -p eddacraft-anvil-intercept --all-targets -- -D warnings`
- [ ] Commit: `feat(intercept): openat2 dirfd read-safety for workspace paths (ADR-061)`

## Task 4: Change classification — per-path identity table

**Files:**
- Create: `crates/anvil-intercept/src/change_class.rs`
- Test: same file

`struct PathIdentity { inode: u64, mtime: i64, size: u64 }`; `IdentityTable` keyed by case-normalised path (case-sensitivity probed once per workspace at startup). `classify(prev: Option<PathIdentity>, now: Option<PathIdentity>, raw: notify::EventKind) -> CanonicalChange` where `CanonicalChange = ContentModify | Create | Delete | Rename { from } `. Inode-flip on same path ⇒ `ContentModify` (atomic-save). `stat_on_validate(table, paths)` reconciles silent drift.

- [ ] Failing tests: `content_modify_same_inode`, `atomic_save_new_inode_is_content_modify` (the load-bearing one — simulate temp+rename: same path, new inode ⇒ ContentModify not Rename), `delete_classified`, `rename_decomposes_to_delete_create`, `case_only_rename_on_insensitive_fs`, `stat_on_validate_detects_drift_without_event`.
- [ ] Run `cargo test -p eddacraft-anvil-intercept change_class` — fail → implement → pass.
- [ ] Commit: `feat(intercept): inode-based FS change classification (ADR-061 §5)`

## Task 5: Invalidation taxonomy → StaleReason (default-deny)

**Files:**
- Create: `crates/anvil-intercept/src/assurance.rs` (taxonomy half)
- Test: same file

`fn taxonomy_reason(change: &CanonicalChange, ctx: &ChangeCtx) -> Option<StaleReason>` returning `None` only for the certifiable content-modify case; every other class (incl. config/boundary/policy edit, `.gitignore`, symlink-retarget, and an `_ => UnknownClass` fallthrough) maps to a `StaleReason`.

- [ ] Failing tests: one per `StaleReason` variant + `unknown_class_defaults_to_stale_not_clean`.
- [ ] Run `cargo test -p eddacraft-anvil-intercept assurance::taxonomy` — fail → implement → pass.
- [ ] Commit: `feat(intercept): default-deny invalidation taxonomy (ADR-061 §5)`

## Task 6: Certifiability — bounded reverse-impact closure

**Files:**
- Create: `crates/anvil-intercept/src/certify.rs`
- Test: same file

`fn certify(graph: &SymbolGraph, change: &CanonicalChange, delta: &GraphDelta, budget: usize) -> Certifiability` where `Certifiability = Certified { paths: Vec<PathBuf> } | Partial { reason: StaleReason }`. Logic: no export-surface delta ⇒ `Certified{[file]}`; else expand `dependents_of(file)` (1-hop), recurse on re-export surface changes, bounded by `budget`; overflow ⇒ `Partial{ImpactSetOverflow}`.

- [ ] Failing tests: `content_modify_no_export_change_certifies_self_only`, `export_surface_change_pulls_in_direct_importers`, `delete_invalidates_importers`, `reexport_chain_recurses_within_budget`, `overflow_returns_partial`. The headline: `new_export_making_unchanged_importer_illegal_is_not_certified_clean` (the reverse-dependency soundness case).
- [ ] Run `cargo test -p eddacraft-anvil-intercept certify` — fail → implement → pass.
- [ ] Commit: `feat(intercept): bounded reverse-impact closure certifiability (ADR-061 §6)`

## Task 7: Interim SymbolGraph cache (MLP2-067 folded in)

**Files:**
- Create: `crates/anvil-intercept/src/kernel_cache.rs`
- Test: same file

`HashMap<WorktreeKey, SymbolGraph>` behind the existing bounded-LRU + generation-guard + unregister-hook pattern (mirror `RuleSetCache`). `apply_delta(key, change)` via `graph::incremental::{update_file,remove_file,re_resolve_imports}`. Eviction bumps generation and demotes assurance to `Stale(WarmStateEvicted)` (Task 9 consumes this).

- [ ] Failing tests: `cold_build_then_warm_read`, `delta_update_mutates_in_place_not_rebuild`, `eviction_bumps_generation`, `generation_guard_blocks_stale_resolve`.
- [ ] Run `cargo test -p eddacraft-anvil-intercept kernel_cache` — fail → implement → pass.
- [ ] Commit: `feat(intercept): per-worktree SymbolGraph cache (MLP2-067, ADR-061 §4)`

## Task 8: `validate_paths` orchestration + latest-state coalescing

**Files:**
- Create: `crates/anvil-intercept/src/validate_paths.rs`
- Modify: `crates/anvil-intercept/src/ipc.rs` (dispatch arm)
- Test: `validate_paths.rs` tests + an ipc integration test

Orchestrate: auth (Task 2) → for each path classify (Task 4) + read-safe bytes (Task 3) → apply delta to cache (Task 7) → certify (Task 6) → `run_antipattern_check(changed_paths, config, workspace_root)` over warm state → assemble `diagnostics` + `evaluated[]` (hashes the daemon computed) + `workspace_assurance` + `coverage`. Coalescing: collapse only identical-`content_hash` duplicates; distinct-hash collapse returns the latest in `evaluated[]`.

- [ ] Failing tests: `validate_paths_certified_clean_for_self_contained_edit`, `validate_paths_partial_stale_on_overflow`, `evaluated_echoes_daemon_computed_hash`, `coalesce_collapses_identical_hash_only`, `client_supplied_hash_not_trusted_for_verdict` (send wrong hash, assert daemon re-reads), `dispatch_arm_routes_validate_paths`.
- [ ] Run `cargo test -p eddacraft-anvil-intercept validate_paths` — fail → implement → pass.
- [ ] Commit: `feat(intercept): validate_paths handler + latest-state coalescing (ADR-061 §2/§5)`

## Task 9: Assurance lifecycle + `workspace_status` + `request_full_scan`

**Files:**
- Modify: `crates/anvil-intercept/src/assurance.rs`, `crates/anvil-intercept/src/ipc.rs`
- Test: `assurance.rs` tests

State machine `Clean→Stale→Pending→Running→Clean`; `reason` non-optional for `Stale`; `scan_started_at` for `Running`; INFO log on every transition; scan-timeout ⇒ `Stale(ScanTimeout)`; daemon restart ⇒ any `Running` becomes `Stale`. Dispatch arms for `workspace_status` + `request_full_scan` (job handle, interactive|background priority).

- [ ] Failing tests: `transition_emits_log`, `stale_requires_reason`, `running_carries_scan_started_at`, `scan_timeout_to_stale`, `restart_running_becomes_stale`, `workspace_status_reports_state`, `request_full_scan_returns_job`.
- [ ] Run `cargo test -p eddacraft-anvil-intercept assurance` — fail → implement → pass.
- [ ] Commit: `feat(intercept): workspace assurance lifecycle + status/full_scan verbs (ADR-061 §9)`

## Task 10: Two cooperating rayon pools + chunked-yield background scans

**Files:**
- Create: `crates/anvil-intercept/src/workspace_pool.rs`
- Test: same file

Build a small interactive `rayon::ThreadPool` + a background pool from one per-host budget. Background full scan = chunked loop checking an `AtomicBool` cancel/yield flag between chunks. Per-`WorktreeKey` in-flight admission token over the existing `Semaphore` (ipc.rs:800).

- [ ] Failing tests: `interactive_pool_not_starved_by_background`, `background_scan_yields_within_one_chunk_on_cancel`, `per_workspace_token_bounds_inflight`.
- [ ] Run `cargo test -p eddacraft-anvil-intercept workspace_pool` — fail → implement → pass.
- [ ] Commit: `feat(intercept): two-pool scheduler + chunked-yield background scan (ADR-061 §4)`

## Task 11: Per-workspace DoS caps

**Files:**
- Modify: `crates/anvil-intercept/src/workspace_pool.rs` / `validate_paths.rs`
- Test: same

Max parse file size (skip oversized + emit a diagnostic), directory-walk depth cap. (Symlink cycles already dead via Task 3.)

- [ ] Failing tests: `oversized_file_skipped_with_diagnostic`, `walk_depth_capped`.
- [ ] fail → implement → pass.
- [ ] Commit: `feat(intercept): per-workspace parse-size and walk-depth caps (ADR-061 §4)`

## Task 12: `watch` client + scoped fallback

**Files:**
- Modify: `crates/anvil-cli/src/commands/watch.rs`
- Test: `watch.rs` tests (extend `watch_action_scope` suite)

Default save-time action: send classified changed paths to `validate_paths`. Daemon-absent ⇒ scoped `check` on changed paths (never `--all`), inheriting Task 3 guards, and surface `workspace_assurance{state: unavailable, reason: daemon-absent}` + WARN on first fallback. Keep `--action none`.

- [ ] Failing tests: `watch_routes_to_validate_paths_when_daemon_present`, `watch_fallback_is_scoped_never_all`, `watch_fallback_reports_unavailable_not_clean`, `first_fallback_warns_once`.
- [ ] Run `cargo test -p eddacraft-anvil -- watch` — fail → implement → pass.
- [ ] Commit: `feat(watch): route save-time validation through daemon, scoped fallback (ADR-061 §3)`

## Task 13: MCP `anvil_validate_write` re-point

**Files:**
- Modify: `crates/anvil-cli/src/mcp/validation.rs` (the in-process scan call site) + `crates/anvil-cli/src/mcp/tools/validate_write.rs`
- Test: corresponding tool test

Re-point the in-process scan to daemon `validate_paths`; in-process fallback when daemon absent (byte-identical via the parity contract).

- [ ] Failing tests: `validate_write_uses_daemon_when_present`, `validate_write_in_process_fallback`.
- [ ] fail → implement → pass.
- [ ] Commit: `feat(mcp): re-point anvil_validate_write to daemon validate_paths (DRVR, ADR-061 §3)`

## Task 14: Confinement mode + `anvil workspace` CLI

**Files:**
- Create: `crates/anvil-intercept/src/confinement.rs`, `crates/anvil-cli/src/commands/workspace.rs`
- Test: both

Operator-level config (`ANVIL_HOME`/XDG, owner-only): `admission = open|allowlist`, `allow = [exact + prefix]`. Allowlist mode refuses non-admitted roots with `workspace-not-admitted`, disables first-touch adopt, implicitly admits the primary check-in root. Config load failure **fails closed + loud**. CLI: `anvil workspace allow|deny|list|mode`.

- [ ] Failing tests: `open_mode_auto_adopts`, `allowlist_refuses_unlisted`, `primary_root_implicitly_admitted`, `prefix_entry_matches_subtree`, `config_load_failure_fails_closed`, `allowlist_not_read_from_repo_dotanvil`.
- [ ] fail → implement → pass.
- [ ] Commit: `feat(intercept,cli): opt-in workspace confinement mode (ADR-061 §7)`

## Task 15: Cross-path diagnostic parity golden test (gate)

**Files:**
- Create: `crates/anvil-intercept/tests/diagnostic_parity.rs`
- Fixture: `crates/anvil-intercept/tests/fixtures/parity-corpus/`

Run a fixed corpus through all four paths (watch+daemon, watch+fallback, MCP+daemon, MCP+fallback); assert identical finding sets **order-normalised** by `(path, rule_id, span_start)`. `workspace_assurance` carved out.

- [ ] Write the test + fixture; verify it fails (paths diverge before sort normalisation is wired).
- [ ] Add the shared sort-before-envelope normalisation; verify pass.
- [ ] Commit: `test(intercept): order-normalised cross-path diagnostic parity (ADR-061 §8)`

## Task 16: Concurrency SLO bench + CI gate

**Files:**
- Modify: `crates/anvil-intercept/benches/ipc_roundtrip.rs`
- Modify: CI workflow that runs the RLB gate (confirm path), `plans/modules/resource-load-benchmarking.aps.md` evidence

Add a `validate_paths` warm-read case + a `4 agents + 1 background scan` ramp asserting interactive p95 within the ADR-031 budget; WARN-log assertion on >80 ms pre-service queue wait (RLB-008). RLB-002 daemon-absent ramp as a separate case.

- [ ] Add bench cases; run `cargo bench -p eddacraft-anvil-intercept ipc_roundtrip` **standalone on a quiet box** (the harness is flaky in a backgrounded agent shell — known).
- [ ] Wire the CI gate; confirm it fails on a synthetic regression.
- [ ] Commit: `perf(intercept): validate_paths SLO bench + concurrency CI gate (RLB-008)`

## Task 17: Surface assurance in `anvil status` / `--json`

**Files:**
- Modify: `crates/anvil-cli/src/commands/status.rs` (or the TUI status surface)
- Test: status render test

Render `clean|stale|pending|running|unavailable` (+ `reason`), and `confined: N` when in allowlist mode; `unprotected (daemon not running)` on absence — never a stale cached `clean`.

- [ ] Failing tests: `status_renders_unavailable_when_daemon_absent`, `status_shows_confined_count`, `status_shows_stale_reason`.
- [ ] fail → implement → pass.
- [ ] Commit: `feat(cli): surface workspace assurance + confinement in status (ADR-061 §9)`

---

## Final verification (before PR)

- [ ] `cargo fmt --all --check` (verify by exit code, not piped tail)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace` (under Node 24 for any JS-touching gate; this is Rust-only)
- [ ] `cargo test -p eddacraft-anvil-checks` (the crate that loads check data — not just the intercept crate)
- [ ] Parity gate (Task 15) + the three §8 gates green; SLO bench (Task 16) green on a quiet box.
- [ ] Council (batch) on the diff before push; address CRITICAL/MAJOR.

## Sequencing & parallelism notes

- Tasks 1→7 are the dependency spine (wire → auth/read-safety → classify → taxonomy → certify → cache). 8 depends on 3–7. 9 depends on 7. 10–11 are independent of 8/9 and can run in parallel. 12–13 depend on 8. 14 depends on 2. 15 depends on 8+12+13. 16 depends on 8+10.
- The resource-model + bench tasks (10, 11, 16) are a candidate **sibling plan** if you want a second pair of hands; they share only `workspace_pool.rs` with the spine.
