//! DSV-003 Task 4 (ADR-061 §5): inode-aware change classification.
//!
//! The watcher feed reports a coarse [`ChangeKind`] (`Created`/`Modified`/
//! `Removed`) per path, but that hint alone cannot distinguish an
//! atomic-save (write-temp-then-rename-over, which surfaces as a `Created`
//! event on a path that already existed) from a genuine new file. Getting
//! that wrong would let an atomic-save look like a rename and wrongly
//! invalidate the warm graph.
//!
//! So classification is **identity-first**: a per-path identity table holds
//! `(inode, mtime, size)`, and ground truth (does the path exist now? did it
//! exist before?) decides the [`CanonicalChange`]; the watcher hint only
//! disambiguates the cold case where we never tracked the path. An inode flip
//! on a still-present path is a [`CanonicalChange::ContentModify`], never a
//! rename.
//!
//! Rename *correlation* (pairing a `Delete(from)` with a `Create(to)` that
//! share an inode into a [`CanonicalChange::Rename`]) is deliberately left to
//! the orchestrator (DSV-005); at this layer a rename decomposes into its two
//! endpoints, each classified independently.

use std::collections::HashMap;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use anvil_intercept_rules::ChangeKind;

/// The identity of a path at a point in time, as seen via `lstat`
/// (symlinks are not followed — the link's own identity is recorded).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathIdentity {
    /// Inode number. An inode flip on the same path is the atomic-save tell.
    pub inode: u64,
    /// Modification time (epoch seconds).
    pub mtime: i64,
    /// Size in bytes.
    pub size: u64,
}

impl PathIdentity {
    fn from_metadata(meta: &std::fs::Metadata) -> Self {
        Self {
            inode: meta.ino(),
            mtime: meta.mtime(),
            size: meta.size(),
        }
    }

    /// `lstat` a path into an identity. `Ok(None)` if it does not exist;
    /// `Err` for any other I/O failure (e.g. a permission error the caller
    /// should surface rather than treat as "absent").
    ///
    /// # Errors
    /// Propagates non-`NotFound` I/O errors from `symlink_metadata`.
    pub fn of(path: &Path) -> io::Result<Option<Self>> {
        match std::fs::symlink_metadata(path) {
            Ok(meta) => Ok(Some(Self::from_metadata(&meta))),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }
}

/// The canonical, backing-agnostic shape of a single path change after
/// identity reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalChange {
    /// The path's content changed (covers atomic-save inode flips).
    ContentModify,
    /// The path is newly present.
    Create,
    /// The path is gone.
    Delete,
    /// The path is the *destination* of a rename; `from` is the prior
    /// root-relative path. Produced by the orchestrator's rename correlation
    /// (DSV-005), not by [`classify`] — included here so the invalidation
    /// taxonomy (DSV-003 Task 5) has a type to map.
    Rename {
        /// The prior root-relative path the file was renamed from.
        from: PathBuf,
    },
}

/// Classify a single path's change from its before/after identity and the
/// watcher hint. Identity is ground truth; `raw` only disambiguates the
/// cold case (no prior identity tracked).
///
/// - `now` absent ⇒ [`CanonicalChange::Delete`] (including the `prev` absent
///   case — a `Removed` event for a path we never tracked: it does not exist,
///   so `Delete` is the safe classification).
/// - `now` present, `prev` present ⇒ [`CanonicalChange::ContentModify`]
///   (**including an inode flip** — the atomic-save case).
/// - `now` present, `prev` absent ⇒ `Create` if the hint is `Created`,
///   otherwise `ContentModify` (a pre-existing file we had not yet tracked).
///
/// Callers are responsible for suppressing calls where `prev` and `now` are
/// *identical* — `classify` would report `ContentModify` for an unchanged file.
/// [`IdentityTable::stat_on_validate`] is the canonical pattern (it skips
/// `prev == now` before classifying).
#[must_use]
pub fn classify(
    prev: Option<&PathIdentity>,
    now: Option<&PathIdentity>,
    raw: ChangeKind,
) -> CanonicalChange {
    match (prev, now) {
        (_, None) => CanonicalChange::Delete,
        (Some(_), Some(_)) => CanonicalChange::ContentModify,
        (None, Some(_)) => match raw {
            ChangeKind::Created => CanonicalChange::Create,
            ChangeKind::Modified | ChangeKind::Removed => CanonicalChange::ContentModify,
        },
    }
}

