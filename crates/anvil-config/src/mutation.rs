//! Shared project-config mutation coordination (PSCAF-001 / ADR-143).
//!
//! Linked worktrees coordinate through one lock path under Git's common
//! directory. Non-Git directories deliberately have no automatic
//! existing-file mutation authority.

use std::path::{Path, PathBuf};

use thiserror::Error;

const LOCK_RELATIVE_PATH: &str = "anvil/config-mutation.lock";

#[derive(Debug, Error)]
pub enum MutationLockError {
    #[error("project has no trustworthy Git common directory")]
    MissingGitCommonDir,
    #[error("cannot inspect Git metadata at {path}: {source}")]
    Inspect {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid Git metadata at {0}")]
    InvalidGitMetadata(PathBuf),
}

/// Resolve the common lock path shared by every linked worktree.
pub fn mutation_lock_path(repo_root: &Path) -> Result<PathBuf, MutationLockError> {
    let dot_git = repo_root.join(".git");
    let metadata = std::fs::symlink_metadata(&dot_git).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            MutationLockError::MissingGitCommonDir
        } else {
            MutationLockError::Inspect {
                path: dot_git.clone(),
                source,
            }
        }
    })?;
    if metadata.file_type().is_symlink() {
        return Err(MutationLockError::InvalidGitMetadata(dot_git));
    }

    let git_dir = if metadata.is_dir() {
        canonicalise(&dot_git)?
    } else if metadata.is_file() {
        let raw =
            std::fs::read_to_string(&dot_git).map_err(|source| MutationLockError::Inspect {
                path: dot_git.clone(),
                source,
            })?;
        let raw_path = raw
            .lines()
            .find_map(|line| line.trim().strip_prefix("gitdir:").map(str::trim))
            .filter(|path| !path.is_empty())
            .ok_or_else(|| MutationLockError::InvalidGitMetadata(dot_git.clone()))?;
        let path = Path::new(raw_path);
        canonicalise(&if path.is_absolute() {
            path.to_path_buf()
        } else {
            repo_root.join(path)
        })?
    } else {
        return Err(MutationLockError::InvalidGitMetadata(dot_git));
    };

    let common_marker = git_dir.join("commondir");
    let common_dir = match std::fs::symlink_metadata(&common_marker) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
            return Err(MutationLockError::InvalidGitMetadata(common_marker));
        }
        Ok(_) => {
            let raw = std::fs::read_to_string(&common_marker).map_err(|source| {
                MutationLockError::Inspect {
                    path: common_marker.clone(),
                    source,
                }
            })?;
            let relative = Path::new(raw.trim());
            if relative.as_os_str().is_empty() {
                return Err(MutationLockError::InvalidGitMetadata(common_marker));
            }
            canonicalise(&if relative.is_absolute() {
                relative.to_path_buf()
            } else {
                git_dir.join(relative)
            })?
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => git_dir,
        Err(source) => {
            return Err(MutationLockError::Inspect {
                path: common_marker,
                source,
            });
        }
    };

    Ok(common_dir.join(LOCK_RELATIVE_PATH))
}

fn canonicalise(path: &Path) -> Result<PathBuf, MutationLockError> {
    std::fs::canonicalize(path).map_err(|source| MutationLockError::Inspect {
        path: path.to_path_buf(),
        source,
    })
}
