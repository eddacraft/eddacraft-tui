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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use anvil_checks::antipattern::types::AntipatternCheckConfig;
use anvil_checks::secret::scanner::scan_content_with_stats;
use anvil_checks::secret::types::SecretCheckConfig;
use anvil_gctx_egress::{
    DEFAULT_SYMBOL_CONTEXT_TOKENS, GctxProjector, MAX_SYMBOL_CONTEXT_TOKENS, Redaction,
    SnippetByteLedger,
};
use anvil_gctx_types::{
    AffectedTestsOutcome, AffectedTestsQuery, ContextSelector, FindCallersOutcome,
    FindCallersQuery, FindDependentsOutcome, FindDependentsQuery, GCTX_EGRESS_ENV,
    GraphEdgesOutcome, GraphEdgesQuery, GraphStatsOutcome, ImpactOutcome, ImpactQuery, OmitReason,
    SearchSymbolsOutcome, SearchSymbolsQuery, SnippetOutcome, SnippetQuery, SymbolContextOutcome,
    SymbolContextProjection, SymbolContextQuery, gctx_egress_disabled_from,
    gctx_snippet_egress_enabled_from,
};
use anvil_graph_cache::clamp_reverse_impact_depth;
use anvil_intercept_proto::protocol::{
    AssuranceState, GctxAffectedTestsRequest, GctxAffectedTestsResponse, GctxFindCallersRequest,
    GctxFindCallersResponse, GctxFindDependentsRequest, GctxFindDependentsResponse,
    GctxGetSnippetRequest, GctxGetSnippetResponse, GctxGraphEdgesRequest, GctxGraphEdgesResponse,
    GctxGraphStatsRequest, GctxGraphStatsResponse, GctxImpactOfChangeRequest,
    GctxImpactOfChangeResponse, GctxSearchSymbolsRequest, GctxSearchSymbolsResponse,
    GctxSymbolContextRequest, GctxSymbolContextResponse, RequestFullScanRequest,
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
use crate::kindling_observation::SaveTimeObservationEmitter;
use crate::rule_cache::WorktreeKey;
use crate::telemetry::{NotificationEnvelope, TelemetryCorrelation, TelemetryEmitter};
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

/// CIB-095c scoped opt-out for the **implicit** background-scan trigger. Setting
/// it to `0` stops a cold-key `validate_paths`/`workspace_status`/GCTX
/// first-contact from spawning an auto-warm scan, while keeping the daemon
/// serving (unlike `ANVIL_WATCH_DAEMON=0`, which bypasses the daemon entirely).
/// An **explicit** `request_full_scan` (`ScanPriority::Interactive`) is never
/// suppressed — only the opportunistic `Background` auto-warm. Trimmed before
/// comparison so `" 0"`/`"0\n"` still disable (no silent fail-open).
const WATCH_DAEMON_SCAN_ENV: &str = "ANVIL_WATCH_DAEMON_SCAN";

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

/// ADR-069 §10 structured snapshot I/O counters. One cumulative counter per
/// outcome, labelled by the [`SnapshotReadError`](crate::snapshot_io::SnapshotReadError)
/// variant on the load path (`absent` / `corrupt` / `version_mismatch` / `io`) and
/// `ok` for an accepted load, plus `write_ok` / `write_error` on the write path.
///
/// These exist so the §7b graduation criterion — **"zero
/// `SnapshotLoadError::Corrupt` across the soak"** — is *verifiable* rather than
/// only being WARN-logged. Plain `AtomicU64`s (matching the broadcaster's
/// `dropped_envelopes` and the mid-edit in-flight counters) so they are lock-free
/// on the load/write path and snapshot-able for the soak query. The labels are the
/// outcome name only — never a path, key, or any decoded byte (PV-10 / verdict
/// N-3: the only label a counter may bind is the variant name).
#[derive(Debug, Default)]
pub struct SnapshotMetrics {
    /// A snapshot loaded and passed the integrity gate.
    load_ok: AtomicU64,
    /// No snapshot file for the key (the normal cold-start case).
    load_absent: AtomicU64,
    /// The integrity gate rejected the bytes as corrupt/oversized/torn
    /// (`SnapshotLoadError::{BadMagic, ChecksumMismatch, CountMismatch, Oversized, Corrupt}`)
    /// — the soak's graduation-blocking class.
    load_corrupt: AtomicU64,
    /// The envelope `format`/`backing` version did not match this build (expected
    /// once after a schema bump, not an error).
    load_version_mismatch: AtomicU64,
    /// A disk error reading the file (not a decode failure).
    load_io: AtomicU64,
    /// A snapshot was durably written.
    write_ok: AtomicU64,
    /// A snapshot write failed (the temp was cleaned up; the key degrades to
    /// no-persistence).
    write_error: AtomicU64,
}

/// A point-in-time read of [`SnapshotMetrics`] for assertions / the soak query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SnapshotMetricsSnapshot {
    pub load_ok: u64,
    pub load_absent: u64,
    pub load_corrupt: u64,
    pub load_version_mismatch: u64,
    pub load_io: u64,
    pub write_ok: u64,
    pub write_error: u64,
}

