//! ACTMO-014: durable worktree-registration store.
//!
//! Persists the activation-spine **durable membership set** under
//! `ANVIL_HOME`, alongside the fence store, so an `anvil start` /
//! `anvil workspace register` registration survives the registering process
//! exiting, the 30 s heartbeat TTL, and a daemon restart (ADR-094 decision 1).
//! On daemon startup `run_foreground` reloads this set into the registry
//! before accepting connections — exactly analogous to fence reload — and a
//! reaper drops entries whose worktree directory is gone.
//!
//! Mirrors [`crate::fence::FenceStore`] and shares the security-sensitive
//! atomic-write + owner-only-parent primitives in [`crate::store_io`]. The
//! on-disk file is `registered-worktrees.json`; an absent file loads as an
//! empty set, so a first run and a clean uninstall are both benign.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anvil_intercept_proto::SessionId;
use anvil_intercept_proto::session::AgentTag;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::store_io::{
    StoreIoError, create_store_file, ensure_store_parent, replace_store_file, temporary_store_path,
    unix_seconds_now,
};

#[cfg(windows)]
use crate::store_io::recover_windows_backup;
#[cfg(unix)]
use crate::store_io::{sync_parent, validate_store_parent};

const REGISTRATION_FILE_VERSION: u8 = 1;

/// Errors from the durable registration store. Mirrors
/// [`crate::fence::FenceStoreError`]'s shape; the shared IO failures map in
/// from [`StoreIoError`].
#[derive(Debug, Error)]
pub enum RegistrationStoreError {
    #[error("failed to read registration store {path:?}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write registration store {path:?}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse registration store {path:?}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("unsupported registration store version {version} in {path:?}")]
    UnsupportedVersion { path: PathBuf, version: u8 },

    #[error("invalid registration record in {path:?}: {reason}")]
    InvalidRecord { path: PathBuf, reason: String },

    #[error("insecure registration store parent {path:?}: {reason}")]
    InsecureStoreParent { path: PathBuf, reason: String },
}

impl From<StoreIoError> for RegistrationStoreError {
    fn from(error: StoreIoError) -> Self {
        match error {
            StoreIoError::Write { path, source } => Self::Write { path, source },
            StoreIoError::InsecureStoreParent { path, reason } => {
                Self::InsecureStoreParent { path, reason }
            }
        }
    }
}

/// One persisted durable registration. Carries exactly what the daemon needs
/// to re-seed the registry on reload: the deterministic activation
/// [`SessionId`], the canonical worktree path, and the activation
/// [`AgentTag`] (so the reloaded session is durable again, not a live lease).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationRecord {
    pub session_id: SessionId,
    pub worktree: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_tag: Option<AgentTag>,
    pub registered_at_unix: u64,
}

