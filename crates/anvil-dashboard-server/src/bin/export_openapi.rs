use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
///
/// Staging uses an exclusive (`create_new`) uniquely named sibling so a
/// pre-planted symlink cannot redirect the write and concurrent exporters
/// cannot share one temporary path.
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

    #[cfg(windows)]
    let (mut file, tmp_path, attempt_id) = open_exclusive_staging_file(dir, file_name)?;
    #[cfg(not(windows))]
    let (mut file, tmp_path, _) = open_exclusive_staging_file(dir, file_name)?;
    if let Err(e) = file.write_all(content).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }
    // Close the staging handle before rename: Windows cannot rename a file
    // that still has an open *source* handle.
    drop(file);

    // On Unix, rename replaces an existing destination atomically. On Windows,
    // rename fails if the destination exists — use backup-then-replace so a
    // failed install can restore the previous contract instead of deleting it
    // first (clawpatch high: data-loss on rename failure).
    // No `return`: once the other arm is cfg-stripped this block is the
    // function's tail expression, so `return` here trips `needless_return`
    // on a Windows build (CIB-193).
    #[cfg(windows)]
    {
        let backup_path = dir.join(format!(".{file_name}.{attempt_id}.bak"));
        replace_existing_via_backup(&tmp_path, path, &backup_path)
    }

    #[cfg(not(windows))]
    {
        match fs::rename(&tmp_path, path) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = fs::remove_file(&tmp_path);
                Err(e)
            }
        }
    }
}

