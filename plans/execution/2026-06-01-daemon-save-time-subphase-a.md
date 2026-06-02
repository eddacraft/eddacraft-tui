# Daemon Save-time Validation — Sub-phase A Implementation Plan

**Goal:** Ship the frozen `validate_paths` wire + watch client + MCP re-point, backed by an interim per-`WorktreeKey` `SymbolGraph` cache (rebuild-on-restart, no persistence), behind the §8 correctness bar.
**Architecture:** Three new verdict-shaped JSON-RPC verbs on the existing intercept daemon reuse the `scan_buffer` handshake/transport/envelope. A per-path identity table classifies FS events; a bounded reverse-impact closure over a **net-new** `DependencyGraph.reverse` index (built + maintained by the cache — see corrections B1/B5) decides certified-vs-Stale; `run_antipattern_check` runs against the changed-path set over warm `SymbolGraph` state. `watch` and MCP become thin daemon clients with a scoped (never `--all`) fallback.
**Tech Stack:** Rust (anvil-intercept, anvil-intercept-proto, anvil-checks, anvil-kernel, anvil-cli), tokio, rayon, JSON-RPC 2.0 NDJSON, criterion (ipc_roundtrip).

Spec: [`plans/specs/2026-06-01-daemon-save-time-validation-contract.md`](../specs/2026-06-01-daemon-save-time-validation-contract.md) · ADR: [`plans/decisions/061-...md`](../decisions/061-save-time-daemon-delta-validation.md)

**Out of scope (do NOT implement here):** sub-phase A′ (GV2 hot-read slice — blocked on the GV2 hot-/non-hot-path boundary gate) and sub-phase B (GV2-021 persistence/warm-start). The backing in this plan is the interim `SymbolGraph` cache only.

---

## ⚠️ Council review corrections — REQUIRED before implementation (2026-06-01)

A review council (architect, kernel-maintainer, adversarial, operations, security,
pragmatic-lead; + Codex input) reviewed this plan and returned **do not start as
written**. Full evidence + rulings:
[`plans/reviews/2026-06-01-daemon-graph-council-verdict.md`](../reviews/2026-06-01-daemon-graph-council-verdict.md).
The tasks below stand, but apply these corrections first (ordered — earlier items
are predecessors).

**Resolution status (2026-06-02):** **B5, B1, B2, B6, B7, B4 are RESOLVED** — folded into
[ADR-061](../decisions/061-save-time-daemon-delta-validation.md) (§6, §9),
the [validation contract](../specs/2026-06-01-daemon-save-time-validation-contract.md)
(§1, §3, §6, §7), and Tasks 1/6/7/8/9 below (with their named tests). B7's
read-safety/pool corrections are folded into Task 2 (net-new auth wording), Task 8 +
the File Map (`run_antipattern_check` bytes+pool entrypoint), and the sequencing notes
(Task 3 a hard predecessor of Task 8; Task 10's pool a predecessor of Task 8's check
call). **Remaining pre-implementation: B3 (item 4) and the ops/security placement
items (item 8).** Sub-phase A coding may begin on the B1/B2/B4/B5/B6/B7 surface once
those remaining items are closed.

1. **Crate boundary (B5, compile blocker).** `anvil-intercept` depends only on
   `anvil-kernel-types`; `anvil-kernel` arrives only via dev-deps (`watcher.rs:28`
   documents the deliberate refusal). Tasks 6/7/8 cannot compile.
   **Resolved by
   [ADR-064](../decisions/064-intercept-graph-cache-crate-boundary.md) (Accepted
   2026-06-02): extract `eddacraft-anvil-graph-cache`** (`SymbolGraph`,
   `DependencyGraph`, incremental apply-delta, `certify`) — `petgraph`-only, no
   parser surface — and depend on it from both `anvil-kernel` and
   `anvil-intercept`; relocate the plain `ImportEdge`/`FileSymbols` structs to
   `anvil-kernel-types::graph`. The cycle audit is clean (kernel does not depend
   on intercept). *Predecessor to 2–3 — land the ADR-064 extraction first.*
