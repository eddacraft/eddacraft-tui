use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::genesis::GenesisAnchor;
use crate::line::{WitnessLine, compute_line_hash};

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("io reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("parse error in {path} at line {line_number}: {source}")]
    Parse {
        path: PathBuf,
        line_number: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "chain break at {path}:{line_number}: prev_line_hash mismatch (expected {expected}, got {actual})"
    )]
    ChainBreak {
        path: PathBuf,
        line_number: usize,
        expected: String,
        actual: String,
    },
    #[error("sequence gap at {path}:{line_number}: expected seq {expected}, got {actual}")]
    SequenceGap {
        path: PathBuf,
        line_number: usize,
        expected: u64,
        actual: u64,
    },
    #[error(
        "unexpected genesis anchor at {path}:{line_number}: a non-first line must reference a SHA-256, not {anchor}"
    )]
    StrayGenesis {
        path: PathBuf,
        line_number: usize,
        anchor: String,
    },
    #[error("first line in {path} does not reference a known genesis anchor: {actual}")]
    UnknownGenesis { path: PathBuf, actual: String },
    /// A merge line cites a `prev_line_hashes[i]` that does not match
    /// any earlier line's canonical hash. The DAG walk cannot anchor
    /// the merge to the referenced parent — the parent's witness is
    /// missing from the chain (or its bytes have drifted).
    #[error(
        "orphan merge parent at {path}:{line_number} (parent index {parent_index}): \
         prev_line_hashes[{parent_index}] = {missing_hash} not found in earlier lines"
    )]
    OrphanMerge {
        path: PathBuf,
        line_number: usize,
        parent_index: usize,
        missing_hash: String,
    },
    /// A merge line's `parent_commits[]` and `prev_line_hashes[]`
    /// arrays disagree in length. Per MLP-005, the two arrays are
    /// indexed in lockstep; a mismatch means the writer produced an
    /// ill-formed record.
    #[error(
        "merge parent arity mismatch at {path}:{line_number}: \
         parent_commits.len() = {parents}, prev_line_hashes.len() = {hashes}"
    )]
    MergeParentArityMismatch {
        path: PathBuf,
        line_number: usize,
        parents: usize,
        hashes: usize,
    },
    /// The legacy linear [`verify_chain`] entry point encountered a
    /// merge line (`parent_commits[]` or `prev_line_hashes[]`
    /// non-empty). The chain is DAG-shaped and must be walked via
    /// [`verify_chain_dag`].
    #[error(
        "legacy linear verifier encountered a merge line at {path}:{line_number} (seq {merge_at_seq}); \
         call verify_chain_dag for DAG-shaped chains"
    )]
    NonLinearChainInLegacyVerifier {
        path: PathBuf,
        line_number: usize,
        merge_at_seq: u64,
    },
    /// First-line `cutoff_commit` does not match the genesis anchor
    /// contract (ADR-037 §D-2 / MLP2-013): `GENESIS-BASELINED` requires
    /// a non-empty cutoff; `GENESIS-FRESH` requires absence.
    #[error(
        "invalid genesis-anchor metadata at {path}:{line_number}: \
         anchor {anchor} expects cutoff_commit {expected}, found {actual}"
    )]
    InvalidGenesisAnchorMetadata {
        path: PathBuf,
        line_number: usize,
        anchor: String,
        /// Human-readable expectation, e.g. `"present"` or `"absent"`.
        expected: &'static str,
        /// Human-readable actual state, e.g. `"present"` or `"absent"`.
        actual: &'static str,
    },
    /// A later line's `project_uuid` differs from the chain's first
    /// walked identity. The witness chain is a single-project ledger
    /// (ADR-036 / line docs): mid-stream identity switches must not
    /// verify as healthy.
    #[error(
        "project identity mismatch at {path}:{line_number}: \
         expected project_uuid {expected}, got {actual}"
    )]
    ProjectUuidMismatch {
        path: PathBuf,
        line_number: usize,
        expected: String,
        actual: String,
    },
}

/// Summary returned by a successful [`verify_chain`] call.
///
/// An empty chain (no files supplied, or every file empty) returns
/// `anchor: None` and `tip_hash: None`. Callers MUST treat `None` as
/// "no anchor walked" rather than confusing it with `Some(Fresh)` —
/// a fresh chain still has a first line whose `prev_line_hash` is
/// `GENESIS-FRESH`, which would be reflected here as `Some(Fresh)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReport {
    /// The genesis anchor the chain begins at. `None` if no lines
    /// were walked.
    pub anchor: Option<GenesisAnchor>,
    /// Total lines walked across all files.
    pub line_count: u64,
    /// The hash of the final line — what the next line's
    /// `prev_line_hash` should equal. `None` if no lines were
    /// walked.
    pub tip_hash: Option<String>,
}