impl RegistrationRecord {
    /// Build a record stamping the registration time as now. The `worktree`
    /// is expected to already be canonical (the caller canonicalises before
    /// registering with the registry).
    #[must_use]
    pub fn new(session_id: SessionId, worktree: PathBuf, agent_tag: Option<AgentTag>) -> Self {
        Self {
            session_id,
            worktree,
            agent_tag,
            registered_at_unix: unix_seconds_now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RegistrationFile {
    version: u8,
    registrations: Vec<RegistrationRecord>,
}

/// The durable registration store. Cheap to clone (`path` only); every method
/// reads or writes the file, so there is no in-process cache to keep coherent
/// across clones — the registry is the live source of truth, this is its
/// persistent shadow.
///
/// **Concurrency (review F1).** `upsert` / `remove` / `replace_all` are
/// read-modify-write (`load` → mutate → `save`). `save` is itself atomic
/// (temp-then-rename), so the file is never observed half-written, but two
/// *concurrent* read-modify-write cycles could lose an update. A process-local
/// `write_lock` serialises the whole `load`→`save` cycle so the store is
/// correct even if a future change moves the daemon to a multi-thread runtime
/// or pushes persistence onto `spawn_blocking`. The lock is `Arc`-shared so
/// `Clone`s of the store (and the `Arc<RegistrationStore>` the daemon shares
/// across IPC tasks) contend on one lock. It guards only this process; the
/// on-disk atomic rename is what protects against a torn file.
#[derive(Clone)]
pub struct RegistrationStore {
    path: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

impl RegistrationStore {
    #[must_use]
    pub fn at_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            write_lock: Arc::new(Mutex::new(())),
        }
    }

    /// The file path the store reads and writes.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load the persisted durable registrations. A missing file is an empty
    /// set (first run / clean uninstall), not an error. Records are returned
    /// sorted by worktree for deterministic startup logging.
    pub fn load(&self) -> Result<Vec<RegistrationRecord>, RegistrationStoreError> {
        #[cfg(unix)]
        validate_store_parent(&self.path)?;
        #[cfg(windows)]
        recover_windows_backup(&self.path)?;

        let content = match std::fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(source) => {
                return Err(RegistrationStoreError::Read {
                    path: self.path.clone(),
                    source,
                });
            }
        };
        let file: RegistrationFile =
            serde_json::from_str(&content).map_err(|source| RegistrationStoreError::Parse {
                path: self.path.clone(),
                source,
            })?;
        if file.version != REGISTRATION_FILE_VERSION {
            return Err(RegistrationStoreError::UnsupportedVersion {
                path: self.path.clone(),
                version: file.version,
            });
        }
        let mut records = self.validate(file.registrations)?;
        records.sort_by(|a, b| a.worktree.cmp(&b.worktree));
        Ok(records)
    }

    /// Add or replace the durable registration for a worktree, keyed on the
    /// canonical worktree path (idempotent re-register refreshes in place).
    pub fn upsert(&self, record: RegistrationRecord) -> Result<(), RegistrationStoreError> {
        let _guard = self.lock();
        let mut records = self.load()?;
        if let Some(existing) = records
            .iter_mut()
            .find(|existing| existing.worktree == record.worktree)
        {
            *existing = record;
        } else {
            records.push(record);
        }
        self.save(&records)
    }

    /// Remove the durable registration for a canonical worktree path.
    /// Returns `true` when a record existed and was removed (idempotent).
    pub fn remove(&self, worktree: &Path) -> Result<bool, RegistrationStoreError> {
        let _guard = self.lock();
        let mut records = self.load()?;
        let before = records.len();
        records.retain(|record| record.worktree != worktree);
        let removed = records.len() != before;
        if removed {
            self.save(&records)?;
        }
        Ok(removed)
    }

    /// Serialise the read-modify-write cycle (review F1). Held across `load`→
    /// `save` so two writers cannot lose an update.
    fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.write_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Overwrite the persisted set wholesale. Used by the startup reaper to
    /// prune entries whose directory is gone in a single atomic write.
    pub fn replace_all(
        &self,
        records: &[RegistrationRecord],
    ) -> Result<(), RegistrationStoreError> {
        let _guard = self.lock();
        self.save(records)
    }

    fn save(&self, records: &[RegistrationRecord]) -> Result<(), RegistrationStoreError> {
        ensure_store_parent(&self.path)?;
        let mut sorted = records.to_vec();
        sorted.sort_by(|a, b| a.worktree.cmp(&b.worktree));
        let file = RegistrationFile {
            version: REGISTRATION_FILE_VERSION,
            registrations: sorted,
        };
        let mut content =
            serde_json::to_vec_pretty(&file).map_err(|source| RegistrationStoreError::Write {
                path: self.path.clone(),
                source: std::io::Error::other(source),
            })?;
        content.push(b'\n');

        let tmp = temporary_store_path(&self.path);
        let mut handle = create_store_file(&tmp)?;
        std::io::Write::write_all(&mut handle, &content)
            .and_then(|()| handle.sync_all())
            .map_err(|source| RegistrationStoreError::Write {
                path: tmp.clone(),
                source,
            })?;
        drop(handle);
        replace_store_file(&tmp, &self.path)?;
        #[cfg(unix)]
        sync_parent(&self.path)?;
        Ok(())
    }

    fn validate(
        &self,
        records: Vec<RegistrationRecord>,
    ) -> Result<Vec<RegistrationRecord>, RegistrationStoreError> {
        for record in &records {
            if !record.worktree.is_absolute() {
                return Err(RegistrationStoreError::InvalidRecord {
                    path: self.path.clone(),
                    reason: format!(
                        "registered worktree is not absolute: {}",
                        record.worktree.display(),
                    ),
                });
            }
        }
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spine_tag() -> AgentTag {
        AgentTag::new(
            "anvil-start",
            anvil_intercept_proto::session::ACTIVATION_SPINE_CLAIMED_AGENT_ID,
            0,
        )
    }

    fn record(worktree: &Path) -> RegistrationRecord {
        RegistrationRecord::new(
            SessionId::new("sess_activation_test"),
            worktree.to_path_buf(),
            Some(spine_tag()),
        )
    }

    /// Nest the store one level under the tempdir, mirroring production where
    /// the file lives under `$ANVIL_HOME` — `ensure_store_parent` creates that
    /// parent at `0700`, which the owner-only-parent check then accepts. (The
    /// raw tempdir inherits the test box's umask and would be rejected.)
    fn store_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("state/registered-worktrees.json")
    }

    /// Create the store's parent dir at `0700` so a hand-written fixture file
    /// passes the owner-only-parent check on load.
    fn seed_secure_parent(path: &Path) {
        let parent = path.parent().expect("parent");
        std::fs::create_dir_all(parent).expect("create parent");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
                .expect("chmod parent");
        }
    }