2. **Reverse index is net-new (B1, critical). ✅ Resolved 2026-06-02** — folded
   into Task 6 (`certify(sym, dep, …)` signature), Task 7 (cache holds the pair +
   `apply_delta` maintains the reverse index), ADR-061 §6, and contract §3; tests
   `certify_uses_dependency_graph_reverse_not_symbol_graph_scan` +
   `reverse_index_consistent_after_delta` recorded on Task 6. Task 7 caches a
   `(SymbolGraph, DependencyGraph)` pair per `WorktreeKey`; cold-build derives
   `DependencyGraph` from resolved import edges; `apply_delta` maintains the reverse
   index. **Task 6 signature → `certify(sym: &SymbolGraph, dep: &DependencyGraph, change, delta, budget)`.**
   `dependents_of` is **not** "existing / O(1)" — it has zero non-test callers today.
3. **`certified` must not over-claim (B2, critical). ✅ Resolved 2026-06-02** —
   `check_families: ["antipattern"]` added to the Task 1 frozen wire
   (`ValidatePathsResponse` + `CheckFamily` enum) and the contract §1 response;
   `coverage: certified` + the §7 parity gate scoped to that family in contract
   §1/§3/§7 and ADR-061 §6/§8; noted that the structural policy checks are not run
   on the `validate_paths` hot path. Test `response_carries_check_families`
   recorded on Task 1. On the hot path only
   `run_antipattern_check` (a stateless regex scanner on `anvil-kernel-types`) runs
   — the four structural
   policy checks (`CrossLayerViolation`/`NewDependencyIntroduction`/`PublicApiExpansion`/`PrivilegeExpansion`,
   `embedded.rs:119-133`) do **not**. Add a **`check_families: ["antipattern"]`** field
   to `ValidatePathsResponse` and scope `coverage: certified` + the §8.2 parity gate
   (ADR-061 §8) to that family across all surfaces. **Do NOT** run the structural
   policy checks on the hot path (council *overturned* that fix — it reopens the
   CPU regression ADR-061 exists to solve; the embedded structural pipeline
   `run_embedded` has no production caller today, and the live structural engine is
   whole-repo `anvil gate`).
4. **Proto envelope type (B3).** `ScanDiagnostics` does not exist; the real type is
   `ScanBufferResponse` (daemon-local, `midedit.rs:68`). Define the shared diagnostic
   type **in `anvil-intercept-proto`** (lighter form: `Vec<anvil_kernel_types::Diagnostic>`
   is fine *if defined in the proto crate, not re-declared daemon-local*); type
   `ValidatePathsResponse.diagnostics` against it; add scan_buffer↔validate_paths
   serialise-parity tests.
5. **Initial assurance state (B6, critical). ✅ Resolved 2026-06-02** — initial
   `Stale(CrossFileResolutionNeeded)` + `watch` auto-`request_full_scan` on
   connect/reconnect folded into contract §6, ADR-061 §9 (full lifecycle diagram
   now starts at `(connect) → Stale(cross-file-resolution-needed)`), and Tasks 7/9;
   tests `initial_workspace_state_is_stale_not_clean` +
   `watch_auto_requests_full_scan_on_connect` recorded on Task 9. Sub-phase A has
   no background scheduler, so the connect-time scan is the only path from initial
   `Stale` to `clean`.
6. **Export-surface conservative default (B4). ✅ Resolved 2026-06-02** — folded
   into ADR-061 §6 (the conservative default + `removed_edges`-always-empty note
   on the certifiability bullets) and contract §3 (the `previously_public`
   set-diff correction). Default any modify touching public/privileged symbols to
   **partial/stale** until a real export-diff helper lands; the decision is made
   by the `GraphDelta.previously_public` set-diff (no dedicated
   `export_surface_changed()` helper mandated for Sub-phase A — the conservative
   default is). Task 6 below carries the edge-case fixtures/tests
   (`body_only_change_certifies_self_only`, `touched_public_symbol_defaults_to_partial`,
   `rename_is_export_surface_change`, `delete_is_export_surface_change`,
   `internal_to_public_defaults_to_partial`,
   `reexport_add_remove_is_surface_change`). Note
   `delta.removed_edges` is always empty (`incremental.rs:150`) — importer
   discovery uses `dependents_of` exclusively.
