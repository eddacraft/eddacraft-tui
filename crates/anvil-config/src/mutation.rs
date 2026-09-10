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

    let (git_dir, gitfile) = if metadata.is_dir() {
        let git_dir = canonicalise(&dot_git)?;
        if std::fs::symlink_metadata(git_dir.join("commondir")).is_ok() {
            return Err(MutationLockError::InvalidGitMetadata(
                git_dir.join("commondir"),
            ));
        }
        (git_dir, false)
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
        let git_dir = canonicalise(&if path.is_absolute() {
            path.to_path_buf()
        } else {
            repo_root.join(path)
        })?;
        validate_worktree_backlink(&dot_git, &git_dir)?;
        (git_dir, true)
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
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => git_dir.clone(),
        Err(source) => {
            return Err(MutationLockError::Inspect {
                path: common_marker,
                source,
            });
        }
    };

    if gitfile {
        validate_gitfile_worktree_layout(&dot_git, &git_dir, &common_dir)?;
    }

    Ok(common_dir.join(LOCK_RELATIVE_PATH))
}

fn validate_worktree_backlink(dot_git: &Path, git_dir: &Path) -> Result<(), MutationLockError> {
    let backlink = git_dir.join("gitdir");
    let metadata = std::fs::symlink_metadata(&backlink)
        .map_err(|_| MutationLockError::InvalidGitMetadata(backlink.clone()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(MutationLockError::InvalidGitMetadata(backlink));
    }
    let raw = std::fs::read_to_string(&backlink).map_err(|source| MutationLockError::Inspect {
        path: backlink.clone(),
        source,
    })?;
    let target = Path::new(raw.trim());
    if target.as_os_str().is_empty() || canonicalise(target)? != canonicalise(dot_git)? {
        return Err(MutationLockError::InvalidGitMetadata(backlink));
    }
    Ok(())
}

/// A gitfile is Git authority only when `git_dir` is the worktree admin
/// directory `<common_dir>/worktrees/<name>` and that entry still points at
/// the gitfile. Reciprocal `gitdir` metadata alone can be planted against an
/// attacker-chosen `commondir`.
fn validate_gitfile_worktree_layout(
    dot_git: &Path,
    git_dir: &Path,
    common_dir: &Path,
) -> Result<(), MutationLockError> {
    let worktrees_path = common_dir.join("worktrees");
    let Ok(metadata) = std::fs::symlink_metadata(&worktrees_path) else {
        return Err(MutationLockError::InvalidGitMetadata(worktrees_path));
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MutationLockError::InvalidGitMetadata(worktrees_path));
    }
    let worktrees = canonicalise(&worktrees_path)?;
    let name = git_dir
        .file_name()
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
        .ok_or_else(|| MutationLockError::InvalidGitMetadata(git_dir.to_path_buf()))?;
    let parent = git_dir
        .parent()
        .ok_or_else(|| MutationLockError::InvalidGitMetadata(git_dir.to_path_buf()))?;
    if parent != worktrees.as_path() {
        return Err(MutationLockError::InvalidGitMetadata(git_dir.to_path_buf()));
    }
    let expected = canonicalise(&worktrees.join(name))?;
    if expected != git_dir {
        return Err(MutationLockError::InvalidGitMetadata(git_dir.to_path_buf()));
    }
    validate_worktree_backlink(dot_git, &expected)
}

fn canonicalise(path: &Path) -> Result<PathBuf, MutationLockError> {
    std::fs::canonicalize(path).map_err(|source| MutationLockError::Inspect {
        path: path.to_path_buf(),
        source,
    })
}
