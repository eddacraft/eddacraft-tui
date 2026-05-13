use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::store::{Baseline, FormatError};

/// Repo-relative path of the baseline file. Always exactly this —
/// changing it is a breaking change for every downstream consumer
/// (gate, L4, audit).
pub const BASELINE_PATH: &str = "anvil/baseline.json";

#[derive(Debug, Error)]
pub enum BaselineIoError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("format: {0}")]
    Format(#[from] FormatError),
    #[error("`{path}` is a symlink; refusing to read/write baseline outside the repo")]
    SymlinkRefusal { path: PathBuf },
}

/// Load `anvil/baseline.json` from `repo_root`. Returns `None` when
/// the file is absent.
///
/// Refuses to read through a symlink at any point in `anvil/` or
/// `anvil/baseline.json` to prevent a malicious worktree state from
/// redirecting the baseline read to an out-of-tree file. This
/// matches the TOCTOU-hardened pattern MLP-001 established for
/// `anvil/project-id`.
pub fn load(repo_root: &Path) -> Result<Option<Baseline>, BaselineIoError> {
    let parent = repo_root.join("anvil");
    refuse_if_symlink(&parent)?;
    let path = repo_root.join(BASELINE_PATH);
    refuse_if_symlink(&path)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    let baseline = Baseline::from_bytes(&bytes)?;
    Ok(Some(baseline))
}

