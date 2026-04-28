//! INTD-003: in-memory session registry.
//!
//! The registry is the single authority on which sessions are active,
//! which worktree each owns, and when a session has gone silent long
//! enough to count as crashed. It is deliberately synchronous and
//! pure: the daemon's `run_foreground` loop owns scheduling and ticks
//! [`SessionRegistry::evict_stale`] from its 250 ms interval.
//!
//! Spawning a background eviction task here would couple the registry
//! to a runtime and make it harder to drive from tests; the council
//! pinned this layer as a synchronous data structure.
//!
//! See `plans/modules/intercept-daemon.aps.md` task INTD-003.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anvil_intercept_proto::{SessionId, SessionRecord, SessionStatus};
use thiserror::Error;

/// Default session heartbeat TTL — pinned at 30 s by INTD-003 in
/// `plans/modules/intercept-daemon.aps.md`. A session that misses this
/// window is treated as crashed.
pub const DEFAULT_HEARTBEAT_TTL: Duration = Duration::from_secs(30);

/// Errors returned by the synchronous registry surface. The wire layer
/// (INTD-002) maps these onto JSON-RPC error codes; that mapping lives
/// outside this module so the registry stays independent of transport.
///
/// `PartialEq` is hand-written rather than derived because
/// [`std::io::Error`] is not `PartialEq` — equality on
/// `WorktreePathInvalid` compares the path and the io error kind, which
/// is the part tests actually care about.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// Another session already owns the canonicalised worktree path.
    /// `existing` is the id of the live owner so the caller can decide
    /// whether to surface, retry, or refuse.
    #[error("worktree already owned by session {existing:?}")]
    WorktreeAlreadyOwned { existing: SessionId },

    /// Worktree path could not be canonicalised — the path either does
    /// not exist or is otherwise unusable as a registry key. v1 refuses
    /// to register such sessions; the launcher must materialise the
    /// worktree before calling.
    #[error("worktree path could not be canonicalised: {path:?}: {source}")]
    WorktreePathInvalid {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A heartbeat / update arrived for a session id the registry has
    /// no record of (never registered, or already evicted).
    #[error("unknown session: {0:?}")]
    UnknownSession(SessionId),
}

impl PartialEq for RegistryError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::WorktreePathInvalid {
                    path: a,
                    source: ae,
                },
                Self::WorktreePathInvalid {
                    path: b,
                    source: be,
                },
            ) => a == b && ae.kind() == be.kind(),
            (
                Self::WorktreeAlreadyOwned { existing: a },
                Self::WorktreeAlreadyOwned { existing: b },
            )
            | (Self::UnknownSession(a), Self::UnknownSession(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for RegistryError {}

/// Process info that the launcher feeds back into the registry once it
/// has spawned the agent and has a pid / pgid to report. `None` fields
/// mean "no update" — they do not clobber an existing `Some`. To
/// explicitly clear a value, the caller must surface a separate API,
/// which v1 has no use for.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessInfo {
    pub pid: Option<u32>,
    /// Unix process group id. Always `None` on Windows; the Job Object
    /// handle is tracked out-of-band by INTD-006.
    pub pgid: Option<i32>,
    /// Override the start time recorded at registration. Reserved for
    /// the launcher reconciliation flow where the daemon was restarted
    /// mid-session and the old start time is more accurate than the
    /// re-registered one. Day-to-day usage should leave this `None`.
    pub started_at_unix: Option<u64>,
}

/// Trait the IPC listener (INTD-002) calls into. Keeps the registry's
/// public surface minimal and lets the listener depend on
/// `Arc<dyn SessionDispatcher>` without binding to the concrete type.
///
/// Lives in `registry.rs` rather than the proto crate because proto is
/// wire types only; this is a daemon-internal extension point.
pub trait SessionDispatcher: Send + Sync + 'static {
    fn register(&self, id: &SessionId, worktree: &Path) -> Result<(), RegistryError>;
    fn heartbeat(&self, id: &SessionId) -> Result<(), RegistryError>;
    fn unregister(&self, id: &SessionId) -> Result<bool, RegistryError>;
    fn list(&self) -> Vec<SessionRecord>;
}

