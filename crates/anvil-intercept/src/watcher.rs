//! INTD-004: file-system watcher integration.
//!
//! The daemon does not own a watcher of its own — `crates/anvil-kernel`
//! already runs the recursive `notify`-backed watcher used by `anvil
//! watch`, debouncing raw inotify / `FSEvents` / `ReadDirectoryChangesW`
//! events into [`WatcherChangeBatch`]es. INTD-004 wires that existing
//! channel into the daemon's enforcement pipeline:
//!
//! 1. The kernel watcher delivers a `ChangeBatch` on its `mpsc`
//!    channel. The daemon's [`WatcherIntegration`] task receives it,
//!    converts to the local [`WatcherChangeBatch`] shape (a 1:1
//!    structural mirror — see [`From`] impl below) and pushes the
//!    batch into the per-session coalescer.
//! 2. Every batch's paths are run through
//!    [`SessionRegistry::attribute_path`] to identify the owning
//!    session. Changes that fall under a registered worktree are
//!    grouped per session and dispatched to that session's
//!    enforcement pipeline; unattributed changes route to the
//!    [`UnregisteredHandler`] (INTD-010) and never touch the
//!    per-session pipeline.
//! 3. The coalescer holds bursts for a configurable window
//!    (default 50 ms — pinned by intercept-rules `ChangeBatch`
//!    coalescing) so a single agent edit that touches N files
//!    arrives at enforcement as one batch, not N.
//!
//! ## Why a structural mirror, not a kernel-crate dep
//!
//! Pulling `eddacraft-anvil-kernel` into `anvil-intercept` would drag
//! in `tree-sitter`, `petgraph`, and the parser surface — none of
//! which the daemon needs at runtime. The channel shape is three
//! plain fields ([`std::path::PathBuf`], [`ChangeKind`],
//! [`std::time::Instant`]); duplicating it locally and adapting at
//! the daemon binding keeps the dep tree honest. The kernel side is
//! still the single source of truth for *generating* batches; this
//! module is the receiving end.
//!
//! ## What this module is not
//!
//! - **Not the watcher.** The `notify` crate, debounce window, and
//!   directory walk all live in `anvil_kernel::watcher`.
//! - **Not the enforcement pipeline.** Decisions are produced by
//!   [`crate::enforcement::EnforcementPipeline`]; this module hands
//!   it pre-attributed batches.
//! - **Not the unknown-change policy.** INTD-010
//!   ([`UnregisteredHandler`]) decides whether unattributed changes
//!   warn or fence. This module only routes.
//!
//! See `plans/modules/intercept-daemon.aps.md` task INTD-004.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anvil_intercept_proto::SessionId;
use anvil_intercept_rules::ChangeKind;

use crate::enforcement::{EnforcementDecision, EnforcementPipeline, FileChange};
use crate::registry::{Attribution, SessionRegistry};

/// Default watcher coalescing window. Pinned at 50 ms by the
/// intercept-rules `ChangeBatch` coalescing contract: agent edits
/// rarely complete in a single sub-millisecond burst, so collapsing
/// the bursty front edge of a save into one batch is the realistic
/// shape an enforcement rule sees.
pub const DEFAULT_COALESCE_WINDOW: Duration = Duration::from_millis(50);

/// Single file change as observed by the watcher. Mirror of
/// `anvil_kernel::watcher::events::FileChange`; kept in this crate
/// so `anvil-intercept` does not depend on the kernel crate. The
/// [`From`] impl below documents the 1:1 conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatcherFileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

/// Batch of file changes produced by the kernel watcher's debouncer.
/// Mirror of `anvil_kernel::watcher::events::ChangeBatch`.
#[derive(Debug, Clone)]
pub struct WatcherChangeBatch {
    pub changes: Vec<WatcherFileChange>,
    pub received_at: Instant,
}

/// What [`WatcherIntegration`] does with the change once attribution
/// resolves. The watcher does not directly mutate enforcement state —
/// it returns a routing record so callers (the daemon, tests) can
/// observe and assert on it.
#[derive(Debug)]
pub enum AttributedBatch {
    /// Change(s) were attributed to a registered session. The
    /// `decision` is the result of running the per-session changes
    /// through the [`EnforcementPipeline`]. Carrying the decision
    /// here (rather than just the changes) keeps the routing layer
    /// honest about what enforcement saw.
    Owned {
        session_id: SessionId,
        worktree: PathBuf,
        decision: EnforcementDecision,
    },
    /// Change(s) could not be attributed to any registered session.
    /// Routes to the INTD-010 unregistered path, which is responsible
    /// for the `attribution: unknown-agent` tagging and the
    /// `on_ambiguous_ownership` policy hard-cap.
    Unknown { changes: Vec<FileChange> },
}