/// Summary returned by a successful [`verify_chain_dag`] call.
///
/// Strict superset of [`ChainReport`]: also records how many merge
/// lines were joined (`merge_count`) so callers can ask whether the
/// chain was strictly linear via [`DagVerification::is_linear`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagVerification {
    /// The genesis anchor the chain begins at. `None` if no lines
    /// were walked.
    pub anchor: Option<GenesisAnchor>,
    /// Total lines walked across all files.
    pub line_count: u64,
    /// The hash of the leaf (terminal) line. `None` if no lines were
    /// walked. Same semantics as [`ChainReport::tip_hash`].
    pub tip_hash: Option<String>,
    /// Number of merge lines (i.e. lines with at least one entry in
    /// `parent_commits[]` / `prev_line_hashes[]`). `0` for a strictly
    /// linear chain.
    pub merge_count: u64,
    /// `seq` of the first merge line encountered, if any. Used by
    /// the legacy [`verify_chain`] wrapper to surface a precise
    /// rejection location.
    pub first_merge_seq: Option<u64>,
    /// `(path, line_number)` of the first merge line, if any. Pinned
    /// to the file-and-line so the legacy wrapper can build a
    /// fully-located error without re-walking.
    pub first_merge_location: Option<(PathBuf, usize)>,
}

impl DagVerification {
    /// `true` when the walk encountered no merge lines.
    pub fn is_linear(&self) -> bool {
        self.merge_count == 0
    }
}

impl From<&DagVerification> for ChainReport {
    fn from(v: &DagVerification) -> Self {
        Self {
            anchor: v.anchor.clone(),
            line_count: v.line_count,
            tip_hash: v.tip_hash.clone(),
        }
    }
}