/// In-memory session registry. Cheap to clone via `Arc`; the internal
/// state is guarded by a single `std::sync::Mutex` because every
/// operation is a small `HashMap` mutation under microseconds — the
/// extra dependency surface of `parking_lot` is not justified at this
/// scale, and `std::sync::Mutex` poisons cleanly under panic which
/// gives us a cheap crash-safety property.
pub struct SessionRegistry {
    inner: Mutex<Inner>,
    ttl: Duration,
}

struct Inner {
    /// `SessionId` -> record. Sole source of truth for the record body.
    sessions: HashMap<SessionId, RegistryEntry>,
    /// Canonicalised worktree path -> session id. Index for the
    /// "single session per worktree" constraint and for
    /// `session_for_worktree`.
    by_worktree: HashMap<PathBuf, SessionId>,
}

struct RegistryEntry {
    record: SessionRecord,
    /// Monotonic timestamp the registry compares against the TTL.
    /// `Instant` is monotonic on every supported platform, so unlike
    /// `last_heartbeat_unix` (wall-clock, can jump backwards on
    /// NTP step) it is safe for liveness checks.
    last_heartbeat: Instant,
}

impl SessionRegistry {
    /// Construct a registry with the default 30 s TTL.
    #[must_use]
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_HEARTBEAT_TTL)
    }

    /// Construct a registry with a custom TTL. Tests and embedded-mode
    /// callers (INTD-009) use this; the daemon entry point sticks with
    /// the default.
    #[must_use]
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(Inner {
                sessions: HashMap::new(),
                by_worktree: HashMap::new(),
            }),
            ttl,
        }
    }

    /// Register a new session against a worktree path.
    ///
    /// **Canonicalisation policy:** the worktree is run through
    /// `std::fs::canonicalize` before use as a registry key, so two
    /// clients spelling the same worktree differently (trailing slash,
    /// `..` segments, symlinks) cannot each "own" the same worktree.
    /// A path that does not exist yields [`RegistryError::WorktreePathInvalid`]
    /// — v1 refuses to register sessions for missing worktrees rather
    /// than silently storing a relative path.
    ///
    /// **Crash-safety:** crashed launchers (where `Drop`-fired
    /// unregister never runs because the process was `SIGKILL`-ed or
    /// `TerminateProcess`-ed) are NOT handled here. They are evicted
    /// by [`SessionRegistry::evict_stale`], which the daemon ticks
    /// every 250 ms.
    ///
    /// Pinned at one session per canonicalised worktree for v1; a
    /// retry on the same worktree returns
    /// [`RegistryError::WorktreeAlreadyOwned`] carrying the existing
    /// owner's id.
    pub fn register(
        &self,
        id: &SessionId,
        worktree: &Path,
        now: Instant,
    ) -> Result<SessionRecord, RegistryError> {
        let canonical = canonicalise(worktree)?;
        let mut inner = self.lock();

        if let Some(existing) = inner.by_worktree.get(&canonical) {
            return Err(RegistryError::WorktreeAlreadyOwned {
                existing: existing.clone(),
            });
        }

        let now_unix = unix_seconds_now();
        let record = SessionRecord {
            id: id.clone(),
            worktree: canonical.clone(),
            pid: None,
            pgid: None,
            started_at_unix: now_unix,
            last_heartbeat_unix: now_unix,
            status: SessionStatus::Active,
        };

        inner.sessions.insert(
            id.clone(),
            RegistryEntry {
                record: record.clone(),
                last_heartbeat: now,
            },
        );
        inner.by_worktree.insert(canonical, id.clone());
        Ok(record)
    }

    /// Update process info for a registered session. `None` fields are
    /// no-ops — they do NOT clobber an existing `Some`.
    pub fn update_process_info(
        &self,
        id: &SessionId,
        info: ProcessInfo,
    ) -> Result<SessionRecord, RegistryError> {
        let mut inner = self.lock();
        let entry = inner
            .sessions
            .get_mut(id)
            .ok_or_else(|| RegistryError::UnknownSession(id.clone()))?;

        if let Some(pid) = info.pid {
            entry.record.pid = Some(pid);
        }
        if let Some(pgid) = info.pgid {
            entry.record.pgid = Some(pgid);
        }
        if let Some(start) = info.started_at_unix {
            entry.record.started_at_unix = start;
        }

        Ok(entry.record.clone())
    }

    /// Refresh the heartbeat for a session. Returns
    /// [`RegistryError::UnknownSession`] if the id is not registered
    /// (or has already been evicted).
    pub fn heartbeat(&self, id: &SessionId, now: Instant) -> Result<(), RegistryError> {
        let mut inner = self.lock();
        let entry = inner
            .sessions
            .get_mut(id)
            .ok_or_else(|| RegistryError::UnknownSession(id.clone()))?;
        entry.last_heartbeat = now;
        entry.record.last_heartbeat_unix = unix_seconds_now();
        Ok(())
    }

    /// Look up the record owning a worktree, if any. The caller is
    /// expected to pass an already-canonicalised path (the listener
    /// canonicalises once at the boundary); paths that fail to
    /// canonicalise yield `None`.
    #[must_use]
    pub fn session_for_worktree(&self, worktree: &Path) -> Option<SessionRecord> {
        let canonical = std::fs::canonicalize(worktree).ok()?;
        let inner = self.lock();
        let id = inner.by_worktree.get(&canonical)?;
        inner.sessions.get(id).map(|entry| entry.record.clone())
    }

    /// All currently active sessions, sorted deterministically by
    /// `started_at_unix` then `SessionId` so tests can compare lists
    /// without flakes under concurrent registration.
    #[must_use]
    pub fn active_sessions(&self) -> Vec<SessionRecord> {
        let inner = self.lock();
        let mut records: Vec<SessionRecord> = inner
            .sessions
            .values()
            .map(|entry| entry.record.clone())
            .collect();
        records.sort_by(|a, b| {
            a.started_at_unix
                .cmp(&b.started_at_unix)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        records
    }

    /// Remove a session by id. Returns `Ok(true)` if a session was
    /// removed, `Ok(false)` if the id was unknown — the latter is not
    /// an error, since the launcher's Drop guard may race the daemon's
    /// eviction tick.
    pub fn unregister(&self, id: &SessionId) -> Result<bool, RegistryError> {
        let mut inner = self.lock();
        let Some(entry) = inner.sessions.remove(id) else {
            return Ok(false);
        };
        inner.by_worktree.remove(&entry.record.worktree);
        Ok(true)
    }

    /// Evict every session whose heartbeat is older than `ttl`.
    /// Returns the ids that were removed so the daemon can fan out
    /// `session.ended` notifications (INTD-013).
    ///
    /// **Boundary policy:** a session at exactly `ttl` is still alive;
    /// at `ttl + 1ns` it evicts. Pinned by the `ttl_boundary` test —
    /// changing this requires updating both the test and the docstring.
    ///
    /// **Tombstone policy:** evicted records are removed from both the
    /// session map and the worktree index. The registry does not keep
    /// `Ended` tombstones — INTD-013 owns the post-eviction
    /// notification, and the wire-level `SessionStatus::Ended` is
    /// reserved for that downstream surface.
    pub fn evict_stale(&self, now: Instant) -> Vec<SessionId> {
        let mut inner = self.lock();
        let ttl = self.ttl;

        let mut stale: Vec<SessionId> = inner
            .sessions
            .iter()
            .filter_map(|(id, entry)| {
                let age = now.saturating_duration_since(entry.last_heartbeat);
                if age > ttl { Some(id.clone()) } else { None }
            })
            .collect();

        for id in &stale {
            if let Some(entry) = inner.sessions.remove(id) {
                inner.by_worktree.remove(&entry.record.worktree);
            }
        }

        stale.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        stale
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // `std::sync::Mutex` poisons on panic; recovering the guard is
        // the right call here because the registry data structure is
        // a plain `HashMap` — a panic mid-mutation cannot leave it in
        // a half-updated state visible to readers (the operations are
        // single-statement inserts/removes). Carrying poisoning forward
        // would let one panicking caller take the whole daemon offline.
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionDispatcher for SessionRegistry {
    fn register(&self, id: &SessionId, worktree: &Path) -> Result<(), RegistryError> {
        SessionRegistry::register(self, id, worktree, Instant::now()).map(|_| ())
    }

    fn heartbeat(&self, id: &SessionId) -> Result<(), RegistryError> {
        SessionRegistry::heartbeat(self, id, Instant::now())
    }

    fn unregister(&self, id: &SessionId) -> Result<bool, RegistryError> {
        SessionRegistry::unregister(self, id)
    }

    fn list(&self) -> Vec<SessionRecord> {
        SessionRegistry::active_sessions(self)
    }
}

fn canonicalise(path: &Path) -> Result<PathBuf, RegistryError> {
    std::fs::canonicalize(path).map_err(|source| RegistryError::WorktreePathInvalid {
        path: path.to_path_buf(),
        source,
    })
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Barrier;
    use std::thread;

    use tempfile::TempDir;

    use super::*;

    fn make_worktree() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn sid(name: &str) -> SessionId {
        SessionId::new(name)
    }

    #[test]
    fn register_list_unregister_round_trip() {
        let registry = SessionRegistry::new();
        let wt_a = make_worktree();
        let wt_b = make_worktree();
        let now = Instant::now();

        registry
            .register(&sid("a"), wt_a.path(), now)
            .expect("register a");
        registry
            .register(&sid("b"), wt_b.path(), now)
            .expect("register b");

        let listed = registry.active_sessions();
        assert_eq!(listed.len(), 2);
        // Sorted by (started_at_unix, id); both started in the same
        // second under test, so ids decide.
        assert_eq!(listed[0].id, sid("a"));
        assert_eq!(listed[1].id, sid("b"));

        let removed = registry.unregister(&sid("a")).expect("unregister");
        assert!(removed);

        let listed = registry.active_sessions();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, sid("b"));
    }

    #[test]
    fn unregister_unknown_id_is_not_an_error() {
        let registry = SessionRegistry::new();
        let removed = registry.unregister(&sid("ghost")).expect("call");
        assert!(!removed, "unknown id returns false, not an error");
    }

    #[test]
    fn second_session_on_same_worktree_returns_already_owned() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();

        registry
            .register(&sid("first"), wt.path(), now)
            .expect("first wins");

        let err = registry
            .register(&sid("second"), wt.path(), now)
            .expect_err("second must lose");

        assert_eq!(
            err,
            RegistryError::WorktreeAlreadyOwned {
                existing: sid("first")
            },
        );
    }

    /// A path with a trailing slash, a `..` segment, or a symlink
    /// indirection canonicalises to the same registry key — so a
    /// second register with a different spelling still detects the
    /// collision.
    #[test]
    fn worktree_path_canonicalisation_collapses_alternative_spellings() {
        let parent = make_worktree();
        let real = parent.path().join("real");
        std::fs::create_dir(&real).expect("real subdir");

        let registry = SessionRegistry::new();
        let now = Instant::now();
        registry
            .register(&sid("first"), &real, now)
            .expect("register canonical");

        // `real/.` should resolve to the same entry.
        let dotted = real.join(".");
        let err = registry
            .register(&sid("second"), &dotted, now)
            .expect_err("dotted form must collide");
        assert_eq!(
            err,
            RegistryError::WorktreeAlreadyOwned {
                existing: sid("first")
            },
        );

        // `parent/real/../real` should also collide.
        let dotdot = parent.path().join("real").join("..").join("real");
        let err = registry
            .register(&sid("third"), &dotdot, now)
            .expect_err("dotdot form must collide");
        assert_eq!(
            err,
            RegistryError::WorktreeAlreadyOwned {
                existing: sid("first")
            },
        );

        // session_for_worktree honours canonicalisation too.
        let found = registry
            .session_for_worktree(&dotted)
            .expect("lookup via dotted");
        assert_eq!(found.id, sid("first"));
    }

    #[test]
    fn worktree_path_missing_on_disk_is_rejected() {
        let registry = SessionRegistry::new();
        let now = Instant::now();
        let missing = std::path::Path::new("/definitely/not/here/anvil-intd-003-ghost");
        let err = registry
            .register(&sid("a"), missing, now)
            .expect_err("missing path");
        match err {
            RegistryError::WorktreePathInvalid { path, .. } => {
                assert_eq!(path, missing);
            }
            other => panic!("expected WorktreePathInvalid, got {other:?}"),
        }
    }

    #[test]
    fn heartbeat_unknown_id_returns_unknown_session() {
        let registry = SessionRegistry::new();
        let err = registry
            .heartbeat(&sid("ghost"), Instant::now())
            .expect_err("must fail");
        assert_eq!(err, RegistryError::UnknownSession(sid("ghost")));
    }

    #[test]
    fn heartbeat_refresh_keeps_session_alive_past_ttl_window() {
        let ttl = Duration::from_secs(30);
        let registry = SessionRegistry::with_ttl(ttl);
        let wt = make_worktree();
        let t0 = Instant::now();
        registry
            .register(&sid("a"), wt.path(), t0)
            .expect("register");

        let t10 = t0 + Duration::from_secs(10);
        registry
            .heartbeat(&sid("a"), t10)
            .expect("heartbeat at t=10s");

        // t=20s, last heartbeat 10s old, TTL 30s — still alive.
        let t20 = t0 + Duration::from_secs(20);
        assert!(registry.evict_stale(t20).is_empty());
        assert_eq!(registry.active_sessions().len(), 1);

        // t=45s, last heartbeat 35s old — evicted.
        let t45 = t0 + Duration::from_secs(45);
        let evicted = registry.evict_stale(t45);
        assert_eq!(evicted, vec![sid("a")]);
        assert!(registry.active_sessions().is_empty());
    }

    /// Pin the boundary: a session at exactly TTL is alive; at TTL +
    /// 1 ns it evicts. The docstring on `evict_stale` declares this
    /// policy; changing it should require updating both.
    #[test]
    fn ttl_boundary_is_inclusive_at_ttl_exclusive_just_after() {
        let ttl = Duration::from_secs(30);
        let registry = SessionRegistry::with_ttl(ttl);
        let wt = make_worktree();
        let t0 = Instant::now();
        registry
            .register(&sid("a"), wt.path(), t0)
            .expect("register");

        let exactly = t0 + ttl;
        assert!(
            registry.evict_stale(exactly).is_empty(),
            "session at exactly TTL must still be alive",
        );

        let just_after = t0 + ttl + Duration::from_nanos(1);
        let evicted = registry.evict_stale(just_after);
        assert_eq!(evicted, vec![sid("a")], "session past TTL must evict");
    }

    /// `pid` / `pgid` / `started_at_unix` update; supplying `None` does
    /// NOT clobber an existing `Some`.
    #[test]
    fn process_info_update_is_partial_and_idempotent() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();
        registry
            .register(&sid("a"), wt.path(), now)
            .expect("register");

        let updated = registry
            .update_process_info(
                &sid("a"),
                ProcessInfo {
                    pid: Some(1234),
                    pgid: Some(1234),
                    started_at_unix: Some(1_700_000_000),
                },
            )
            .expect("first update");
        assert_eq!(updated.pid, Some(1234));
        assert_eq!(updated.pgid, Some(1234));
        assert_eq!(updated.started_at_unix, 1_700_000_000);

        // None fields are no-ops — Some values from the prior update
        // survive.
        let again = registry
            .update_process_info(&sid("a"), ProcessInfo::default())
            .expect("noop update");
        assert_eq!(again.pid, Some(1234));
        assert_eq!(again.pgid, Some(1234));
        assert_eq!(again.started_at_unix, 1_700_000_000);

        // Re-applying the same update is idempotent.
        let same = registry
            .update_process_info(
                &sid("a"),
                ProcessInfo {
                    pid: Some(1234),
                    pgid: Some(1234),
                    started_at_unix: Some(1_700_000_000),
                },
            )
            .expect("idempotent update");
        assert_eq!(same, updated);
    }

    #[test]
    fn update_process_info_unknown_session_is_unknown_error() {
        let registry = SessionRegistry::new();
        let err = registry
            .update_process_info(&sid("ghost"), ProcessInfo::default())
            .expect_err("must fail");
        assert_eq!(err, RegistryError::UnknownSession(sid("ghost")));
    }

    /// Two threads racing on `register` for the same canonicalised
    /// worktree — exactly one wins; the other gets
    /// `WorktreeAlreadyOwned`. The registry must be `Send + Sync`.
    #[test]
    fn concurrent_register_on_same_worktree_picks_exactly_one_winner() {
        let registry = Arc::new(SessionRegistry::new());
        let wt = Arc::new(make_worktree());
        let barrier = Arc::new(Barrier::new(2));

        let r1 = Arc::clone(&registry);
        let w1 = Arc::clone(&wt);
        let b1 = Arc::clone(&barrier);
        let h1 = thread::spawn(move || {
            b1.wait();
            r1.register(&sid("a"), w1.path(), Instant::now())
        });

        let r2 = Arc::clone(&registry);
        let w2 = Arc::clone(&wt);
        let b2 = Arc::clone(&barrier);
        let h2 = thread::spawn(move || {
            b2.wait();
            r2.register(&sid("b"), w2.path(), Instant::now())
        });

        let result_1 = h1.join().expect("thread 1");
        let result_2 = h2.join().expect("thread 2");

        let oks = [&result_1, &result_2].iter().filter(|r| r.is_ok()).count();
        let errs = [&result_1, &result_2].iter().filter(|r| r.is_err()).count();
        assert_eq!(oks, 1, "exactly one register must succeed");
        assert_eq!(errs, 1, "exactly one register must fail");

        let listed = registry.active_sessions();
        assert_eq!(listed.len(), 1, "registry holds exactly one entry");
    }

    /// Active-list ordering is deterministic under concurrent
    /// registration: list it twice between identical state and the
    /// orderings match.
    #[test]
    fn active_sessions_ordering_is_deterministic_under_concurrency() {
        let registry = Arc::new(SessionRegistry::new());
        let dirs: Vec<TempDir> = (0..8).map(|_| make_worktree()).collect();
        let dirs = Arc::new(dirs);

        let mut handles = Vec::new();
        for i in 0..8 {
            let r = Arc::clone(&registry);
            let d = Arc::clone(&dirs);
            handles.push(thread::spawn(move || {
                r.register(&sid(&format!("s{i}")), d[i].path(), Instant::now())
                    .expect("register");
            }));
        }
        for h in handles {
            h.join().expect("join");
        }

        let first = registry.active_sessions();
        let second = registry.active_sessions();
        assert_eq!(
            first, second,
            "list ordering must be deterministic between calls",
        );

        // Sorted by (started_at_unix, id); ids tie-break.
        let mut expected: Vec<String> = (0..8).map(|i| format!("s{i}")).collect();
        expected.sort();
        let actual: Vec<String> = first.iter().map(|r| r.id.as_str().to_string()).collect();
        assert_eq!(actual, expected);
    }

    /// Crash-safe Drop is NOT this layer's job. A launcher whose
    /// process was SIGKILL-ed never calls `unregister`; we rely on
    /// `evict_stale` to reap. This test is a doc test for that
    /// contract — without it, a future refactor that adds a Drop
    /// guard could silently change the behaviour.
    #[test]
    fn crashed_launcher_recovery_relies_on_evict_stale_not_drop() {
        let ttl = Duration::from_millis(1);
        let registry = SessionRegistry::with_ttl(ttl);
        let wt = make_worktree();
        let t0 = Instant::now();
        registry
            .register(&sid("crashed"), wt.path(), t0)
            .expect("register");

        // Simulate "process gone" by simply not calling unregister and
        // letting the heartbeat go stale.
        let later = t0 + Duration::from_millis(5);
        let evicted = registry.evict_stale(later);
        assert_eq!(evicted, vec![sid("crashed")]);

        // After eviction, the worktree is free for a fresh registration.
        registry
            .register(&sid("recovered"), wt.path(), later)
            .expect("worktree freed by eviction");
    }

    /// `SessionDispatcher` trait dispatch works against the concrete
    /// registry — this is the surface INTD-002 calls into via
    /// `Arc<dyn SessionDispatcher>`.
    #[test]
    fn session_dispatcher_trait_dispatches_to_registry() {
        let registry: Arc<dyn SessionDispatcher> = Arc::new(SessionRegistry::new());
        let wt = make_worktree();

        registry.register(&sid("a"), wt.path()).expect("register");
        assert_eq!(registry.list().len(), 1);

        registry.heartbeat(&sid("a")).expect("heartbeat");
        assert!(registry.unregister(&sid("a")).expect("unregister"));
        assert!(registry.list().is_empty());
    }
}
