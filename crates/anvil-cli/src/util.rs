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

/// Format an `anyhow::Error` for user-facing display with path-leakage guardrails.
///
/// - Default (`verbose = false`): prints only the outermost context (`{err}`).
///   Relies on the outer context being a programmer-written, path-free string
///   (e.g. `"starting engine watcher"`).
/// - Verbose (`verbose = true`): prints the full anyhow chain (`{err:#}`),
///   which may include absolute paths from `notify::Error`, `std::io::Error`,
///   or similar filesystem-origin errors.
///
/// **Blind spot**: if a caller constructs context strings that embed paths
/// (e.g. `.with_context(|| format!("reading {}", path.display()))`), those
/// paths are part of the outermost message and WILL appear even at
/// `verbose = false`. The convention in `docs/guides/cli-output-streams.md`
/// forbids path-embedding context strings on error chains routed through
/// this helper — this function does not redact them automatically.
pub fn format_user_error(err: &anyhow::Error, verbose: bool) -> String {
    if verbose {
        format!("{err:#}")
    } else {
        format!("{err}")
    }
}

/// Write `data` to `path` atomically by writing to a uniquely-named temporary
/// file in the same directory and then renaming. This prevents partial/corrupt
/// state files if the process crashes or is interrupted mid-write.
///
/// Uses `tempfile` for unpredictable filenames (prevents symlink attacks).
/// On Unix the temp file is created with mode 0o600.
///
/// **Symlink-parent guard (LAUNCH-009.5):** if the immediate parent
/// directory is itself a symlink, the function refuses to write. POSIX
/// `rename(2)` replaces a symlink at the *target* path safely (the symlink
/// is destroyed, not followed), but the *temp file* is created via
/// `tempfile_in(parent)`, which writes through the parent's symlink. A
/// `~/.cursor` symlink pointing outside `$HOME` would let the install path
/// land a sensitive config file (e.g. `~/.claude.json` carries auth tokens)
/// in an unintended directory. Users with intentionally-symlinked editor
/// config dirs should resolve the symlink (`mv` the real dir into place)
/// before running `anvil start`. This is a stricter check than necessary
/// but the simplest portable guard; a per-target HOME-containment check
/// would be platform-fragile.
///
/// Note: this provides process-crash atomicity, not power-loss durability
/// (no `fsync` before rename).
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));

    refuse_if_parent_is_symlink(dir, path)?;

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

    // On Windows, restrict the file to the current user only (matching Unix 0o600).
    // icacls is available on all modern Windows (Vista+).
    #[cfg(windows)]
    {
        restrict_windows_permissions(path);
    }

    Ok(())
}

/// Refuse if `dir` (the immediate parent of the write target) exists and
/// is a symlink. A non-existent parent is allowed — `tempfile_in` will
/// surface the I/O error from there. A real directory is allowed.
///
/// See the doc comment on [`atomic_write`] for the threat model.
fn refuse_if_parent_is_symlink(dir: &Path, target: &Path) -> Result<()> {
    match std::fs::symlink_metadata(dir) {
        Ok(md) if md.file_type().is_symlink() => {
            anyhow::bail!(
                "refusing to write {} — its parent directory {} is a symlink. \
                 Resolve the symlink (move the real directory into place) \
                 and re-run.",
                target.display(),
                dir.display(),
            )
        }
        _ => Ok(()),
    }
}

/// Create `path` exclusively and write `data`. Fails with `AlreadyExists` if
/// the file is already present — use this instead of `atomic_write` when the
/// caller has already decided "only create, do not overwrite", so the
/// check-then-write window cannot be exploited by a concurrent writer.
///
/// On Unix the file is created with mode 0o600. Inherits the
/// symlink-parent guard from [`atomic_write`] — see that doc for rationale.
pub fn write_new(path: &Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    refuse_if_parent_is_symlink(dir, path)?;

    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts
        .open(path)
        .with_context(|| format!("creating {}", path.display()))?;
    file.write_all(data)
        .with_context(|| format!("writing {}", path.display()))?;
    file.flush()
        .with_context(|| format!("flushing {}", path.display()))?;

    // On Windows, restrict the file to the current user only (matching the
    // Unix 0o600 set at creation time). Best-effort; emits a warning rather
    // than failing the write if icacls is unavailable.
    #[cfg(windows)]
    {
        restrict_windows_permissions(path);
    }

    Ok(())
}