/// DAG-aware chain verifier (MLP2-011).
///
/// Walks `paths` in order and validates **both** the linear edge
/// (`prev_line_hash` chains to the immediately prior line) and any
/// merge edges (`prev_line_hashes[i]` references the canonical hash
/// of an earlier line). Builds a `line_hash -> seq` index up-front so
/// merge joins can be resolved against any earlier witness in the
/// file set, not just the running tip.
///
/// Detects the same anomalies the legacy linear verifier did
/// (`ChainBreak`, `SequenceGap`, `StrayGenesis`, `UnknownGenesis`)
/// plus:
///
/// - [`VerifyError::OrphanMerge`] — a merge line cites a parent hash
///   that doesn't appear in any earlier line.
/// - [`VerifyError::MergeParentArityMismatch`] — the merge line's
///   `parent_commits[]` and `prev_line_hashes[]` arrays disagree in
///   length.
/// - [`VerifyError::ProjectUuidMismatch`] — a later line's
///   `project_uuid` differs from the first walked line's identity.
///
/// `prev_line_hashes[i] = None` is **not** an error: per
/// [`crate::WitnessLine`] the writer records `None` when a parent
/// had no witnessed history (e.g. a branch that adopted Anvil after
/// branching off). The DAG walk skips those entries — the gap is
/// honest, not orphaned.
pub fn verify_chain_dag(paths: &[&Path]) -> Result<DagVerification, VerifyError> {
    let mut state = WalkState::default();
    for path in paths {
        let contents = fs::read_to_string(path).map_err(|source| VerifyError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        for (idx, raw_line) in contents.lines().enumerate() {
            if raw_line.is_empty() {
                continue;
            }
            walk_line(&mut state, path, idx + 1, raw_line)?;
        }
    }
    Ok(state.into_report())
}

/// Mutable accumulator threaded through [`verify_chain_dag`]'s walk.
///
/// Pulled out so the main function stays under the clippy
/// `too_many_lines` threshold without sacrificing the linear flow
/// that makes the per-line invariants easy to audit.
#[derive(Default)]
struct WalkState {
    tip: Option<String>,
    anchor: Option<GenesisAnchor>,
    total: u64,
    next_expected_seq: u64,
    merge_count: u64,
    first_merge_seq: Option<u64>,
    first_merge_location: Option<(PathBuf, usize)>,
    // `line_hash -> seq` index of every walked line. Used to resolve
    // merge parent references against any earlier witness, not just
    // the running tip.
    index: HashMap<String, u64>,
    /// `project_uuid` of the first walked line. Every subsequent line
    /// must match; mid-chain identity switches are rejected.
    project_uuid: Option<String>,
}

impl WalkState {
    fn into_report(self) -> DagVerification {
        DagVerification {
            anchor: self.anchor,
            line_count: self.total,
            tip_hash: self.tip,
            merge_count: self.merge_count,
            first_merge_seq: self.first_merge_seq,
            first_merge_location: self.first_merge_location,
        }
    }
}

fn walk_line(
    state: &mut WalkState,
    path: &Path,
    line_number: usize,
    raw_line: &str,
) -> Result<(), VerifyError> {
    let line: WitnessLine =
        serde_json::from_str(raw_line).map_err(|source| VerifyError::Parse {
            path: path.to_path_buf(),
            line_number,
            source,
        })?;

    check_linear_edge(state, path, line_number, &line)?;
    check_sequence(state, path, line_number, &line)?;
    check_project_identity(state, path, line_number, &line)?;
    check_merge_edges(state, path, line_number, &line)?;

    // Re-canonicalise the parsed line and update the tip + index. We
    // canonicalise rather than hashing the raw bytes because a
    // downstream tool might re-format the file (e.g. add a trailing
    // space) without altering the semantic record — the verifier
    // should still pass.
    let canonical = line
        .to_canonical_bytes()
        .map_err(|source| VerifyError::Parse {
            path: path.to_path_buf(),
            line_number,
            source,
        })?;
    let line_hash = compute_line_hash(&canonical);
    // First-write wins on hash collision (cryptographically
    // implausible): preserve the earliest seq mapping.
    state.index.entry(line_hash.clone()).or_insert(line.seq);
    state.tip = Some(line_hash);
    state.total += 1;
    Ok(())
}

fn check_linear_edge(
    state: &mut WalkState,
    path: &Path,
    line_number: usize,
    line: &WitnessLine,
) -> Result<(), VerifyError> {
    if state.total == 0 {
        let a = GenesisAnchor::parse(&line.prev_line_hash).ok_or_else(|| {
            VerifyError::UnknownGenesis {
                path: path.to_path_buf(),
                actual: line.prev_line_hash.clone(),
            }
        })?;
        check_genesis_cutoff_contract(path, line_number, &a, line)?;
        state.anchor = Some(a);
        return Ok(());
    }
    if GenesisAnchor::parse(&line.prev_line_hash).is_some() {
        return Err(VerifyError::StrayGenesis {
            path: path.to_path_buf(),
            line_number,
            anchor: line.prev_line_hash.clone(),
        });
    }
    // `tip` is always `Some` after the first line. Surface a
    // chain-break rather than panicking on internal state misuse.
    let expected = state.tip.as_deref().unwrap_or("<tip unset>").to_string();
    if line.prev_line_hash != expected {
        return Err(VerifyError::ChainBreak {
            path: path.to_path_buf(),
            line_number,
            expected,
            actual: line.prev_line_hash.clone(),
        });
    }
    Ok(())
}

/// Enforce a single `project_uuid` across the whole chain.
///
/// The first walked line pins the chain identity. Every later line
/// must carry the same value; a correctly hash-linked mid-stream
/// switch must not verify as healthy (witness is a single-project
/// ledger per ADR-036 / `WitnessLine::project_uuid` docs).
fn check_project_identity(
    state: &mut WalkState,
    path: &Path,
    line_number: usize,
    line: &WitnessLine,
) -> Result<(), VerifyError> {
    match &state.project_uuid {
        None => {
            state.project_uuid = Some(line.project_uuid.clone());
            Ok(())
        }
        Some(expected) if expected == &line.project_uuid => Ok(()),
        Some(expected) => Err(VerifyError::ProjectUuidMismatch {
            path: path.to_path_buf(),
            line_number,
            expected: expected.clone(),
            actual: line.project_uuid.clone(),
        }),
    }
}

/// Enforce ADR-037 §D-2 / MLP2-013 pairing of genesis anchors with
/// `cutoff_commit` on the first chain line only.
///
/// - `GENESIS-BASELINED` must carry a non-empty `cutoff_commit` (the
///   baseline cut-over SHA lives on the line body, not the anchor string).
/// - `GENESIS-FRESH` must omit `cutoff_commit` (greenfield has no cutoff).
fn check_genesis_cutoff_contract(
    path: &Path,
    line_number: usize,
    anchor: &GenesisAnchor,
    line: &WitnessLine,
) -> Result<(), VerifyError> {
    let has_cutoff = line.cutoff_commit.as_ref().is_some_and(|s| !s.is_empty());
    match anchor {
        GenesisAnchor::Baselined if !has_cutoff => Err(VerifyError::InvalidGenesisAnchorMetadata {
            path: path.to_path_buf(),
            line_number,
            anchor: anchor.anchor_string().to_string(),
            expected: "present",
            actual: "absent",
        }),
        GenesisAnchor::Fresh if has_cutoff => Err(VerifyError::InvalidGenesisAnchorMetadata {
            path: path.to_path_buf(),
            line_number,
            anchor: anchor.anchor_string().to_string(),
            expected: "absent",
            actual: "present",
        }),
        _ => Ok(()),
    }
}

fn check_sequence(
    state: &mut WalkState,
    path: &Path,
    line_number: usize,
    line: &WitnessLine,
) -> Result<(), VerifyError> {
    // `WalkState::default()` leaves `next_expected_seq` at `0`; the
    // first line is at seq `1`.
    let expected = state.next_expected_seq.max(1);
    if line.seq != expected {
        return Err(VerifyError::SequenceGap {
            path: path.to_path_buf(),
            line_number,
            expected,
            actual: line.seq,
        });
    }
    state.next_expected_seq = expected.saturating_add(1);
    Ok(())
}

fn check_merge_edges(
    state: &mut WalkState,
    path: &Path,
    line_number: usize,
    line: &WitnessLine,
) -> Result<(), VerifyError> {
    let is_merge = !line.parent_commits.is_empty() || !line.prev_line_hashes.is_empty();
    if !is_merge {
        return Ok(());
    }
    if line.parent_commits.len() != line.prev_line_hashes.len() {
        return Err(VerifyError::MergeParentArityMismatch {
            path: path.to_path_buf(),
            line_number,
            parents: line.parent_commits.len(),
            hashes: line.prev_line_hashes.len(),
        });
    }
    for (parent_index, maybe_hash) in line.prev_line_hashes.iter().enumerate() {
        let Some(h) = maybe_hash else {
            // `None` is an honest gap (parent had no witnessed
            // history); not an orphan.
            continue;
        };
        if !state.index.contains_key(h) {
            return Err(VerifyError::OrphanMerge {
                path: path.to_path_buf(),
                line_number,
                parent_index,
                missing_hash: h.clone(),
            });
        }
    }
    state.merge_count = state.merge_count.saturating_add(1);
    if state.first_merge_seq.is_none() {
        state.first_merge_seq = Some(line.seq);
        state.first_merge_location = Some((path.to_path_buf(), line_number));
    }
    Ok(())
}

/// Walk `paths` (in order) and verify the witness chain integrity.
///
/// Pass archive files **first** (in seq order), then the active file
/// last. The first line of the first file must reference a known
/// genesis anchor; every subsequent line's `prev_line_hash` must
/// equal `compute_line_hash(prior_canonical_bytes)`.
///
/// **Linear-only contract.** Since MLP2-011, this entry point is a
/// thin wrapper around [`verify_chain_dag`] that additionally
/// rejects chains containing any merge line. Use
/// [`verify_chain_dag`] directly for DAG-shaped chains produced by
/// `anvil hook post-merge`.
///
/// Returns `Ok(ChainReport)` when the chain is intact and strictly
/// linear. Errors are detailed enough for an operator to find the
/// broken line without inspecting the file.
#[deprecated(
    since = "0.6.2-beta",
    note = "Use verify_chain_dag instead; verify_chain rejects DAG chains \
            (post-MLP-005 merge witnesses) with NonLinearChainInLegacyVerifier"
)]
pub fn verify_chain(paths: &[&Path]) -> Result<ChainReport, VerifyError> {
    let dag = verify_chain_dag(paths)?;
    if !dag.is_linear() {
        // Take the recorded first-merge location for a precise error.
        let (path, line_number) = dag
            .first_merge_location
            .clone()
            .unwrap_or_else(|| (PathBuf::new(), 0));
        let merge_at_seq = dag.first_merge_seq.unwrap_or_default();
        return Err(VerifyError::NonLinearChainInLegacyVerifier {
            path,
            line_number,
            merge_at_seq,
        });
    }
    Ok(ChainReport::from(&dag))
}