/// A per-workspace table of path identities, keyed by a case-normalised path.
///
/// Case sensitivity is probed once per workspace at startup; on a
/// case-insensitive filesystem the keys are lower-cased so a case-only rename
/// (`Foo.rs` → `foo.rs`) collapses to one entry and reads as a content modify,
/// not a spurious delete+create.
#[derive(Debug, Default)]
pub struct IdentityTable {
    case_insensitive: bool,
    entries: HashMap<String, PathIdentity>,
}

impl IdentityTable {
    /// Build an empty table. `case_insensitive` is the once-probed filesystem
    /// property for this workspace (see [`probe_case_insensitive`]).
    #[must_use]
    pub fn new(case_insensitive: bool) -> Self {
        Self {
            case_insensitive,
            entries: HashMap::new(),
        }
    }

    fn key(&self, path: &str) -> String {
        if self.case_insensitive {
            path.to_lowercase()
        } else {
            path.to_string()
        }
    }

    /// Record (or overwrite) the identity stored for `path`.
    pub fn record(&mut self, path: &str, identity: PathIdentity) {
        let key = self.key(path);
        self.entries.insert(key, identity);
    }

    /// Forget the identity stored for `path` (e.g. after a delete).
    pub fn forget(&mut self, path: &str) {
        let key = self.key(path);
        self.entries.remove(&key);
    }

    /// The identity previously recorded for `path`, if any.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&PathIdentity> {
        self.entries.get(&self.key(path))
    }

    /// Re-stat `rels` (relative to `root`) and reconcile silent drift — a
    /// change the watcher feed missed (lost event, daemon restart). Returns
    /// the detected `(path, change)` deltas and updates the table to match
    /// reality. Paths whose identity is unchanged emit nothing.
    ///
    /// # Errors
    /// Propagates a non-`NotFound` I/O error from stat'ing any path.
    pub fn stat_on_validate(
        &mut self,
        root: &Path,
        rels: &[&str],
    ) -> io::Result<Vec<(String, CanonicalChange)>> {
        let mut drift = Vec::new();
        for &rel in rels {
            let now = PathIdentity::of(&root.join(rel))?;
            let prev = self.get(rel).copied();
            if prev == now {
                continue; // no drift
            }
            let change = classify(prev.as_ref(), now.as_ref(), ChangeKind::Modified);
            match now {
                Some(id) => self.record(rel, id),
                None => self.forget(rel),
            }
            drift.push((rel.to_string(), change));
        }
        Ok(drift)
    }
}