/// Save `baseline` to `anvil/baseline.json` under `repo_root`,
/// creating `anvil/` if needed.
///
/// TOCTOU-hardened: the symlink check fires before AND after
/// `create_dir_all` so a racing process cannot swap the directory
/// for a symlink between our pre-check and our write. The write
/// itself is atomic — we serialise to a temporary sibling and
/// rename into place, which prevents readers from observing a
/// half-written file under crash or concurrent reload.
pub fn save(repo_root: &Path, baseline: &Baseline) -> Result<(), BaselineIoError> {
    let parent = repo_root.join("anvil");
    refuse_if_symlink(&parent)?;
    fs::create_dir_all(&parent)?;
    refuse_if_symlink(&parent)?;

    let final_path = repo_root.join(BASELINE_PATH);
    refuse_if_symlink(&final_path)?;

    let tmp_path = parent.join(".baseline.json.tmp");
    // Refuse a pre-existing symlink at the temp path too. Otherwise a
    // hostile worktree state could pre-create `.baseline.json.tmp` as
    // a symlink pointing outside the repo, and our `File::create`
    // would happily write through it (overwriting the target).
    refuse_if_symlink(&tmp_path)?;
    let bytes = baseline.to_canonical_bytes()?;
    {
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    refuse_if_symlink(&final_path)?;
    atomic_replace(&tmp_path, &final_path)?;
    Ok(())
}

/// Atomically replace `dest` with `src`. POSIX `rename(2)` overwrites
/// silently; Windows `MoveFileExW` (which `std::fs::rename` calls)
/// returns `AlreadyExists` when `dest` exists, so we fall back to
/// remove-then-rename on that one error path. The window between the
/// remove and the rename is narrow and only matters on Windows; on
/// POSIX the first rename always wins.
fn atomic_replace(src: &Path, dest: &Path) -> Result<(), BaselineIoError> {
    match fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            // Refuse symlinks again before the remove so we don't
            // chase a swapped link out of the repo.
            refuse_if_symlink(dest)?;
            fs::remove_file(dest)?;
            fs::rename(src, dest)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Refuse if `path` is a symlink — including a *broken* symlink whose
/// target doesn't exist. `Path::exists()` returns false for a broken
/// symlink (it follows the link before checking), which would let an
/// attacker stage a symlink to a non-existent file as a "doesn't
/// exist" path and bypass our refusal. We use `symlink_metadata()`
/// which inspects the link itself, not its target.
fn refuse_if_symlink(path: &Path) -> Result<(), BaselineIoError> {
    match path.symlink_metadata() {
        Ok(meta) if meta.file_type().is_symlink() => Err(BaselineIoError::SymlinkRefusal {
            path: path.to_path_buf(),
        }),
        // Not a symlink → not refused. Either a regular file/dir or
        // something else (socket, etc.); the caller's subsequent
        // operations will surface a more specific error if so.
        Ok(_) => Ok(()),
        // ENOENT → path doesn't exist at all, including as a symlink.
        // That's fine; the caller will create it.
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute_fingerprint;
    use crate::finding::BaselineFinding;
    use crate::store::{Baseline, BaselineMetadata};

    fn metadata() -> BaselineMetadata {
        BaselineMetadata {
            created_at: "2026-05-13T00:00:00Z".to_string(),
            created_by_version: "0.7.0-beta".to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
        }
    }

    fn sample() -> Baseline {
        Baseline::new(
            metadata(),
            vec![BaselineFinding {
                rule_id: "anti-pattern:guardrail-suppression".to_string(),
                file_path: "src/lib.rs".to_string(),
                fingerprint: compute_fingerprint(
                    "anti-pattern:guardrail-suppression",
                    "// @ts-ignore",
                )
                .unwrap(),
            }],
        )
    }

    #[test]
    fn save_then_load_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let b = sample();
        save(tmp.path(), &b).unwrap();
        let loaded = load(tmp.path()).unwrap().expect("baseline exists");
        assert_eq!(loaded, b);
    }

    #[test]
    fn load_returns_none_when_file_absent() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn save_creates_anvil_directory_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!tmp.path().join("anvil").exists());
        save(tmp.path(), &sample()).unwrap();
        assert!(tmp.path().join("anvil").is_dir());
        assert!(tmp.path().join(BASELINE_PATH).is_file());
    }

    #[test]
    fn save_writes_canonical_trailing_newline() {
        let tmp = tempfile::tempdir().unwrap();
        save(tmp.path(), &sample()).unwrap();
        let bytes = fs::read(tmp.path().join(BASELINE_PATH)).unwrap();
        assert!(bytes.ends_with(b"\n"), "canonical bytes end in newline");
    }

    #[test]
    fn save_is_atomic_via_temp_then_rename() {
        let tmp = tempfile::tempdir().unwrap();
        save(tmp.path(), &sample()).unwrap();
        // After save, the .tmp file must not linger.
        let tmp_path = tmp.path().join("anvil").join(".baseline.json.tmp");
        assert!(!tmp_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn save_refuses_when_anvil_dir_is_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        symlink(other.path(), tmp.path().join("anvil")).unwrap();
        let err = save(tmp.path(), &sample()).unwrap_err();
        assert!(matches!(err, BaselineIoError::SymlinkRefusal { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn load_refuses_when_anvil_dir_is_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        // Make the symlink target a real anvil dir so load would
        // otherwise succeed; the refusal is policy-driven, not
        // missing-file-driven.
        fs::create_dir(other.path().join("anvil")).unwrap();
        symlink(other.path().join("anvil"), tmp.path().join("anvil")).unwrap();
        let err = load(tmp.path()).unwrap_err();
        assert!(matches!(err, BaselineIoError::SymlinkRefusal { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn save_refuses_when_baseline_file_is_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("anvil")).unwrap();
        let outside = tmp.path().join("outside.json");
        fs::write(&outside, "{}").unwrap();
        symlink(&outside, tmp.path().join(BASELINE_PATH)).unwrap();
        let err = save(tmp.path(), &sample()).unwrap_err();
        assert!(matches!(err, BaselineIoError::SymlinkRefusal { .. }));
    }

    #[test]
    fn save_overwrites_existing_baseline_atomically() {
        let tmp = tempfile::tempdir().unwrap();
        save(tmp.path(), &sample()).unwrap();
        // Modify and resave.
        let mut b = sample();
        b.cutoff_commit = Some("abc123".to_string());
        save(tmp.path(), &b).unwrap();
        let loaded = load(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.cutoff_commit.as_deref(), Some("abc123"));
    }

    #[cfg(unix)]
    #[test]
    fn save_refuses_when_tmp_path_is_symlink_before_write() {
        // A hostile worktree state could pre-create
        // `.baseline.json.tmp` as a symlink pointing out of the
        // repo; without the tmp-path refusal, `File::create` would
        // happily write *through* the link.
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("anvil")).unwrap();
        let outside = tmp.path().join("outside.json");
        symlink(
            &outside,
            tmp.path().join("anvil").join(".baseline.json.tmp"),
        )
        .unwrap();
        let err = save(tmp.path(), &sample()).unwrap_err();
        assert!(matches!(err, BaselineIoError::SymlinkRefusal { .. }));
        // The outside file must NOT exist — the symlink shouldn't
        // have been written through.
        assert!(!outside.exists());
    }

    #[cfg(unix)]
    #[test]
    fn refuse_if_symlink_catches_broken_symlinks() {
        // `Path::exists()` returns false for a broken symlink (it
        // follows the link). The earlier impl used `.exists()` and
        // would silently allow a broken-symlink baseline path. The
        // fixed impl uses `symlink_metadata` and refuses on the
        // link itself.
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("anvil")).unwrap();
        // Symlink to a target that doesn't exist.
        symlink(
            tmp.path().join("nonexistent-target"),
            tmp.path().join(BASELINE_PATH),
        )
        .unwrap();
        let err = save(tmp.path(), &sample()).unwrap_err();
        assert!(matches!(err, BaselineIoError::SymlinkRefusal { .. }));
    }
}
