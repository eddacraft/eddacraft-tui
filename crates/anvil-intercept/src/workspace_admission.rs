//! DSV-003 Task 2 (ADR-061 §4): the per-connection admitted-workspace-root set.
//!
//! `validate_paths` is the first daemon verb to read arbitrary on-disk paths a
//! client names, so a verb is authorised against a root **iff** that root has
//! been admitted on this connection. Each admitted entry pairs the *canonical*
//! root path (the key an incoming `workspace_root` is matched against) with the
//! once-opened [`WorkspaceAnchor`](crate::workspace_anchor::WorkspaceAnchor) —
//! the read anchor and the workspace identity (a Unix `O_PATH` dirfd or a
//! Windows directory handle). All later **reads** go through the held anchor, so
//! a root-directory retarget after admission cannot redirect them (security C2);
//! a stale anchor fails closed rather than re-resolving the path.
//!
//! This swap-immunity is for *reads*, not for the *admission* step: admission
//! canonicalises the root string and then opens it, so a same-uid writer could
//! in principle swap the directory between the canonicalise (which decides
//! allowlist membership) and the open (which pins the fd). That window is
//! in-model — the trust boundary is `SO_PEERCRED` same-uid (contract §4), so
//! allowlist confinement is only ever as strong as that boundary — and crosses
//! no privilege boundary.
//!
//! ## Admission modes (security C3)
//!
//! - **Open** (default): the set grows on first contact — the first time a
//!   nameable root is authorised it is auto-admitted. A compromised *same-uid*
//!   agent can therefore adopt any root it can name and read arbitrary on-disk
//!   content under the daemon. This is acceptable **only** because the trust
//!   boundary is `SO_PEERCRED` same-uid; no intra-uid boundary is claimed
//!   (contract §4).
//! - **Allowlist**: only operator-pre-admitted roots are authorised; an
//!   unlisted root is refused. This is the confinement boundary for operators
//!   who need one (the `anvil workspace` CLI that configures it is DSV-008).
//!
//! Workspace identity is derived purely from the canonicalised path + the held
//! dirfd. There is deliberately **no** procfs per-pid working-directory lookup
//! anywhere in the auth path — a connection's working directory is not its
//! workspace.
//!
//! **Scope (DSV-003):** this is the standalone, unit-tested admission
//! component. Threading a `&mut AdmittedRoots` through the `ipc.rs` connection
//! handler and calling [`AdmittedRoots::authorise`] from the `validate_paths`
//! dispatch arm is DSV-005 work — that arm is the only real consumer, so the
//! wiring lands with it rather than as inert plumbing here.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use crate::workspace_anchor::WorkspaceAnchor;

/// How the admitted-root set decides whether to admit a not-yet-seen root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionMode {
    /// First-touch adopt: any nameable root is auto-admitted on first contact.
    Open,
    /// Confinement: only operator-pre-admitted roots are authorised.
    Allowlist,
}

/// The set of roots permitted in `Allowlist` mode: a set of *exact* canonical
/// roots plus a list of *prefix* canonical roots (a root at or beneath a prefix
/// is permitted, so a single `prefix` entry confines a whole subtree).
///
/// Built by [`crate::confinement`] from operator config (DSV-008, Task 14) and
/// applied here, so the daemon's admission decision understands both the exact
/// and the subtree (`prefix`) forms of an operator allow entry. Empty in `Open`
/// mode (the set grows on first contact instead).
#[derive(Debug, Default, Clone)]
pub struct AllowPolicy {
    /// Canonical roots matched exactly.
    exact: BTreeSet<PathBuf>,
    /// Canonical roots whose entire subtree is permitted.
    prefixes: Vec<PathBuf>,
}

impl AllowPolicy {
    /// Build a policy from already-canonicalised exact + prefix roots. The
    /// caller is responsible for canonicalisation so a `prefix` match compares
    /// like-for-like against a canonical incoming root.
    #[must_use]
    pub fn new(
        exact: impl IntoIterator<Item = PathBuf>,
        prefixes: impl IntoIterator<Item = PathBuf>,
    ) -> Self {
        Self {
            exact: exact.into_iter().collect(),
            prefixes: prefixes.into_iter().collect(),
        }
    }

    /// Whether `canonical_root` is permitted: an exact match, or at/beneath any
    /// prefix root. `starts_with` is component-wise, so `/a/b` permits `/a/b/c`
    /// but never the sibling `/a/b-other`.
    #[must_use]
    pub fn permits(&self, canonical_root: &Path) -> bool {
        self.exact.contains(canonical_root)
            || self
                .prefixes
                .iter()
                .any(|prefix| canonical_root.starts_with(prefix))
    }
}

