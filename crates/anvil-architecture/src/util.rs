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

    // On Windows, TempPath::persist uses std::fs::rename, which fails if the
    // destination already exists. Remove the existing file first.
    #[cfg(windows)]
    {
        if let Err(e) = std::fs::remove_file(path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e);
        }
    }

    tmp_path.persist(path).map_err(|e| e.error)?;

    Ok(())
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

    fn write_temp(bytes: &[u8]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(tmp.path(), bytes).expect("write temp");
        tmp
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
}
