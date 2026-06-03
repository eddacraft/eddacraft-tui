//! DSV-005 Task 8: the per-connection save-time verb orchestration.
//!
//! [`validate_paths`](crate::validate_paths::validate_paths) is the pure verdict
//! core (DSV-004/Task 8, already merged). This module is the daemon-side
//! orchestration that wires it to a live connection: it owns the shared,
//! cross-connection [`SaveTimeState`] (the warm graph cache, the per-`WorktreeKey`
//! assurance machines, the antipattern config, the interactive pool, and the
//! operator confinement policy) and the per-connection [`SaveTimeConn`] that
//! threads the admitted-root set through `ipc.rs`.
//!
//! The three save-time verbs are routed here from the `ipc.rs` JSON-RPC dispatch
//! arm (the special-method pattern, mirroring `handle_scan_buffer_jsonrpc`):
//! `validate_paths`, `workspace_status`, and `request_full_scan`.
//!
//! ## Authorisation (DSV-003 Task 2, deferred here)
//!
//! Each verb authorises its `workspace_root` against the connection's
//! [`AdmittedRoots`] set before touching any byte: the set is built once per
//! connection from the operator [`Confinement`] (open by default; allowlist when
//! confined), seeded with the first-named root as the primary check-in root, and
//! grows on first contact in open mode. All reads go through the held `O_PATH`
//! dirfd, so a refused root never reaches the filesystem and an admitted root
//! cannot be retargeted after admission (security C2/C3 — see
//! [`crate::workspace_admission`]).
//!
//! ## `fed_symbols`
//!
//! The kernel→daemon `FileSymbols` feed (ADR-064, Task 7 producer) is not wired
//! yet, so `fed_symbols` returns `None` for every path: every verdict is a safe
//! `Partial(CrossFileResolutionNeeded)` (B4 conservative default) until the feed
//! lands. The orchestration is otherwise live end to end.
//!
//! Unix-only: the verbs read arbitrary on-disk paths through `openat2`-guarded
//! dirfds, which have no Windows analogue in Sub-phase A (Windows `validate_paths`
//! GA is tracked separately — DSV out-of-scope).
#![cfg(unix)]

use std::collections::HashMap;
use std::io;
use std::os::fd::BorrowedFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use anvil_checks::antipattern::types::AntipatternCheckConfig;
use anvil_intercept_proto::protocol::{
    RequestFullScanRequest, RequestFullScanResponse, ValidatePathsRequest, ValidatePathsResponse,
    WorkspaceStatusRequest, WorkspaceStatusResponse,
};

use crate::assurance::{AssuranceMachine, ScanPriority};
use crate::confinement::Confinement;
use crate::ipc::{SaveTimeDispatch, SaveTimeError};
use crate::kernel_cache::KernelGraphCache;
use crate::path_safety::{normalise_rel, read_under};
use crate::rule_cache::WorktreeKey;
use crate::validate_paths::{ValidateEnv, validate_paths as run_validate_paths};
use crate::workspace_admission::AdmittedRoots;
use crate::workspace_pool::WorkScheduler;

/// The reverse-impact certify budget for the interactive verdict path. Bounds
/// the importer-closure walk per certify so a pathological fan-out cannot stall
/// the interactive pool; an overflow degrades to `Partial(ImpactSetOverflow)`,
/// which is safe. The `DoS` parse-size / walk-depth caps are DSV-006 (Task 11).
const SAVE_TIME_CERTIFY_BUDGET: usize = 256;

