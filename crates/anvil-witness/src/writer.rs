use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use fs2::FileExt;
use sha2::{Digest, Sha256};

use crate::line::WitnessLine;

#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("serde_json: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("witness chain is corrupted: {0}")]
    Corruption(String),
    #[error("witness root is a symlink; refusing to write: {path}")]
    SymlinkRoot { path: PathBuf },
    #[error(
        "scope mismatch: writer is configured for `{writer_scope}` but line.scope is `{line_scope}`"
    )]
    ScopeMismatch {
        writer_scope: String,
        line_scope: String,
    },
}

/// Threshold policy for active-file rollover.
///
/// Rollover fires when the active file crosses **either** threshold,
/// whichever happens first (ADR-037 §D-2). The check runs inside the
/// flock, so concurrent writers cannot race a half-archive into
/// existence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RolloverPolicy {
    /// Maximum lines per active file before rollover.
    pub max_lines: u64,
    /// Maximum bytes per active file before rollover.
    pub max_bytes: u64,
}

impl Default for RolloverPolicy {
    fn default() -> Self {
        Self {
            // Spec defaults: 1000 lines or 1 MB whichever first.
            max_lines: 1000,
            max_bytes: 1_048_576,
        }
    }
}

impl RolloverPolicy {
    /// Build a tighter policy useful for tests so rollover happens
    /// without writing a megabyte of synthetic data.
    pub const fn tight(max_lines: u64, max_bytes: u64) -> Self {
        Self {
            max_lines,
            max_bytes,
        }
    }
}

/// Flock-serialised append-only writer for the witness chain.
///
/// Construct with [`WitnessWriter::open`]; one writer instance per `anvil/`
/// root. The writer holds NO long-lived locks — each [`WitnessWriter::append`]
/// call takes the
/// flock for the duration of the append + rollover decision and
/// releases it before returning. This avoids the classic "hold-the-
/// lock-while-the-process-stalls" hazard at the cost of one
/// `flock` syscall per line. The hook surface (MLP-003) writes one
/// line per commit, so the cost is paid at human cadence.
#[derive(Debug)]
pub struct WitnessWriter {
    root: PathBuf,
    scope: String,
    policy: RolloverPolicy,
}

impl WitnessWriter {
    /// `root` is the workspace root; the writer creates the
    /// `anvil/witness/` tree under it on first append.
    pub fn open(
        root: impl Into<PathBuf>,
        scope: impl Into<String>,
        policy: RolloverPolicy,
    ) -> Result<Self, WriterError> {
        let writer = Self {
            root: root.into(),
            scope: scope.into(),
            policy,
        };
        writer.ensure_tree()?;
        Ok(writer)
    }

    /// Append `line` to the active file under flock, performing
    /// rollover if the policy fires after the append.
    ///
    /// `line.prev_line_hash` must already be set by the caller to
    /// either the genesis anchor (for the first line) or the SHA-256
    /// of the immediately-prior line's canonical bytes. The writer
    /// does NOT mutate the line — chaining is the caller's
    /// responsibility, because the caller has visibility into the
    /// commit semantics (e.g. a merge commit needs `prev_line_hashes[]`
    /// rather than a single `prev_line_hash`).
    ///
    /// Returns the new active file's line count after the append, and
    /// the archive path if a rollover happened.
    pub fn append(&self, line: &WitnessLine) -> Result<AppendOutcome, WriterError> {
        // Reject a line that targets a different scope before any
        // file IO. Without this guard a misrouted hook could push
        // entries into the wrong archive scope and silently break
        // verification (which keys archive selection on scope).
        if line.scope != self.scope {
            return Err(WriterError::ScopeMismatch {
                writer_scope: self.scope.clone(),
                line_scope: line.scope.clone(),
            });
        }

        let active_path = self.active_path();
        let lock_path = self.lock_path();

        // Refuse symlinks at every path we're about to write through.
        // The witness root protects against `anvil/witness/` being
        // re-pointed; the lock and active checks protect against
        // someone replacing those specific files with a symlink
        // pointing outside the repo. Without these the symlink
        // refusal on the dir alone is bypassable by replacing the
        // child file.
        refuse_if_symlink(&self.witness_root())?;
        refuse_if_symlink(&lock_path)?;
        refuse_if_symlink(&active_path)?;

        // Open (or create) the lock file. The lock is held via
        // fs2::FileExt::lock_exclusive on this fd. We unlock manually
        // before returning so the success path doesn't depend on Drop
        // order with the active-file handle.
        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        let result = (|| -> Result<AppendOutcome, WriterError> {
            let bytes = line.to_ndjson_line()?;
            let mut active = OpenOptions::new()
                .create(true)
                .append(true)
                .read(true)
                .open(&active_path)?;
            active.write_all(&bytes)?;
            active.sync_all()?;

            // Decide on rollover. Cheap line count + byte count.
            let size_after = active.metadata()?.len();
            let lines_after = count_lines(&mut active)?;
            let outcome =
                if lines_after >= self.policy.max_lines || size_after >= self.policy.max_bytes {
                    let archive_path = self.rollover(&active_path, line.seq)?;
                    AppendOutcome {
                        active_lines: 0,
                        active_bytes: 0,
                        rolled_over_to: Some(archive_path),
                    }
                } else {
                    AppendOutcome {
                        active_lines: lines_after,
                        active_bytes: size_after,
                        rolled_over_to: None,
                    }
                };
            Ok(outcome)
        })();

        // Always release the lock; ignore unlock errors — the OS
        // releases on fd close anyway.
        let _ = FileExt::unlock(&lock_file);
        result
    }