7. **Read-safety + pool gaps (B7, new majors). ✅ Resolved 2026-06-02** — folded into
   Task 8 (orchestration prose), the File Map (`crates/anvil-checks/src/antipattern/check.rs`
   now listed Modify), the sequencing notes, and Task 2 (net-new auth wording).
   Task 3 (openat2/`RESOLVE_NO_SYMLINKS`) is now a **hard predecessor of Task 8**:
   `run_antipattern_check` (`check.rs:95`) takes file paths and does unguarded
   `fs::read_to_string` (`check.rs:118`) on the global rayon pool (`.par_iter()`,
   `check.rs:113`). The daemon path must instead scan **pre-read guarded bytes** on
   **Task 10's interactive `&rayon::ThreadPool`** — Task 8 records the preferred shape
   (extract a bytes+pool core; keep the path-based fn as a thin wrapper for the 9
   disk-reading CLI call sites) and the full caller list. Task 2 reworded: the
   workspace-root auth handshake is **net-new** (the `validate_workspace_roots` API
   ships but has no production caller — DRVR-001 Wave 2 left it unwired), not "reuse".
8. **Placement + observability (ops/security majors).** Resolve `confinement.rs`
   placement (the `ANVIL_HOME` resolver it reuses lives in `anvil-cli`); specify the
   structured-log fields for assurance transitions (route via the ADR-035
   Notification envelope); add a daemon mid-session disconnect/reconnect spec for
   `watch`.
9. **Non-blocking:** mark the interim cache API as A′-replaced (module comments);
   note the SLO bench (Tasks 10/11/16) is an ADR-061 §9 **Phase-2 merge dependency**,
   not a sibling-plan candidate; make the `ALL_ANVIL_METHODS` pin test two-directional;
   narrow the "never a source read-oracle" wording.

