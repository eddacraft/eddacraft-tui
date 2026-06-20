use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

use crate::rate_window::{RateDecision, RateWindow};
use crate::registry::SessionRegistry;
use crate::telemetry::{FenceTransition, TelemetryCorrelation, TelemetryEmitter};

const FENCE_FILE_VERSION: u8 = 1;

/// MLP2-026: rate-window capacity for the `degraded:fence-cascade`
/// detector. `RateWindow` admits up to `capacity` events before
/// throttling; the 5-in-60s threshold requires capacity 4 so the
/// fifth `record()` call within 60 s returns
/// `RateDecision::Throttle` — that is the engage trigger.
///
/// See `plans/specs/2026-05-16-mlp2-026-fence-cascade-control-lane.md`
/// §3.1 and the Council 2026-05-15 off-by-one correction.
pub const CASCADE_RATE_WINDOW_CAPACITY: usize = 4;

/// MLP2-026: rate-window duration paired with
/// [`CASCADE_RATE_WINDOW_CAPACITY`]. Five fires within 60 s engage
/// the cascade.
pub const CASCADE_RATE_WINDOW_DURATION: Duration = Duration::from_mins(1);

#[derive(Debug, Error)]
pub enum FenceStoreError {
    #[error("cannot resolve user state directory for anvil intercept fences")]
    StateDirectoryUnavailable,

    #[error("worktree path could not be canonicalised: {path:?}: {source}")]
    WorktreePathInvalid {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read fence store {path:?}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write fence store {path:?}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse fence store {path:?}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("unsupported fence store version {version} in {path:?}")]
    UnsupportedVersion { path: PathBuf, version: u8 },

    #[error("invalid fence record in {path:?}: {reason}")]
    InvalidRecord { path: PathBuf, reason: String },

    #[error("insecure fence store parent {path:?}: {reason}")]
    InsecureStoreParent { path: PathBuf, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FenceRecord {
    pub worktree: PathBuf,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<PathBuf>,
    pub reason: String,
    pub fenced_at_unix: u64,
}

/// MLP2-026: per-worktree cascade engaged-state record. Persisted
/// inside `FenceFile.cascades` so daemon restart preserves the
/// security-relevant engaged flag — only the in-memory
/// [`RateWindow`] resets on restart, which is the correct posture:
/// the engaged flag stays sticky, the firing window is rebuilt.
///
/// See `plans/specs/2026-05-16-mlp2-026-fence-cascade-control-lane.md`
/// §3.1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeRecord {
    /// Canonical worktree path. Mirrors the
    /// [`FenceRecord::worktree`] canonicalisation convention.
    pub worktree: PathBuf,
    /// Engage timestamp as Unix seconds. Mirrors
    /// [`FenceRecord::fenced_at_unix`].
    pub since_unix: u64,
    /// Always [`crate::telemetry::DEGRADED_FENCE_CASCADE`] for v1.
    /// Stored explicitly so structured-log consumers do not need
    /// to look up the constant.
    pub reason: String,
}

impl CascadeRecord {
    fn matches(&self, worktree: &Path) -> bool {
        self.worktree == worktree
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FenceState {
    records: Vec<FenceRecord>,
    /// MLP2-026: cascade engaged-state records loaded from
    /// [`FenceFile::cascades`]. Persistence keeps cascade sticky
    /// across daemon restart; the in-memory rate windows on
    /// [`FenceStore`] are NOT persisted.
    cascades: Vec<CascadeRecord>,
}

impl FenceState {
    #[must_use]
    pub fn active_fences(&self) -> &[FenceRecord] {
        &self.records
    }

    #[must_use]
    pub fn is_fenced(&self, worktree: &Path) -> bool {
        let Some(canonical) = lookup_path(worktree) else {
            return false;
        };
        self.records.iter().any(|record| record.matches(&canonical))
    }

    /// MLP2-071 D6: `true` iff the worktree currently carries a
    /// **spoof** fence — a [`FenceRecord`] whose reason is
    /// [`crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION`], written by
    /// [`Self::fence_worktree_for_spoof`] (MLP2-025) when the
    /// write-time env-tag cross-check fails.
    ///
    /// The fan-out's [`crate::fanout::RegistryOwnershipResolver`]
    /// consults this (via the session's worktree) to deny a
    /// degraded-spoofed origin's envelopes to cross-session
    /// subscribers regardless of policy. A non-spoof fence
    /// (`degraded:fence-cascade`, an explicit operator fence) does
    /// NOT make a session degraded-spoofed, so this is narrower than
    /// [`Self::is_fenced`].
    #[must_use]
    pub fn is_spoof_fenced(&self, worktree: &Path) -> bool {
        let Some(canonical) = lookup_path(worktree) else {
            return false;
        };
        self.records.iter().any(|record| {
            record.matches(&canonical)
                && record.reason == crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION
        })
    }

    /// MLP2-026: read-only access to the persisted cascade
    /// engaged-state records.
    #[must_use]
    pub fn active_cascades(&self) -> &[CascadeRecord] {
        &self.cascades
    }

    /// MLP2-026: `true` iff a [`CascadeRecord`] exists for any
    /// path that canonicalises to `worktree`. See spec §6 `inv-1`.
    #[must_use]
    pub fn is_cascaded(&self, worktree: &Path) -> bool {
        let Some(canonical) = lookup_path(worktree) else {
            return false;
        };
        self.cascades
            .iter()
            .any(|record| record.matches(&canonical))
    }

    fn upsert(&mut self, record: FenceRecord) {
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|existing| existing.matches(&record.worktree))
        {
            *existing = record;
        } else {
            self.records.push(record);
        }
        self.records.sort_by(|a, b| a.worktree.cmp(&b.worktree));
    }

    fn remove(&mut self, worktree: &Path) -> Option<FenceRecord> {
        let index = self
            .records
            .iter()
            .position(|record| record.matches(worktree))?;
        Some(self.records.remove(index))
    }

    fn upsert_cascade(&mut self, record: CascadeRecord) {
        if let Some(existing) = self
            .cascades
            .iter_mut()
            .find(|existing| existing.matches(&record.worktree))
        {
            *existing = record;
        } else {
            self.cascades.push(record);
        }
        self.cascades.sort_by(|a, b| a.worktree.cmp(&b.worktree));
    }

    fn remove_cascade(&mut self, worktree: &Path) -> Option<CascadeRecord> {
        let index = self
            .cascades
            .iter()
            .position(|record| record.matches(worktree))?;
        Some(self.cascades.remove(index))
    }
}

impl FenceRecord {
    fn matches(&self, worktree: &Path) -> bool {
        self.worktree == worktree || self.aliases.iter().any(|alias| alias == worktree)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FenceFile {
    version: u8,
    fences: Vec<FenceRecord>,
    /// MLP2-026: cascade engaged-state records. Wire-additive via
    /// `#[serde(default, skip_serializing_if = "Vec::is_empty")]`,
    /// matching the `FenceRecord::aliases` precedent. `version`
    /// stays at 1; a pre-MLP2-026 fence file (no `cascades` key)
    /// deserialises with `cascades = vec![]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    cascades: Vec<CascadeRecord>,
}

/// MLP2-026: in-memory rate-window registry shared across all
/// [`FenceStore`] clones. The on-disk [`FenceFile`] is the
/// persistent layer; this map only tracks the **firing rate** —
/// it rebuilds empty on daemon restart, which is the correct
/// posture (the engaged flag survives via [`CascadeRecord`], the
/// firing window does not).
type CascadeWindows = Arc<Mutex<HashMap<PathBuf, Arc<RateWindow>>>>;

#[derive(Clone)]
pub struct FenceStore {
    path: PathBuf,
    loaded_state: Arc<Mutex<Option<FenceState>>>,
    /// MLP2-026: per-worktree firing-rate trackers. Lazily created
    /// on first fire for a worktree.
    cascade_windows: CascadeWindows,
    telemetry: Arc<Mutex<Option<Arc<FenceTelemetry>>>>,
    /// DPO-002: the Kindling sink + daemon session id for
    /// `constraint_applied` rows. Stored parallel to `telemetry` (its
    /// own Mutex-held slot) so a fence engage produces an audit-grade
    /// constraint row even when the cross-session telemetry fan-out is
    /// not wired. `None` (the default) keeps fence engages silent.
    observation: Arc<Mutex<Option<FenceObservation>>>,
}

struct FenceTelemetry {
    registry: Arc<SessionRegistry>,
    broadcaster: Arc<crate::broadcaster::TelemetryBroadcaster>,
    emitter: Mutex<TelemetryEmitter>,
}

/// DPO-002: the constraint-observation collaborators a [`FenceStore`]
/// holds — the Kindling sink and the daemon-stable session id stamped
/// onto every `constraint_applied` row.
struct FenceObservation {
    sink: Arc<dyn crate::kindling_observation::KindlingObservationSink>,
    daemon_session_id: String,
    /// DPO-002 (council C): whether the absolute worktree path may appear
    /// on the `constraint_applied` row. Threaded from the same
    /// `ANVIL_OBSERVATION_INCLUDE_PATHS` posture the save-time emitter
    /// uses; when `false` the row's `worktree` field is redacted.
    include_paths: bool,
}

impl FenceStore {
    #[must_use]
    pub fn at_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            loaded_state: Arc::new(Mutex::new(None)),
            cascade_windows: Arc::new(Mutex::new(HashMap::new())),
            telemetry: Arc::new(Mutex::new(None)),
            observation: Arc::new(Mutex::new(None)),
        }
    }