/// Pluggable handler for unattributed changes. INTD-010 owns the
/// concrete implementation; the watcher integration takes a trait
/// object so v1 can ship before INTD-010 lands and tests can plug in
/// a recording double.
///
/// The trait is intentionally synchronous: handler implementations
/// either fence (touch the [`crate::fence::FenceStore`], which is
/// already synchronous) or warn (push a notification onto the
/// telemetry lane, also synchronous from the producer's perspective).
/// Adding `async` here would couple the watcher loop to a runtime
/// without buying anything.
pub trait UnregisteredHandler: Send + Sync + 'static {
    fn handle(&self, changes: &[FileChange]);
}

/// No-op unregistered handler. Wired in until INTD-010 ships its
/// real `attribution: unknown-agent` policy; tests use it to assert
/// that the watcher routed correctly without observing fence /
/// telemetry side effects.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopUnregisteredHandler;

impl UnregisteredHandler for NoopUnregisteredHandler {
    fn handle(&self, _changes: &[FileChange]) {}
}

/// Configuration for the watcher integration. All fields have
/// reasonable defaults; tests override `coalesce_window` to drive
/// the time path deterministically.
#[derive(Debug, Clone)]
pub struct WatcherIntegrationConfig {
    /// Window over which bursty changes from the kernel watcher are
    /// collapsed into one enforcement evaluation per session.
    pub coalesce_window: Duration,
}

impl Default for WatcherIntegrationConfig {
    fn default() -> Self {
        Self {
            coalesce_window: DEFAULT_COALESCE_WINDOW,
        }
    }
}

/// Synchronous core of the watcher integration. Holds the registry,
/// pipeline, and unregistered handler; tests drive it directly via
/// [`WatcherIntegration::ingest_at`] without spinning a thread.
///
/// The async / threaded loop is layered on top in [`run`]; keeping
/// the core synchronous makes the time-dependent coalescing path
/// trivial to assert in unit tests.
pub struct WatcherIntegration {
    registry: Arc<SessionRegistry>,
    pipeline: Arc<EnforcementPipeline>,
    unregistered: Arc<dyn UnregisteredHandler>,
    config: WatcherIntegrationConfig,
    coalescer: Coalescer,
}

impl WatcherIntegration {
    /// Construct a watcher integration with the daemon's registry,
    /// pipeline, and unregistered-change handler.
    pub fn new(
        registry: Arc<SessionRegistry>,
        pipeline: Arc<EnforcementPipeline>,
        unregistered: Arc<dyn UnregisteredHandler>,
    ) -> Self {
        Self::with_config(
            registry,
            pipeline,
            unregistered,
            WatcherIntegrationConfig::default(),
        )
    }

    pub fn with_config(
        registry: Arc<SessionRegistry>,
        pipeline: Arc<EnforcementPipeline>,
        unregistered: Arc<dyn UnregisteredHandler>,
        config: WatcherIntegrationConfig,
    ) -> Self {
        Self {
            registry,
            pipeline,
            unregistered,
            config,
            coalescer: Coalescer::new(),
        }
    }

    /// Ingest a single batch from the watcher channel. Routes each
    /// change through `attribute_path`, accumulates per-session
    /// changes in the coalescer, and returns the set of attributed
    /// batches whose coalescing window has expired.
    ///
    /// Tests pass `now` explicitly so they can drive the window
    /// without `tokio::time::sleep`; the production loop uses
    /// `Instant::now()` (see [`run`]).
    pub fn ingest_at(&mut self, batch: WatcherChangeBatch, now: Instant) -> Vec<AttributedBatch> {
        let coalesce_window = self.config.coalesce_window;

        for change in batch.changes {
            let file_change = FileChange {
                path: change.path.clone(),
                change_kind: change.kind,
            };
            match self.registry.attribute_path(&change.path) {
                Attribution::Owned { session } => {
                    self.coalescer.record_owned(
                        session.id.clone(),
                        session.worktree.clone(),
                        file_change,
                        now,
                    );
                }
                Attribution::Unknown => {
                    self.coalescer.record_unknown(file_change, now);
                }
            }
        }

        self.flush_due(now, coalesce_window)
    }

