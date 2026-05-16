//! Witness-segment discovery (MLP2-061 / MLP2-062 follow-up).
//!
//! Single source of truth for the ordered list of witness files
//! callers must walk when verifying or harvesting the chain: archive
//! segments first (lexicographic, matches the `<scope>-<seq>-<merkle>`
//! rollover naming), then `active.ndjson` when present.
//!
//! Before this module existed, the pre-push hook, `anvil l4-validate`,
//! and `anvil audit-chain` each carried their own copy of this
//! helper. Council quick review flagged the duplication as a
//! correctness risk: any drift between the three orderings would let
//! the chain verifier and the witnessed-set harvester cover different
//! bytes, silently reopening the MLP2-062 trust gap.

use std::path::{Path, PathBuf};

/// Build the ordered list of witness files for chain verification or
/// witnessed-set harvesting.
///
/// - Archive segments under `anvil/witness/archive/` are streamed
///   first, sorted lexicographically (matches `<scope>-<seq>-<merkle>`
///   so older segments precede newer ones).
/// - `anvil/witness/active.ndjson` is appended last when present.
///
/// Returns an empty `Vec` when no witness tree exists yet (fresh-
/// adoption shape). Non-`.ndjson` archive entries are skipped so a
/// stray file in the archive directory cannot poison the chain walk.
#[must_use]
pub fn witness_paths(repo_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let archive_dir = repo_root.join("anvil").join("witness").join("archive");
    if let Ok(entries) = std::fs::read_dir(&archive_dir) {
        let mut files: Vec<PathBuf> = entries
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("ndjson"))
            .collect();
        files.sort();
        out.extend(files);
    }
    let active = repo_root
        .join("anvil")
        .join("witness")
        .join("active.ndjson");
    if active.exists() {
        out.push(active);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn returns_empty_when_witness_tree_is_absent() {
        let tmp = TempDir::new().unwrap();
        assert!(witness_paths(tmp.path()).is_empty());
    }

    #[test]
    fn appends_active_after_archives_in_lexicographic_order() {
        let tmp = TempDir::new().unwrap();
        let archive = tmp.path().join("anvil/witness/archive");
        fs::create_dir_all(&archive).unwrap();
        // Names chosen so lex order != insertion order.
        fs::write(archive.join("active-00000000000000000050-mid.ndjson"), "").unwrap();
        fs::write(
            archive.join("active-00000000000000000003-oldest.ndjson"),
            "",
        )
        .unwrap();
        fs::write(
            archive.join("active-00000000000000000100-newest.ndjson"),
            "",
        )
        .unwrap();
        // Non-ndjson sibling must be ignored.
        fs::write(archive.join("readme.txt"), "").unwrap();
        fs::write(tmp.path().join("anvil/witness/active.ndjson"), "").unwrap();

        let paths = witness_paths(tmp.path());
        let names: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            vec![
                "active-00000000000000000003-oldest.ndjson",
                "active-00000000000000000050-mid.ndjson",
                "active-00000000000000000100-newest.ndjson",
                "active.ndjson",
            ],
        );
    }

    #[test]
    fn omits_active_when_only_archive_segments_present() {
        let tmp = TempDir::new().unwrap();
        let archive = tmp.path().join("anvil/witness/archive");
        fs::create_dir_all(&archive).unwrap();
        fs::write(archive.join("active-00000000000000000001-only.ndjson"), "").unwrap();

        let paths = witness_paths(tmp.path());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("active-00000000000000000001-only.ndjson"));
    }
}