    /// DPO-002: inject the Kindling sink (and daemon session id) so every
    /// successful fence engage emits one `constraint_applied` row. Mirrors the
    /// [`Self::with_telemetry`] builder pattern; independent of telemetry so a
    /// host can wire either, both, or neither. Production wires this from
    /// `anvil-cli` alongside the other observation surfaces.
    ///
    /// `include_paths` (council C) gates whether the row carries the real
    /// absolute worktree path; pass the same value the save-time emitter
    /// derives from `ANVIL_OBSERVATION_INCLUDE_PATHS`.
    #[must_use]
    pub fn with_observation_sink(
        self,
        sink: Arc<dyn crate::kindling_observation::KindlingObservationSink>,
        daemon_session_id: String,
        include_paths: bool,
    ) -> Self {
        self.set_observation_sink(sink, daemon_session_id, include_paths);
        self
    }

    /// DPO-002: set the Kindling sink in place (the non-consuming form of
    /// [`Self::with_observation_sink`], mirroring [`Self::set_telemetry`]).
    pub fn set_observation_sink(
        &self,
        sink: Arc<dyn crate::kindling_observation::KindlingObservationSink>,
        daemon_session_id: String,
        include_paths: bool,
    ) {
        *self
            .observation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(FenceObservation {
            sink,
            daemon_session_id,
            include_paths,
        });
    }

    #[must_use]
    pub fn with_telemetry(
        self,
        registry: Arc<SessionRegistry>,
        broadcaster: Arc<crate::broadcaster::TelemetryBroadcaster>,
    ) -> Self {
        self.set_telemetry(registry, broadcaster);
        self
    }

    pub fn set_telemetry(
        &self,
        registry: Arc<SessionRegistry>,
        broadcaster: Arc<crate::broadcaster::TelemetryBroadcaster>,
    ) {
        *self
            .telemetry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(FenceTelemetry {
            registry,
            broadcaster,
            emitter: Mutex::new(TelemetryEmitter::new()),
        }));
    }

    /// Return the last successfully loaded/saved spoof-fence view without
    /// touching the filesystem. A missing cache means the store has not proved
    /// a clean state in this process, so callers that enforce cross-session
    /// telemetry policy should fail closed.
    #[must_use]
    pub fn is_spoof_fenced_cached(&self, worktree: &Path) -> Option<bool> {
        self.loaded_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|state| state.is_spoof_fenced(worktree))
    }

    pub(crate) fn cache_loaded_state(&self, state: &FenceState) {
        self.set_loaded_state(state);
    }

    /// MLP2-026: snapshot accessor for cascade engaged-state. See
    /// spec §5.2. Returns `false` on `load()` failure rather than
    /// propagating — the register-time call site (§4.2) is on the
    /// hot path and a degraded fence-store I/O is a separate
    /// concern that [`crate::registry::RegistryError::FenceStateUnavailable`]
    /// surfaces on other paths.
    #[must_use]
    pub fn is_cascaded(&self, worktree: &Path) -> bool {
        self.load().is_ok_and(|state| state.is_cascaded(worktree))
    }