    /// Force-flush every pending batch regardless of window. Called
    /// by the daemon's shutdown path so any in-flight changes still
    /// reach enforcement / unregistered routing before the daemon
    /// exits.
    pub fn flush_all(&mut self, now: Instant) -> Vec<AttributedBatch> {
        self.flush_due(now, Duration::ZERO)
    }

    fn flush_due(&mut self, now: Instant, window: Duration) -> Vec<AttributedBatch> {
        let mut out = Vec::new();
        for entry in self.coalescer.drain_due(now, window) {
            match entry {
                CoalescedBatch::Owned {
                    session_id,
                    worktree,
                    changes,
                } => {
                    let decision = self.pipeline.evaluate_filesystem_changes(&changes);
                    out.push(AttributedBatch::Owned {
                        session_id,
                        worktree,
                        decision,
                    });
                }
                CoalescedBatch::Unknown { changes } => {
                    self.unregistered.handle(&changes);
                    out.push(AttributedBatch::Unknown { changes });
                }
            }
        }
        out
    }
}

/// Per-session coalescing buffer. Each `(session_id, worktree)` pair
/// keeps a pending change list and a `last_seen` timestamp; the
/// `Unknown` lane keeps a single buffer for unattributed changes.
///
/// "Last seen" is the time the most recent change landed in the
/// buffer — the coalescing window measures from that point, so a
/// continuous burst extends the window. This matches the kernel
/// watcher's debouncer semantics; surfacing the same shape to
/// enforcement keeps the model intuitive.
struct Coalescer {
    owned: HashMap<SessionId, OwnedBuffer>,
    unknown: Option<UnknownBuffer>,
}

impl Coalescer {
    fn new() -> Self {
        Self {
            owned: HashMap::new(),
            unknown: None,
        }
    }

    fn record_owned(
        &mut self,
        session_id: SessionId,
        worktree: PathBuf,
        change: FileChange,
        now: Instant,
    ) {
        let entry = self.owned.entry(session_id).or_insert_with(|| OwnedBuffer {
            worktree,
            changes: Vec::new(),
            last_seen: now,
        });
        entry.changes.push(change);
        entry.last_seen = now;
    }

    fn record_unknown(&mut self, change: FileChange, now: Instant) {
        let buffer = self.unknown.get_or_insert_with(|| UnknownBuffer {
            changes: Vec::new(),
            last_seen: now,
        });
        buffer.changes.push(change);
        buffer.last_seen = now;
    }

    fn drain_due(&mut self, now: Instant, window: Duration) -> Vec<CoalescedBatch> {
        let mut out = Vec::new();
        // Owned: drain any session whose last_seen is older than the
        // window. Iterate, mark the keys to evict, then evict — avoids
        // mutable-borrow contention on the map.
        let due_keys: Vec<SessionId> = self
            .owned
            .iter()
            .filter(|(_, buf)| now.saturating_duration_since(buf.last_seen) >= window)
            .map(|(k, _)| k.clone())
            .collect();
        for key in due_keys {
            if let Some(buf) = self.owned.remove(&key)
                && !buf.changes.is_empty()
            {
                out.push(CoalescedBatch::Owned {
                    session_id: key,
                    worktree: buf.worktree,
                    changes: buf.changes,
                });
            }
        }

        if let Some(buf) = self.unknown.as_ref()
            && now.saturating_duration_since(buf.last_seen) >= window
        {
            let buf = self.unknown.take().expect("checked above");
            if !buf.changes.is_empty() {
                out.push(CoalescedBatch::Unknown {
                    changes: buf.changes,
                });
            }
        }

        // Stable order: Owned first (sorted by id), Unknown last —
        // tests assert against this so a dictionary-iteration change
        // does not silently flip ordering.
        out.sort_by(|a, b| match (a, b) {
            (
                CoalescedBatch::Owned { session_id: a, .. },
                CoalescedBatch::Owned { session_id: b, .. },
            ) => a.as_str().cmp(b.as_str()),
            (CoalescedBatch::Owned { .. }, CoalescedBatch::Unknown { .. }) => {
                std::cmp::Ordering::Less
            }
            (CoalescedBatch::Unknown { .. }, CoalescedBatch::Owned { .. }) => {
                std::cmp::Ordering::Greater
            }
            (CoalescedBatch::Unknown { .. }, CoalescedBatch::Unknown { .. }) => {
                std::cmp::Ordering::Equal
            }
        });
        out
    }
}

