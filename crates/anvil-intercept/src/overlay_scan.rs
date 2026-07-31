//! GBASE-004: compute a worktree overlay fragment (walk/hash/classify dirty
//! files) for compose against a shared base.

#![cfg(any(unix, windows))]

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::Path;

use anvil_graph_cache::overlay::{ChangedSet, OverlayCoverage, OverlayFragment, classify_changes};
use anvil_graph_cache::snapshot::SnapshotPayload;
use anvil_kernel_types::{FileSymbols, content_hash};

use crate::save_time::SymbolParser;
use crate::workspace_anchor::WorkspaceAnchor;
use crate::workspace_pool::{DosCaps, walk_gitignored};

/// Compute a worktree's live overlay versus `base` (ADR-105 §1 / GBASE-004).
///
/// `base` is a loaded shared-base payload (its
/// [`file_hashes`](SnapshotPayload::file_hashes) table is the authoritative
/// per-file content-hash set the diff reconciles against); `root` is the
/// worktree root; `parser` is the injected [`SymbolParser`]; `caps` are the same
/// [`DosCaps`] the full-scan executor honours.
///
/// Returns an [`OverlayFragment`] **ready to compose** (GBASE-006) — composition
/// itself is out of scope. Deterministic: the same worktree state against the
/// same base yields a structurally identical fragment (sorted walk, sorted
/// classification, no wall-clock, no randomness).
///
/// # Errors
/// Returns the underlying [`io::Error`] when the workspace **anchor cannot be
/// opened** (root missing, not a directory, access denied). This is an
/// **environmental** failure, and the overlay is deliberately *fallible* here so
/// the caller serves the **unmodified base / cold path** rather than acting on a
/// destructive fragment: with no anchor, every base file would read as absent and
/// the whole base would be tombstoned. A read failure on an *individual* file is
/// **not** an error — it is skipped (see the read-skip posture below), because one
/// bad file must never abort the whole overlay.
///
/// # Read-skip posture (walked-but-unreadable files)
/// A file the walk saw but could not read this pass (transient permission /
/// symlink / race) is **skipped entirely** — never hashed, classified,
/// tombstoned, or upserted. It stays in the *walked set* (so a base file is never
/// mistaken for deleted merely because a read blipped) but out of the *hash map*
/// (so it is never classified). The loaded base version keeps composing and the
/// next `compute_overlay` self-heals once the read succeeds; the stale-until-
/// reconcile trust line (ADR-069/-105 §4) protects verdicts in the interim. The
/// count is surfaced on [`OverlayCoverage::skipped_unreadable`] and each skip is
/// logged.
///
/// # Bounded posture
/// When the worktree exceeds the walk's `max_walk_files` cap the fragment is
/// marked **bounded** ([`OverlayCoverage::is_bounded`]) and deletion inference is
/// suppressed — a base file absent from the truncated walk is not assumed
/// deleted (ADR-085 Bounded posture: never a silent over-claim).
pub fn compute_overlay(
    base: &SnapshotPayload,
    root: &Path,
    parser: &dyn SymbolParser,
    caps: &DosCaps,
) -> io::Result<OverlayFragment> {
    // --- worktree side: walk + hash (no parse yet) ---
    // The truncation boundary (which files land under the cap when bounded) rides
    // `walk_gitignored`'s readdir traversal order — inherited from the full-scan
    // executor, not new here; a bounded fragment is honest about coverage but the
    // *identity* of the walked prefix is only as stable as readdir order.
    let walk = walk_gitignored(root, caps.max_walk_depth, caps.max_walk_files);

    // Open the anchor ONCE, up front, and BAIL on failure: with no anchor every
    // base file would look absent and the fragment would tombstone the whole base
    // — a destructive result from an environmental fault. The caller degrades to
    // serving the unmodified base (cold path), exactly as the full-scan executor
    // aborts a scan on anchor-open failure.
    let anchor = WorkspaceAnchor::open(root)?;

    // `worktree_files` = every file the walk saw present on disk (readable or not)
    // — the deletion authority. `worktree_hashes` = the readable subset with a
    // recomputed hash — the classification authority.
    let mut worktree_files: BTreeSet<String> = BTreeSet::new();
    let mut worktree_hashes: BTreeMap<String, u64> = BTreeMap::new();
    let mut skipped_unreadable: u64 = 0;
    for abs in &walk.files {
        let Some(rel) = workspace_relative(root, abs) else {
            continue;
        };
        // Record presence FIRST, before the read: a walked file exists on disk
        // regardless of whether this pass can read it, so it is never a deletion.
        worktree_files.insert(rel.clone());
        match anchor.read_rel(&rel) {
            Ok(bytes) => {
                // Recompute the same GV2-032 key the base producer stamped over the
                // parsed bytes, so a byte-identical file hashes equal to its base
                // entry. The walk already bounds file *count*; individual over-cap
                // files are skipped only at the parse step (below), so hashing every
                // readable walked file keeps deletion inference sound.
                worktree_hashes.insert(rel, content_hash(&bytes));
            }
            Err(err) => {
                // Walked-but-unreadable: skip entirely (least-destructive). It
                // stays in `worktree_files` (never deleted) but out of the hash map
                // (never classified), so the loaded base version keeps composing.
                skipped_unreadable += 1;
                tracing::debug!(
                    target: "anvil_intercept::overlay",
                    workspace_root = %root.display(),
                    file = %rel,
                    error = %err,
                    "overlay skipped a walked-but-unreadable file (base version kept)",
                );
            }
        }
    }

    let coverage = OverlayCoverage {
        walked_files: walk.files.len() as u64,
        total_files: walk.total as u64,
        skipped_unreadable,
    };

    // --- base side: the full file set + the (subset) content-hash table ---
    // Membership is decided by the file SET, not the hash table: a hashless base
    // file (tail-language extractor stamping None) is still in the base, so it
    // must not read as `added`. The hash table only proves *unchangedness* for the
    // files that carry a hash.
    let base_files: BTreeSet<String> = base
        .tracked_files()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let base_hashes: BTreeMap<String, u64> = base.file_hashes().iter().cloned().collect();

    // --- pure diff (content-hash authoritative for hashed files; conservative
    // always-modified for hashless base files; deletion off the WALKED set) ---
    let raw = classify_changes(
        &base_files,
        &base_hashes,
        &worktree_files,
        &worktree_hashes,
        coverage.is_bounded(),
    );

    // --- scoped parse: added + modified only ---
    // Tombstones remove the base shadow of every deletion AND every modification;
    // upserts carry the re-parsed symbols for added + modified files. An added
    // file the parser cannot handle (unsupported/unparseable) is NOT a tracked
    // code file — drop it from the changed set entirely (matches the executor's
    // walk semantics: a file that yields no symbols is invisible downstream). A
    // *modified* file that no longer parses stays in the changed set as a
    // tombstone-only entry (its base symbols must still be removed).
    let mut upserts: Vec<FileSymbols> = Vec::new();
    let mut added: Vec<String> = Vec::new();
    for file in &raw.added {
        if let Some(symbols) = parse_file(&anchor, parser, file, caps.max_parse_bytes) {
            upserts.push(symbols);
            added.push(file.clone());
        }
        // else: unsupported/unparseable new file — not part of the overlay.
    }
    for file in &raw.modified {
        if let Some(symbols) = parse_file(&anchor, parser, file, caps.max_parse_bytes) {
            upserts.push(symbols);
        }
        // else: a base-tracked file that no longer parses — tombstone only.
    }
    upserts.sort_by(|a, b| a.file.cmp(&b.file));

    // Tombstones: deletions ∪ modifications (base shadow removed before compose).
    let mut tombstones: Vec<String> = raw.deleted.clone();
    tombstones.extend(raw.modified.iter().cloned());
    tombstones.sort();
    tombstones.dedup();

    let changed = ChangedSet {
        added,
        modified: raw.modified,
        deleted: raw.deleted,
    };

    Ok(OverlayFragment {
        upserts,
        tombstones,
        changed,
        coverage,
    })
}

