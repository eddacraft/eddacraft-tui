use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

const FENCE_FILE_VERSION: u8 = 1;

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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FenceState {
    records: Vec<FenceRecord>,
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
}

#[derive(Debug, Clone)]
pub struct FenceStore {
    path: PathBuf,
}

impl FenceStore {
    #[must_use]
    pub fn at_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<FenceState, FenceStoreError> {
        #[cfg(unix)]
        validate_store_parent(&self.path)?;
        #[cfg(windows)]
        recover_windows_backup(&self.path)?;
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(FenceState::default());
            }
            Err(source) => {
                return Err(FenceStoreError::Read {
                    path: self.path.clone(),
                    source,
                });
            }
        };
        let file: FenceFile =
            serde_json::from_str(&content).map_err(|source| FenceStoreError::Parse {
                path: self.path.clone(),
                source,
            })?;
        if file.version != FENCE_FILE_VERSION {
            return Err(FenceStoreError::UnsupportedVersion {
                path: self.path.clone(),
                version: file.version,
            });
        }
        let mut state = FenceState {
            records: validate_records(&self.path, file.fences)?,
        };
        state.records.sort_by(|a, b| a.worktree.cmp(&b.worktree));
        Ok(state)
    }

    pub fn fence_worktree(
        &self,
        worktree: &Path,
        reason: impl Into<String>,
    ) -> Result<FenceRecord, FenceStoreError> {
        let canonical = canonicalise_worktree(worktree)?;
        let aliases = original_worktree_alias(worktree, &canonical)?;
        let record = FenceRecord {
            worktree: canonical,
            aliases,
            reason: reason.into(),
            fenced_at_unix: unix_seconds_now(),
        };
        let mut state = self.load()?;
        state.upsert(record.clone());
        self.save(&state)?;
        Ok(record)
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
        }
        Ok(removed)
    }

    fn save(&self, state: &FenceState) -> Result<(), FenceStoreError> {
        ensure_store_parent(&self.path)?;
        let file = FenceFile {
            version: FENCE_FILE_VERSION,
            fences: state.records.clone(),
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
        Ok(())
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
    use std::time::{Duration, Instant};

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    use anvil_intercept_proto::SessionId;
    use tempfile::TempDir;

    use crate::registry::SessionRegistry;

    use super::*;

    fn make_worktree() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn store_in(temp: &TempDir) -> FenceStore {
        FenceStore::at_path(temp.path().join("state/intercept-fences.json"))
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
}