/// Probe whether `root`'s filesystem is case-insensitive, once, by creating a
/// lower-cased probe file and checking whether its upper-cased name resolves
/// to the same inode. Best-effort: any I/O failure defaults to the safe
/// case-sensitive answer (`false`), which never collapses distinct paths.
#[must_use]
pub fn probe_case_insensitive(root: &Path) -> bool {
    let lower = root.join(".anvil-case-probe");
    let upper = root.join(".ANVIL-CASE-PROBE");
    if std::fs::write(&lower, b"").is_err() {
        return false;
    }
    let result = match (PathIdentity::of(&lower), PathIdentity::of(&upper)) {
        (Ok(Some(a)), Ok(Some(b))) => a.inode == b.inode,
        _ => false,
    };
    let _ = std::fs::remove_file(&lower);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(inode: u64, mtime: i64, size: u64) -> PathIdentity {
        PathIdentity { inode, mtime, size }
    }

    #[test]
    fn content_modify_same_inode() {
        let prev = id(5, 100, 10);
        let now = id(5, 200, 12); // same inode, new mtime/size
        assert_eq!(
            classify(Some(&prev), Some(&now), ChangeKind::Modified),
            CanonicalChange::ContentModify
        );
    }

    #[test]
    fn atomic_save_new_inode_is_content_modify() {
        // The load-bearing case: write-temp-then-rename-over yields a NEW
        // inode on a path that already existed, and the watcher often reports
        // it as `Created`. Ground truth (path existed before) wins ⇒ a content
        // modify, NOT a rename or a fresh create.
        let prev = id(5, 100, 10);
        let now = id(9, 200, 14); // inode flipped
        assert_eq!(
            classify(Some(&prev), Some(&now), ChangeKind::Created),
            CanonicalChange::ContentModify
        );
    }

    #[test]
    fn delete_classified() {
        let prev = id(5, 100, 10);
        assert_eq!(
            classify(Some(&prev), None, ChangeKind::Removed),
            CanonicalChange::Delete
        );
    }

    #[test]
    fn create_classified_when_cold_and_hint_is_created() {
        assert_eq!(
            classify(None, Some(&id(7, 1, 1)), ChangeKind::Created),
            CanonicalChange::Create
        );
    }

    #[test]
    fn cold_modify_hint_is_content_modify_not_create() {
        // Pre-existing file we had not tracked yet: a Modified hint with no
        // prior identity must not masquerade as a brand-new Create.
        assert_eq!(
            classify(None, Some(&id(7, 1, 1)), ChangeKind::Modified),
            CanonicalChange::ContentModify
        );
    }

    #[test]
    fn rename_decomposes_to_delete_create() {
        // At this layer a rename is two independent endpoints: the `from`
        // path disappears (Delete) and the `to` path appears (Create).
        // Correlating them into CanonicalChange::Rename is the orchestrator's
        // job (DSV-005), not classify's.
        let from = classify(Some(&id(5, 100, 10)), None, ChangeKind::Removed);
        let to = classify(None, Some(&id(5, 100, 10)), ChangeKind::Created);
        assert_eq!(from, CanonicalChange::Delete);
        assert_eq!(to, CanonicalChange::Create);
    }

    #[test]
    fn case_only_rename_on_insensitive_fs() {
        // On a case-insensitive fs the keys collapse, so recording `Foo.rs`
        // and looking up `foo.rs` hits the same entry — a case-only rename is
        // a content modify of one path, not a delete+create of two.
        let mut table = IdentityTable::new(true);
        table.record("src/Foo.rs", id(5, 100, 10));
        assert_eq!(table.get("src/foo.rs"), Some(&id(5, 100, 10)));

        // On a case-sensitive fs the two names are distinct entries.
        let mut sensitive = IdentityTable::new(false);
        sensitive.record("src/Foo.rs", id(5, 100, 10));
        assert_eq!(sensitive.get("src/foo.rs"), None);
    }

    #[test]
    fn stat_on_validate_detects_drift_without_event() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("a.rs"), b"first").unwrap();

        let mut table = IdentityTable::new(false);
        let baseline = PathIdentity::of(&tmp.path().join("a.rs")).unwrap().unwrap();
        table.record("a.rs", baseline);

        // Mutate the file with no watcher event delivered. Sleep a beat so
        // mtime advances on coarse-resolution clocks; the size change alone is
        // also enough to register drift.
        std::fs::write(tmp.path().join("a.rs"), b"second-and-longer").unwrap();

        let drift = table.stat_on_validate(tmp.path(), &["a.rs"]).unwrap();
        assert_eq!(
            drift,
            vec![("a.rs".to_string(), CanonicalChange::ContentModify)]
        );

        // A second pass with no further change detects nothing (table now
        // matches reality).
        let none = table.stat_on_validate(tmp.path(), &["a.rs"]).unwrap();
        assert!(none.is_empty(), "no drift on an unchanged file: {none:?}");
    }

    #[test]
    fn stat_on_validate_detects_deletion() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("gone.rs"), b"x").unwrap();
        let mut table = IdentityTable::new(false);
        table.record(
            "gone.rs",
            PathIdentity::of(&tmp.path().join("gone.rs"))
                .unwrap()
                .unwrap(),
        );

        std::fs::remove_file(tmp.path().join("gone.rs")).unwrap();
        let drift = table.stat_on_validate(tmp.path(), &["gone.rs"]).unwrap();
        assert_eq!(
            drift,
            vec![("gone.rs".to_string(), CanonicalChange::Delete)]
        );
        assert_eq!(table.get("gone.rs"), None, "table forgets a deleted path");
    }
}
