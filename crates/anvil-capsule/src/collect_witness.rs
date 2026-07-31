//! Collect witness-chain evidence for a capsule commit range (GITGOV).

use std::collections::BTreeSet;
use std::path::Path;

use anvil_witness::{WitnessLine, witness_paths};

use crate::errors::CapsuleError;

/// The collected witness chain plus the PR-relevant `seq` window.
///
/// `ndjson` is the verbatim, whole-chain byte stream written to
/// `witness.ndjson`; `seq_start`/`seq_end` are the manifest range
/// pointers ([`crate::CapsuleRange`]). Both pointers are `None` — and
/// serialise as absent, never `null` — when no witness line attests a
/// commit in the range (an empty range, or a range whose commits were
/// authored without the witnessing hook).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectedWitness {
    /// Verbatim concatenation of every witness segment in walk order
    /// (archives lexicographically, then `active.ndjson`). Empty when
    /// the repo has no witness tree.
    pub ndjson: Vec<u8>,
    /// First witness `seq` attesting a commit in the range, if any.
    pub seq_start: Option<u64>,
    /// Last witness `seq` attesting a commit in the range, if any.
    pub seq_end: Option<u64>,
}

/// Collect the complete witness chain under `repo_root` and locate the
/// `seq` window of lines attesting commits in `range_commits`.
///
/// `range_commits` is the set of full commit SHAs the capsule covers
/// (the `sha` of every entry in the GITGOV-005 `commits.json`). Witness
/// lines carry the same full SHA the enforcing hook resolved via
/// `git rev-parse HEAD`, so membership is exact-string.
///
/// Lines are parsed only to read `seq`/`commit_sha` for the window
/// computation; the chain copied into [`CollectedWitness::ndjson`] is
/// the untouched on-disk bytes. The hash chain itself is **not**
/// verified here — that is the verification engine's job (GITGOV-009),
/// which re-runs [`anvil_witness::verify_chain_dag`] over the collected
/// file.
///
/// # Errors
///
/// [`CapsuleError::Collect`] when a witness segment cannot be read, or
/// carries a line that is not valid canonical witness JSON. A repo with
/// no witness tree is **not** an error — it collects as an empty chain.
pub fn collect_witness(
    repo_root: &Path,
    range_commits: &BTreeSet<String>,
) -> Result<CollectedWitness, CapsuleError> {
    let mut collected = CollectedWitness::default();

    for path in witness_paths(repo_root) {
        let bytes = std::fs::read(&path).map_err(|e| CapsuleError::Collect {
            path: relative(repo_root, &path),
            detail: format!("reading witness segment: {e}"),
        })?;
        if bytes.is_empty() {
            continue;
        }

        // Window pointers come from parsing each line's seq/commit_sha.
        // Split on '\n' (canonical NDJSON terminator); the trailing
        // chunk after the final newline is empty and skipped, matching
        // `verify_chain_dag`'s empty-line handling. A trailing '\r' is
        // stripped so a `\r\n` segment parses here exactly as the
        // verifier's `str::lines()` would see it — the verbatim bytes
        // copied below still keep the original line endings.
        for (offset, raw) in bytes.split(|b| *b == b'\n').enumerate() {
            let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
            if raw.is_empty() {
                continue;
            }
            // `match` rather than `map_err(closure)` so the 1-based line
            // number is read in plain control flow (a closure capture is
            // a CodeQL "unused variable" blind spot).
            let line = match WitnessLine::from_ndjson_line(raw) {
                Ok(line) => line,
                Err(e) => {
                    return Err(CapsuleError::Collect {
                        path: format!("{}:{}", relative(repo_root, &path), offset + 1),
                        detail: format!("parsing witness line: {e}"),
                    });
                }
            };
            if line
                .commit_sha
                .as_deref()
                .is_some_and(|sha| range_commits.contains(sha))
            {
                collected.seq_start =
                    Some(collected.seq_start.map_or(line.seq, |s| s.min(line.seq)));
                collected.seq_end = Some(collected.seq_end.map_or(line.seq, |e| e.max(line.seq)));
            }
        }

        // Verbatim copy; guard a newline boundary so a segment without
        // a trailing newline cannot glue onto the next segment.
        collected.ndjson.extend_from_slice(&bytes);
        if !collected.ndjson.ends_with(b"\n") {
            collected.ndjson.push(b'\n');
        }
    }

    Ok(collected)
}

