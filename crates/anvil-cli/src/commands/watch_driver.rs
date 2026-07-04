//! DSV-048 (ADR-101): the headless save-time driver side of `anvil watch`.
//!
//! The intercept daemon's `SaveTimeDriverSupervisor` (DSV-047) spawns one
//! detached `anvil watch --save-time-driver --worktree <canonical-root>`
//! child per durable registered worktree. This module owns the driver-side
//! pieces of that contract:
//!
//! - **The findings log.** The child owns the findings log end-to-end —
//!   open, append, rotate at [`MAX_LOG_BYTES`] — via [`DriverLog`]. The
//!   supervisor only redirects the child's stdout/stderr to a *separate*
//!   crash-capture file (`<worktree-id>.spawn.log`); it never writes to the
//!   findings log (single-writer rule — rotation under a supervisor-held
//!   redirect fd is the failure mode this split avoids).
//! - **Log-path resolution.** The supervisor hands the path down via
//!   [`DRIVER_LOG_ENV`]; the default (manual runs, tests) lands under the
//!   per-user runtime directory, mirroring the daemon's PID-file precedence
//!   (`ANVIL_HOME` → `XDG_RUNTIME_DIR` → `LOCALAPPDATA` → `~/.local/state`).
//!
//! Degraded-path note: when daemon routing is unavailable the dispatcher's
//! scoped subprocess fallback inherits stdout/stderr, so its findings land in
//! the supervisor's crash-capture file rather than the findings log. Driver
//! mode assumes the parent daemon is live; the fallback exists so a saved
//! file is still checked, not to keep the findings log complete.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Environment variable through which the supervisor (DSV-047) hands the
/// findings-log path to the driver child.
pub(crate) const DRIVER_LOG_ENV: &str = "ANVIL_SAVE_TIME_DRIVER_LOG";

/// Rotation threshold for the findings log (design spec §Findings without a
/// terminal: "rotated/truncated at 1 MiB").
const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Append-only findings log owned by the driver child.
///
/// Each append re-opens the file, which keeps rotation trivially safe: no
/// long-lived fd can point at a renamed file. Appends happen at most once per
/// debounced save batch, so the reopen cost is irrelevant.
#[derive(Debug)]
pub(crate) struct DriverLog {
    path: PathBuf,
}

impl DriverLog {
    pub(crate) const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Append one rendered block to the log, rotating first when the current
    /// file has reached [`MAX_LOG_BYTES`]. Creates the parent directory on
    /// first use (the default-path case; the supervisor pre-creates its own
    /// runtime dir).
    pub(crate) fn append(&self, block: &[u8]) -> std::io::Result<()> {
        use std::io::Write as _;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.rotate_if_needed()?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(block)
    }