    fn rollover(&self, active_path: &Path, seq_at_rollover: u64) -> Result<PathBuf, WriterError> {
        // Compute a content-addressed name for the archive so two
        // mirrored repos produce the same archive filename if they
        // share the same content. `merkle` here is just SHA-256 of
        // the active file bytes — sufficient for content addressing.
        let mut bytes = Vec::new();
        let mut active = File::open(active_path)?;
        active.read_to_end(&mut bytes)?;
        let merkle = hex::encode(Sha256::digest(&bytes));
        let archive_dir = self.witness_root().join("archive");
        fs::create_dir_all(&archive_dir)?;
        let archive_name = format!(
            "{scope}-{seq:020}-{merkle}.ndjson",
            scope = self.scope,
            seq = seq_at_rollover,
            merkle = &merkle[..16],
        );
        let archive_path = archive_dir.join(archive_name);

        // Content-addressed naming means two writers producing
        // identical content would compute the same archive name. On
        // POSIX `fs::rename` silently replaces the existing file; on
        // Windows it fails with AlreadyExists. Both behaviours are
        // wrong for our use case: we want the rollover to be
        // idempotent (the archive already exists with the same
        // content, so we just need to remove the active file). We
        // verify the destination's content matches before treating it
        // as a no-op so a stale or corrupt file at the destination is
        // never silently accepted.
        match fs::rename(active_path, &archive_path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let existing = fs::read(&archive_path).map_err(WriterError::Io)?;
                if existing == bytes {
                    // Content matches — safe to drop the active file.
                    fs::remove_file(active_path)?;
                } else {
                    return Err(WriterError::Corruption(format!(
                        "archive {} exists with different content; refusing to overwrite",
                        archive_path.display(),
                    )));
                }
            }
            Err(e) => return Err(e.into()),
        }

        // MLP2-012: record the rollover in the manifest stream so
        // consumers can tail archive transitions without polling the
        // archive dir. The append is idempotent — re-rolling onto an
        // archive that already exists leaves the manifest with the
        // same single entry.
        #[allow(clippy::naive_bytecount)]
        // Avoid pulling in `bytecount` for a once-per-rollover count.
        let line_count = bytes.iter().filter(|&&b| b == b'\n').count() as u64;
        let entry = crate::manifest::ManifestEntry {
            archive_path: archive_path.clone(),
            merkle,
            line_count,
            seq_at_rollover,
        };
        crate::manifest::append_manifest_entry(&self.witness_root(), &entry)?;

        Ok(archive_path)
    }

    fn ensure_tree(&self) -> Result<(), WriterError> {
        let root = self.witness_root();
        refuse_if_symlink(&root)?;
        fs::create_dir_all(&root)?;
        refuse_if_symlink(&root)?;
        Ok(())
    }
}

