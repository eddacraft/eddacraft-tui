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
//! grows on first contact in open mode. All reads go through the held
//! [`WorkspaceAnchor`], so a refused root never reaches the filesystem and an
//! admitted root cannot be retargeted after admission (security C2/C3 — see
//! [`crate::workspace_admission`]).
//!
//! ## Symbols feed ([`SymbolParser`])
//!
//! To certify, the verdict needs the edited file's parsed [`FileSymbols`]. The
//! daemon never parses (ADR-064); instead it enriches the change it holds by
//! computing symbols through an injected [`SymbolParser`] (a Messaging Gateway —
//! the tree-sitter impl lives in `anvil-cli`), handing it the **exact**
//! anchor-guarded bytes it read and hashed. When no parser is injected the feed
//! yields `None` and every verdict is a safe `Partial(CrossFileResolutionNeeded)`
//! (B4 conservative default).
//!
//! Cross-platform (DSV-010b / ADR-070 Stage 2): the verbs read arbitrary
//! on-disk paths a client names through a held [`WorkspaceAnchor`] — a Unix
//! `openat2`-guarded dirfd or a Windows directory-handle + `OBJ_DONT_REPARSE`
//! ladder (ADR-068). The verdict spine is platform-neutral; only the anchor's
//! read primitive is platform-split, behind the one type.
#![cfg(any(unix, windows))]

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use anvil_checks::antipattern::types::AntipatternCheckConfig;
use anvil_gctx_egress::GctxProjector;
use anvil_gctx_types::{
    FindDependentsOutcome, FindDependentsQuery, SearchSymbolsOutcome, SearchSymbolsQuery,
};
use anvil_graph_cache::clamp_reverse_impact_depth;
use anvil_intercept_proto::protocol::{
    AssuranceState, GctxFindDependentsRequest, GctxFindDependentsResponse,
    GctxSearchSymbolsRequest, GctxSearchSymbolsResponse, RequestFullScanRequest,
    RequestFullScanResponse, StaleReason, ValidatePathsRequest, ValidatePathsResponse,
    WorkspaceAssurance, WorkspaceStatusRequest, WorkspaceStatusResponse,
};
use anvil_kernel_types::FileSymbols;

use crate::assurance::{AssuranceMachine, ScanPriority};
use crate::broadcaster::TelemetryBroadcaster;
use crate::confinement::Confinement;
use crate::full_scan_executor::{ScanContext, ScanCoordinator, prepare_scan};
use crate::ipc::{GctxDispatch, SaveTimeDispatch, SaveTimeError};
use crate::kernel_cache::KernelGraphCache;
use crate::rule_cache::WorktreeKey;
use crate::telemetry::{TelemetryCorrelation, TelemetryEmitter};
use crate::validate_paths::{ValidateEnv, validate_paths as run_validate_paths};
use crate::workspace_admission::AdmittedRoots;
use crate::workspace_anchor::WorkspaceAnchor;
use crate::workspace_pool::{DosCaps, WorkScheduler};

/// The reverse-impact certify budget for the interactive verdict path. Bounds
/// the importer-closure walk per certify so a pathological fan-out cannot stall
/// the interactive pool; an overflow degrades to `Partial(ImpactSetOverflow)`,
/// which is safe. The `DoS` parse-size / walk-depth caps are DSV-006 (Task 11).
const SAVE_TIME_CERTIFY_BUDGET: usize = 256;

/// Environment variable exposing the GV2-026 reverse-impact hop-depth lever
/// (ADR-063 §3). Parsed to a `u32` and clamped into `1..=MAX_REVERSE_IMPACT_DEPTH`;
/// unset or unparseable folds to the 1-hop default.
const REVERSE_IMPACT_DEPTH_ENV: &str = "ANVIL_REVERSE_IMPACT_DEPTH";

/// The default reverse-impact hop depth when the lever is unset (ADR-063 §3:
/// "Default 1 hop", the ADR-061 §6 certifiability closure).
const DEFAULT_REVERSE_IMPACT_DEPTH: u32 = 1;

