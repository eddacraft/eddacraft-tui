//! Policy for file changes in worktrees not yet registered with the daemon.

use std::path::PathBuf;
use std::sync::Arc;

use crate::config::Resolved;
use crate::enforcement::FileChange;
use crate::fence::FenceStore;
use crate::watcher::UnregisteredHandler;

/// What [`UnregisteredChangePolicy::handle`] decided to do for a
/// given unattributed change. Returned to callers (tests, future
/// telemetry surfaces) so the routing edge is observable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnregisteredOutcome {
    /// The worktree corresponding to the change was fenced. Carries
    /// the worktree path and the reason posted to the fence store
    /// (always derived from the change's path; never operator
    /// input).
    Fenced { worktree: PathBuf, reason: String },
    /// No worktree could be derived from the change (the path had
    /// no parent or fence-store interaction failed). Surfaced so
    /// the caller can log; the daemon stays up.
    UnableToFence { reason: String },
}

/// INTD-010 handler. Plugged into [`UnregisteredHandler`] by the
/// daemon so the watcher's unknown-attribution path applies the
/// configured policy.
///
/// The handler holds an `Arc<FenceStore>` (fence state survives
/// daemon restarts per INTD-007) and a snapshot of the resolved
/// config. Config is wrapped in an `arc_swap`-style cell at the
/// daemon level, but the v1 handler reads a single resolved value
/// at construction time — config-reload semantics are out of scope
/// for INTD-010.
pub struct UnregisteredChangePolicy {
    fence_store: Arc<FenceStore>,
    config: Resolved,
}

impl UnregisteredChangePolicy {
    pub fn new(fence_store: Arc<FenceStore>, config: Resolved) -> Self {
        Self {
            fence_store,
            config,
        }
    }

    /// Apply the policy to a single batch of unattributed changes.
    /// Returns one [`UnregisteredOutcome`] per distinct worktree
    /// the changes touch; if multiple changes share the same
    /// worktree, the worktree is fenced once.
    ///
    /// Tests call this directly so they can assert the fence-store
    /// side effect. Production callers go through the
    /// `UnregisteredHandler` trait the watcher depends on.
    pub fn apply(&self, changes: &[FileChange]) -> Vec<UnregisteredOutcome> {
        // The hard-cap invariant lives here, not in `Resolved`:
        // even when `on_ambiguous_ownership` is `Warn`, INTD-010
        // applies a fence. The parse vocabulary in
        // `AmbiguousOwnership` already refuses values stricter than
        // `Fence`, and AD-3 makes the always-fence policy the
        // belt-and-braces invariant.
        let _ = self.config.on_ambiguous_ownership; // Read for clarity.

        let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        let mut outcomes = Vec::new();
        for change in changes {
            let Some(worktree) = derive_worktree_for(&change.path) else {
                outcomes.push(UnregisteredOutcome::UnableToFence {
                    reason: format!(
                        "no worktree could be derived from {}",
                        change.path.display(),
                    ),
                });
                continue;
            };
            if !seen.insert(worktree.clone()) {
                continue;
            }
            let reason = format!(
                "attribution: unknown-agent — change at {}",
                change.path.display(),
            );
            match self.fence_store.fence_worktree(&worktree, &reason) {
                Ok(_) => outcomes.push(UnregisteredOutcome::Fenced { worktree, reason }),
                Err(err) => outcomes.push(UnregisteredOutcome::UnableToFence {
                    reason: format!("fence store rejected fence: {err}"),
                }),
            }
        }
        outcomes
    }
}

impl UnregisteredHandler for UnregisteredChangePolicy {
    fn handle(&self, changes: &[FileChange]) {
        let _ = self.apply(changes);
    }
}

/// Derive the worktree key for a change that could not be attributed
/// to a registered session.
///
/// Prefer the nearest enclosing Git worktree root (directory that
/// contains a `.git` entry — either a directory for a main worktree
/// or a `gitdir:` file for a linked worktree). Registration and
/// [`crate::fence::FenceState::is_fenced`] both key on that worktree
/// path, so fencing only the file's immediate parent would leave the
/// real worktree unfenced and would fail to de-duplicate fences
/// across nested child directories.
///
/// When no Git boundary is found (non-git trees, synthetic fixtures),
/// fall back to the change's parent directory so the policy still
/// records *some* fence key rather than dropping the event.
fn derive_worktree_for(path: &std::path::Path) -> Option<PathBuf> {
    if let Some(root) = enclosing_git_worktree(path) {
        return Some(root);
    }
    path.parent().map(std::path::Path::to_path_buf)
}

