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
    #[error("chain break at {path}:{line_number}: prev_line_hash mismatch (expected {expected}, got {actual})")]
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
    #[error("unexpected genesis anchor at {path}:{line_number}: a non-first line must reference a SHA-256, not {anchor}")]
    StrayGenesis {
        path: PathBuf,
        line_number: usize,
        anchor: String,
    },
    #[error("first line in {path} does not reference a known genesis anchor: {actual}")]
    UnknownGenesis { path: PathBuf, actual: String },
}

/// Summary returned by a successful [`verify_chain`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReport {
    /// The genesis anchor the chain begins at.
    pub anchor: GenesisAnchor,
    /// Total lines walked across all files.
    pub line_count: u64,
    /// The hash of the final line — what the next line's
    /// `prev_line_hash` should equal.
    pub tip_hash: String,
}

/// Walk `paths` (in order) and verify the witness chain integrity.
///
/// Pass archive files **first** (in seq order), then the active file
/// last. The first line of the first file must reference a known
/// genesis anchor; every subsequent line's `prev_line_hash` must
/// equal `compute_line_hash(prior_canonical_bytes)`.
///
/// Returns `Ok(ChainReport)` when the chain is intact. Errors are
/// detailed enough for an operator to find the broken line without
/// inspecting the file.
pub fn verify_chain(paths: &[&Path]) -> Result<ChainReport, VerifyError> {
    let mut tip: Option<String> = None;
    let mut anchor: Option<GenesisAnchor> = None;
    let mut total: u64 = 0;
    let mut next_expected_seq: u64 = 1;

    for path in paths {
        let contents = fs::read_to_string(path).map_err(|source| VerifyError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        for (idx, raw_line) in contents.lines().enumerate() {
            if raw_line.is_empty() {
                // Skip empty lines (final-newline artefacts).
                continue;
            }
            let line_number = idx + 1;
            let line: WitnessLine = serde_json::from_str(raw_line).map_err(|source| {
                VerifyError::Parse {
                    path: path.to_path_buf(),
                    line_number,
                    source,
                }
            })?;

            // Check the `prev_line_hash` against either the genesis
            // anchor (first line only) or the running tip.
            if total == 0 {
                // First line — must reference a genesis anchor.
                let a = GenesisAnchor::parse(&line.prev_line_hash).ok_or_else(|| {
                    VerifyError::UnknownGenesis {
                        path: path.to_path_buf(),
                        actual: line.prev_line_hash.clone(),
                    }
                })?;
                anchor = Some(a);
            } else {
                // Subsequent lines — must NOT reference a genesis
                // anchor (that'd mean two chain roots), and the
                // recorded hash must equal the running tip.
                if GenesisAnchor::parse(&line.prev_line_hash).is_some() {
                    return Err(VerifyError::StrayGenesis {
                        path: path.to_path_buf(),
                        line_number,
                        anchor: line.prev_line_hash.clone(),
                    });
                }
                let expected = tip.as_deref().expect("tip set after first line").to_string();
                if line.prev_line_hash != expected {
                    return Err(VerifyError::ChainBreak {
                        path: path.to_path_buf(),
                        line_number,
                        expected,
                        actual: line.prev_line_hash.clone(),
                    });
                }
            }

            if line.seq != next_expected_seq {
                return Err(VerifyError::SequenceGap {
                    path: path.to_path_buf(),
                    line_number,
                    expected: next_expected_seq,
                    actual: line.seq,
                });
            }
            next_expected_seq = next_expected_seq.saturating_add(1);

            // Re-canonicalise the parsed line and update the tip.
            // We canonicalise rather than hashing the raw bytes
            // because a downstream tool might re-format the file
            // (e.g. add a trailing space) without altering the
            // semantic record — the verifier should still pass.
            let canonical = line.to_canonical_bytes().map_err(|source| {
                VerifyError::Parse {
                    path: path.to_path_buf(),
                    line_number,
                    source,
                }
            })?;
            tip = Some(compute_line_hash(&canonical));
            total += 1;
        }
    }

    Ok(ChainReport {
        anchor: anchor.unwrap_or(GenesisAnchor::Fresh),
        line_count: total,
        tip_hash: tip.unwrap_or_default(),
    })
}

#[cfg(test)]
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
            agent_tag: None,
            rules_sha: None,
            ts: "2026-05-13T00:00:00Z".to_string(),
            validation_at: "pre-commit".to_string(),
        }
    }

    fn write_chain(writer: &WitnessWriter, count: u64) -> Vec<String> {
        let mut prev = GenesisAnchor::Fresh.anchor_string();
        let mut hashes = vec![prev.clone()];
        for seq in 1..=count {
            let l = line(seq, &prev);
            writer.append(&l).unwrap();
            prev = compute_line_hash(&l.to_canonical_bytes().unwrap());
            hashes.push(prev.clone());
        }
        hashes
    }

    #[test]
    fn verify_passes_on_clean_chain() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        write_chain(&writer, 5);
        let report = verify_chain(&[writer.active_path().as_path()]).unwrap();
        assert_eq!(report.line_count, 5);
        assert_eq!(report.anchor, GenesisAnchor::Fresh);
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
        let kept: Vec<&str> = contents.lines().enumerate().filter(|(i, _)| *i != 1).map(|(_, l)| l).collect();
        fs::write(&active, kept.join("\n") + "\n").unwrap();

        let err = verify_chain(&[active.as_path()]).unwrap_err();
        // Either ChainBreak (hash points to dropped line) or
        // SequenceGap (seq 1 -> seq 3) is acceptable; both flag the
        // anomaly. The verifier checks the chain first.
        assert!(
            matches!(err, VerifyError::ChainBreak { .. } | VerifyError::SequenceGap { .. }),
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
        assert!(matches!(err, VerifyError::UnknownGenesis { .. }), "got {err:?}");
    }

    #[test]
    fn verify_walks_archive_then_active() {
        // Simulate rollover: archive has lines 1-3, active has 4-5.
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(
            dir.path(),
            "active",
            RolloverPolicy::tight(3, 1_000_000),
        )
        .unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string();
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
}
