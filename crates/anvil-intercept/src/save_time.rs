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
//! ## Symbols feed ([`SymbolParser`])
//!
//! To certify, the verdict needs the edited file's parsed [`FileSymbols`]. The
//! daemon never parses (ADR-064); instead it enriches the change it holds by
//! computing symbols through an injected [`SymbolParser`] (a Messaging Gateway —
//! the tree-sitter impl lives in `anvil-cli`), handing it the **exact**
//! openat2-guarded bytes it read and hashed. When no parser is injected the feed
//! yields `None` and every verdict is a safe `Partial(CrossFileResolutionNeeded)`
//! (B4 conservative default).
//!
//! Unix-only: the verbs read arbitrary on-disk paths through `openat2`-guarded
//! dirfds, which have no Windows analogue in Sub-phase A (Windows `validate_paths`
//! GA is tracked separately — DSV out-of-scope).
#![cfg(unix)]

use std::collections::HashMap;
use std::io;
use std::os::fd::BorrowedFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use anvil_checks::antipattern::types::AntipatternCheckConfig;
use anvil_intercept_proto::protocol::{
    AssuranceState, RequestFullScanRequest, RequestFullScanResponse, StaleReason,
    ValidatePathsRequest, ValidatePathsResponse, WorkspaceAssurance, WorkspaceStatusRequest,
    WorkspaceStatusResponse,
};
use anvil_kernel_types::FileSymbols;

use crate::assurance::{AssuranceMachine, ScanPriority};
use crate::confinement::Confinement;
use crate::ipc::{SaveTimeDispatch, SaveTimeError};
use crate::kernel_cache::KernelGraphCache;
use crate::path_safety::{normalise_rel, read_under};
use crate::rule_cache::WorktreeKey;
use crate::validate_paths::{ValidateEnv, validate_paths as run_validate_paths};
use crate::workspace_admission::AdmittedRoots;
use crate::workspace_pool::{DosCaps, WorkScheduler};

/// The reverse-impact certify budget for the interactive verdict path. Bounds
/// the importer-closure walk per certify so a pathological fan-out cannot stall
/// the interactive pool; an overflow degrades to `Partial(ImpactSetOverflow)`,
/// which is safe. The `DoS` parse-size / walk-depth caps are DSV-006 (Task 11).
const SAVE_TIME_CERTIFY_BUDGET: usize = 256;

/// Turns a file's bytes into [`FileSymbols`]. Injected (dependency-inverted) so
/// the daemon obtains parsed symbols WITHOUT linking a parser (ADR-064): this
/// trait is defined in the daemon crate, the tree-sitter-backed impl lives in
/// `anvil-cli` and is wired in via `ForegroundOpts`.
///
/// ## Integration-pattern framing (EIP)
///
/// This is a **Messaging Gateway**: the daemon codes against the domain method
/// (`parse(bytes) → symbols`) and only the injected impl knows the transport
/// (an in-process call today; a future out-of-process parser service could sit
/// behind the same trait without touching the daemon). The verdict path uses it
/// as a **Content Enricher by Computation** — the daemon's save-time change
/// message lacks the parsed symbols, so it augments *the message it already
/// holds* (the guarded bytes it read and hashed) by computing them here. That
/// "enrich the message you hold" property is what makes the verdict race-free:
/// the daemon hands the impl the **exact** openat2-guarded bytes it attested, so
/// there is no second read that could race the edit (a B2 false-attestation
/// hazard). A push-based watcher feed that enriched from its *own* earlier read
/// would be a different message and is therefore only ever an advisory
/// cache-warmer, never the attestation source.
///
/// ## Symbol-id contract
///
/// The impl MUST assign symbol ids from a path-stable, collision-resistant base
/// (not the parser's default per-file 0-based ids) so re-parsing a file yields
/// matching ids and distinct files do not collide in the warm graph.
///
/// `Debug` so the injecting `ForegroundOpts` (which derives `Debug`) can hold an
/// `Arc<dyn SymbolParser>`; impls are expected to be simple/stateless.
pub trait SymbolParser: Send + Sync + std::fmt::Debug {
    /// Parse `bytes` (the guarded content of `path`) into symbols, or `None`
    /// when the language is unsupported or the parse fails — a `None` keeps the
    /// verdict a safe `Partial`.
    fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols>;
}

