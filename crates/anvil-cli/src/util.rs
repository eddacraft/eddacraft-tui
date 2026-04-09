use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Resolve the workspace root via `git rev-parse --show-toplevel`.
///
/// Canonicalises the git result to collapse symlinks. Falls back to
/// the current directory (returned as-is, not canonicalised). Returns
/// an error only when no usable path can be determined.
pub fn workspace_root() -> Result<PathBuf> {
    let git_failure = match std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) if output.status.success() => {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                let root = PathBuf::from(stdout.trim());
                if let Ok(canonical) = root.canonicalize() {
                    return Ok(canonical);
                }
                return Ok(root);
            }

            Some("git rev-parse returned non-UTF-8 output".to_string())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                Some(format!(
                    "git rev-parse failed with status {}",
                    output.status
                ))
            } else {
                Some(format!(
                    "git rev-parse failed with status {}: {}",
                    output.status, stderr
                ))
            }
        }
        Err(err) => Some(format!("failed to run git rev-parse: {err}")),
    };

    std::env::current_dir().with_context(|| {
        if let Some(reason) = &git_failure {
            format!("failed to determine workspace root: {reason}; current directory unresolvable")
        } else {
            "failed to determine workspace root: current directory unresolvable".to_string()
        }
    })
}

/// Write `data` to `path` atomically by writing to a uniquely-named temporary
/// file in the same directory and then renaming. This prevents partial/corrupt
/// state files if the process crashes or is interrupted mid-write.
///
/// Uses `tempfile` for unpredictable filenames (prevents symlink attacks).
/// On Unix the temp file is created with mode 0o600.
///
/// Note: this provides process-crash atomicity, not power-loss durability
/// (no `fsync` before rename).
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));

    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut builder = tempfile::Builder::new();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o600));
    }

    let mut tmp = builder
        .tempfile_in(dir)
        .with_context(|| format!("creating temp file in {}", dir.display()))?;

    tmp.write_all(data)
        .with_context(|| format!("writing temp file for {}", path.display()))?;
    tmp.flush()
        .with_context(|| format!("flushing temp file for {}", path.display()))?;

    let tmp_path = tmp.into_temp_path();
    let tmp_display = tmp_path.display().to_string();

    // On Windows, TempPath::persist uses std::fs::rename under the hood, which
    // fails if the destination already exists. Remove the existing file first.
    #[cfg(windows)]
    {
        if let Err(err) = std::fs::remove_file(path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                return Err(err)
                    .with_context(|| format!("removing existing file {}", path.display()));
            }
        }
    }

    tmp_path
        .persist(path)
        .with_context(|| format!("persisting {tmp_display} -> {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        atomic_write(&path, b"hello").unwrap();

        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(&path, "old").unwrap();

        atomic_write(&path, b"new").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.json");

        atomic_write(&path, b"secret").unwrap();

        let perms = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }

    #[test]
    fn workspace_root_returns_absolute_path() {
        let root = workspace_root().unwrap();
        assert!(
            root.is_absolute(),
            "workspace root should be absolute, got: {root:?}"
        );
    }

    #[test]
    fn workspace_root_is_canonical() {
        let root = workspace_root().unwrap();
        if let Ok(canonical) = root.canonicalize() {
            assert_eq!(root, canonical);
        }
    }
}