    /// Rename the live log to `<path>.1` (replacing any prior rotation) once
    /// it reaches the size cap, so the next append starts a fresh file while
    /// the most recent history survives one generation. The prior `.1` is
    /// removed first: Windows `rename` fails over an existing destination
    /// (unlike POSIX), and a failed rotation must not wedge appends.
    fn rotate_if_needed(&self) -> std::io::Result<()> {
        match std::fs::metadata(&self.path) {
            Ok(meta) if meta.len() >= MAX_LOG_BYTES => {
                let mut rotated = self.path.clone().into_os_string();
                rotated.push(".1");
                let rotated = PathBuf::from(rotated);
                match std::fs::remove_file(&rotated) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e),
                }
                std::fs::rename(&self.path, rotated)
            }
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// The one-line stderr summary emitted when a batch produces new findings.
/// Stderr is the supervisor's crash-capture channel, so this doubles as a
/// liveness breadcrumb without duplicating the full finding lines.
pub(crate) fn driver_summary_line(finding_count: usize, log_path: &Path) -> String {
    format!(
        "anvil watch (save-time driver): {finding_count} new finding(s) — see {}",
        log_path.display()
    )
}

/// Resolve the findings-log path for a driver run: the supervisor-provided
/// [`DRIVER_LOG_ENV`] wins; otherwise a per-worktree default under the
/// runtime directory.
pub(crate) fn resolve_driver_log_path(worktree: &Path) -> Result<PathBuf> {
    resolve_driver_log_path_from(
        std::env::var_os(DRIVER_LOG_ENV).filter(|v| !v.is_empty()),
        non_empty_env("ANVIL_HOME"),
        non_empty_env("XDG_RUNTIME_DIR"),
        if cfg!(windows) {
            non_empty_env("LOCALAPPDATA")
        } else {
            None
        },
        non_empty_env("HOME").or_else(|| non_empty_env("USERPROFILE")),
        worktree,
    )
}

/// Pure resolver for [`resolve_driver_log_path`] — candidate roots are passed
/// explicitly so it unit-tests without mutating the process environment. The
/// default-root precedence mirrors the daemon's PID-file resolution
/// (`anvil_intercept::default_pid_file_path`) so driver artefacts sit beside
/// the daemon's runtime state: `{ANVIL_HOME}/runtime/save-time-drivers/`
/// (ADR-101), else `$XDG_RUNTIME_DIR/anvil/save-time-drivers/`, else
/// `%LOCALAPPDATA%\anvil\save-time-drivers\`, else
/// `~/.local/state/anvil/save-time-drivers/`.
fn resolve_driver_log_path_from(
    env_override: Option<std::ffi::OsString>,
    anvil_home: Option<PathBuf>,
    xdg_runtime_dir: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
    home: Option<PathBuf>,
    worktree: &Path,
) -> Result<PathBuf> {
    if let Some(path) = env_override {
        return Ok(PathBuf::from(path));
    }
    let dir = if let Some(prefix) = anvil_home {
        prefix.join("runtime").join("save-time-drivers")
    } else if let Some(runtime_dir) = xdg_runtime_dir {
        runtime_dir.join("anvil").join("save-time-drivers")
    } else if let Some(local_app_data) = local_app_data {
        local_app_data.join("anvil").join("save-time-drivers")
    } else {
        home.context("cannot resolve home directory for the save-time driver log")?
            .join(".local")
            .join("state")
            .join("anvil")
            .join("save-time-drivers")
    };
    Ok(dir.join(worktree_log_file_name(worktree)))
}

fn non_empty_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Filename-safe, collision-resistant log name for a worktree: the leaf
/// directory name (for humans) plus a 12-hex prefix of the SHA-256 of the
/// canonical path (for uniqueness across same-named worktrees). Stable across
/// runs so restarts append to the same log.
fn worktree_log_file_name(worktree: &Path) -> String {
    use sha2::{Digest as _, Sha256};
    let digest = Sha256::digest(worktree.as_os_str().as_encoded_bytes());
    let mut hex = String::with_capacity(12);
    for byte in &digest[..6] {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    let leaf = worktree.file_name().map_or_else(
        || "worktree".to_owned(),
        |n| n.to_string_lossy().replace(['/', '\\', ':', ' '], "-"),
    );
    format!("{leaf}-{hex}.log")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_save_time_driver_log_appends_and_creates_parents() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = DriverLog::new(dir.path().join("nested").join("wt.log"));
        log.append(b"finding line one\n").expect("first append");
        log.append(b"finding line two\n").expect("second append");
        let content = std::fs::read_to_string(log.path()).expect("read log");
        assert_eq!(content, "finding line one\nfinding line two\n");
    }

    #[test]
    fn watch_save_time_driver_log_rotates_at_cap() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("wt.log");
        let log = DriverLog::new(path.clone());
        // Fill past the cap in one append, then trigger rotation on the next.
        let big = vec![b'x'; usize::try_from(MAX_LOG_BYTES).expect("cap fits usize")];
        log.append(&big).expect("filling append");
        log.append(b"fresh line\n").expect("post-cap append");

        let rotated = std::fs::read(dir.path().join("wt.log.1")).expect("rotated file exists");
        assert_eq!(rotated.len() as u64, MAX_LOG_BYTES, "history moved aside");
        let fresh = std::fs::read_to_string(&path).expect("fresh log");
        assert_eq!(fresh, "fresh line\n", "live log restarted after rotation");

        // A SECOND rotation must replace the existing `.1` (Windows `rename`
        // fails over an existing destination; the remove-then-rename must
        // keep appends working after the first generation).
        let big2 = vec![b'y'; usize::try_from(MAX_LOG_BYTES).expect("cap fits usize")];
        log.append(&big2).expect("second filling append");
        log.append(b"second fresh\n")
            .expect("append after second rotation");
        let rotated2 = std::fs::read(dir.path().join("wt.log.1")).expect(".1 replaced");
        assert!(
            rotated2.starts_with(b"fresh line\n") && rotated2.ends_with(b"yy"),
            "second generation replaced the first"
        );
        let fresh2 = std::fs::read_to_string(&path).expect("fresh log 2");
        assert_eq!(fresh2, "second fresh\n");
    }

    #[test]
    fn watch_save_time_driver_log_env_override_wins() {
        let resolved = resolve_driver_log_path_from(
            Some(std::ffi::OsString::from("/custom/driver.log")),
            Some(PathBuf::from("/anvil-home")),
            Some(PathBuf::from("/run/user/1000")),
            None,
            Some(PathBuf::from("/home/u")),
            Path::new("/ws/repo"),
        )
        .expect("resolve");
        assert_eq!(resolved, PathBuf::from("/custom/driver.log"));
    }

    #[test]
    fn watch_save_time_driver_default_path_precedence() {
        // ANVIL_HOME re-roots under <prefix>/runtime/ (ADR-101 / DISTRIB-006).
        let with_home = resolve_driver_log_path_from(
            None,
            Some(PathBuf::from("/anvil-home")),
            Some(PathBuf::from("/run/user/1000")),
            None,
            Some(PathBuf::from("/home/u")),
            Path::new("/ws/repo"),
        )
        .expect("resolve");
        assert!(
            with_home.starts_with("/anvil-home/runtime/save-time-drivers"),
            "ANVIL_HOME wins: {}",
            with_home.display()
        );

        // XDG runtime dir is the platform default beneath it.
        let with_xdg = resolve_driver_log_path_from(
            None,
            None,
            Some(PathBuf::from("/run/user/1000")),
            None,
            Some(PathBuf::from("/home/u")),
            Path::new("/ws/repo"),
        )
        .expect("resolve");
        assert!(
            with_xdg.starts_with("/run/user/1000/anvil/save-time-drivers"),
            "XDG fallback: {}",
            with_xdg.display()
        );
    }

    #[test]
    fn watch_save_time_driver_log_name_is_stable_and_distinct() {
        let a1 = worktree_log_file_name(Path::new("/ws/repo"));
        let a2 = worktree_log_file_name(Path::new("/ws/repo"));
        let b = worktree_log_file_name(Path::new("/elsewhere/repo"));
        assert_eq!(a1, a2, "stable across runs");
        assert_ne!(a1, b, "same leaf, different roots must not collide");
        assert!(a1.starts_with("repo-"), "{a1}");
        assert_eq!(
            Path::new(&a1).extension(),
            Some(std::ffi::OsStr::new("log")),
            "{a1}"
        );
    }

    #[test]
    fn watch_save_time_driver_summary_line_names_the_log() {
        let line = driver_summary_line(3, Path::new("/logs/wt.log"));
        assert_eq!(
            line,
            "anvil watch (save-time driver): 3 new finding(s) — see /logs/wt.log"
        );
    }
}