/// Resolve the reverse-impact hop-depth lever from a raw env value (ADR-063 §3).
///
/// Pure so the resolution is unit-testable without mutating process env: an
/// unset (`None`) or unparseable value yields the 1-hop default, and the parsed
/// value is clamped into `1..=MAX_REVERSE_IMPACT_DEPTH` via
/// [`clamp_reverse_impact_depth`] — an over-cap setting is clamped, not honoured.
/// Resolved once per save where the budget is resolved; never re-read on the hot
/// loop.
#[must_use]
fn resolve_reverse_impact_depth(raw: Option<&str>) -> u32 {
    let requested = raw
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_REVERSE_IMPACT_DEPTH);
    clamp_reverse_impact_depth(requested)
}

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
    /// The warm per-`WorktreeKey` `(SymbolGraph, DependencyGraph)` cache. Behind
    /// an `Arc` so the DSV-045 full-scan executor's background job can share it
    /// (`apply_delta` is internally locked).
    cache: Arc<KernelGraphCache>,
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
    /// DSV-044: production telemetry fanout. When present, assurance transitions
    /// emit the same envelope as the tracing mirror through `Fanout::route`.
    broadcaster: Option<Arc<TelemetryBroadcaster>>,
    telemetry: Mutex<TelemetryEmitter>,
    /// DSV-045 (ADR-085): per-key full-scan coalescing + cancel coordination.
    /// Shared (cheap clone) into each spawned [`ScanContext`].
    coordinator: ScanCoordinator,
    /// DSV-030 (ADR-069): the warm-graph snapshot directory when persistence is
    /// enabled (`ANVIL_PERSIST_GRAPH` on + a resolvable state dir), else `None`
    /// (default-off). `None` makes every persistence operation a no-op, so the
    /// daemon's behaviour is byte-for-byte today's rebuild-on-restart.
    snapshot_dir: Option<PathBuf>,
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
        let cache = KernelGraphCache::new();
        // GV2-027: record which resident backing the daemon certifies against at
        // startup — `gv2-hotindex-v1` after the A→A′ swap. Wire-invisible; this
        // is the one diagnostic surface for the otherwise-internal marker.
        tracing::debug!(
            backing = cache.backing_schema_version(),
            "save-time graph cache initialised"
        );
        Self {
            cache: Arc::new(cache),
            assurance: Mutex::new(HashMap::new()),
            config,
            scheduler,
            confinement,
            parser: None,
            caps: DosCaps::default(),
            broadcaster: None,
            telemetry: Mutex::new(TelemetryEmitter::new()),
            coordinator: ScanCoordinator::new(),
            snapshot_dir: None,
        }
    }

    /// Enable warm-graph persistence (DSV-030 / ADR-069) by injecting the
    /// resolved snapshot directory. `run_foreground` calls this only when
    /// `ANVIL_PERSIST_GRAPH` is affirmative AND a state dir resolves; otherwise
    /// the daemon stays default-off (`None`) and writes nothing.
    #[must_use]
    pub fn with_snapshot_dir(mut self, dir: PathBuf) -> Self {
        self.snapshot_dir = Some(dir);
        self
    }

    /// Whether warm-graph persistence is enabled (the snapshot dir is wired).
    #[must_use]
    pub fn persistence_enabled(&self) -> bool {
        self.snapshot_dir.is_some()
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

    /// Attach the production telemetry fanout used by DSV-044 transition
    /// producers. Tests and embedded listeners can omit it and keep tracing-only
    /// behaviour.
    #[must_use]
    pub fn with_broadcaster(mut self, broadcaster: Arc<TelemetryBroadcaster>) -> Self {
        self.broadcaster = Some(broadcaster);
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

    /// The full-scan coordinator (DSV-045), so an interactive `validate_paths`
    /// can preempt an in-flight background scan for the worktree.
    #[must_use]
    pub(crate) fn scan_coordinator(&self) -> &ScanCoordinator {
        &self.coordinator
    }

    /// Build a [`ScanContext`] for the DSV-045 executor from the shared
    /// collaborators — a cheap clone of the cache `Arc`, the injected parser,
    /// the `DoS` caps, and the coordinator.
    #[must_use]
    pub(crate) fn scan_context(&self) -> ScanContext {
        ScanContext::new(
            Arc::clone(&self.cache),
            self.parser.clone(),
            self.caps,
            self.coordinator.clone(),
            self.snapshot_dir.clone(),
        )
    }

    /// DSV-030 (ADR-069 §3): on a cold-key GCTX first contact, restore the warm
    /// graph from a snapshot on the **background** pool (the snapshot load is disk
    /// I/O — ADR-063 classes it background-only, never on the save-time hot path)
    /// so reads are served the restored (stale) graph rather than `NotReady` while
    /// a reconcile is pending. No-op when persistence is off, the key is already
    /// warm, or a scan is enqueued. The restored entry is a **read-only stand-in**:
    /// the machine stays `Stale`, and the reconcile full scan is disk-authoritative
    /// (the restored entry is dropped before the rebuild).
    pub(crate) fn spawn_restore(&self, key: &WorktreeKey, canonical_root: &Path) {
        #[cfg(unix)]
        {
            let Some(dir) = self.snapshot_dir.clone() else {
                return;
            };
            if self.cache.contains(key) || self.coordinator.is_enqueued(key) {
                return;
            }
            let cache = Arc::clone(&self.cache);
            let coordinator = self.coordinator.clone();
            let key = key.clone();
            let root = canonical_root.to_path_buf();
            self.scheduler.background().spawn(move || {
                restore_snapshot_into_cache(&cache, &coordinator, &dir, &key, &root);
            });
        }
        #[cfg(not(unix))]
        {
            let _ = (key, canonical_root);
        }
    }

    /// Persist every warm worktree's graph on graceful shutdown (DSV-030 /
    /// ADR-069 §4 — "written … on graceful daemon shutdown"). No-op when
    /// persistence is off. Best-effort per key; a failure logs and continues.
    pub fn persist_all_on_shutdown(&self) {
        #[cfg(unix)]
        {
            let Some(dir) = self.snapshot_dir.as_deref() else {
                return;
            };
            let mut written = 0usize;
            let mut failed = 0usize;
            for key in self.cache.warm_keys() {
                let built = self.cache.with_graphs(&key, |sym, dep| {
                    anvil_graph_cache::snapshot::SnapshotPayload::from_graphs(sym, dep)
                });
                let Some(Ok(payload)) = built else {
                    continue;
                };
                match crate::snapshot_io::write_snapshot(dir, key.as_path(), &payload) {
                    Ok(()) => written += 1,
                    // ADR-069 §10: a write failure is surfaced (WARN), never silent
                    // loss — even on the shutdown flush. (The per-fleet counter +
                    // ADR-035 notification are a documented follow-up.)
                    Err(err) => {
                        failed += 1;
                        tracing::warn!(
                            target: "anvil_intercept::snapshot",
                            workspace_root = %key.as_path().display(),
                            error = %err,
                            "snapshot write failed on shutdown; skipping (no persistence for this key)",
                        );
                    }
                }
            }
            if written > 0 || failed > 0 {
                tracing::info!(
                    target: "anvil_intercept::snapshot",
                    written,
                    failed,
                    "persisted warm graph snapshots on shutdown"
                );
            }
        }
    }

    /// Sweep orphaned `*.tmp` files left by an interrupted write (ADR-069 §10),
    /// run once at daemon start. No-op when persistence is off.
    pub fn sweep_snapshot_temps_on_start(&self) {
        #[cfg(unix)]
        {
            if let Some(dir) = self.snapshot_dir.as_deref() {
                let removed = crate::snapshot_io::sweep_orphan_temps(dir);
                if removed > 0 {
                    tracing::info!(
                        target: "anvil_intercept::snapshot",
                        removed,
                        "swept orphaned snapshot temp files on start"
                    );
                }
            }
        }
    }

    /// Resolve (inserting on first contact) the per-key assurance machine handle
    /// — the same `Arc<Mutex<…>>` `with_machine` operates on — so the executor
    /// can take the brief `start_scan`/`complete_scan` lock off the hot path.
    #[must_use]
    pub(crate) fn machine_handle(&self, key: &WorktreeKey) -> Arc<Mutex<AssuranceMachine>> {
        let mut guard = self.lock_map();
        Arc::clone(guard.entry(key.clone()).or_default())
    }

    /// Decide + spawn a full-scan job for `key` if one is warranted (DSV-045).
    /// Coalesced and self-gating via [`prepare_scan`]; the job runs on the
    /// background pool so it never touches the interactive verdict budget.
    fn spawn_scan(&self, key: &WorktreeKey, root: &Path, priority: ScanPriority) {
        let ctx = self.scan_context();
        let machine = self.machine_handle(key);
        if let Some(job) = prepare_scan(&ctx, &machine, key, root, priority) {
            self.scheduler.background().spawn(move || job.run());
        }
    }

    /// Drop a worktree's warm cache + assurance machine. Wired to the registry
    /// unregister hook (DSV-040), which fires only when the **last** session
    /// for the worktree leaves, so a live peer session never has its warm state
    /// pulled out from under it.
    ///
    /// **Non-atomic by design:** the cache drop and the assurance-machine
    /// removal take two separate locks in sequence, so a concurrent
    /// `validate_paths` racing between them could re-seed a fresh machine that
    /// this call then removes. That is harmless here — it only drops a
    /// performance cache and the machine re-seeds to the most-conservative
    /// `Stale(CrossFileResolutionNeeded)` (never a falsely-clean verdict), and
    /// the worktree is fully unregistered so no client should be mid-verdict. A
    /// future correctness-bearing consumer composed into the same hook closure
    /// (e.g. MLP2-014's rule cache) must NOT assume atomicity across the two.
    pub fn invalidate(&self, key: &WorktreeKey) {
        self.cache.invalidate(key);
        self.lock_map().remove(key);
        // DSV-045: prune the executor's per-key coordination so its maps do not
        // grow one entry per worktree ever seen. Safe non-atomically with the
        // above (the worktree is fully unregistered); a still-running scan keeps
        // its `scan-enqueued` flag (its `EnqueuedGuard` resets it on exit).
        self.coordinator.forget(key);
        // DSV-030 (ADR-069 §10): the last session for this worktree left — drop
        // its on-disk snapshot too, so an unregistered worktree leaves no stale
        // cache file behind. Best-effort; a failure is logged, never fatal.
        #[cfg(unix)]
        if let Some(dir) = self.snapshot_dir.as_deref()
            && let Err(err) = crate::snapshot_io::remove_snapshot(dir, key.as_path())
        {
            tracing::warn!(
                target: "anvil_intercept::snapshot",
                workspace_root = %key.as_path().display(),
                error = %err,
                "failed to remove snapshot on unregister",
            );
        }
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
    fn with_machine<R>(
        &self,
        key: &WorktreeKey,
        correlation: Option<TelemetryCorrelation>,
        f: impl FnOnce(&mut AssuranceMachine) -> R,
    ) -> R {
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
            self.broadcast_assurance_transition(key.as_path(), &transition, correlation);
        }
        result
    }

    fn broadcast_assurance_transition(
        &self,
        workspace_root: &Path,
        transition: &AssuranceTransition,
        correlation: Option<TelemetryCorrelation>,
    ) {
        let (Some(broadcaster), Some(correlation)) = (&self.broadcaster, correlation) else {
            return;
        };
        let workspace_root = workspace_root.display().to_string();
        let mut emitter = self
            .telemetry
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let envelope = emitter.envelope_for_assurance_transition(
            correlation,
            &workspace_root,
            transition.from,
            transition.to,
            transition.reason,
        );
        let outcome = broadcaster.broadcast(&envelope);
        tracing::debug!(
            target: "anvil_intercept::assurance",
            workspace_root,
            delivered = outcome.delivered,
            dropped = outcome.dropped,
            "workspace assurance transition broadcast",
        );
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

/// Load a snapshot for `key` and restore it into `cache` for reads (DSV-030 /
/// ADR-069 §3). Re-checks the cold/not-enqueued guard (time may have passed since
/// the caller checked) so a restore never overwrites a concurrent scan's
/// authoritative graph. Marks the key restored so the next reconcile scan drops
/// the read-only entry before its disk-authoritative rebuild. Every load failure
/// is logged per §10 severity and is a no-op (cold rebuild).
#[cfg(unix)]
fn restore_snapshot_into_cache(
    cache: &KernelGraphCache,
    coordinator: &ScanCoordinator,
    dir: &Path,
    key: &WorktreeKey,
    canonical_root: &Path,
) {
    if cache.contains(key) || coordinator.is_enqueued(key) {
        return;
    }
    let payload = match crate::snapshot_io::load_snapshot(dir, canonical_root) {
        Ok(payload) => payload,
        // No snapshot is the normal first-run case; a rejected snapshot is logged
        // per ADR-069 §10 severity. Either way ⇒ cold rebuild, no-op here.
        Err(err) => {
            log_snapshot_read_error(canonical_root, &err);
            return;
        }
    };
    // A decoded-but-internally-inconsistent payload (duplicate id / dangling
    // edge) ⇒ cold rebuild, never a panic.
    let Ok((sym, dep)) = payload.into_graphs() else {
        tracing::warn!(
            target: "anvil_intercept::snapshot",
            workspace_root = %canonical_root.display(),
            "snapshot decoded but rebuild was inconsistent; cold rebuild",
        );
        return;
    };
    // Compare-and-insert: `restore` only inserts if the key is still cold, so if
    // a reconcile scan beat us to warming it we no-op rather than clobber its
    // authoritative graph. Either way the next scan's prune keeps the entry
    // disk-authoritative, so this is correctness-safe regardless of the race.
    if cache.restore(key, sym, dep) {
        tracing::info!(
            target: "anvil_intercept::snapshot",
            workspace_root = %canonical_root.display(),
            "warm-start: restored graph from snapshot (stale until reconcile)",
        );
    }
}

/// Log a snapshot read failure at the ADR-069 §10 severity: a missing snapshot
/// is the expected first-run case (DEBUG); a version/schema mismatch is an
/// expected one-time event after a schema bump (INFO); a corrupt/oversized/torn
/// body or a disk error is worth investigating (WARN). Every case is a cold
/// rebuild — the daemon never refuses to start. No path/identity bytes from the
/// snapshot are echoed.
#[cfg(unix)]
fn log_snapshot_read_error(workspace_root: &Path, err: &crate::snapshot_io::SnapshotReadError) {
    use crate::snapshot_io::SnapshotReadError;
    use anvil_graph_cache::snapshot::SnapshotLoadError;
    let workspace_root = workspace_root.display();
    match err {
        SnapshotReadError::NotFound => tracing::debug!(
            target: "anvil_intercept::snapshot",
            %workspace_root,
            "no warm-start snapshot (cold rebuild)",
        ),
        SnapshotReadError::Rejected(SnapshotLoadError::VersionMismatch {
            found_format,
            expected_format,
            found_backing,
            expected_backing,
        }) => tracing::info!(
            target: "anvil_intercept::snapshot",
            %workspace_root,
            found_format, expected_format, found_backing, expected_backing,
            "snapshot version/schema mismatch; cold rebuild (expected after a schema bump)",
        ),
        SnapshotReadError::Rejected(reason) => tracing::warn!(
            target: "anvil_intercept::snapshot",
            %workspace_root,
            reason = ?reason,
            "snapshot rejected by the integrity gate; cold rebuild",
        ),
        SnapshotReadError::Io(source) => tracing::warn!(
            target: "anvil_intercept::snapshot",
            %workspace_root,
            error = %source,
            "snapshot read failed; cold rebuild",
        ),
    }
}

/// Emit the standard `tracing` mirror for a machine transition observed by the
/// full-scan executor (DSV-045). The executor mutates the per-key
/// [`AssuranceMachine`] directly under its own brief lock (never through
/// `with_machine`, which is the client-correlated verdict path), so it routes
/// its `Pending`/`Running`/`Clean`/`Bounded`/`Stale` transitions through this
/// shared helper to keep the assurance log consistent. There is no client
/// correlation on a daemon-internal scan, so the DSV-044 broadcast (which
/// default-denies an envelope with no originating session) is intentionally not
/// invoked here — tracing only.
pub(crate) fn trace_machine_transition(
    workspace_root: &Path,
    before: &WorkspaceAssurance,
    after: &WorkspaceAssurance,
) {
    if let Some(transition) = transition_between(before, after, None) {
        emit_assurance_transition(workspace_root, &transition);
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
    originating_session: Option<OriginatingSession>,
}

#[derive(Debug, Clone)]
struct OriginatingSession {
    session_id: String,
    worktree: PathBuf,
}

impl<'a> SaveTimeConn<'a> {
    /// Open a per-connection context over the shared state.
    #[must_use]
    pub fn new(state: &'a SaveTimeState) -> Self {
        Self {
            state,
            admitted: None,
            originating_session: None,
        }
    }

    fn telemetry_correlation_for(
        session: Option<&OriginatingSession>,
        workspace_root: &Path,
    ) -> Option<TelemetryCorrelation> {
        let session = session?;
        if session.worktree != workspace_root {
            return None;
        }
        let session_id = session.session_id.clone();
        Some(TelemetryCorrelation {
            session_id: Some(session_id.clone()),
            originating_session_id: Some(session_id),
            originating_driver_id: Some(crate::telemetry::INTERCEPT_DRIVER_ID.to_string()),
            ..TelemetryCorrelation::default()
        })
    }
}

impl SaveTimeDispatch for SaveTimeConn<'_> {
    fn set_originating_session(&mut self, session_id: &str, worktree: &Path) {
        let Ok(worktree) = std::fs::canonicalize(worktree) else {
            self.originating_session = None;
            return;
        };
        self.originating_session = Some(OriginatingSession {
            session_id: session_id.to_string(),
            worktree,
        });
    }

    fn validate_paths(
        &mut self,
        request: &ValidatePathsRequest,
    ) -> Result<ValidatePathsResponse, SaveTimeError> {
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        // Copy the shared-state reference so `state.*` reads stay disjoint from
        // the per-connection `self.admitted` field the held fd borrows.
        let state = self.state;
        let anchor = authorise_root(&mut self.admitted, &state.confinement, &root)?;
        // Key on the *canonical* root so the assurance machine + warm cache key
        // on the same value `AdmittedRoots` admitted under — a symlinked or
        // non-canonical client root must not split state into two keys.
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical.clone());
        // The pure core keys the cache off `request.workspace_root`, so feed it
        // the canonical form too (it is also the antipattern display root).
        let request = ValidatePathsRequest {
            workspace_root: canonical.to_string_lossy().into_owned(),
            paths: request.paths.clone(),
        };

        // All reads go through the held anchor — the guarded bytes the
        // antipattern check scans, never a re-opened path (B7 / security C2).
        // `WorkspaceAnchor::read_rel` normalises + structurally validates the
        // path and reads it beneath the held handle (no symlink / no reparse,
        // refuse-don't-truncate over the ceiling) on whichever platform.
        let read_guarded = move |rel: &str| -> io::Result<Vec<u8>> { anchor.read_rel(rel) };

        // Resolve the GV2-026 reverse-impact hop-depth lever once per save, here
        // beside the budget (config layer) — never on the hot per-path loop. An
        // over-cap or unparseable `ANVIL_REVERSE_IMPACT_DEPTH` is clamped to the
        // 1-hop default..=hard cap envelope (ADR-063 §3).
        let reverse_impact_depth =
            resolve_reverse_impact_depth(std::env::var(REVERSE_IMPACT_DEPTH_ENV).ok().as_deref());
        let env = ValidateEnv {
            config: &state.config,
            pool: state.scheduler.interactive(),
            budget: SAVE_TIME_CERTIFY_BUDGET,
            reverse_impact_depth,
            caps: &state.caps,
        };
        // Parse the EXACT guarded bytes the daemon read (handed in by
        // `validate_paths`) via the injected kernel-backed parser. No parser
        // wired ⇒ `None` ⇒ a safe `Partial` (B4); the daemon never parses.
        //
        // DSV-042: the parse (CPU-bound tree-sitter) is offloaded onto the
        // interactive pool via `install`, the same pool the antipattern scan
        // already runs on (`env.pool`). This keeps the verdict's CPU work
        // bounded by the one interactive pool, so N concurrent agents cannot
        // each run a parse inline and oversubscribe the cores (ADR-067 / the
        // `4 agents + 1 scan` SLO). `install` blocks this connection thread —
        // already dedicated to awaiting this verdict — and runs the parse on a
        // pool thread; the parser is `Send + Sync` and is handed the SAME
        // guarded bytes (no second read → preserves the B2 no-false-attestation
        // contract). The parse runs on the calling thread's pool stack, so a
        // `None`-yielding (unsupported/failed) parse still costs only the cheap
        // pool hand-off.
        let parser = state.parser.as_deref();
        let parse_pool = state.scheduler.interactive();
        let fed_symbols = move |path: &str, bytes: &[u8]| {
            parser.and_then(|p| parse_pool.install(|| p.parse(Path::new(path), bytes)))
        };
        // DSV-045 (Decision 9): an interactive verdict preempts an in-flight
        // background scan for this worktree so it hands cores back at the next
        // chunk boundary. A no-op when no scan is running.
        state.scan_coordinator().cancel(&key);

        let response = state.with_machine(&key, correlation, |machine| {
            // DSV-045 (Decision 4): a save that lands while a scan is `Running`
            // dirties it (checked here, before the verdict can change the state),
            // so the scan's `complete_scan` fails safe to `Stale` + re-queue
            // instead of certifying a graph that may not reflect this save.
            machine.note_apply_delta();
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
        // DSV-030 (ADR-069 §3): first-contact warm-start restore (background, off
        // the hot path). Populates the cache from a snapshot so reads during the
        // reconcile scan below are served the restored (stale) graph; the scan's
        // prune keeps it disk-authoritative. No-op when persistence is off / warm.
        state.spawn_restore(&key, key.as_path());
        // DSV-045 (Decision 10): first-contact auto-warm. Opportunistic and
        // self-gating — a no-op when the worktree is already warm or a scan is
        // already enqueued; on a fresh cold key it drives the cache warm so the
        // next GCTX query / status need not wait for a manual save.
        state.spawn_scan(&key, &canonical, ScanPriority::Background);
        Ok(response)
    }

    fn workspace_status(
        &mut self,
        request: &WorkspaceStatusRequest,
    ) -> Result<WorkspaceStatusResponse, SaveTimeError> {
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);
        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());
        // DSV-030: first-contact warm-start restore (background); then the
        // DSV-045 reconcile scan.
        state.spawn_restore(&key, key.as_path());
        // DSV-045 (Decision 10): first-contact auto-warm (self-gating) — a
        // `workspace_status` against a fresh cold key kicks off a background scan.
        state.spawn_scan(&key, key.as_path(), ScanPriority::Background);
        Ok(WorkspaceStatusResponse {
            workspace_assurance,
        })
    }

    fn request_full_scan(
        &mut self,
        request: &RequestFullScanRequest,
    ) -> Result<RequestFullScanResponse, SaveTimeError> {
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);
        let workspace_assurance = state.with_machine(&key, correlation, |machine| {
            // An explicit client request is interactive (client-blocking): queue
            // the scan (→ `Pending`) and broadcast the transition with the
            // client's correlation. The DSV-045 executor below drives it
            // `Pending → Running → Clean`/`Bounded` on the background pool.
            machine.request_full_scan(ScanPriority::Interactive);
            machine.snapshot()
        });
        // DSV-045: spawn the executor job (coalesced — N requests drive one scan).
        // `spawn_scan` → `prepare_scan` re-asserts `request_full_scan` under its
        // own lock; that second call is idempotent (the machine is already
        // `Pending`, so it is a no-op) — the `with_machine` call above exists
        // solely to capture the response snapshot and broadcast the transition
        // with the client's correlation, which the lock-free executor path does
        // not carry.
        state.spawn_scan(&key, key.as_path(), ScanPriority::Interactive);
        Ok(RequestFullScanResponse {
            workspace_assurance,
        })
    }
}

impl GctxDispatch for SaveTimeConn<'_> {
    fn search_symbols(
        &mut self,
        request: &GctxSearchSymbolsRequest,
    ) -> Result<GctxSearchSymbolsResponse, SaveTimeError> {
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        // ADR-084 C3 / CE-8: admit the client-supplied root against this
        // connection's admitted-root set before any read. A hostile MCP client
        // can send an arbitrary or sibling-worktree root; this is the same gate
        // the save-time verbs use, and a refusal blocks the projection.
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        // DSV-030 (ADR-069 §3): a fresh GCTX session on a cold key kicks off a
        // background warm-start restore (off the hot path); a subsequent query is
        // then served the restored (stale) graph rather than `NotReady`. GCTX
        // does not trigger a reconcile scan, so this is the surface that benefits
        // most from persistence. No-op when persistence is off / already warm.
        state.spawn_restore(&key, key.as_path());

        // CE-7: the assurance snapshot always rides along, whether or not the
        // graph is readable.
        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());

        let outcome = gctx_search_outcome(
            state,
            &key,
            &workspace_assurance,
            &request.query,
            gctx_egress_disabled(),
        );

        // CE-10: bind telemetry to the exhaustive PII-free outcome enum plus
        // response-aggregate counts only — never symbol names, paths, or query
        // text. Rides the ADR-035 tracing pipe (the `dispatch_span` set in
        // `ipc.rs` carries the traceparent).
        let (matched, returned) = match &outcome {
            SearchSymbolsOutcome::Ready(projection) => (
                projection.redaction_summary.matched,
                projection.redaction_summary.returned,
            ),
            _ => (0, 0),
        };
        tracing::info!(
            target: "anvil_intercept::gctx",
            outcome = outcome.telemetry_outcome().as_str(),
            matched,
            returned,
            "gctx search served",
        );

        Ok(GctxSearchSymbolsResponse {
            workspace_assurance,
            outcome,
        })
    }

    fn find_dependents(
        &mut self,
        request: &GctxFindDependentsRequest,
    ) -> Result<GctxFindDependentsResponse, SaveTimeError> {
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        // ADR-084 C3 / CE-8: admit the client-supplied root before any read.
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        // CE-7: the assurance snapshot always rides along.
        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());

        let outcome = gctx_find_dependents_outcome(
            state,
            &key,
            &workspace_assurance,
            &request.query,
            gctx_egress_disabled(),
        );

        // CE-10: bind telemetry to the exhaustive PII-free outcome enum plus
        // response-aggregate counts only — never paths or query text.
        let (matched, returned) = match &outcome {
            FindDependentsOutcome::Ready(projection) => (
                projection.redaction_summary.matched,
                projection.redaction_summary.returned,
            ),
            _ => (0, 0),
        };
        tracing::info!(
            target: "anvil_intercept::gctx",
            outcome = outcome.telemetry_outcome().as_str(),
            matched,
            returned,
            "gctx find_dependents served",
        );

        Ok(GctxFindDependentsResponse {
            workspace_assurance,
            outcome,
        })
    }
}