impl SnapshotMetrics {
    /// Record the outcome of a [`load_snapshot`](crate::snapshot_io::load_snapshot)
    /// call: the `Ok` arm is `ok`, each error maps to its labelled counter.
    #[cfg(unix)]
    fn record_load(
        &self,
        result: &Result<
            anvil_graph_cache::snapshot::SnapshotPayload,
            crate::snapshot_io::SnapshotReadError,
        >,
    ) {
        use crate::snapshot_io::SnapshotReadError;
        use anvil_graph_cache::snapshot::SnapshotLoadError;
        let counter = match result {
            Ok(_) => &self.load_ok,
            Err(SnapshotReadError::NotFound) => &self.load_absent,
            Err(SnapshotReadError::Io(_)) => &self.load_io,
            Err(SnapshotReadError::Rejected(SnapshotLoadError::VersionMismatch { .. })) => {
                &self.load_version_mismatch
            }
            Err(SnapshotReadError::Rejected(_)) => &self.load_corrupt,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Record the outcome of a [`write_snapshot`](crate::snapshot_io::write_snapshot)
    /// call (`ok` / `error`).
    #[cfg_attr(windows, allow(dead_code))]
    pub(crate) fn record_write(&self, result: &io::Result<()>) {
        let counter = if result.is_ok() {
            &self.write_ok
        } else {
            &self.write_error
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// CIB-095f: count a snapshot that was DROPPED before the write attempt — a
    /// `from_graphs` build rejection (a non-workspace-relative path, ADR-069 §8)
    /// loses the snapshot just as a failed write does, so it must be observable
    /// in `write_error` rather than silently skipped.
    #[cfg_attr(windows, allow(dead_code))]
    pub(crate) fn record_write_error(&self) {
        self.write_error.fetch_add(1, Ordering::Relaxed);
    }

    /// A consistent point-in-time read of every counter.
    #[must_use]
    pub fn snapshot(&self) -> SnapshotMetricsSnapshot {
        SnapshotMetricsSnapshot {
            load_ok: self.load_ok.load(Ordering::Relaxed),
            load_absent: self.load_absent.load(Ordering::Relaxed),
            load_corrupt: self.load_corrupt.load(Ordering::Relaxed),
            load_version_mismatch: self.load_version_mismatch.load(Ordering::Relaxed),
            load_io: self.load_io.load(Ordering::Relaxed),
            write_ok: self.write_ok.load(Ordering::Relaxed),
            write_error: self.write_error.load(Ordering::Relaxed),
        }
    }
}

/// ADR-035 (CIB-092h): a cheap-to-clone handle the DSV-045 background executor
/// uses to raise the same persist-failure Notification the shutdown flush does,
/// without borrowing the whole [`SaveTimeState`]. Bundles the broadcaster with a
/// dedicated [`TelemetryEmitter`] (its own producer-instance id + seq stream — the
/// background scan has no per-connection emitter to share). Present only when a
/// broadcaster is wired AND persistence is enabled.
#[derive(Clone)]
#[cfg_attr(windows, allow(dead_code))]
pub(crate) struct PersistFailureNotifier {
    broadcaster: Arc<TelemetryBroadcaster>,
    emitter: Arc<Mutex<TelemetryEmitter>>,
}

impl PersistFailureNotifier {
    fn new(broadcaster: Arc<TelemetryBroadcaster>) -> Self {
        Self {
            broadcaster,
            emitter: Arc::new(Mutex::new(TelemetryEmitter::new())),
        }
    }

    /// Build + broadcast the ADR-035 degradation Notification for a failed write.
    /// Returns the envelope so callers/tests can assert on it. Echoes only the
    /// `io::ErrorKind` discriminant (PV-10).
    #[cfg_attr(windows, allow(dead_code))]
    pub(crate) fn notify(&self, workspace_root: &Path, err: &io::Error) -> NotificationEnvelope {
        let message = format!("snapshot write failed ({})", err.kind());
        let envelope = {
            let mut emitter = self.emitter.lock().unwrap_or_else(PoisonError::into_inner);
            emitter.persist_failure_health_envelope(
                TelemetryCorrelation::default(),
                workspace_root,
                message,
            )
        };
        let _ = self.broadcaster.broadcast(&envelope);
        envelope
    }
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
    /// DPO-001: the save-time `gate_evaluated` emitter. When present, every
    /// `validate_paths` verdict produces a Kindling row (pass and fail) through
    /// the IPC arm. `None` (the default) keeps the daemon silent — the emitter
    /// is wired from `anvil-cli` alongside the other observation surfaces.
    observation_emitter: Option<Arc<SaveTimeObservationEmitter>>,
    /// DSV-030 / ADR-069 §10 (CIB-092b): cumulative snapshot load/write outcome
    /// counters. Always present (cheap, lock-free); they stay `0` while
    /// persistence is off. The §7b soak's "zero `Corrupt`" graduation check reads
    /// `load_corrupt` here.
    snapshot_metrics: Arc<SnapshotMetrics>,
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
            observation_emitter: None,
            snapshot_metrics: Arc::new(SnapshotMetrics::default()),
        }
    }

    /// DSV-030 / ADR-069 §10: the cumulative snapshot I/O outcome counters
    /// (CIB-092b). Read for assertions and the §7b soak graduation check.
    #[must_use]
    pub fn snapshot_metrics(&self) -> SnapshotMetricsSnapshot {
        self.snapshot_metrics.snapshot()
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

    /// DPO-001: attach the save-time `gate_evaluated` emitter. With it wired,
    /// every `validate_paths` verdict produces a Kindling row (pass and fail);
    /// without it the daemon stays silent (tests, embedded listeners). The
    /// emitter is decoupled from the DSV-044 telemetry correlation/session gate
    /// — it fires on the verdict alone.
    #[must_use]
    pub fn with_observation_emitter(mut self, emitter: Arc<SaveTimeObservationEmitter>) -> Self {
        self.observation_emitter = Some(emitter);
        self
    }

    /// DPO-001: the wired save-time `gate_evaluated` emitter, if any. The IPC
    /// `validate_paths` arm reads this to emit a row after each verdict.
    #[must_use]
    pub fn observation_emitter(&self) -> Option<&Arc<SaveTimeObservationEmitter>> {
        self.observation_emitter.as_ref()
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
        // CIB-092b/h: hand the executor the shared snapshot counters and, when
        // persistence is enabled with a broadcaster wired, a persist-failure
        // notifier so its background write path observes + surfaces failures the
        // same way the shutdown flush does.
        let notifier = self
            .broadcaster
            .as_ref()
            .filter(|_| self.persistence_enabled())
            .map(|b| Arc::new(PersistFailureNotifier::new(Arc::clone(b))));
        ScanContext::new(
            Arc::clone(&self.cache),
            self.parser.clone(),
            self.caps,
            self.coordinator.clone(),
            self.snapshot_dir.clone(),
            Arc::clone(&self.snapshot_metrics),
            notifier,
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
            let metrics = Arc::clone(&self.snapshot_metrics);
            let key = key.clone();
            let root = canonical_root.to_path_buf();
            self.scheduler.background().spawn(move || {
                restore_snapshot_into_cache(&cache, &coordinator, &metrics, &dir, &key, &root);
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
                let payload = match built {
                    Some(Ok(payload)) => payload,
                    // CIB-095f / cross-ref: a `from_graphs` build rejection (a
                    // non-workspace-relative path, ADR-069 §8) drops the snapshot.
                    // Surface it consistently with `persist_after_scan` (WARN) and
                    // count it as a lost write rather than the previous SILENT
                    // skip, so an operator sees it in the cumulative metrics.
                    Some(Err(_)) => {
                        failed += 1;
                        self.snapshot_metrics.record_write_error();
                        tracing::warn!(
                            target: "anvil_intercept::snapshot",
                            workspace_root = %key.as_path().display(),
                            "snapshot build rejected a non-relative path on shutdown; \
                             skipped persisting (no persistence for this key)",
                        );
                        continue;
                    }
                    // The entry was evicted between `warm_keys` and here — nothing
                    // to write, not a failure.
                    None => continue,
                };
                let result = crate::snapshot_io::write_snapshot(dir, key.as_path(), &payload);
                // ADR-069 §10 (CIB-092b): count every write attempt by outcome.
                self.snapshot_metrics.record_write(&result);
                match result {
                    Ok(()) => written += 1,
                    // ADR-069 §10: a write failure is surfaced (WARN + counter),
                    // never silent loss — even on the shutdown flush. The WARN here
                    // plus the cumulative-metrics info! below are the real
                    // operator-visible signal (CIB-092 item 1). The ADR-035 envelope
                    // built by `notify_persist_write_failure` is NOT delivered for a
                    // daemon-internal write (no originating session ⇒ INTD-015
                    // fanout denies it); see that fn's doc (CIB-092h item 4).
                    Err(err) => {
                        failed += 1;
                        tracing::warn!(
                            target: "anvil_intercept::snapshot",
                            workspace_root = %key.as_path().display(),
                            error = %err,
                            "snapshot write failed on shutdown; skipping (no persistence for this key)",
                        );
                        self.notify_persist_write_failure(key.as_path(), &err);
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
            // CIB-092 council survivor (item 1): emit the FULL cumulative snapshot
            // I/O counters as a structured event at shutdown so the §7b soak's
            // "zero SnapshotLoadError::Corrupt across the soak" graduation criterion
            // is scrapeable from outside the process — the counters otherwise live
            // only in process memory and vanish at exit. Unconditional (even a
            // write-only or load-only run carries a meaningful corrupt/io readout).
            emit_cumulative_snapshot_metrics(&self.snapshot_metrics.snapshot());
        }
    }

    /// Reclaim orphaned `*.snap` snapshots whose worktree was deleted while the
    /// daemon was down (ADR-069 §10, CIB-096). A worktree deleted while the daemon
    /// was down never fires its unregister hook (the normal `remove_snapshot`
    /// path), so its snapshot would otherwise linger forever; this sweep removes
    /// any `.snap` whose `<hash>.root` companion points at a path that no longer
    /// exists, returning the count removed. No-op when persistence is off.
    ///
    /// Existence-based (companion `.root` file), so it needs **no registry
    /// keep-set and is safe at cold boot** — it can never wipe a live, not-yet-
    /// reattached snapshot. A snapshot with a missing/unreadable companion is kept
    /// (fail-safe). Stray `.root` companions with no matching `.snap` are cleaned up.
    pub fn sweep_orphan_snapshots_on_start(&self) -> usize {
        #[cfg(unix)]
        {
            let Some(dir) = self.snapshot_dir.as_deref() else {
                return 0;
            };
            // The single INFO report point is the `lib.rs` caller (CIB-096
            // follow-up: deduplicated). This method only returns the count; it does
            // not log, to avoid two INFO lines for the one reclaim event.
            crate::snapshot_io::sweep_orphan_snapshots_on_start(dir)
        }
        #[cfg(not(unix))]
        {
            0
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
        self.spawn_scan_gated(
            key,
            root,
            priority,
            implicit_scan_disabled(std::env::var(WATCH_DAEMON_SCAN_ENV).ok().as_deref()),
        );
    }

    /// CIB-095c: `spawn_scan` with the scoped opt-out decision injected (testable
    /// without mutating process env). When `implicit_scan_disabled` is set, an
    /// **implicit** `Background` auto-warm is suppressed — no scan is enqueued and
    /// the machine is left untouched — while an **explicit** `Interactive`
    /// `request_full_scan` is always honoured.
    fn spawn_scan_gated(
        &self,
        key: &WorktreeKey,
        root: &Path,
        priority: ScanPriority,
        implicit_scan_disabled: bool,
    ) {
        if implicit_scan_disabled && priority == ScanPriority::Background {
            return;
        }
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

    /// ADR-035 (CIB-092h): raise an operator-facing degradation Notification when
    /// **persistence is explicitly enabled** but a snapshot write failed. A
    /// disabled-persistence write never happens (the dir is `None`), so this is a
    /// no-op unless the operator opted in. Built + broadcast through the same
    /// `TelemetryBroadcaster` the assurance path uses; returns the envelope (when
    /// one was built) so the behaviour is directly assertable. The message echoes
    /// only the `io::ErrorKind` discriminant — never a path or identity byte.
    ///
    /// Daemon-internal writes (shutdown flush / background scan) carry no
    /// originating session, so the INTD-015 fanout (`fanout.rs::decide`)
    /// **hard-denies this envelope to every subscriber** — an envelope with no
    /// `originating_session_id` fails the originator check before the
    /// ownership/cross-session branches even run. There is no daemon-local health
    /// sink that bypasses the session-deny (it is a deliberate INTD-015 invariant),
    /// so **this notification is never delivered to an operator today**. The
    /// real, user-visible operator signal for a degraded persist is the
    /// `tracing::warn!` per failed write PLUS the cumulative snapshot-metrics
    /// shutdown `info!` log (CIB-092 item 1) — NOT this notification. The envelope
    /// is still built + offered to the broadcaster only so a *future*
    /// session-correlated producer (or an in-process subscriber) can observe it; it
    /// costs nothing and the broadcaster simply drops it now (CIB-092h item 4).
    #[cfg_attr(windows, allow(dead_code))]
    fn notify_persist_write_failure(
        &self,
        workspace_root: &Path,
        err: &io::Error,
    ) -> Option<NotificationEnvelope> {
        if !self.persistence_enabled() {
            return None;
        }
        let broadcaster = self.broadcaster.as_ref()?;
        let message = format!("snapshot write failed ({})", err.kind());
        let envelope = {
            let mut emitter = self
                .telemetry
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            emitter.persist_failure_health_envelope(
                TelemetryCorrelation::default(),
                workspace_root,
                message,
            )
        };
        let outcome = broadcaster.broadcast(&envelope);
        tracing::debug!(
            target: "anvil_intercept::snapshot",
            workspace_root = %workspace_root.display(),
            delivered = outcome.delivered,
            dropped = outcome.dropped,
            "persist-failure notification raised (ADR-035)",
        );
        Some(envelope)
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

/// CIB-092 council survivor (item 1): emit the cumulative snapshot I/O counters as
/// one structured `tracing::info!` so the §7b soak's "zero `Corrupt`" graduation
/// signal survives process exit and is scrapeable. Every field of
/// [`SnapshotMetricsSnapshot`] rides the event (load ok/absent/corrupt/version/io,
/// write ok/error) — no path, key, or decoded byte (PV-10: counts only). Split out
/// of [`SaveTimeState::persist_all_on_shutdown`] so the emission site is a single,
/// directly-testable function.
#[cfg_attr(windows, allow(dead_code))]
fn emit_cumulative_snapshot_metrics(metrics: &SnapshotMetricsSnapshot) {
    tracing::info!(
        target: "anvil_intercept::snapshot",
        load_ok = metrics.load_ok,
        load_absent = metrics.load_absent,
        load_corrupt = metrics.load_corrupt,
        load_version_mismatch = metrics.load_version_mismatch,
        load_io = metrics.load_io,
        write_ok = metrics.write_ok,
        write_error = metrics.write_error,
        "cumulative snapshot I/O metrics at shutdown (ADR-069 §10)",
    );
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
    metrics: &SnapshotMetrics,
    dir: &Path,
    key: &WorktreeKey,
    canonical_root: &Path,
) {
    if cache.contains(key) || coordinator.is_enqueued(key) {
        return;
    }
    let result = crate::snapshot_io::load_snapshot(dir, canonical_root);
    // ADR-069 §10 (CIB-092b): one counter increment per load, labelled by outcome,
    // before the WARN/INFO/DEBUG mirror — so the §7b "zero Corrupt" soak check is
    // a counter read, not a log scrape.
    metrics.record_load(&result);
    let payload = match result {
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
    /// CE-6 per-session snippet byte ledger keyed on `(file, ByteRange)` (GCTX-022).
    snippet_byte_ledger: SnippetByteLedger,
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
            snippet_byte_ledger: SnippetByteLedger::default(),
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
    /// DPO-001: delegate to the shared state so the IPC `validate_paths` arm can
    /// reach the save-time emitter through the per-connection dispatch object.
    fn observation_emitter(&self) -> Option<&Arc<SaveTimeObservationEmitter>> {
        self.state.observation_emitter()
    }

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
        // CIB-091b (CE-6 gap): validate the raw root (NUL + size cap) before it
        // reaches `PathBuf::from`/`canonicalize`, returning a structured
        // `InvalidQuery` rather than a generic `-32603 Internal`.
        if let Some(reason) = invalid_workspace_root_reason(&request.workspace_root) {
            return Ok(GctxSearchSymbolsResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: SearchSymbolsOutcome::InvalidQuery { reason },
            });
        }
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
        // CIB-091b (CE-6 gap): validate the raw root before canonicalisation.
        if let Some(reason) = invalid_workspace_root_reason(&request.workspace_root) {
            return Ok(GctxFindDependentsResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: FindDependentsOutcome::InvalidQuery { reason },
            });
        }
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        // ADR-084 C3 / CE-8: admit the client-supplied root before any read.
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        // N8 (CIB-095): trigger the first-contact warm-start restore on every
        // graph-reading GCTX verb, not just `search_symbols` — so a fresh session
        // reading via `find_dependents`/`find_callers`/`graph_stats`/
        // `graph_edges`/`impact_of_change`/`affected_tests` is served the restored
        // (stale) graph rather than `NotReady`. Background + self-gating: a no-op
        // when persistence is off, the key is already warm, or a scan is enqueued.
        state.spawn_restore(&key, key.as_path());

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

    fn find_callers(
        &mut self,
        request: &GctxFindCallersRequest,
    ) -> Result<GctxFindCallersResponse, SaveTimeError> {
        // CIB-091b (CE-6 gap): validate the raw root before canonicalisation.
        if let Some(reason) = invalid_workspace_root_reason(&request.workspace_root) {
            return Ok(GctxFindCallersResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: FindCallersOutcome::InvalidQuery { reason },
            });
        }
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        // ADR-084 C3 / CE-8: admit the client-supplied root before any read.
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        // N8 (CIB-095): trigger the first-contact warm-start restore on every
        // graph-reading GCTX verb, not just `search_symbols` — so a fresh session
        // reading via `find_dependents`/`find_callers`/`graph_stats`/
        // `graph_edges`/`impact_of_change`/`affected_tests` is served the restored
        // (stale) graph rather than `NotReady`. Background + self-gating: a no-op
        // when persistence is off, the key is already warm, or a scan is enqueued.
        state.spawn_restore(&key, key.as_path());

        // CE-7: the assurance snapshot always rides along.
        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());

        let outcome = gctx_find_callers_outcome(
            state,
            &key,
            &workspace_assurance,
            &request.query,
            gctx_egress_disabled(),
        );

        // CE-10: enum-only telemetry + response-aggregate counts — never caller
        // identities or query text.
        let (matched, returned) = match &outcome {
            FindCallersOutcome::Ready(projection) => (
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
            "gctx find_callers served",
        );

        Ok(GctxFindCallersResponse {
            workspace_assurance,
            outcome,
        })
    }

    fn graph_stats(
        &mut self,
        request: &GctxGraphStatsRequest,
    ) -> Result<GctxGraphStatsResponse, SaveTimeError> {
        // CIB-091b (CE-6 gap): validate the raw root before canonicalisation.
        // `graph://stats` takes no query and so has no `InvalidQuery` arm; a
        // malformed root degrades to `Unavailable` (the named no-graph surface)
        // rather than a generic `-32603 Internal`.
        if invalid_workspace_root_reason(&request.workspace_root).is_some() {
            return Ok(GctxGraphStatsResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: GraphStatsOutcome::Unavailable,
            });
        }
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        // ADR-084 C3 / CE-8: admit the client-supplied root before any read.
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        // N8 (CIB-095): trigger the first-contact warm-start restore on every
        // graph-reading GCTX verb, not just `search_symbols` — so a fresh session
        // reading via `find_dependents`/`find_callers`/`graph_stats`/
        // `graph_edges`/`impact_of_change`/`affected_tests` is served the restored
        // (stale) graph rather than `NotReady`. Background + self-gating: a no-op
        // when persistence is off, the key is already warm, or a scan is enqueued.
        state.spawn_restore(&key, key.as_path());

        // CE-7: the assurance snapshot always rides along.
        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());

        let outcome =
            gctx_graph_stats_outcome(state, &key, &workspace_assurance, gctx_egress_disabled());

        // CE-10: enum-only telemetry — counts only, never paths or names.
        tracing::info!(
            target: "anvil_intercept::gctx",
            outcome = outcome.telemetry_outcome().as_str(),
            "gctx graph_stats served",
        );

        Ok(GctxGraphStatsResponse {
            workspace_assurance,
            outcome,
        })
    }

    fn graph_edges(
        &mut self,
        request: &GctxGraphEdgesRequest,
    ) -> Result<GctxGraphEdgesResponse, SaveTimeError> {
        // CIB-091b (CE-6 gap): validate the raw root before canonicalisation.
        if let Some(reason) = invalid_workspace_root_reason(&request.workspace_root) {
            return Ok(GctxGraphEdgesResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: GraphEdgesOutcome::InvalidQuery { reason },
            });
        }
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        // ADR-084 C3 / CE-8: admit the client-supplied root before any read.
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        // N8 (CIB-095): trigger the first-contact warm-start restore on every
        // graph-reading GCTX verb, not just `search_symbols` — so a fresh session
        // reading via `find_dependents`/`find_callers`/`graph_stats`/
        // `graph_edges`/`impact_of_change`/`affected_tests` is served the restored
        // (stale) graph rather than `NotReady`. Background + self-gating: a no-op
        // when persistence is off, the key is already warm, or a scan is enqueued.
        state.spawn_restore(&key, key.as_path());

        // CE-7: the assurance snapshot always rides along.
        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());

        let outcome = gctx_graph_edges_outcome(
            state,
            &key,
            &workspace_assurance,
            &request.query,
            gctx_egress_disabled(),
        );

        // CE-10: enum-only telemetry + response-aggregate counts — never edge
        // identities or query text.
        let (matched, returned) = match &outcome {
            GraphEdgesOutcome::Ready(projection) => (
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
            "gctx graph_edges served",
        );

        Ok(GctxGraphEdgesResponse {
            workspace_assurance,
            outcome,
        })
    }

    fn impact_of_change(
        &mut self,
        request: &GctxImpactOfChangeRequest,
    ) -> Result<GctxImpactOfChangeResponse, SaveTimeError> {
        // CIB-091b (CE-6 gap): validate the raw root before canonicalisation.
        if let Some(reason) = invalid_workspace_root_reason(&request.workspace_root) {
            return Ok(GctxImpactOfChangeResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: ImpactOutcome::InvalidQuery { reason },
            });
        }
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        // ADR-084 C3 / CE-8: admit the client-supplied root before any read.
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        // N8 (CIB-095): trigger the first-contact warm-start restore on every
        // graph-reading GCTX verb, not just `search_symbols` — so a fresh session
        // reading via `find_dependents`/`find_callers`/`graph_stats`/
        // `graph_edges`/`impact_of_change`/`affected_tests` is served the restored
        // (stale) graph rather than `NotReady`. Background + self-gating: a no-op
        // when persistence is off, the key is already warm, or a scan is enqueued.
        state.spawn_restore(&key, key.as_path());

        // CE-7: the assurance snapshot always rides along.
        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());

        let outcome = gctx_impact_outcome(
            state,
            &key,
            &workspace_assurance,
            &request.query,
            gctx_egress_disabled(),
        );

        // CE-10: bind telemetry to the exhaustive PII-free outcome enum plus
        // response-aggregate counts only — never paths or query text. Note the
        // impact-specific reading (the report is not paginated, so there is no
        // pre/post-redaction relationship like the sibling verbs): `matched` is
        // the total projected surface (affected symbols + dependent files) and
        // `returned` is the dependent-file count.
        let (matched, returned) = match &outcome {
            ImpactOutcome::Ready(report) => (
                report.summary.affected_symbols + report.summary.dependent_files,
                report.summary.dependent_files,
            ),
            _ => (0, 0),
        };
        tracing::info!(
            target: "anvil_intercept::gctx",
            outcome = outcome.telemetry_outcome().as_str(),
            matched,
            returned,
            "gctx impact_of_change served",
        );

        Ok(GctxImpactOfChangeResponse {
            workspace_assurance,
            outcome,
        })
    }

    fn affected_tests(
        &mut self,
        request: &GctxAffectedTestsRequest,
    ) -> Result<GctxAffectedTestsResponse, SaveTimeError> {
        // CIB-091b (CE-6 gap): validate the raw root before canonicalisation.
        if let Some(reason) = invalid_workspace_root_reason(&request.workspace_root) {
            return Ok(GctxAffectedTestsResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: AffectedTestsOutcome::InvalidQuery { reason },
            });
        }
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        // ADR-084 C3 / CE-8: admit the client-supplied root before any read.
        authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        // N8 (CIB-095): trigger the first-contact warm-start restore on every
        // graph-reading GCTX verb, not just `search_symbols` — so a fresh session
        // reading via `find_dependents`/`find_callers`/`graph_stats`/
        // `graph_edges`/`impact_of_change`/`affected_tests` is served the restored
        // (stale) graph rather than `NotReady`. Background + self-gating: a no-op
        // when persistence is off, the key is already warm, or a scan is enqueued.
        state.spawn_restore(&key, key.as_path());

        // CE-7: the assurance snapshot always rides along.
        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());

        let outcome = gctx_affected_tests_outcome(
            state,
            &key,
            &workspace_assurance,
            &request.query,
            gctx_egress_disabled(),
        );

        // CE-10: bind telemetry to the exhaustive PII-free outcome enum plus
        // response-aggregate counts only — never paths or query text. `matched`
        // is the total projected surface (tests + coverage gaps) and `returned`
        // is the attributed test count.
        let (matched, returned) = match &outcome {
            AffectedTestsOutcome::Ready(report) => (
                report.summary.tests + report.summary.coverage_gaps,
                report.summary.tests,
            ),
            _ => (0, 0),
        };
        tracing::info!(
            target: "anvil_intercept::gctx",
            outcome = outcome.telemetry_outcome().as_str(),
            matched,
            returned,
            "gctx affected_tests served",
        );

        Ok(GctxAffectedTestsResponse {
            workspace_assurance,
            outcome,
        })
    }

    fn get_snippet(
        &mut self,
        request: &GctxGetSnippetRequest,
    ) -> Result<GctxGetSnippetResponse, SaveTimeError> {
        if let Some(reason) = invalid_workspace_root_reason(&request.workspace_root) {
            return Ok(GctxGetSnippetResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: SnippetOutcome::InvalidQuery { reason },
            });
        }
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        let anchor = authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        state.spawn_restore(&key, key.as_path());

        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());

        let egress_env = std::env::var(GCTX_EGRESS_ENV).ok();
        let include_source =
            request.query.include_source && gctx_snippet_egress_enabled_from(egress_env.as_deref());

        let outcome = gctx_get_snippet_outcome(
            state,
            &key,
            &workspace_assurance,
            &request.query,
            gctx_egress_disabled_from(egress_env.as_deref()),
            include_source,
            |file| anchor.read_rel(file).ok(),
            &mut self.snippet_byte_ledger,
        );

        tracing::info!(
            target: "anvil_intercept::gctx",
            outcome = outcome.telemetry_outcome().as_str(),
            "gctx get_snippet served",
        );

        Ok(GctxGetSnippetResponse {
            workspace_assurance,
            outcome,
        })
    }