#[cfg(test)]
#[allow(deprecated)] // Tests exercise the linear-only `verify_chain` wrapper deliberately.
mod tests {
    use super::*;
    use crate::genesis::GenesisAnchor;
    use crate::writer::{RolloverPolicy, WitnessWriter};
    use tempfile::TempDir;

    fn line(seq: u64, prev: &str) -> WitnessLine {
        WitnessLine {
            seq,
            scope: "active".to_string(),
            kind: "witness".to_string(),
            prev_line_hash: prev.to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
            commit_sha: Some(format!("c{seq}")),
            parent_commits: Vec::new(),
            prev_line_hashes: Vec::new(),
            agent_tag: None,
            rules_sha: None,
            cutoff_commit: None,
            ts: "2026-05-13T00:00:00Z".to_string(),
            validation_at: "pre-commit".to_string(),
        }
    }

    fn write_chain(writer: &WitnessWriter, count: u64) -> Vec<String> {
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        let mut hashes = vec![prev.clone()];
        for seq in 1..=count {
            let l = line(seq, &prev);
            writer.append(&l).unwrap();
            prev = compute_line_hash(&l.to_canonical_bytes().unwrap());
            hashes.push(prev.clone());
        }
        hashes
    }

    /// Build a witness file containing a linear prefix of `prefix_len`
    /// lines followed by a single merge line whose `prev_line_hashes[]`
    /// references two earlier-line hashes (a "merge-join").
    ///
    /// Returns `(path, per_seq_hash, merge_line_hash)`. `per_seq_hash[i]`
    /// is the canonical hash of the line with `seq == i + 1`.
    fn write_merge_fixture(dir: &TempDir, prefix_len: usize) -> (PathBuf, Vec<String>, String) {
        assert!(prefix_len >= 2, "need at least 2 linear lines to merge");
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        let mut per_seq: Vec<String> = Vec::new();
        for seq in 1..=prefix_len {
            let l = line(seq as u64, &prev);
            writer.append(&l).unwrap();
            let h = compute_line_hash(&l.to_canonical_bytes().unwrap());
            per_seq.push(h.clone());
            prev = h;
        }
        // Append a merge line: cites two earlier-line hashes as parents.
        // For prefix_len == 4 this references seq 4 (first parent =
        // running tip) and seq 2 (the "other branch tip").
        let other_parent = per_seq[prefix_len / 2 - 1].clone();
        let first_parent = per_seq[prefix_len - 1].clone();
        let merge_seq = (prefix_len + 1) as u64;
        let mut merge = line(merge_seq, &prev);
        merge.parent_commits = vec![
            format!("commit-{merge_seq}-A"),
            format!("commit-{merge_seq}-B"),
        ];
        merge.prev_line_hashes = vec![Some(first_parent), Some(other_parent)];
        writer.append(&merge).unwrap();
        let merge_hash = compute_line_hash(&merge.to_canonical_bytes().unwrap());
        per_seq.push(merge_hash.clone());

        (writer.active_path(), per_seq, merge_hash)
    }