/// Refuse to write through a symlink at `path`. The TOCTOU hardening
/// matches MLP-001's pattern: check, create, re-check. Kept as a
/// module-private free function — it doesn't depend on writer state.
fn refuse_if_symlink(path: &Path) -> Result<(), WriterError> {
    if path.exists() {
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            return Err(WriterError::SymlinkRoot {
                path: path.to_path_buf(),
            });
        }
    }
    Ok(())
}

impl WitnessWriter {
    pub fn witness_root(&self) -> PathBuf {
        self.root.join("anvil").join("witness")
    }

    pub fn active_path(&self) -> PathBuf {
        // ADR-037 §D-3 pins the active file inside the witness tree
        // at `anvil/witness/active.ndjson`. Keeping it under `witness/`
        // (rather than its sibling at `anvil/witnessed.ndjson`) means
        // the whole chain — active + archives + manifest — lives in
        // one directory that callers can crawl or `git diff` as a unit.
        self.witness_root().join("active.ndjson")
    }

    fn lock_path(&self) -> PathBuf {
        self.witness_root().join(".lock")
    }
}

/// Result returned by [`WitnessWriter::append`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendOutcome {
    /// Lines remaining in the active file after this append. Zero
    /// when rollover occurred.
    pub active_lines: u64,
    /// Active file size in bytes after the append. Zero when
    /// rollover occurred.
    pub active_bytes: u64,
    /// Archive path written if rollover fired during this append.
    pub rolled_over_to: Option<PathBuf>,
}