/// Create a uniquely named staging file next to the export destination.
///
/// Uses `OpenOptions::create_new` so the open is exclusive: on Unix this maps
/// to `O_CREAT|O_EXCL`, which refuses to follow a pre-existing symlink and
/// fails with `AlreadyExists` if the path is occupied. That closes both the
/// predictable-name concurrent-clobber race and the `File::create` symlink
/// redirection window a PID-only (or PID+nanos without exclusive create)
/// staging path allowed.
fn open_exclusive_staging_file(
    dir: &Path,
    file_name: &str,
) -> std::io::Result<(File, PathBuf, String)> {
    let pid = std::process::id();
    for attempt in 0u32..32 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        // Mix pid, wall-clock nanos, and attempt so concurrent callers and
        // same-nanosecond retries almost never collide; create_new still
        // serialises any true collision safely.
        let nonce =
            nanos ^ (u128::from(pid) << 64) ^ (u128::from(attempt) << 48) ^ u128::from(attempt);
        let attempt_id = format!("{pid}-{nonce}");
        let tmp_path = dir.join(format!(".{file_name}.{attempt_id}.tmp"));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
        {
            Ok(file) => return Ok((file, tmp_path, attempt_id)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "exhausted exclusive temporary export file name attempts",
    ))
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

    let dest_existed = match fs::symlink_metadata(dest) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            let _ = fs::remove_file(tmp);
            return Err(e);
        }
    };

    if dest_existed && let Err(e) = fs::rename(dest, backup) {
        let _ = fs::remove_file(tmp);
        return Err(e);
    }

    match fs::rename(tmp, dest) {
        Ok(()) => {
            if dest_existed {
                let _ = fs::remove_file(backup);
            }
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(tmp);
            if dest_existed && let Err(restore_err) = fs::rename(backup, dest) {
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
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    /// Leftover staging temps beside the destination (legacy fixed name and
    /// exclusive unique pattern).
    fn staging_leftovers(dir: &Path, file_name: &str) -> Vec<String> {
        let prefix = format!(".{file_name}.");
        fs::read_dir(dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| {
                n.starts_with(&prefix)
                    && Path::new(n)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("tmp"))
            })
            .collect()
    }

    #[test]
    fn atomic_write_replaces_existing_content() {
        let dir = temp_dir("replace");
        let path = dir.join("openapi.json");
        fs::write(&path, b"stale\n").expect("seed");

        atomic_write(&path, b"{\"ok\":true}\n").expect("atomic write");

        assert_eq!(fs::read_to_string(&path).expect("read"), "{\"ok\":true}\n");
        let leftovers = staging_leftovers(&dir, "openapi.json");
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
    fn open_exclusive_staging_file_returns_distinct_paths() {
        let dir = temp_dir("distinct");
        let (f1, p1, id1) = open_exclusive_staging_file(&dir, "openapi.json").expect("first");
        drop(f1);
        let (f2, p2, id2) = open_exclusive_staging_file(&dir, "openapi.json").expect("second");
        drop(f2);
        assert_ne!(p1, p2, "exclusive staging paths must not collide");
        assert_ne!(id1, id2, "attempt ids must differ");
        fs::write(&p1, b"one").expect("write p1");
        fs::write(&p2, b"two").expect("write p2");
        assert_eq!(fs::read(&p1).expect("read p1"), b"one");
        assert_eq!(fs::read(&p2).expect("read p2"), b"two");
        let _ = fs::remove_dir_all(&dir);
    }

    /// A shared staging name breaks concurrent writers two ways: the
    /// loser's `rename` hits `NotFound` because the winner already moved
    /// the shared temp away, or two writers interleave into the same file
    /// and a corrupted mixture is renamed into place.
    #[test]
    fn atomic_write_concurrent_calls_do_not_share_staging_path() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        const ROUNDS: usize = 20;
        let payloads = [
            br#"{"writer":"a","pad":"aaaaaaaaaaaaaaaa"}"#.as_slice(),
            br#"{"writer":"b","pad":"bbbbbbbbbbbbbbbb"}"#.as_slice(),
            br#"{"writer":"c","pad":"cccccccccccccccc"}"#.as_slice(),
            br#"{"writer":"d","pad":"dddddddddddddddd"}"#.as_slice(),
        ];

        for round in 0..ROUNDS {
            let dir = temp_dir(&format!("concurrent-{round}"));
            let path = Arc::new(dir.join("openapi.json"));
            let barrier = Arc::new(Barrier::new(payloads.len()));
            let mut handles = Vec::new();
            for payload in payloads {
                let path = Arc::clone(&path);
                let barrier = Arc::clone(&barrier);
                handles.push(thread::spawn(move || {
                    barrier.wait();
                    atomic_write(path.as_ref(), payload)
                }));
            }
            let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            assert!(
                results.iter().all(std::result::Result::is_ok),
                "round {round}: concurrent write must not fail from shared staging clobber: {results:?}",
            );

            let body = fs::read(path.as_ref()).expect("read final");
            assert!(
                payloads.contains(&body.as_slice()),
                "round {round}: final body must be one complete writer payload, got {body:?}",
            );
            let leftovers = staging_leftovers(&dir, "openapi.json");
            assert!(
                leftovers.is_empty(),
                "round {round}: staging temps leaked: {leftovers:?}",
            );
            let _ = fs::remove_dir_all(&dir);
        }
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_does_not_follow_legacy_fixed_temp_sibling_symlink() {
        // Historical fixed staging path (`.openapi.json.tmp`) is both a
        // concurrent-clobber and a File::create symlink-redirection hazard.
        // Write must not use it at all: a pre-planted symlink there must
        // neither be followed nor block the write.
        use std::os::unix::fs::symlink;

        let dir = temp_dir("legacy-symlink");
        let outside = dir.join("outside-sentinel");
        fs::write(&outside, b"safe-sentinel\n").expect("seed sentinel");
        let legacy = dir.join(".openapi.json.tmp");
        symlink(&outside, &legacy).expect("plant legacy symlink");

        let dest = dir.join("openapi.json");
        atomic_write(&dest, b"{\"ok\":true}\n").expect("write");

        assert_eq!(
            fs::read_to_string(&outside).expect("sentinel"),
            "safe-sentinel\n",
            "legacy fixed temp symlink must not be written through"
        );
        assert_eq!(fs::read_to_string(&dest).expect("dest"), "{\"ok\":true}\n");
        assert!(
            legacy
                .symlink_metadata()
                .expect("meta")
                .file_type()
                .is_symlink(),
            "legacy plant should remain an untouched symlink"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn exclusive_staging_skips_preplanted_symlink_without_following() {
        // If an attacker occupies a candidate staging path with a symlink,
        // create_new must fail closed (AlreadyExists) and retry a different
        // name — never truncate the symlink target via File::create.
        use std::os::unix::fs::symlink;

        let dir = temp_dir("planted-staging");
        let outside = dir.join("outside-sentinel");
        fs::write(&outside, b"safe-sentinel\n").expect("seed sentinel");

        let (f, planted, _) =
            open_exclusive_staging_file(&dir, "openapi.json").expect("claim path");
        drop(f);
        fs::remove_file(&planted).expect("remove regular");
        symlink(&outside, &planted).expect("plant symlink at claimed path");

        let (f2, p2, _) = open_exclusive_staging_file(&dir, "openapi.json").expect("retry");
        assert_ne!(planted, p2, "must not reopen the planted symlink path");
        drop(f2);
        let _ = fs::remove_file(&p2);

        let dest = dir.join("openapi.json");
        atomic_write(&dest, b"{\"ok\":true}\n").expect("write after plant");

        assert_eq!(
            fs::read_to_string(&outside).expect("sentinel"),
            "safe-sentinel\n",
            "planted staging symlink must not be written through"
        );
        assert_eq!(fs::read_to_string(&dest).expect("dest"), "{\"ok\":true}\n");
        assert!(
            planted
                .symlink_metadata()
                .expect("meta")
                .file_type()
                .is_symlink(),
            "planted path should remain a symlink"
        );
        let _ = fs::remove_dir_all(&dir);
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
