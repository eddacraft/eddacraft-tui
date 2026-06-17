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
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e);
            }
        }
    }

    tmp_path.persist(path).map_err(|e| e.error)?;

    Ok(())
}
