//! INTL-001: launch-context resolution.
//!
//! The launcher needs the same vocabulary the daemon already speaks
//! at registration time: which cwd, which worktree, which tmux pane.
//! Worktree resolution walks up from `--cwd` looking for either a
//! `.git` directory (regular checkout) or a `.git` file (linked
//! worktree). On miss the launcher uses the cwd itself rather than
//! refusing to launch — a non-git directory is still a session the
//! daemon may want to fence later.

use std::env;
use std::path::{Path, PathBuf};

/// Resolved context for a single launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchContext {
    /// Working directory the child is spawned in. Always absolute.
    pub cwd: PathBuf,
    /// Best-effort worktree root for the daemon's fence key.
    /// Equals `cwd` when no git boundary was found.
    pub worktree: PathBuf,
    /// `$TMUX_PANE` if present — useful for matching the daemon's
    /// session telemetry against a tmux pane id.
    pub tmux_pane: Option<String>,
}

impl LaunchContext {
    /// Resolve the launch context.
    ///
    /// `cwd_override` is the optional `--cwd` from the CLI;
    /// `worktree_override` is the optional `--worktree`. When both
    /// are absent the function reads `std::env::current_dir()` and
    /// walks for the worktree.
    pub fn resolve(
        cwd_override: Option<PathBuf>,
        worktree_override: Option<PathBuf>,
    ) -> Result<Self, ContextError> {
        let cwd = match cwd_override {
            Some(p) => canonicalise_existing(&p)?,
            None => env::current_dir().map_err(ContextError::CwdUnavailable)?,
        };
        let worktree = match worktree_override {
            Some(p) => canonicalise_existing(&p)?,
            None => find_worktree_root(&cwd).unwrap_or_else(|| cwd.clone()),
        };
        let tmux_pane = env::var("TMUX_PANE").ok().filter(|s| !s.is_empty());
        Ok(Self {
            cwd,
            worktree,
            tmux_pane,
        })
    }
}