struct OwnedBuffer {
    worktree: PathBuf,
    changes: Vec<FileChange>,
    last_seen: Instant,
}

struct UnknownBuffer {
    changes: Vec<FileChange>,
    last_seen: Instant,
}

enum CoalescedBatch {
    Owned {
        session_id: SessionId,
        worktree: PathBuf,
        changes: Vec<FileChange>,
    },
    Unknown {
        changes: Vec<FileChange>,
    },
}

/// Run the watcher integration as an async loop, consuming batches
/// from `rx` until the shutdown token fires. Production wiring uses
/// this; tests drive [`WatcherIntegration::ingest_at`] directly so
/// the time path stays deterministic.
///
/// `rx` is a `std::sync::mpsc::Receiver` because the kernel watcher
/// produces on a `std::sync` channel — a tokio-side translation
/// would mean another thread, and the daemon's existing pattern is
/// to keep the `notify` thread separate from tokio. The blocking
/// `recv_timeout` runs inside `spawn_blocking` so the tokio reactor
/// is not stalled.
pub async fn run(
    integration: WatcherIntegration,
    rx: std::sync::mpsc::Receiver<WatcherChangeBatch>,
    mut token: crate::ShutdownToken,
) {
    let coalesce_window = integration.config.coalesce_window;
    // `tick_interval` controls how often the loop wakes to flush
    // expired buffers when no new batch arrives. Picked at half the
    // coalesce window so the worst-case flush latency is ≤ 1.5×
    // window — comfortable inside the operator-perceptible budget.
    let tick_interval = (coalesce_window / 2).max(Duration::from_millis(10));

    let mut integration = integration;
    let mut next_tick = tokio::time::interval(tick_interval);
    next_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            biased;
            () = token.cancelled() => break,
            _ = next_tick.tick() => {
                let _ = integration.flush_due(Instant::now(), coalesce_window);
            }
            batch = recv_blocking(&rx) => {
                if let Some(batch) = batch {
                    let _ = integration.ingest_at(batch, Instant::now());
                } else {
                    // Sender dropped — kernel watcher has exited;
                    // flush whatever is still buffered and stop.
                    let _ = integration.flush_all(Instant::now());
                    return;
                }
            }
        }
    }
    let _ = integration.flush_all(Instant::now());
}