/// CE-11 kill-switch. `ANVIL_GCTX_EGRESS` is re-read **per call** (never cached
/// at start-up): `0` disables egress on the next call; unset or any other value
/// (incl. the `1` that additionally opts into Phase-2 snippets) leaves the
/// identity surface on. The owner-confirmed default is identity-on.
const GCTX_EGRESS_ENV: &str = "ANVIL_GCTX_EGRESS";

fn gctx_egress_disabled() -> bool {
    gctx_egress_disabled_from(std::env::var(GCTX_EGRESS_ENV).ok().as_deref())
}

/// Pure kill-switch resolution (CE-11), testable without mutating process env:
/// the (whitespace-trimmed) value `0` disables; unset or any other value (incl.
/// the snippet opt-in `1`) leaves identity egress on. Trimming avoids a silent
/// fail-open when an operator sets `" 0"` or a trailing-newline `"0\n"`.
fn gctx_egress_disabled_from(raw: Option<&str>) -> bool {
    raw.map(str::trim) == Some("0")
}

/// Compute the GCTX search outcome for an admitted root (GCTX-010 / ADR-084).
///
/// CE-7 degradation, by assurance state: `Unavailable` → `Unavailable`;
/// `Pending`/`Running` (warming) → `NotReady`; `Clean`/`Stale` → read the warm
/// graph. A `Clean`/`Stale` worktree with no resident warm pair (a fresh session
/// not yet save-populated — the cache and the assurance machine are non-atomic
/// by design) also degrades to `NotReady`: there is **no whole-file fallback**.
///
/// ADR-084 C2: the matched candidates are collected **under** the cache lock
/// (inside `with_graphs`) and sorted/paginated/sealed **after** it releases.
fn gctx_search_outcome(
    state: &SaveTimeState,
    key: &WorktreeKey,
    assurance: &WorkspaceAssurance,
    query: &SearchSymbolsQuery,
    egress_disabled: bool,
) -> SearchSymbolsOutcome {
    // CE-11 kill-switch (resolved per call by the caller): an operator-disabled
    // surface egresses nothing — no query validation, no graph read — and
    // self-reports `Disabled`.
    if egress_disabled {
        return SearchSymbolsOutcome::Disabled;
    }

    // CE-6: reject a hostile or malformed query before touching the graph.
    if let Some(reason) = invalid_query_reason(query) {
        return SearchSymbolsOutcome::InvalidQuery { reason };
    }

    match assurance.state {
        // `Unknown` is the deser-only forward-compat fallback (ADR-085 Decision
        // 5b) — the local machine never produces it, but a consumer MUST treat
        // it fail-safe; here that means the same "no trustworthy graph" answer
        // as a daemon-absent surface.
        AssuranceState::Unavailable | AssuranceState::Unknown => SearchSymbolsOutcome::Unavailable,
        AssuranceState::Pending | AssuranceState::Running => SearchSymbolsOutcome::NotReady {
            recovery_hint: "the workspace graph is warming; retry the search shortly".to_string(),
        },
        // DSV-045: `Bounded` is a *populated* (warm-but-truncated) graph — read
        // like `Clean`/`Stale`, never demoted to `NotReady` (ADR-085 Decision
        // 5). The bounded-result truncation marker on the projection is a
        // GCTX-010 consumer concern; the assurance snapshot already carries
        // `scan_coverage` so the client can surface the bound.
        AssuranceState::Clean | AssuranceState::Stale | AssuranceState::Bounded => {
            // C2: collect under the lock, project after release.
            let candidates = state.cache.with_graphs(key, |sym, _dep| {
                GctxProjector::collect_candidates(sym, query)
            });
            match candidates {
                // A malformed / cross-query pagination cursor surfaces here as a
                // structured `InvalidQuery` (CE-6).
                Some(candidates) => match GctxProjector::project(candidates, query) {
                    Ok(projection) => SearchSymbolsOutcome::Ready(projection),
                    Err(reason) => SearchSymbolsOutcome::InvalidQuery { reason },
                },
                None => SearchSymbolsOutcome::NotReady {
                    recovery_hint: concat!(
                        "the workspace graph is not yet populated; ",
                        "save a file or request a full scan to warm it"
                    )
                    .to_string(),
                },
            }
        }
    }
}