    /// MLP2-026: operator-clear of a cascade engaged-state. Removes
    /// the matching [`CascadeRecord`] from the on-disk file, resets
    /// the in-memory rate window for the worktree, and persists
    /// the change. Idempotent.
    ///
    /// Returns `Ok(true)` when a cascade record existed and was
    /// removed, `Ok(false)` when no cascade was engaged for the
    /// worktree (idempotent operator-clear). `Err(_)` only on
    /// underlying I/O failure.
    ///
    /// See spec §5.3.
    pub fn clear_cascade(&self, worktree: &Path) -> Result<bool, FenceStoreError> {
        let canonical =
            lookup_path(worktree).ok_or_else(|| FenceStoreError::WorktreePathInvalid {
                path: worktree.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "worktree must be absolute or canonicalisable to clear cascade",
                ),
            })?;
        let mut state = self.load()?;
        let removed = state.remove_cascade(&canonical);
        if removed.is_some() {
            self.save(&state)?;
        }
        // Reset the in-memory rate window unconditionally — see spec
        // §6 inv-3: defensive on both Ok(true) and the
        // idempotent-miss `Ok(false)` arm.
        self.reset_cascade_window(&canonical);
        Ok(removed.is_some())
    }

    /// MLP2-026: record a fence-fire through the per-worktree
    /// rate window and return whether the fire engaged a cascade.
    /// Internal helper called from [`Self::fence_worktree`] (F2).
    fn record_cascade_fire(&self, worktree: &Path, now: Instant) -> RateDecision {
        let window = {
            let mut windows = self
                .cascade_windows
                .lock()
                .expect("cascade_windows lock poisoned");
            Arc::clone(windows.entry(worktree.to_path_buf()).or_insert_with(|| {
                Arc::new(RateWindow::new(
                    CASCADE_RATE_WINDOW_CAPACITY,
                    CASCADE_RATE_WINDOW_DURATION,
                ))
            }))
        };
        window.record(now)
    }

    /// MLP2-026: reset the in-memory rate window for a worktree.
    /// Drops the existing `RateWindow` so the next fire starts
    /// counting from zero. Called from [`Self::clear_cascade`].
    fn reset_cascade_window(&self, worktree: &Path) {
        let mut windows = self
            .cascade_windows
            .lock()
            .expect("cascade_windows lock poisoned");
        windows.remove(worktree);
    }

    pub fn load(&self) -> Result<FenceState, FenceStoreError> {
        #[cfg(unix)]
        if let Err(error) = validate_store_parent(&self.path) {
            self.clear_loaded_state();
            return Err(error);
        }
        #[cfg(windows)]
        if let Err(error) = recover_windows_backup(&self.path) {
            self.clear_loaded_state();
            return Err(error);
        }
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                let state = FenceState::default();
                self.set_loaded_state(&state);
                return Ok(state);
            }
            Err(source) => {
                self.clear_loaded_state();
                return Err(FenceStoreError::Read {
                    path: self.path.clone(),
                    source,
                });
            }
        };
        let file: FenceFile = match serde_json::from_str(&content) {
            Ok(file) => file,
            Err(source) => {
                self.clear_loaded_state();
                return Err(FenceStoreError::Parse {
                    path: self.path.clone(),
                    source,
                });
            }
        };
        if file.version != FENCE_FILE_VERSION {
            self.clear_loaded_state();
            return Err(FenceStoreError::UnsupportedVersion {
                path: self.path.clone(),
                version: file.version,
            });
        }
        let mut state = FenceState {
            records: match validate_records(&self.path, file.fences) {
                Ok(records) => records,
                Err(error) => {
                    self.clear_loaded_state();
                    return Err(error);
                }
            },
            cascades: match validate_cascades(&self.path, file.cascades) {
                Ok(cascades) => cascades,
                Err(error) => {
                    self.clear_loaded_state();
                    return Err(error);
                }
            },
        };
        state.records.sort_by(|a, b| a.worktree.cmp(&b.worktree));
        state.cascades.sort_by(|a, b| a.worktree.cmp(&b.worktree));
        self.set_loaded_state(&state);
        Ok(state)
    }

    pub fn fence_worktree(
        &self,
        worktree: &Path,
        reason: impl Into<String>,
    ) -> Result<FenceRecord, FenceStoreError> {
        let canonical = canonicalise_worktree(worktree)?;
        let aliases = original_worktree_alias(worktree, &canonical)?;
        let now_unix = unix_seconds_now();
        let record = FenceRecord {
            worktree: canonical.clone(),
            aliases,
            reason: reason.into(),
            fenced_at_unix: now_unix,
        };

        // MLP2-026: record this fire through the per-worktree
        // rate window. On Throttle, engage the cascade (upsert a
        // CascadeRecord) and emit telemetry exactly once per
        // engage (spec §6 inv-3, §10 Q3 — emit-once, not
        // per-excess-fire).
        let decision = self.record_cascade_fire(&canonical, Instant::now());

        let mut state = self.load()?;
        state.upsert(record.clone());

        let cascade_engaged =
            matches!(decision, RateDecision::Throttle { .. }) && !state.is_cascaded(&canonical);
        if cascade_engaged {
            state.upsert_cascade(CascadeRecord {
                worktree: canonical.clone(),
                since_unix: now_unix,
                reason: crate::telemetry::DEGRADED_FENCE_CASCADE.to_string(),
            });
            tracing::warn!(
                target: "anvil_intercept::fence",
                reason = crate::telemetry::DEGRADED_FENCE_CASCADE,
                worktree = %canonical.display(),
                since_unix = now_unix,
                "cascade engaged after 5 fences in 60s",
            );
        }

        // DPO-002: every successful engage produces exactly one
        // `constraint_applied` row (the council producer-coverage fix —
        // pre-DPO-002 only the rate-limited cascade transition surfaced). Emit
        // once per call, not per excess fire; `cascade` flags whether this same
        // call engaged the cascade. On a cascade engage the row carries the
        // pinned cascade reason (matching the persisted `CascadeRecord`); an
        // ordinary engage carries the engage's own reason (normalised by
        // `from_fence`).
        //
        // Emit BEFORE the fence-file persist (ADR-088 / council T4):
        // emitting first gives best-effort ORDERING (the row tends to
        // precede the persisted state). The row itself is BEST-EFFORT, not
        // guaranteed (council D): it crosses the non-blocking sink boundary
        // and can be dropped on a full channel. The authoritative record of
        // which worktrees are fenced is the persistent fence-state file
        // written by `save()` below — the observation row is a queryable
        // signal, not the source of truth. The sink contract is
        // non-blocking and errors are swallowed, so this cannot fail or
        // stall the persist.
        let constraint_reason = if cascade_engaged {
            crate::telemetry::DEGRADED_FENCE_CASCADE
        } else {
            record.reason.as_str()
        };
        self.emit_constraint_applied(&canonical, constraint_reason, cascade_engaged);

        self.save(&state)?;
        self.emit_fence_transition(&canonical, FenceTransition::ActiveToFenced);
        Ok(record)
    }

    /// DPO-002: emit one `constraint_applied` Kindling row for a successful
    /// fence engage. No-op when no sink is wired. Sink errors are logged and
    /// swallowed — a fence is persisted regardless of observation-sink health.
    fn emit_constraint_applied(&self, worktree: &Path, reason: &str, cascade: bool) {
        let observation = self
            .observation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(observation) = observation.as_ref() else {
            return;
        };
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let row = crate::kindling_observation::from_fence(
            &observation.daemon_session_id,
            &timestamp,
            &worktree.display().to_string(),
            reason,
            cascade,
            observation.include_paths,
        );
        if let Err(err) = observation.sink.try_emit_constraint_applied(row) {
            tracing::warn!(
                target: "anvil_intercept::fence",
                error = %err,
                worktree = %worktree.display(),
                "constraint_applied emit dropped: sink failure",
            );
        }
    }

    /// MLP2-025b: convenience over [`Self::fence_worktree`] that pins
    /// the reason string to the
    /// [`crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION`] const so the
    /// daemon control-lane has exactly one source of truth for the
    /// spoof-fence reason. See
    /// `plans/specs/2026-05-16-mlp2-025-spoof-cross-check-control-lane.md`
    /// §5.3.
    pub fn fence_worktree_for_spoof(
        &self,
        worktree: &Path,
    ) -> Result<FenceRecord, FenceStoreError> {
        self.fence_worktree(worktree, crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION)
    }

    pub fn unblock_worktree(
        &self,
        worktree: &Path,
    ) -> Result<Option<FenceRecord>, FenceStoreError> {
        let canonical =
            lookup_path(worktree).ok_or_else(|| FenceStoreError::WorktreePathInvalid {
                path: worktree.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "worktree must be absolute or canonicalisable to unblock",
                ),
            })?;
        let mut state = self.load()?;
        let removed = state.remove(&canonical);
        if removed.is_some() {
            self.save(&state)?;
            self.emit_fence_transition(&canonical, FenceTransition::FencedToActive);
        }
        Ok(removed)
    }

    fn emit_fence_transition(&self, worktree: &Path, transition: FenceTransition) {
        let telemetry = self
            .telemetry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let Some(telemetry) = telemetry else {
            return;
        };
        let sessions: Vec<_> = telemetry
            .registry
            .active_sessions()
            .into_iter()
            .filter(|session| session.worktree == worktree)
            .collect();
        if sessions.is_empty() {
            return;
        }

        let mut emitter = telemetry
            .emitter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for session in sessions {
            let session_id = session.id.as_str().to_string();
            let envelope = emitter.envelope_for_fence_transition(
                TelemetryCorrelation {
                    session_id: Some(session_id.clone()),
                    originating_session_id: Some(session_id),
                    originating_driver_id: Some(crate::telemetry::INTERCEPT_DRIVER_ID.to_string()),
                    ..TelemetryCorrelation::default()
                },
                worktree,
                transition,
            );
            let outcome = telemetry.broadcaster.broadcast(&envelope);
            tracing::debug!(
                target: "anvil_intercept::fence",
                worktree = %worktree.display(),
                delivered = outcome.delivered,
                dropped = outcome.dropped,
                "fence transition broadcast",
            );
        }
    }

    fn save(&self, state: &FenceState) -> Result<(), FenceStoreError> {
        ensure_store_parent(&self.path)?;
        let file = FenceFile {
            version: FENCE_FILE_VERSION,
            fences: state.records.clone(),
            cascades: state.cascades.clone(),
        };
        let mut content =
            serde_json::to_vec_pretty(&file).map_err(|source| FenceStoreError::Write {
                path: self.path.clone(),
                source: std::io::Error::other(source),
            })?;
        content.push(b'\n');

        let tmp = temporary_store_path(&self.path);
        let mut file = create_store_file(&tmp)?;
        file.write_all(&content)
            .and_then(|()| file.sync_all())
            .map_err(|source| FenceStoreError::Write {
                path: tmp.clone(),
                source,
            })?;
        drop(file);
        replace_store_file(&tmp, &self.path)?;
        #[cfg(unix)]
        sync_parent(&self.path)?;
        self.set_loaded_state(state);
        Ok(())
    }

    fn set_loaded_state(&self, state: &FenceState) {
        *self
            .loaded_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(state.clone());
    }

    fn clear_loaded_state(&self) {
        *self
            .loaded_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
}