/// Read + parse one changed file through the injected parser, honouring the
/// parse-size cap. `None` when the file is unreadable, over the parse cap, or the
/// parser declines it (unsupported/unparseable) — every case the executor's
/// `apply_file` also skips.
fn parse_file(
    anchor: &WorkspaceAnchor,
    parser: &dyn SymbolParser,
    rel: &str,
    max_parse_bytes: usize,
) -> Option<FileSymbols> {
    let bytes = anchor.read_rel(rel).ok()?;
    // DoS parse-size cap (mirrors the full-scan executor's `apply_file`): a file
    // too large to parse is skipped — never truncated into a partial parse.
    if bytes.len() > max_parse_bytes {
        return None;
    }
    parser.parse(Path::new(rel), &bytes)
}

/// The forward-slash, workspace-root-relative form of `abs` under `root` — the
/// exact key the parser assigns symbols under and the base stores file hashes as,
/// so the worktree map and the base table compare apples to apples. Copied from
/// the full-scan executor's identical helper (the key contract is shared). `None`
/// if `abs` is not under `root` or relativises to empty.
fn workspace_relative(root: &Path, abs: &Path) -> Option<String> {
    let rel = abs.strip_prefix(root).ok()?;
    let joined: String = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    (!joined.is_empty()).then_some(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    use anvil_graph_cache::{DependencyGraph, SymbolGraph, update_file};
    use anvil_kernel_types::{ImportEdge, SymbolKind, SymbolNode, TrustLevel, Visibility};

    // ---- A path-stable, content-hashing test parser -----------------------
    //
    // Mirrors `full_scan_executor`'s `LineParser` but — crucially for GBASE-004
    // — stamps `content_hash: Some(content_hash(bytes))`, exactly the GV2-032 key
    // the real language extractors set. That is what lets a base built through
    // `SnapshotPayload::from_graphs` (via `update_file`) and an overlay computed
    // here share hash provenance: a byte-identical file is unchanged on both
    // sides. `export NAME` → a public symbol; `import ./spec` → an import edge;
    // anything else is inert text (still hashed).
    #[derive(Debug, Default)]
    struct LineParser;

    fn stable_id(file: &str, name: &str) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in file.bytes().chain(std::iter::once(0)).chain(name.bytes()) {
            h ^= u64::from(byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    impl SymbolParser for LineParser {
        fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols> {
            // Model a real language parser: decline any file whose extension the
            // parser does not handle (here, only `.ts`), exactly as the kernel
            // parser returns `None` for an unsupported file. This is what makes an
            // unsupported *added* file (e.g. `notes.txt`) drop out of the overlay.
            if path.extension().and_then(|e| e.to_str()) != Some("ts") {
                return None;
            }
            let text = std::str::from_utf8(bytes).ok()?;
            let file = path.to_string_lossy().into_owned();
            let mut symbols = Vec::new();
            let mut imports = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                if let Some(name) = line.strip_prefix("export ") {
                    let name = name.trim();
                    symbols.push(SymbolNode {
                        id: stable_id(&file, name),
                        kind: SymbolKind::Function,
                        name: name.to_string(),
                        visibility: Visibility::Public,
                        file: file.clone(),
                        trust_level: TrustLevel::Unknown,
                        span: None,
                    });
                } else if let Some(spec) = line.strip_prefix("import ") {
                    imports.push(ImportEdge {
                        from_file: file.clone(),
                        to_source: spec.trim().to_string(),
                        line: 0,
                    });
                }
            }
            Some(FileSymbols {
                file,
                symbols,
                imports,
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: Some(content_hash(bytes)),
            })
        }
    }

    /// A parser that declines everything (`None`) — stands in for an unsupported
    /// or unparseable file.
    #[derive(Debug, Default)]
    struct NullParser;
    impl SymbolParser for NullParser {
        fn parse(&self, _path: &Path, _bytes: &[u8]) -> Option<FileSymbols> {
            None
        }
    }

    /// A polyglot parser modelling the real hashed/hashless split: `.ts` files go
    /// through the hash-stamping [`LineParser`]; `.go` files parse to a symbol but
    /// stamp `content_hash: None` — exactly what the tail-language extractors do
    /// via `tail_common::finish`. Non-UTF-8 bytes are declined (`None`), the real
    /// unparseable path. This lets a base fixture carry a *hashless* file that is
    /// still in the base graph's file set.
    #[derive(Debug, Default)]
    struct PolyParser;
    impl SymbolParser for PolyParser {
        fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols> {
            let text = std::str::from_utf8(bytes).ok()?; // non-UTF-8 ⇒ unparseable.
            let ext = path.extension().and_then(|e| e.to_str());
            match ext {
                Some("ts") => LineParser.parse(path, bytes),
                Some("go") => {
                    let file = path.to_string_lossy().into_owned();
                    let symbols = text
                        .lines()
                        .filter_map(|l| l.trim().strip_prefix("export "))
                        .map(|name| SymbolNode {
                            id: stable_id(&file, name.trim()),
                            kind: SymbolKind::Function,
                            name: name.trim().to_string(),
                            visibility: Visibility::Public,
                            file: file.clone(),
                            trust_level: TrustLevel::Unknown,
                            span: None,
                        })
                        .collect();
                    Some(FileSymbols {
                        file,
                        symbols,
                        imports: Vec::new(),
                        reexports: Vec::new(),
                        calls: Vec::new(),
                        calls_partial: false,
                        has_unresolved_dynamic_import: false,
                        // The tail-language gap: no content hash stamped.
                        content_hash: None,
                    })
                }
                _ => None,
            }
        }
    }

    fn write_bytes(root: &Path, rel: &str, body: &[u8]) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, body).expect("write file");
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, body).expect("write file");
    }

    /// Build a base payload from an on-disk fixture via the **real GBASE-001
    /// producer path shape**: parse each file through the same parser and fold it
    /// in with `update_file` (the cold-scan build the producer mirrors), then
    /// `SnapshotPayload::from_graphs`. So the base's `file_hashes` and the
    /// overlay's recomputed hashes share provenance.
    fn base_from_files(
        root: &Path,
        files: &[(&str, &str)],
        parser: &dyn SymbolParser,
    ) -> SnapshotPayload {
        let mut sym = SymbolGraph::new();
        for (rel, body) in files {
            write(root, rel, body);
            if let Some(fs) = parser.parse(Path::new(rel), body.as_bytes()) {
                update_file(&mut sym, fs);
            }
        }
        SnapshotPayload::from_graphs(&sym, &DependencyGraph::new()).expect("base payload builds")
    }

    fn caps() -> DosCaps {
        DosCaps::default()
    }

    fn tombstones(f: &OverlayFragment) -> Vec<String> {
        f.tombstones.clone()
    }

    fn upsert_files(f: &OverlayFragment) -> Vec<String> {
        f.upserts.iter().map(|u| u.file.clone()).collect()
    }

    // ---- (a) clean worktree == base ⇒ empty fragment ----------------------
    #[test]
    fn clean_worktree_equal_to_base_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = LineParser;
        // Build the base from the SAME on-disk files the overlay will walk.
        let base = base_from_files(
            root,
            &[("a.ts", "export a\nimport ./b"), ("b.ts", "export b")],
            &parser,
        );
        let frag = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");
        assert!(
            frag.is_empty(),
            "a worktree identical to its base has an empty overlay"
        );
        assert!(frag.changed.is_empty());
        assert!(!frag.coverage.is_bounded());
    }

    // ---- (b) one modified file ⇒ that file only ---------------------------
    #[test]
    fn single_modified_file_covers_exactly_that_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = LineParser;
        let base = base_from_files(root, &[("a.ts", "export a"), ("b.ts", "export b")], &parser);
        // Modify a.ts on disk only (base still holds the old bytes' hash).
        write(root, "a.ts", "export a\nexport a2");

        let frag = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");
        assert_eq!(frag.changed.modified, vec!["a.ts".to_owned()]);
        assert!(frag.changed.added.is_empty());
        assert!(frag.changed.deleted.is_empty());
        // Add (new symbols) + tombstone-of-base-version for the modified file.
        assert_eq!(upsert_files(&frag), vec!["a.ts".to_owned()]);
        assert_eq!(tombstones(&frag), vec!["a.ts".to_owned()]);
        // The upsert carries the NEW parse (two symbols).
        assert_eq!(frag.upserts[0].symbols.len(), 2);
    }

    // ---- (c) added file ⇒ add only ----------------------------------------
    #[test]
    fn added_file_is_add_only_no_tombstone() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = LineParser;
        let base = base_from_files(root, &[("a.ts", "export a")], &parser);
        write(root, "c.ts", "export c");

        let frag = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");
        assert_eq!(frag.changed.added, vec!["c.ts".to_owned()]);
        assert!(frag.changed.modified.is_empty());
        assert!(frag.changed.deleted.is_empty());
        assert_eq!(upsert_files(&frag), vec!["c.ts".to_owned()]);
        assert!(
            tombstones(&frag).is_empty(),
            "a pure add tombstones nothing"
        );
    }

    // ---- (d) deleted file ⇒ tombstone only --------------------------------
    #[test]
    fn deleted_file_is_tombstone_only_no_upsert() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = LineParser;
        let base = base_from_files(
            root,
            &[("a.ts", "export a"), ("gone.ts", "export gone")],
            &parser,
        );
        // Remove gone.ts from disk; a.ts unchanged.
        std::fs::remove_file(root.join("gone.ts")).unwrap();

        let frag = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");
        assert_eq!(frag.changed.deleted, vec!["gone.ts".to_owned()]);
        assert!(frag.changed.added.is_empty());
        assert!(frag.changed.modified.is_empty());
        assert!(upsert_files(&frag).is_empty(), "a deletion has no upsert");
        assert_eq!(tombstones(&frag), vec!["gone.ts".to_owned()]);
    }

    // ---- (e) mixed: modify + add + delete in one pass ---------------------
    #[test]
    fn mixed_dirty_state_covers_exactly_the_changed_set() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = LineParser;
        let base = base_from_files(
            root,
            &[
                ("keep.ts", "export keep"),
                ("edit.ts", "export edit"),
                ("drop.ts", "export drop"),
            ],
            &parser,
        );
        // keep.ts untouched; edit.ts modified; drop.ts deleted; new.ts added.
        write(root, "edit.ts", "export edit\nexport edit2");
        std::fs::remove_file(root.join("drop.ts")).unwrap();
        write(root, "new.ts", "export fresh");

        let frag = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");
        assert_eq!(frag.changed.added, vec!["new.ts".to_owned()]);
        assert_eq!(frag.changed.modified, vec!["edit.ts".to_owned()]);
        assert_eq!(frag.changed.deleted, vec!["drop.ts".to_owned()]);
        // keep.ts must never appear anywhere.
        assert_eq!(
            upsert_files(&frag),
            vec!["edit.ts".to_owned(), "new.ts".to_owned()]
        );
        assert_eq!(
            tombstones(&frag),
            vec!["drop.ts".to_owned(), "edit.ts".to_owned()]
        );
        assert!(
            !upsert_files(&frag).contains(&"keep.ts".to_owned())
                && !tombstones(&frag).contains(&"keep.ts".to_owned()),
            "an unchanged file is absent from the overlay entirely"
        );
    }

    // ---- (f) determinism: two runs structurally identical -----------------
    #[test]
    fn two_runs_over_same_state_are_structurally_identical() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = LineParser;
        let base = base_from_files(
            root,
            &[
                ("a.ts", "export a"),
                ("b.ts", "export b"),
                ("c.ts", "export c"),
            ],
            &parser,
        );
        write(root, "a.ts", "export a\nexport a2");
        write(root, "d.ts", "export d");
        std::fs::remove_file(root.join("c.ts")).unwrap();

        let f1 = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");
        let f2 = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");
        assert_eq!(f1.changed, f2.changed, "changed set is deterministic");
        assert_eq!(tombstones(&f1), tombstones(&f2));
        assert_eq!(upsert_files(&f1), upsert_files(&f2));
        // Upsert symbol ids are path-stable, so the fragments are byte-comparable
        // on their symbol payloads too.
        for (u1, u2) in f1.upserts.iter().zip(&f2.upserts) {
            let ids1: Vec<u64> = u1.symbols.iter().map(|s| s.id).collect();
            let ids2: Vec<u64> = u2.symbols.iter().map(|s| s.id).collect();
            assert_eq!(ids1, ids2, "identical symbol ids across runs");
            assert_eq!(u1.content_hash, u2.content_hash);
        }
    }

    // ---- (g) gitignored + unsupported files excluded ----------------------
    #[test]
    fn gitignored_and_unsupported_files_are_excluded() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = LineParser;
        let base = base_from_files(root, &[("a.ts", "export a")], &parser);

        // A gitignored dir + file — must never enter the walk.
        write(root, ".gitignore", "ignored/\n");
        write(root, "ignored/secret.ts", "export secret");
        // An unsupported *added* file: present on disk, but the parser declines
        // it. The walk sees it (not gitignored), so it is hashed and classified
        // as "added" — but the scoped parse drops it, so it is NOT in the overlay.
        write(root, "notes.txt", "just prose, no exports");

        let frag = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");
        assert!(
            !frag.changed.added.contains(&"notes.txt".to_owned()),
            "an unsupported added file is not a tracked code change"
        );
        assert!(
            frag.changed.added.iter().all(|f| f != "ignored/secret.ts"),
            "a gitignored file never enters the walk"
        );
        assert!(upsert_files(&frag).is_empty());
    }

    /// The `notes.txt` case, made explicit with a parser that declines *added*
    /// files: even though the hash-diff flags it added, the scoped parse's `None`
    /// removes it from the fragment's changed set.
    #[test]
    fn unparseable_added_file_dropped_from_changed_set() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let base = {
            // Base built with the real (accepting) parser holds a.ts.
            let real = LineParser;
            base_from_files(root, &[("a.ts", "export a")], &real)
        };
        write(root, "b.ts", "export b"); // a genuine add on disk

        // But compute with a parser that declines EVERYTHING: b.ts is hash-added
        // yet parses to None ⇒ dropped from the changed set.
        let frag = compute_overlay(&base, root, &NullParser, &caps()).expect("anchor opens");
        assert!(
            frag.changed.added.is_empty(),
            "a null-parsed add contributes nothing to the overlay"
        );
        assert!(frag.upserts.is_empty());
    }

    // ---- (h) over-cap ⇒ bounded, not silent truncation --------------------
    #[test]
    fn over_walk_cap_is_bounded_and_suppresses_false_deletions() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = LineParser;
        // Base of 8 files.
        let files: Vec<(String, String)> = (0..8)
            .map(|i| (format!("f{i}.ts"), format!("export s{i}")))
            .collect();
        let refs: Vec<(&str, &str)> = files
            .iter()
            .map(|(f, b)| (f.as_str(), b.as_str()))
            .collect();
        let base = base_from_files(root, &refs, &parser);

        // Cap the walk at 3 files: 8 on disk > 3 ⇒ bounded.
        let caps = DosCaps {
            max_walk_files: 3,
            ..DosCaps::default()
        };
        let frag = compute_overlay(&base, root, &parser, &caps).expect("anchor opens");
        assert!(
            frag.coverage.is_bounded(),
            "over-cap walk ⇒ bounded coverage"
        );
        assert_eq!(frag.coverage.walked_files, 3);
        assert_eq!(frag.coverage.total_files, 8);
        // The 5 unwalked base files must NOT be reported as deletions.
        assert!(
            frag.changed.deleted.is_empty(),
            "a bounded walk must not infer deletions from absence"
        );
    }

    // ---- (i) hashless base file, unchanged on disk ⇒ conservative modified ----
    #[test]
    fn hashless_base_file_unchanged_is_reparsed_and_tombstoned() {
        // A tail-language (.go) file in the base carries NO content hash, so its
        // unchangedness cannot be proven — it is conservatively `modified`:
        // re-parsed (upsert) AND base-shadow tombstoned. Composition-equivalent
        // (remove-then-re-add the same symbols), just not free. The .ts sibling,
        // being hashed and unchanged, is correctly excluded.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = PolyParser;
        let base = base_from_files(
            root,
            &[("a.ts", "export a"), ("svc.go", "export Svc")],
            &parser,
        );
        // Nothing edited on disk — svc.go is byte-identical to the base.
        let frag = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");

        // a.ts (hashed, unchanged) excluded; svc.go (hashless) conservative-modified.
        assert!(frag.changed.added.is_empty());
        assert_eq!(frag.changed.modified, vec!["svc.go".to_owned()]);
        assert!(frag.changed.deleted.is_empty());
        // Re-parsed (upsert) AND tombstoned — no staleness, no leak.
        assert_eq!(upsert_files(&frag), vec!["svc.go".to_owned()]);
        assert_eq!(tombstones(&frag), vec!["svc.go".to_owned()]);
        assert_eq!(
            frag.upserts[0].symbols.len(),
            1,
            "svc.go re-parsed to its symbol"
        );
    }

    // ---- (ii) hashless base file, now unparseable ⇒ tombstone-only (no leak) ----
    #[test]
    fn hashless_base_file_becomes_unparseable_is_tombstone_only() {
        // The data-loss edge the classification rule fixes: a hashless base file
        // whose worktree bytes become unparseable. Routed through `modified` (not
        // `added`), so its base shadow IS tombstoned — the base's stale symbols
        // cannot leak into composition. It produces no upsert (unparseable).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = PolyParser;
        let base = base_from_files(root, &[("svc.go", "export Svc")], &parser);
        // Corrupt svc.go to invalid UTF-8 on disk ⇒ the parser declines it.
        write_bytes(root, "svc.go", &[0xff, 0xfe, 0x00, 0x9c]);

        let frag = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");
        assert!(
            frag.changed.added.is_empty(),
            "never an add — that would leak"
        );
        assert_eq!(frag.changed.modified, vec!["svc.go".to_owned()]);
        assert!(
            upsert_files(&frag).is_empty(),
            "an unparseable file yields no upsert"
        );
        assert_eq!(
            tombstones(&frag),
            vec!["svc.go".to_owned()],
            "the base shadow IS tombstoned — no stale-symbol leak"
        );
    }

    // ---- FINDING 1: anchor-open failure ⇒ typed error, never a destructive frag --
    #[test]
    fn anchor_open_failure_returns_error_not_a_whole_base_tombstone() {
        // If the anchor cannot open, an empty worktree map would classify EVERY
        // base file as deleted — a fragment that tombstones the whole base. The
        // computation must instead fail, so the caller serves the unmodified base.
        // Simulate a non-openable root with a path that does not exist.
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("no-such-dir");
        let base = base_from_files(
            tmp.path(),
            &[("a.ts", "export a"), ("b.ts", "export b")],
            &LineParser,
        );
        let result = compute_overlay(&base, &missing, &LineParser, &caps());
        assert!(
            result.is_err(),
            "anchor-open failure ⇒ typed error, never a destructive whole-base tombstone"
        );
    }

    // ---- FINDING 2: walked-but-unreadable base file ⇒ skipped, not tombstoned ----
    #[cfg(unix)]
    #[test]
    fn walked_but_unreadable_base_file_is_skipped_and_recorded_not_tombstoned() {
        // A mode-000 regular file WALKS (is_file) but `read_rel` fails EACCES. It
        // must be skipped: not deleted (it exists on disk), not tombstoned, not
        // upserted — the loaded base version keeps composing — and the skip is
        // recorded on coverage + logged. Deterministic on a non-root runner; under
        // root the mode is bypassed (read would succeed), so skip the assertion
        // there — the pure `classify_changes` test covers the logic uid-agnostically.
        use std::os::unix::fs::PermissionsExt;
        if nix::unistd::geteuid().is_root() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = LineParser;
        let base = base_from_files(
            root,
            &[("a.ts", "export a"), ("locked.ts", "export locked")],
            &parser,
        );
        // Make locked.ts unreadable this pass (present on disk, read fails).
        let locked = root.join("locked.ts");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let frag = compute_overlay(&base, root, &parser, &caps()).expect("anchor opens");

        // Restore perms so TempDir cleanup is unimpeded.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();

        assert_eq!(
            frag.coverage.skipped_unreadable, 1,
            "the unreadable walked file is counted as a skip"
        );
        assert!(
            !frag.changed.deleted.contains(&"locked.ts".to_owned()),
            "a walked-but-unreadable base file exists ⇒ never deleted"
        );
        assert!(
            !tombstones(&frag).contains(&"locked.ts".to_owned()),
            "an unreadable base file is never tombstoned (no stale-base wipe)"
        );
        assert!(
            frag.changed.modified.is_empty() && frag.changed.added.is_empty(),
            "no worktree hash ⇒ never classified; a.ts is unchanged"
        );
        assert!(
            !upsert_files(&frag).contains(&"locked.ts".to_owned()),
            "an unreadable file yields no upsert"
        );
    }

    // GBASE-007 layer (ii): production overlay pipeline vs cold scan parity.
    // Layer (i) golden pins compose shape; this exercises real walk/hash/parse.

    use anvil_graph_cache::{compose, re_resolve_imports};
    use anvil_kernel_types::{EdgeType, SymbolIdentity};
    use std::collections::{BTreeMap, BTreeSet};

    /// Derive the Imports-only cross-file dependency graph — the production base
    /// producer's `derive_dependency_graph` rule (kept in lockstep with the cold
    /// oracle). The base MUST carry this forward map or `compose`'s `BaseReresolve`
    /// cannot re-bind a surviving base file's import of a modified overlay file.
    fn derive_dep(sym: &SymbolGraph) -> DependencyGraph {
        let mut dep = DependencyGraph::new();
        for node in sym.inner().node_weights() {
            for edge in sym.outgoing_edges(node.id) {
                if edge.edge_type != EdgeType::Imports {
                    continue;
                }
                if let (Some(f), Some(t)) = (sym.get_symbol(edge.from), sym.get_symbol(edge.to))
                    && f.file != t.file
                {
                    dep.add_dependency(f.file.clone(), t.file.clone());
                }
            }
        }
        dep
    }

    /// Build a base payload from on-disk files through the **producer shape**
    /// (parse each file, fold with `update_file`, derive the Imports-only dep map),
    /// then round-trip it through the sealed `ANVILGB1` base bytes so it is
    /// recovered exactly as `load_base` would deliver it.
    fn base_payload_with_dep(
        root: &Path,
        files: &[(&str, &str)],
        parser: &dyn SymbolParser,
    ) -> SnapshotPayload {
        let mut sym = SymbolGraph::new();
        for (rel, body) in files {
            write(root, rel, body);
            if let Some(fs) = parser.parse(Path::new(rel), body.as_bytes()) {
                update_file(&mut sym, fs);
            }
        }
        let dep = derive_dep(&sym);
        let payload = SnapshotPayload::from_graphs(&sym, &dep).expect("base payload builds");
        SnapshotPayload::from_base_bytes(&payload.to_base_bytes()).expect("base decodes")
    }

    /// A cold scan of the current on-disk state: parse every present file fresh,
    /// fold with `update_file`, then re-resolve every import so forward references
    /// bind regardless of insertion order — the parity ground truth.
    fn cold_scan(root: &Path, files: &[&str], parser: &dyn SymbolParser) -> SymbolGraph {
        let mut sym = SymbolGraph::new();
        let mut all_imports = Vec::new();
        for rel in files {
            let bytes = std::fs::read(root.join(rel)).expect("read on-disk file");
            if let Some(fs) = parser.parse(Path::new(rel), &bytes) {
                all_imports.extend(fs.imports.iter().cloned());
                update_file(&mut sym, fs);
            }
        }
        re_resolve_imports(&mut sym, &all_imports);
        sym
    }

    /// The `Imports`-only, id-independent edge set (keyed by stable identity).
    fn import_identities(
        sym: &SymbolGraph,
    ) -> BTreeSet<(SymbolIdentity, SymbolIdentity, EdgeType)> {
        let mut identity_of: BTreeMap<u64, SymbolIdentity> = BTreeMap::new();
        let names: BTreeSet<&str> = sym.file_names().collect();
        for file in names {
            let symbols = sym.symbols_in_file(file);
            let identities = SymbolIdentity::for_file_symbols(&symbols);
            for (node, identity) in symbols.iter().zip(identities) {
                identity_of.insert(node.id, identity);
            }
        }
        sym.inner()
            .edge_weights()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .filter_map(|e| {
                Some((
                    identity_of.get(&e.from)?.clone(),
                    identity_of.get(&e.to)?.clone(),
                    e.edge_type,
                ))
            })
            .collect()
    }

    fn dep_view(dep: &DependencyGraph, files: &[&str]) -> BTreeMap<String, Vec<String>> {
        let mut out = BTreeMap::new();
        for f in files {
            let mut targets: Vec<String> = dep
                .dependencies_of(f)
                .iter()
                .map(|s| (*s).to_owned())
                .collect();
            targets.sort();
            if !targets.is_empty() {
                out.insert((*f).to_owned(), targets);
            }
        }
        out
    }

    #[test]
    fn real_pipeline_compose_equals_cold_scan_of_dirty_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let parser = PolyParser;

        // --- base(X): a polyglot committed base (TS hashed + Go hashless) ---
        let payload = base_payload_with_dep(
            root,
            &[
                ("core.ts", "export core"),
                ("widget.ts", "export widget"),
                ("gone.ts", "export gone"),
                ("helper.go", "export Help"), // hashless survivor (unchanged)
                ("svc.go", "export Svc"),     // hashless (unchanged on disk)
                ("consumer.ts", "export consumer\nimport ./widget"),
                ("needsgone.ts", "export needs\nimport ./gone"),
            ],
            &parser,
        );

        // --- dirty the worktree on disk: modify + add + delete ---
        write(root, "widget.ts", "export widget2"); // modify (base importer: consumer)
        write(
            root,
            "feature.ts",
            "export feature\nimport ./core\nimport ./widget\nimport ./mid",
        );
        write(root, "mid.ts", "export mid\nimport ./core"); // multi-hop feature→mid→core
        std::fs::remove_file(root.join("gone.ts")).unwrap(); // delete (base importer: needsgone)

        // --- REAL overlay computation (walk + hash + classify + scoped parse) ---
        // Clone the loaded payload up front so a second determinism run can compose
        // the same base again (compose takes the payload by value).
        let payload_rerun = payload.clone();
        let fragment = compute_overlay(&payload, root, &parser, &caps()).expect("anchor opens");

        // Sanity: the real classifier saw the whole combined change shape, INCLUDING
        // the hashless (.go) files routed conservative-modified (GBASE-004 note).
        assert_eq!(fragment.changed.deleted, vec!["gone.ts".to_owned()]);
        assert!(fragment.changed.added.contains(&"feature.ts".to_owned()));
        assert!(fragment.changed.added.contains(&"mid.ts".to_owned()));
        assert!(fragment.changed.modified.contains(&"widget.ts".to_owned()));
        assert!(
            fragment.changed.modified.contains(&"svc.go".to_owned())
                && fragment.changed.modified.contains(&"helper.go".to_owned()),
            "hashless base files take the conservative always-modified path through compose"
        );

        // --- compose the loaded base with the REAL overlay ---
        let (sym, dep) = compose(payload, &fragment).expect("compose");

        // --- cold scan of the dirtied on-disk state (gone.ts absent) ---
        let combined = [
            "core.ts",
            "widget.ts",
            "consumer.ts",
            "needsgone.ts",
            "helper.go",
            "svc.go",
            "feature.ts",
            "mid.ts",
        ];
        let cold = cold_scan(root, &combined, &parser);

        // Parity on the reconstructable ADR-105 §3 import contract.
        let composed_imports = import_identities(&sym);
        let cold_imports = import_identities(&cold);
        assert_eq!(
            composed_imports, cold_imports,
            "GBASE-007 layer (ii): the REAL pipeline's composed import set must equal a cold scan\n\
             composed = {composed_imports:#?}\ncold = {cold_imports:#?}"
        );
        assert_eq!(
            composed_imports.len(),
            5,
            "must exercise consumer→widget (BaseReresolve), feature→core/widget/mid, mid→core"
        );

        // Dependency graph parity (Imports-only), incl. the base→overlay re-bind.
        assert_eq!(
            dep_view(&dep, &combined),
            dep_view(&derive_dep(&cold), &combined),
            "the real pipeline's composed dependency graph must equal the cold scan's"
        );

        // Determinism: a second run of the SAME real pipeline over the SAME on-disk
        // state composes to the same import set (no wall-clock, no readdir-order
        // leak into identities).
        let fragment_b =
            compute_overlay(&payload_rerun, root, &parser, &caps()).expect("anchor opens");
        let (sym_b, dep_b) = compose(payload_rerun, &fragment_b).expect("compose");
        assert_eq!(
            import_identities(&sym_b),
            composed_imports,
            "the real pipeline is deterministic across runs (import set)"
        );
        assert_eq!(
            dep_view(&dep_b, &combined),
            dep_view(&dep, &combined),
            "the real pipeline is deterministic across runs (dependency graph)"
        );
    }
}