#[cfg(windows)]
fn current_user_sid() -> Result<String> {
    let output = std::process::Command::new("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .context("failed to run whoami /user")?;

    if !output.status.success() {
        anyhow::bail!("whoami /user exited with status {}", output.status);
    }

    let stdout =
        String::from_utf8(output.stdout).context("whoami /user returned non-UTF-8 output")?;

    let line = stdout
        .lines()
        .find(|l| !l.trim().is_empty())
        .context("whoami returned no user information")?;

    // CSV output: "DOMAIN\User","S-1-5-21-..."
    let trimmed = line.trim().trim_matches('"');
    let sid = trimmed
        .rsplit("\",\"")
        .next()
        .context("whoami CSV output missing SID")?;

    let is_valid_sid = sid.starts_with("S-")
        && sid
            .as_bytes()
            .iter()
            .skip(2)
            .all(|b| b.is_ascii_digit() || *b == b'-');
    if !is_valid_sid {
        anyhow::bail!("whoami returned an invalid SID: {sid}");
    }

    Ok(sid.to_string())
}

/// Restrict a file to the current user only on Windows via `icacls`.
///
/// Uses the current user's SID (via `whoami /user`) instead of the
/// USERNAME environment variable to avoid granting permissions to
/// well-known group names like "Everyone" that happen to be
/// alphanumeric. Best-effort: emits a warning to the `tracing`
/// stream if the restriction cannot be applied but does not fail
/// the write operation. This mirrors the Unix 0o600 set at creation
/// time.
#[cfg(windows)]
fn restrict_windows_permissions(path: &Path) {
    let sid = match current_user_sid() {
        Ok(sid) => sid,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "cannot restrict file permissions: could not determine current user SID",
            );
            return;
        }
    };

    let status = std::process::Command::new("icacls")
        .arg(path)
        .args(["/inheritance:r", "/grant:r"])
        .arg(format!("*{sid}:(F)"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => tracing::warn!(
            path = %path.display(),
            exit = %s,
            "failed to restrict file permissions: icacls exited non-zero",
        ),
        Err(e) => tracing::warn!(
            path = %path.display(),
            error = %e,
            "failed to run icacls",
        ),
    }
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
    fn atomic_write_refuses_when_parent_is_symlink() {
        // LAUNCH-009.5 council remediation: a symlinked parent dir would
        // let `tempfile_in` write through the link to an unintended
        // location. Refuse loudly instead of silently following.
        use std::os::unix::fs::symlink;

        let real = tempfile::tempdir().unwrap();
        let real_dir = real.path().join("real-config-dir");
        std::fs::create_dir(&real_dir).unwrap();

        let staging = tempfile::tempdir().unwrap();
        let symlinked_parent = staging.path().join("editor-config");
        symlink(&real_dir, &symlinked_parent).unwrap();

        let path = symlinked_parent.join("mcp.json");
        let err = atomic_write(&path, b"{}").expect_err("symlinked parent must be refused");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error must explain the refusal mentions symlink: {msg}"
        );
        assert!(
            !path.exists(),
            "no file should have been written when the guard refuses"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_new_refuses_when_parent_is_symlink() {
        use std::os::unix::fs::symlink;

        let real = tempfile::tempdir().unwrap();
        let real_dir = real.path().join("real-config-dir");
        std::fs::create_dir(&real_dir).unwrap();

        let staging = tempfile::tempdir().unwrap();
        let symlinked_parent = staging.path().join("editor-config");
        symlink(&real_dir, &symlinked_parent).unwrap();

        let path = symlinked_parent.join("config.json");
        let err = write_new(&path, b"{}").expect_err("symlinked parent must be refused");
        assert!(format!("{err:#}").contains("symlink"));
        assert!(!path.exists());
    }

    #[test]
    fn atomic_write_succeeds_when_parent_is_a_real_dir() {
        // Sanity guard: the symlink check must not produce false positives
        // on ordinary directories.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("subdir").join("file.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        atomic_write(&path, b"ok").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "ok");
    }

    #[test]
    fn write_new_creates_file_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fresh.rc");

        write_new(&path, b"hello").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn write_new_errors_when_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.rc");
        std::fs::write(&path, "keep-me").unwrap();

        let err = write_new(&path, b"overwrite").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("creating") || msg.contains("exists"),
            "error message should mention creation failure: {msg}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "keep-me",
            "write_new must not overwrite an existing file"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_new_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.rc");

        write_new(&path, b"secret").unwrap();

        let perms = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
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
    fn format_user_error_default_omits_chain() {
        let inner = anyhow::anyhow!("inotify: /home/victim/secret-project");
        let outer = inner.context("starting engine watcher");

        let msg = format_user_error(&outer, false);

        assert!(
            msg.contains("starting engine watcher"),
            "default mode should include the outer context: {msg}"
        );
        assert!(
            !msg.contains("/home/victim/secret-project"),
            "default mode must not leak wrapped root-cause paths: {msg}"
        );
    }

    #[test]
    fn format_user_error_verbose_includes_chain() {
        let inner = anyhow::anyhow!("inotify: /home/victim/secret-project");
        let outer = inner.context("starting engine watcher");

        let msg = format_user_error(&outer, true);

        assert!(msg.contains("starting engine watcher"), "verbose: {msg}");
        assert!(
            msg.contains("/home/victim/secret-project"),
            "verbose must include the full chain for debugging: {msg}"
        );
    }

    /// Documents the blind spot: paths embedded in the OUTER context string
    /// itself (via `.with_context(|| format!("reading {}", p.display()))`)
    /// are part of the outermost message and will leak even at
    /// `verbose = false`. The convention in `cli-output-streams.md`
    /// forbids this pattern on sites routed through `format_user_error`.
    /// This test locks the behaviour in so a future change to the helper
    /// that silently widened the contract would trip the assertion and
    /// force an explicit convention update.
    #[test]
    fn format_user_error_does_not_redact_paths_in_outer_context() {
        let err = anyhow::anyhow!("io error")
            .context(format!("reading {}", "/home/victim/secret-project"));

        let msg = format_user_error(&err, false);

        assert!(
            msg.contains("/home/victim/secret-project"),
            "path in outer context is NOT redacted — callers must avoid this pattern: {msg}"
        );
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
