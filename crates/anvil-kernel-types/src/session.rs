//! Control/session join primitives (GV2-013, G-05).
//!
//! The control/session graph is keyed by worktree; the semantic, dependency, and
//! trust graphs are keyed by workspace-root-relative file paths. [`WorkspaceRoot`]
//! is the bridge between them: it relativises an absolute path to the
//! workspace-root-relative key the resident graphs use, so a control-graph join
//! can name the files a session touched without ever embedding the absolute root
//! (a privacy line shared with the persistence snapshot — identities stay
//! root-relative).
//!
//! This is a join *key*, not the control graph itself. The control authority
//! (the intercept daemon's session registry) owns the session records; consumers
//! provide the live join, keeping this crate free of the daemon proto types.

use std::path::{Path, PathBuf};

/// The absolute root of a worktree — the join key between the control/session
/// graph and the file-keyed graphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRoot {
    root: PathBuf,
}

impl WorkspaceRoot {
    /// Wrap an absolute worktree root.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The wrapped root path.
    pub fn as_path(&self) -> &Path {
        &self.root
    }

    /// Relativise `path` to the workspace-root-relative key the resident graphs
    /// use, or `None` if `path` is not under this root.
    ///
    /// The result uses forward slashes on every platform so the key matches the
    /// graph's stored paths (Anvil determinism + the snapshot's
    /// `is_workspace_root_relative` contract). The absolute root is never part of
    /// the returned key.
    ///
    /// `strip_prefix` is purely **lexical** — it resolves no symlinks and no `..`
    /// components. Callers must pass canonical paths; as a fail-closed guard this
    /// returns `None` for any path that lexically matches the root prefix but then
    /// escapes it via a `..` component (so a non-canonical path can never yield a
    /// key that points outside the worktree).
    pub fn relativise(&self, path: &Path) -> Option<String> {
        let rel = path.strip_prefix(&self.root).ok()?;
        if rel
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return None;
        }
        Some(rel.to_string_lossy().replace('\\', "/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_root_relativises_under_root() {
        let root = WorkspaceRoot::new("/home/dev/proj");
        assert_eq!(
            root.relativise(Path::new("/home/dev/proj/src/a.ts")),
            Some("src/a.ts".to_string())
        );
        // The root itself relativises to the empty key.
        assert_eq!(
            root.relativise(Path::new("/home/dev/proj")),
            Some(String::new())
        );
    }

    #[test]
    fn workspace_root_rejects_outside_root() {
        let root = WorkspaceRoot::new("/home/dev/proj");
        assert_eq!(root.relativise(Path::new("/home/dev/other/a.ts")), None);
        assert_eq!(root.relativise(Path::new("/etc/passwd")), None);
    }

    #[test]
    fn workspace_root_rejects_parent_dir_escape() {
        let root = WorkspaceRoot::new("/home/dev/proj");
        // Lexically matches the root prefix but escapes via `..` — fail closed.
        assert_eq!(
            root.relativise(Path::new("/home/dev/proj/../other/secret.ts")),
            None
        );
        assert_eq!(
            root.relativise(Path::new("/home/dev/proj/src/../../etc/passwd")),
            None
        );
    }
}
