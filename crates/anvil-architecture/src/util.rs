use std::path::Path;

/// Workspace-root-relative path with forward slashes, or `None` if `abs` is not
/// under `root`, escapes it through an un-normalised `..` component, or is not
/// valid UTF-8.
///
/// The `..` guard stops a `..`-bearing manifest path (a Cargo `members` /
/// `[[bin]] path`, or a Python entry-point module reference) from persisting a
/// path that points outside the workspace into a baseline. Shared by the Rust
/// (`detection`) and Python (`python_detection`) entry-point detectors.
pub(crate) fn relative_slash(root: &Path, abs: &Path) -> Option<String> {
    let rel = abs.strip_prefix(root).ok()?;
    if rel
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    Some(rel.to_str()?.replace('\\', "/"))
}

/// Atomically write `content` to `path` via a temp file + rename.
///
/// Creates a temporary file in the same directory, writes content, then
/// renames to the target path. This prevents corruption if the process
/// is interrupted mid-write.
///
/// On Windows, where `rename` cannot replace an existing destination, the
/// prior file is moved aside to a unique sibling backup and restored if the
/// install rename fails — so a failed persist never deletes the previous
/// baseline or architecture YAML.
pub(crate) fn atomic_write(path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    use std::io::Write;

    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("no parent directory for {}", path.display()),
        )
    })?;

    let mut tmp = tempfile::Builder::new().tempfile_in(dir)?;
    tmp.write_all(content)?;
    tmp.flush()?;

    let tmp_path = tmp.into_temp_path();

    // On Unix, rename replaces an existing destination atomically. On Windows,
    // rename fails if the destination exists — use backup-then-replace so a
    // failed install can restore the previous content instead of deleting it
    // first (clawpatch: data-loss on persist failure).
    // No `return`: once the other arm is cfg-stripped this block is the
    // function's tail expression, so `return` here trips `needless_return`
    // on a Windows build.
    #[cfg(windows)]
    {
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
        let attempt_id = unique_attempt_id();
        let backup_path = dir.join(format!(".{file_name}.{attempt_id}.bak"));
        // Defuse TempPath cleanup so we own the staging path through the
        // backup-replace helper (which removes it on both success and failure).
        let staging = tmp_path.keep().map_err(|e| e.error)?;
        replace_existing_via_backup(&staging, path, &backup_path)
    }

    #[cfg(not(windows))]
    {
        tmp_path.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
}

#[cfg(windows)]
fn unique_attempt_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    // Pair wall-clock nanos with a process-local counter so concurrent or
    // same-tick attempts cannot collide on a coarse SystemTime resolution.
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("{}-{nanos}-{seq}", std::process::id())
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
    use std::fs;

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

/// Read `path` into a string, refusing files larger than `cap` bytes.
///
/// A pre-read `fstat` rejects an over-cap file before any allocation, and the
/// read itself is `take`-limited to `cap + 1` so a file that grows past the cap
/// between the stat and the read is still caught (mirrors
/// `anvil_config::read_to_string_bounded`, but with a caller-chosen cap). The
/// over-cap case surfaces as [`std::io::ErrorKind::InvalidData`] so callers fold
/// it into their existing IO-error handling.
///
/// This bounds the memory a CLI command or MCP resource commits when a
/// (possibly hostile or corrupt) workspace file is unexpectedly large (CIB-084).
pub fn read_to_string_capped(path: &Path, cap: u64) -> std::io::Result<String> {
    use std::io::Read;

    let file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    if size > cap {
        return Err(over_cap(path, cap));
    }
    // `size <= cap`, so the capacity hint is bounded; the `+ 1` on the read still
    // catches a file that grew past the cap between the stat and the read.
    let mut contents = String::with_capacity(usize::try_from(size).unwrap_or(0));
    file.take(cap.saturating_add(1))
        .read_to_string(&mut contents)?;
    if contents.len() as u64 > cap {
        return Err(over_cap(path, cap));
    }
    Ok(contents)
}

fn over_cap(path: &Path, cap: u64) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("{} exceeds the {cap}-byte read cap", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp(bytes: &[u8]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(tmp.path(), bytes).expect("write temp");
        tmp
    }

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "anvil-arch-util-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn reads_a_file_under_the_cap() {
        let tmp = write_temp(b"hello");
        assert_eq!(read_to_string_capped(tmp.path(), 1024).unwrap(), "hello");
    }

    #[test]
    fn reads_a_file_exactly_at_the_cap() {
        let tmp = write_temp(b"abcd");
        assert_eq!(read_to_string_capped(tmp.path(), 4).unwrap(), "abcd");
    }

    #[test]
    fn rejects_a_file_over_the_cap() {
        let tmp = write_temp(b"0123456789");
        let err = read_to_string_capped(tmp.path(), 4).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn atomic_write_creates_and_replaces_content() {
        let dir = temp_dir("atomic-roundtrip");
        let path = dir.join("baseline.json");

        atomic_write(&path, b"first\n").expect("create");
        assert_eq!(fs::read_to_string(&path).expect("read"), "first\n");

        atomic_write(&path, b"second\n").expect("replace");
        assert_eq!(fs::read_to_string(&path).expect("read"), "second\n");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_via_backup_preserves_destination_when_install_fails() {
        let dir = temp_dir("backup-restore");
        let dest = dir.join("architecture.yaml");
        let tmp = dir.join(".architecture.yaml.tmp");
        let backup = dir.join(".architecture.yaml.bak");

        fs::write(&dest, b"original-yaml\n").expect("seed dest");
        // Missing `tmp` forces the install rename to fail after dest is moved
        // aside; the helper must restore the original bytes from backup.
        let err = replace_existing_via_backup(&tmp, &dest, &backup).expect_err("install must fail");
        assert!(
            err.kind() == std::io::ErrorKind::NotFound || err.raw_os_error().is_some(),
            "unexpected error: {err:?}"
        );

        assert_eq!(
            fs::read_to_string(&dest).expect("restored dest"),
            "original-yaml\n",
            "previous content must be restored when install fails"
        );
        assert!(!backup.exists(), "backup should be consumed by restore");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_via_backup_replaces_and_removes_backup() {
        let dir = temp_dir("backup-success");
        let dest = dir.join("baseline.json");
        let tmp = dir.join(".baseline.json.tmp");
        let backup = dir.join(".baseline.json.bak");

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
        let dest = dir.join("architecture.yaml");
        let tmp = dir.join(".architecture.yaml.tmp");
        let backup = dir.join(".architecture.yaml.bak");

        fs::write(&tmp, b"fresh\n").expect("seed tmp");

        replace_existing_via_backup(&tmp, &dest, &backup).expect("create");

        assert_eq!(fs::read_to_string(&dest).expect("read"), "fresh\n");
        assert!(!backup.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