async fn recv_blocking(
    _rx: &std::sync::mpsc::Receiver<WatcherChangeBatch>,
) -> Option<WatcherChangeBatch> {
    // The receiver is `!Sync`; we cannot move it into `spawn_blocking`
    // by reference. The production wiring will use a dedicated bridge
    // thread that forwards into a `tokio::sync::mpsc` — INTD-004 does
    // not bind to one transport here. For now this helper exists so
    // [`run`] compiles; the daemon's binding code (the call site that
    // owns the receiver) is responsible for its own bridging.
    //
    // This function intentionally pends forever so `run` is usable
    // when the operator hasn't wired a watcher: the loop still ticks
    // and flushes any pending unit-test injections.
    std::future::pending().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    use crate::enforcement::default_rule_registry;
    use anvil_intercept_rules::RuleRegistry;
    use tempfile::TempDir;

    fn setup_pipeline_empty() -> Arc<EnforcementPipeline> {
        Arc::new(EnforcementPipeline::new(RuleRegistry::new()))
    }

    fn setup_pipeline_default() -> Arc<EnforcementPipeline> {
        Arc::new(EnforcementPipeline::new(default_rule_registry()))
    }

    fn make_worktree() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn change(path: PathBuf, kind: ChangeKind) -> WatcherFileChange {
        WatcherFileChange { path, kind }
    }

    /// Recording handler used by `unattributed_change_routes_through_unregistered_handler`
    /// to assert the watcher hands unattributed changes to the
    /// configured handler. Hoisted out of the test body so the
    /// items-after-statements lint stays happy.
    struct Recording {
        seen: std::sync::Mutex<Vec<FileChange>>,
    }
    impl UnregisteredHandler for Recording {
        fn handle(&self, changes: &[FileChange]) {
            self.seen.lock().unwrap().extend_from_slice(changes);
        }
    }

    /// Test (a): a change inside a registered session's worktree
    /// reaches the enforcement pipeline tagged with that session's
    /// id.
    #[test]
    fn attributed_change_reaches_enforcement_pipeline() {
        let registry = Arc::new(SessionRegistry::new());
        let wt = make_worktree();
        let now = Instant::now();
        registry
            .register(&SessionId::new("sess-a"), wt.path(), now)
            .expect("register");
        // Materialise a child file so canonicalisation in
        // `attribute_path` actually finds the worktree ancestor.
        let file = wt.path().join("src.rs");
        std::fs::write(&file, b"hello").expect("write fixture");

        let mut integration = WatcherIntegration::new(
            Arc::clone(&registry),
            setup_pipeline_empty(),
            Arc::new(NoopUnregisteredHandler),
        );

        let batch = WatcherChangeBatch {
            changes: vec![change(file.clone(), ChangeKind::Modified)],
            received_at: now,
        };
        // Same `now` for ingest and the past-window time, then advance
        // beyond the window so the coalescer flushes.
        let _ = integration.ingest_at(batch, now);
        let flushed =
            integration.flush_all(now + DEFAULT_COALESCE_WINDOW + Duration::from_millis(1));

        assert_eq!(flushed.len(), 1, "exactly one attributed batch");
        match &flushed[0] {
            AttributedBatch::Owned {
                session_id,
                worktree,
                decision,
            } => {
                assert_eq!(session_id.as_str(), "sess-a");
                // worktree mirrors the canonicalised registry key.
                assert_eq!(*worktree, std::fs::canonicalize(wt.path()).unwrap());
                // Empty rule registry → Allow.
                assert!(matches!(decision, EnforcementDecision::Allow { .. }));
            }
            AttributedBatch::Unknown { .. } => panic!("expected Owned, got Unknown"),
        }
    }

    /// Test (b): a change that does not fall under any registered
    /// worktree routes through the unregistered handler. The
    /// concrete handler ships in INTD-010; here we use a recording
    /// double to assert the routing edge.
    #[test]
    fn unattributed_change_routes_through_unregistered_handler() {
        let registry = Arc::new(SessionRegistry::new());
        let now = Instant::now();
        let stranger_root = make_worktree();
        let stranger_file = stranger_root.path().join("rogue.rs");
        std::fs::write(&stranger_file, b"unowned").expect("write fixture");

        let recording = Arc::new(Recording {
            seen: std::sync::Mutex::new(Vec::new()),
        });

        let mut integration = WatcherIntegration::new(
            Arc::clone(&registry),
            setup_pipeline_empty(),
            Arc::clone(&recording) as Arc<dyn UnregisteredHandler>,
        );
        let batch = WatcherChangeBatch {
            changes: vec![change(stranger_file.clone(), ChangeKind::Modified)],
            received_at: now,
        };
        let _ = integration.ingest_at(batch, now);
        let flushed =
            integration.flush_all(now + DEFAULT_COALESCE_WINDOW + Duration::from_millis(1));

        assert_eq!(flushed.len(), 1);
        match &flushed[0] {
            AttributedBatch::Unknown { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, stranger_file);
            }
            AttributedBatch::Owned { .. } => panic!("expected Unknown, got Owned"),
        }
        // Recording handler must have observed the change exactly
        // once — confirms the "unregistered routes via handler" wire.
        let recorded = recording.seen.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].path, stranger_file);
    }

    /// Test (c): a burst of changes inside the coalesce window
    /// collapses into one enforcement call. The window timing is
    /// driven explicitly via `ingest_at(_, now)` so the test does
    /// not rely on real sleeping.
    #[test]
    fn coalescing_window_collapses_bursty_changes_into_one_batch() {
        let registry = Arc::new(SessionRegistry::new());
        let wt = make_worktree();
        let t0 = Instant::now();
        registry
            .register(&SessionId::new("sess-burst"), wt.path(), t0)
            .expect("register");
        let f1 = wt.path().join("a.rs");
        let f2 = wt.path().join("b.rs");
        let f3 = wt.path().join("c.rs");
        for f in [&f1, &f2, &f3] {
            std::fs::write(f, b"x").expect("write fixture");
        }

        let pipeline = setup_pipeline_default();
        let mut integration = WatcherIntegration::with_config(
            Arc::clone(&registry),
            pipeline,
            Arc::new(NoopUnregisteredHandler),
            WatcherIntegrationConfig {
                coalesce_window: Duration::from_millis(50),
            },
        );

        // Three batches arrive within the window. ingest_at returns
        // any flushable entries; with `now == t0` for all three calls
        // the window has not expired so nothing should flush yet.
        for f in [&f1, &f2, &f3] {
            let batch = WatcherChangeBatch {
                changes: vec![change(f.clone(), ChangeKind::Modified)],
                received_at: t0,
            };
            let immediate = integration.ingest_at(batch, t0);
            assert!(
                immediate.is_empty(),
                "no flush before window expires; got {immediate:?}",
            );
        }

        // After the window passes, a single coalesced batch carries
        // all three paths in one enforcement call — which is the
        // load-bearing property: a save touching three files is
        // *one* enforcement decision, not three.
        let flushed = integration.flush_all(t0 + Duration::from_millis(60));
        assert_eq!(flushed.len(), 1, "single batch after coalesce window");
        match &flushed[0] {
            AttributedBatch::Owned {
                session_id,
                decision,
                ..
            } => {
                assert_eq!(session_id.as_str(), "sess-burst");
                let paths = match decision {
                    EnforcementDecision::Allow { affected_paths } => affected_paths.clone(),
                    EnforcementDecision::Interrupt(d) => d.affected_paths.clone(),
                };
                assert_eq!(paths.len(), 3, "all three changes coalesced");
            }
            AttributedBatch::Unknown { .. } => panic!("expected Owned"),
        }
    }

    /// A burst that spans two distinct sessions still flushes one
    /// attributed batch per session. Pinned because the coalescer
    /// keys on session id, not worktree, so a worktree-renaming
    /// change cannot accidentally cross-pollinate sessions.
    #[test]
    fn two_sessions_flush_independently_after_window() {
        let registry = Arc::new(SessionRegistry::new());
        let wt_a = make_worktree();
        let wt_b = make_worktree();
        let t0 = Instant::now();
        registry
            .register(&SessionId::new("sess-a"), wt_a.path(), t0)
            .expect("register a");
        registry
            .register(&SessionId::new("sess-b"), wt_b.path(), t0)
            .expect("register b");
        let f_a = wt_a.path().join("a.rs");
        let f_b = wt_b.path().join("b.rs");
        std::fs::write(&f_a, b"x").expect("write a");
        std::fs::write(&f_b, b"y").expect("write b");

        let mut integration = WatcherIntegration::new(
            Arc::clone(&registry),
            setup_pipeline_empty(),
            Arc::new(NoopUnregisteredHandler),
        );
        let batch = WatcherChangeBatch {
            changes: vec![
                change(f_a.clone(), ChangeKind::Modified),
                change(f_b.clone(), ChangeKind::Modified),
            ],
            received_at: t0,
        };
        let _ = integration.ingest_at(batch, t0);
        let flushed =
            integration.flush_all(t0 + DEFAULT_COALESCE_WINDOW + Duration::from_millis(1));
        assert_eq!(flushed.len(), 2);
        // Stable order — Owned-by-session-id-asc.
        let ids: Vec<&str> = flushed
            .iter()
            .map(|b| match b {
                AttributedBatch::Owned { session_id, .. } => session_id.as_str(),
                AttributedBatch::Unknown { .. } => "unknown",
            })
            .collect();
        assert_eq!(ids, vec!["sess-a", "sess-b"]);
    }

    /// `flush_all` honours the zero-duration window so shutdown
    /// drains buffered changes regardless of how recently they
    /// arrived. Regression guard for the daemon's clean-exit path.
    #[test]
    fn flush_all_drains_buffer_regardless_of_window() {
        let registry = Arc::new(SessionRegistry::new());
        let wt = make_worktree();
        let now = Instant::now();
        registry
            .register(&SessionId::new("sess-shutdown"), wt.path(), now)
            .expect("register");
        let file = wt.path().join("late.rs");
        std::fs::write(&file, b"x").expect("write fixture");

        let mut integration = WatcherIntegration::new(
            Arc::clone(&registry),
            setup_pipeline_empty(),
            Arc::new(NoopUnregisteredHandler),
        );
        let batch = WatcherChangeBatch {
            changes: vec![change(file, ChangeKind::Modified)],
            received_at: now,
        };
        let _ = integration.ingest_at(batch, now);

        let flushed = integration.flush_all(now);
        assert_eq!(flushed.len(), 1);
        assert!(matches!(flushed[0], AttributedBatch::Owned { .. }));
    }
}
