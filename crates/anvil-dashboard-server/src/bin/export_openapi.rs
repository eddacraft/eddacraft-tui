use std::io::Write;
use std::path::{Path, PathBuf};

use anvil_dashboard_server::openapi_document;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: export-openapi <output-path>")?;
    let mut document = serde_json::to_vec_pretty(&openapi_document())?;
    document.push(b'\n');
    atomic_write(&output, &document)?;
    Ok(())
}

/// Write `content` via a same-directory temp file + rename so an interrupted
/// export cannot leave the committed `OpenAPI` contract truncated.
fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    // Bare filenames like `openapi.json` report an empty parent; treat that as
    // the current directory. Only error when `parent()` is actually missing
    // (e.g. a root path).
    let dir = match path.parent() {
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("no parent directory for {}", path.display()),
            ));
        }
        Some(p) if p.as_os_str().is_empty() => Path::new("."),
        Some(p) => p,
    };

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("openapi.json");
    // Per-attempt uniqueness (PID + nanos) avoids tmp/backup collisions across
    // concurrent callers or retries after a failed restore.
    let attempt = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    );
    let tmp_path = dir.join(format!(".{file_name}.{attempt}.tmp"));

    {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
    }

    // On Unix, rename replaces an existing destination atomically. On Windows,
    // rename fails if the destination exists — use backup-then-replace so a
    // failed install can restore the previous contract instead of deleting it
    // first (clawpatch high: data-loss on rename failure).
    // No `return`: once the other arm is cfg-stripped this block is the
    // function's tail expression, so `return` here trips `needless_return`
    // on a Windows build (CIB-193).
    #[cfg(windows)]
    {
        let backup_path = dir.join(format!(".{file_name}.{attempt}.bak"));
        replace_existing_via_backup(&tmp_path, path, &backup_path)
    }

    #[cfg(not(windows))]
    {
        match std::fs::rename(&tmp_path, path) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = std::fs::remove_file(&tmp_path);
                Err(e)
            }
        }
    }
}

/// Install `tmp` over `dest` without permanently discarding the previous
/// destination until the install rename succeeds.
///
/// Steps: move `dest` aside to `backup` (if present), rename `tmp` → `dest`,
/// then remove `backup`. If the install rename fails, restore `backup` → `dest`
/// and surface the original install error.
///
/// Used on Windows (where `rename` cannot replace). Also exercised on all
/// platforms in unit tests so the restore path is covered without a Windows
/// runner.
#[cfg(any(windows, test))]
fn replace_existing_via_backup(tmp: &Path, dest: &Path, backup: &Path) -> std::io::Result<()> {
    // Callers pass a unique backup path per attempt, so we do not delete any
    // pre-existing path here — that would risk clobbering a recovery backup
    // from an earlier failed restore.

    let dest_existed = match std::fs::symlink_metadata(dest) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            let _ = std::fs::remove_file(tmp);
            return Err(e);
        }
    };

    if dest_existed && let Err(e) = std::fs::rename(dest, backup) {
        let _ = std::fs::remove_file(tmp);
        return Err(e);
    }

    match std::fs::rename(tmp, dest) {
        Ok(()) => {
            if dest_existed {
                let _ = std::fs::remove_file(backup);
            }
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(tmp);
            if dest_existed && let Err(restore_err) = std::fs::rename(backup, dest) {
                return Err(std::io::Error::new(
                    e.kind(),
                    format!(
                        "failed to install new content at {}: {e}; also failed to restore previous content from {}: {restore_err}",
                        dest.display(),
                        backup.display()
                    ),
                ));
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "export-openapi-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn atomic_write_replaces_existing_content() {
        let dir = temp_dir("replace");
        let path = dir.join("openapi.json");
        fs::write(&path, b"stale\n").expect("seed");

        atomic_write(&path, b"{\"ok\":true}\n").expect("atomic write");

        assert_eq!(fs::read_to_string(&path).expect("read"), "{\"ok\":true}\n");
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|e| e.path() != path)
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp file should be renamed away: {leftovers:?}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_accepts_bare_filename_in_cwd() {
        let cwd = temp_dir("cwd");
        let prev = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&cwd).expect("chdir");

        let result = atomic_write(Path::new("openapi.json"), b"{\"cwd\":true}\n");
        std::env::set_current_dir(prev).expect("restore cwd");
        result.expect("bare filename write");

        assert_eq!(
            fs::read_to_string(cwd.join("openapi.json")).expect("read"),
            "{\"cwd\":true}\n"
        );
        let _ = fs::remove_dir_all(&cwd);
    }

    #[test]
    fn replace_via_backup_preserves_destination_when_install_fails() {
        let dir = temp_dir("backup-restore");
        let dest = dir.join("openapi.json");
        let tmp = dir.join(".openapi.json.tmp");
        let backup = dir.join(".openapi.json.bak");

        fs::write(&dest, b"original-contract\n").expect("seed dest");
        // Missing `tmp` forces the install rename to fail after dest is moved
        // aside; the helper must restore the original contract from backup.
        let err = replace_existing_via_backup(&tmp, &dest, &backup).expect_err("install must fail");
        assert!(
            err.kind() == std::io::ErrorKind::NotFound || err.raw_os_error().is_some(),
            "unexpected error: {err:?}"
        );

        assert_eq!(
            fs::read_to_string(&dest).expect("restored dest"),
            "original-contract\n",
            "previous contract must be restored when install fails"
        );
        assert!(!backup.exists(), "backup should be consumed by restore");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_via_backup_replaces_and_removes_backup() {
        let dir = temp_dir("backup-success");
        let dest = dir.join("openapi.json");
        let tmp = dir.join(".openapi.json.tmp");
        let backup = dir.join(".openapi.json.bak");

        fs::write(&dest, b"old\n").expect("seed dest");
        fs::write(&tmp, b"new\n").expect("seed tmp");

        replace_existing_via_backup(&tmp, &dest, &backup).expect("replace");

        assert_eq!(fs::read_to_string(&dest).expect("read"), "new\n");
        assert!(!tmp.exists(), "tmp should be gone");
        assert!(!backup.exists(), "backup should be removed after success");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_via_backup_creates_when_dest_missing() {
        let dir = temp_dir("backup-create");
        let dest = dir.join("openapi.json");
        let tmp = dir.join(".openapi.json.tmp");
        let backup = dir.join(".openapi.json.bak");

        fs::write(&tmp, b"fresh\n").expect("seed tmp");

        replace_existing_via_backup(&tmp, &dest, &backup).expect("create");

        assert_eq!(fs::read_to_string(&dest).expect("read"), "fresh\n");
        assert!(!backup.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