/// Shared, cross-connection save-time state. Held in an `Arc` on the
/// `IpcListener` and cloned per connection; every interior field is safe to
/// share (`KernelGraphCache` and the assurance map carry their own locks, the
/// pools are `Sync`, the config + confinement are immutable).
pub struct SaveTimeState {
    /// The warm per-`WorktreeKey` `(SymbolGraph, DependencyGraph)` cache.
    cache: KernelGraphCache,
    /// The per-`WorktreeKey` workspace-assurance state machines. Each machine
    /// sits behind its **own** lock so a verdict on one worktree (which holds
    /// its machine lock across the antipattern scan) does not serialise verdicts
    /// on other worktrees; the outer map lock is held only to fetch/insert the
    /// per-key handle. Same-worktree verdicts still serialise (correct — one
    /// in-flight verdict per worktree).
    assurance: Mutex<HashMap<WorktreeKey, Arc<Mutex<AssuranceMachine>>>>,
    /// Antipattern check configuration (patterns, extensions, threshold).
    config: AntipatternCheckConfig,
    /// The two cooperating rayon pools; the antipattern scan runs on the
    /// interactive pool (B7), never the global one.
    scheduler: WorkScheduler,
    /// The operator confinement policy a per-connection admitted-root set is
    /// built from (open by default).
    confinement: Confinement,
    /// The injected kernel-backed parser. `None` (the default) ⇒ `fed_symbols`
    /// yields `None` and every verdict stays a safe `Partial` (the daemon never
    /// parses on its own); a `Some` is wired from `anvil-cli`.
    parser: Option<Arc<dyn SymbolParser>>,
    /// Per-workspace `DoS` caps (DSV-006 / Task 11): the parse-size cap the
    /// verdict path enforces per file. Daemon-level policy, immutable once
    /// built; defaults to [`DosCaps::default`] (a future operator-config
    /// surface can override it — the same deferred surface noted for the
    /// antipattern config).
    caps: DosCaps,
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
            parser: None,
            caps: DosCaps::default(),
        }
    }

    /// Inject the kernel-backed [`SymbolParser`] (dependency-inverted from
    /// `anvil-cli`). Without it, verdicts stay `Partial`.
    #[must_use]
    pub fn with_parser(mut self, parser: Arc<dyn SymbolParser>) -> Self {
        self.parser = Some(parser);
        self
    }

    /// Override the per-workspace [`DosCaps`] (DSV-006 / Task 11). Defaults to
    /// [`DosCaps::default`]; this is the seam a future operator-config surface
    /// (and tests) set them through.
    #[must_use]
    pub fn with_caps(mut self, caps: DosCaps) -> Self {
        self.caps = caps;
        self
    }

    /// Whether a parser is wired (used by `run_foreground` to warn on a
    /// daemon that will only ever return `Partial`).
    #[must_use]
    pub fn has_parser(&self) -> bool {
        self.parser.is_some()
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
        self.lock_map().remove(key);
    }

    #[allow(clippy::type_complexity)]
    fn lock_map(
        &self,
    ) -> std::sync::MutexGuard<'_, HashMap<WorktreeKey, Arc<Mutex<AssuranceMachine>>>> {
        // The map critical section is allocation-only (entry/insert/remove) and
        // does not panic, so a poisoned lock cannot leave a half-mutated map —
        // recover the guard rather than propagate the poison (mirrors
        // `workspace_pool`'s rationale).
        self.assurance
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Run `f` against the worktree's assurance machine, inserting a fresh one
    /// (`Stale(CrossFileResolutionNeeded)`, B6) on first contact, and emit a
    /// transition record (DSV-005 Task 9) when `f` changed the coarse state or
    /// staleness reason.
    ///
    /// The outer map lock is held only long enough to fetch the per-key handle;
    /// `f` (which may run the antipattern scan) executes under the **per-key**
    /// machine lock, so verdicts on distinct worktrees proceed in parallel. The
    /// transition record is emitted *after* the machine lock is released.
    fn with_machine<R>(&self, key: &WorktreeKey, f: impl FnOnce(&mut AssuranceMachine) -> R) -> R {
        let machine = {
            let mut guard = self.lock_map();
            // `Arc<Mutex<AssuranceMachine>>::default()` ⇒ a fresh machine, i.e.
            // `Stale(CrossFileResolutionNeeded)` (B6), on first contact.
            Arc::clone(guard.entry(key.clone()).or_default())
        };
        let (result, transition) = {
            let mut machine = machine.lock().unwrap_or_else(PoisonError::into_inner);
            let before = machine.snapshot();
            // The scan start time is cleared by a transition *out* of Running
            // (`complete_scan`/`mark_stale`), so capture it both sides and keep
            // whichever is set: the new start on `→Running`, else the start the
            // ending scan had been running with.
            let before_started = machine.scan_started_at().map(str::to_string);
            let result = f(&mut machine);
            let after = machine.snapshot();
            let scan_started_at = machine
                .scan_started_at()
                .map(str::to_string)
                .or(before_started);
            let transition = transition_between(&before, &after, scan_started_at);
            (result, transition)
        };
        if let Some(transition) = transition {
            emit_assurance_transition(key.as_path(), &transition);
        }
        result
    }
}