    fn symbol_context(
        &mut self,
        request: &GctxSymbolContextRequest,
    ) -> Result<GctxSymbolContextResponse, SaveTimeError> {
        if let Some(reason) = invalid_workspace_root_reason(&request.workspace_root) {
            return Ok(GctxSymbolContextResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: SymbolContextOutcome::InvalidQuery { reason },
            });
        }
        let root = PathBuf::from(&request.workspace_root);
        let originating_session = self.originating_session.clone();
        let state = self.state;
        let anchor = authorise_root(&mut self.admitted, &state.confinement, &root)?;
        let canonical = canonical_root(&root)?;
        let correlation = Self::telemetry_correlation_for(originating_session.as_ref(), &canonical);
        let key = WorktreeKey::from_canonical(canonical);

        state.spawn_restore(&key, key.as_path());

        let workspace_assurance =
            state.with_machine(&key, correlation, |machine| machine.snapshot());

        let egress_env = std::env::var(GCTX_EGRESS_ENV).ok();
        let include_source =
            request.query.include_source && gctx_snippet_egress_enabled_from(egress_env.as_deref());

        let outcome = gctx_symbol_context_outcome(
            state,
            &key,
            &workspace_assurance,
            &request.query,
            gctx_egress_disabled_from(egress_env.as_deref()),
            include_source,
            |file| anchor.read_rel(file).ok(),
            &mut self.snippet_byte_ledger,
        );

        let (label, returned, omitted, redacted) = match &outcome {
            SymbolContextOutcome::Ready(p)
            | SymbolContextOutcome::Bounded(p)
            | SymbolContextOutcome::BudgetExceeded(p) => (
                p.redaction_summary.outcome.as_str(),
                p.snippets.len(),
                p.omitted_context.len(),
                p.redaction_summary.redacted_secrets,
            ),
            SymbolContextOutcome::NotReady { .. } => ("warming", 0, 0, 0),
            SymbolContextOutcome::Unavailable => ("unavailable", 0, 0, 0),
            SymbolContextOutcome::Disabled => ("disabled", 0, 0, 0),
            SymbolContextOutcome::InvalidQuery { .. } => ("invalid_query", 0, 0, 0),
        };
        tracing::info!(
            target: "anvil_intercept::gctx",
            outcome = label,
            returned,
            omitted,
            redacted,
            "gctx symbol_context served",
        );

        Ok(GctxSymbolContextResponse {
            workspace_assurance,
            outcome,
        })
    }
}

fn gctx_egress_disabled() -> bool {
    gctx_egress_disabled_from(std::env::var(GCTX_EGRESS_ENV).ok().as_deref())
}

