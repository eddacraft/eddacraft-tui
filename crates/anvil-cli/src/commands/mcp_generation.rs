//! Install-scoped MCP refresh generation (MCPLH-003).
//!
//! Live `mcp serve` processes treat a generation greater than their
//! last-seen value as a prompt to re-check the preferred binary. The
//! file lives next to the intercept PID file (`ANVIL_HOME` / XDG runtime),
//! never under a project `.anvil/` directory.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::util::atomic_write_nofollow;

const GENERATION_FILE_NAME: &str = "mcp-refresh.generation";

/// Path of the install-scoped generation file.
pub(crate) fn generation_path() -> Result<PathBuf> {
    let pid_path = anvil_intercept::default_pid_file_path()
        .context("resolving install-scoped MCP refresh generation path")?;
    Ok(pid_path.with_file_name(GENERATION_FILE_NAME))
}

/// Current generation, or `0` when the file is missing or unreadable.
#[must_use]
pub(crate) fn current_generation() -> u64 {
    generation_path()
        .ok()
        .and_then(|path| read_generation(&path).ok())
        .unwrap_or(0)
}

/// Read a generation counter. Missing or empty files are generation `0`.
pub(crate) fn read_generation(path: &Path) -> Result<u64> {
    match fs::read_to_string(path) {
        Ok(raw) if raw.trim().is_empty() => Ok(0),
        Ok(raw) => raw
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .parse::<u64>()
            .with_context(|| format!("parsing refresh generation {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => {
            Err(error).with_context(|| format!("reading refresh generation {}", path.display()))
        }
    }
}

/// Increment the generation file and return the new value.
pub(crate) fn bump_generation(path: &Path) -> Result<u64> {
    let next = read_generation(path)?.saturating_add(1);
    write_generation(path, next)?;
    Ok(next)
}

fn write_generation(path: &Path, value: u64) -> Result<()> {
    write_generation_sidecar(path, format!("{value}\n").as_bytes())
        .with_context(|| format!("writing refresh generation {}", path.display()))
}

/// Write an install-scoped sidecar next to the generation file (pin,
/// last-poked CLI version). Creates the parent directory and locks it
/// down the same way as the generation counter.
pub(crate) fn write_generation_sidecar(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating generation directory {}", parent.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(parent)
                .with_context(|| format!("stat generation directory {}", parent.display()))?
                .permissions();
            perms.set_mode(0o700);
            fs::set_permissions(parent, perms).with_context(|| {
                format!("locking down generation directory {}", parent.display())
            })?;
        }
    }
    atomic_write_nofollow(path, bytes).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{bump_generation, read_generation};

    #[test]
    fn missing_generation_reads_as_zero() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mcp-refresh.generation");
        assert_eq!(read_generation(&path).expect("read"), 0);
    }

    #[test]
    fn bump_generation_is_monotonic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mcp-refresh.generation");
        assert_eq!(bump_generation(&path).expect("first bump"), 1);
        assert_eq!(bump_generation(&path).expect("second bump"), 2);
        assert_eq!(read_generation(&path).expect("read"), 2);
    }
}
