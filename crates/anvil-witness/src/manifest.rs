//! Witness chain manifest types and serialisation.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::writer::WriterError;

/// A single manifest entry — one rollover event.
///
/// The struct is `Serialize + Deserialize` against the on-disk NDJSON
/// shape; field names match the JSON keys byte-for-byte. Keep the
/// order deterministic so a `serde_json::to_string` produces stable
/// output for the same input — the writer relies on this for its
/// idempotent re-write check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Path to the archive file written at this rollover boundary.
    /// Stored as written by the caller — the writer passes the
    /// canonical archive path it returned from rollover. Consumers
    /// resolve relative to the repo root.
    pub archive_path: PathBuf,
    /// Full hex SHA-256 of the archived file bytes. 64 lowercase
    /// hex characters; the archive name carries only the first 16
    /// for filesystem readability so this field is the canonical
    /// digest for verification.
    pub merkle: String,
    /// Number of NDJSON lines in the archived file at rollover time.
    pub line_count: u64,
    /// `WitnessLine::seq` of the final line written before rollover.
    /// Combined with [`Self::line_count`] this gives the inclusive
    /// `[start..=end]` range covered by the archive.
    pub seq_at_rollover: u64,
}

impl ManifestEntry {
    /// Encode to a single NDJSON line. The trailing `\n` is included
    /// so callers append directly without remembering to add it.
    pub fn to_ndjson_line(&self) -> Result<Vec<u8>, WriterError> {
        let mut out = serde_json::to_vec(self)?;
        out.push(b'\n');
        Ok(out)
    }
}

/// Path of the manifest file under a witness root.
///
/// `witness_root` is the directory `crates/anvil-witness/src/writer.rs`
/// uses (`<repo>/anvil/witness`). Tests pass the same path they pass
/// the writer; the helper does not canonicalise.
#[must_use]
pub fn manifest_path(witness_root: &Path) -> PathBuf {
    witness_root.join("manifest").join("chain.ndjson")
}

/// Append a manifest entry under the witness root. The caller is
/// expected to be holding the writer's flock — the rollover path in
/// [`crate::WitnessWriter::append`] is the only production caller and
/// already serialises through that lock.
///
/// **Idempotency.** Content-addressed archive naming means a re-run
/// rollover can land on an archive that already exists with
/// byte-identical content. In that case the writer keeps the existing
/// archive and removes the active file. The manifest mirrors that
/// semantics: an entry whose `(archive_path, merkle, line_count,
/// seq_at_rollover)` already appears as the *last* entry is treated
/// as a no-op. Older entries are not scanned (a manifest with N
/// historical rollovers stays O(1) per call); a re-rolled identical
/// archive that happens to share content with something far older
/// would produce a duplicate, but the writer's content-addressed
/// rename guarantees the *active rollover* sequence never produces
/// that pattern.
pub fn append_manifest_entry(
    witness_root: &Path,
    entry: &ManifestEntry,
) -> Result<(), WriterError> {
    let manifest_dir = witness_root.join("manifest");
    fs::create_dir_all(&manifest_dir)?;
    let path = manifest_dir.join("chain.ndjson");

    if last_entry_matches(&path, entry)? {
        // Idempotent re-write: nothing to do. The writer's rollover
        // path entered this branch because the archive already
        // existed with identical content, so the manifest already
        // records the same event.
        return Ok(());
    }

    let bytes = entry.to_ndjson_line()?;
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Read every manifest entry under the witness root in append order.
///
/// Returns an empty vec when the manifest file does not exist yet
/// (the first rollover creates it).
pub fn manifest_tail(witness_root: &Path) -> Result<Vec<ManifestEntry>, WriterError> {
    let path = manifest_path(witness_root);
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(WriterError::Io(e)),
    };
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let entry: ManifestEntry = serde_json::from_str(&line)?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Check whether the *last* entry in the manifest file at `path`
/// equals `entry`. Returns `false` when the file does not exist or
/// is empty.
fn last_entry_matches(path: &Path, entry: &ManifestEntry) -> Result<bool, WriterError> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(WriterError::Io(e)),
    };
    let reader = BufReader::new(file);
    let mut last: Option<String> = None;
    for line in reader.lines() {
        let line = line?;
        if !line.is_empty() {
            last = Some(line);
        }
    }
    let Some(last) = last else {
        return Ok(false);
    };
    let parsed: ManifestEntry = match serde_json::from_str(&last) {
        Ok(p) => p,
        // A corrupt tail entry is treated as "not matching" so the
        // append path writes a fresh line below it. A separate
        // verifier surfaces the corruption — this code path is the
        // idempotency check, not the integrity check.
        Err(_) => return Ok(false),
    };
    Ok(&parsed == entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_entry(name: &str, seq: u64) -> ManifestEntry {
        ManifestEntry {
            archive_path: PathBuf::from(format!("anvil/witness/archive/{name}.ndjson")),
            merkle: format!("{name:0>64}"),
            line_count: 3,
            seq_at_rollover: seq,
        }
    }

    #[test]
    fn ndjson_line_ends_in_newline_and_round_trips() {
        let entry = sample_entry("abc", 5);
        let bytes = entry.to_ndjson_line().unwrap();
        assert!(bytes.ends_with(b"\n"));
        let line = std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap();
        let parsed: ManifestEntry = serde_json::from_str(line).unwrap();
        assert_eq!(parsed, entry);
    }

    #[test]
    fn manifest_tail_returns_empty_when_no_manifest() {
        let dir = TempDir::new().unwrap();
        let entries = manifest_tail(dir.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn append_then_tail_returns_entries_in_append_order() {
        let dir = TempDir::new().unwrap();
        let a = sample_entry("a", 10);
        let b = sample_entry("b", 20);
        append_manifest_entry(dir.path(), &a).unwrap();
        append_manifest_entry(dir.path(), &b).unwrap();
        let entries = manifest_tail(dir.path()).unwrap();
        assert_eq!(entries, vec![a, b]);
    }

    /// MLP2-012 idempotency: appending the same entry twice in a row
    /// records it once. The writer's rollover path re-enters this
    /// when an archive already exists with identical bytes (content-
    /// addressed naming); the manifest must mirror that semantics.
    #[test]
    fn append_is_idempotent_when_last_entry_matches() {
        let dir = TempDir::new().unwrap();
        let a = sample_entry("a", 10);
        append_manifest_entry(dir.path(), &a).unwrap();
        append_manifest_entry(dir.path(), &a).unwrap();
        let entries = manifest_tail(dir.path()).unwrap();
        assert_eq!(entries, vec![a]);
    }

    /// A second entry with a different `merkle` (or any other field)
    /// is appended even when it shares a name with an earlier entry.
    /// Idempotency is a "last-entry equals" check, not a "seen this
    /// merkle anywhere in history" check.
    #[test]
    fn append_writes_new_entry_when_different_from_last() {
        let dir = TempDir::new().unwrap();
        let a = sample_entry("a", 10);
        let mut a_changed = a.clone();
        a_changed.line_count = 4; // different volume
        append_manifest_entry(dir.path(), &a).unwrap();
        append_manifest_entry(dir.path(), &a_changed).unwrap();
        let entries = manifest_tail(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], a);
        assert_eq!(entries[1], a_changed);
    }

    #[test]
    fn manifest_path_is_under_witness_root_manifest_dir() {
        let path = manifest_path(Path::new("/repo/anvil/witness"));
        assert_eq!(
            path,
            PathBuf::from("/repo/anvil/witness/manifest/chain.ndjson"),
        );
    }
}