    #[test]
    fn verify_passes_on_clean_chain() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        write_chain(&writer, 5);
        let report = verify_chain(&[writer.active_path().as_path()]).unwrap();
        assert_eq!(report.line_count, 5);
        assert_eq!(report.anchor, Some(GenesisAnchor::Fresh));
        assert!(report.tip_hash.is_some());
    }

    #[test]
    fn verify_empty_chain_yields_none_anchor() {
        let dir = TempDir::new().unwrap();
        let empty = dir.path().join("empty.ndjson");
        std::fs::write(&empty, "").unwrap();
        let report = verify_chain(&[empty.as_path()]).unwrap();
        assert_eq!(report.line_count, 0);
        assert_eq!(report.anchor, None);
        assert_eq!(report.tip_hash, None);
    }

    #[test]
    fn verify_detects_tampered_payload() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        write_chain(&writer, 3);

        // Tamper with the first line's payload — change commit_sha.
        let active = writer.active_path();
        let mut contents = fs::read_to_string(&active).unwrap();
        contents = contents.replace("\"c1\"", "\"c1-evil\"");
        fs::write(&active, contents).unwrap();

        let err = verify_chain(&[active.as_path()]).unwrap_err();
        assert!(matches!(err, VerifyError::ChainBreak { .. }), "got {err:?}");
    }

    #[test]
    fn verify_detects_dropped_line() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        write_chain(&writer, 3);

        // Drop the middle line.
        let active = writer.active_path();
        let contents = fs::read_to_string(&active).unwrap();
        let kept: Vec<&str> = contents
            .lines()
            .enumerate()
            .filter(|(i, _)| *i != 1)
            .map(|(_, l)| l)
            .collect();
        fs::write(&active, kept.join("\n") + "\n").unwrap();

        let err = verify_chain(&[active.as_path()]).unwrap_err();
        // Either ChainBreak (hash points to dropped line) or
        // SequenceGap (seq 1 -> seq 3) is acceptable; both flag the
        // anomaly. The verifier checks the chain first.
        assert!(
            matches!(
                err,
                VerifyError::ChainBreak { .. } | VerifyError::SequenceGap { .. }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn verify_detects_unknown_genesis() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("witness.ndjson");
        let mut l = line(1, "not-a-genesis-anchor");
        // Use a 64-char hex string so the line "looks" like a
        // normal-chain reference rather than an anchor.
        l.prev_line_hash = "1234567890abcdef".repeat(4);
        let mut bytes = l.to_canonical_bytes().unwrap();
        bytes.push(b'\n');
        fs::write(&path, bytes).unwrap();

        let err = verify_chain(&[path.as_path()]).unwrap_err();
        assert!(
            matches!(err, VerifyError::UnknownGenesis { .. }),
            "got {err:?}"
        );
    }

    /// MLP2-013: a chain rooted at `GENESIS-BASELINED` (`cutoff_commit`
    /// recorded on the genesis line body) must verify cleanly with
    /// subsequent pre-commit lines chaining off it. Pin that the
    /// verifier accepts both anchor types at the chain root.
    #[test]
    fn verify_accepts_chain_starting_with_genesis_baselined() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let genesis = WitnessLine::genesis(
            &GenesisAnchor::Baselined,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-05-13T00:00:00Z",
            "baseline",
            Some("a3b2ea4ecafef00d".to_string()),
        );
        writer.append(&genesis).unwrap();
        let mut prev = compute_line_hash(&genesis.to_canonical_bytes().unwrap());
        for seq in 2..=4 {
            let l = line(seq, &prev);
            writer.append(&l).unwrap();
            prev = compute_line_hash(&l.to_canonical_bytes().unwrap());
        }
        let report = verify_chain(&[writer.active_path().as_path()]).unwrap();
        assert_eq!(report.anchor, Some(GenesisAnchor::Baselined));
        assert_eq!(report.line_count, 4);
    }

    /// MLP2-013 companion: greenfield `GENESIS-FRESH` adoption (no
    /// cutoff) still verifies cleanly. Pins the two-anchor support
    /// the verifier already exposed; this guards regression as the
    /// genesis call sites widen.
    #[test]
    fn verify_accepts_chain_starting_with_genesis_fresh() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let genesis = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-05-13T00:00:00Z",
            "pre-commit",
            None,
        );
        writer.append(&genesis).unwrap();
        let report = verify_chain(&[writer.active_path().as_path()]).unwrap();
        assert_eq!(report.anchor, Some(GenesisAnchor::Fresh));
        assert_eq!(report.line_count, 1);
    }

    /// Regression: a `GENESIS-BASELINED` first line without `cutoff_commit`
    /// must not verify. The baseline boundary is recorded on the line
    /// body; accepting a missing cutoff reports a healthy chain that
    /// never pinned the cut-over commit.
    #[test]
    fn verify_rejects_baselined_genesis_without_cutoff_commit() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("witness.ndjson");
        let mut genesis = WitnessLine::genesis(
            &GenesisAnchor::Baselined,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-05-13T00:00:00Z",
            "baseline",
            None,
        );
        // Builder allows None for callers that fill fields later; the
        // verifier must still refuse the invalid pairing on disk.
        assert!(genesis.cutoff_commit.is_none());
        let mut bytes = genesis.to_canonical_bytes().unwrap();
        bytes.push(b'\n');
        fs::write(&path, bytes).unwrap();

        let err = verify_chain_dag(&[path.as_path()]).unwrap_err();
        assert!(
            matches!(
                err,
                VerifyError::InvalidGenesisAnchorMetadata {
                    expected: "present",
                    actual: "absent",
                    ..
                }
            ),
            "got {err:?}"
        );
        // Empty-string cutoff is not a present cutoff either.
        genesis.cutoff_commit = Some(String::new());
        let mut bytes = genesis.to_canonical_bytes().unwrap();
        bytes.push(b'\n');
        fs::write(&path, bytes).unwrap();
        let err = verify_chain_dag(&[path.as_path()]).unwrap_err();
        assert!(
            matches!(err, VerifyError::InvalidGenesisAnchorMetadata { .. }),
            "empty cutoff must also fail, got {err:?}"
        );
    }

    /// Regression: a `GENESIS-FRESH` first line must not carry
    /// `cutoff_commit`. Greenfield adoption has no baseline boundary;
    /// a spurious cutoff would invent one.
    #[test]
    fn verify_rejects_fresh_genesis_with_cutoff_commit() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("witness.ndjson");
        let mut genesis = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-05-13T00:00:00Z",
            "pre-commit",
            None,
        );
        genesis.cutoff_commit = Some("a3b2ea4ecafef00d".to_string());
        let mut bytes = genesis.to_canonical_bytes().unwrap();
        bytes.push(b'\n');
        fs::write(&path, bytes).unwrap();

        let err = verify_chain_dag(&[path.as_path()]).unwrap_err();
        assert!(
            matches!(
                err,
                VerifyError::InvalidGenesisAnchorMetadata {
                    expected: "absent",
                    actual: "present",
                    ..
                }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn verify_walks_archive_then_active() {
        // Simulate rollover: archive has lines 1-3, active has 4-5.
        let dir = TempDir::new().unwrap();
        let writer =
            WitnessWriter::open(dir.path(), "active", RolloverPolicy::tight(3, 1_000_000)).unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        let mut rolled_to = None;
        for seq in 1..=5 {
            let l = line(seq, &prev);
            let out = writer.append(&l).unwrap();
            prev = compute_line_hash(&l.to_canonical_bytes().unwrap());
            if let Some(a) = out.rolled_over_to {
                rolled_to = Some(a);
            }
        }
        let archive = rolled_to.unwrap();
        let active = writer.active_path();
        assert!(archive.exists());
        assert!(active.exists());

        let report = verify_chain(&[archive.as_path(), active.as_path()]).unwrap();
        assert_eq!(report.line_count, 5);
    }

    // ─── MLP2-011 DAG-aware verifier tests ────────────────────────

    #[test]
    fn verify_chain_dag_accepts_linear_chain() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        write_chain(&writer, 5);
        let v = verify_chain_dag(&[writer.active_path().as_path()]).unwrap();
        assert_eq!(v.line_count, 5);
        assert_eq!(v.anchor, Some(GenesisAnchor::Fresh));
        assert!(v.is_linear(), "no-merge chain should be linear");
        assert_eq!(v.merge_count, 0);
        assert!(v.tip_hash.is_some());
    }

    #[test]
    fn verify_chain_dag_accepts_merge_join() {
        // Fixture mirroring MLP-005's merge_witness_plan: four linear
        // lines followed by a merge line citing two earlier-line
        // hashes as `prev_line_hashes[]`.
        let dir = TempDir::new().unwrap();
        let (path, per_seq, _merge_hash) = write_merge_fixture(&dir, 4);
        let v = verify_chain_dag(&[path.as_path()]).unwrap();
        assert_eq!(v.line_count, 5, "4 linear + 1 merge");
        assert_eq!(v.merge_count, 1);
        assert!(!v.is_linear());
        assert_eq!(v.first_merge_seq, Some(5));
        // The leaf hash is the merge line's hash, distinct from any
        // earlier-line hash recorded in the index.
        let tip = v.tip_hash.as_deref().unwrap();
        assert!(!per_seq[..per_seq.len() - 1].iter().any(|h| h == tip));
    }

    #[test]
    fn verify_chain_dag_rejects_tamper_at_merge_parent() {
        // Flip a byte in one of the merge line's parent records.
        // The earlier parent (seq 2) is the "other branch" tip. After
        // we tamper with seq 2, the merge line's
        // `prev_line_hashes[1]` (which referenced seq 2's hash) no
        // longer resolves against the index → OrphanMerge.
        let dir = TempDir::new().unwrap();
        let (path, _per_seq, _merge_hash) = write_merge_fixture(&dir, 4);
        let contents = fs::read_to_string(&path).unwrap();
        // Tamper with seq 2's commit_sha. seq 2 is line index 1.
        let tampered = contents.replacen("\"c2\"", "\"c2-evil\"", 1);
        fs::write(&path, tampered).unwrap();

        let err = verify_chain_dag(&[path.as_path()]).unwrap_err();
        // Tamper of seq 2 immediately breaks the linear chain at
        // seq 3 (which references seq 2's hash). The verifier checks
        // the linear edge before the merge edges, so ChainBreak
        // fires first — and its `line_number` points to seq 3.
        match err {
            VerifyError::ChainBreak { line_number, .. } => {
                assert_eq!(
                    line_number, 3,
                    "tamper at seq 2 breaks seq 3's prev_line_hash"
                );
            }
            other => panic!("expected ChainBreak from tamper at merge parent, got {other:?}"),
        }
    }

    #[test]
    fn verify_chain_dag_rejects_orphan_merge_when_parent_hash_unknown() {
        // Construct a merge line whose `prev_line_hashes[1]` cites a
        // hash that has never appeared in the chain. This isolates
        // the orphan-merge detection from any linear-edge tamper.
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        for seq in 1..=3 {
            let l = line(seq, &prev);
            writer.append(&l).unwrap();
            prev = compute_line_hash(&l.to_canonical_bytes().unwrap());
        }
        // Merge line at seq 4: first parent = running tip (valid),
        // second parent = a hash that's NOT in the index.
        let mut merge = line(4, &prev);
        let unknown = "f".repeat(64);
        merge.parent_commits = vec!["commit-A".to_string(), "commit-B".to_string()];
        merge.prev_line_hashes = vec![Some(prev.clone()), Some(unknown.clone())];
        writer.append(&merge).unwrap();

        let err = verify_chain_dag(&[writer.active_path().as_path()]).unwrap_err();
        match err {
            VerifyError::OrphanMerge {
                parent_index,
                missing_hash,
                ..
            } => {
                assert_eq!(parent_index, 1);
                assert_eq!(missing_hash, unknown);
            }
            other => panic!("expected OrphanMerge, got {other:?}"),
        }
    }

    #[test]
    fn verify_chain_dag_accepts_none_parent_hash_as_honest_gap() {
        // `prev_line_hashes[i] = None` means "this parent had no
        // witnessed history" (e.g. branch adopted Anvil after
        // forking). Not an error.
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        for seq in 1..=3 {
            let l = line(seq, &prev);
            writer.append(&l).unwrap();
            prev = compute_line_hash(&l.to_canonical_bytes().unwrap());
        }
        let mut merge = line(4, &prev);
        merge.parent_commits = vec!["commit-A".to_string(), "commit-orphan".to_string()];
        merge.prev_line_hashes = vec![Some(prev.clone()), None];
        writer.append(&merge).unwrap();

        let v = verify_chain_dag(&[writer.active_path().as_path()]).unwrap();
        assert_eq!(v.merge_count, 1);
        assert_eq!(v.line_count, 4);
    }

    #[test]
    fn verify_chain_dag_accepts_all_none_parent_hashes_as_honest_gaps() {
        // Council MINOR (wave 1I review) — pin the contract that a
        // merge line where every `prev_line_hashes[i]` is `None`
        // (i.e. every parent had no witnessed history at fork time)
        // is accepted as a valid honest gap, not rejected as orphan.
        // The linear `prev_line_hash` edge still chains, so chain
        // integrity is preserved even when the DAG-parent linkage
        // is entirely absent. Lock the surface so a future
        // tightening of orphan-merge semantics doesn't silently
        // reject legitimately-adopted branches.
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        for seq in 1..=3 {
            let l = line(seq, &prev);
            writer.append(&l).unwrap();
            prev = compute_line_hash(&l.to_canonical_bytes().unwrap());
        }
        let mut merge = line(4, &prev);
        merge.parent_commits = vec!["commit-A".to_string(), "commit-B".to_string()];
        merge.prev_line_hashes = vec![None, None];
        writer.append(&merge).unwrap();

        let v = verify_chain_dag(&[writer.active_path().as_path()]).unwrap();
        assert_eq!(v.merge_count, 1);
        assert_eq!(v.line_count, 4);
    }

    #[test]
    fn verify_chain_dag_rejects_dropped_line() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        write_chain(&writer, 4);

        // Drop the middle line (seq 2, index 1).
        let active = writer.active_path();
        let contents = fs::read_to_string(&active).unwrap();
        let kept: Vec<&str> = contents
            .lines()
            .enumerate()
            .filter(|(i, _)| *i != 1)
            .map(|(_, l)| l)
            .collect();
        fs::write(&active, kept.join("\n") + "\n").unwrap();

        let err = verify_chain_dag(&[active.as_path()]).unwrap_err();
        assert!(
            matches!(
                err,
                VerifyError::ChainBreak { .. } | VerifyError::SequenceGap { .. }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn verify_chain_dag_rejects_stray_genesis() {
        // Hand-craft a file with two GENESIS-FRESH anchors. The
        // writer's append API enforces monotonic chaining, so we
        // build the file bytes directly.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("witness.ndjson");
        let l1 = line(1, GenesisAnchor::Fresh.anchor_string());
        let mut l2 = line(2, GenesisAnchor::Fresh.anchor_string());
        // Real-chain field still says GENESIS-FRESH to trigger the
        // stray-genesis check (rather than a chain break).
        l2.prev_line_hash = GenesisAnchor::Fresh.anchor_string().to_string();
        let mut bytes = l1.to_ndjson_line().unwrap();
        bytes.extend_from_slice(&l2.to_ndjson_line().unwrap());
        fs::write(&path, bytes).unwrap();

        let err = verify_chain_dag(&[path.as_path()]).unwrap_err();
        assert!(
            matches!(err, VerifyError::StrayGenesis { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn verify_chain_dag_rejects_arity_mismatch_on_merge() {
        // `parent_commits.len() != prev_line_hashes.len()` is a
        // writer-side bug; the verifier must surface it.
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        for seq in 1..=2 {
            let l = line(seq, &prev);
            writer.append(&l).unwrap();
            prev = compute_line_hash(&l.to_canonical_bytes().unwrap());
        }
        let mut merge = line(3, &prev);
        merge.parent_commits = vec!["commit-A".to_string(), "commit-B".to_string()];
        merge.prev_line_hashes = vec![Some(prev.clone())]; // only one!
        writer.append(&merge).unwrap();

        let err = verify_chain_dag(&[writer.active_path().as_path()]).unwrap_err();
        match err {
            VerifyError::MergeParentArityMismatch {
                parents, hashes, ..
            } => {
                assert_eq!(parents, 2);
                assert_eq!(hashes, 1);
            }
            other => panic!("expected MergeParentArityMismatch, got {other:?}"),
        }
    }

    #[test]
    fn verify_chain_deprecation_wraps_dag_for_linear() {
        // verify_chain on a linear chain matches verify_chain_dag's
        // projection.
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        write_chain(&writer, 4);
        let report = verify_chain(&[writer.active_path().as_path()]).unwrap();
        let dag = verify_chain_dag(&[writer.active_path().as_path()]).unwrap();
        assert_eq!(report, ChainReport::from(&dag));
        assert!(dag.is_linear());
    }

    #[test]
    fn verify_chain_rejects_chain_containing_a_merge() {
        // The legacy linear contract: verify_chain must reject any
        // chain whose DAG walk encounters a merge line.
        let dir = TempDir::new().unwrap();
        let (path, _per_seq, _merge_hash) = write_merge_fixture(&dir, 4);
        let err = verify_chain(&[path.as_path()]).unwrap_err();
        match err {
            VerifyError::NonLinearChainInLegacyVerifier { merge_at_seq, .. } => {
                assert_eq!(merge_at_seq, 5);
            }
            other => panic!("expected NonLinearChainInLegacyVerifier, got {other:?}"),
        }
    }

    /// Regression (clawpatch witness project-identity finding): a
    /// hash-intact chain that switches `project_uuid` mid-stream must
    /// not verify. The first walked line pins the chain identity.
    #[test]
    fn verify_chain_dag_rejects_project_uuid_switch_mid_chain() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("witness.ndjson");
        let uuid_a = "01997e4a-1b2c-7345-8901-abcdef123456";
        let uuid_b = "01997e4a-ffff-7345-8901-abcdef123456";

        let l1 = line(1, GenesisAnchor::Fresh.anchor_string());
        assert_eq!(l1.project_uuid, uuid_a);
        let h1 = compute_line_hash(&l1.to_canonical_bytes().unwrap());

        let mut l2 = line(2, &h1);
        l2.project_uuid = uuid_b.to_string();

        let mut bytes = l1.to_ndjson_line().unwrap();
        bytes.extend_from_slice(&l2.to_ndjson_line().unwrap());
        fs::write(&path, bytes).unwrap();

        let err = verify_chain_dag(&[path.as_path()]).unwrap_err();
        match err {
            VerifyError::ProjectUuidMismatch {
                line_number,
                expected,
                actual,
                ..
            } => {
                assert_eq!(line_number, 2);
                assert_eq!(expected, uuid_a);
                assert_eq!(actual, uuid_b);
            }
            other => panic!("expected ProjectUuidMismatch, got {other:?}"),
        }
    }

    /// Companion: a multi-line chain that keeps a constant
    /// `project_uuid` still verifies (identity enforcement must not
    /// break healthy chains).
    #[test]
    fn verify_chain_dag_accepts_constant_project_uuid() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        write_chain(&writer, 4);
        let v = verify_chain_dag(&[writer.active_path().as_path()]).unwrap();
        assert_eq!(v.line_count, 4);
        assert_eq!(v.anchor, Some(GenesisAnchor::Fresh));
    }
}