/// Shared, cross-connection save-time state. Held in an `Arc` on the
/// `IpcListener` and cloned per connection; every interior field is safe to
/// share (`KernelGraphCache` and the assurance map carry their own locks, the
/// pools are `Sync`, the config + confinement are immutable).
pub struct SaveTimeState {
    /// The warm per-`WorktreeKey` `(SymbolGraph, DependencyGraph)` cache.
    cache: KernelGraphCache,
    /// The per-`WorktreeKey` workspace-assurance state machines.
    assurance: Mutex<HashMap<WorktreeKey, AssuranceMachine>>,
    /// Antipattern check configuration (patterns, extensions, threshold).
    config: AntipatternCheckConfig,
    /// The two cooperating rayon pools; the antipattern scan runs on the
    /// interactive pool (B7), never the global one.
    scheduler: WorkScheduler,
    /// The operator confinement policy a per-connection admitted-root set is
    /// built from (open by default).
    confinement: Confinement,
}

impl SaveTimeState {
    /// Build the shared state from its collaborators. The daemon constructs this
    /// once in `run_foreground` and injects it via
    /// [`IpcListener::with_save_time_state`](crate::ipc::IpcListener::with_save_time_state).
    #[must_use]
    pub fn new(
        scheduler: WorkScheduler,
        config: AntipatternCheckConfig,
        confinement: Confinement,
    ) -> Self {
        Self {
            cache: KernelGraphCache::new(),
            assurance: Mutex::new(HashMap::new()),
            config,
            scheduler,
            confinement,
        }
    }

    /// The operator confinement policy (used by the registry unregister hook
    /// wiring and tests).
    #[must_use]
    pub fn confinement(&self) -> &Confinement {
        &self.confinement
    }

    /// Drop a worktree's warm cache + assurance machine (wired to the registry
    /// unregister hook so an unregistered session does not leave stale warm
    /// state behind).
    pub fn invalidate(&self, key: &WorktreeKey) {
        self.cache.invalidate(key);
        self.lock_assurance().remove(key);
    }

    fn lock_assurance(&self) -> std::sync::MutexGuard<'_, HashMap<WorktreeKey, AssuranceMachine>> {
        self.assurance
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Run `f` against the worktree's assurance machine, inserting a fresh one
    /// (`Stale(CrossFileResolutionNeeded)`, B6) on first contact.
    fn with_machine<R>(&self, key: &WorktreeKey, f: impl FnOnce(&mut AssuranceMachine) -> R) -> R {
        let mut guard = self.lock_assurance();
        // `AssuranceMachine::default()` is `new()` — `Stale(CrossFileResolutionNeeded)` (B6).
        let machine = guard.entry(key.clone()).or_default();
        f(machine)
    }
}

/// Per-connection save-time context: borrows the shared [`SaveTimeState`] and
/// owns this connection's [`AdmittedRoots`] set, built lazily on the first verb.
pub struct SaveTimeConn<'a> {
    state: &'a SaveTimeState,
    /// The admitted-root set, built once on the first verb (seeded with that
    /// verb's root as the primary check-in root — the merged confinement
    /// contract that `to_admitted_roots` is called once per connection).
    admitted: Option<AdmittedRoots>,
}

impl<'a> SaveTimeConn<'a> {
    /// Open a per-connection context over the shared state.
    #[must_use]
    pub fn new(state: &'a SaveTimeState) -> Self {
        Self {
            state,
            admitted: None,
        }
    }
}