/// CIB-095c: pure resolution of the [`WATCH_DAEMON_SCAN_ENV`] scoped opt-out.
/// The (whitespace-trimmed) value `0` disables the implicit background-scan
/// trigger; unset or any other value leaves it on. Trimming avoids a silent
/// fail-open when an operator sets `" 0"` or a trailing-newline `"0\n"`.
fn implicit_scan_disabled(raw: Option<&str>) -> bool {
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
                Some((candidates, omitted_sensitive)) => {
                    match GctxProjector::project(candidates, query, omitted_sensitive) {
                        Ok(projection) => SearchSymbolsOutcome::Ready(projection),
                        Err(reason) => SearchSymbolsOutcome::InvalidQuery { reason },
                    }
                }
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
            let collected = state.cache.with_graphs(key, |_sym, dep| {
                GctxProjector::collect_dependents(dep, &file, depth)
            });
            match collected {
                Some((candidates, walk_truncated, omitted_sensitive)) => {
                    match GctxProjector::project_dependents(
                        candidates,
                        query,
                        depth,
                        walk_truncated,
                        omitted_sensitive,
                    ) {
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
    invalid_relative_path_reason("file filter", file)
}

/// Compute the GCTX `find_callers` outcome for an admitted root (GCTX-014 /
/// ADR-084). Mirrors [`gctx_find_dependents_outcome`]: kill-switch, query
/// validation, CE-7 degradation, depth clamp, collect-under-lock / project-after.
/// The `partial` marker (CALL-1) is set when the bounded walk was budget-truncated,
/// the graph is not fully resolved (`Stale` / `Bounded`), **or** the target has an
/// unresolved call site naming it in the daemon accumulator (an unresolved call
/// leaves no edge, so a Clean graph can still be missing callers — ADR-086 §1).
fn gctx_find_callers_outcome(
    state: &SaveTimeState,
    key: &WorktreeKey,
    assurance: &WorkspaceAssurance,
    query: &FindCallersQuery,
    egress_disabled: bool,
) -> FindCallersOutcome {
    // CE-11 kill-switch.
    if egress_disabled {
        return FindCallersOutcome::Disabled;
    }

    // CE-6: reject a malformed / absent target before touching the graph.
    if let Some(reason) = invalid_find_callers_query_reason(query) {
        return FindCallersOutcome::InvalidQuery { reason };
    }
    let Some(target) = query.target.clone() else {
        return FindCallersOutcome::InvalidQuery {
            reason: "target is required".to_string(),
        };
    };

    // GV2-026 lever: clamp the requested depth.
    let depth = clamp_reverse_impact_depth(query.max_depth.unwrap_or(1));

    match assurance.state {
        AssuranceState::Unavailable | AssuranceState::Unknown => FindCallersOutcome::Unavailable,
        AssuranceState::Pending | AssuranceState::Running => FindCallersOutcome::NotReady {
            recovery_hint: "the workspace graph is warming; retry the traversal shortly"
                .to_string(),
        },
        AssuranceState::Clean | AssuranceState::Stale | AssuranceState::Bounded => {
            // A non-Clean graph may be missing call edges → the caller set is
            // partial (CALL-1).
            let graph_partial = assurance.state != AssuranceState::Clean;
            // CALL-1 honesty: even on a Clean graph, an unresolved call site
            // naming this target (dynamic dispatch, default-export callee, over-cap
            // overload, import to a non-resident file) leaves no edge and is
            // invisible to the `callers_of` walk — so the caller set may be
            // incomplete. Fold that accumulator-derived signal in so `partial` is
            // honest rather than a false "complete" (council CR-2/SEC-1).
            let callers_incomplete =
                graph_partial || state.cache.target_has_unresolved_callers(key, &target);
            // C2: collect under the lock (symbol graph), project after release.
            let collected = state.cache.with_graphs(key, |sym, _dep| {
                GctxProjector::collect_callers(sym, &target, depth)
            });
            match collected {
                Some((candidates, walk_truncated, omitted_sensitive)) => {
                    match GctxProjector::project_callers(
                        candidates,
                        query,
                        depth,
                        walk_truncated,
                        callers_incomplete,
                        omitted_sensitive,
                    ) {
                        Ok(projection) => FindCallersOutcome::Ready(projection),
                        Err(reason) => FindCallersOutcome::InvalidQuery { reason },
                    }
                }
                None => FindCallersOutcome::NotReady {
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

/// Compute the GCTX `graph_stats` outcome (GCTX-030). Kill-switch, CE-7
/// degradation, then read the counts under the lock. No query, so no
/// `InvalidQuery` arm; a readable graph (even an empty one) is always `Ready`.
fn gctx_graph_stats_outcome(
    state: &SaveTimeState,
    key: &WorktreeKey,
    assurance: &WorkspaceAssurance,
    egress_disabled: bool,
) -> GraphStatsOutcome {
    if egress_disabled {
        return GraphStatsOutcome::Disabled;
    }
    match assurance.state {
        AssuranceState::Unavailable | AssuranceState::Unknown => GraphStatsOutcome::Unavailable,
        AssuranceState::Pending | AssuranceState::Running => GraphStatsOutcome::NotReady {
            recovery_hint: "the workspace graph is warming; retry shortly".to_string(),
        },
        AssuranceState::Clean | AssuranceState::Stale | AssuranceState::Bounded => {
            // C2: read the counts under the lock; the projection is counts-only.
            let collected = state.cache.with_graphs(key, |sym, dep| {
                GctxProjector::project_stats(
                    sym.node_count(),
                    sym.edge_count(),
                    dep.file_count(),
                    dep.edge_count(),
                )
            });
            match collected {
                Some(projection) => GraphStatsOutcome::Ready(projection),
                None => GraphStatsOutcome::NotReady {
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

/// Compute the GCTX `graph_edges` outcome (GCTX-030). Kill-switch, CE-6 query
/// validation, CE-7 degradation, collect-under-lock / project-after.
fn gctx_graph_edges_outcome(
    state: &SaveTimeState,
    key: &WorktreeKey,
    assurance: &WorkspaceAssurance,
    query: &GraphEdgesQuery,
    egress_disabled: bool,
) -> GraphEdgesOutcome {
    if egress_disabled {
        return GraphEdgesOutcome::Disabled;
    }
    if let Some(reason) = invalid_graph_edges_query_reason(query) {
        return GraphEdgesOutcome::InvalidQuery { reason };
    }
    match assurance.state {
        AssuranceState::Unavailable | AssuranceState::Unknown => GraphEdgesOutcome::Unavailable,
        AssuranceState::Pending | AssuranceState::Running => GraphEdgesOutcome::NotReady {
            recovery_hint: "the workspace graph is warming; retry shortly".to_string(),
        },
        AssuranceState::Clean | AssuranceState::Stale | AssuranceState::Bounded => {
            // C2: collect under the lock (symbol graph), project after release.
            let collected = state.cache.with_graphs(key, |sym, _dep| {
                GctxProjector::collect_all_edges(sym, query.file.as_deref())
            });
            match collected {
                Some((candidates, bounded, omitted_sensitive)) => {
                    match GctxProjector::project_edges(
                        candidates,
                        query,
                        bounded,
                        omitted_sensitive,
                    ) {
                        Ok(projection) => GraphEdgesOutcome::Ready(projection),
                        Err(reason) => GraphEdgesOutcome::InvalidQuery { reason },
                    }
                }
                None => GraphEdgesOutcome::NotReady {
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

/// CE-6 query hygiene for `graph_edges`: the optional `file` filter, when set,
/// must be a valid workspace-relative path. `cursor` validity is checked in the
/// projector.
fn invalid_graph_edges_query_reason(query: &GraphEdgesQuery) -> Option<String> {
    match query.file.as_deref() {
        Some("") => Some("file must not be empty".to_string()),
        Some(file) => invalid_relative_path_reason("file", file),
        None => None,
    }
}

/// CE-6 query hygiene for `find_callers`: the `target` symbol identity is required
/// and its `file` must be a valid workspace-relative path. `max_depth` is clamped
/// (not rejected); `cursor` validity is checked in the projector.
fn invalid_find_callers_query_reason(query: &FindCallersQuery) -> Option<String> {
    let Some(target) = query.target.as_ref() else {
        return Some("target is required".to_string());
    };
    if target.name.is_empty() {
        return Some("target.name must not be empty".to_string());
    }
    if target.file.is_empty() {
        return Some("target.file must not be empty".to_string());
    }
    invalid_relative_path_reason("target.file", &target.file)
}

/// CIB-091b (CE-6 gap): validate a client-supplied `workspace_root` *before* it
/// reaches `PathBuf::from`/`canonicalize`. Rejects a NUL byte (which would make
/// `canonicalize` fail with an opaque IO error → `-32603 Internal`) and a value
/// over the [`MAX_WORKSPACE_ROOT_BYTES`] cap. Returns the rejection reason, or
/// `None` when acceptable. The per-verb admission gate (`authorise_root`) still
/// runs afterwards; this only guards the raw-string preconditions canonicalisation
/// cannot express cleanly.
///
/// CIB-091b refinement: a workspace ROOT is an absolute filesystem path, not a
/// substring query filter, so it is bounded by `PATH_MAX` (4096), not the 512-byte
/// per-param [`MAX_FILTER_BYTES`] query cap. The 512B cap stays on the actual
/// query/filter params (`invalid_relative_path_reason`, `invalid_query_reason`);
/// only this root check uses the larger `PATH_MAX`-appropriate bound so a
/// legitimately deep root is not wrongly rejected.
fn invalid_workspace_root_reason(workspace_root: &str) -> Option<String> {
    if workspace_root.is_empty() {
        return Some("workspace_root must not be empty".to_string());
    }
    if workspace_root.len() > MAX_WORKSPACE_ROOT_BYTES {
        return Some(format!(
            "workspace_root exceeds {MAX_WORKSPACE_ROOT_BYTES} bytes"
        ));
    }
    if workspace_root.contains('\0') {
        return Some("workspace_root must not contain a NUL byte".to_string());
    }
    None
}

/// `PATH_MAX`-appropriate cap for a `workspace_root` (CIB-091b refinement). A
/// root is an absolute filesystem path, so it is bounded by the conventional
/// `PATH_MAX` (4096) rather than the 512-byte per-param query-filter cap
/// ([`MAX_FILTER_BYTES`]) — a 512B bound wrongly rejected legitimately deep roots.
const MAX_WORKSPACE_ROOT_BYTES: usize = 4096;

/// The CE-7 assurance snapshot to ride along with a GCTX response when the
/// `workspace_root` is rejected before any state read (CIB-091b): the daemon
/// never reached the assurance machine, so it reports `Unavailable` /
/// `DaemonAbsent` — the same shape an absent graph carries.
fn unavailable_assurance() -> WorkspaceAssurance {
    WorkspaceAssurance {
        state: AssuranceState::Unavailable,
        reason: Some(StaleReason::DaemonAbsent),
        generation: 0,
        last_full_scan: None,
        scan_coverage: None,
    }
}

/// CE-6 per-path hygiene shared by the GCTX traversal verbs: a workspace-relative
/// path filter must be ≤ [`MAX_FILTER_BYTES`], NUL-free, non-absolute (Unix or
/// Windows-drive), free of `..` traversal components, and not scheme-prefixed
/// (`npm:`, `https:`, `data:`, …). `label` names the offending field in the
/// returned reason. Returns the rejection reason, or `None` when acceptable.
fn invalid_relative_path_reason(label: &str, value: &str) -> Option<String> {
    if value.len() > MAX_FILTER_BYTES {
        return Some(format!("{label} exceeds {MAX_FILTER_BYTES} bytes"));
    }
    if value.contains('\0') {
        return Some(format!("{label} must not contain a NUL byte"));
    }
    // Reject any absolute / rooted form: a Unix `/…`, a Windows drive `C:\…`, and
    // a leading slash/backslash (a `\\server\share` UNC root, which
    // `Path::is_absolute` does NOT flag on Unix). This keeps CE-6 validation in
    // lockstep with the egress `is_absolute_path_like` drop, so a rooted path can
    // never pass validation only to be silently dropped downstream.
    if Path::new(value).is_absolute()
        || has_windows_drive_absolute_prefix(value)
        || value.starts_with(['/', '\\'])
    {
        return Some(format!("{label} must be a workspace-relative path"));
    }
    if Path::new(value)
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Some(format!("{label} must not contain a `..` component"));
    }
    if has_uri_scheme_prefix(value) {
        return Some(format!(
            "{label} must not be scheme-prefixed (e.g. npm:, https:)"
        ));
    }
    None
}

/// Compute the GCTX impact-of-change outcome for an admitted root (GCTX-012 /
/// ADR-084). Mirrors [`gctx_find_dependents_outcome`]: same kill-switch,
/// query-validation, and CE-7 degradation arms. The read is a single
/// `with_graphs` over **both** warm graphs (symbol surface + dependent closure)
/// inside the lock; the report is sorted/sealed after release (C2).
fn gctx_impact_outcome(
    state: &SaveTimeState,
    key: &WorktreeKey,
    assurance: &WorkspaceAssurance,
    query: &ImpactQuery,
    egress_disabled: bool,
) -> ImpactOutcome {
    // CE-11 kill-switch.
    if egress_disabled {
        return ImpactOutcome::Disabled;
    }

    // CE-6: reject a hostile or malformed change set before touching the graph.
    if let Some(reason) = invalid_impact_query_reason(query) {
        return ImpactOutcome::InvalidQuery { reason };
    }

    // GV2-026 lever: clamp the dependent-closure depth into the ADR-063 envelope.
    let depth = clamp_reverse_impact_depth(query.max_depth.unwrap_or(1));
    let changed_files = query.changed_files.clone();

    match assurance.state {
        AssuranceState::Unavailable | AssuranceState::Unknown => ImpactOutcome::Unavailable,
        AssuranceState::Pending | AssuranceState::Running => ImpactOutcome::NotReady {
            recovery_hint: "the workspace graph is warming; retry the impact query shortly"
                .to_string(),
        },
        AssuranceState::Clean | AssuranceState::Stale | AssuranceState::Bounded => {
            // C2: collect under the lock (both graphs), project after release. The
            // `changed_files` / `truncated` counts come from `collect_impact`, so
            // the summary always reflects the seeds it actually walked.
            let collected = state.cache.with_graphs(key, |sym, dep| {
                GctxProjector::collect_impact(sym, dep, &changed_files, depth)
            });
            match collected {
                Some(collected) => ImpactOutcome::Ready(GctxProjector::project_impact(collected)),
                None => ImpactOutcome::NotReady {
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

/// CE-6 hygiene for `impact_of_change`, delegating to the shared change-set
/// validator.
fn invalid_impact_query_reason(query: &ImpactQuery) -> Option<String> {
    invalid_changed_files_reason(&query.changed_files)
}

/// Compute the GCTX affected-tests outcome for an admitted root (GCTX-013 /
/// ADR-084). Mirrors [`gctx_impact_outcome`]: same kill-switch, query-validation,
/// and CE-7 degradation arms. The read is a single `with_graphs` over the warm
/// dependency graph inside the lock (the symbol graph is unused — test
/// attribution is purely import-edge derived); the report is sorted/sealed after
/// release (C2).
fn gctx_affected_tests_outcome(
    state: &SaveTimeState,
    key: &WorktreeKey,
    assurance: &WorkspaceAssurance,
    query: &AffectedTestsQuery,
    egress_disabled: bool,
) -> AffectedTestsOutcome {
    // CE-11 kill-switch.
    if egress_disabled {
        return AffectedTestsOutcome::Disabled;
    }

    // CE-6: reject a hostile or malformed change set before touching the graph.
    if let Some(reason) = invalid_changed_files_reason(&query.changed_files) {
        return AffectedTestsOutcome::InvalidQuery { reason };
    }

    // GV2-026 lever: clamp the discovery depth into the ADR-063 envelope.
    let depth = clamp_reverse_impact_depth(query.max_depth.unwrap_or(1));
    let changed_files = query.changed_files.clone();

    match assurance.state {
        AssuranceState::Unavailable | AssuranceState::Unknown => AffectedTestsOutcome::Unavailable,
        AssuranceState::Pending | AssuranceState::Running => AffectedTestsOutcome::NotReady {
            recovery_hint: "the workspace graph is warming; retry the affected-tests query shortly"
                .to_string(),
        },
        AssuranceState::Clean | AssuranceState::Stale | AssuranceState::Bounded => {
            // C2: collect under the lock (dependency graph only), project after
            // release. The `changed_files` / `truncated` counts come from
            // `collect_affected_tests`, so the summary always reflects the seeds
            // it actually walked.
            let collected = state.cache.with_graphs(key, |_sym, dep| {
                GctxProjector::collect_affected_tests(dep, &changed_files, depth)
            });
            match collected {
                Some(collected) => {
                    AffectedTestsOutcome::Ready(GctxProjector::project_affected_tests(collected))
                }
                None => AffectedTestsOutcome::NotReady {
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

/// CE-2 secret-scan redaction choke point for snippet text (fail-closed: scan
/// errors and oversize-line skips redact the whole candidate).
fn redact_gctx_snippet(text: &str) -> Redaction {
    const FAIL_CLOSED_PLACEHOLDER: &str = "<REDACTED>";
    let config = SecretCheckConfig::default();
    let (findings, stats) = scan_content_with_stats(text, "<gctx-snippet>", &config);
    if stats.lines_skipped_oversize > 0 {
        return Redaction {
            text: FAIL_CLOSED_PLACEHOLDER.to_string(),
            redacted_hits: 1,
        };
    }
    if findings.is_empty() {
        return Redaction {
            text: text.to_string(),
            redacted_hits: 0,
        };
    }
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    for finding in &findings {
        if finding.line >= 1 && finding.line <= lines.len() {
            lines[finding.line - 1].clone_from(&finding.redacted_line);
        }
    }
    Redaction {
        text: lines.join("\n"),
        redacted_hits: u32::try_from(findings.len()).unwrap_or(u32::MAX),
    }
}

#[allow(clippy::too_many_arguments)]
fn gctx_get_snippet_outcome(
    state: &SaveTimeState,
    key: &WorktreeKey,
    assurance: &WorkspaceAssurance,
    query: &SnippetQuery,
    egress_disabled: bool,
    include_source: bool,
    read_file: impl Fn(&str) -> Option<Vec<u8>>,
    byte_ledger: &mut SnippetByteLedger,
) -> SnippetOutcome {
    if egress_disabled {
        return SnippetOutcome::Disabled;
    }
    if query.target.name.is_empty() {
        return SnippetOutcome::InvalidQuery {
            reason: "target.name must not be empty".to_string(),
        };
    }
    if let Some(reason) = invalid_relative_path_reason("target.file", &query.target.file) {
        return SnippetOutcome::InvalidQuery { reason };
    }

    match assurance.state {
        AssuranceState::Unavailable | AssuranceState::Unknown => SnippetOutcome::Unavailable,
        AssuranceState::Pending | AssuranceState::Running => SnippetOutcome::NotReady {
            recovery_hint: "the workspace graph is warming; retry the snippet query shortly"
                .to_string(),
        },
        AssuranceState::Clean | AssuranceState::Stale | AssuranceState::Bounded => {
            let resolved = state.cache.with_graphs(key, |sym, _dep| {
                GctxProjector::resolve_snippet_location(sym, &query.target)
            });
            let location = match resolved {
                None => {
                    return SnippetOutcome::NotReady {
                        recovery_hint: concat!(
                            "the workspace graph is not yet populated; ",
                            "save a file or request a full scan to warm it"
                        )
                        .to_string(),
                    };
                }
                Some(None) => return SnippetOutcome::SymbolNotFound,
                Some(Some(loc)) => loc,
            };
            let Some(bytes) = read_file(&location.file) else {
                return SnippetOutcome::SymbolNotFound;
            };
            let mut result = GctxProjector::project_snippet(
                &location,
                &bytes,
                include_source,
                redact_gctx_snippet,
            );
            if include_source && let Some(text) = result.text.as_ref() {
                let byte_cost = u32::try_from(text.len()).unwrap_or(u32::MAX);
                if byte_cost > 0 && !byte_ledger.can_admit(&result.file, result.span, byte_cost) {
                    result.text = None;
                } else if byte_cost > 0 {
                    byte_ledger.record(&result.file, result.span, byte_cost);
                }
            }
            SnippetOutcome::Ready(result)
        }
    }
}

fn clamp_symbol_context_token_budget(requested: Option<u32>) -> u32 {
    requested
        .unwrap_or(DEFAULT_SYMBOL_CONTEXT_TOKENS)
        .clamp(1, MAX_SYMBOL_CONTEXT_TOKENS)
}

fn invalid_symbol_context_query_reason(query: &SymbolContextQuery) -> Option<String> {
    let file = match &query.selector {
        ContextSelector::File { file } => file,
        ContextSelector::Symbol(id) => &id.file,
    };
    if file.is_empty() {
        return Some("selector file path must not be empty".to_string());
    }
    invalid_relative_path_reason("selector.file", file)
}

fn seal_symbol_context_outcome(projection: SymbolContextProjection) -> SymbolContextOutcome {
    if projection.redaction_summary.outcome == anvil_gctx_types::GctxOutcome::BudgetExceeded
        && projection
            .omitted_context
            .iter()
            .any(|o| o.reason == OmitReason::ByteCeiling)
    {
        SymbolContextOutcome::BudgetExceeded(projection)
    } else if projection
        .omitted_context
        .iter()
        .any(|o| o.reason == OmitReason::Budget)
    {
        SymbolContextOutcome::Bounded(projection)
    } else {
        SymbolContextOutcome::Ready(projection)
    }
}

#[allow(clippy::too_many_arguments)]
fn gctx_symbol_context_outcome(
    state: &SaveTimeState,
    key: &WorktreeKey,
    assurance: &WorkspaceAssurance,
    query: &SymbolContextQuery,
    egress_disabled: bool,
    include_source: bool,
    read_file: impl Fn(&str) -> Option<Vec<u8>>,
    byte_ledger: &mut SnippetByteLedger,
) -> SymbolContextOutcome {
    if egress_disabled {
        return SymbolContextOutcome::Disabled;
    }
    if let Some(reason) = invalid_symbol_context_query_reason(query) {
        return SymbolContextOutcome::InvalidQuery { reason };
    }
    let budget = clamp_symbol_context_token_budget(query.token_budget);

    match assurance.state {
        AssuranceState::Unavailable | AssuranceState::Unknown => SymbolContextOutcome::Unavailable,
        AssuranceState::Pending | AssuranceState::Running => SymbolContextOutcome::NotReady {
            recovery_hint: "the workspace graph is warming; retry the context query shortly"
                .to_string(),
        },
        AssuranceState::Clean | AssuranceState::Stale | AssuranceState::Bounded => {
            let collected = state.cache.with_graphs(key, |sym, dep| {
                let candidates =
                    GctxProjector::collect_context_candidates(sym, dep, &query.selector);
                let mut locations = std::collections::HashMap::new();
                for (identity, _) in &candidates {
                    if let Some(loc) = GctxProjector::resolve_snippet_location(sym, identity) {
                        locations.insert(identity.clone(), loc);
                    }
                }
                (candidates, locations)
            });
            let Some((candidates, locations)) = collected else {
                return SymbolContextOutcome::NotReady {
                    recovery_hint: concat!(
                        "the workspace graph is not yet populated; ",
                        "save a file or request a full scan to warm it"
                    )
                    .to_string(),
                };
            };
            let mut file_bytes = std::collections::HashMap::new();
            for (identity, _) in &candidates {
                if let Some(loc) = locations.get(identity) {
                    file_bytes
                        .entry(loc.file.clone())
                        .or_insert_with(|| read_file(&loc.file).unwrap_or_default());
                }
            }
            let projection = GctxProjector::project_symbol_context(
                candidates,
                &locations,
                &file_bytes,
                include_source,
                budget,
                redact_gctx_snippet,
                Some(byte_ledger),
            );
            seal_symbol_context_outcome(projection)
        }
    }
}

/// CE-6 hygiene shared by the change-set GCTX verbs (`impact_of_change`,
/// `affected_tests`). Rejects an empty or over-cap change set
/// (≤ [`MAX_CHANGED_FILES`]) and any malformed changed-file path **before** the
/// graph is read, reusing [`invalid_relative_path_reason`] per path. Returns the
/// rejection reason, or `None`.
fn invalid_changed_files_reason(changed_files: &[String]) -> Option<String> {
    if let Some(reason) = anvil_gctx_types::invalid_changed_files_structure(changed_files) {
        return Some(reason);
    }
    for file in changed_files {
        if let Some(reason) = invalid_relative_path_reason("changed file path", file) {
            return Some(reason);
        }
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
        // 095a: route the path-like `file` filter through the shared per-path
        // hygiene so `search_symbols` rejects the same rooted forms the sibling
        // GCTX verbs (e.g. `impact_of_change`) do — including a leading-slash /
        // backslash `\\server\share` UNC root, which `Path::is_absolute` does
        // NOT flag on Unix. The byte-cap / NUL checks above are retained because
        // they also cover the non-path `name`/`language` filters.
        if let Some(reason) = invalid_relative_path_reason("file filter", file) {
            return Some(reason);
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
    use anvil_gctx_types::MAX_CHANGED_FILES;
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
                    span: None,
                })
                .collect(),
            imports: Vec::new(),
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
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
        assert!(
            !seen.is_empty(),
            "the parser should be invoked for the edited path",
        );
        assert!(
            seen.iter()
                .all(|(path, bytes)| path == "src/a.ts" && bytes == &body),
            "every observed parser call got the bytes the daemon read; seen={seen:?}",
        );
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

    /// CIB-091b (CE-6 gap): a `workspace_root` carrying a NUL byte is rejected as
    /// a structured `InvalidQuery` *before* `canonicalize`, not surfaced as a
    /// generic `-32603 Internal` (which a raw `canonicalize` of a NUL path would
    /// produce).
    #[test]
    fn gctx_search_rejects_nul_workspace_root() {
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let request = GctxSearchSymbolsRequest {
            workspace_root: "/tmp/proj\0/evil".to_string(),
            query: SearchSymbolsQuery::default(),
        };
        let resp = conn.search_symbols(&request).expect("validated in-band");
        assert!(
            matches!(resp.outcome, SearchSymbolsOutcome::InvalidQuery { .. }),
            "a NUL-bearing workspace_root must be a structured InvalidQuery: {:?}",
            resp.outcome
        );
    }

    /// CIB-091b refinement: a `workspace_root` over the PATH_MAX-appropriate cap
    /// (4096 bytes) is rejected as a structured `InvalidQuery` before any
    /// canonicalisation/admission. A root is an absolute filesystem path, not a
    /// 512-byte query filter, so the bound is `PATH_MAX`, not `MAX_FILTER_BYTES`.
    #[test]
    fn gctx_search_rejects_oversized_workspace_root() {
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let request = GctxSearchSymbolsRequest {
            // > 4096 bytes: over the PATH_MAX-appropriate root cap.
            workspace_root: format!("/tmp/{}", "a".repeat(4100)),
            query: SearchSymbolsQuery::default(),
        };
        let resp = conn.search_symbols(&request).expect("validated in-band");
        assert!(
            matches!(resp.outcome, SearchSymbolsOutcome::InvalidQuery { .. }),
            "an over-cap (>4096B) workspace_root must be a structured InvalidQuery: {:?}",
            resp.outcome
        );
    }

    /// CIB-091b refinement: a legitimately deep `workspace_root` (~1KB — well over
    /// the old 512-byte filter cap but under `PATH_MAX`) is NOT rejected by the
    /// pre-canonicalise size check. It is a non-existent path, so admission later
    /// degrades it to a CE-7 `Unavailable`/`DaemonAbsent` outcome — what matters
    /// here is that it is NOT an `InvalidQuery` size rejection.
    #[test]
    fn gctx_search_accepts_deep_workspace_root_under_path_max() {
        assert_eq!(
            invalid_workspace_root_reason(&format!("/tmp/{}", "a".repeat(1000))),
            None,
            "a ~1KB root (over 512B, under PATH_MAX) must not be size-rejected"
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

    /// 095a: a `\\server\share\…` UNC root must be rejected by `search_symbols`
    /// the same way `impact_of_change` rejects it. `Path::is_absolute` is false
    /// for a UNC root on Unix, so the rejection relies on the leading-separator
    /// check the sibling `invalid_relative_path_reason` already applies.
    #[test]
    fn gctx_search_rejects_unc_file_filter() {
        let tmp = tempfile::tempdir().expect("tempdir");
        for bad in ["\\\\server\\share\\x.ts", "/abs.ts"] {
            let outcome = gctx_invalid_query(
                tmp.path(),
                SearchSymbolsQuery {
                    file: Some(bad.to_string()),
                    ..Default::default()
                },
            );
            assert!(
                matches!(outcome, SearchSymbolsOutcome::InvalidQuery { .. }),
                "a rooted/UNC file filter {bad:?} must be rejected: {outcome:?}",
            );
        }
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

    // --- GCTX-014 find_callers (daemon wiring) ---

    fn callers_request(root: &Path, name: &str, file: &str) -> GctxFindCallersRequest {
        GctxFindCallersRequest {
            workspace_root: root.to_string_lossy().into_owned(),
            query: FindCallersQuery {
                target: Some(anvil_kernel_types::SymbolIdentity {
                    file: file.to_string(),
                    kind: anvil_kernel_types::SymbolKind::Function,
                    name: name.to_string(),
                    ordinal: 0,
                }),
                ..Default::default()
            },
        }
    }

    /// Warm a file whose `caller` function calls the same-file `callee`, building
    /// the resident `Calls` edge the caller traversal reads.
    fn warm_with_call(state: &SaveTimeState, root: &Path, file: &str) {
        use anvil_kernel_types::{CallSite, CalleeRef, LocalSymbolRef};
        let key = WorktreeKey::from_canonical(std::fs::canonicalize(root).expect("canonical"));
        let mut symbols = file_symbols(file, &["callee", "caller"], 0);
        symbols.calls = vec![CallSite {
            from: LocalSymbolRef {
                kind: anvil_kernel_types::SymbolKind::Function,
                name: "caller".to_string(),
                ordinal: 0,
                module_scope: false,
            },
            callee: CalleeRef {
                name: "callee".to_string(),
                via_import: None,
            },
            line: 1,
        }];
        state.cache.apply_delta(&key, ChangeKind::Create, symbols);
    }

    /// A warm worktree resolves the caller set: `caller` calling `callee` makes
    /// `find_callers(callee)` report `caller` at distance 1, not heuristic.
    #[test]
    fn gctx_callers_ready_reports_caller_when_warm() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm_with_call(&state, tmp.path(), "a.ts");

        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .find_callers(&callers_request(tmp.path(), "callee", "a.ts"))
            .expect("admitted");

        match resp.outcome {
            FindCallersOutcome::Ready(projection) => {
                let callers: Vec<(&str, u32, bool)> = projection
                    .callers
                    .iter()
                    .map(|c| (c.caller.name.as_str(), c.distance, c.heuristic))
                    .collect();
                assert_eq!(callers, [("caller", 1, false)]);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    /// CE-6: a `find_callers` query with no target is a structured `InvalidQuery`.
    #[test]
    fn gctx_callers_rejects_missing_target() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm_with_call(&state, tmp.path(), "a.ts");
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .find_callers(&GctxFindCallersRequest {
                workspace_root: tmp.path().to_string_lossy().into_owned(),
                query: FindCallersQuery::default(),
            })
            .expect("admitted");
        assert!(matches!(
            resp.outcome,
            FindCallersOutcome::InvalidQuery { .. }
        ));
    }

    /// CE-7: a cold worktree degrades to `NotReady`, never an empty `Ready`.
    #[test]
    fn gctx_callers_not_ready_on_cold_worktree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .find_callers(&callers_request(tmp.path(), "callee", "a.ts"))
            .expect("admitted");
        assert!(matches!(resp.outcome, FindCallersOutcome::NotReady { .. }));
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

    // --- GCTX-012 impact_of_change (daemon wiring) ---

    fn impact_request(root: &Path, changed: &[&str]) -> GctxImpactOfChangeRequest {
        GctxImpactOfChangeRequest {
            workspace_root: root.to_string_lossy().into_owned(),
            query: ImpactQuery {
                changed_files: changed.iter().map(ToString::to_string).collect(),
                max_depth: None,
            },
        }
    }

    /// A warm worktree yields a `Ready` report: symbols defined in the changed
    /// file (affected), its importers (dependent files), and the test subset.
    #[test]
    fn gctx_impact_ready_reports_blast_radius_when_warm() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        warm_with_import(&state, tmp.path(), "b.ts", &["beta"], "./a", 100);
        warm_with_import(&state, tmp.path(), "a.test.ts", &["t"], "./a", 200);

        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .impact_of_change(&impact_request(tmp.path(), &["a.ts"]))
            .expect("admitted");
        match resp.outcome {
            ImpactOutcome::Ready(report) => {
                assert_eq!(report.affected_symbols.len(), 1);
                assert_eq!(report.affected_symbols[0].identity.name, "alpha");
                let deps: Vec<&str> = report
                    .dependent_files
                    .iter()
                    .map(|d| d.file.as_str())
                    .collect();
                assert_eq!(deps, ["a.test.ts", "b.ts"]);
                assert_eq!(report.known_tests, ["a.test.ts"]);
                assert_eq!(report.summary.changed_files, 1);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    /// CE-7: a cold worktree degrades to `NotReady`, never an empty `Ready`.
    #[test]
    fn gctx_impact_not_ready_on_cold_worktree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .impact_of_change(&impact_request(tmp.path(), &["a.ts"]))
            .expect("admitted");
        assert!(
            matches!(resp.outcome, ImpactOutcome::NotReady { .. }),
            "cold worktree must degrade to NotReady: {:?}",
            resp.outcome
        );
    }

    /// CE-11 kill-switch: a disabled surface self-reports `Disabled` even warm.
    #[test]
    fn gctx_impact_kill_switch_disables_egress() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        let key =
            WorktreeKey::from_canonical(std::fs::canonicalize(tmp.path()).expect("canonical"));
        let clean = assurance(AssuranceState::Clean, None);
        let disabled = gctx_impact_outcome(
            &state,
            &key,
            &clean,
            &ImpactQuery {
                changed_files: vec!["a.ts".into()],
                max_depth: None,
            },
            true,
        );
        assert!(matches!(disabled, ImpactOutcome::Disabled));
    }

    /// CE-6: an empty change set and an over-cap change set are both rejected.
    #[test]
    fn gctx_impact_rejects_empty_and_over_cap_change_sets() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        let mut conn = SaveTimeConn::new(&state);

        let empty = conn
            .impact_of_change(&impact_request(tmp.path(), &[]))
            .expect("admitted");
        assert!(matches!(empty.outcome, ImpactOutcome::InvalidQuery { .. }));

        let over: Vec<String> = (0..=MAX_CHANGED_FILES)
            .map(|i| format!("src/f{i}.ts"))
            .collect();
        let over_req = GctxImpactOfChangeRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            query: ImpactQuery {
                changed_files: over,
                max_depth: None,
            },
        };
        let over_resp = conn.impact_of_change(&over_req).expect("admitted");
        assert!(matches!(
            over_resp.outcome,
            ImpactOutcome::InvalidQuery { .. }
        ));
    }

    /// CE-6: a `..`-escaping changed path is rejected before any read.
    #[test]
    fn gctx_impact_rejects_path_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .impact_of_change(&impact_request(tmp.path(), &["../escape.ts"]))
            .expect("admitted");
        assert!(
            matches!(resp.outcome, ImpactOutcome::InvalidQuery { .. }),
            "a `..` changed path must be rejected: {:?}",
            resp.outcome
        );
    }

    /// CE-6: a rooted changed path (leading `/`, or a `\\server\share` UNC root
    /// that `Path::is_absolute` misses on Unix) is rejected — keeping validation
    /// in lockstep with the egress absolute-path drop so the count stays honest.
    #[test]
    fn gctx_impact_rejects_rooted_and_unc_paths() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        let mut conn = SaveTimeConn::new(&state);
        for bad in ["/abs.ts", "\\\\server\\share\\x.ts"] {
            let resp = conn
                .impact_of_change(&impact_request(tmp.path(), &["a.ts", bad]))
                .expect("admitted");
            assert!(
                matches!(resp.outcome, ImpactOutcome::InvalidQuery { .. }),
                "a rooted/UNC path {bad:?} must be rejected: {:?}",
                resp.outcome
            );
        }
    }

    /// C3 / CE-8: an unadmitted root is refused daemon-side before projection.
    #[test]
    fn gctx_impact_rejects_unadmitted_root() {
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
        conn.impact_of_change(&impact_request(primary.path(), &["a.ts"]))
            .expect("primary root is implicitly admitted");
        let refused = conn.impact_of_change(&impact_request(unlisted.path(), &["a.ts"]));
        assert!(
            matches!(refused, Err(SaveTimeError::NotAdmitted)),
            "an unadmitted root must be refused: {refused:?}",
        );
    }

    // --- GCTX-013 affected_tests (daemon wiring) ---

    fn affected_tests_request(root: &Path, changed: &[&str]) -> GctxAffectedTestsRequest {
        GctxAffectedTestsRequest {
            workspace_root: root.to_string_lossy().into_owned(),
            query: AffectedTestsQuery {
                changed_files: changed.iter().map(ToString::to_string).collect(),
                max_depth: None,
            },
        }
    }

    /// A warm worktree yields a `Ready` report: a test importing the changed
    /// source appears with an evidence edge, and a second changed source with no
    /// test is a coverage gap.
    #[test]
    fn gctx_affected_tests_ready_attributes_tests_and_gaps_when_warm() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "s.ts", &["s"], 0);
        warm(&state, tmp.path(), "u.ts", &["u"], 100);
        warm_with_import(&state, tmp.path(), "s.test.ts", &["t"], "./s", 200);

        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .affected_tests(&affected_tests_request(tmp.path(), &["s.ts", "u.ts"]))
            .expect("admitted");
        match resp.outcome {
            AffectedTestsOutcome::Ready(report) => {
                assert!(report.heuristic);
                assert_eq!(report.tests.len(), 1);
                assert_eq!(report.tests[0].file, "s.test.ts");
                assert_eq!(report.tests[0].changed_dependencies, ["s.ts"]);
                assert_eq!(report.tests[0].distance, 1);
                assert_eq!(report.coverage_gaps, ["u.ts"]);
                assert_eq!(report.summary.changed_files, 2);
                assert_eq!(report.summary.evidence_edges, 1);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    /// CE-7: a cold worktree degrades to `NotReady`, never an empty `Ready`.
    #[test]
    fn gctx_affected_tests_not_ready_on_cold_worktree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .affected_tests(&affected_tests_request(tmp.path(), &["a.ts"]))
            .expect("admitted");
        assert!(
            matches!(resp.outcome, AffectedTestsOutcome::NotReady { .. }),
            "cold worktree must degrade to NotReady: {:?}",
            resp.outcome
        );
    }

    /// CE-11 kill-switch: a disabled surface self-reports `Disabled` even warm.
    #[test]
    fn gctx_affected_tests_kill_switch_disables_egress() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        let key =
            WorktreeKey::from_canonical(std::fs::canonicalize(tmp.path()).expect("canonical"));
        let clean = assurance(AssuranceState::Clean, None);
        let disabled = gctx_affected_tests_outcome(
            &state,
            &key,
            &clean,
            &AffectedTestsQuery {
                changed_files: vec!["a.ts".into()],
                max_depth: None,
            },
            true,
        );
        assert!(matches!(disabled, AffectedTestsOutcome::Disabled));
    }

    /// CE-6: an empty change set and an over-cap change set are both rejected.
    #[test]
    fn gctx_affected_tests_rejects_empty_and_over_cap_change_sets() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        warm(&state, tmp.path(), "a.ts", &["alpha"], 0);
        let mut conn = SaveTimeConn::new(&state);

        let empty = conn
            .affected_tests(&affected_tests_request(tmp.path(), &[]))
            .expect("admitted");
        assert!(matches!(
            empty.outcome,
            AffectedTestsOutcome::InvalidQuery { .. }
        ));

        let over: Vec<String> = (0..=MAX_CHANGED_FILES)
            .map(|i| format!("src/f{i}.ts"))
            .collect();
        let over_req = GctxAffectedTestsRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            query: AffectedTestsQuery {
                changed_files: over,
                max_depth: None,
            },
        };
        let over_resp = conn.affected_tests(&over_req).expect("admitted");
        assert!(matches!(
            over_resp.outcome,
            AffectedTestsOutcome::InvalidQuery { .. }
        ));
    }

    /// CE-6: a `..`-escaping changed path is rejected before any read.
    #[test]
    fn gctx_affected_tests_rejects_path_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .affected_tests(&affected_tests_request(tmp.path(), &["../escape.ts"]))
            .expect("admitted");
        assert!(
            matches!(resp.outcome, AffectedTestsOutcome::InvalidQuery { .. }),
            "a `..` changed path must be rejected: {:?}",
            resp.outcome
        );
    }

    /// C3 / CE-8: an unadmitted root is refused daemon-side before projection.
    #[test]
    fn gctx_affected_tests_rejects_unadmitted_root() {
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
        conn.affected_tests(&affected_tests_request(primary.path(), &["a.ts"]))
            .expect("primary root is implicitly admitted");
        let refused = conn.affected_tests(&affected_tests_request(unlisted.path(), &["a.ts"]));
        assert!(
            matches!(refused, Err(SaveTimeError::NotAdmitted)),
            "an unadmitted root must be refused: {refused:?}",
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

    /// CIB-095c: pure resolution of the scoped scan opt-out mirrors the egress
    /// kill-switch contract — trimmed `0` disables; unset/other leaves it on.
    #[test]
    fn implicit_scan_disabled_only_on_trimmed_zero() {
        assert!(implicit_scan_disabled(Some("0")));
        assert!(implicit_scan_disabled(Some(" 0")));
        assert!(implicit_scan_disabled(Some("0\n")));
        assert!(!implicit_scan_disabled(None)); // unset → scan on (default)
        assert!(!implicit_scan_disabled(Some("1")));
        assert!(!implicit_scan_disabled(Some("")));
        assert!(!implicit_scan_disabled(Some("false")));
    }

    /// CIB-095c: with the scoped opt-out set, a cold-key implicit `Background`
    /// auto-warm does NOT enqueue a scan (the machine is left at its prior
    /// `Stale` state); an explicit `Interactive` request is still honoured.
    #[test]
    fn implicit_background_scan_is_suppressed_by_scoped_opt_out() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = state();
        let key =
            WorktreeKey::from_canonical(std::fs::canonicalize(tmp.path()).expect("canonical"));

        // A fresh cold key is `Stale` (never clean before a scan, B6).
        assert_eq!(
            state.machine_handle(&key).lock().expect("lock").state(),
            AssuranceState::Stale,
        );

        // Opt-out set → the implicit Background auto-warm is a no-op: no enqueue,
        // machine untouched.
        state.spawn_scan_gated(&key, tmp.path(), ScanPriority::Background, true);
        assert!(
            !state.scan_coordinator().is_enqueued(&key),
            "the scoped opt-out must suppress the implicit background scan",
        );
        assert_eq!(
            state.machine_handle(&key).lock().expect("lock").state(),
            AssuranceState::Stale,
            "a suppressed scan must not transition the machine to Pending",
        );

        // An explicit Interactive request is never suppressed by the opt-out.
        state.spawn_scan_gated(&key, tmp.path(), ScanPriority::Interactive, true);
        assert_ne!(
            state.machine_handle(&key).lock().expect("lock").state(),
            AssuranceState::Stale,
            "an explicit request_full_scan must still be honoured under the opt-out",
        );
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
        let tmp = tempfile::tempdir().expect("tempdir");
        // The writer creates the `graph-cache` subdir owner-only 0700 (the
        // tempdir root may be group-readable, which the dir-security check
        // rejects — production always points at a `graph-cache` subdir).
        let dir = tmp.path().join("graph-cache");
        let root = std::path::PathBuf::from("/ws-restart");
        let key = WorktreeKey::from_canonical(root.clone());

        // Daemon 1: warm a key, then persist on shutdown.
        let state1 = state().with_snapshot_dir(dir.clone());
        state1.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["alpha"], 0),
        );
        assert!(state1.cache.contains(&key));
        state1.persist_all_on_shutdown();
        assert!(
            std::fs::read_dir(&dir).unwrap().next().is_some(),
            "a snapshot file must be written on shutdown",
        );

        // Daemon 2 (fresh cache, same snapshot dir): cold until restore.
        let state2 = state().with_snapshot_dir(dir.clone());
        assert!(!state2.cache.contains(&key), "fresh daemon starts cold");

        restore_snapshot_into_cache(
            &state2.cache,
            state2.scan_coordinator(),
            &state2.snapshot_metrics,
            &dir,
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
        let gc = dir.path().join("graph-cache");
        assert!(
            !gc.exists(),
            "persistence-off must not even create the snapshot dir",
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "persistence-off must write nothing under the state dir",
        );

        // Positive control (non-vacuous): the SAME dir DOES receive a snapshot
        // once persistence is wired — proving the OFF assertion above is meaningful
        // (the dir is the real target, not an unrelated path).
        let on = state().with_snapshot_dir(gc.clone());
        assert!(on.persistence_enabled());
        on.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["a"], 0),
        );
        on.persist_all_on_shutdown();
        assert!(
            std::fs::read_dir(&gc).unwrap().next().is_some(),
            "persistence-on must write the snapshot to the wired dir",
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
            &state.snapshot_metrics,
            dir.path(),
            &key,
            std::path::Path::new("/ws-none"),
        );
        assert!(!state.cache.contains(&key), "no snapshot ⇒ stays cold");
    }

    // ---- CIB-092f: ADR-069 §3 verdict-gate end-to-end ----

    /// The §3 safety property, end-to-end: after a snapshot is restored into the
    /// cache, a `validate_paths` in the **restore→reconcile window** (before any
    /// reconcile scan has certified) serves a **non-Certified** (`Stale`) verdict —
    /// the restored graph is a read-only stand-in, never trusted as certified. The
    /// restore populates the cache, but the assurance machine stays `Stale`, so the
    /// verdict is re-derived rather than carried across the restart.
    #[cfg(unix)]
    #[test]
    fn restored_snapshot_serves_stale_verdict_until_reconcile() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        fs::create_dir(&src).expect("mkdir");
        fs::write(src.join("a.ts"), b"export function alpha() {}").expect("write");
        let dir = tmp.path().join("graph-cache");
        // The cache + machine key on the CANONICAL root (what validate_paths uses).
        let canonical = std::fs::canonicalize(tmp.path()).expect("canonicalize");
        let key = WorktreeKey::from_canonical(canonical.clone());

        // Stage a snapshot on disk for this worktree, then restore it.
        let writer = state().with_snapshot_dir(dir.clone());
        writer.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["alpha"], 0),
        );
        writer.persist_all_on_shutdown();

        let state = state().with_snapshot_dir(dir.clone());
        restore_snapshot_into_cache(
            &state.cache,
            state.scan_coordinator(),
            &state.snapshot_metrics,
            &dir,
            &key,
            &canonical,
        );
        assert!(state.cache.contains(&key), "restore populated the cache");

        // The verdict in the restore→reconcile window must NOT be Certified — the
        // restored entry is stale until a reconcile scan rebuilds it.
        let mut conn = SaveTimeConn::new(&state);
        let resp = conn
            .validate_paths(&ValidatePathsRequest {
                workspace_root: tmp.path().to_string_lossy().into_owned(),
                paths: vec![modified("src/a.ts")],
            })
            .expect("admitted");
        assert_eq!(
            resp.workspace_assurance.state,
            AssuranceState::Stale,
            "a restored-but-not-reconciled worktree must serve Stale, never Certified/Clean",
        );
    }

    /// N8 (CIB-095): a graph-reading GCTX verb OTHER than `search_symbols`
    /// (`find_dependents` here) triggers the first-contact warm-start restore on a
    /// cold key — previously only `search_symbols` did. With a staged snapshot the
    /// background restore warms the cache so a follow-up read is served the
    /// restored graph rather than `NotReady`.
    #[cfg(unix)]
    #[test]
    fn gctx_find_dependents_triggers_first_contact_restore() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("graph-cache");
        let canonical = std::fs::canonicalize(tmp.path()).expect("canonicalize");
        let key = WorktreeKey::from_canonical(canonical.clone());

        // Stage a snapshot on disk for this worktree.
        let writer = state().with_snapshot_dir(dir.clone());
        writer.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["alpha"], 0),
        );
        writer.persist_all_on_shutdown();

        // A fresh daemon (cold cache) wired to the same snapshot dir.
        let state = state().with_snapshot_dir(dir.clone());
        assert!(!state.cache.contains(&key), "fresh daemon starts cold");

        // `find_dependents` on the cold key must now kick off the background
        // restore (N8: not just `search_symbols`).
        let mut conn = SaveTimeConn::new(&state);
        let _ = conn
            .find_dependents(&dependents_request(tmp.path(), "src/a.ts", None))
            .expect("admitted");

        // The restore runs on the background pool; poll briefly for it to land.
        let warmed = (0..200).any(|_| {
            if state.cache.contains(&key) {
                true
            } else {
                std::thread::sleep(std::time::Duration::from_millis(5));
                false
            }
        });
        assert!(
            warmed,
            "find_dependents on a cold key must trigger the warm-start restore",
        );
    }

    // ---- CIB-096: orphan `.snap` startup reclaim (existence-based) ----

    /// A snapshot whose worktree root still exists is kept; a snapshot whose root
    /// was deleted while the daemon was down is reclaimed via its `.root`
    /// companion. Off-by-default: no dir wired ⇒ a no-op.
    #[cfg(unix)]
    #[test]
    fn sweep_orphan_snapshots_keeps_live_drops_gone() {
        use anvil_graph_cache::snapshot::snapshot_filename;
        let tmp = tempfile::tempdir().expect("tempdir");
        let gc = tmp.path().join("graph-cache");
        // A live worktree (still on disk) and one deleted while the daemon was down.
        let live = tmp.path().join("live");
        let gone = tmp.path().join("gone");
        fs::create_dir(&live).unwrap();
        fs::create_dir(&gone).unwrap();
        let live_c = fs::canonicalize(&live).unwrap();
        let gone_c = fs::canonicalize(&gone).unwrap();

        // Off-by-default: with no dir wired the sweep does nothing.
        let off = state();
        assert_eq!(off.sweep_orphan_snapshots_on_start(), 0);

        let on = state().with_snapshot_dir(gc.clone());
        // Warm both keys then flush on shutdown (publishes `.snap` + `.root`
        // companions through the real write path).
        let live_key = WorktreeKey::from_canonical(live_c.clone());
        let gone_key = WorktreeKey::from_canonical(gone_c.clone());
        on.cache.apply_delta(
            &live_key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["alpha"], 0),
        );
        on.cache.apply_delta(
            &gone_key,
            ChangeKind::Create,
            file_symbols("src/b.ts", &["beta"], 0),
        );
        on.persist_all_on_shutdown();
        // Now the second worktree disappears.
        fs::remove_dir(&gone_c).unwrap();

        let removed = on.sweep_orphan_snapshots_on_start();
        assert_eq!(removed, 1, "only the deleted worktree's snapshot drops");
        assert!(gc.join(snapshot_filename(&live_c)).exists());
        assert!(!gc.join(snapshot_filename(&gone_c)).exists());
    }

    // ---- CIB-092b: ADR-069 §10 snapshot I/O metric counters ----

    /// A successful write (shutdown flush) then a successful load (warm-start
    /// restore) each advance the right counter; nothing else moves. This is the
    /// happy path the §7b soak measures `load_ok`/`write_ok` against.
    #[cfg(unix)]
    #[test]
    fn metrics_count_a_successful_write_then_load() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("graph-cache");
        let root = std::path::PathBuf::from("/ws-metrics-ok");
        let key = WorktreeKey::from_canonical(root.clone());

        let writer = state().with_snapshot_dir(dir.clone());
        writer.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["alpha"], 0),
        );
        writer.persist_all_on_shutdown();
        let m = writer.snapshot_metrics();
        assert_eq!(m.write_ok, 1, "one successful write counted");
        assert_eq!(m.write_error, 0);

        // Fresh daemon: restore from the same dir → one `load_ok`.
        let reader = state().with_snapshot_dir(dir.clone());
        restore_snapshot_into_cache(
            &reader.cache,
            reader.scan_coordinator(),
            &reader.snapshot_metrics,
            &dir,
            &key,
            &root,
        );
        let m = reader.snapshot_metrics();
        assert_eq!(m.load_ok, 1, "one accepted load counted");
        assert_eq!(
            (
                m.load_corrupt,
                m.load_absent,
                m.load_io,
                m.load_version_mismatch
            ),
            (0, 0, 0, 0),
            "an accepted load must not trip any error counter",
        );
    }

    /// A planted-garbage `.snap` increments `load_corrupt` — the soak's
    /// graduation-blocking class — and the restore is a no-op (cold rebuild).
    #[cfg(unix)]
    #[test]
    fn metrics_count_a_corrupt_load() {
        use anvil_graph_cache::snapshot::snapshot_filename;
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().to_path_buf();
        let root = std::path::PathBuf::from("/ws-metrics-corrupt");
        let key = WorktreeKey::from_canonical(root.clone());
        // A file at the snapshot path that is not a valid snapshot ⇒ Rejected(Corrupt).
        fs::write(dir.join(snapshot_filename(&root)), b"not a snapshot").unwrap();

        let state = state().with_snapshot_dir(dir.clone());
        restore_snapshot_into_cache(
            &state.cache,
            state.scan_coordinator(),
            &state.snapshot_metrics,
            &dir,
            &key,
            &root,
        );
        let m = state.snapshot_metrics();
        assert_eq!(m.load_corrupt, 1, "a corrupt snapshot must be counted");
        assert_eq!(m.load_ok, 0);
        assert!(
            !state.cache.contains(&key),
            "a corrupt load is a cold-rebuild no-op"
        );
    }

    /// CIB-092 council survivor (item 1): the cumulative-metrics emitter carries
    /// EVERY counter so a soak harness scraping the shutdown event reads the full
    /// `SnapshotMetricsSnapshot`. A field-capturing tracing layer asserts the
    /// `"anvil_intercept::snapshot"` event fires with the exact cumulative values.
    #[cfg(unix)]
    #[test]
    fn cumulative_metrics_emit_carries_every_counter() {
        use std::sync::{Arc as StdArc, Mutex as StdMutex};
        use tracing::field::{Field, Visit};
        use tracing::subscriber::with_default;
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::SubscriberExt;

        // A layer that captures u64 fields + the message of any event on the
        // snapshot target, so the test asserts on structured values not text.
        #[derive(Default)]
        struct Captured {
            fields: std::collections::HashMap<String, u64>,
            message: String,
            fired: bool,
        }
        struct CaptureLayer(StdArc<StdMutex<Captured>>);
        struct FieldVisitor<'a>(&'a mut Captured);
        impl Visit for FieldVisitor<'_> {
            fn record_u64(&mut self, field: &Field, value: u64) {
                self.0.fields.insert(field.name().to_string(), value);
            }
            fn record_i64(&mut self, field: &Field, value: i64) {
                if let Ok(v) = u64::try_from(value) {
                    self.0.fields.insert(field.name().to_string(), v);
                }
            }
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0.message = format!("{value:?}");
                }
            }
        }
        impl<S: tracing::Subscriber> Layer<S> for CaptureLayer {
            fn enabled(
                &self,
                _metadata: &tracing::Metadata<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) -> bool {
                true
            }
            fn max_level_hint(&self) -> Option<tracing::level_filters::LevelFilter> {
                Some(tracing::level_filters::LevelFilter::TRACE)
            }
            fn on_event(
                &self,
                event: &tracing::Event<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                if event.metadata().target() != "anvil_intercept::snapshot" {
                    return;
                }
                let mut cap = self.0.lock().unwrap();
                // Only the cumulative-metrics event carries `load_corrupt`.
                let mut probe = Captured::default();
                event.record(&mut FieldVisitor(&mut probe));
                if probe.fields.contains_key("load_corrupt") {
                    cap.fields = probe.fields;
                    cap.message = probe.message;
                    cap.fired = true;
                }
            }
        }

        let captured = StdArc::new(StdMutex::new(Captured::default()));
        let subscriber =
            tracing_subscriber::registry().with(CaptureLayer(StdArc::clone(&captured)));

        with_default(subscriber, || {
            // Seed the counters with a mixed cumulative state, then drive a real
            // shutdown flush (one successful write) so the emit reflects reality.
            let tmp = tempfile::tempdir().expect("tempdir");
            let dir = tmp.path().join("graph-cache");
            let key = WorktreeKey::from_canonical(std::path::PathBuf::from("/ws-emit"));
            let state = state().with_snapshot_dir(dir);
            // Pre-bump load counters so the emit must carry non-zero cumulative load
            // values, not just the write it does itself.
            state
                .snapshot_metrics
                .record_load(&Err(crate::snapshot_io::SnapshotReadError::NotFound));
            state.snapshot_metrics.record_load(&Err(
                crate::snapshot_io::SnapshotReadError::Rejected(
                    anvil_graph_cache::snapshot::SnapshotLoadError::Corrupt,
                ),
            ));
            state.cache.apply_delta(
                &key,
                ChangeKind::Create,
                file_symbols("src/a.ts", &["alpha"], 0),
            );
            state.persist_all_on_shutdown();
        });

        let cap = captured.lock().unwrap();
        assert!(cap.fired, "the cumulative-metrics shutdown event must fire");
        assert_eq!(cap.fields.get("write_ok"), Some(&1), "write_ok cumulative");
        assert_eq!(cap.fields.get("write_error"), Some(&0));
        assert_eq!(cap.fields.get("load_absent"), Some(&1));
        assert_eq!(
            cap.fields.get("load_corrupt"),
            Some(&1),
            "the soak's graduation-blocking counter must ride the shutdown event",
        );
        assert_eq!(cap.fields.get("load_ok"), Some(&0));
        assert!(
            cap.message.contains("cumulative snapshot I/O metrics"),
            "the event message identifies the soak readout: {}",
            cap.message,
        );
    }

    /// A missing snapshot (the normal first-run case) counts as `absent`, never
    /// `corrupt` — so it never pollutes the §7b "zero Corrupt" graduation signal.
    #[cfg(unix)]
    #[test]
    fn metrics_count_an_absent_load_as_absent_not_corrupt() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().to_path_buf();
        let root = std::path::PathBuf::from("/ws-absent");
        let key = WorktreeKey::from_canonical(root.clone());
        let state = state().with_snapshot_dir(dir.clone());
        restore_snapshot_into_cache(
            &state.cache,
            state.scan_coordinator(),
            &state.snapshot_metrics,
            &dir,
            &key,
            &root,
        );
        let m = state.snapshot_metrics();
        assert_eq!(m.load_absent, 1);
        assert_eq!(m.load_corrupt, 0);
    }

    // ---- CIB-092h: ADR-035 Notification on persist write failure ----

    fn deny_all_broadcaster() -> Arc<TelemetryBroadcaster> {
        // A fanout that authorises nobody (no subscriber registered) — the
        // envelope is still *built* and offered; we assert on the returned
        // envelope, not on delivery (daemon-internal writes carry no session, so
        // the fanout default-denies external delivery by design).
        let fanout = Arc::new(Fanout::with_cross_session_policy(
            Box::new(SingleOwnerResolver {
                subscriber: SubscriberId::new("none"),
                session_id: "none".to_string(),
            }),
            CrossSessionPolicy::Deny,
        ));
        Arc::new(TelemetryBroadcaster::new(fanout))
    }

    /// With persistence enabled and a broadcaster wired, a write failure raises an
    /// ADR-035 `Health`/`High` degradation Notification (not only a WARN).
    #[test]
    fn enabled_persistence_write_failure_raises_adr035_notification() {
        use anvil_kernel_types::{NotificationClass, NotificationPriority};
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("graph-cache");
        let state = state()
            .with_snapshot_dir(dir)
            .with_broadcaster(deny_all_broadcaster());
        assert!(state.persistence_enabled());

        let err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let envelope = state
            .notify_persist_write_failure(std::path::Path::new("/ws-fail"), &err)
            .expect("an enabled-persistence write failure must raise a notification");
        assert_eq!(envelope.notification.class, NotificationClass::Health);
        assert_eq!(envelope.notification.priority, NotificationPriority::High);
        assert_eq!(
            envelope.correlation.worktree.as_deref(),
            Some("/ws-fail"),
            "the worktree rides the correlation",
        );
        // PV-10: the message carries only the ErrorKind discriminant, no path/bytes.
        assert!(
            !envelope.notification.message.contains("/ws-fail"),
            "the notification message must not echo the worktree path",
        );
        // ADR-090 (CIB-098): the persist-failure envelope is flagged as a
        // daemon-originated, worktree-scoped health envelope (the only way
        // the fan-out authorises it by worktree), and carries no session id
        // (it is daemon-originated, not session-scoped).
        assert!(
            envelope.daemon_worktree_health,
            "the persist-failure envelope must be flagged daemon-worktree-health (ADR-090)",
        );
        assert_eq!(
            envelope.correlation.originating_session_id, None,
            "the persist-failure envelope is daemon-originated — no session id",
        );
    }

    /// Disabled persistence never raises the notification (the write never happens).
    #[test]
    fn disabled_persistence_write_failure_raises_no_notification() {
        let state = state().with_broadcaster(deny_all_broadcaster());
        assert!(!state.persistence_enabled());
        let err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        assert!(
            state
                .notify_persist_write_failure(std::path::Path::new("/ws"), &err)
                .is_none(),
            "persistence-off must not raise a degradation notification",
        );
    }

    /// End-to-end: a real failed shutdown write (a symlinked, security-rejected
    /// snapshot dir) increments `write_error` AND, with persistence enabled + a
    /// broadcaster, the write-failure path is exercised through `persist_all_on_shutdown`.
    #[cfg(unix)]
    #[test]
    fn shutdown_write_failure_counts_error_and_notifies() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // A symlinked `graph-cache` dir is refused by the writer's dir-security
        // check (InvalidData), so the write fails deterministically.
        let real = tmp.path().join("real");
        fs::create_dir(&real).unwrap();
        let link = tmp.path().join("graph-cache");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let key = WorktreeKey::from_canonical(std::path::PathBuf::from("/ws-shutdown-fail"));
        let state = state()
            .with_snapshot_dir(link)
            .with_broadcaster(deny_all_broadcaster());
        state.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["a"], 0),
        );
        state.persist_all_on_shutdown();
        let m = state.snapshot_metrics();
        assert_eq!(m.write_error, 1, "a failed shutdown write must be counted");
        assert_eq!(m.write_ok, 0);
    }

    /// N2 / CIB-095d: the shutdown flush still persists when offloaded to
    /// `spawn_blocking` (the exact pattern `run_foreground` now uses on both exit
    /// paths) — the blocking task is awaited, so the writes complete before exit.
    #[cfg(unix)]
    #[test]
    fn persist_on_shutdown_via_spawn_blocking_still_writes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("graph-cache");
        let key = WorktreeKey::from_canonical(std::path::PathBuf::from("/ws-blocking"));

        let state = std::sync::Arc::new(state().with_snapshot_dir(dir.clone()));
        state.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("src/a.ts", &["alpha"], 0),
        );

        // Mirror `run_foreground`'s shutdown offload: persist on a blocking pool
        // thread, awaited on a current-thread runtime.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let state = std::sync::Arc::clone(&state);
            tokio::task::spawn_blocking(move || state.persist_all_on_shutdown())
                .await
                .expect("blocking persist task");
        });

        assert!(
            std::fs::read_dir(&dir).unwrap().next().is_some(),
            "the offloaded shutdown flush must still write a snapshot",
        );
        assert_eq!(state.snapshot_metrics().write_ok, 1);
    }

    /// CIB-095f / cross-ref: a key whose resident graph carries a
    /// non-workspace-relative path makes `from_graphs` reject the build. The
    /// shutdown flush must NOT silently skip it — it increments `write_error`
    /// (the snapshot is lost) and warns, consistent with `persist_after_scan`.
    #[cfg(unix)]
    #[test]
    fn shutdown_from_graphs_build_error_counts_write_error_not_silent_skip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("graph-cache");
        let key = WorktreeKey::from_canonical(std::path::PathBuf::from("/ws-build-reject"));
        let state = state().with_snapshot_dir(dir);
        assert!(state.persistence_enabled());

        // An ABSOLUTE file path in a resident symbol → `from_graphs` rejects it
        // with `NonRelativePath` (ADR-069 §8).
        state.cache.apply_delta(
            &key,
            ChangeKind::Create,
            file_symbols("/abs/escape.ts", &["a"], 0),
        );

        state.persist_all_on_shutdown();
        let m = state.snapshot_metrics();
        assert_eq!(
            m.write_error, 1,
            "a from_graphs build rejection must be counted, not silently skipped",
        );
        assert_eq!(m.write_ok, 0, "nothing was successfully written");
    }
}