/// One observed workspace-assurance transition. The coarse `from`/`to` states
/// and `grouping` ride the ADR-035 [`NotificationEnvelope`](crate::telemetry)
/// (built by `telemetry::envelope_for_assurance_transition`); the precise
/// machine fields here ride the mirrored `tracing` event only — they are kept
/// off the wire `NotificationContext` until that struct + `redact_envelope` are
/// extended (Task 9 Cond A).
struct AssuranceTransition {
    from: AssuranceState,
    to: AssuranceState,
    reason: Option<StaleReason>,
    generation: u64,
    scan_started_at: Option<String>,
}

/// A transition is observed when the coarse `state` OR the staleness `reason`
/// changed across `f` — a same-state same-reason verdict (e.g. a re-stale with
/// the identical cause) is not a transition and emits nothing.
fn transition_between(
    before: &WorkspaceAssurance,
    after: &WorkspaceAssurance,
    scan_started_at: Option<String>,
) -> Option<AssuranceTransition> {
    (before.state != after.state || before.reason != after.reason).then_some(AssuranceTransition {
        from: before.state,
        to: after.state,
        reason: after.reason,
        generation: after.generation,
        scan_started_at,
    })
}

/// Emit the mirrored `tracing` event for one assurance transition. The
/// structured ADR-035 envelope (the operator-facing surface) is built from the
/// same `from`/`to`/`reason` by `telemetry::envelope_for_assurance_transition`
/// and routed through the fanout when the Phase E producer wire-up reads it;
/// the machine fields below ride this event so they never cross the envelope
/// wire prematurely (Cond A).
///
/// Phase E note: routing the envelope to subscribers requires threading the
/// connection's session correlation in (the fanout default-denies an envelope
/// with no `originating_session_id`); this tracing mirror is same-uid-local and
/// carries no such requirement.
///
/// Level: a coarse state change (e.g. `clean → stale`, `stale → pending`) is a
/// trust/lifecycle boundary and logs at `info`; a same-state change of only the
/// staleness *reason* (`stale → stale` with a new cause) is routine under heavy
/// editing and logs at `debug` to keep the stream readable.
fn emit_assurance_transition(workspace_root: &Path, transition: &AssuranceTransition) {
    let workspace_root = workspace_root.display();
    if transition.from == transition.to {
        tracing::debug!(
            target: "anvil_intercept::assurance",
            %workspace_root,
            state = ?transition.to,
            reason = ?transition.reason,
            generation = transition.generation,
            "workspace assurance staleness reason changed",
        );
    } else {
        tracing::info!(
            target: "anvil_intercept::assurance",
            %workspace_root,
            from = ?transition.from,
            to = ?transition.to,
            reason = ?transition.reason,
            generation = transition.generation,
            scan_started_at = transition.scan_started_at.as_deref(),
            "workspace assurance transition",
        );
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
        // Copy the shared-state reference so `state.*` reads stay disjoint from
        // the per-connection `self.admitted` field the held fd borrows.
        let state = self.state;
        let fd = authorise_root(&mut self.admitted, &state.confinement, &root)?;
        // Key on the *canonical* root so the assurance machine + warm cache key
        // on the same value `AdmittedRoots` admitted under — a symlinked or
        // non-canonical client root must not split state into two keys.
        let canonical = canonical_root(&root)?;
        let key = WorktreeKey::from_canonical(canonical.clone());
        // The pure core keys the cache off `request.workspace_root`, so feed it
        // the canonical form too (it is also the antipattern display root).
        let request = ValidatePathsRequest {
            workspace_root: canonical.to_string_lossy().into_owned(),
            paths: request.paths.clone(),
        };

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
            caps: &state.caps,
        };
        // Parse the EXACT guarded bytes the daemon read (handed in by
        // `validate_paths`) via the injected kernel-backed parser. No parser
        // wired ⇒ `None` ⇒ a safe `Partial` (B4); the daemon never parses.
        let parser = state.parser.as_deref();
        let fed_symbols =
            move |path: &str, bytes: &[u8]| parser.and_then(|p| p.parse(Path::new(path), bytes));
        let response = state.with_machine(&key, |machine| {
            run_validate_paths(
                &request,
                &state.cache,
                machine,
                read_guarded,
                fed_symbols,
                &env,
            )
        });
        tracing::debug!(
            target: "anvil_intercept::save_time",
            workspace_root = %canonical.display(),
            paths = request.paths.len(),
            coverage = ?response.coverage,
            assurance = ?response.workspace_assurance.state,
            "validate_paths verdict",
        );
        Ok(response)
    }

    fn workspace_status(
        &mut self,
        request: &WorkspaceStatusRequest,
    ) -> Result<WorkspaceStatusResponse, SaveTimeError> {
        let root = PathBuf::from(&request.workspace_root);
        let state = self.state;
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let key = WorktreeKey::from_canonical(canonical_root(&root)?);
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
        let state = self.state;
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let key = WorktreeKey::from_canonical(canonical_root(&root)?);
        let workspace_assurance = state.with_machine(&key, |machine| {
            // An explicit client request is interactive (client-blocking). This
            // queues the scan (→ `Pending`); the scan *executor* that drives
            // `Pending → Running → Clean` is DSV-006 (Task 16) — until it lands
            // the worktree stays `Pending` (a restart demotes it to `Stale`).
            machine.request_full_scan(ScanPriority::Interactive);
            machine.snapshot()
        });
        Ok(RequestFullScanResponse {
            workspace_assurance,
        })
    }
}