impl SaveTimeDispatch for SaveTimeConn<'_> {
    fn validate_paths(
        &mut self,
        request: &ValidatePathsRequest,
    ) -> Result<ValidatePathsResponse, SaveTimeError> {
        let root = PathBuf::from(&request.workspace_root);
        let key = WorktreeKey::from_canonical(root.clone());
        // Copy the shared-state reference so `state.*` reads stay disjoint from
        // the per-connection `self.admitted` field the held fd borrows.
        let state = self.state;
        let fd = authorise_root(&mut self.admitted, &state.confinement, &root)?;

        // All reads go through the held dirfd — the guarded bytes the
        // antipattern check scans, never a re-opened path (B7 / security C2).
        let read_guarded = move |rel: &str| -> io::Result<Vec<u8>> {
            let parsed = normalise_rel(rel).map_err(|escape| {
                io::Error::new(io::ErrorKind::InvalidInput, format!("{escape:?}"))
            })?;
            read_under(fd, &parsed)
        };

        let env = ValidateEnv {
            config: &state.config,
            pool: state.scheduler.interactive(),
            budget: SAVE_TIME_CERTIFY_BUDGET,
        };
        Ok(state.with_machine(&key, |machine| {
            // `fed_symbols` is stubbed `None` until the kernel feed lands (Task
            // 7 producer) — every verdict is a safe `Partial` (B4).
            run_validate_paths(request, &state.cache, machine, read_guarded, |_| None, &env)
        }))
    }

    fn workspace_status(
        &mut self,
        request: &WorkspaceStatusRequest,
    ) -> Result<WorkspaceStatusResponse, SaveTimeError> {
        let root = PathBuf::from(&request.workspace_root);
        let key = WorktreeKey::from_canonical(root.clone());
        let state = self.state;
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let workspace_assurance = state.with_machine(&key, |machine| machine.snapshot());
        Ok(WorkspaceStatusResponse {
            workspace_assurance,
        })
    }

    fn request_full_scan(
        &mut self,
        request: &RequestFullScanRequest,
    ) -> Result<RequestFullScanResponse, SaveTimeError> {
        let root = PathBuf::from(&request.workspace_root);
        let key = WorktreeKey::from_canonical(root.clone());
        let state = self.state;
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let workspace_assurance = state.with_machine(&key, |machine| {
            // An explicit client request is interactive (client-blocking).
            machine.request_full_scan(ScanPriority::Interactive);
            machine.snapshot()
        });
        Ok(RequestFullScanResponse {
            workspace_assurance,
        })
    }
}