/// Errors during context resolution.
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("could not read current directory: {0}")]
    CwdUnavailable(#[source] std::io::Error),
    #[error("path does not exist: {0}")]
    PathMissing(PathBuf),
    #[error("path is not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("could not canonicalise {path}: {source}")]
    Canonicalise {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

fn canonicalise_existing(p: &Path) -> Result<PathBuf, ContextError> {
    if !p.exists() {
        return Err(ContextError::PathMissing(p.to_path_buf()));
    }
    // Reject regular files / sockets / FIFOs masquerading as a
    // `--cwd` or `--worktree` argument; the child would just fail
    // later with a generic spawn error and we lose the "bad launch
    // context" signal.
    let metadata = p.metadata().map_err(|source| ContextError::Canonicalise {
        path: p.to_path_buf(),
        source,
    })?;
    if !metadata.is_dir() {
        return Err(ContextError::NotADirectory(p.to_path_buf()));
    }
    p.canonicalize()
        .map_err(|source| ContextError::Canonicalise {
            path: p.to_path_buf(),
            source,
        })
}

/// Walk up the directory tree looking for a `.git` entry (either a
/// directory or a file — `git worktree add` writes a `.git` *file*
/// that points at the parent repo's worktree metadata). Returns
/// `None` when no boundary is found before the filesystem root.
fn find_worktree_root(start: &Path) -> Option<PathBuf> {
    let mut current: &Path = start;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Tempdir whose ancestors have no `.git` entry, so the no-boundary case
    /// is meaningful. Default `tempfile` under `/tmp` is not safe: an ambient
    /// empty `/tmp/.git` (other tools leave those) makes the walk stop at
    /// `/tmp` and this assertion fails spuriously.
    fn tempdir_outside_any_git_tree() -> tempfile::TempDir {
        let candidates: Vec<PathBuf> = [
            std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from),
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")),
            Some(PathBuf::from("/var/tmp")),
            Some(std::env::temp_dir()),
        ]
        .into_iter()
        .flatten()
        .collect();

        for base in candidates {
            let nest = base.join("anvil-run-test-tmp");
            // Refuse nests whose ancestors include a git worktree before
            // creating anything — `find_worktree_root` only inspects path
            // components, so the nest need not exist yet. Creating first
            // would leave `anvil-run-test-tmp` behind when the base sits
            // inside a repo (e.g. XDG_CACHE_HOME under a checkout).
            if find_worktree_root(&nest).is_some() {
                continue;
            }
            if fs::create_dir_all(&nest).is_err() {
                continue;
            }
            if let Ok(tmp) = tempfile::Builder::new().prefix("no-git-").tempdir_in(&nest) {
                // Double-check: no boundary between nest and the new dir.
                if find_worktree_root(tmp.path()).is_none() {
                    return tmp;
                }
            }
        }
        panic!(
            "could not allocate a tempdir outside any git tree; \
             clear ambient .git entries under TMPDIR (e.g. empty /tmp/.git)"
        );
    }

    #[test]
    fn worktree_root_is_cwd_when_no_git_boundary() {
        let tmp = tempdir_outside_any_git_tree();
        let ctx = LaunchContext::resolve(Some(tmp.path().to_path_buf()), None).expect("resolve");
        // `tmp` is canonicalised; without a `.git` ancestor the worktree is the cwd.
        assert_eq!(ctx.worktree, ctx.cwd);
    }

    #[test]
    fn worktree_root_finds_dot_git_directory() {
        let tmp = tempfile::tempdir().expect("tmp");
        let repo = tmp.path().join("repo");
        let nested = repo.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(repo.join(".git")).unwrap();
        let ctx = LaunchContext::resolve(Some(nested.clone()), None).expect("resolve");
        let canonical_repo = repo.canonicalize().unwrap();
        assert_eq!(ctx.worktree, canonical_repo);
    }

    #[test]
    fn worktree_root_finds_dot_git_file_for_linked_worktrees() {
        // `git worktree add` stores a `.git` *file* in the linked
        // worktree pointing at `gitdir: ...`. The walker must
        // accept the file form, not only the directory form.
        let tmp = tempfile::tempdir().expect("tmp");
        let linked = tmp.path().join("linked");
        fs::create_dir_all(&linked).unwrap();
        fs::write(
            linked.join(".git"),
            "gitdir: /tmp/main/.git/worktrees/linked\n",
        )
        .unwrap();
        let ctx = LaunchContext::resolve(Some(linked.clone()), None).expect("resolve");
        assert_eq!(ctx.worktree, linked.canonicalize().unwrap());
    }

    #[test]
    fn explicit_worktree_override_wins_over_walk() {
        let tmp = tempfile::tempdir().expect("tmp");
        let cwd = tmp.path().join("cwd");
        let wt = tmp.path().join("wt");
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(&wt).unwrap();
        let ctx = LaunchContext::resolve(Some(cwd.clone()), Some(wt.clone())).expect("resolve");
        assert_eq!(ctx.worktree, wt.canonicalize().unwrap());
    }

    #[test]
    fn missing_cwd_override_is_rejected() {
        let err = LaunchContext::resolve(Some(PathBuf::from("/no/such/dir/anvil-test")), None)
            .expect_err("missing path must error");
        assert!(matches!(err, ContextError::PathMissing(_)));
    }

    #[test]
    fn file_override_is_rejected_as_not_a_directory() {
        // A regular file passed as `--cwd` or `--worktree` would
        // pass the old `exists()` check and only fail later as a
        // generic spawn error; surface it now with the right
        // diagnostic.
        let tmp = tempfile::tempdir().expect("tmp");
        let file_path = tmp.path().join("regular-file");
        fs::write(&file_path, b"not a directory").unwrap();
        let err = LaunchContext::resolve(Some(file_path.clone()), None)
            .expect_err("regular file must error");
        assert!(matches!(err, ContextError::NotADirectory(_)));

        let wt_err =
            LaunchContext::resolve(Some(tmp.path().to_path_buf()), Some(file_path.clone()))
                .expect_err("regular file as --worktree must error");
        assert!(matches!(wt_err, ContextError::NotADirectory(_)));
    }
}