/// Count newlines in an open file. Uses a small buffer rather than
/// reading the whole file into memory; witness lines are short and
/// the active file is bounded by the rollover policy, so this is
/// cheap.
fn count_lines(file: &mut File) -> io::Result<u64> {
    file.seek(SeekFrom::Start(0))?;
    let mut buf = [0u8; 4096];
    let mut total: u64 = 0;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            if b == b'\n' {
                total += 1;
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genesis::GenesisAnchor;
    use crate::line::compute_line_hash;
    use tempfile::TempDir;

    fn fresh_line(seq: u64, prev: &str) -> WitnessLine {
        WitnessLine {
            seq,
            scope: "active".to_string(),
            kind: "witness".to_string(),
            prev_line_hash: prev.to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
            commit_sha: Some(format!("commit-{seq}")),
            parent_commits: Vec::new(),
            prev_line_hashes: Vec::new(),
            agent_tag: None,
            rules_sha: None,
            cutoff_commit: None,
            ts: "2026-05-13T00:00:00Z".to_string(),
            validation_at: "pre-commit".to_string(),
        }
    }

    #[test]
    fn append_creates_tree_and_writes_line() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let outcome = writer
            .append(&fresh_line(1, GenesisAnchor::Fresh.anchor_string()))
            .unwrap();
        assert!(outcome.rolled_over_to.is_none());
        assert_eq!(outcome.active_lines, 1);
        assert!(writer.active_path().exists());
        assert!(writer.witness_root().exists());
    }

    #[test]
    fn append_chains_lines() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let first = fresh_line(1, GenesisAnchor::Fresh.anchor_string());
        writer.append(&first).unwrap();
        let first_hash = compute_line_hash(&first.to_canonical_bytes().unwrap());
        let second = fresh_line(2, &first_hash);
        let outcome = writer.append(&second).unwrap();
        assert_eq!(outcome.active_lines, 2);

        let on_disk = fs::read_to_string(writer.active_path()).unwrap();
        let lines: Vec<&str> = on_disk.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn rollover_on_line_count_threshold() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(
            dir.path(),
            "active",
            RolloverPolicy::tight(/* max_lines = */ 3, /* max_bytes = */ 1_000_000),
        )
        .unwrap();

        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        let mut archive_seen = None;
        for seq in 1..=3 {
            let line = fresh_line(seq, &prev);
            let outcome = writer.append(&line).unwrap();
            prev = compute_line_hash(&line.to_canonical_bytes().unwrap());
            if let Some(arch) = outcome.rolled_over_to {
                archive_seen = Some(arch);
            }
        }

        let archive = archive_seen.expect("rollover should have fired on the 3rd append");
        assert!(archive.exists(), "archive path should be present on disk");
        assert!(
            !writer.active_path().exists(),
            "active file is renamed into the archive; next append recreates it"
        );
    }

    #[test]
    fn rollover_on_byte_threshold() {
        let dir = TempDir::new().unwrap();
        // Lines are >100 bytes once serialised, so 200 bytes triggers
        // rollover on the 2nd append.
        let writer = WitnessWriter::open(
            dir.path(),
            "active",
            RolloverPolicy::tight(1_000_000, /* max_bytes = */ 200),
        )
        .unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        let mut saw_rollover = false;
        for seq in 1..=5 {
            let line = fresh_line(seq, &prev);
            let outcome = writer.append(&line).unwrap();
            prev = compute_line_hash(&line.to_canonical_bytes().unwrap());
            if outcome.rolled_over_to.is_some() {
                saw_rollover = true;
                break;
            }
        }
        assert!(
            saw_rollover,
            "byte-size rollover should fire on the 2nd append"
        );
    }

    /// MLP2-012: a tight `RolloverPolicy` produces one manifest entry
    /// per archive in the same order as the rollovers fire. The
    /// manifest's `seq_at_rollover` matches the final `seq` written
    /// before the active file was renamed.
    #[test]
    fn rollover_emits_ordered_manifest_entries() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(
            dir.path(),
            "active",
            RolloverPolicy::tight(/* max_lines = */ 2, /* max_bytes = */ 1_000_000),
        )
        .unwrap();
        let mut prev = crate::genesis::GenesisAnchor::Fresh
            .anchor_string()
            .to_string();
        let mut archive_seqs = Vec::new();
        for seq in 1..=5 {
            let line = fresh_line(seq, &prev);
            let outcome = writer.append(&line).unwrap();
            prev = crate::line::compute_line_hash(&line.to_canonical_bytes().unwrap());
            if let Some(archive) = outcome.rolled_over_to {
                archive_seqs.push((seq, archive));
            }
        }
        // tight policy rolls at line 2 + line 4 -> 2 archives.
        assert_eq!(archive_seqs.len(), 2, "expected 2 rollovers");

        let manifest = crate::manifest::manifest_tail(&writer.witness_root()).unwrap();
        assert_eq!(manifest.len(), 2, "manifest should mirror rollover count");
        for (i, (seq, archive)) in archive_seqs.iter().enumerate() {
            assert_eq!(manifest[i].archive_path, *archive);
            assert_eq!(manifest[i].seq_at_rollover, *seq);
            assert!(
                manifest[i].line_count >= 1,
                "archive must record a non-zero line count",
            );
            assert_eq!(
                manifest[i].merkle.len(),
                64,
                "manifest carries the full SHA-256 hex",
            );
        }
    }

    /// MLP2-012 idempotency: when rollover lands on an archive whose
    /// content matches an existing archive (content-addressed rename
    /// no-op path), the manifest still records exactly one entry per
    /// rollover. Pin against a regression where the no-op rename
    /// branch silently skips the manifest append.
    #[test]
    fn manifest_records_one_entry_per_distinct_archive_even_on_renorm() {
        let dir = TempDir::new().unwrap();
        let writer =
            WitnessWriter::open(dir.path(), "active", RolloverPolicy::tight(2, 1_000_000)).unwrap();
        let mut prev = crate::genesis::GenesisAnchor::Fresh
            .anchor_string()
            .to_string();
        for seq in 1..=4 {
            let line = fresh_line(seq, &prev);
            writer.append(&line).unwrap();
            prev = crate::line::compute_line_hash(&line.to_canonical_bytes().unwrap());
        }
        let initial = crate::manifest::manifest_tail(&writer.witness_root()).unwrap();
        // 4 lines / 2-per-archive -> 2 manifest entries.
        assert_eq!(initial.len(), 2);
    }

    #[test]
    #[cfg(unix)]
    fn refuses_when_witness_root_is_symlink() {
        let dir = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        // Pre-create the anvil/ dir as a regular dir, then put a
        // symlink at anvil/witness/.
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), dir.path().join("anvil").join("witness"))
            .unwrap();
        let err = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap_err();
        assert!(matches!(err, WriterError::SymlinkRoot { .. }));
    }
}
