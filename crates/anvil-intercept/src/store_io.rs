//! Shared atomic-store filesystem primitives for the daemon's persisted
//! state files under `ANVIL_HOME`.
//!
//! ACTMO-014 factored these out of `fence.rs` so the durable
//! registration store ([`crate::registration_store`]) and the fence store
//! ([`crate::fence`]) share one implementation of the security-sensitive
//! parts — owner-only parent directories, `O_NOFOLLOW`-equivalent symlink
//! refusal, `0600` temp files, and the temp-then-rename atomic replace.
//! Duplicating that code across two stores would be a drift hazard, so both
//! map [`StoreIoError`] into their own error type via `From`.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

/// Neutral IO error for the shared store primitives. Each concrete store maps
/// these into its own error type so the store's public surface keeps its own
/// vocabulary.
#[derive(Debug)]
pub enum StoreIoError {
    /// A write / rename / metadata operation failed.
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The store's parent directory failed an ownership / mode / symlink
    /// check and writing would be insecure.
    InsecureStoreParent { path: PathBuf, reason: String },
}

impl std::fmt::Display for StoreIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Write { path, source } => {
                write!(f, "failed to write store {}: {source}", path.display())
            }
            Self::InsecureStoreParent { path, reason } => {
                write!(f, "insecure store parent {}: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for StoreIoError {}

/// A unique sibling temp path for the atomic temp-then-rename write. Encodes
/// the pid and a nanosecond stamp so concurrent writers never collide.
#[must_use]
pub fn temporary_store_path(path: &Path) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    path.with_extension(format!("json.tmp.{}.{unique}", std::process::id()))
}

/// Create a fresh `0600` store file, failing if it already exists.
pub fn create_store_file(path: &Path) -> Result<File, StoreIoError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    options.open(path).map_err(|source| StoreIoError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Atomically replace `target` with `tmp`. On Windows this routes through a
/// `.bak` backup so a crash mid-rename is recoverable via
/// [`recover_windows_backup`].
pub fn replace_store_file(tmp: &Path, target: &Path) -> Result<(), StoreIoError> {
    #[cfg(windows)]
    {
        let backup = windows_backup_path(target);
        if backup.exists() {
            fs::remove_file(&backup).map_err(|source| StoreIoError::Write {
                path: backup.clone(),
                source,
            })?;
        }
        match fs::rename(target, &backup) {
            Ok(()) => {}
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(StoreIoError::Write {
                    path: target.to_path_buf(),
                    source,
                });
            }
        }

        if let Err(source) = fs::rename(tmp, target) {
            if backup.exists() {
                let _ = fs::rename(&backup, target);
            }
            return Err(StoreIoError::Write {
                path: target.to_path_buf(),
                source,
            });
        }
        if backup.exists() {
            fs::remove_file(&backup).map_err(|source| StoreIoError::Write {
                path: backup,
                source,
            })?;
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        fs::rename(tmp, target).map_err(|source| StoreIoError::Write {
            path: target.to_path_buf(),
            source,
        })
    }
}

/// Windows crash recovery: if `target` is missing but its `.bak` exists (a
/// crash landed between the two renames in [`replace_store_file`]), restore it.
#[cfg(windows)]
pub fn recover_windows_backup(target: &Path) -> Result<(), StoreIoError> {
    let backup = windows_backup_path(target);
    if !target.exists() && backup.exists() {
        fs::rename(&backup, target).map_err(|source| StoreIoError::Write {
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

/// fsync the parent directory so the rename is durable across a power loss.
#[cfg(unix)]
pub fn sync_parent(path: &Path) -> Result<(), StoreIoError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|source| StoreIoError::Write {
            path: parent.to_path_buf(),
            source,
        })?;

    Ok(())
}

/// Ensure the store's parent exists as an owner-only (`0700`) real directory,
/// creating it if needed.
pub fn ensure_store_parent(path: &Path) -> Result<(), StoreIoError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    #[cfg(unix)]
    {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder
            .create(parent)
            .map_err(|source| StoreIoError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        validate_existing_store_parent(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|source| {
            StoreIoError::Write {
                path: parent.to_path_buf(),
                source,
            }
        })?;
    }

    #[cfg(not(unix))]
    fs::create_dir_all(parent).map_err(|source| StoreIoError::Write {
        path: parent.to_path_buf(),
        source,
    })?;

    Ok(())
}

/// Refuse to read a store whose parent is a symlink, not owned by the current
/// user, or group/other-accessible.
#[cfg(unix)]
pub fn validate_store_parent(path: &Path) -> Result<(), StoreIoError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    if parent.exists() {
        validate_existing_store_parent(parent)?;
    }

    Ok(())
}

#[cfg(unix)]
fn validate_existing_store_parent(parent: &Path) -> Result<(), StoreIoError> {
    let metadata = fs::symlink_metadata(parent).map_err(|source| StoreIoError::Write {
        path: parent.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(StoreIoError::InsecureStoreParent {
            path: parent.to_path_buf(),
            reason: "parent must be a real directory, not a symlink".to_string(),
        });
    }
    if metadata.uid() != nix::unistd::geteuid().as_raw() {
        return Err(StoreIoError::InsecureStoreParent {
            path: parent.to_path_buf(),
            reason: "parent must be owned by the current user".to_string(),
        });
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(StoreIoError::InsecureStoreParent {
            path: parent.to_path_buf(),
            reason: "parent must be private to the current user".to_string(),
        });
    }
    Ok(())
}

/// Wall-clock Unix seconds, saturating to 0 before the epoch.
#[must_use]
pub fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