/// Compute the GCTX dependents outcome for an admitted root (GCTX-011 / ADR-084).
///
/// Mirrors [`gctx_search_outcome`] exactly for the kill-switch, query-validation,
/// and CE-7 degradation arms — the difference is the read primitive: a
/// depth-bounded reverse-impact (dependents) walk over the warm
/// [`anvil_graph_cache::DependencyGraph`] instead of a symbol scan. The traversal
/// `depth` is resolved through the GV2-026 [`clamp_reverse_impact_depth`] lever
/// (an over-cap `max_depth` is clamped, not honoured); absent defaults to one hop.
///
/// ADR-084 C2: candidates are collected **under** the cache lock (inside
/// `with_graphs`) and sorted/paginated/sealed **after** it releases.
fn gctx_find_dependents_outcome(
    state: &SaveTimeState,
    key: &WorktreeKey,
    assurance: &WorkspaceAssurance,
    query: &FindDependentsQuery,
    egress_disabled: bool,
) -> FindDependentsOutcome {
    // CE-11 kill-switch.
    if egress_disabled {
        return FindDependentsOutcome::Disabled;
    }

    // CE-6: reject a hostile or malformed query before touching the graph.
    if let Some(reason) = invalid_find_dependents_query_reason(query) {
        return FindDependentsOutcome::InvalidQuery { reason };
    }
    // A dependents walk needs a target file; absence is a structured rejection
    // (the egress projector has no meaningful "all dependents" answer).
    let Some(file) = query.file.as_deref() else {
        return FindDependentsOutcome::InvalidQuery {
            reason: "file is required".to_string(),
        };
    };
    let file = file.to_string();

    // GV2-026 lever: clamp the requested depth into `1..=MAX_REVERSE_IMPACT_DEPTH`
    // (an unset / over-cap value folds to the default floor / hard ceiling).
    let depth = clamp_reverse_impact_depth(query.max_depth.unwrap_or(1));

    match assurance.state {
        AssuranceState::Unavailable | AssuranceState::Unknown => FindDependentsOutcome::Unavailable,
        AssuranceState::Pending | AssuranceState::Running => FindDependentsOutcome::NotReady {
            recovery_hint: "the workspace graph is warming; retry the traversal shortly"
                .to_string(),
        },
        AssuranceState::Clean | AssuranceState::Stale | AssuranceState::Bounded => {
            // C2: collect under the lock, project after release.
            let candidates = state.cache.with_graphs(key, |_sym, dep| {
                GctxProjector::collect_dependents(dep, &file, depth)
            });
            match candidates {
                Some(candidates) => {
                    match GctxProjector::project_dependents(candidates, query, depth) {
                        Ok(projection) => FindDependentsOutcome::Ready(projection),
                        // A malformed / cross-query pagination cursor (CE-6).
                        Err(reason) => FindDependentsOutcome::InvalidQuery { reason },
                    }
                }
                None => FindDependentsOutcome::NotReady {
                    recovery_hint: concat!(
                        "the workspace graph is not yet populated; ",
                        "save a file or request a full scan to warm it"
                    )
                    .to_string(),
                },
            }
        }
    }
}