/// Authorise `root` against the connection's admitted set, building it on first
/// contact (seeded with `root` as the primary check-in root). Returns the held
/// read-anchor dirfd. Kept a free function over the `admitted` field (not a
/// `&mut self` method) so the returned fd's borrow stays disjoint from the
/// caller's shared-state reads.
fn authorise_root<'f>(
    admitted: &'f mut Option<AdmittedRoots>,
    confinement: &Confinement,
    root: &Path,
) -> Result<BorrowedFd<'f>, SaveTimeError> {
    let set = admitted.get_or_insert_with(|| confinement.to_admitted_roots(root));
    set.authorise(root)
        .map_err(SaveTimeError::Io)?
        .ok_or(SaveTimeError::NotAdmitted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_intercept_proto::protocol::{
        AssuranceState, ChangeDescriptor, ChangeKindWire, Coverage, StaleReason,
    };
    use std::fs;

    fn state() -> SaveTimeState {
        SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        )
    }

    fn modified(path: &str) -> ChangeDescriptor {
        ChangeDescriptor {
            path: path.to_string(),
            change: ChangeKindWire::Modified,
            content_hash: None,
            mtime: None,
        }
    }

    /// The daemon reads each path through the admitted dirfd and echoes ITS
    /// hash of the on-disk bytes — proving the verdict scans the guarded bytes,
    /// never a path the client re-named (B7 / security C2).
    #[test]
    fn validate_paths_passes_guarded_bytes_not_paths_to_check() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        fs::create_dir(&src).expect("mkdir");
        let body = b"export function foo() { return 1; }";
        fs::write(src.join("a.ts"), body).expect("write");

        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let request = ValidatePathsRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            paths: vec![modified("src/a.ts")],
        };
        let resp = conn.validate_paths(&request).expect("admitted");

        assert_eq!(resp.evaluated.len(), 1);
        assert_eq!(resp.evaluated[0].path, "src/a.ts");
        assert_eq!(
            resp.evaluated[0].content_hash.as_deref(),
            Some(crate::validate_paths::content_hash(body).as_str()),
            "the daemon echoes its hash of the dirfd-read bytes",
        );
    }

    /// A path that escapes the admitted root (absolute / `..`) is refused by the
    /// guard and contributes no readable bytes — never read by re-opening.
    #[test]
    fn validate_refuses_escaping_path_under_guard() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let request = ValidatePathsRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            paths: vec![modified("../../etc/passwd")],
        };
        let resp = conn.validate_paths(&request).expect("admitted");
        assert_eq!(resp.coverage, Coverage::Partial);
        assert_eq!(
            resp.evaluated[0].content_hash, None,
            "an escaping path is refused by the guard, so there is nothing to hash",
        );
    }

    /// B6: a verdict on a never-seen workspace is `Partial` (the warm cache is
    /// cold and the feed has not delivered symbols), and the workspace starts
    /// `Stale`, never `Clean`.
    #[test]
    fn initial_workspace_state_is_stale_not_clean() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        fs::create_dir(&src).expect("mkdir");
        fs::write(src.join("a.ts"), b"export const x = 1;").expect("write");

        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let request = ValidatePathsRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            paths: vec![modified("src/a.ts")],
        };
        let resp = conn.validate_paths(&request).expect("admitted");
        assert_eq!(resp.coverage, Coverage::Partial);
        assert_eq!(resp.workspace_assurance.state, AssuranceState::Stale);
        assert_eq!(
            resp.workspace_assurance.reason,
            Some(StaleReason::CrossFileResolutionNeeded),
        );
    }

    /// `workspace_status` is read-only and reports the cold workspace as
    /// `Stale(CrossFileResolutionNeeded)` (B6) without certifying anything.
    #[test]
    fn workspace_status_reports_state() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let request = WorkspaceStatusRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
        };
        let resp = conn.workspace_status(&request).expect("admitted");
        assert_eq!(resp.workspace_assurance.state, AssuranceState::Stale);
        assert_eq!(
            resp.workspace_assurance.reason,
            Some(StaleReason::CrossFileResolutionNeeded),
        );
    }

    /// `request_full_scan` queues a scan and returns the post-request snapshot
    /// — `Pending` with no staleness reason (a queued job, not a verdict).
    #[test]
    fn request_full_scan_returns_job() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let request = RequestFullScanRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
        };
        let resp = conn.request_full_scan(&request).expect("admitted");
        assert_eq!(resp.workspace_assurance.state, AssuranceState::Pending);
        assert_eq!(resp.workspace_assurance.reason, None);
    }

    /// In allowlist mode an unlisted root is refused before any byte is read.
    #[test]
    fn allowlist_refuses_unlisted_root() {
        let allowed = tempfile::tempdir().expect("tempdir");
        let other = tempfile::tempdir().expect("tempdir");
        // Confine to `allowed` only.
        let confinement = Confinement::from_file(crate::confinement::ConfinementConfigFile {
            admission: crate::confinement::AdmissionModeFile::Allowlist,
            allow: vec![crate::confinement::AllowEntry {
                path: allowed.path().to_path_buf(),
                kind: crate::confinement::MatchKind::Exact,
            }],
        });
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            confinement,
        );
        let mut conn = SaveTimeConn::new(&state);
        // The first verb seeds the primary root; use `other` as primary so the
        // allowlist (which lists only `allowed`) does NOT implicitly cover it.
        // A status query on `other`'s sibling is refused.
        let request = WorkspaceStatusRequest {
            workspace_root: other.path().to_string_lossy().into_owned(),
        };
        // `other` becomes the implicitly-admitted primary, so it authorises;
        // a *different* unlisted root does not.
        conn.workspace_status(&request).expect("primary admitted");
        let third = tempfile::tempdir().expect("tempdir");
        let refused = conn.workspace_status(&WorkspaceStatusRequest {
            workspace_root: third.path().to_string_lossy().into_owned(),
        });
        assert!(
            matches!(refused, Err(SaveTimeError::NotAdmitted)),
            "an unlisted, non-primary root must be refused: {refused:?}",
        );
    }
}
