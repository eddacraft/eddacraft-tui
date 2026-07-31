//! GBASE-004 (ADR-105): worktree overlay fragment — changed files vs a loaded
//! shared base, ready to compose.
//!
//! Holds only the dirty delta; composition is [`compose`].

use std::collections::{BTreeMap, BTreeSet};

use anvil_kernel_types::FileSymbols;

/// The exact set of files that differ between a worktree and its base, classified
/// by how they differ (ADR-105 §1). Every vector is **sorted and de-duplicated**
/// so the classification is deterministic: the same worktree state against the
/// same base yields an identical `ChangedSet`.
///
/// The three classes are disjoint by construction: a file is at most one of
/// added / modified / deleted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangedSet {
    /// Files present in the worktree but **not** in the base's file set — new
    /// files the base never saw. Sorted, workspace-root-relative.
    pub added: Vec<String>,
    /// Files in the base whose worktree content is not provably unchanged — a
    /// hashed file whose hash differs, or a hashless base file (unchangedness
    /// unprovable ⇒ conservative re-parse). Sorted, workspace-root-relative.
    pub modified: Vec<String>,
    /// Files present in the base's file set but **absent** from the worktree
    /// walk — deletions. Sorted, workspace-root-relative. **Empty when the walk
    /// was bounded** (see [`classify_changes`]): absence-from-a-truncated-walk is
    /// not authoritative evidence of deletion.
    pub deleted: Vec<String>,
}

impl ChangedSet {
    /// `true` when nothing differs — a clean worktree identical to its base.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// The total number of changed files across all three classes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }
}

/// The walk-coverage envelope of an overlay computation (ADR-085 Bounded
/// posture). Mirrors the full-scan executor's `ScanCoverage`: a worktree whose
/// file count exceeded the walk cap is covered only for the walked prefix, and
/// the fragment says so rather than silently under-reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayCoverage {
    /// Files the walk actually visited (at most the `max_walk_files` cap).
    pub walked_files: u64,
    /// Total gitignore-filtered files the walk found (a lower bound once the
    /// count ceiling is hit). `> walked_files` exactly when truncated.
    pub total_files: u64,
    /// Files the walk visited but could **not read** this pass (a transient
    /// permission/symlink/race error). They are **skipped entirely** — never
    /// hashed, classified, tombstoned, or upserted — so a read blip cannot make a
    /// base file look deleted (the loaded base version keeps composing; the next
    /// compute self-heals). A non-zero count means the fragment is a partial view
    /// of the worktree; the skipped paths are logged (not carried here, to keep
    /// this envelope `Copy` and path-free).
    pub skipped_unreadable: u64,
}

impl OverlayCoverage {
    /// `true` when the worktree exceeded the walk cap — the overlay covers only
    /// the walked prefix and deletion inference is suppressed.
    #[must_use]
    pub fn is_bounded(&self) -> bool {
        self.total_files > self.walked_files
    }
}

/// A worktree's live overlay: the changed-file diff versus a loaded base, ready
/// to compose (GBASE-006) but **not** yet composed (ADR-105 §1). See the module
/// docs for the field-by-field contract.
#[derive(Debug, Clone)]
pub struct OverlayFragment {
    /// Re-parsed symbols for every added + modified file, sorted by file, with
    /// the parser's raw ids preserved for GBASE-005 to re-key at compose time.
    pub upserts: Vec<FileSymbols>,
    /// Base-side files to remove before composing: deletions **and** the base
    /// shadow of every modified file. Sorted, de-duplicated,
    /// workspace-root-relative.
    pub tombstones: Vec<String>,
    /// The exact changed-set provenance this fragment was built from.
    pub changed: ChangedSet,
    /// The walk-coverage envelope (bounded when the worktree over-ran the cap).
    pub coverage: OverlayCoverage,
}

impl OverlayFragment {
    /// `true` when the worktree is identical to its base — no upserts, no
    /// tombstones, an empty changed set. (A bounded-but-otherwise-clean walk is
    /// still empty: bounding only suppresses *deletion* inference.)
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.upserts.is_empty() && self.tombstones.is_empty() && self.changed.is_empty()
    }
}