/// Walk ancestors of `path` looking for a Git worktree root. Starts
/// at the path's parent (the file itself is never a worktree root)
/// and stops at the first directory that is a real Git worktree root.
/// Does not shell out to `git`.
///
/// A real root is a directory whose `.git` entry is either a directory
/// (main worktree) or a file whose contents begin with a `gitdir:`
/// pointer (linked worktree / some submodule layouts). A bare file
/// named `.git` that is *not* a gitdir pointer does not count — that
/// would let an accidental or adversarial nested file truncate the
/// walk and fence the wrong directory.
fn enclosing_git_worktree(path: &std::path::Path) -> Option<PathBuf> {
    for dir in path.ancestors().skip(1) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        if is_git_worktree_root(dir) {
            return Some(dir.to_path_buf());
        }
    }
    None
}

/// `true` when `dir` looks like a Git worktree root (main or linked).
fn is_git_worktree_root(dir: &std::path::Path) -> bool {
    let git = dir.join(".git");
    let Ok(meta) = std::fs::symlink_metadata(&git) else {
        return false;
    };
    if meta.is_dir() {
        return true;
    }
    if !meta.is_file() {
        return false;
    }
    // Linked worktree (and some submodule) form: `.git` is a file
    // containing `gitdir: <path>`. Reject other files so a stray
    // nested `.git` cannot truncate the ancestor walk.
    match std::fs::read_to_string(&git) {
        Ok(contents) => contents
            .lines()
            .any(|line| line.trim().starts_with("gitdir:")),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AmbiguousOwnership, Mode as ConfigMode};
    use anvil_intercept_rules::ChangeKind;
    use std::path::Path;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn store_in(tmp: &TempDir) -> Arc<FenceStore> {
        // Mirror the existing fence-store test pattern: nest under a
        // `state/` subdir so the FenceStore creates the parent at
        // 0700 itself (the explicit private-parent check refuses
        // 0755 / world-readable tempdirs).
        Arc::new(FenceStore::at_path(
            tmp.path().join("state/intercept-fences.json"),
        ))
    }

    fn change_in(worktree: &Path, name: &str) -> FileChange {
        FileChange {
            path: worktree.join(name),
            change_kind: ChangeKind::Modified,
        }
    }

    /// Seed a minimal Git worktree marker so fixtures stay hermetic even
    /// when `TMPDIR` lives inside another repository (nearest-`.git`
    /// walk would otherwise climb out of the fixture).
    fn init_git_worktree(worktree: &Path) {
        std::fs::create_dir_all(worktree).unwrap();
        let git = worktree.join(".git");
        if !git.exists() {
            std::fs::create_dir(&git).unwrap();
        }
    }

    fn config_with(amb: AmbiguousOwnership) -> Resolved {
        Resolved {
            mode: ConfigMode::Fence,
            on_ambiguous_ownership: amb,
            ..Resolved::default()
        }
    }

    /// Test (a): an attributed change does NOT reach this module.
    /// We assert the property by construction — the watcher's
    /// `Owned` branch never calls `UnregisteredHandler::handle`.
    /// `crate::watcher::tests::attributed_change_reaches_enforcement_pipeline`
    /// already pins this; the test here is a wire-level smoke
    /// asserting the policy honours the hard-cap fence.
    #[test]
    fn fence_setting_fences_the_changes_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("workspace");
        init_git_worktree(&worktree);
        let store = store_in(&tmp);
        let policy = UnregisteredChangePolicy::new(
            Arc::clone(&store),
            config_with(AmbiguousOwnership::Fence),
        );

        let changes = vec![change_in(&worktree, "file.rs")];
        let outcomes = policy.apply(&changes);
        assert_eq!(outcomes.len(), 1);
        match &outcomes[0] {
            UnregisteredOutcome::Fenced {
                worktree: w,
                reason,
            } => {
                assert_eq!(w, &worktree);
                assert!(
                    reason.contains("unknown-agent"),
                    "fence reason must tag attribution: unknown-agent, got {reason}",
                );
            }
            other @ UnregisteredOutcome::UnableToFence { .. } => {
                panic!("expected Fenced, got {other:?}")
            }
        }
        // The worktree must be persisted in the fence store —
        // checks the side effect, not just the outcome shape.
        assert!(
            store.load().expect("load").is_fenced(&worktree),
            "worktree must be fenced after policy.apply",
        );
    }

    /// Test (b): unknown-attribution changes are tagged correctly
    /// in the fence-store reason. The reason string is the audit
    /// trail operators read; pinning the prefix prevents drift
    /// between what telemetry says and what the persisted fence
    /// records.
    #[test]
    fn unknown_change_is_tagged_in_fence_reason() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("rogue");
        init_git_worktree(&worktree);
        let store = store_in(&tmp);
        let policy = UnregisteredChangePolicy::new(
            Arc::clone(&store),
            config_with(AmbiguousOwnership::Fence),
        );

        policy.apply(&[change_in(&worktree, "rogue.rs")]);
        let state = store.load().expect("load fence state");
        let record = state
            .active_fences()
            .iter()
            .find(|r| r.worktree == std::fs::canonicalize(&worktree).unwrap())
            .expect("worktree fenced");
        assert!(
            record.reason.contains("unknown-agent"),
            "fence record must carry the unknown-agent tag: {}",
            record.reason,
        );
    }

    /// Test (c): the AD-3 hard cap. Even an operator-set `warn`
    /// for `on_ambiguous_ownership` results in a fence — the
    /// always-fence rule is a code invariant, not a config knob.
    /// Pinned because removing it would silently weaken the
    /// security posture.
    #[test]
    fn warn_setting_still_hard_caps_to_fence() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("ws");
        init_git_worktree(&worktree);
        let store = store_in(&tmp);
        let policy = UnregisteredChangePolicy::new(
            Arc::clone(&store),
            config_with(AmbiguousOwnership::Warn),
        );

        let outcomes = policy.apply(&[change_in(&worktree, "f.rs")]);
        assert!(
            matches!(outcomes[0], UnregisteredOutcome::Fenced { .. }),
            "warn setting must still fence — AD-3 hard cap; got {:?}",
            outcomes[0],
        );
        assert!(store.load().unwrap().is_fenced(&worktree));
    }

    /// Nested paths must fence the git worktree root, not the file's
    /// immediate parent. `is_fenced` matches exact worktree keys used at
    /// registration, so fencing only `.../src/nested` leaves the real
    /// worktree unfenced and fails to de-duplicate across child dirs.
    #[test]
    fn nested_change_fences_enclosing_git_worktree_root() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("workspace");
        init_git_worktree(&worktree);
        let nested = worktree.join("src/nested");
        std::fs::create_dir_all(&nested).unwrap();
        let store = store_in(&tmp);
        let policy = UnregisteredChangePolicy::new(
            Arc::clone(&store),
            config_with(AmbiguousOwnership::Fence),
        );

        let changes = vec![change_in(&nested, "file.rs")];
        let outcomes = policy.apply(&changes);
        assert_eq!(outcomes.len(), 1);
        match &outcomes[0] {
            UnregisteredOutcome::Fenced { worktree: w, .. } => {
                assert_eq!(
                    w, &worktree,
                    "fence key must be the git worktree root, not the nested parent",
                );
            }
            other @ UnregisteredOutcome::UnableToFence { .. } => {
                panic!("expected Fenced, got {other:?}")
            }
        }
        assert!(
            store.load().expect("load").is_fenced(&worktree),
            "canonical worktree root must be fenced after nested unknown change",
        );
        assert!(
            !store.load().expect("load").is_fenced(&nested),
            "nested parent directory must not be the fence key",
        );
    }

    /// Linked worktrees store `.git` as a file pointing at a gitdir.
    /// The fence key is still the linked worktree root that contains
    /// that file, not a parent of the gitdir.
    #[test]
    fn nested_change_fences_linked_worktree_with_git_file() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("linked-wt");
        let nested = worktree.join("crates/foo");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(worktree.join(".git"), b"gitdir: /tmp/fake-gitdir\n").unwrap();
        let store = store_in(&tmp);
        let policy = UnregisteredChangePolicy::new(
            Arc::clone(&store),
            config_with(AmbiguousOwnership::Fence),
        );

        let outcomes = policy.apply(&[change_in(&nested, "lib.rs")]);
        match &outcomes[0] {
            UnregisteredOutcome::Fenced { worktree: w, .. } => {
                assert_eq!(w, &worktree);
            }
            other @ UnregisteredOutcome::UnableToFence { .. } => {
                panic!("expected Fenced, got {other:?}")
            }
        }
        assert!(store.load().unwrap().is_fenced(&worktree));
    }

    /// Multiple changes inside the same worktree fence the
    /// worktree once, not N times. The policy de-duplicates by
    /// derived worktree key — including when nested paths would
    /// otherwise yield distinct parent directories.
    #[test]
    fn multiple_changes_in_same_worktree_fence_once() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("multi");
        init_git_worktree(&worktree);
        std::fs::create_dir_all(worktree.join("src/a")).unwrap();
        std::fs::create_dir_all(worktree.join("src/b")).unwrap();
        let store = store_in(&tmp);
        let policy = UnregisteredChangePolicy::new(
            Arc::clone(&store),
            config_with(AmbiguousOwnership::Fence),
        );

        let changes = vec![
            change_in(&worktree.join("src/a"), "a.rs"),
            change_in(&worktree.join("src/b"), "b.rs"),
            change_in(&worktree, "c.rs"),
        ];
        let outcomes = policy.apply(&changes);
        let fenced_count = outcomes
            .iter()
            .filter(|o| matches!(o, UnregisteredOutcome::Fenced { .. }))
            .count();
        assert_eq!(
            fenced_count, 1,
            "single fence per worktree, got {outcomes:?}"
        );
        assert!(store.load().unwrap().is_fenced(&worktree));
    }

    /// When no Git worktree boundary exists above the change, fall
    /// back to the file's parent directory (pre-fix behaviour for
    /// non-git trees). Prefer a known non-repo host on Unix so the
    /// fallback is exercised even when process `TMPDIR` sits inside a
    /// real repository; keep a last-resort skip for exotic hosts.
    #[test]
    fn non_git_tree_falls_back_to_parent_directory() {
        let tmp = {
            #[cfg(unix)]
            {
                tempfile::Builder::new()
                    .prefix("anvil-unreg-nongit-")
                    .tempdir_in("/tmp")
                    .unwrap_or_else(|_| tempfile::tempdir().unwrap())
            }
            #[cfg(not(unix))]
            {
                tempfile::tempdir().unwrap()
            }
        };
        if enclosing_git_worktree(tmp.path()).is_some() {
            // Host temp still sits under a real git worktree; the
            // nearest-root walk is correct to prefer it — skip the
            // parent-fallback assertion rather than asserting the
            // wrong key.
            return;
        }
        let worktree = tmp.path().join("nongit-ws");
        std::fs::create_dir(&worktree).unwrap();
        let store = store_in(&tmp);
        let policy = UnregisteredChangePolicy::new(
            Arc::clone(&store),
            config_with(AmbiguousOwnership::Fence),
        );

        let outcomes = policy.apply(&[change_in(&worktree, "file.rs")]);
        match &outcomes[0] {
            UnregisteredOutcome::Fenced { worktree: w, .. } => {
                assert_eq!(w, &worktree);
            }
            other @ UnregisteredOutcome::UnableToFence { .. } => {
                panic!("expected Fenced, got {other:?}")
            }
        }
        assert!(store.load().unwrap().is_fenced(&worktree));
    }

    /// A stray nested `.git` *file* that is not a gitdir pointer must
    /// not truncate the ancestor walk. The real worktree root above
    /// it remains the fence key.
    #[test]
    fn stray_nested_git_file_does_not_truncate_walk() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("workspace");
        init_git_worktree(&worktree);
        let nested = worktree.join("src/nested");
        std::fs::create_dir_all(&nested).unwrap();
        // Adversarial / accidental file — not a gitdir pointer.
        std::fs::write(nested.join(".git"), b"not a gitdir pointer\n").unwrap();
        let store = store_in(&tmp);
        let policy = UnregisteredChangePolicy::new(
            Arc::clone(&store),
            config_with(AmbiguousOwnership::Fence),
        );

        let outcomes = policy.apply(&[change_in(&nested, "file.rs")]);
        match &outcomes[0] {
            UnregisteredOutcome::Fenced { worktree: w, .. } => {
                assert_eq!(
                    w, &worktree,
                    "stray nested .git file must not become the fence key",
                );
            }
            other @ UnregisteredOutcome::UnableToFence { .. } => {
                panic!("expected Fenced, got {other:?}")
            }
        }
        assert!(store.load().unwrap().is_fenced(&worktree));
    }

    /// Path with no parent (e.g. `"file.rs"` at the cwd root) is
    /// surfaced as `UnableToFence`. The daemon does not crash;
    /// the operator gets a log entry.
    #[test]
    fn change_with_no_parent_surfaces_unable_to_fence() {
        let tmp = tempfile::tempdir().unwrap();
        let store = store_in(&tmp);
        let policy = UnregisteredChangePolicy::new(
            Arc::clone(&store),
            config_with(AmbiguousOwnership::Fence),
        );

        let change = FileChange {
            path: PathBuf::from("/"),
            change_kind: ChangeKind::Modified,
        };
        let outcomes = policy.apply(&[change]);
        assert!(matches!(
            outcomes[0],
            UnregisteredOutcome::UnableToFence { .. }
        ));
    }
}