    #[test]
    fn missing_file_loads_as_empty_set() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RegistrationStore::at_path(store_path(&dir));
        assert!(store.load().expect("load missing").is_empty());
    }

    #[test]
    fn upsert_then_load_round_trips_and_survives_a_fresh_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = store_path(&dir);
        let worktree = PathBuf::from("/abs/worktree-a");
        RegistrationStore::at_path(&path)
            .upsert(record(&worktree))
            .expect("upsert");

        // A fresh store at the same path (simulating a daemon restart) reads
        // the persisted set back.
        let reloaded = RegistrationStore::at_path(&path).load().expect("reload");
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].worktree, worktree);
        assert_eq!(reloaded[0].agent_tag, Some(spine_tag()));
    }

    #[test]
    fn upsert_is_idempotent_on_worktree_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = store_path(&dir);
        let store = RegistrationStore::at_path(&path);
        let worktree = PathBuf::from("/abs/worktree-a");
        store.upsert(record(&worktree)).expect("first");
        store.upsert(record(&worktree)).expect("second");
        assert_eq!(store.load().expect("load").len(), 1);
    }

    #[test]
    fn remove_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = store_path(&dir);
        let store = RegistrationStore::at_path(&path);
        let worktree = PathBuf::from("/abs/worktree-a");
        store.upsert(record(&worktree)).expect("upsert");

        assert!(store.remove(&worktree).expect("first remove"));
        assert!(!store.remove(&worktree).expect("second remove is a no-op"));
        assert!(store.load().expect("load").is_empty());
    }

    #[test]
    fn replace_all_prunes_to_the_supplied_set() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = store_path(&dir);
        let store = RegistrationStore::at_path(&path);
        store.upsert(record(Path::new("/abs/a"))).expect("a");
        store.upsert(record(Path::new("/abs/b"))).expect("b");

        store
            .replace_all(&[record(Path::new("/abs/a"))])
            .expect("prune to just a");
        let loaded = store.load().expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].worktree, PathBuf::from("/abs/a"));
    }

    #[test]
    fn non_absolute_worktree_record_is_rejected_on_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = store_path(&dir);
        // Hand-write a file with a relative worktree path.
        let body = serde_json::json!({
            "version": REGISTRATION_FILE_VERSION,
            "registrations": [{
                "session_id": "sess_activation_test",
                "worktree": "relative/worktree",
                "registered_at_unix": 1,
            }],
        });
        seed_secure_parent(&path);
        std::fs::write(&path, serde_json::to_vec_pretty(&body).unwrap()).expect("write");
        let err = RegistrationStore::at_path(&path)
            .load()
            .expect_err("relative path must be rejected");
        assert!(matches!(err, RegistrationStoreError::InvalidRecord { .. }));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = store_path(&dir);
        let body = serde_json::json!({ "version": 99, "registrations": [] });
        seed_secure_parent(&path);
        std::fs::write(&path, serde_json::to_vec(&body).unwrap()).expect("write");
        let err = RegistrationStore::at_path(&path)
            .load()
            .expect_err("bad version must be rejected");
        assert!(matches!(
            err,
            RegistrationStoreError::UnsupportedVersion { version: 99, .. }
        ));
    }
}
