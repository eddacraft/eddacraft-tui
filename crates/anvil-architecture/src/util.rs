use std::path::Path;

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