fn validate_records(
    store_path: &Path,
    records: Vec<FenceRecord>,
) -> Result<Vec<FenceRecord>, FenceStoreError> {
    let mut seen = HashSet::new();
    for record in &records {
        if !record.worktree.is_absolute() {
            return Err(FenceStoreError::InvalidRecord {
                path: store_path.to_path_buf(),
                reason: format!(
                    "fenced worktree is not absolute: {}",
                    record.worktree.display(),
                ),
            });
        }
        for alias in &record.aliases {
            if !alias.is_absolute() {
                return Err(FenceStoreError::InvalidRecord {
                    path: store_path.to_path_buf(),
                    reason: format!("fenced worktree alias is not absolute: {}", alias.display()),
                });
            }
        }
        if !seen.insert(record.worktree.clone()) {
            return Err(FenceStoreError::InvalidRecord {
                path: store_path.to_path_buf(),
                reason: format!("duplicate fenced worktree: {}", record.worktree.display()),
            });
        }
        for alias in &record.aliases {
            if !seen.insert(alias.clone()) {
                return Err(FenceStoreError::InvalidRecord {
                    path: store_path.to_path_buf(),
                    reason: format!("duplicate fenced worktree alias: {}", alias.display()),
                });
            }
        }
    }
    Ok(records)
}

/// MLP2-026: validate cascade records on `load()`. Mirrors
/// [`validate_records`]: absolute paths only, no duplicates.
fn validate_cascades(
    store_path: &Path,
    records: Vec<CascadeRecord>,
) -> Result<Vec<CascadeRecord>, FenceStoreError> {
    let mut seen = HashSet::new();
    for record in &records {
        if !record.worktree.is_absolute() {
            return Err(FenceStoreError::InvalidRecord {
                path: store_path.to_path_buf(),
                reason: format!(
                    "cascade worktree is not absolute: {}",
                    record.worktree.display(),
                ),
            });
        }
        if !seen.insert(record.worktree.clone()) {
            return Err(FenceStoreError::InvalidRecord {
                path: store_path.to_path_buf(),
                reason: format!("duplicate cascade worktree: {}", record.worktree.display()),
            });
        }
    }
    Ok(records)
}

fn original_worktree_alias(
    worktree: &Path,
    canonical: &Path,
) -> Result<Vec<PathBuf>, FenceStoreError> {
    let original = if worktree.is_absolute() {
        worktree.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|source| FenceStoreError::WorktreePathInvalid {
                path: worktree.to_path_buf(),
                source,
            })?
            .join(worktree)
    };

    if original == canonical {
        Ok(Vec::new())
    } else {
        Ok(vec![original])
    }
}

fn temporary_store_path(path: &Path) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    path.with_extension(format!("json.tmp.{}.{unique}", std::process::id()))
}