/// CE-6 query hygiene for `find_dependents`. Like [`invalid_query_reason`], it
/// rejects a malformed `file` **before** the graph is read: a per-param byte cap,
/// no NUL, no absolute path (Unix or Windows-drive), no `..` traversal component,
/// and no scheme prefix. `max_depth` is clamped (not rejected); `cursor` validity
/// is checked in the projector. Returns the rejection reason, or `None`.
fn invalid_find_dependents_query_reason(query: &FindDependentsQuery) -> Option<String> {
    // `file` is the one required field: a dependents walk has no meaningful
    // "all files" answer. Reject an absent or empty value here (not via a
    // downstream guard's ordering) so the validation is self-contained — a direct
    // IPC client cannot reach the graph read with a `Some("")` that would resolve
    // to an empty `Ready` instead of a structured `InvalidQuery`.
    let Some(file) = query.file.as_deref() else {
        return Some("file is required".to_string());
    };
    if file.is_empty() {
        return Some("file must not be empty".to_string());
    }
    if file.len() > MAX_FILTER_BYTES {
        return Some(format!("file filter exceeds {MAX_FILTER_BYTES} bytes"));
    }
    if file.contains('\0') {
        return Some("file filter must not contain a NUL byte".to_string());
    }
    if Path::new(file).is_absolute() || has_windows_drive_absolute_prefix(file) {
        return Some("file filter must be a workspace-relative path".to_string());
    }
    if Path::new(file)
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Some("file filter must not contain a `..` component".to_string());
    }
    if has_uri_scheme_prefix(file) {
        return Some("file filter must not be scheme-prefixed (e.g. npm:, https:)".to_string());
    }
    None
}

/// Per-param byte cap (GCTX-001 spec, CE-6: "≤ 512 bytes/param").
const MAX_FILTER_BYTES: usize = 512;

/// CE-6 query hygiene. Rejects malformed input with a structured reason
/// **before** the graph is queried (GCTX-001 spec "Input validation").
///
/// Every string filter is capped at [`MAX_FILTER_BYTES`] and must contain no NUL
/// (defends downstream consumers and bounds per-node match work independently of
/// the IPC frame cap). The path-like `file` filter additionally rejects absolute
/// paths (Unix or Windows-drive), `..` traversal components, and scheme-prefixed
/// inputs (`npm:`, `https:`, `data:`, …). The `file` filter is a pure in-memory
/// substring match against already-relative graph paths — it never opens a file
/// — so these path checks are input hygiene and a forward guard, not a live
/// traversal defence. `limit` is clamped (not rejected); `cursor` validity is
/// checked in the projector. Returns the rejection reason, or `None` when
/// acceptable.
fn invalid_query_reason(query: &SearchSymbolsQuery) -> Option<String> {
    for (label, value) in [
        ("name", query.name.as_deref()),
        ("language", query.language.as_deref()),
        ("file", query.file.as_deref()),
    ] {
        if let Some(value) = value {
            if value.len() > MAX_FILTER_BYTES {
                return Some(format!("{label} filter exceeds {MAX_FILTER_BYTES} bytes"));
            }
            if value.contains('\0') {
                return Some(format!("{label} filter must not contain a NUL byte"));
            }
        }
    }
    if let Some(file) = query.file.as_deref() {
        if Path::new(file).is_absolute() || has_windows_drive_absolute_prefix(file) {
            return Some("file filter must be a workspace-relative path".to_string());
        }
        if Path::new(file)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Some("file filter must not contain a `..` component".to_string());
        }
        if has_uri_scheme_prefix(file) {
            return Some("file filter must not be scheme-prefixed (e.g. npm:, https:)".to_string());
        }
    }
    None
}