/// Classify which files differ between a base and a worktree (ADR-105 §1).
/// **Pure** — no I/O, no parsing; the intercept-side scoping computes the inputs
/// and calls this.
///
/// Membership is decided by `base_files` (the base's full file set — see
/// [`SnapshotPayload::tracked_files`](crate::SnapshotPayload::tracked_files)),
/// while `base_hashes` (a subset — only files whose producer stamped a hash) is
/// consulted only to prove *unchangedness*.
///
/// The worktree side is split into **two** inputs, deliberately:
/// - `worktree_files` — every file the walk **saw present on disk** this pass
///   (readable *or not*). This is the authority for deletion.
/// - `worktree_hashes` — the readable subset with a recomputed content hash. This
///   is the authority for add/modify classification.
///
/// The split is what makes an *unreadable* walked file non-destructive: it is in
/// `worktree_files` (so it is **not** inferred deleted) but absent from
/// `worktree_hashes` (so it is neither added nor modified) — it is skipped
/// entirely, and the loaded base version keeps composing until the next pass.
///
/// - **added**: in `worktree_hashes`, **not in `base_files`** — a genuinely new,
///   readable file. (An unreadable would-be add is simply skipped.)
/// - **modified**: in `worktree_hashes` **and** `base_files`, and either its
///   `base_hashes` entry differs from the worktree hash, or it has **no** hash
///   entry (hashless base file — unchangedness unprovable, so re-parse
///   conservatively; this also keeps it on the tombstone-bearing path). A hashed
///   file whose hash matches is excluded (exact, never re-parsed).
/// - **deleted**: in `base_files`, **not in `worktree_files`** — **only when
///   `!bounded`**. When the walk was truncated (`bounded == true`) a base file's
///   absence from the (partial) walked set is not evidence of deletion (it may
///   lie beyond the walk cap), so no deletions are inferred (ADR-085 Bounded
///   posture: never a silent over-claim).
///
/// All inputs are sorted ([`BTreeSet`]/[`BTreeMap`]), so the returned
/// [`ChangedSet`] vectors are sorted and de-duplicated by construction.
#[must_use]
pub fn classify_changes(
    base_files: &BTreeSet<String>,
    base_hashes: &BTreeMap<String, u64>,
    worktree_files: &BTreeSet<String>,
    worktree_hashes: &BTreeMap<String, u64>,
    bounded: bool,
) -> ChangedSet {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    for (file, wt_hash) in worktree_hashes {
        if !base_files.contains(file) {
            added.push(file.clone());
            continue;
        }
        match base_hashes.get(file) {
            // Hashed and identical ⇒ provably unchanged, excluded (the exact path).
            Some(base_hash) if base_hash == wt_hash => {}
            // Everything else in the base is `modified`: a hashed file whose hash
            // differs (a real edit), OR a hashless base file whose unchangedness
            // cannot be proven (conservative re-parse). Routing the hashless case
            // here — never `added` — keeps it on the tombstone path (no leak).
            _ => modified.push(file.clone()),
        }
    }
    // Deletion uses the WALKED set (present on disk), not the readable-hash subset,
    // so a walked-but-unreadable base file is never inferred deleted. Suppressed on
    // a bounded walk: absence from a truncated walk cannot distinguish a deleted
    // file from an unwalked one.
    let deleted = if bounded {
        Vec::new()
    } else {
        base_files
            .iter()
            .filter(|file| !worktree_files.contains(*file))
            .cloned()
            .collect()
    };
    ChangedSet {
        added,
        modified,
        deleted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, u64)]) -> BTreeMap<String, u64> {
        pairs.iter().map(|(f, h)| ((*f).to_owned(), *h)).collect()
    }

    /// The base file set derived from a hash table's keys — the common case where
    /// every base file carries a hash.
    fn files_of(base: &BTreeMap<String, u64>) -> BTreeSet<String> {
        base.keys().cloned().collect()
    }

    fn set(files: &[&str]) -> BTreeSet<String> {
        files.iter().map(|f| (*f).to_owned()).collect()
    }

    #[test]
    fn clean_worktree_yields_empty_changed_set() {
        let base = map(&[("a.ts", 1), ("b.ts", 2)]);
        let wt = map(&[("a.ts", 1), ("b.ts", 2)]);
        let changed = classify_changes(&files_of(&base), &base, &files_of(&wt), &wt, false);
        assert!(changed.is_empty(), "identical hashes ⇒ nothing changed");
    }

    #[test]
    fn modified_added_deleted_are_classified_disjointly() {
        let base = map(&[("keep.ts", 1), ("edit.ts", 2), ("gone.ts", 3)]);
        let wt = map(&[("keep.ts", 1), ("edit.ts", 99), ("new.ts", 4)]);
        let changed = classify_changes(&files_of(&base), &base, &files_of(&wt), &wt, false);
        assert_eq!(changed.added, vec!["new.ts".to_owned()]);
        assert_eq!(changed.modified, vec!["edit.ts".to_owned()]);
        assert_eq!(changed.deleted, vec!["gone.ts".to_owned()]);
        assert_eq!(changed.len(), 3);
    }

    #[test]
    fn classification_is_sorted_and_deterministic() {
        let base = map(&[("m.ts", 1), ("z.ts", 1), ("a.ts", 1)]);
        let wt = map(&[
            ("m.ts", 2),
            ("z.ts", 2),
            ("a.ts", 2),
            ("c.ts", 9),
            ("b.ts", 9),
        ]);
        let files = files_of(&base);
        let wt_files = files_of(&wt);
        let a = classify_changes(&files, &base, &wt_files, &wt, false);
        let b = classify_changes(&files, &base, &wt_files, &wt, false);
        assert_eq!(a, b, "same inputs ⇒ identical classification");
        assert_eq!(a.added, vec!["b.ts".to_owned(), "c.ts".to_owned()]);
        assert_eq!(
            a.modified,
            vec!["a.ts".to_owned(), "m.ts".to_owned(), "z.ts".to_owned()]
        );
    }

    #[test]
    fn bounded_walk_suppresses_deletion_inference() {
        // A base file absent from a truncated worktree map is NOT a deletion.
        let base = map(&[("a.ts", 1), ("b.ts", 2), ("c.ts", 3)]);
        let wt = map(&[("a.ts", 1)]); // b.ts, c.ts merely beyond the cap.
        let bounded = classify_changes(&files_of(&base), &base, &files_of(&wt), &wt, true);
        assert!(
            bounded.deleted.is_empty(),
            "bounded walk must not infer deletions from absence"
        );
        // Unbounded, the same absence IS a deletion.
        let unbounded = classify_changes(&files_of(&base), &base, &files_of(&wt), &wt, false);
        assert_eq!(
            unbounded.deleted,
            vec!["b.ts".to_owned(), "c.ts".to_owned()]
        );
    }

    #[test]
    fn hashless_base_file_is_modified_never_added() {
        // A base file present in the FILE SET but absent from the hash table (a
        // tail-language file whose extractor stamped None) must classify as
        // `modified` — conservative re-parse + tombstone — never `added`. An
        // `added` misclassification would both defeat exactness AND drop the
        // tombstone path (the data-loss leak this rule fixes).
        let base_files = set(&["hashed.ts", "hashless.go"]);
        let base_hashes = map(&[("hashed.ts", 1)]); // hashless.go carries no hash.
        // Worktree: hashed.ts unchanged; hashless.go present (unchanged bytes,
        // but we cannot prove it) ⇒ conservative modified.
        let wt = map(&[("hashed.ts", 1), ("hashless.go", 42)]);
        let changed = classify_changes(&base_files, &base_hashes, &files_of(&wt), &wt, false);
        assert!(
            changed.added.is_empty(),
            "a hashless base file is not an add"
        );
        assert_eq!(changed.modified, vec!["hashless.go".to_owned()]);
        assert!(changed.deleted.is_empty());
    }

    #[test]
    fn hashless_base_file_absent_from_walk_is_deleted() {
        // Deletion inference uses the full base FILE SET, so a hashless base file
        // gone from the worktree is still a deletion (unbounded).
        let base_files = set(&["a.ts", "gone.go"]);
        let base_hashes = map(&[("a.ts", 1)]);
        let wt = map(&[("a.ts", 1)]);
        let changed = classify_changes(&base_files, &base_hashes, &files_of(&wt), &wt, false);
        assert_eq!(changed.deleted, vec!["gone.go".to_owned()]);
        assert!(changed.added.is_empty() && changed.modified.is_empty());
    }

    #[test]
    fn walked_but_unreadable_base_file_is_neither_deleted_nor_modified() {
        // The read-failure edge: a base file that WAS walked (present on disk) but
        // could not be read is in the walked set yet absent from the hash map. It
        // must be skipped entirely — never deleted (it exists), never modified (no
        // hash to compare) — so the loaded base version keeps composing.
        let base_files = set(&["a.ts", "locked.ts"]);
        let base_hashes = map(&[("a.ts", 1), ("locked.ts", 5)]);
        // Walked set includes locked.ts (present on disk); hash map does not (read
        // failed this pass).
        let worktree_files = set(&["a.ts", "locked.ts"]);
        let worktree_hashes = map(&[("a.ts", 1)]);
        let changed = classify_changes(
            &base_files,
            &base_hashes,
            &worktree_files,
            &worktree_hashes,
            false,
        );
        assert!(
            changed.deleted.is_empty(),
            "a walked-but-unreadable base file exists ⇒ never deleted"
        );
        assert!(
            changed.modified.is_empty(),
            "no worktree hash ⇒ never classified modified"
        );
        assert!(changed.added.is_empty());
    }
}