fn create_store_file(path: &Path) -> Result<File, FenceStoreError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    options.open(path).map_err(|source| FenceStoreError::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn replace_store_file(tmp: &Path, target: &Path) -> Result<(), FenceStoreError> {
    #[cfg(windows)]
    {
        let backup = windows_backup_path(target);
        if backup.exists() {
            fs::remove_file(&backup).map_err(|source| FenceStoreError::Write {
                path: backup.clone(),
                source,
            })?;
        }
        match fs::rename(target, &backup) {
            Ok(()) => {}
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(FenceStoreError::Write {
                    path: target.to_path_buf(),
                    source,
                });
            }
        }

        if let Err(source) = fs::rename(tmp, target) {
            if backup.exists() {
                let _ = fs::rename(&backup, target);
            }
            return Err(FenceStoreError::Write {
                path: target.to_path_buf(),
                source,
            });
        }
        if backup.exists() {
            fs::remove_file(&backup).map_err(|source| FenceStoreError::Write {
                path: backup,
                source,
            })?;
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        fs::rename(tmp, target).map_err(|source| FenceStoreError::Write {
            path: target.to_path_buf(),
            source,
        })
    }
}

#[cfg(windows)]
fn recover_windows_backup(target: &Path) -> Result<(), FenceStoreError> {
    let backup = windows_backup_path(target);
    if !target.exists() && backup.exists() {
        fs::rename(&backup, target).map_err(|source| FenceStoreError::Write {
            path: target.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

#[cfg(windows)]
fn windows_backup_path(target: &Path) -> PathBuf {
    target.with_extension("json.bak")
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> Result<(), FenceStoreError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|source| FenceStoreError::Write {
            path: parent.to_path_buf(),
            source,
        })?;

    Ok(())
}

pub fn default_fence_state_path() -> Result<PathBuf, FenceStoreError> {
    default_fence_state_path_from_env(|name| env::var_os(name))
}

fn default_fence_state_path_from_env(
    mut get_env: impl FnMut(&str) -> Option<OsString>,
) -> Result<PathBuf, FenceStoreError> {
    if cfg!(windows)
        && let Some(local_app_data) = non_empty_env(&mut get_env, "LOCALAPPDATA")
    {
        return Ok(local_app_data.join("anvil").join("intercept-fences.json"));
    }

    if let Some(state_home) = non_empty_env(&mut get_env, "XDG_STATE_HOME") {
        return Ok(state_home.join("anvil").join("intercept-fences.json"));
    }

    let home = non_empty_env(&mut get_env, "HOME")
        .or_else(|| non_empty_env(&mut get_env, "USERPROFILE"))
        .ok_or(FenceStoreError::StateDirectoryUnavailable)?;
    Ok(home
        .join(".local")
        .join("state")
        .join("anvil")
        .join("intercept-fences.json"))
}

fn non_empty_env(
    get_env: &mut impl FnMut(&str) -> Option<OsString>,
    name: &str,
) -> Option<PathBuf> {
    get_env(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn canonicalise_worktree(worktree: &Path) -> Result<PathBuf, FenceStoreError> {
    fs::canonicalize(worktree).map_err(|source| FenceStoreError::WorktreePathInvalid {
        path: worktree.to_path_buf(),
        source,
    })
}

fn lookup_path(worktree: &Path) -> Option<PathBuf> {
    fs::canonicalize(worktree)
        .ok()
        .or_else(|| worktree.is_absolute().then(|| worktree.to_path_buf()))
}

fn ensure_store_parent(path: &Path) -> Result<(), FenceStoreError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    #[cfg(unix)]
    {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder
            .create(parent)
            .map_err(|source| FenceStoreError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        validate_existing_store_parent(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|source| {
            FenceStoreError::Write {
                path: parent.to_path_buf(),
                source,
            }
        })?;
    }

    #[cfg(not(unix))]
    fs::create_dir_all(parent).map_err(|source| FenceStoreError::Write {
        path: parent.to_path_buf(),
        source,
    })?;

    Ok(())
}

#[cfg(unix)]
fn validate_store_parent(path: &Path) -> Result<(), FenceStoreError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    if parent.exists() {
        validate_existing_store_parent(parent)?;
    }

    Ok(())
}

#[cfg(unix)]
fn validate_existing_store_parent(parent: &Path) -> Result<(), FenceStoreError> {
    let metadata = fs::symlink_metadata(parent).map_err(|source| FenceStoreError::Write {
        path: parent.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(FenceStoreError::InsecureStoreParent {
            path: parent.to_path_buf(),
            reason: "parent must be a real directory, not a symlink".to_string(),
        });
    }
    if metadata.uid() != nix::unistd::geteuid().as_raw() {
        return Err(FenceStoreError::InsecureStoreParent {
            path: parent.to_path_buf(),
            reason: "parent must be owned by the current user".to_string(),
        });
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(FenceStoreError::InsecureStoreParent {
            path: parent.to_path_buf(),
            reason: "parent must be private to the current user".to_string(),
        });
    }
    Ok(())
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    use anvil_intercept_proto::SessionId;
    use tempfile::TempDir;

    use crate::registry::SessionRegistry;
    use crate::{
        broadcaster::TelemetryBroadcaster,
        fanout::{CrossSessionPolicy, Fanout, OwnershipResolver, SubscriberId},
    };

    use super::*;

    fn make_worktree() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn store_in(temp: &TempDir) -> FenceStore {
        FenceStore::at_path(temp.path().join("state/intercept-fences.json"))
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

    fn store_with_telemetry(
        temp: &TempDir,
        worktree: &TempDir,
    ) -> (
        FenceStore,
        Arc<TelemetryBroadcaster>,
        tokio::sync::mpsc::Receiver<String>,
    ) {
        let registry = Arc::new(SessionRegistry::new());
        let session = SessionId::new("sess-A");
        registry
            .register(&session, worktree.path(), None, Instant::now())
            .expect("register session");
        let subscriber = SubscriberId::new("subscriber-A");
        let fanout = Arc::new(Fanout::with_cross_session_policy(
            Box::new(SingleOwnerResolver {
                subscriber: subscriber.clone(),
                session_id: session.as_str().to_string(),
            }),
            CrossSessionPolicy::Deny,
        ));
        let broadcaster = Arc::new(TelemetryBroadcaster::new(fanout));
        let rx = broadcaster.register(subscriber, None);
        let store = store_in(temp).with_telemetry(registry, Arc::clone(&broadcaster));
        (store, broadcaster, rx)
    }

    #[test]
    fn missing_store_loads_as_empty_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = store_in(&temp).load().expect("load missing store");

        assert!(state.active_fences().is_empty());
    }

    #[test]
    fn fenced_worktree_survives_store_reload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let store = store_in(&temp);

        store
            .fence_worktree(worktree.path(), "rule violation")
            .expect("fence worktree");
        let reloaded = store.load().expect("reload fences");

        assert!(reloaded.is_fenced(worktree.path()));
        assert_eq!(reloaded.active_fences()[0].reason, "rule violation");
    }

    #[test]
    fn fence_worktree_emits_active_to_fenced_through_fanout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let (store, broadcaster, mut rx) = store_with_telemetry(&temp, &worktree);

        store
            .fence_worktree(worktree.path(), "rule violation")
            .expect("fence worktree");

        let frame = rx.try_recv().expect("fence transition frame queued");
        let value: serde_json::Value = serde_json::from_str(&frame).expect("frame json");
        assert_eq!(
            value["params"]["correlation"]["originating_session_id"],
            "sess-A",
        );
        assert_eq!(
            value["params"]["grouping"]["transition"],
            serde_json::json!({"from": "active", "to": "fenced"}),
        );
        assert_eq!(broadcaster.dropped_envelopes(), 0);
    }

    #[test]
    fn unblock_worktree_emits_fenced_to_active_through_fanout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let (store, _broadcaster, mut rx) = store_with_telemetry(&temp, &worktree);
        store
            .fence_worktree(worktree.path(), "rule violation")
            .expect("fence worktree");
        let _ = rx.try_recv().expect("initial fence transition");

        store
            .unblock_worktree(worktree.path())
            .expect("unblock worktree")
            .expect("fence existed");

        let frame = rx.try_recv().expect("unblock transition frame queued");
        let value: serde_json::Value = serde_json::from_str(&frame).expect("frame json");
        assert_eq!(
            value["params"]["grouping"]["transition"],
            serde_json::json!({"from": "fenced", "to": "active"}),
        );
    }

    #[test]
    fn session_eviction_does_not_clear_persisted_fence() {
        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let store = store_in(&temp);
        let registry = SessionRegistry::with_ttl(Duration::from_millis(1));
        let session = SessionId::new("sess-evict");
        let registered_at = Instant::now();

        registry
            .register(&session, worktree.path(), None, registered_at)
            .expect("register session");
        store
            .fence_worktree(worktree.path(), "manual review required")
            .expect("fence worktree");
        registry.evict_stale(registered_at + Duration::from_millis(2));

        assert!(
            store
                .load()
                .expect("reload fences")
                .is_fenced(worktree.path())
        );
    }

    #[test]
    fn explicit_unblock_removes_persisted_fence() {
        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let store = store_in(&temp);

        store
            .fence_worktree(worktree.path(), "operator action")
            .expect("fence worktree");
        let removed = store
            .unblock_worktree(worktree.path())
            .expect("unblock worktree");

        assert_eq!(removed.expect("removed fence").reason, "operator action");
        assert!(
            !store
                .load()
                .expect("reload fences")
                .is_fenced(worktree.path())
        );
    }

    #[test]
    fn deleted_worktree_can_still_be_queried_and_unblocked() {
        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let worktree_path = worktree.path().to_path_buf();
        let store = store_in(&temp);

        store
            .fence_worktree(&worktree_path, "stale worktree")
            .expect("fence worktree");
        drop(worktree);
        let state = store.load().expect("reload fences");

        assert!(state.is_fenced(&worktree_path));
        assert!(
            store
                .unblock_worktree(&worktree_path)
                .expect("unblock deleted worktree")
                .is_some()
        );
    }

    #[cfg(unix)]
    #[test]
    fn deleted_symlink_worktree_can_still_be_queried_and_unblocked() {
        let temp = tempfile::tempdir().expect("tempdir");
        let target = temp.path().join("target-worktree");
        let link = temp.path().join("linked-worktree");
        fs::create_dir(&target).expect("create target worktree");
        symlink(&target, &link).expect("create worktree symlink");
        let store = store_in(&temp);

        store
            .fence_worktree(&link, "symlinked worktree")
            .expect("fence symlink worktree");
        fs::remove_dir(&target).expect("remove target worktree");
        fs::remove_file(&link).expect("remove worktree symlink");
        let state = store.load().expect("reload fences");

        assert!(state.is_fenced(&link));
        assert!(
            store
                .unblock_worktree(&link)
                .expect("unblock deleted symlink worktree")
                .is_some()
        );
    }

    #[test]
    fn refencing_existing_worktree_replaces_store_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let store = store_in(&temp);

        store
            .fence_worktree(worktree.path(), "first")
            .expect("first fence");
        store
            .fence_worktree(worktree.path(), "second")
            .expect("replace fence");
        let reloaded = store.load().expect("reload fences");

        assert_eq!(reloaded.active_fences().len(), 1);
        assert_eq!(reloaded.active_fences()[0].reason, "second");
    }

    #[cfg(unix)]
    #[test]
    fn store_parent_symlink_is_rejected() {
        let temp = tempfile::tempdir().expect("tempdir");
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::create_dir(&target).expect("create target");
        symlink(&target, &link).expect("create symlink");
        let worktree = make_worktree();
        let store = FenceStore::at_path(link.join("intercept-fences.json"));

        let err = store
            .fence_worktree(worktree.path(), "symlink parent")
            .expect_err("symlink parent should be rejected");

        assert!(matches!(
            err,
            FenceStoreError::Write { .. } | FenceStoreError::InsecureStoreParent { .. }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn store_parent_with_group_write_permission_is_rejected() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store_dir = temp.path().join("state");
        fs::create_dir(&store_dir).expect("create state dir");
        fs::set_permissions(&store_dir, fs::Permissions::from_mode(0o770))
            .expect("make state dir group-writable");
        let store = FenceStore::at_path(store_dir.join("intercept-fences.json"));

        let err = store
            .load()
            .expect_err("group-writable parent should be rejected");

        assert!(matches!(err, FenceStoreError::InsecureStoreParent { .. }));
    }

    #[test]
    fn default_path_uses_xdg_state_home_before_home() {
        let path = default_fence_state_path_from_env(|name| match name {
            "XDG_STATE_HOME" => Some(OsString::from("/state")),
            "HOME" => Some(OsString::from("/home/anvil")),
            _ => None,
        })
        .expect("default path");

        assert_eq!(path, PathBuf::from("/state/anvil/intercept-fences.json"));
    }

    /// MLP2-025b: `fence_worktree_for_spoof` records a fence whose
    /// reason is exactly `DEGRADED_SPOOFED_ATTRIBUTION`. Pins the
    /// reason-string contract that the spec §5.3 / §8 depend on.
    #[test]
    fn fence_worktree_for_spoof_records_degraded_reason() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        let worktree = tempfile::tempdir().expect("worktree tempdir");

        let record = store
            .fence_worktree_for_spoof(worktree.path())
            .expect("spoof fence");

        assert_eq!(
            record.reason,
            crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION
        );
        assert_eq!(record.reason, "degraded:spoofed-attribution");

        let reloaded = store.load().expect("reload");
        assert!(reloaded.is_fenced(worktree.path()));
        assert_eq!(
            reloaded.active_fences()[0].reason,
            crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION,
        );
    }

    // ---- MLP2-026: cascade engaged-state ----------------------------

    /// MLP2-026: a pre-MLP2-026 fence file (no `cascades` key)
    /// deserialises with `cascades = vec![]`. Pins the
    /// wire-additive guard. Use `store.save` to bootstrap the
    /// state-dir permissions, then overwrite the file with the
    /// legacy wire shape and `load` it back.
    #[test]
    fn fence_file_without_cascades_key_loads_with_empty_cascades() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        // Bootstrap the parent dir with correct perms via the
        // store's own save path.
        store.save(&FenceState::default()).expect("bootstrap save");
        // Now overwrite with the legacy wire shape.
        std::fs::write(&store.path, br#"{"version":1,"fences":[]}"#).unwrap();
        let state = store.load().expect("legacy fence file loads");
        assert!(
            state.active_cascades().is_empty(),
            "missing cascades key defaults to vec![]"
        );
    }

    /// MLP2-026: `cascades` is omitted from the wire when empty,
    /// preserving the pre-MLP2-026 wire shape exactly.
    #[test]
    fn save_omits_cascades_when_empty() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        let worktree = tempfile::tempdir().expect("worktree tempdir");

        // Record a regular fence so the file gets persisted with
        // a populated `fences` list but no cascades.
        store
            .fence_worktree(worktree.path(), "rule violation")
            .expect("fence");

        let raw = std::fs::read_to_string(&store.path).expect("read file");
        assert!(
            !raw.contains("\"cascades\""),
            "cascades omitted on wire when empty: {raw}"
        );
    }

    /// MLP2-026: a `CascadeRecord` round-trips through `save` →
    /// `load`. Pins the persisted shape that the spec §6 inv-4
    /// (restart preserves engage flag) depends on.
    #[test]
    fn cascade_record_round_trips_through_store_reload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let canonical = worktree.path().canonicalize().expect("canonicalise");

        // Manually inject a cascade record via the store's
        // internal save (the public engage path lands in F2; this
        // test only pins persistence, not the engage trigger).
        let mut state = store.load().expect("initial load");
        state.upsert_cascade(CascadeRecord {
            worktree: canonical.clone(),
            since_unix: 1_700_000_500,
            reason: "degraded:fence-cascade".to_string(),
        });
        store.save(&state).expect("save");

        let reloaded = store.load().expect("reload");
        assert!(reloaded.is_cascaded(worktree.path()));
        assert_eq!(reloaded.active_cascades().len(), 1);
        assert_eq!(reloaded.active_cascades()[0].worktree, canonical);
        assert_eq!(reloaded.active_cascades()[0].since_unix, 1_700_000_500);
        assert_eq!(
            reloaded.active_cascades()[0].reason,
            "degraded:fence-cascade"
        );
    }

    /// MLP2-026: `is_cascaded` returns true iff a `CascadeRecord`
    /// exists for the canonical form of the supplied worktree.
    /// Snapshot semantics: reads disk, swallows I/O failures
    /// (returns false on load error). Spec §5.2.
    #[test]
    fn is_cascaded_returns_true_after_cascade_record_persisted() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let canonical = worktree.path().canonicalize().expect("canonicalise");

        assert!(
            !store.is_cascaded(worktree.path()),
            "is_cascaded false initially"
        );

        let mut state = store.load().expect("load");
        state.upsert_cascade(CascadeRecord {
            worktree: canonical.clone(),
            since_unix: 1_700_000_500,
            reason: "degraded:fence-cascade".to_string(),
        });
        store.save(&state).expect("save");

        assert!(
            store.is_cascaded(worktree.path()),
            "is_cascaded true after persist"
        );
    }

    /// MLP2-026: `clear_cascade` removes the record AND resets
    /// the in-memory `RateWindow`. Spec §5.3 + §6 inv-3.
    #[test]
    fn clear_cascade_is_idempotent_and_resets_window() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let canonical = worktree.path().canonicalize().expect("canonicalise");

        let mut state = store.load().expect("load");
        state.upsert_cascade(CascadeRecord {
            worktree: canonical.clone(),
            since_unix: 1_700_000_500,
            reason: "degraded:fence-cascade".to_string(),
        });
        store.save(&state).expect("save");

        // First clear: removed=true.
        let cleared = store.clear_cascade(worktree.path()).expect("clear");
        assert!(cleared, "first clear removes the record");
        assert!(!store.is_cascaded(worktree.path()));

        // Second clear (idempotent): removed=false, no error.
        let cleared = store.clear_cascade(worktree.path()).expect("clear again");
        assert!(!cleared, "idempotent clear returns false");
        assert!(!store.is_cascaded(worktree.path()));
    }

    /// MLP2-026: cascade-rate-window constants match the spec
    /// §3.1 contract (capacity 4, window 60s — i.e. 5 fires in 60s
    /// trigger throttle/engage).
    #[test]
    fn cascade_rate_window_constants_match_spec() {
        assert_eq!(CASCADE_RATE_WINDOW_CAPACITY, 4);
        assert_eq!(CASCADE_RATE_WINDOW_DURATION, Duration::from_mins(1));
    }

    /// MLP2-026: four fence fires within 60 s do NOT engage the
    /// cascade — the rate window admits up to `capacity` events
    /// before throttling. Spec §3.1 + §4.1.
    #[test]
    fn four_fences_in_sixty_seconds_do_not_engage_cascade() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        for i in 0..4 {
            store
                .fence_worktree(worktree.path(), format!("fire {i}"))
                .expect("fence fires admitted");
        }
        let state = store.load().expect("reload");
        assert!(
            !state.is_cascaded(worktree.path()),
            "4 fires within 60s must NOT engage cascade; active_cascades={:?}",
            state.active_cascades()
        );
    }

    /// MLP2-026: the FIFTH fence fire within 60 s engages the
    /// cascade. The 5th `record()` call returns
    /// `RateDecision::Throttle` (capacity=4 admits 4 events,
    /// throttles the 5th). Spec §3.1 + §4.1.
    #[test]
    fn five_fences_in_sixty_seconds_engage_cascade() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        for i in 0..5 {
            store
                .fence_worktree(worktree.path(), format!("fire {i}"))
                .expect("fence");
        }
        let state = store.load().expect("reload");
        assert!(
            state.is_cascaded(worktree.path()),
            "5 fires within 60s MUST engage cascade"
        );
        assert_eq!(state.active_cascades().len(), 1);
        assert_eq!(
            state.active_cascades()[0].reason,
            crate::telemetry::DEGRADED_FENCE_CASCADE
        );
    }

    /// MLP2-026: subsequent fires on an already-cascaded worktree
    /// do NOT re-engage / overwrite the cascade record. Spec §10
    /// Q3: emit-once per cascade, not per excess fire. The
    /// `since_unix` stays at the original engage timestamp.
    #[test]
    fn additional_fires_after_engage_do_not_overwrite_cascade_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        let worktree = tempfile::tempdir().expect("worktree tempdir");

        for i in 0..5 {
            store
                .fence_worktree(worktree.path(), format!("fire {i}"))
                .expect("fence");
        }
        let after_engage = store.load().expect("reload");
        let first_since_unix = after_engage.active_cascades()[0].since_unix;

        // Fire two more times. Even if the rate-window slides on, the
        // existing cascade record must be preserved unchanged.
        std::thread::sleep(Duration::from_millis(20));
        for i in 5..7 {
            store
                .fence_worktree(worktree.path(), format!("fire {i}"))
                .expect("fence");
        }
        let state = store.load().expect("reload");
        assert_eq!(state.active_cascades().len(), 1);
        assert_eq!(
            state.active_cascades()[0].since_unix,
            first_since_unix,
            "cascade since_unix must not be overwritten by subsequent fires",
        );
    }

    /// MLP2-026: cascade engaged-state survives a daemon restart.
    /// Simulate by dropping and recreating the `FenceStore` (the
    /// in-memory rate window is rebuilt empty; the disk-persisted
    /// cascade record is loaded). Spec §6 inv-4.
    #[test]
    fn cascade_record_survives_simulated_daemon_restart() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_in(&temp);
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        for i in 0..5 {
            store
                .fence_worktree(worktree.path(), format!("fire {i}"))
                .expect("fence");
        }
        assert!(store.is_cascaded(worktree.path()));

        // Drop the store; create a new one at the same path
        // (the in-memory window is freshly empty).
        drop(store);
        let store_after_restart =
            FenceStore::at_path(temp.path().join("state/intercept-fences.json"));
        assert!(
            store_after_restart.is_cascaded(worktree.path()),
            "cascade record must survive daemon restart",
        );
    }

    // ----- DPO-002: fence constraint_applied emission -----

    /// DPO-002: a single (non-cascading) fence engage produces exactly one
    /// `constraint_applied` row — the critical council producer-coverage fix.
    /// The reason normalises to the bounded token and the cascade flag is
    /// `false`.
    #[test]
    fn single_fence_engage_emits_exactly_one_constraint_row() {
        use crate::kindling_observation::{
            FENCE_GATE_ID, FENCE_REASON_OPERATOR, KindlingObservationSink,
            RecordingKindlingObservationSink,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let store = store_in(&temp).with_observation_sink(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            "daemon-session-fence".to_string(),
            true,
        );

        store
            .fence_worktree(worktree.path(), "rule violation")
            .expect("fence worktree");

        let rows = recorder.recorded_constraints();
        assert_eq!(rows.len(), 1, "one engage must produce exactly one row");
        let row = &rows[0];
        assert_eq!(row.constraint_id, FENCE_GATE_ID);
        assert_eq!(row.gate_id, FENCE_GATE_ID);
        assert_eq!(row.session_id, "daemon-session-fence");
        assert_eq!(
            row.reason, FENCE_REASON_OPERATOR,
            "free-form reason must normalise to the bounded token",
        );
        assert!(!row.cascade, "an ordinary engage is not a cascade");
        assert_ne!(
            row.worktree, "<redacted>",
            "include_paths=true keeps the real worktree path on the row",
        );
    }

    /// DPO-002 (council C): with `include_paths=false` the engage row's
    /// worktree is redacted to the schema-stable placeholder; with
    /// `include_paths=true` the real canonical path is present. Pins the
    /// path-suppression gate that mirrors the save-time `gate_evaluated`
    /// `ANVIL_OBSERVATION_INCLUDE_PATHS` posture.
    #[test]
    fn fence_engage_redacts_worktree_when_include_paths_false() {
        use crate::kindling_observation::{
            KindlingObservationSink, RecordingKindlingObservationSink,
        };

        // include_paths = false → worktree redacted.
        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let store = store_in(&temp).with_observation_sink(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            "daemon-session-fence".to_string(),
            false,
        );
        store
            .fence_worktree(worktree.path(), "rule violation")
            .expect("fence worktree");
        let rows = recorder.recorded_constraints();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].worktree, "<redacted>",
            "include_paths=false must redact the worktree path",
        );

        // include_paths = true → worktree present (the canonical path).
        let temp_on = tempfile::tempdir().expect("tempdir");
        let worktree_on = make_worktree();
        let canonical = worktree_on.path().canonicalize().expect("canonicalise");
        let recorder_on = Arc::new(RecordingKindlingObservationSink::new());
        let store_on = store_in(&temp_on).with_observation_sink(
            Arc::clone(&recorder_on) as Arc<dyn KindlingObservationSink>,
            "daemon-session-fence".to_string(),
            true,
        );
        store_on
            .fence_worktree(worktree_on.path(), "rule violation")
            .expect("fence worktree");
        let rows_on = recorder_on.recorded_constraints();
        assert_eq!(rows_on.len(), 1);
        assert_eq!(
            rows_on[0].worktree,
            canonical.display().to_string(),
            "include_paths=true must keep the real worktree path",
        );
    }

    /// DPO-002: a cascade-engaging fire (the 5th within 60s) emits a row with
    /// `cascade = true` and the pinned cascade reason. Each engage in the run
    /// produces exactly one row (emit-once per engage), so five fires yield
    /// five rows with exactly one cascade row.
    #[test]
    fn cascade_engage_sets_cascade_flag_true() {
        use crate::kindling_observation::{
            KindlingObservationSink, RecordingKindlingObservationSink,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let store = store_in(&temp).with_observation_sink(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            "daemon-session-fence".to_string(),
            true,
        );

        for i in 0..5 {
            store
                .fence_worktree(worktree.path(), format!("fire {i}"))
                .expect("fence");
        }

        let rows = recorder.recorded_constraints();
        assert_eq!(rows.len(), 5, "one row per engage");
        let cascade_rows: Vec<_> = rows.iter().filter(|r| r.cascade).collect();
        assert_eq!(
            cascade_rows.len(),
            1,
            "exactly the cascade-engaging fire sets cascade=true",
        );
        assert_eq!(
            cascade_rows[0].reason,
            crate::telemetry::DEGRADED_FENCE_CASCADE,
        );
    }

    /// DPO-002: a sink error on the constraint emit is logged + swallowed — the
    /// fence is still persisted (the engage succeeds regardless of sink health).
    #[test]
    fn fence_engage_survives_constraint_sink_error() {
        use crate::kindling_observation::{
            KindlingObservationSink, KindlingSinkError, RecordingKindlingObservationSink,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let worktree = make_worktree();
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        recorder.fail_next_constraint_with(KindlingSinkError::Unavailable("db locked".into()));
        let store = store_in(&temp).with_observation_sink(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            "daemon-session-fence".to_string(),
            true,
        );

        let record = store
            .fence_worktree(worktree.path(), "rule violation")
            .expect("fence still persists despite sink error");
        assert!(store.load().expect("reload").is_fenced(&record.worktree));
        assert!(
            recorder.recorded_constraints().is_empty(),
            "failed emit must not record the row",
        );
    }
}