fn has_windows_drive_absolute_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// Whether `value` begins with a URI scheme (`scheme:` where `scheme` is ≥2
/// chars, alpha-led, `[A-Za-z0-9+.-]`). The ≥2 length excludes a single-letter
/// Windows drive (`C:`), which [`has_windows_drive_absolute_prefix`] handles.
fn has_uri_scheme_prefix(value: &str) -> bool {
    let value = value.trim_start();
    let Some(colon) = value.find(':') else {
        return false;
    };
    let scheme = &value[..colon];
    scheme.len() >= 2
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

/// Canonicalise an already-admitted root for use as the assurance/cache key.
/// The root resolved at admission, so a failure here is an internal error
/// (a race that removed the root between admission and keying).
fn canonical_root(root: &Path) -> Result<PathBuf, SaveTimeError> {
    std::fs::canonicalize(root).map_err(SaveTimeError::Io)
}

/// Authorise `root` against the connection's admitted set, building it on first
/// contact (seeded with `root` as the primary check-in root). Returns the held
/// read [`WorkspaceAnchor`]. Kept a free function over the `admitted` field (not
/// a `&mut self` method) so the returned anchor's borrow stays disjoint from the
/// caller's shared-state reads.
fn authorise_root<'f>(
    admitted: &'f mut Option<AdmittedRoots>,
    confinement: &Confinement,
    root: &Path,
) -> Result<&'f WorkspaceAnchor, SaveTimeError> {
    let set = admitted.get_or_insert_with(|| confinement.to_admitted_roots(root));
    set.authorise(root)
        .map_err(SaveTimeError::Io)?
        .ok_or(SaveTimeError::NotAdmitted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fanout::{CrossSessionPolicy, Fanout, OwnershipResolver, SubscriberId};
    use anvil_graph_cache::MAX_REVERSE_IMPACT_DEPTH;
    use anvil_graph_cache::certify::ChangeKind;
    use anvil_intercept_proto::protocol::{
        AssuranceState, ChangeDescriptor, ChangeKindWire, Coverage, StaleReason,
    };
    use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};
    use std::fs;

    // ---- GV2-026: reverse-impact hop-depth lever resolution (ADR-063 §3) ----

    #[test]
    fn resolve_reverse_impact_depth_unset_defaults_to_one_hop() {
        assert_eq!(resolve_reverse_impact_depth(None), 1);
    }

    #[test]
    fn resolve_reverse_impact_depth_parses_in_range() {
        assert_eq!(resolve_reverse_impact_depth(Some("1")), 1);
        assert_eq!(resolve_reverse_impact_depth(Some("2")), 2);
        assert_eq!(resolve_reverse_impact_depth(Some(" 2 ")), 2);
    }

    #[test]
    fn resolve_reverse_impact_depth_over_cap_is_clamped_not_honoured() {
        assert_eq!(
            resolve_reverse_impact_depth(Some("5")),
            MAX_REVERSE_IMPACT_DEPTH
        );
        assert_eq!(
            resolve_reverse_impact_depth(Some("4294967295")),
            MAX_REVERSE_IMPACT_DEPTH
        );
    }

    #[test]
    fn resolve_reverse_impact_depth_zero_or_garbage_folds_to_default() {
        assert_eq!(
            resolve_reverse_impact_depth(Some("0")),
            1,
            "0 → 1-hop floor"
        );
        assert_eq!(resolve_reverse_impact_depth(Some("")), 1, "empty → default");
        assert_eq!(
            resolve_reverse_impact_depth(Some("two")),
            1,
            "garbage → default"
        );
        assert_eq!(
            resolve_reverse_impact_depth(Some("-1")),
            1,
            "negative → default"
        );
    }

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
            reexports: Vec::new(),
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

    struct SingleOwnerResolver {
        subscriber: SubscriberId,
        session_id: String,
    }

    impl OwnershipResolver for SingleOwnerResolver {
        fn is_authorised(&self, subscriber: &SubscriberId, originating_session_id: &str) -> bool {
            subscriber == &self.subscriber && originating_session_id == self.session_id
        }
    }

    /// DSV-044: assurance transitions built by the save-time state now route
    /// through the production broadcaster, not only the tracing mirror. The
    /// registered session id is load-bearing: the fanout authorises on it.
    #[test]
    fn assurance_transition_emits_through_fanout() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let subscriber = SubscriberId::new("subscriber-A");
        let fanout = Arc::new(Fanout::with_cross_session_policy(
            Box::new(SingleOwnerResolver {
                subscriber: subscriber.clone(),
                session_id: "sess-A".to_string(),
            }),
            CrossSessionPolicy::Deny,
        ));
        let broadcaster = Arc::new(TelemetryBroadcaster::new(fanout));
        let mut rx = broadcaster.register(subscriber, None);
        let state = state().with_broadcaster(Arc::clone(&broadcaster));
        let mut conn = SaveTimeConn::new(&state);
        conn.set_originating_session("sess-A", tmp.path());

        let resp = conn
            .request_full_scan(&RequestFullScanRequest {
                workspace_root: tmp.path().to_string_lossy().into_owned(),
            })
            .expect("admitted");

        assert_eq!(resp.workspace_assurance.state, AssuranceState::Pending);
        let frame = rx.try_recv().expect("assurance transition frame queued");
        let value: serde_json::Value = serde_json::from_str(&frame).expect("frame json");
        assert_eq!(
            value["method"],
            crate::broadcaster::TELEMETRY_NOTIFICATION_METHOD
        );
        assert_eq!(
            value["params"]["correlation"]["originating_session_id"],
            "sess-A",
        );
        assert_eq!(
            value["params"]["grouping"]["transition"],
            serde_json::json!({"from": "stale", "to": "pending"}),
        );
        assert_eq!(broadcaster.dropped_envelopes(), 0);
    }

    #[test]
    fn assurance_transition_does_not_reuse_session_for_other_worktree() {
        let registered = tempfile::tempdir().expect("registered tempdir");
        let other = tempfile::tempdir().expect("other tempdir");
        let subscriber = SubscriberId::new("subscriber-A");
        let fanout = Arc::new(Fanout::with_cross_session_policy(
            Box::new(SingleOwnerResolver {
                subscriber: subscriber.clone(),
                session_id: "sess-A".to_string(),
            }),
            CrossSessionPolicy::Deny,
        ));
        let broadcaster = Arc::new(TelemetryBroadcaster::new(fanout));
        let mut rx = broadcaster.register(subscriber, None);
        let state = state().with_broadcaster(Arc::clone(&broadcaster));
        let mut conn = SaveTimeConn::new(&state);
        conn.set_originating_session("sess-A", registered.path());

        conn.request_full_scan(&RequestFullScanRequest {
            workspace_root: other.path().to_string_lossy().into_owned(),
        })
        .expect("admitted");

        assert!(
            rx.try_recv().is_err(),
            "a transition on another admitted root must not be emitted as sess-A",
        );
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

    // ---- GCTX-010 / ADR-084: identity-only graph context search ----

    fn gctx_request(root: &Path) -> GctxSearchSymbolsRequest {
        GctxSearchSymbolsRequest {
            workspace_root: root.to_string_lossy().into_owned(),
            query: SearchSymbolsQuery::default(),
        }
    }

    fn warm(state: &SaveTimeState, root: &Path, file: &str, names: &[&str], base: u64) {
        let key = WorktreeKey::from_canonical(std::fs::canonicalize(root).expect("canonical"));
        state
            .cache
            .apply_delta(&key, ChangeKind::Create, file_symbols(file, names, base));
    }

    /// A warm worktree yields a `Ready` projection with identities in a stable
    /// total order (by `SymbolIdentity`), regardless of save/insertion order.
    #[test]
    fn gctx_search_ready_orders_identities_when_warm() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        // Warm two files out of sorted order; output must be ordered a, b.
        warm(&state, tmp.path(), "src/b.ts", &["beta"], 100);
        warm(&state, tmp.path(), "src/a.ts", &["alpha"], 0);

        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .search_symbols(&gctx_request(tmp.path()))
            .expect("admitted");

        match resp.outcome {
            SearchSymbolsOutcome::Ready(projection) => {
                let files: Vec<&str> = projection
                    .symbols
                    .iter()
                    .map(|s| s.identity.file.as_str())
                    .collect();
                assert_eq!(files, ["src/a.ts", "src/b.ts"]);
                assert_eq!(projection.redaction_summary.matched, 2);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    /// CE-7: a cold worktree (no resident warm pair) degrades to `NotReady` with
    /// a recovery hint — never an empty `Ready`, never a file read.
    #[test]
    fn gctx_search_not_ready_on_cold_worktree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .search_symbols(&gctx_request(tmp.path()))
            .expect("admitted");
        assert!(
            matches!(resp.outcome, SearchSymbolsOutcome::NotReady { .. }),
            "cold worktree must degrade to NotReady: {:?}",
            resp.outcome
        );
    }

    /// CE-7: while a scan is queued (`Pending`), results are suppressed as
    /// `NotReady` even though the cache already holds a warm pair.
    #[test]
    fn gctx_search_not_ready_while_scan_pending() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "src/a.ts", &["alpha"], 0);

        // Put the worktree's machine into `Pending` directly. (`request_full_scan`
        // also reaches `Pending`, but the DSV-045 executor then drives it
        // `Running → Clean` on the background pool, which would race this
        // assertion; we assert the GCTX *state mapping* — a non-terminal scan
        // suppresses results — deterministically here.)
        let key =
            WorktreeKey::from_canonical(std::fs::canonicalize(tmp.path()).expect("canonical"));
        state
            .machine_handle(&key)
            .lock()
            .expect("machine lock")
            .request_full_scan(ScanPriority::Interactive);

        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .search_symbols(&gctx_request(tmp.path()))
            .expect("admitted");
        assert!(
            matches!(resp.outcome, SearchSymbolsOutcome::NotReady { .. }),
            "a pending scan must suppress results: {:?}",
            resp.outcome
        );
    }

    /// C3 / CE-8: an unadmitted (cross-worktree / unlisted) root is refused
    /// daemon-side before any projection.
    #[test]
    fn gctx_search_rejects_unadmitted_root() {
        let primary = tempfile::tempdir().expect("tempdir");
        let unlisted = tempfile::tempdir().expect("tempdir");
        let confinement = Confinement::from_file(crate::confinement::ConfinementConfigFile {
            admission: crate::confinement::AdmissionModeFile::Allowlist,
            allow: Vec::new(),
        });
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            confinement,
        );
        let mut conn = SaveTimeConn::new(&state);

        // The first verb's root is the implicitly-admitted primary.
        conn.search_symbols(&gctx_request(primary.path()))
            .expect("primary root is implicitly admitted");
        // A different, unlisted root is refused.
        let refused = conn.search_symbols(&gctx_request(unlisted.path()));
        assert!(
            matches!(refused, Err(SaveTimeError::NotAdmitted)),
            "an unadmitted root must be refused: {refused:?}",
        );
    }

    /// CE-6: a `file` filter that escapes the workspace is rejected as
    /// `InvalidQuery` before any read.
    #[test]
    fn gctx_search_rejects_file_filter_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let request = GctxSearchSymbolsRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            query: SearchSymbolsQuery {
                file: Some("../escape".to_string()),
                ..Default::default()
            },
        };
        let resp = conn.search_symbols(&request).expect("admitted");
        assert!(
            matches!(resp.outcome, SearchSymbolsOutcome::InvalidQuery { .. }),
            "a `..` file filter must be rejected: {:?}",
            resp.outcome
        );
    }

    /// CE-6: reject Windows drive absolute paths even on Unix runners, because
    /// the filter contract is workspace-relative across supported platforms.
    #[test]
    fn gctx_search_rejects_windows_drive_file_filter() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let request = GctxSearchSymbolsRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            query: SearchSymbolsQuery {
                file: Some("C:/escape/src/lib.rs".to_string()),
                ..Default::default()
            },
        };
        let resp = conn.search_symbols(&request).expect("admitted");
        assert!(
            matches!(resp.outcome, SearchSymbolsOutcome::InvalidQuery { .. }),
            "a Windows absolute file filter must be rejected: {:?}",
            resp.outcome
        );
    }

    fn gctx_invalid_query(root: &Path, query: SearchSymbolsQuery) -> SearchSymbolsOutcome {
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        conn.search_symbols(&GctxSearchSymbolsRequest {
            workspace_root: root.to_string_lossy().into_owned(),
            query,
        })
        .expect("admitted")
        .outcome
    }

    // --- GCTX-011 find_dependents (daemon wiring) ---

    fn dependents_request(
        root: &Path,
        file: &str,
        max_depth: Option<u32>,
    ) -> GctxFindDependentsRequest {
        GctxFindDependentsRequest {
            workspace_root: root.to_string_lossy().into_owned(),
            query: FindDependentsQuery {
                file: Some(file.to_string()),
                max_depth,
                ..Default::default()
            },
        }
    }

    /// Warm `file` with a single relative import specifier, building the
    /// `importer → imported` dependency edge the reverse-impact walk reads.
    fn warm_with_import(
        state: &SaveTimeState,
        root: &Path,
        file: &str,
        names: &[&str],
        import: &str,
        base: u64,
    ) {
        let key = WorktreeKey::from_canonical(std::fs::canonicalize(root).expect("canonical"));
        let mut symbols = file_symbols(file, names, base);
        symbols.imports = vec![anvil_kernel_types::ImportEdge {
            from_file: file.to_string(),
            to_source: import.to_string(),
            line: 0,
        }];
        state.cache.apply_delta(&key, ChangeKind::Create, symbols);
    }

    /// A warm worktree resolves the file-keyed importer set: `b.ts` importing
    /// `./a` makes `a.ts` report `b.ts` at distance 1.
    #[test]
    fn gctx_dependents_ready_reports_importer_when_warm() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        warm_with_import(&state, tmp.path(), "b.ts", &["beta"], "./a", 100);

        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .find_dependents(&dependents_request(tmp.path(), "a.ts", None))
            .expect("admitted");

        match resp.outcome {
            FindDependentsOutcome::Ready(projection) => {
                let files: Vec<(&str, u32)> = projection
                    .dependents
                    .iter()
                    .map(|d| (d.file.as_str(), d.distance))
                    .collect();
                assert_eq!(files, [("b.ts", 1)]);
                assert_eq!(projection.redaction_summary.matched, 1);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    /// A file with no importers is a `Ready` empty page (not `NotReady`).
    #[test]
    fn gctx_dependents_ready_empty_when_no_importers() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);

        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .find_dependents(&dependents_request(tmp.path(), "a.ts", None))
            .expect("admitted");
        // An empty readable result classifies as a `miss` (CE-10), not a warming.
        assert_eq!(resp.outcome.telemetry_outcome().as_str(), "miss");
        match resp.outcome {
            FindDependentsOutcome::Ready(projection) => {
                assert!(projection.dependents.is_empty());
            }
            other => panic!("expected Ready empty, got {other:?}"),
        }
    }

    /// CE-7: a cold worktree degrades to `NotReady`, never an empty `Ready`.
    #[test]
    fn gctx_dependents_not_ready_on_cold_worktree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .find_dependents(&dependents_request(tmp.path(), "a.ts", None))
            .expect("admitted");
        assert!(
            matches!(resp.outcome, FindDependentsOutcome::NotReady { .. }),
            "cold worktree must degrade to NotReady: {:?}",
            resp.outcome
        );
    }

    /// CE-11 kill-switch: a disabled surface self-reports `Disabled` even warm.
    #[test]
    fn gctx_dependents_kill_switch_disables_egress() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        let key =
            WorktreeKey::from_canonical(std::fs::canonicalize(tmp.path()).expect("canonical"));
        let clean = assurance(AssuranceState::Clean, None);

        let disabled = gctx_find_dependents_outcome(
            &state,
            &key,
            &clean,
            &FindDependentsQuery {
                file: Some("a.ts".into()),
                ..Default::default()
            },
            true,
        );
        assert!(matches!(disabled, FindDependentsOutcome::Disabled));
    }

    /// A dependents query with no `file` is a structured `InvalidQuery`.
    #[test]
    fn gctx_dependents_missing_file_is_invalid_query() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .find_dependents(&GctxFindDependentsRequest {
                workspace_root: tmp.path().to_string_lossy().into_owned(),
                query: FindDependentsQuery::default(),
            })
            .expect("admitted");
        assert!(
            matches!(resp.outcome, FindDependentsOutcome::InvalidQuery { .. }),
            "a missing file must be rejected: {:?}",
            resp.outcome
        );
    }

    /// CE-6: an empty `file` is rejected daemon-side as `InvalidQuery` — a direct
    /// IPC client cannot slip a `Some("")` past validation into an empty `Ready`.
    #[test]
    fn gctx_dependents_empty_file_is_invalid_query() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .find_dependents(&dependents_request(tmp.path(), "", None))
            .expect("admitted");
        assert!(
            matches!(resp.outcome, FindDependentsOutcome::InvalidQuery { .. }),
            "an empty file must be rejected: {:?}",
            resp.outcome
        );
    }

    /// CE-6: a `..`-escaping file is rejected before any read.
    #[test]
    fn gctx_dependents_rejects_file_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .find_dependents(&dependents_request(tmp.path(), "../escape", None))
            .expect("admitted");
        assert!(
            matches!(resp.outcome, FindDependentsOutcome::InvalidQuery { .. }),
            "a `..` file must be rejected: {:?}",
            resp.outcome
        );
    }

    /// C3 / CE-8: an unadmitted root is refused daemon-side before projection.
    #[test]
    fn gctx_dependents_rejects_unadmitted_root() {
        let primary = tempfile::tempdir().expect("tempdir");
        let unlisted = tempfile::tempdir().expect("tempdir");
        let confinement = Confinement::from_file(crate::confinement::ConfinementConfigFile {
            admission: crate::confinement::AdmissionModeFile::Allowlist,
            allow: Vec::new(),
        });
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            confinement,
        );
        let mut conn = SaveTimeConn::new(&state);
        conn.find_dependents(&dependents_request(primary.path(), "a.ts", None))
            .expect("primary root is implicitly admitted");
        let refused = conn.find_dependents(&dependents_request(unlisted.path(), "a.ts", None));
        assert!(
            matches!(refused, Err(SaveTimeError::NotAdmitted)),
            "an unadmitted root must be refused: {refused:?}",
        );
    }

    /// CE-6: a dependents page mints an opaque `next_cursor` that resumes the walk
    /// with no overlap or gap, end to end through the daemon dispatch.
    #[test]
    fn gctx_dependents_paginate_with_opaque_cursor() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        // Three direct importers of a.ts.
        warm_with_import(&state, tmp.path(), "b.ts", &["b"], "./a", 100);
        warm_with_import(&state, tmp.path(), "c.ts", &["c"], "./a", 200);
        warm_with_import(&state, tmp.path(), "d.ts", &["d"], "./a", 300);

        let mut conn = SaveTimeConn::new(&state);
        let mut seen: Vec<String> = Vec::new();
        let mut cursor = None;
        for _ in 0..5 {
            let request = GctxFindDependentsRequest {
                workspace_root: tmp.path().to_string_lossy().into_owned(),
                query: FindDependentsQuery {
                    file: Some("a.ts".into()),
                    limit: Some(2),
                    cursor: cursor.clone(),
                    ..Default::default()
                },
            };
            let resp = conn.find_dependents(&request).expect("admitted");
            let FindDependentsOutcome::Ready(projection) = resp.outcome else {
                panic!("expected Ready");
            };
            seen.extend(projection.dependents.iter().map(|d| d.file.clone()));
            match projection.next_cursor {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        assert_eq!(
            seen,
            ["b.ts", "c.ts", "d.ts"],
            "every importer exactly once"
        );
    }

    /// CE-6: a search that pages through the daemon mints an opaque `next_cursor`
    /// and the echoed cursor resumes the walk with no overlap or gap.
    #[test]
    fn gctx_search_paginates_with_opaque_cursor() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "src/a.ts", &["a"], 0);
        warm(&state, tmp.path(), "src/b.ts", &["b"], 100);
        warm(&state, tmp.path(), "src/c.ts", &["c"], 200);
        let mut conn = SaveTimeConn::new(&state);

        let page1 = conn
            .search_symbols(&GctxSearchSymbolsRequest {
                workspace_root: tmp.path().to_string_lossy().into_owned(),
                query: SearchSymbolsQuery {
                    limit: Some(2),
                    ..Default::default()
                },
            })
            .expect("admitted");
        let (first, cursor) = match page1.outcome {
            SearchSymbolsOutcome::Ready(p) => {
                assert_eq!(p.symbols.len(), 2);
                assert_eq!(p.redaction_summary.matched, 3);
                (
                    p.symbols
                        .iter()
                        .map(|s| s.identity.file.clone())
                        .collect::<Vec<_>>(),
                    p.next_cursor.expect("a second page remains"),
                )
            }
            other => panic!("expected Ready, got {other:?}"),
        };

        let page2 = conn
            .search_symbols(&GctxSearchSymbolsRequest {
                workspace_root: tmp.path().to_string_lossy().into_owned(),
                query: SearchSymbolsQuery {
                    limit: Some(2),
                    cursor: Some(cursor),
                    ..Default::default()
                },
            })
            .expect("admitted");
        match page2.outcome {
            SearchSymbolsOutcome::Ready(p) => {
                assert_eq!(p.symbols.len(), 1, "last page holds the remainder");
                assert!(p.next_cursor.is_none(), "no further pages");
                let mut all = first;
                all.extend(p.symbols.iter().map(|s| s.identity.file.clone()));
                assert_eq!(all, ["src/a.ts", "src/b.ts", "src/c.ts"]);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    /// CE-6: a malformed pagination cursor is a structured `InvalidQuery`, not a
    /// panic or a silently-empty page.
    #[test]
    fn gctx_search_rejects_malformed_cursor() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "src/a.ts", &["a"], 0);
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .search_symbols(&GctxSearchSymbolsRequest {
                workspace_root: tmp.path().to_string_lossy().into_owned(),
                query: SearchSymbolsQuery {
                    cursor: Some(anvil_gctx_types::OpaqueCursor::new("garbage".into())),
                    ..Default::default()
                },
            })
            .expect("admitted");
        assert!(
            matches!(resp.outcome, SearchSymbolsOutcome::InvalidQuery { .. }),
            "a malformed cursor must be rejected: {:?}",
            resp.outcome
        );
    }

    /// CE-6 input validation (rejected before the graph is queried).
    #[test]
    fn gctx_search_rejects_oversized_nul_and_scheme_filters() {
        let tmp = tempfile::tempdir().expect("tempdir");

        let oversized = gctx_invalid_query(
            tmp.path(),
            SearchSymbolsQuery {
                name: Some("x".repeat(513)),
                ..Default::default()
            },
        );
        assert!(matches!(
            oversized,
            SearchSymbolsOutcome::InvalidQuery { .. }
        ));

        let nul = gctx_invalid_query(
            tmp.path(),
            SearchSymbolsQuery {
                name: Some("ab\0cd".to_string()),
                ..Default::default()
            },
        );
        assert!(matches!(nul, SearchSymbolsOutcome::InvalidQuery { .. }));

        let scheme = gctx_invalid_query(
            tmp.path(),
            SearchSymbolsQuery {
                file: Some("https://evil/x".to_string()),
                ..Default::default()
            },
        );
        assert!(matches!(scheme, SearchSymbolsOutcome::InvalidQuery { .. }));
    }

    /// CE-11 kill-switch: a disabled surface egresses nothing even with a warm,
    /// `Clean` graph, and self-reports `Disabled` (distinct from `Unavailable`).
    #[test]
    fn gctx_kill_switch_disables_egress_even_when_warm() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "src/a.ts", &["a"], 0);
        let key =
            WorktreeKey::from_canonical(std::fs::canonicalize(tmp.path()).expect("canonical"));
        let clean = assurance(AssuranceState::Clean, None);

        let disabled =
            gctx_search_outcome(&state, &key, &clean, &SearchSymbolsQuery::default(), true);
        assert!(
            matches!(disabled, SearchSymbolsOutcome::Disabled),
            "kill-switch must disable a warm graph: {disabled:?}",
        );

        let enabled =
            gctx_search_outcome(&state, &key, &clean, &SearchSymbolsQuery::default(), false);
        assert!(
            matches!(enabled, SearchSymbolsOutcome::Ready(_)),
            "an enabled surface serves the warm graph: {enabled:?}",
        );
    }

    #[test]
    fn gctx_egress_disabled_only_on_trimmed_zero() {
        assert!(gctx_egress_disabled_from(Some("0")));
        // Whitespace / trailing newline must still disable (no silent fail-open).
        assert!(gctx_egress_disabled_from(Some(" 0")));
        assert!(gctx_egress_disabled_from(Some("0 ")));
        assert!(gctx_egress_disabled_from(Some("0\n")));
        assert!(!gctx_egress_disabled_from(None)); // unset → on (default)
        assert!(!gctx_egress_disabled_from(Some("1"))); // snippet opt-in → on
        assert!(!gctx_egress_disabled_from(Some(""))); // empty → on
        assert!(!gctx_egress_disabled_from(Some("false")));
        assert!(!gctx_egress_disabled_from(Some("00")));
    }

    /// CE-10: a readable result classifies `hit`/`miss` by content; the daemon
    /// telemetry binds to that PII-free enum.
    #[test]
    fn gctx_telemetry_outcome_splits_hit_and_miss() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "src/a.ts", &["alpha"], 0);
        let key =
            WorktreeKey::from_canonical(std::fs::canonicalize(tmp.path()).expect("canonical"));
        let clean = assurance(AssuranceState::Clean, None);

        let hit = gctx_search_outcome(
            &state,
            &key,
            &clean,
            &SearchSymbolsQuery {
                name: Some("alpha".into()),
                ..Default::default()
            },
            false,
        );
        assert_eq!(hit.telemetry_outcome().as_str(), "hit");

        let miss = gctx_search_outcome(
            &state,
            &key,
            &clean,
            &SearchSymbolsQuery {
                name: Some("nonexistent".into()),
                ..Default::default()
            },
            false,
        );
        assert_eq!(miss.telemetry_outcome().as_str(), "miss");
    }

    fn assurance(state: AssuranceState, reason: Option<StaleReason>) -> WorkspaceAssurance {
        WorkspaceAssurance {
            state,
            reason,
            generation: 0,
            last_full_scan: None,
            scan_coverage: None,
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

    // ---- DSV-030: warm-start persistence (ADR-069) ----

    /// The item's headline validation: a warm graph persisted on shutdown is
    /// restored into a *fresh* daemon's cache on warm-start, and the restored
    /// worktree comes up **`Stale`** (the verdict is re-derived, never carried
    /// across the restart).
    #[cfg(unix)]
    #[test]
    fn warm_start_round_trips_indexes_and_stays_stale() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = std::path::PathBuf::from("/ws-restart");
        let key = WorktreeKey::from_canonical(root.clone());

        // Daemon 1: warm a key, then persist on shutdown.
        let state1 = state().with_snapshot_dir(dir.path().to_path_buf());
        state1.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["alpha"], 0),
        );
        assert!(state1.cache.contains(&key));
        state1.persist_all_on_shutdown();
        assert!(
            std::fs::read_dir(dir.path()).unwrap().next().is_some(),
            "a snapshot file must be written on shutdown",
        );

        // Daemon 2 (fresh cache, same snapshot dir): cold until restore.
        let state2 = state().with_snapshot_dir(dir.path().to_path_buf());
        assert!(!state2.cache.contains(&key), "fresh daemon starts cold");

        restore_snapshot_into_cache(
            &state2.cache,
            state2.scan_coordinator(),
            dir.path(),
            &key,
            &root,
        );
        assert!(
            state2.cache.contains(&key),
            "warm-start restored the indexes"
        );

        // Verdict re-derived: a restored worktree is Stale, never carried Clean.
        let machine = state2.machine_handle(&key);
        assert_eq!(
            machine.lock().unwrap().state(),
            AssuranceState::Stale,
            "a restored worktree must come up Stale (verdict re-derived)",
        );
    }

    /// Default-off (ADR-069 §7): with no snapshot dir wired, the daemon writes
    /// **nothing** — no file, no dir creation — across a warm + shutdown cycle.
    #[cfg(unix)]
    #[test]
    fn default_off_persists_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let key = WorktreeKey::from_canonical(std::path::PathBuf::from("/ws-off"));

        // OFF: `state()` has no snapshot dir → persistence off → no write.
        let off = state();
        assert!(!off.persistence_enabled());
        off.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["a"], 0),
        );
        off.persist_all_on_shutdown();
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "persistence-off must write nothing under the snapshot dir",
        );

        // Positive control (non-vacuous): the SAME dir DOES receive a snapshot
        // once persistence is wired — proving the OFF assertion above is meaningful
        // (the dir is the real target, not an unrelated path).
        let on = state().with_snapshot_dir(dir.path().to_path_buf());
        assert!(on.persistence_enabled());
        on.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["a"], 0),
        );
        on.persist_all_on_shutdown();
        assert!(
            std::fs::read_dir(dir.path()).unwrap().next().is_some(),
            "persistence-on must write the snapshot to this very dir",
        );
    }

    /// A warm-start with no snapshot on disk is a quiet no-op (the normal
    /// first-run / fresh-worktree case) — the key stays cold, no panic.
    #[cfg(unix)]
    #[test]
    fn restore_without_a_snapshot_is_a_noop() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = state().with_snapshot_dir(dir.path().to_path_buf());
        let key = WorktreeKey::from_canonical(std::path::PathBuf::from("/ws-none"));
        restore_snapshot_into_cache(
            &state.cache,
            state.scan_coordinator(),
            dir.path(),
            &key,
            std::path::Path::new("/ws-none"),
        );
        assert!(!state.cache.contains(&key), "no snapshot ⇒ stays cold");
    }
}