/// A per-connection set of admitted workspace roots, each paired with its held
/// [`WorkspaceAnchor`] (read anchor + identity — a Unix dirfd or a Windows
/// directory handle).
#[derive(Debug)]
pub struct AdmittedRoots {
    mode: AdmissionMode,
    /// Roots permitted in `Allowlist` mode. Empty in `Open` mode.
    allow: AllowPolicy,
    /// Canonical root → held anchor. Insertion-once; never re-resolved.
    admitted: BTreeMap<PathBuf, WorkspaceAnchor>,
}

impl AdmittedRoots {
    /// Open mode: the set grows on first contact.
    #[must_use]
    pub fn new_open() -> Self {
        Self {
            mode: AdmissionMode::Open,
            allow: AllowPolicy::default(),
            admitted: BTreeMap::new(),
        }
    }

    /// Allowlist mode over exact roots only. Each entry is canonicalised at
    /// construction; entries that do not currently resolve are dropped (they
    /// cannot match a real, openable root anyway). For prefix (subtree) entries
    /// or operator-config-driven policies use [`Self::new_allowlist_with_policy`].
    #[must_use]
    pub fn new_allowlist<I>(allowed: I) -> Self
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let exact = allowed
            .into_iter()
            .filter_map(|p| std::fs::canonicalize(p).ok());
        Self::new_allowlist_with_policy(AllowPolicy::new(exact, std::iter::empty()))
    }

    /// Allowlist mode driven by an explicit [`AllowPolicy`] (exact + prefix).
    /// This is the seam [`crate::confinement`] uses to apply operator
    /// confinement config (DSV-008): the policy already carries the
    /// canonicalised allow roots plus the implicitly-admitted primary root.
    #[must_use]
    pub fn new_allowlist_with_policy(allow: AllowPolicy) -> Self {
        Self {
            mode: AdmissionMode::Allowlist,
            allow,
            admitted: BTreeMap::new(),
        }
    }

    /// This connection's admission mode.
    #[must_use]
    pub fn mode(&self) -> AdmissionMode {
        self.mode
    }

    /// Whether `canonical_root` is currently admitted (already has a held fd).
    #[must_use]
    pub fn is_admitted(&self, canonical_root: &Path) -> bool {
        self.admitted.contains_key(canonical_root)
    }

    /// Explicitly admit a root: open its [`WorkspaceAnchor`] once and store it
    /// under its canonical path. Idempotent — a second call for an
    /// already-admitted root keeps the original anchor (identity is pinned at
    /// first admission).
    ///
    /// # Errors
    /// Propagates canonicalisation / open failures (root missing or not a
    /// directory).
    pub fn admit(&mut self, root: &Path) -> io::Result<()> {
        let canonical = std::fs::canonicalize(root)?;
        if self.admitted.contains_key(&canonical) {
            return Ok(());
        }
        let anchor = WorkspaceAnchor::open(&canonical)?;
        self.admitted.insert(canonical, anchor);
        Ok(())
    }

    /// Authorise a verb against `workspace_root`, returning the held read
    /// [`WorkspaceAnchor`] iff the root is authorised on this connection.
    ///
    /// - `Open` mode: a not-yet-admitted root is auto-admitted (first-touch)
    ///   and its anchor returned.
    /// - `Allowlist` mode: an already-admitted root returns its anchor; an
    ///   unadmitted-but-allowed root is admitted then returned; an unlisted
    ///   root returns `Ok(None)` (refused).
    ///
    /// # Errors
    /// Propagates the open/canonicalise error when admission is attempted for
    /// a root that should be admissible but cannot be opened.
    pub fn authorise(&mut self, workspace_root: &Path) -> io::Result<Option<&WorkspaceAnchor>> {
        // A root that does not resolve is never authorised — but this is a
        // refusal, not a hard error (the client named a vanished path).
        let Ok(canonical) = std::fs::canonicalize(workspace_root) else {
            return Ok(None);
        };

        if !self.admitted.contains_key(&canonical) {
            let admissible = match self.mode {
                AdmissionMode::Open => true,
                AdmissionMode::Allowlist => self.allow.permits(&canonical),
            };
            if !admissible {
                return Ok(None);
            }
            let anchor = WorkspaceAnchor::open(&canonical)?;
            self.admitted.insert(canonical.clone(), anchor);
        }

        Ok(self.admitted.get(&canonical))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_paths_authorised_for_session_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("marker"), b"present").expect("write marker");
        let mut roots = AdmittedRoots::new_open();
        roots.admit(tmp.path()).expect("admit root");

        let anchor = roots
            .authorise(tmp.path())
            .expect("authorise io")
            .expect("an admitted root is authorised");
        // A live, readable anchor: it reads a file beneath the held root.
        assert_eq!(
            anchor.read_rel("marker").expect("read via anchor"),
            b"present"
        );
    }

    #[test]
    fn validate_paths_refused_for_unrelated_root_in_allowlist_mode() {
        let allowed = tempfile::tempdir().expect("tempdir");
        let other = tempfile::tempdir().expect("tempdir");

        let mut roots = AdmittedRoots::new_allowlist([allowed.path().to_path_buf()]);
        // The allowed root authorises...
        assert!(
            roots.authorise(allowed.path()).expect("io").is_some(),
            "allowlisted root must authorise"
        );
        // ...an unrelated root does not, and is NOT silently admitted.
        assert!(
            roots.authorise(other.path()).expect("io").is_none(),
            "an unlisted root must be refused in allowlist mode"
        );
        let canonical_other = std::fs::canonicalize(other.path()).unwrap();
        assert!(!roots.is_admitted(&canonical_other));
    }

    #[test]
    fn root_set_grows_on_first_touch_in_open_mode() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut roots = AdmittedRoots::new_open();
        let canonical = std::fs::canonicalize(tmp.path()).unwrap();

        assert!(
            !roots.is_admitted(&canonical),
            "not admitted before first touch"
        );
        // First authorise auto-admits in open mode.
        assert!(roots.authorise(tmp.path()).expect("io").is_some());
        assert!(roots.is_admitted(&canonical), "admitted after first touch");
    }

    #[test]
    fn admission_pins_the_anchor_identity_across_calls() {
        // The anchor is opened once and reused — re-authorising the same root
        // returns the same held anchor, not a freshly re-resolved one (C2).
        // Identity is checked by address: the second authorise must hand back
        // the exact same stored `WorkspaceAnchor` (no second open).
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut roots = AdmittedRoots::new_open();

        let first = std::ptr::from_ref(roots.authorise(tmp.path()).expect("io").unwrap()) as usize;
        let second = std::ptr::from_ref(roots.authorise(tmp.path()).expect("io").unwrap()) as usize;
        assert_eq!(
            first, second,
            "the held anchor is pinned at first admission"
        );
    }

    #[test]
    fn allow_policy_prefix_permits_subtree_not_sibling() {
        // A single `prefix` entry confines a whole subtree, but the
        // component-wise `starts_with` must not leak to a sibling whose name
        // merely shares a textual prefix.
        let policy = AllowPolicy::new(
            [PathBuf::from("/srv/exact")],
            [PathBuf::from("/home/op/projects")],
        );
        assert!(
            policy.permits(Path::new("/srv/exact")),
            "exact root permitted"
        );
        assert!(
            policy.permits(Path::new("/home/op/projects")),
            "the prefix root itself is permitted"
        );
        assert!(
            policy.permits(Path::new("/home/op/projects/foo/bar")),
            "a root beneath the prefix is permitted"
        );
        assert!(
            !policy.permits(Path::new("/home/op/projects-other")),
            "a textual-prefix sibling is NOT permitted (component-wise match)"
        );
        assert!(
            !policy.permits(Path::new("/srv/other")),
            "an unlisted root is refused"
        );
    }

    #[test]
    fn unresolvable_root_is_refused_not_errored() {
        let mut roots = AdmittedRoots::new_open();
        let result = roots.authorise(Path::new("/no/such/anvil/root"));
        assert!(
            matches!(result, Ok(None)),
            "a vanished root is a refusal, not a hard error: {result:?}"
        );
    }

    #[test]
    fn no_cwd_in_auth_path() {
        // The workspace is identified by its canonical path + held dirfd, never
        // by the peer's working directory. Guard against a regression that
        // reaches for the procfs per-pid working directory or the process
        // working directory in the admission/read path. Needles are assembled
        // at runtime so this test's own source does not trip the check.
        let admission = include_str!("workspace_admission.rs");
        let path_safety = include_str!("path_safety.rs");
        let proc_needle = format!("/{}/", "proc");
        let cwd_needle = format!("current{}dir", "_");
        for (name, src) in [
            ("workspace_admission", admission),
            ("path_safety", path_safety),
        ] {
            assert!(
                !src.contains(&proc_needle),
                "{name} must not consult the procfs per-pid working directory in the auth/read path"
            );
            assert!(
                !src.contains(&cwd_needle),
                "{name} must not consult the process working directory in the auth/read path"
            );
        }
    }
}