/// Repo-relative display of a witness segment for error messages,
/// falling back to the full path if it is not under `repo_root`.
fn relative(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_witness::{
        GenesisAnchor, RolloverPolicy, WitnessWriter, compute_line_hash, verify_chain_dag,
    };

    /// A witness line attesting `commit_sha`, chaining off `prev`.
    fn line(seq: u64, prev: &str, commit_sha: Option<&str>) -> WitnessLine {
        WitnessLine {
            seq,
            scope: "active".to_string(),
            kind: "witness".to_string(),
            prev_line_hash: prev.to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
            commit_sha: commit_sha.map(str::to_string),
            parent_commits: Vec::new(),
            prev_line_hashes: Vec::new(),
            agent_tag: None,
            rules_sha: None,
            cutoff_commit: None,
            ts: "2026-06-08T00:00:00Z".to_string(),
            validation_at: "pre-commit".to_string(),
        }
    }

    /// Build a witness tree under `dir/anvil/witness` from `commit_shas`
    /// (one line per entry, `seq` starting at 1), rolling over every
    /// `roll_at` lines so the chain spans archive segments + active.
    fn build_chain(dir: &Path, commit_shas: &[Option<&str>], roll_at: u64) -> WitnessWriter {
        let writer =
            WitnessWriter::open(dir, "active", RolloverPolicy::tight(roll_at, 1_000_000)).unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        for (i, sha) in commit_shas.iter().enumerate() {
            let l = line(i as u64 + 1, &prev, *sha);
            writer.append(&l).unwrap();
            prev = compute_line_hash(&l.to_canonical_bytes().unwrap());
        }
        writer
    }

    fn shas(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn collect_witness_empty_tree_is_present_but_empty() {
        let dir = tempfile::tempdir().unwrap();
        let collected = collect_witness(dir.path(), &shas(&["whatever"])).unwrap();
        assert!(collected.ndjson.is_empty(), "no witness tree → empty chain");
        assert_eq!(collected.seq_start, None);
        assert_eq!(collected.seq_end, None);
    }

    #[test]
    fn collect_witness_concatenates_whole_chain_across_archive_and_active() {
        let dir = tempfile::tempdir().unwrap();
        // 5 lines, rollover every 3 → archive holds seq 1-3, active 4-5.
        let commits = [Some("c1"), Some("c2"), Some("c3"), Some("c4"), Some("c5")];
        build_chain(dir.path(), &commits, 3);

        // Sanity: the tree really did split into archive + active.
        let paths = witness_paths(dir.path());
        assert!(paths.len() >= 2, "expected archive + active, got {paths:?}");

        let collected = collect_witness(dir.path(), &shas(&[])).unwrap();
        let text = std::str::from_utf8(&collected.ndjson).unwrap();
        let line_count = text.lines().filter(|l| !l.is_empty()).count();
        assert_eq!(line_count, 5, "whole chain present, not subset");
    }

    /// The core contract: the collected single file verifies under the
    /// **same** `verify_chain_dag` as the original multi-segment tree,
    /// with identical line count and tip — no re-modelled extract.
    #[test]
    fn collect_witness_is_verify_chain_dag_equivalent_to_segments() {
        let dir = tempfile::tempdir().unwrap();
        let commits = [Some("c1"), Some("c2"), Some("c3"), Some("c4"), Some("c5")];
        build_chain(dir.path(), &commits, 3);

        let segments = witness_paths(dir.path());
        let seg_refs: Vec<&Path> = segments.iter().map(std::path::PathBuf::as_path).collect();
        let from_segments = verify_chain_dag(&seg_refs).unwrap();

        let collected = collect_witness(dir.path(), &shas(&[])).unwrap();
        let single = dir.path().join("collected.ndjson");
        std::fs::write(&single, &collected.ndjson).unwrap();
        let from_single = verify_chain_dag(&[single.as_path()]).unwrap();

        assert_eq!(from_single.line_count, from_segments.line_count);
        assert_eq!(from_single.tip_hash, from_segments.tip_hash);
        assert_eq!(from_single.anchor, from_segments.anchor);
    }

    #[test]
    fn collect_witness_marks_seq_window_of_range_commits() {
        let dir = tempfile::tempdir().unwrap();
        let commits = [Some("c1"), Some("c2"), Some("c3"), Some("c4"), Some("c5")];
        build_chain(dir.path(), &commits, 3);

        // Range covers the middle commits; window is their seq span.
        let collected = collect_witness(dir.path(), &shas(&["c2", "c4"])).unwrap();
        assert_eq!(collected.seq_start, Some(2));
        assert_eq!(collected.seq_end, Some(4));
    }

    #[test]
    fn collect_witness_window_is_none_when_no_range_commit_witnessed() {
        let dir = tempfile::tempdir().unwrap();
        let commits = [Some("c1"), Some("c2"), Some("c3")];
        build_chain(dir.path(), &commits, 10);

        // Whole chain still ships; only the window is absent.
        let collected = collect_witness(dir.path(), &shas(&["unwitnessed-sha"])).unwrap();
        assert_eq!(collected.seq_start, None);
        assert_eq!(collected.seq_end, None);
        assert!(!collected.ndjson.is_empty(), "chain still collected whole");
    }

    /// Bookkeeping lines (rollover/baseline events) carry no
    /// `commit_sha` and must never widen the window.
    #[test]
    fn collect_witness_ignores_lines_without_commit_sha() {
        let dir = tempfile::tempdir().unwrap();
        let commits = [None, Some("c2"), None, Some("c4")];
        build_chain(dir.path(), &commits, 10);

        let collected = collect_witness(dir.path(), &shas(&["c2", "c4"])).unwrap();
        assert_eq!(collected.seq_start, Some(2));
        assert_eq!(collected.seq_end, Some(4));
    }

    /// Present-but-broken evidence fails loudly — a corrupt witness line
    /// is a `Collect` error, never silently dropped from the chain.
    #[test]
    fn collect_witness_corrupt_line_is_collect_error() {
        let dir = tempfile::tempdir().unwrap();
        build_chain(dir.path(), &[Some("c1")], 10);
        let active = dir.path().join("anvil/witness/active.ndjson");
        std::fs::write(&active, "{not valid json\n").unwrap();

        let err = collect_witness(dir.path(), &shas(&[])).unwrap_err();
        assert!(matches!(err, CapsuleError::Collect { .. }), "got {err:?}");
        let msg = err.to_string();
        assert!(msg.contains("active.ndjson"), "names the segment: {msg}");
    }

    #[test]
    fn collect_witness_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let commits = [Some("c1"), Some("c2"), Some("c3"), Some("c4")];
        build_chain(dir.path(), &commits, 3);

        let range = shas(&["c2", "c3"]);
        let first = collect_witness(dir.path(), &range).unwrap();
        let second = collect_witness(dir.path(), &range).unwrap();
        assert_eq!(first, second);
    }

    /// Sparse coverage: range commits at seq 1 and seq 5 with non-range
    /// commits between them. The window is the min/max span `[1, 5]` —
    /// it may enclose lines attesting commits outside the range (the
    /// `CapsuleRange` doc states the window is not exclusively the PR's).
    #[test]
    fn collect_witness_window_spans_non_contiguous_range_commits() {
        let dir = tempfile::tempdir().unwrap();
        let commits = [
            Some("c1"),
            Some("other2"),
            Some("other3"),
            Some("other4"),
            Some("c5"),
        ];
        build_chain(dir.path(), &commits, 3);

        let collected = collect_witness(dir.path(), &shas(&["c1", "c5"])).unwrap();
        assert_eq!(collected.seq_start, Some(1));
        assert_eq!(collected.seq_end, Some(5));
    }

    /// A commit re-witnessed twice (e.g. a retry) appears on two lines;
    /// the window spans the first and last occurrence.
    #[test]
    fn collect_witness_window_spans_duplicate_commit_sha() {
        let dir = tempfile::tempdir().unwrap();
        let commits = [Some("c1"), Some("c2"), Some("c1")];
        build_chain(dir.path(), &commits, 10);

        let collected = collect_witness(dir.path(), &shas(&["c1"])).unwrap();
        assert_eq!(collected.seq_start, Some(1));
        assert_eq!(collected.seq_end, Some(3));
    }

    /// Archive segments present with an existing-but-empty
    /// `active.ndjson` (a freshly rolled-over file): the empty segment
    /// is skipped, the archived chain still collects whole.
    #[test]
    fn collect_witness_skips_empty_active_beside_archive() {
        let dir = tempfile::tempdir().unwrap();
        build_chain(dir.path(), &[Some("c1"), Some("c2"), Some("c3")], 2);
        // Force the present-but-empty active shape.
        let active = dir.path().join("anvil/witness/active.ndjson");
        std::fs::write(&active, b"").unwrap();
        assert!(active.exists() && std::fs::metadata(&active).unwrap().len() == 0);

        let collected = collect_witness(dir.path(), &shas(&["c1"])).unwrap();
        let text = std::str::from_utf8(&collected.ndjson).unwrap();
        assert_eq!(
            text.lines().filter(|l| !l.is_empty()).count(),
            2,
            "archived lines still collected"
        );
        assert_eq!(collected.seq_start, Some(1));
    }

    /// A `\r\n`-terminated segment parses exactly as the verifier's
    /// `str::lines()` would see it — no spurious `Collect` error — and
    /// the original line endings survive into the verbatim bytes.
    #[test]
    fn collect_witness_tolerates_crlf_segment() {
        let dir = tempfile::tempdir().unwrap();
        let witness_dir = dir.path().join("anvil/witness");
        std::fs::create_dir_all(&witness_dir).unwrap();
        let l = line(1, GenesisAnchor::Fresh.anchor_string(), Some("c1"));
        let lf = l.to_ndjson_line().unwrap();
        let crlf = String::from_utf8(lf).unwrap().replace('\n', "\r\n");
        std::fs::write(witness_dir.join("active.ndjson"), &crlf).unwrap();

        let collected = collect_witness(dir.path(), &shas(&["c1"])).unwrap();
        assert_eq!(collected.seq_start, Some(1));
        assert_eq!(collected.seq_end, Some(1));
        assert!(
            collected.ndjson.windows(2).any(|w| w == b"\r\n"),
            "verbatim copy preserves the original CRLF endings"
        );
    }
}
