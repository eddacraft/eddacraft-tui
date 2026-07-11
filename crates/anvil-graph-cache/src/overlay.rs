//! GBASE-004 (ADR-105 §1): the **worktree overlay fragment** — the changed-file
//! diff of a dirty worktree versus a loaded shared base, ready to compose onto
//! that base (GBASE-006).
//!
//! ADR-105 persists the shared, dependency-honest majority of a repo's graph
//! **once per merge-base commit** (the base) and keeps only the small,
//! changed-file remainder worktree-private (the *overlay*). This module owns the
//! **shape** of that overlay and the **pure, content-hash-authoritative diff**
//! that classifies which files differ; the intercept-side
//! [`crate`](../../anvil_intercept/overlay_scan/index.html)-equivalent scoping
//! (walk + parse the changed set through the injected parser) assembles the
//! [`OverlayFragment`] this module defines.
//!
//! # Why content hashes, not git
//!
//! A dirty working tree has **no committed state to trust** (ADR-105 §1) — the
//! whole point of the overlay is the *uncommitted* delta. So the diff is
//! authoritative on **content hashes**, never a git subprocess: the base's
//! [`SnapshotPayload::file_hashes`](crate::SnapshotPayload::file_hashes) table
//! (the GV2-032 per-file [`anvil_kernel_types::content_hash`] the base producer
//! stamped) is compared file-by-file against the same hash recomputed over the
//! worktree's live bytes. Identical hash ⇒ unchanged (excluded); differing ⇒
//! modified; worktree-only ⇒ added; base-only ⇒ deleted.
//!
//! # Hashed vs hashless base files (exactness posture)
//!
//! Membership in the base is decided by the base's **file set**
//! ([`SnapshotPayload::tracked_files`]), *not* by its hash table — because not
//! every base file carries a hash. TS / Rust / Python stamp a real content hash;
//! the tail-language extractors (Dart / Go / Java / Kotlin / C# / C / C++ / Zig /
//! Wat, via `tail_common::finish`) currently stamp **none**. So:
//!
//! - For a base file **with** a hash, the diff is **exact**: same hash ⇒
//!   unchanged and excluded (never re-parsed).
//! - For a base file **without** a hash, unchangedness cannot be *proven*, so the
//!   posture is **conservative — always `modified`**: it is re-parsed and its
//!   base shadow tombstoned + re-upserted (composition-equivalent, just not free).
//!   This is correct but defeats the never-re-parse-the-unchanged-majority win
//!   for those languages until the kernel-side follow-up stamps hashes in
//!   `tail_common::finish`. Crucially, routing a hashless base file through
//!   `modified` (not `added`) also **preserves the tombstone-on-parse-failure
//!   path**: were it misclassified `added`, a now-unparseable worktree version
//!   would drop with no tombstone and leak the base's stale symbols into
//!   composition forever.
//!
//! # Fragment shape (what GBASE-005/-006 build on)
//!
//! - [`OverlayFragment::upserts`] — the re-parsed [`FileSymbols`] for every
//!   added and modified file, **carrying the parser's raw ids untouched**. The
//!   fragment deliberately does **not** pre-allocate composed ids: GBASE-005 owns
//!   the disjoint base↔overlay id watermark and re-resolves cross-boundary
//!   imports at compose time, so baking ids here would preclude it. Composition
//!   (GBASE-006) applies each upsert through the same `update_file` path the base
//!   producer and daemon use.
//! - [`OverlayFragment::tombstones`] — the base-side files whose base graph
//!   fragment must be **removed** before composing: every deleted file, **and the
//!   base shadow of every modified file** (its stale base symbols, superseded by
//!   the re-parse). Composition removes these from the loaded base, then applies
//!   the upserts.
//! - [`OverlayFragment::changed`] — the exact classified [`ChangedSet`]
//!   provenance the fragment was built from (the GBASE-004 exactness contract).
//! - [`OverlayFragment::coverage`] — the walk's [`OverlayCoverage`], so an
//!   over-cap worktree is an honest **bounded** fragment (ADR-085 Bounded
//!   posture), never a silent truncation that would misread unwalked base files
//!   as deletions.
//!
//! Composition itself (loading the base, applying tombstones + upserts, the
//! disjoint-id re-resolution) is **out of GBASE-004 scope** — this module stops
//! at producing a fragment that is *ready* to compose.

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
/// consulted only to prove *unchangedness*:
///
/// - **added**: in `worktree`, **not in `base_files`** — a genuinely new file.
/// - **modified**: in `base_files` **and** either (a) it has a `base_hashes`
///   entry that differs from the worktree hash, or (b) it has **no** hash entry
///   (hashless base file — unchangedness is unprovable, so re-parse
///   conservatively; this also keeps it on the tombstone-bearing path). A hashed
///   file whose hash matches is excluded (exact, never re-parsed).
/// - **deleted**: in `base_files`, not in `worktree` — **only when `!bounded`**.
///   When the walk was truncated (`bounded == true`) a base file's absence from
///   the (partial) worktree map is not evidence of deletion (it may lie beyond
///   the walk cap), so no deletions are inferred (ADR-085 Bounded posture: never
///   a silent over-claim).
///
/// `base_files` is a [`BTreeSet`] and `worktree` a [`BTreeMap`], so iteration is
/// already sorted; the returned [`ChangedSet`] vectors are sorted and
/// de-duplicated by construction.
#[must_use]
pub fn classify_changes(
    base_files: &BTreeSet<String>,
    base_hashes: &BTreeMap<String, u64>,
    worktree: &BTreeMap<String, u64>,
    bounded: bool,
) -> ChangedSet {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    for (file, wt_hash) in worktree {
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
    // Deletion inference is suppressed on a bounded walk: absence from a truncated
    // worktree map cannot distinguish a deleted file from an unwalked one.
    let deleted = if bounded {
        Vec::new()
    } else {
        base_files
            .iter()
            .filter(|file| !worktree.contains_key(*file))
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
        let changed = classify_changes(&files_of(&base), &base, &wt, false);
        assert!(changed.is_empty(), "identical hashes ⇒ nothing changed");
    }

    #[test]
    fn modified_added_deleted_are_classified_disjointly() {
        let base = map(&[("keep.ts", 1), ("edit.ts", 2), ("gone.ts", 3)]);
        let wt = map(&[("keep.ts", 1), ("edit.ts", 99), ("new.ts", 4)]);
        let changed = classify_changes(&files_of(&base), &base, &wt, false);
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
        let a = classify_changes(&files, &base, &wt, false);
        let b = classify_changes(&files, &base, &wt, false);
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
        let bounded = classify_changes(&files_of(&base), &base, &wt, true);
        assert!(
            bounded.deleted.is_empty(),
            "bounded walk must not infer deletions from absence"
        );
        // Unbounded, the same absence IS a deletion.
        let unbounded = classify_changes(&files_of(&base), &base, &wt, false);
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
        let changed = classify_changes(&base_files, &base_hashes, &wt, false);
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
        let changed = classify_changes(&base_files, &base_hashes, &wt, false);
        assert_eq!(changed.deleted, vec!["gone.go".to_owned()]);
        assert!(changed.added.is_empty() && changed.modified.is_empty());
    }
}