**GV2 / A′-only (do not block Sub-phase A):** land
`docs/architecture/graph-v2-foundation-spec.md` before ticking "taxonomy accepted";
GV2-002 stable identity before the export fast-path graduates from conservative-partial.

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
| `crates/anvil-checks/src/antipattern/check.rs` | Modify | **B7:** extract a daemon entrypoint that takes **pre-read guarded bytes** (not paths; closes the `fs::read_to_string` TOCTOU at `check.rs:118`) + a **`&rayon::ThreadPool`** so Task 10's interactive pool governs it (replaces the global-pool `.par_iter()` at `check.rs:113`). See Task 8 for the signature-vs-wrapper decision and the full disk-caller list. |

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
#[serde(rename_all = "kebab-case")] pub enum CheckFamily { Antipattern }   // B2: families the hot path runs; frozen as [antipattern] for sub-phase A
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
    pub check_families: Vec<CheckFamily>,   // B2: `certified` attests ONLY these families (= [antipattern])
}
// WorkspaceStatusRequest/Response, RequestFullScanRequest/Response similarly.
```

- [ ] Write failing tests: `validate_paths_method_const`, `change_descriptor_roundtrip_all_variants`, `response_tolerates_unknown_additive_field` (MLP2-052 forward-compat style — deserialize a JSON with an extra field, assert Ok), `stale_reason_kebab_wire_strings`, `no_graph_version_field` (assert serialized `WorkspaceAssurance`/`WorkspaceStatus` JSON has no `graph_version` key), `response_carries_check_families` (B2: a `certified` response serialises `check_families: ["antipattern"]`).
- [ ] Run: `cargo test -p eddacraft-anvil-intercept-proto` — verify fail.
- [ ] Implement the types.
- [ ] Run: `cargo test -p eddacraft-anvil-intercept-proto` — verify pass.
- [ ] Commit: `feat(intercept-proto): freeze validate_paths verdict-shaped wire (ADR-061)`

## Task 2: Authorisation — growable per-connection workspace-root set; drop cwd

**Files:**
- Modify: `crates/anvil-intercept/src/auth.rs`, `crates/anvil-intercept/src/ipc.rs`
- Test: `crates/anvil-intercept/src/auth.rs` tests

Wire `DriverManifest::validate_workspace_roots` against active sessions. **This is net-new wiring (B7), not "reuse":** the API ships in `auth.rs` with unit tests but has **no production caller today** — the `auth.rs` module header (lines 26–30) documents the driver consumer as deferred to DRVR-001 (Wave 2: "no `lib.rs` consumer side-effect is added in this PR"), and `validate_workspace_roots`/`is_driver_allowed` have zero call sites outside `auth.rs` itself (confirmed by grep of `ipc.rs`). `validate_paths` is the **first verb to read arbitrary on-disk paths**, which makes this handshake (and Task 3's read-safety) load-bearing rather than incidental reuse. Add a per-connection `BTreeSet<PathBuf>` of admitted (canonicalised) roots that grows on first contact (mode-gated by Task 14). Authorise a verb iff its `workspace_root` is in the set (or admissible under `open` mode). No `/proc/<pid>/cwd` anywhere.

- [ ] Failing tests: `validate_paths_authorised_for_session_root`, `validate_paths_refused_for_unrelated_root_in_allowlist_mode`, `root_set_grows_on_first_touch_in_open_mode`, `no_cwd_in_auth_path` (grep-guard test or absence assertion), `validate_workspace_roots_now_has_a_production_caller` (B7: asserts `ipc.rs` dispatch wires the previously-unwired API).
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

`fn certify(sym: &SymbolGraph, dep: &DependencyGraph, change: &CanonicalChange, delta: &GraphDelta, budget: usize) -> Certifiability` where `Certifiability = Certified { paths: Vec<PathBuf> } | Partial { reason: StaleReason }`. **(Corrected per B1: takes the net-new `DependencyGraph` — `dependents_of` lives there, not on `SymbolGraph`.)** Logic: no export-surface delta ⇒ `Certified{[file]}`; else expand `dep.dependents_of(file)` (1-hop), recurse on re-export surface changes, bounded by `budget`; overflow ⇒ `Partial{ImpactSetOverflow}`. **Conservative default (B4): the export-surface decision is the `GraphDelta.previously_public` set-diff (no dedicated `export_surface_changed()` helper mandated for Sub-phase A); any modify touching public/privileged symbols defaults to `Partial` until a real export-diff helper lands — only a body-only change with no `previously_public` delta stays `Certified{[file]}`. This is conservatively safe (rename = delete+add = surface change). Importer discovery reads `dependents_of` exclusively — `delta.removed_edges` is always empty (`incremental.rs:150`), so certify must never branch on it.**

- [ ] Failing tests: `content_modify_no_export_change_certifies_self_only`, `export_surface_change_pulls_in_direct_importers`, `delete_invalidates_importers`, `reexport_chain_recurses_within_budget`, `overflow_returns_partial`. The headline: `new_export_making_unchanged_importer_illegal_is_not_certified_clean` (the reverse-dependency soundness case). **B1-required:** `certify_uses_dependency_graph_reverse_not_symbol_graph_scan` (asserts the closure reads `dep.dependents_of`, not a `SymbolGraph` scan) and `reverse_index_consistent_after_delta` (the `apply_delta`-maintained reverse index matches a cold rebuild). **B4-required export-surface fixtures (driven off the `previously_public` set-diff):** `body_only_change_certifies_self_only` (→ certified), `touched_public_symbol_defaults_to_partial` (any `previously_public` delta ⇒ partial), `rename_is_export_surface_change` (→ partial), `delete_is_export_surface_change` (→ partial), `internal_to_public_defaults_to_partial` (new public symbol ⇒ partial), `reexport_add_remove_is_surface_change` (re-export add/remove ⇒ partial), and `certify_never_reads_removed_edges` (asserts the closure does not branch on the always-empty `delta.removed_edges`).
- [ ] Run `cargo test -p eddacraft-anvil-intercept certify` — fail → implement → pass.
- [ ] Commit: `feat(intercept): bounded reverse-impact closure certifiability (ADR-061 §6)`

## Task 7: Interim SymbolGraph cache (MLP2-067 folded in)

**Files:**
- Create: `crates/anvil-intercept/src/kernel_cache.rs`
- Test: same file

`HashMap<WorktreeKey, (SymbolGraph, DependencyGraph)>` **(corrected per B1: holds the pair; cold-build derives `DependencyGraph` from resolved import edges, `apply_delta` maintains the reverse index)** behind the existing bounded-LRU + generation-guard + unregister-hook pattern (mirror `RuleSetCache`). `apply_delta(key, change)` via `graph::incremental::{update_file,remove_file,re_resolve_imports}` plus reverse-index maintenance. Eviction bumps generation and demotes assurance to `Stale(WarmStateEvicted)` (Task 9 consumes this). **First connect / cold key ⇒ initial state `Stale(CrossFileResolutionNeeded)` (B6), never `Clean`.**

- [ ] Failing tests: `cold_build_then_warm_read`, `delta_update_mutates_in_place_not_rebuild`, `eviction_bumps_generation`, `generation_guard_blocks_stale_resolve`.
- [ ] Run `cargo test -p eddacraft-anvil-intercept kernel_cache` — fail → implement → pass.
- [ ] Commit: `feat(intercept): per-worktree SymbolGraph cache (MLP2-067, ADR-061 §4)`

## Task 8: `validate_paths` orchestration + latest-state coalescing

**Files:**
- Create: `crates/anvil-intercept/src/validate_paths.rs`
- Modify: `crates/anvil-intercept/src/ipc.rs` (dispatch arm)
- Modify: `crates/anvil-checks/src/antipattern/check.rs` (B7 predecessor — bytes+pool entrypoint, see below)
- Test: `validate_paths.rs` tests + an ipc integration test

Orchestrate: auth (Task 2) → for each path classify (Task 4) + read-safe bytes (Task 3) → apply delta to cache (Task 7) → certify (Task 6) → `run_antipattern_check(...)` over warm state → assemble `diagnostics` + `evaluated[]` (hashes the daemon computed) + `workspace_assurance` + `coverage`. **Per corrections B7/B3: the check here scans the Task 3 pre-read guarded bytes (NOT raw `changed_paths` — re-reading reopens the openat2/TOCTOU window) and runs under Task 10's interactive `&rayon::ThreadPool` (NOT the global pool); a bytes+pool entrypoint must therefore exist before this task lands (see the B7 predecessor step), and the `diagnostics` it yields are scoped to `check_families: ["antipattern"]`.** Coalescing: collapse only identical-`content_hash` duplicates; distinct-hash collapse returns the latest in `evaluated[]`.

- [ ] **B7 predecessor — give `run_antipattern_check` a guarded-bytes + injected-pool path in `crates/anvil-checks/src/antipattern/check.rs`.** Today it is `run_antipattern_check(files: &[&str], config, workspace_root)`: it reads each path with `fs::read_to_string` (`check.rs:118`) on the global rayon pool (`.par_iter()`, `check.rs:113`). **Design decision (flag for Council):** prefer extracting a core `run_antipattern_check_bytes(files: &[(&str, &[u8])], config, workspace_root, pool: &rayon::ThreadPool)` that the daemon calls with Task 3's guarded bytes and Task 10's interactive pool, and making the existing path-based `run_antipattern_check` a thin wrapper (reads bytes via `fs::read_to_string` on a default pool, then delegates) — this closes the daemon-path TOCTOU and pool-bleed **without** churning the disk-reading callers, which have no openat2 guard and legitimately read from cwd. The alternative — change the one signature and migrate every caller — touches **9 call sites in 8 files**: `commands/check.rs:280`, `commands/gate.rs:619`, `commands/drift.rs:707`, `commands/baseline.rs:588`, `insights/suppressions.rs:146`, `l4_engine.rs:216` (watch), `mcp/tools/check.rs:88`, `mcp/tools/gate.rs:124`, `services/sample_analyser.rs:113,196`, and couples every CLI surface to Task 10's pool. (`embedded.rs` is **not** a caller — it already takes caller-supplied bytes via `EnforcementPipeline`, which is the model to mirror.) Failing tests in `anvil-checks`: `run_antipattern_check_bytes_scans_supplied_bytes_not_disk` (the daemon core never touches the filesystem for content) and `run_antipattern_check_bytes_runs_on_supplied_pool` (work executes on the injected pool, not the global one). Run `cargo test -p eddacraft-anvil-checks`.
- [ ] Failing tests: `validate_paths_certified_clean_for_self_contained_edit`, `validate_paths_partial_stale_on_overflow`, `evaluated_echoes_daemon_computed_hash`, `coalesce_collapses_identical_hash_only`, `client_supplied_hash_not_trusted_for_verdict` (send wrong hash, assert daemon re-reads), `dispatch_arm_routes_validate_paths`, `validate_paths_passes_guarded_bytes_not_paths_to_check` (B7: the handler hands `run_antipattern_check` the Task 3 bytes, never re-opens the file).
- [ ] Run `cargo test -p eddacraft-anvil-intercept validate_paths` — fail → implement → pass.
- [ ] Commit: `feat(intercept): validate_paths handler + latest-state coalescing (ADR-061 §2/§5)`

## Task 9: Assurance lifecycle + `workspace_status` + `request_full_scan`

**Files:**
- Modify: `crates/anvil-intercept/src/assurance.rs`, `crates/anvil-intercept/src/ipc.rs`
- Test: `assurance.rs` tests

State machine **initial `Stale(CrossFileResolutionNeeded)` (B6) →** `Pending→Running→Clean`, then `Clean→Stale` on an uncertifiable delta; `reason` non-optional for `Stale`; `scan_started_at` for `Running`; structured transition log via the ADR-035 Notification envelope (named fields, not a bare INFO line — B8); scan-timeout ⇒ `Stale(ScanTimeout)`; daemon restart ⇒ any `Running` becomes `Stale`. `watch` auto-issues `request_full_scan` on connect/reconnect. Dispatch arms for `workspace_status` + `request_full_scan` (job handle, interactive|background priority).

- [ ] Failing tests: `transition_emits_log`, `stale_requires_reason`, `running_carries_scan_started_at`, `scan_timeout_to_stale`, `restart_running_becomes_stale`, `workspace_status_reports_state`, `request_full_scan_returns_job`. **B6-required:** `initial_workspace_state_is_stale_not_clean` (a fresh/cold-key workspace starts `Stale(CrossFileResolutionNeeded)`, and `validate_paths` on it returns `coverage: partial` until a scan completes) and `watch_auto_requests_full_scan_on_connect`.
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

- Tasks 1→7 are the dependency spine (wire → auth/read-safety → classify → taxonomy → certify → cache). 8 depends on 3–7 **and on Task 10's pool construction** (B7). **(B7) Task 3 (read-safety) is a *hard* predecessor of Task 8** — Task 8 must hand `run_antipattern_check` Task 3's guarded bytes, never re-open files. **(B7) Task 8 also depends on Task 10's interactive pool** (the check runs on it, not the global pool) and on the `run_antipattern_check` signature change (File Map `check.rs`). 9 depends on 7. Task 10's *pool construction* is therefore a predecessor of Task 8, but Task 10's chunked background-scan loop and Task 11 stay independent of 8/9 and can run in parallel. 12–13 depend on 8. 14 depends on 2. 15 depends on 8+12+13. 16 depends on 8+10.
- The resource-model + bench tasks (10, 11, 16) are a candidate **sibling plan** if you want a second pair of hands; they share only `workspace_pool.rs` with the spine.