/// Canonicalise an already-admitted root for use as the assurance/cache key.
/// The root resolved at admission, so a failure here is an internal error
/// (a race that removed the root between admission and keying).
fn canonical_root(root: &Path) -> Result<PathBuf, SaveTimeError> {
    std::fs::canonicalize(root).map_err(SaveTimeError::Io)
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
    use anvil_graph_cache::certify::ChangeKind;
    use anvil_intercept_proto::protocol::{
        AssuranceState, ChangeDescriptor, ChangeKindWire, Coverage, StaleReason,
    };
    use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};
    use std::fs;

    fn state() -> SaveTimeState {
        SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        )
    }

    /// A `FileSymbols` for `file` exporting public functions `names`, ids from
    /// `base` (mirrors the `validate_paths` test helper).
    fn file_symbols(file: &str, names: &[&str], base: u64) -> FileSymbols {
        FileSymbols {
            file: file.to_string(),
            symbols: names
                .iter()
                .enumerate()
                .map(|(i, n)| SymbolNode {
                    id: base + i as u64,
                    kind: SymbolKind::Function,
                    name: (*n).to_string(),
                    visibility: Visibility::Public,
                    file: file.to_string(),
                    trust_level: TrustLevel::Unknown,
                })
                .collect(),
            imports: Vec::new(),
        }
    }

    /// A parser that hands back a fixed surface for `file` regardless of bytes —
    /// stands in for the kernel-backed parser so the consumption path is testable
    /// without tree-sitter.
    #[derive(Debug)]
    struct FixedParser {
        file: String,
        names: Vec<String>,
    }

    impl SymbolParser for FixedParser {
        fn parse(&self, _path: &Path, _bytes: &[u8]) -> Option<FileSymbols> {
            let names: Vec<&str> = self.names.iter().map(String::as_str).collect();
            Some(file_symbols(&self.file, &names, 0))
        }
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

    /// DSV: the registry unregister hook's closure body — drop a worktree's
    /// warm graph cache + assurance machine — actually reclaims both. A
    /// `validate_paths` with a parser warms the cache and seeds the assurance
    /// machine for the canonical key; `invalidate` (what `run_foreground`'s
    /// hook calls) must leave neither behind.
    #[test]
    fn invalidate_reclaims_warm_cache_and_assurance_machine() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        fs::create_dir(&src).expect("mkdir");
        fs::write(src.join("a.ts"), b"export function foo() { return 1; }").expect("write");

        let state = state().with_parser(Arc::new(FixedParser {
            file: "src/a.ts".to_string(),
            names: vec!["foo".to_string()],
        }));
        let mut conn = SaveTimeConn::new(&state);
        let request = ValidatePathsRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            paths: vec![modified("src/a.ts")],
        };
        conn.validate_paths(&request).expect("admitted");

        // The cache + assurance machine key on the CANONICAL root — the same
        // value the registry's unregister hook reconstructs from the canonical
        // worktree path.
        let key = WorktreeKey::from_canonical(
            std::fs::canonicalize(tmp.path()).expect("canonicalize root"),
        );
        assert!(
            state.cache.contains(&key),
            "validate_paths must warm the graph cache for the canonical key",
        );
        assert!(
            state.lock_map().contains_key(&key),
            "validate_paths must seed the assurance machine for the canonical key",
        );

        // Fire the hook's closure body.
        state.invalidate(&key);

        assert!(
            !state.cache.contains(&key),
            "invalidate must drop the warm cache entry",
        );
        assert!(
            !state.lock_map().contains_key(&key),
            "invalidate must drop the assurance machine",
        );
    }

    /// A parser that records the (path, bytes) it was handed, so a test can
    /// prove the daemon forwards the exact guarded bytes it read.
    #[derive(Debug, Default)]
    struct CapturingParser {
        seen: Mutex<Vec<(String, Vec<u8>)>>,
    }

    impl SymbolParser for CapturingParser {
        fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols> {
            self.seen
                .lock()
                .unwrap()
                .push((path.to_string_lossy().into_owned(), bytes.to_vec()));
            None
        }
    }

    /// The parser is handed the **exact** bytes the daemon read and hashed —
    /// the Content Enricher "enrich the message you hold" property (no second
    /// read that could race the edit).
    #[test]
    fn parser_receives_the_exact_guarded_bytes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        fs::create_dir(&src).expect("mkdir");
        let body = b"export const value = 41;".to_vec();
        fs::write(src.join("a.ts"), &body).expect("write");

        let capture = Arc::new(CapturingParser::default());
        let state = state().with_parser(capture.clone());
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .validate_paths(&ValidatePathsRequest {
                workspace_root: tmp.path().to_string_lossy().into_owned(),
                paths: vec![modified("src/a.ts")],
            })
            .expect("admitted");

        let seen = capture.seen.lock().unwrap();
        assert_eq!(seen.len(), 1, "the parser was invoked for the one path");
        assert_eq!(seen[0].0, "src/a.ts");
        assert_eq!(seen[0].1, body, "the parser got the bytes the daemon read");
        // And those bytes are the ones echoed as the daemon-computed hash.
        assert_eq!(
            resp.evaluated[0].content_hash.as_deref(),
            Some(crate::validate_paths::content_hash(&body).as_str()),
        );
    }

    /// With an injected parser delivering a matching (body-only) surface over
    /// the warm cache, a clean edit certifies end to end through the daemon —
    /// proving the dependency-inverted feed unblocks `Certified`.
    #[test]
    fn validate_certifies_when_parser_feeds_matching_surface() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        fs::create_dir(&src).expect("mkdir");
        fs::write(src.join("a.ts"), b"export function foo() { return 2; }").expect("write");

        let state = state().with_parser(Arc::new(FixedParser {
            file: "src/a.ts".to_string(),
            names: vec!["foo".to_string()],
        }));
        // Pre-warm the cache with the prior surface (foo) under the canonical
        // key the verdict will use, so a body-only re-edit is self-contained.
        let canonical = std::fs::canonicalize(tmp.path()).expect("canonical");
        let key = WorktreeKey::from_canonical(canonical);
        state.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["foo"], 0),
        );

        let mut conn = SaveTimeConn::new(&state);
        let request = ValidatePathsRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            paths: vec![modified("src/a.ts")],
        };
        let resp = conn.validate_paths(&request).expect("admitted");
        assert_eq!(
            resp.coverage,
            Coverage::Certified,
            "a self-contained body-only edit with a fed matching surface certifies",
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

    /// In allowlist mode: the primary check-in root (first verb) is implicitly
    /// admitted, an allow-listed (non-primary) root is admitted via the policy,
    /// and an unlisted non-primary root is refused before any byte is read.
    #[test]
    fn allowlist_admits_listed_root_and_refuses_unlisted() {
        let primary = tempfile::tempdir().expect("tempdir"); // not listed
        let allowed = tempfile::tempdir().expect("tempdir"); // listed exact
        let unlisted = tempfile::tempdir().expect("tempdir"); // not listed
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

        let status = |conn: &mut SaveTimeConn, dir: &tempfile::TempDir| {
            conn.workspace_status(&WorkspaceStatusRequest {
                workspace_root: dir.path().to_string_lossy().into_owned(),
            })
        };

        // The first verb's root becomes the implicitly-admitted primary, even
        // though it is not on the allow list.
        status(&mut conn, &primary).expect("primary check-in root is implicitly admitted");
        // An explicitly allow-listed root is admitted — the primary use case for
        // allowlist mode (this exercises AdmittedRoots::authorise's allow path).
        status(&mut conn, &allowed).expect("an allow-listed root is admitted");
        // A root that is neither the primary nor on the allow list is refused.
        let refused = status(&mut conn, &unlisted);
        assert!(
            matches!(refused, Err(SaveTimeError::NotAdmitted)),
            "an unlisted, non-primary root must be refused: {refused:?}",
        );
    }

    fn assurance(state: AssuranceState, reason: Option<StaleReason>) -> WorkspaceAssurance {
        WorkspaceAssurance {
            state,
            reason,
            generation: 0,
            last_full_scan: None,
        }
    }

    /// DSV-005 Task 9: a state change OR a reason change is a transition; a
    /// same-state, same-reason verdict emits nothing.
    #[test]
    fn transition_detected_on_state_or_reason_change() {
        let stale_cross = assurance(
            AssuranceState::Stale,
            Some(StaleReason::CrossFileResolutionNeeded),
        );
        let pending = assurance(AssuranceState::Pending, None);
        let stale_overflow = assurance(AssuranceState::Stale, Some(StaleReason::ImpactSetOverflow));

        // State change → transition.
        let t = transition_between(&stale_cross, &pending, None).expect("state change");
        assert_eq!(t.from, AssuranceState::Stale);
        assert_eq!(t.to, AssuranceState::Pending);

        // Same state, different reason → transition (the cause changed); both
        // ends stay Stale.
        let t = transition_between(&stale_cross, &stale_overflow, None)
            .expect("reason change is a transition");
        assert_eq!(t.from, AssuranceState::Stale);
        assert_eq!(t.to, AssuranceState::Stale);
        assert_eq!(t.reason, Some(StaleReason::ImpactSetOverflow));

        // No change → no transition.
        assert!(
            transition_between(&stale_cross, &stale_cross, None).is_none(),
            "an unchanged verdict is not a transition",
        );
    }
}
