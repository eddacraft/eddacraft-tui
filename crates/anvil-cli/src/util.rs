use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

const MAX_MUTATION_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// Descriptor-bound identity and digest of a regular file.
///
/// `ctime_*` defends against inode-number reuse: a same-bytes unlink+create on
/// busy tmpfs runners can recycle `(dev, ino)` while still minting a new ctime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileStamp {
    identity_a: u64,
    identity_b: u64,
    ctime_secs: i64,
    ctime_nsec: i64,
    digest: [u8; 32],
}

impl FileStamp {
    /// Identity that survives rename (ctime changes on rename).
    fn same_inode_and_digest(&self, other: &Self) -> bool {
        self.identity_a == other.identity_a
            && self.identity_b == other.identity_b
            && self.digest == other.digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObservedFile {
    pub bytes: Vec<u8>,
    pub stamp: FileStamp,
}

/// Observe a bounded regular file without following its leaf or parents.
pub(crate) fn observe_regular_nofollow(path: &Path) -> Result<ObservedFile> {
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;

        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let leaf = path
            .file_name()
            .with_context(|| format!("read path has no file name: {}", path.display()))?;
        let parent_fd = open_dir_nofollow_unix(parent)?;
        observe_regular_at_unix(parent_fd.as_fd(), leaf)
    }
    #[cfg(windows)]
    {
        let observed = anvil_intercept_win32::path_nofollow::observe_regular_nofollow(path)?;
        if observed.bytes.len() as u64 > MAX_MUTATION_FILE_BYTES {
            bail!("file exceeds {MAX_MUTATION_FILE_BYTES} byte mutation limit");
        }
        let digest = sha256_bytes(&observed.bytes);
        Ok(ObservedFile {
            bytes: observed.bytes,
            stamp: FileStamp {
                identity_a: observed.volume_serial,
                identity_b: observed.file_index,
                // Win32 file_index is stable enough; ctime is Unix-oriented.
                ctime_secs: 0,
                ctime_nsec: 0,
                digest,
            },
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        bail!("descriptor-bound file observation is unsupported on this platform")
    }
}

/// Replace an existing regular file only when its descriptor-bound identity
/// and digest still match the planning observation.
pub(crate) fn compare_and_swap_nofollow(
    path: &Path,
    expected: &ObservedFile,
    replacement: &[u8],
) -> Result<ObservedFile> {
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;

        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let leaf = path
            .file_name()
            .with_context(|| format!("write path has no file name: {}", path.display()))?;
        let parent_fd = open_dir_nofollow_unix(parent)?;
        let current = observe_regular_at_unix(parent_fd.as_fd(), leaf)?;
        if current.stamp != expected.stamp {
            bail!(
                "{} changed after planning; re-run to compute a fresh patch",
                path.display()
            );
        }
        atomic_write_at_unix(parent_fd.as_fd(), parent, leaf, replacement)?;
        observe_regular_at_unix(parent_fd.as_fd(), leaf)
    }
    #[cfg(windows)]
    {
        let observed = anvil_intercept_win32::path_nofollow::compare_and_swap_nofollow(
            path,
            expected.stamp.identity_a,
            expected.stamp.identity_b,
            &expected.bytes,
            replacement,
        )?;
        if observed.bytes.len() as u64 > MAX_MUTATION_FILE_BYTES {
            bail!("file exceeds {MAX_MUTATION_FILE_BYTES} byte mutation limit");
        }
        let digest = sha256_bytes(&observed.bytes);
        Ok(ObservedFile {
            bytes: observed.bytes,
            stamp: FileStamp {
                identity_a: observed.volume_serial,
                identity_b: observed.file_index,
                // Win32 file_index is stable enough; ctime is Unix-oriented.
                ctime_secs: 0,
                ctime_nsec: 0,
                digest,
            },
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, expected, replacement);
        bail!("descriptor-bound compare-and-swap is unsupported on this platform")
    }
}

pub(crate) fn remove_if_unchanged(path: &Path, expected: &ObservedFile) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;

        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let leaf = path
            .file_name()
            .with_context(|| format!("remove path has no file name: {}", path.display()))?;
        let parent_fd = open_dir_nofollow_unix(parent)?;
        let current = match observe_regular_at_unix(parent_fd.as_fd(), leaf) {
            Ok(current) => current,
            Err(error)
                if error.chain().any(|cause| {
                    cause
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
                }) =>
            {
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        if current.stamp != expected.stamp {
            return Ok(false);
        }
        quarantine_remove_at_unix(parent_fd.as_fd(), leaf, expected)
    }
    #[cfg(windows)]
    {
        Ok(anvil_intercept_win32::path_nofollow::remove_if_unchanged(
            path,
            expected.stamp.identity_a,
            expected.stamp.identity_b,
            &expected.bytes,
        )?)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, expected);
        bail!("descriptor-bound guarded removal is unsupported on this platform")
    }
}

#[cfg(unix)]
fn quarantine_remove_at_unix(
    parent_fd: std::os::fd::BorrowedFd<'_>,
    leaf: &std::ffi::OsStr,
    expected: &ObservedFile,
) -> Result<bool> {
    use std::os::fd::AsFd;
    use std::sync::atomic::{AtomicU64, Ordering};

    use nix::fcntl::{OFlag, openat, renameat};
    use nix::sys::stat::{Mode, mkdirat};
    use nix::unistd::{UnlinkatFlags, unlinkat};

    static QUARANTINE_COUNTER: AtomicU64 = AtomicU64::new(0);
    let quarantine = loop {
        let candidate = format!(
            ".anvil-remove-{}-{}",
            std::process::id(),
            QUARANTINE_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        match mkdirat(
            parent_fd,
            candidate.as_str(),
            Mode::from_bits_truncate(0o700),
        ) {
            Ok(()) => break candidate,
            Err(nix::errno::Errno::EEXIST) => {}
            Err(error) => return Err(std::io::Error::from(error).into()),
        }
    };
    let quarantine_fd = openat(
        parent_fd,
        quarantine.as_str(),
        OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    let candidate = std::ffi::OsStr::new("candidate");
    if let Err(error) = renameat(parent_fd, leaf, quarantine_fd.as_fd(), candidate) {
        let _ = unlinkat(parent_fd, quarantine.as_str(), UnlinkatFlags::RemoveDir);
        if error == nix::errno::Errno::ENOENT {
            return Ok(false);
        }
        return Err(std::io::Error::from(error).into());
    }

    let moved = observe_regular_at_unix(quarantine_fd.as_fd(), candidate)?;
    // Rename updates ctime; compare inode+digest only.
    if moved.stamp.same_inode_and_digest(&expected.stamp) {
        unlinkat(quarantine_fd.as_fd(), candidate, UnlinkatFlags::NoRemoveDir)
            .map_err(std::io::Error::from)?;
        unlinkat(parent_fd, quarantine.as_str(), UnlinkatFlags::RemoveDir)
            .map_err(std::io::Error::from)?;
        return Ok(true);
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        use nix::fcntl::{RenameFlags, renameat2};
        if renameat2(
            quarantine_fd.as_fd(),
            candidate,
            parent_fd,
            leaf,
            RenameFlags::RENAME_NOREPLACE,
        )
        .is_ok()
        {
            let _ = unlinkat(parent_fd, quarantine.as_str(), UnlinkatFlags::RemoveDir);
            return Ok(false);
        }
    }

    bail!("file changed during guarded removal; bytes were preserved in {quarantine}/candidate")
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    Sha256::digest(bytes).into()
}

/// Held, process-scoped exclusion for cooperating project-config writers.
///
/// The lock lives below Git's common directory, so linked worktrees contend on
/// the same OS advisory lock. Acquisition never waits: callers can surface a
/// patch and `needs_input` instead of wedging an interactive command.
#[derive(Debug)]
pub(crate) struct ConfigMutationLock {
    _file: std::fs::File,
}

impl ConfigMutationLock {
    pub(crate) fn try_acquire(repo_root: &Path) -> Result<Self> {
        let lock_path = anvil_config::mutation_lock_path(repo_root)
            .context("resolving the shared project-config mutation lock")?;
        let parent = lock_path
            .parent()
            .with_context(|| format!("lock path has no parent: {}", lock_path.display()))?;
        create_dir_all_nofollow(parent)
            .with_context(|| format!("creating lock directory {}", parent.display()))?;
        let file = open_config_lock_file(parent, &lock_path)?;
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(Self { _file: file }),
            Err(error) if is_fs2_lock_contended(&error) => bail!(
                "another anvil process is already modifying project configuration; retry after it finishes"
            ),
            Err(error) => Err(error).context("acquiring project-config mutation lock"),
        }
    }
}

/// Acquire the shared lock whenever a main-config variant already exists.
/// Absent-file creation remains available outside Git and must use an
/// exclusive writer.
pub(crate) fn lock_existing_project_config(root: &Path) -> Result<Option<ConfigMutationLock>> {
    let exists = [
        ".anvil.yaml",
        ".anvil.yml",
        ".anvil.json",
        ".anvil.toml",
        ".anvilrc",
    ]
    .into_iter()
    .any(|name| std::fs::symlink_metadata(root.join(name)).is_ok());
    if exists {
        ConfigMutationLock::try_acquire(root).map(Some)
    } else {
        Ok(None)
    }
}

/// Coordinate absent-file creation when Git metadata is available. A plain
/// directory may still create an absent file exclusively, but malformed or
/// unsafe Git metadata is never treated as "not Git".
pub(crate) fn lock_project_config_create(root: &Path) -> Result<Option<ConfigMutationLock>> {
    match anvil_config::mutation_lock_path(root) {
        Ok(_) => ConfigMutationLock::try_acquire(root).map(Some),
        Err(anvil_config::MutationLockError::MissingGitCommonDir) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn is_fs2_lock_contended(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock
        || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
}

#[cfg(unix)]
fn open_config_lock_file(parent: &Path, lock_path: &Path) -> Result<std::fs::File> {
    use std::os::fd::AsFd;

    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::{Mode, fchmod};

    let parent_fd = open_dir_nofollow_unix(parent)?;
    let leaf = lock_path
        .file_name()
        .with_context(|| format!("lock path has no file name: {}", lock_path.display()))?;
    let flags =
        OFlag::O_CREAT | OFlag::O_RDWR | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK;
    let fd = openat(
        parent_fd.as_fd(),
        leaf,
        flags,
        Mode::from_bits_truncate(0o600),
    )
    .map_err(std::io::Error::from)
    .with_context(|| format!("opening lock file {}", lock_path.display()))?;
    fchmod(&fd, Mode::from_bits_truncate(0o600))
        .map_err(std::io::Error::from)
        .with_context(|| format!("securing lock file {}", lock_path.display()))?;
    Ok(std::fs::File::from(fd))
}

#[cfg(windows)]
fn open_config_lock_file(parent: &Path, lock_path: &Path) -> Result<std::fs::File> {
    anvil_intercept_win32::path_nofollow::open_lock_file_nofollow(lock_path)
        .with_context(|| format!("opening lock file below {}", parent.display()))
}

#[cfg(not(any(unix, windows)))]
fn open_config_lock_file(parent: &Path, lock_path: &Path) -> Result<std::fs::File> {
    refuse_symlink_path_components(parent)?;
    let mut options = std::fs::OpenOptions::new();
    options.create(true).read(true).write(true).truncate(false);
    options
        .open(lock_path)
        .with_context(|| format!("opening lock file {}", lock_path.display()))
}

/// Re-export of [`anvil_kernel::watcher::filter::is_ignored_dir_name`] —
/// the canonical denylist lives in `anvil-kernel` so the watcher and the
/// cli command surfaces (`audit`, `baseline`, `check`, `drift`, `gate`)
/// cannot drift. Add new entries in `anvil-kernel/src/watcher/filter.rs`,
/// not here.
pub(crate) use anvil_kernel::watcher::filter::is_ignored_dir_name;

/// File extensions the gate/audit secret allow-list covers (plus `.env*`
/// filenames). Single definition for the #1798 lock-step: audit and gate
/// must not drift. Expanding this list is a product decision (CIB-255
/// disclosure prefers stating the domain over silent expansion).
///
/// Matching is **ASCII case-insensitive** on both the worktree (`Path`)
/// and staged/raw (`[u8]`) predicates so the same path cannot be in-domain
/// on one code path and out-of-domain on the other (e.g. `leak.TS` on
/// case-insensitive filesystems).
pub(crate) const SECRET_SCAN_EXTS: &[&str] =
    &["ts", "js", "rs", "json", "yaml", "yml", "toml", "env"];

/// True when `ext` is in [`SECRET_SCAN_EXTS`] (ASCII case-insensitive).
#[must_use]
pub(crate) fn secret_scan_ext_allowed(ext: &str) -> bool {
    SECRET_SCAN_EXTS
        .iter()
        .any(|allowed| ext.eq_ignore_ascii_case(allowed))
}

/// Byte-path variant of [`secret_scan_ext_allowed`] for staged inventory
/// paths that may not be UTF-8 as a whole but still carry an ASCII extension.
#[must_use]
pub(crate) fn secret_scan_ext_bytes_allowed(ext: &[u8]) -> bool {
    SECRET_SCAN_EXTS
        .iter()
        .any(|allowed| ext.eq_ignore_ascii_case(allowed.as_bytes()))
}

/// Build the [`anvil_checks::secret::SecretCheckConfig`] used by the
/// secret-scan surfaces (`audit`, `check`, `gate`) for a project rooted at
/// `root`.
///
/// This is the **single seam** through which project-level secret-scan
/// configuration flows. Today it returns the defaults, but consolidating the
/// three call sites here means the planned `.anvilrc` allowlist/exclude
/// surface becomes a change to *this one function* — the commands never need
/// to touch it again.
///
/// When that surface lands, map the project config's allowlist entries into
/// `SecretCheckConfig::custom_allowlist`. Suppressions from those entries are
/// already recorded with `AllowlistProvenance::Custom` and surfaced at scan
/// time (see [`secret_suppression_note`]), so an `.anvilrc` opt-out can never
/// silently hide a real credential — the operator sees every allowlisted match
/// called out, with the pattern that suppressed it.
#[must_use]
pub(crate) fn secret_check_config(_root: &Path) -> anvil_checks::secret::SecretCheckConfig {
    // EXTENSION POINT: load `.anvilrc` from `_root` and fold its secret-scan
    // allowlist into `custom_allowlist` here.
    anvil_checks::secret::SecretCheckConfig::default()
}

/// Render a one-line, non-noisy callout summarising allowlist suppressions
/// from a secret scan, or `None` when nothing was suppressed. Keeps the raw
/// total terse but breaks out the operator-configured (`.anvilrc`) count,
/// since those are the suppressions that can mask a genuine credential and so
/// must never pass unseen. Full per-entry provenance lives in the structured
/// `SecretCheckResult::suppressions` for callers that surface it.
#[must_use]
pub(crate) fn secret_suppression_note(
    suppressions: &[anvil_checks::secret::Suppression],
) -> Option<String> {
    if suppressions.is_empty() {
        return None;
    }
    let operator = suppressions
        .iter()
        .filter(|s| {
            matches!(
                s.provenance,
                anvil_checks::secret::AllowlistProvenance::Custom { .. }
            )
        })
        .count();
    let inline = suppressions
        .iter()
        .filter(|s| {
            matches!(
                s.provenance,
                anvil_checks::secret::AllowlistProvenance::InlineIgnore { .. }
            )
        })
        .count();
    let allowlist_detail = if operator > 0 {
        format!(" ({operator} via project allowlist)")
    } else {
        String::new()
    };
    let inline_detail = if inline > 0 {
        format!(" ({inline} via @anvil-ignore, still listed)")
    } else {
        String::new()
    };
    let detail = format!("{allowlist_detail}{inline_detail}");
    Some(format!(
        "ℹ {} match(es) withheld by allowlist (not flagged){detail}",
        suppressions.len()
    ))
}

/// [`secret_suppression_note`] rendered as a message suffix: a `\n\n`-separated
/// block ready to append to a check message, or an empty string when nothing
/// was suppressed.
#[must_use]
pub(crate) fn secret_suppression_suffix(
    suppressions: &[anvil_checks::secret::Suppression],
) -> String {
    secret_suppression_note(suppressions).map_or(String::new(), |note| format!("\n\n{note}"))
}

/// Render a secret scan's coverage gaps as their own block, mirroring the
/// shape `gate`'s pattern-error suffix already uses for unusable config.
/// Empty when the scan covered everything it was asked to.
///
/// SDT-006 introduced this rendering inside `gate`; SDT-008 moved it here
/// unchanged so `audit` and planless `check` report a coverage failure in
/// exactly gate's words. Three surfaces describing the same unread file three
/// different ways is the same defect class this module exists to remove — an
/// operator comparing surfaces must not have to work out whether they are
/// looking at one problem or three.
#[must_use]
pub(crate) fn secret_coverage_suffix(coverage_notes: &[String]) -> String {
    if coverage_notes.is_empty() {
        return String::new();
    }

    format!(
        "\n\n⚠ The scan could not cover everything it was asked to:\n{}",
        coverage_notes
            .iter()
            .map(|note| format!("  - {note}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// Resolve the user's home directory, honouring the platform's home
/// environment variable before the OS known-folder API.
///
/// `dirs::home_dir()` on Windows reads `FOLDERID_Profile` via the Known
/// Folder API and **ignores `%USERPROFILE%`**; on Unix it honours `$HOME`.
/// anvil reads and writes editor MCP config under the home dir
/// (`~/.cursor/mcp.json`, `~/.claude.json`) and detects installed clients
/// from there, so a home that diverges from the one the user's shell and
/// editor actually use makes anvil install to — and report on — the wrong
/// location. On Windows `%USERPROFILE%` can differ from the known-folder
/// profile (redirected/roaming/relocated profiles), which surfaced as
/// activation over-claims and "anvil installed it but my editor can't see
/// it" reports. Preferring the platform home env var keeps anvil aligned
/// with the user's environment, and lets tests isolate home via
/// `USERPROFILE`/`HOME`. Falls back to `dirs::home_dir()` when the env var
/// is unset or empty.
pub fn user_home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let from_env = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let from_env = std::env::var_os("HOME");

    from_env
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
}

fn windows_system32_executable_from(
    system_root: Option<&std::ffi::OsStr>,
    name: &str,
) -> Result<PathBuf> {
    let system_root = system_root
        .filter(|value| !value.is_empty())
        .context("SystemRoot is missing or empty")?;
    let system_root = Path::new(system_root);
    if !system_root.is_absolute() {
        bail!("SystemRoot is not an absolute path");
    }

    let executable = system_root.join("System32").join(name);
    if !executable.is_file() {
        bail!(
            "Windows system executable does not exist: {}",
            executable.display()
        );
    }
    Ok(executable)
}

fn windows_system32_executable(name: &str) -> Result<PathBuf> {
    windows_system32_executable_from(std::env::var_os("SystemRoot").as_deref(), name)
}

/// Hand a URL to the platform's default browser.
///
/// On failure returns the reason so callers can phrase their own line. A failed
/// launch is never fatal: every caller also prints the URL for the reader to
/// open themselves.
///
/// Spawning is skipped under `cfg(test)`: a test run must not open windows on
/// the developer's desktop.
pub fn open_in_browser(url: &str) -> std::result::Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    let mut command = if cfg!(target_os = "macos") {
        let mut command = std::process::Command::new("open");
        command.arg(url);
        command
    } else if cfg!(target_os = "windows") {
        let cmd = windows_system32_executable("cmd.exe").map_err(|error| error.to_string())?;
        let mut command = std::process::Command::new(cmd);
        // The empty argument is `start`'s title parameter: without it a URL
        // containing spaces or quotes would be read as the window title.
        command.args(["/C", "start", "", url]);
        command
    } else {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    };
    match command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
    {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(stderr.lines().next().unwrap_or("unknown error").to_owned())
        }
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(any(windows, test))]
fn outermost_git_workspace_boundary_with(
    canonical_cwd: &Path,
    mut has_git_marker: impl FnMut(&Path) -> bool,
) -> PathBuf {
    let mut boundary = canonical_cwd.to_path_buf();
    for ancestor in canonical_cwd.ancestors() {
        if has_git_marker(ancestor) {
            boundary = ancestor.to_path_buf();
        }
    }
    boundary
}

/// Use the outermost repository marker so an attacker-controlled nested `.git`
/// cannot shrink the exclusion boundary. Inaccessible markers are treated as
/// present so resolution fails conservatively.
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
fn outermost_git_workspace_boundary(canonical_cwd: &Path) -> PathBuf {
    outermost_git_workspace_boundary_with(
        canonical_cwd,
        |ancestor| match std::fs::symlink_metadata(ancestor.join(".git")) {
            Ok(_) => true,
            Err(error) => error.kind() != std::io::ErrorKind::NotFound,
        },
    )
}

#[cfg(any(windows, test))]
fn resolve_windows_git_program_with_boundary(
    path_entries: impl IntoIterator<Item = PathBuf>,
    cwd: &Path,
    excluded_root: &Path,
    workspace_boundary: &Path,
) -> Result<PathBuf> {
    let cwd = crate::display_path::canonicalise(cwd)
        .context("canonicalising the current directory for git resolution")?;
    let excluded_root = crate::display_path::canonicalise(excluded_root)
        .context("canonicalising the excluded workspace root for git resolution")?;
    let workspace_boundary = crate::display_path::canonicalise(workspace_boundary)
        .context("canonicalising the current workspace boundary for git resolution")?;

    for entry in path_entries {
        if entry.as_os_str().is_empty() || !entry.is_absolute() {
            continue;
        }

        let candidate = entry.join("git.exe");
        if !candidate.is_file() {
            continue;
        }
        let Ok(candidate) = crate::display_path::canonicalise(&candidate) else {
            continue;
        };
        if candidate.starts_with(&cwd)
            || candidate.starts_with(&workspace_boundary)
            || candidate.starts_with(&excluded_root)
        {
            continue;
        }
        return Ok(candidate);
    }

    bail!("no trusted git.exe found on PATH")
}

#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
fn resolve_windows_git_program(
    path_entries: impl IntoIterator<Item = PathBuf>,
    cwd: &Path,
    excluded_root: &Path,
) -> Result<PathBuf> {
    let canonical_cwd = crate::display_path::canonicalise(cwd)
        .context("canonicalising the current directory for workspace discovery")?;
    let workspace_boundary = outermost_git_workspace_boundary(&canonical_cwd);
    resolve_windows_git_program_with_boundary(
        path_entries,
        &canonical_cwd,
        excluded_root,
        &workspace_boundary,
    )
}

#[cfg(test)]
fn resolve_workspace_git_program(
    path_entries: impl IntoIterator<Item = PathBuf>,
    cwd: &Path,
    has_git_marker: impl FnMut(&Path) -> bool,
) -> Result<PathBuf> {
    let canonical_cwd =
        crate::display_path::canonicalise(cwd).context("canonicalising the test workspace")?;
    let workspace_boundary = outermost_git_workspace_boundary_with(&canonical_cwd, has_git_marker);
    resolve_windows_git_program_with_boundary(
        path_entries,
        &canonical_cwd,
        &canonical_cwd,
        &workspace_boundary,
    )
}

#[cfg(windows)]
fn git_program(excluded_root: &Path) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("resolving the current directory for git")?;
    let path = std::env::var_os("PATH")
        .filter(|value| !value.is_empty())
        .context("PATH is missing or empty")?;
    resolve_windows_git_program(std::env::split_paths(&path), &cwd, excluded_root)
}

#[cfg(not(windows))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared signature lets Windows fail closed while Unix keeps bare git"
)]
fn git_program(_excluded_root: &Path) -> Result<PathBuf> {
    Ok(PathBuf::from("git"))
}

/// Resolve the workspace root via `git rev-parse --show-toplevel`.
///
/// Canonicalises the git result to collapse symlinks. Falls back to
/// the current directory (returned as-is, not canonicalised). Returns
/// an error only when no usable path can be determined.
///
/// Canonicalisation goes through [`crate::display_path::canonicalise`] rather
/// than [`std::fs::canonicalize`] (CIB-237): the latter returns a Windows
/// NT-extended `\\?\C:\...` root, which both leaks into printed output and
/// fails to prefix-match the ordinary paths the directory walker yields.
pub fn workspace_root() -> Result<PathBuf> {
    let git_failure = match git_program(Path::new(".")).and_then(|git| {
        std::process::Command::new(git)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .map_err(anyhow::Error::from)
    }) {
        Ok(output) if output.status.success() => {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                let root = PathBuf::from(stdout.trim());
                if let Ok(canonical) = crate::display_path::canonicalise(&root) {
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

/// CIB-199: return the subset of `paths` that `.gitattributes` marks as
/// `linguist-generated` (value `true` or `set`), via one batched
/// `git check-attr`. The returned strings are exactly the input strings that
/// matched, so callers can filter with set membership whether `paths` are
/// workspace-relative or absolute (git echoes each path back verbatim).
///
/// Best-effort: any git failure (no repo, git absent, non-zero exit) yields an
/// empty set, so anti-pattern scanning behaves exactly as before wherever the
/// attribute is unused.
///
/// stdin and stdout are drained concurrently. Writing the whole path list
/// before reading `check-attr` output deadlocks once the stdout pipe fills
/// (a few hundred files).
pub(crate) fn git_generated_paths(
    root: &Path,
    paths: &[String],
) -> std::collections::HashSet<String> {
    use std::process::{Command, Stdio};

    let mut generated = std::collections::HashSet::new();
    if paths.is_empty() {
        return generated;
    }
    if !attributes_declare_linguist_generated(root) {
        return generated;
    }

    let Ok(git) = git_program(root) else {
        return generated;
    };

    let Ok(mut child) = Command::new(git)
        .arg("-C")
        .arg(root)
        .args(["check-attr", "--stdin", "-z", "linguist-generated"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return generated;
    };

    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return generated;
    };
    let Some(mut stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return generated;
    };

    let mut buf = Vec::new();
    for path in paths {
        buf.extend_from_slice(path.as_bytes());
        buf.push(0);
    }
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&buf);
        drop(stdin);
    });

    let mut stdout_buf = Vec::new();
    let read_ok = stdout.read_to_end(&mut stdout_buf).is_ok();
    let _ = writer.join();
    let status = child.wait();
    if !read_ok {
        return generated;
    }
    let Ok(status) = status else {
        return generated;
    };
    if !status.success() {
        return generated;
    }

    // `-z` output is NUL-separated triples: <path>\0<attr>\0<value>\0…
    let mut fields = stdout_buf.split(|&byte| byte == 0);
    while let (Some(path), Some(_attr), Some(value)) = (fields.next(), fields.next(), fields.next())
    {
        let value = std::str::from_utf8(value).unwrap_or_default();
        if value == "true" || value == "set" {
            generated.insert(String::from_utf8_lossy(path).into_owned());
        }
    }

    generated
}

/// True when any attributes file in `root` mentions `linguist-generated`.
/// Used to skip the `git check-attr` spawn on repos that never set the
/// attribute (the common case).
fn attributes_declare_linguist_generated(root: &Path) -> bool {
    const NEEDLE: &[u8] = b"linguist-generated";
    let contains_needle = |path: &Path| {
        std::fs::read(path)
            .is_ok_and(|bytes| bytes.windows(NEEDLE.len()).any(|window| window == NEEDLE))
    };

    if let Some(info) = git_dir_info_attributes(root)
        && contains_needle(&info)
    {
        return true;
    }

    let walker = ignore::WalkBuilder::new(root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                return !is_ignored_dir_name(&name);
            }
            true
        })
        .build();
    for entry in walker.filter_map(Result::ok) {
        if entry.file_name() != ".gitattributes" {
            continue;
        }
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        if contains_needle(entry.path()) {
            return true;
        }
    }
    false
}

fn git_dir_info_attributes(root: &Path) -> Option<PathBuf> {
    let git = root.join(".git");
    if git.is_file() {
        let text = std::fs::read_to_string(&git).ok()?;
        let line = text.lines().find_map(|l| l.strip_prefix("gitdir:"))?;
        let git_dir = PathBuf::from(line.trim());
        let git_dir = if git_dir.is_relative() {
            root.join(git_dir)
        } else {
            git_dir
        };
        Some(git_dir.join("info/attributes"))
    } else if git.is_dir() {
        Some(git.join("info/attributes"))
    } else {
        None
    }
}

/// Write `data` to `path` atomically by writing to a uniquely-named temporary
/// file in the same directory and then renaming. This prevents partial/corrupt
/// state files if the process crashes or is interrupted mid-write.
///
/// Uses `tempfile` for unpredictable filenames (prevents symlink attacks).
/// On Unix the temp file is created with mode 0o600.
///
/// Note: this provides process-crash atomicity, not power-loss durability
/// (no `fsync` before rename). Callers that need a stricter guard against
/// symlinked parent directories (e.g. the MCP install path, where the
/// target file lives in `$HOME` and a redirected parent would leak auth
/// tokens to an unintended directory) should call
/// [`refuse_if_parent_is_symlink`] before invoking this function. The
/// function intentionally does NOT enforce that guard itself — broad
/// callers (`.anvilrc`, baseline snapshots, etc.) legitimately run inside
/// symlinked workspace roots and must not be blocked.
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
        if let Err(err) = std::fs::remove_file(path)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            return Err(err).with_context(|| format!("removing existing file {}", path.display()));
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

/// Refuse if the immediate parent directory of `target` exists and is a
/// symlink.
///
/// **Threat model (LAUNCH-009.5):** the MCP install path writes editor
/// config files at `~/.cursor/mcp.json` and `~/.claude.json`. POSIX
/// `rename(2)` replaces a symlink at the *target* path safely (the
/// symlink is destroyed, not followed), but the *temp file* used by
/// `atomic_write`'s `tempfile_in(parent)` writes through the parent's
/// symlink. A `~/.cursor` symlink pointing outside `$HOME` would let
/// the install path land a sensitive config file (e.g. `.claude.json`
/// carries auth tokens) in an unintended directory.
///
/// **Scoping:** this guard is opt-in. `atomic_write` and `write_new`
/// do NOT enforce it themselves — broad callers (`.anvilrc`, baseline
/// snapshots, credential caches) legitimately run inside symlinked
/// workspace roots and refusing those would break ordinary developer
/// workflows. Only the MCP install path
/// (`activation::orchestrator::install`) calls this guard before
/// writing.
///
/// **Granularity:** stricter than necessary (a HOME-containment check
/// would be finer-grained but platform-fragile). Users with
/// intentionally-symlinked editor config dirs should resolve the
/// symlink (`mv` the real dir into place) before running
/// `anvil start`.
pub fn refuse_if_parent_is_symlink(target: &Path) -> Result<()> {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
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

/// Create every missing directory component of `path` without following
/// symlink components.
///
/// On Unix this walks the path with `openat`/`mkdirat` and
/// `O_DIRECTORY|O_NOFOLLOW`, so a concurrent swap of a checked component for
/// a symlink cannot redirect directory creation outside the intended tree.
/// One exception: an existing directory symlink that is a direct child of
/// `/` is followed. That is the OS compatibility hop (`/var` →
/// `/private/var` and `/tmp` → `/private/tmp` on macOS; usr-merge `/bin`
/// on Linux). Nested and relative symlink components stay refused.
/// On Windows the same guarantee is provided by handle-relative
/// `NtCreateFile` with `OBJ_DONT_REPARSE` (junctions and symlinks). Other
/// platforms fall back to [`std::fs::create_dir_all`] followed by a
/// best-effort symlink-component refusal.
pub fn create_dir_all_nofollow(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        create_dir_all_nofollow_unix(path)
    }
    #[cfg(windows)]
    {
        anvil_intercept_win32::path_nofollow::create_dir_all_nofollow(path)
            .with_context(|| format!("creating directory {}", path.display()))
    }
    #[cfg(not(any(unix, windows)))]
    {
        std::fs::create_dir_all(path)
            .with_context(|| format!("creating directory {}", path.display()))?;
        refuse_symlink_path_components(path)?;
        Ok(())
    }
}

/// Atomically write `data` to `path` while refusing symlink path components
/// on the way to the parent directory.
///
/// On Unix the parent is created (if needed) and opened with no-follow
/// semantics, the payload is written to a unique temp leaf via `openat`, and
/// the temp is `renameat`ed into place under the pinned parent directory fd.
/// That closes the TOCTOU window where a checked parent directory is swapped
/// for a symlink between a path-based safety check and `tempfile_in(parent)`.
///
/// On Windows the parent is pinned with `OBJ_DONT_REPARSE` and the payload
/// is renamed into place under that handle. Other platforms create parents,
/// re-check for symlink components, then delegate to [`atomic_write`].
pub fn atomic_write_nofollow(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if path.file_name().is_none() {
        bail!("write path has no file name: {}", path.display());
    }

    create_dir_all_nofollow(parent)
        .with_context(|| format!("creating directory {}", parent.display()))?;

    #[cfg(unix)]
    {
        let leaf = path
            .file_name()
            .with_context(|| format!("write path has no file name: {}", path.display()))?;
        atomic_write_nofollow_unix(parent, leaf, data)
            .with_context(|| format!("writing {}", path.display()))
    }
    #[cfg(windows)]
    {
        anvil_intercept_win32::path_nofollow::atomic_write_nofollow(path, data)
            .with_context(|| format!("writing {}", path.display()))
    }
    #[cfg(not(any(unix, windows)))]
    {
        refuse_symlink_path_components(path)?;
        // Immediate pre-write parent check: platforms without openat rely on
        // this to refuse a redirected parent before tempfile creation.
        refuse_if_parent_is_symlink(path)?;
        atomic_write(path, data)
    }
}

/// Remove a file without following symlink path components.
///
/// On Unix the parent is opened with no-follow semantics and the leaf is
/// unlinked via `unlinkat`. A swapped ancestor therefore fails closed instead
/// of deleting a file outside the intended tree. A leaf symlink is unlinked
/// itself (not its target).
///
/// Windows uses the same handle-relative `OBJ_DONT_REPARSE` walk. Other
/// platforms refuse any existing symlink component (including a leaf) then
/// call [`std::fs::remove_file`]. That remaining fallback is best-effort and
/// still has a TOCTOU window.
pub fn remove_file_nofollow(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        remove_nofollow_unix(path, false)
    }
    #[cfg(windows)]
    {
        anvil_intercept_win32::path_nofollow::remove_file_nofollow(path)
            .with_context(|| format!("removing file {}", path.display()))
    }
    #[cfg(not(any(unix, windows)))]
    {
        refuse_symlink_path_components(path)?;
        std::fs::remove_file(path).with_context(|| format!("removing file {}", path.display()))
    }
}

/// Remove an empty directory without following symlink path components.
///
/// Unix uses the same no-follow `unlinkat` walk as [`remove_file_nofollow`].
/// Windows uses handle-relative `OBJ_DONT_REPARSE`. Other platforms refuse
/// symlink components then call [`std::fs::remove_dir`], with a remaining
/// best-effort TOCTOU limit.
pub fn remove_dir_nofollow(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        remove_nofollow_unix(path, true)
    }
    #[cfg(windows)]
    {
        anvil_intercept_win32::path_nofollow::remove_dir_nofollow(path)
            .with_context(|| format!("removing directory {}", path.display()))
    }
    #[cfg(not(any(unix, windows)))]
    {
        refuse_symlink_path_components(path)?;
        std::fs::remove_dir(path).with_context(|| format!("removing directory {}", path.display()))
    }
}

/// Refuse when any existing component of `path` is a symlink.
///
/// Used by the non-Unix, non-Windows `atomic_write_nofollow` /
/// `create_dir_all_nofollow` fallbacks where handle-relative no-follow is
/// unavailable.
#[cfg(not(any(unix, windows)))]
fn refuse_symlink_path_components(path: &Path) -> Result<()> {
    let mut cursor = PathBuf::new();
    for component in path.components() {
        cursor.push(component);
        let Ok(metadata) = std::fs::symlink_metadata(&cursor) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            bail!(
                "refusing path through symlink {}: resolve the symlink and re-run",
                cursor.display()
            );
        }
    }
    Ok(())
}

#[cfg(unix)]
fn unix_dir_open_flags() -> (nix::fcntl::OFlag, nix::fcntl::OFlag) {
    use nix::fcntl::OFlag;
    let dir_flags = OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_CLOEXEC;
    (dir_flags, dir_flags | OFlag::O_NOFOLLOW)
}

#[cfg(unix)]
fn try_open_dir_component(
    parent: Option<std::os::fd::BorrowedFd<'_>>,
    name: &std::ffi::OsStr,
    flags: nix::fcntl::OFlag,
) -> std::result::Result<std::os::fd::OwnedFd, nix::errno::Errno> {
    use nix::fcntl::{open, openat};
    use nix::sys::stat::Mode;
    match parent {
        Some(dirfd) => openat(dirfd, name, flags, Mode::empty()),
        None => open(Path::new(name), flags, Mode::empty()),
    }
}

#[cfg(unix)]
fn open_or_mkdir_unix(
    parent: Option<std::os::fd::BorrowedFd<'_>>,
    name: &std::ffi::OsStr,
    existing_flags: nix::fcntl::OFlag,
    created_flags: nix::fcntl::OFlag,
) -> Result<std::os::fd::OwnedFd> {
    use nix::errno::Errno;
    use nix::sys::stat::mkdirat;

    match try_open_dir_component(parent, name, existing_flags) {
        Ok(fd) => Ok(fd),
        Err(Errno::ENOENT) => {
            // Concurrent creators are fine: if mkdir loses the race we
            // re-open the winner instead of treating EEXIST as fatal.
            let create_result = match parent {
                Some(dirfd) => {
                    mkdirat(dirfd, name, nix::sys::stat::Mode::from_bits_truncate(0o755))
                        .map_err(std::io::Error::from)
                }
                None => std::fs::create_dir(name),
            };
            match create_result {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(err) => {
                    return Err(err).with_context(|| {
                        format!("creating directory component {}", Path::new(name).display())
                    });
                }
            }
            try_open_dir_component(parent, name, created_flags)
                .map_err(std::io::Error::from)
                .with_context(|| {
                    format!(
                        "opening created directory component {}",
                        Path::new(name).display()
                    )
                })
        }
        Err(Errno::ELOOP) => bail!(
            "refusing path through symlink {}: resolve the symlink and re-run",
            Path::new(name).display()
        ),
        Err(Errno::ENOTDIR) => bail!(
            "refusing path component that is not a real directory (symlink or non-directory): {}",
            Path::new(name).display()
        ),
        Err(err) => Err(std::io::Error::from(err))
            .with_context(|| format!("opening directory component {}", Path::new(name).display())),
    }
}

#[cfg(unix)]
fn create_dir_all_nofollow_unix(path: &Path) -> Result<()> {
    use std::os::fd::{AsFd, OwnedFd};
    use std::path::Component;

    use nix::fcntl::open;
    use nix::sys::stat::Mode;

    let (dir_flags, nofollow_dir_flags) = unix_dir_open_flags();
    let mut components = path.components();
    // Follow a single existing directory symlink that is a direct child of
    // `/` (macOS `/var`, Linux usr-merge `/bin`). Nested hops stay
    // O_NOFOLLOW so a planted `escape -> /etc` still cannot redirect.
    let mut follow_root_compat = false;
    let mut dirfd: OwnedFd = match components.next() {
        Some(Component::RootDir) => {
            follow_root_compat = true;
            open(Path::new("/"), dir_flags, Mode::empty())
                .map_err(std::io::Error::from)
                .with_context(|| "opening /")?
        }
        Some(Component::CurDir) => open(Path::new("."), dir_flags, Mode::empty())
            .map_err(std::io::Error::from)
            .with_context(|| "opening current directory")?,
        Some(Component::Normal(name)) => {
            open_or_mkdir_unix(None, name, nofollow_dir_flags, nofollow_dir_flags)?
        }
        Some(Component::ParentDir) => {
            bail!("refusing path with parent-dir component {}", path.display())
        }
        Some(Component::Prefix(_)) => {
            bail!("refusing path with Windows prefix {}", path.display())
        }
        None => return Ok(()),
    };

    for component in components {
        match component {
            Component::Normal(name) => {
                let existing_flags = if follow_root_compat {
                    dir_flags
                } else {
                    nofollow_dir_flags
                };
                dirfd = open_or_mkdir_unix(
                    Some(dirfd.as_fd()),
                    name,
                    existing_flags,
                    nofollow_dir_flags,
                )?;
                follow_root_compat = false;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                bail!("refusing path with parent-dir component {}", path.display())
            }
            other => {
                bail!(
                    "refusing path with unsupported component {other:?} in {}",
                    path.display()
                )
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn open_dir_nofollow_unix(path: &Path) -> Result<std::os::fd::OwnedFd> {
    use std::os::fd::{AsFd, OwnedFd};
    use std::path::Component;

    use nix::fcntl::{OFlag, open, openat};
    use nix::sys::stat::Mode;

    let dir_flags = OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_CLOEXEC;
    let nofollow_dir_flags = dir_flags | OFlag::O_NOFOLLOW;

    let mut components = path.components();
    let mut follow_root_compat = false;
    let mut dirfd: OwnedFd = match components.next() {
        Some(Component::RootDir) => {
            follow_root_compat = true;
            open(Path::new("/"), dir_flags, Mode::empty())
                .map_err(std::io::Error::from)
                .with_context(|| "opening /")?
        }
        Some(Component::CurDir) => open(Path::new("."), dir_flags, Mode::empty())
            .map_err(std::io::Error::from)
            .with_context(|| "opening current directory")?,
        Some(Component::Normal(name)) => {
            open(Path::new(name), nofollow_dir_flags, Mode::empty())
                .map_err(|err| map_nofollow_dir_open_error(err, Path::new(name)))?
        }
        Some(Component::ParentDir) => {
            bail!("refusing path with parent-dir component {}", path.display())
        }
        Some(Component::Prefix(_)) => {
            bail!("refusing path with Windows prefix {}", path.display())
        }
        None => bail!("cannot open empty path as directory"),
    };

    for component in components {
        match component {
            Component::Normal(name) => {
                let flags = if follow_root_compat {
                    dir_flags
                } else {
                    nofollow_dir_flags
                };
                dirfd = openat(dirfd.as_fd(), name, flags, Mode::empty())
                    .map_err(|err| map_nofollow_dir_open_error(err, Path::new(name)))?;
                follow_root_compat = false;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                bail!("refusing path with parent-dir component {}", path.display())
            }
            other => {
                bail!(
                    "refusing path with unsupported component {other:?} in {}",
                    path.display()
                )
            }
        }
    }
    Ok(dirfd)
}

#[cfg(unix)]
fn map_nofollow_dir_open_error(err: nix::errno::Errno, component: &Path) -> anyhow::Error {
    use nix::errno::Errno;
    match err {
        Errno::ELOOP => anyhow::anyhow!(
            "refusing path through symlink {}: resolve the symlink and re-run",
            component.display()
        ),
        Errno::ENOTDIR => anyhow::anyhow!(
            "refusing path component that is not a real directory (symlink or non-directory): {}",
            component.display()
        ),
        other => anyhow::Error::from(std::io::Error::from(other)).context(format!(
            "opening directory component {}",
            component.display()
        )),
    }
}

#[cfg(unix)]
fn remove_nofollow_unix(path: &Path, is_dir: bool) -> Result<()> {
    use std::os::fd::AsFd;

    use nix::unistd::{UnlinkatFlags, unlinkat};

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let leaf = path
        .file_name()
        .with_context(|| format!("remove path has no file name: {}", path.display()))?;
    let dirfd = open_dir_nofollow_unix(parent)?;
    let flags = if is_dir {
        UnlinkatFlags::RemoveDir
    } else {
        UnlinkatFlags::NoRemoveDir
    };
    unlinkat(dirfd.as_fd(), leaf, flags)
        .map_err(std::io::Error::from)
        .with_context(|| {
            if is_dir {
                format!("removing directory {}", path.display())
            } else {
                format!("removing file {}", path.display())
            }
        })
}

#[cfg(unix)]
#[allow(
    clippy::useless_conversion,
    reason = "libc device and inode widths differ across supported Unix targets"
)]
fn observe_regular_at_unix(
    parent: std::os::fd::BorrowedFd<'_>,
    leaf: &std::ffi::OsStr,
) -> Result<ObservedFile> {
    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::Mode;

    let fd = openat(
        parent,
        leaf,
        OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    observe_regular_fd_unix(fd)
}

#[cfg(unix)]
#[allow(
    clippy::useless_conversion,
    reason = "libc device and inode widths differ across supported Unix targets"
)]
fn observe_regular_fd_unix(fd: std::os::fd::OwnedFd) -> Result<ObservedFile> {
    use std::io::{Seek, SeekFrom};
    use std::os::fd::AsFd;

    use nix::sys::stat::{SFlag, fstat};

    let stat = fstat(&fd).map_err(std::io::Error::from)?;
    if !SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFREG) {
        bail!("observed path is not a regular file");
    }
    let size = u64::try_from(stat.st_size).map_err(|_| anyhow::anyhow!("negative file size"))?;
    if size > MAX_MUTATION_FILE_BYTES {
        bail!("file exceeds {MAX_MUTATION_FILE_BYTES} byte mutation limit");
    }
    let capacity =
        usize::try_from(size).map_err(|_| anyhow::anyhow!("file size does not fit usize"))?;
    let mut bytes = Vec::with_capacity(capacity);
    let mut file = std::fs::File::from(fd);
    file.seek(SeekFrom::Start(0))?;
    (&mut file)
        .take(MAX_MUTATION_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_MUTATION_FILE_BYTES {
        bail!("file exceeds {MAX_MUTATION_FILE_BYTES} byte mutation limit");
    }
    // Re-stat after the read so the stamp matches the bytes we actually hashed,
    // and so a mid-read replacement cannot forge the planning identity.
    let stat = fstat(file.as_fd()).map_err(std::io::Error::from)?;
    if !SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFREG) {
        bail!("observed path is not a regular file");
    }
    let size_after =
        u64::try_from(stat.st_size).map_err(|_| anyhow::anyhow!("negative file size"))?;
    if size_after != bytes.len() as u64 {
        bail!("file changed while observing");
    }
    let identity_a = stat
        .st_dev
        .try_into()
        .map_err(|_| anyhow::anyhow!("file device identity does not fit u64"))?;
    let identity_b = stat
        .st_ino
        .try_into()
        .map_err(|_| anyhow::anyhow!("file inode identity does not fit u64"))?;
    Ok(ObservedFile {
        stamp: FileStamp {
            identity_a,
            identity_b,
            ctime_secs: stat.st_ctime,
            ctime_nsec: i64::from(stat.st_ctime_nsec),
            digest: sha256_bytes(&bytes),
        },
        bytes,
    })
}

#[cfg(unix)]
fn atomic_write_nofollow_unix(parent: &Path, leaf: &std::ffi::OsStr, data: &[u8]) -> Result<()> {
    use std::os::fd::AsFd;

    let dirfd = open_dir_nofollow_unix(parent)?;
    atomic_write_at_unix(dirfd.as_fd(), parent, leaf, data)
}

#[cfg(unix)]
fn atomic_write_at_unix(
    dirfd: std::os::fd::BorrowedFd<'_>,
    parent: &Path,
    leaf: &std::ffi::OsStr,
    data: &[u8],
) -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};

    use nix::fcntl::{OFlag, openat, renameat};
    use nix::sys::stat::{Mode, fchmod};
    use nix::unistd::{UnlinkatFlags, unlinkat};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut last_err: Option<anyhow::Error> = None;

    for attempt in 0..32u32 {
        let temp_name = format!(
            ".anvil-write-{}-{}-{attempt}.tmp",
            std::process::id(),
            nanos
        );
        let flags =
            OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_WRONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
        let fd = match openat(
            dirfd,
            temp_name.as_str(),
            flags,
            Mode::from_bits_truncate(0o600),
        ) {
            Ok(fd) => fd,
            Err(nix::errno::Errno::EEXIST) => continue,
            Err(err) => {
                return Err(std::io::Error::from(err))
                    .with_context(|| format!("creating temp file under {}", parent.display()));
            }
        };
        if let Err(err) = fchmod(&fd, Mode::from_bits_truncate(0o600)) {
            let _ = unlinkat(dirfd, temp_name.as_str(), UnlinkatFlags::NoRemoveDir);
            return Err(std::io::Error::from(err)).context("setting temp file mode 0o600");
        }

        let mut file = std::fs::File::from(fd);
        if let Err(err) = file.write_all(data).and_then(|()| file.flush()) {
            let _ = unlinkat(dirfd, temp_name.as_str(), UnlinkatFlags::NoRemoveDir);
            return Err(err).context("writing temp file payload");
        }
        drop(file);

        match renameat(dirfd, temp_name.as_str(), dirfd, leaf) {
            Ok(()) => return Ok(()),
            Err(err) => {
                let _ = unlinkat(dirfd, temp_name.as_str(), UnlinkatFlags::NoRemoveDir);
                last_err = Some(
                    anyhow::Error::from(std::io::Error::from(err)).context(format!(
                        "renaming temp file into place under {}",
                        parent.display()
                    )),
                );
            }
        }
    }

    Err(last_err.unwrap_or_else(|| {
        anyhow::anyhow!(
            "could not allocate a unique temp file under {}",
            parent.display()
        )
    }))
}

/// Create `path` exclusively and write `data`. Fails with `AlreadyExists` if
/// the file is already present — use this instead of `atomic_write` when the
/// caller has already decided "only create, do not overwrite", so the
/// check-then-write window cannot be exploited by a concurrent writer.
///
/// On Unix the file is created with mode 0o600.
#[cfg_attr(
    any(unix, windows),
    allow(dead_code, reason = "portable fallback and direct unit coverage")
)]
pub fn write_new(path: &Path, data: &[u8]) -> Result<()> {
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

/// Create a new file through a pinned, no-follow parent and never replace an
/// existing leaf.
pub fn write_new_nofollow(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|candidate| !candidate.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    create_dir_all_nofollow(parent)?;

    #[cfg(unix)]
    {
        let leaf = path
            .file_name()
            .with_context(|| format!("write path has no file name: {}", path.display()))?;
        write_new_nofollow_unix(parent, leaf, path, data, |file, payload| {
            file.write_all(payload).and_then(|()| file.flush())
        })
    }

    #[cfg(windows)]
    {
        anvil_intercept_win32::path_nofollow::write_new_nofollow(path, data)
            .with_context(|| format!("creating {}", path.display()))
    }

    #[cfg(not(any(unix, windows)))]
    {
        refuse_symlink_path_components(parent)?;
        write_new(path, data)
    }
}

#[cfg(unix)]
fn write_new_nofollow_unix<F>(
    parent: &Path,
    leaf: &std::ffi::OsStr,
    display_path: &Path,
    data: &[u8],
    writer: F,
) -> Result<()>
where
    F: FnOnce(&mut std::fs::File, &[u8]) -> std::io::Result<()>,
{
    use std::os::fd::AsFd;
    use std::sync::atomic::{AtomicU64, Ordering};

    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::{Mode, fchmod};
    use nix::unistd::{UnlinkatFlags, linkat, unlinkat};

    static CREATE_COUNTER: AtomicU64 = AtomicU64::new(0);
    let parent_fd = open_dir_nofollow_unix(parent)?;
    let flags =
        OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_RDWR | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let (temp_name, fd) = loop {
        let temp_name = format!(
            ".anvil-new-{}-{}",
            std::process::id(),
            CREATE_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        match openat(
            parent_fd.as_fd(),
            temp_name.as_str(),
            flags,
            Mode::from_bits_truncate(0o600),
        ) {
            Ok(fd) => break (temp_name, fd),
            Err(nix::errno::Errno::EEXIST) => {}
            Err(error) => {
                return Err(std::io::Error::from(error))
                    .with_context(|| format!("staging {}", display_path.display()));
            }
        }
    };
    if let Err(error) = fchmod(&fd, Mode::from_bits_truncate(0o600)) {
        let file = std::fs::File::from(fd);
        cleanup_failed_create_unix(parent_fd.as_fd(), std::ffi::OsStr::new(&temp_name), file)?;
        return Err(std::io::Error::from(error))
            .with_context(|| format!("securing {}", display_path.display()));
    }
    let mut file = std::fs::File::from(fd);
    if let Err(error) = writer(&mut file, data) {
        cleanup_failed_create_unix(parent_fd.as_fd(), std::ffi::OsStr::new(&temp_name), file)?;
        return Err(error).with_context(|| format!("writing {}", display_path.display()));
    }
    drop(file);
    if let Err(error) = linkat(
        parent_fd.as_fd(),
        temp_name.as_str(),
        parent_fd.as_fd(),
        leaf,
        nix::fcntl::AtFlags::empty(),
    ) {
        let staged = observe_regular_at_unix(parent_fd.as_fd(), std::ffi::OsStr::new(&temp_name))?;
        let _ =
            quarantine_remove_at_unix(parent_fd.as_fd(), std::ffi::OsStr::new(&temp_name), &staged);
        return Err(std::io::Error::from(error))
            .with_context(|| format!("publishing {} without replacement", display_path.display()));
    }
    unlinkat(
        parent_fd.as_fd(),
        temp_name.as_str(),
        UnlinkatFlags::NoRemoveDir,
    )
    .map_err(std::io::Error::from)
    .with_context(|| format!("removing staged hard link for {}", display_path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn cleanup_failed_create_unix(
    parent_fd: std::os::fd::BorrowedFd<'_>,
    leaf: &std::ffi::OsStr,
    file: std::fs::File,
) -> Result<()> {
    use nix::unistd::dup;

    let observed_fd = dup(&file).map_err(std::io::Error::from)?;
    let observed = observe_regular_fd_unix(observed_fd)?;
    drop(file);
    if quarantine_remove_at_unix(parent_fd, leaf, &observed)? {
        Ok(())
    } else {
        bail!("created file changed before failed-write cleanup")
    }
}

#[cfg(windows)]
fn current_user_sid() -> Result<String> {
    anvil_intercept_win32::current_user_sid()
        .context("failed to determine current user SID from process token")
}

/// Restrict a file to the current user only on Windows via `icacls`.
///
/// Uses the current user's SID from the process-token Win32 API instead of the
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

    let icacls = match windows_system32_executable("icacls.exe") {
        Ok(icacls) => icacls,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "cannot restrict file permissions: could not resolve icacls",
            );
            return;
        }
    };

    let status = std::process::Command::new(icacls)
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
    use anvil_checks::secret::{AllowlistProvenance, Suppression};
    use anvil_kernel::watcher::filter::IGNORE_DIRS;

    #[test]
    fn windows_system32_executable_requires_an_absolute_existing_file() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = temp.path().join("workspace");
        let planted_cmd = cwd.join("cmd.exe");
        let system_root = temp.path().join("Windows");
        let system32 = system_root.join("System32");
        std::fs::create_dir_all(&system32).unwrap();
        std::fs::create_dir(&cwd).unwrap();
        std::fs::write(&planted_cmd, b"").unwrap();
        let cmd = system32.join("cmd.exe");
        std::fs::write(&cmd, b"").unwrap();

        assert_eq!(
            windows_system32_executable_from(Some(system_root.as_os_str()), "cmd.exe").unwrap(),
            cmd
        );
        assert_ne!(
            cmd, planted_cmd,
            "the planted cwd cmd.exe must never be selected"
        );
        assert!(
            windows_system32_executable_from(None, "cmd.exe").is_err(),
            "a missing SystemRoot must not fall back to a bare executable"
        );
        assert!(
            windows_system32_executable_from(Some(std::ffi::OsStr::new("")), "cmd.exe").is_err(),
            "an empty SystemRoot must not fall back to a bare executable"
        );
        assert!(
            windows_system32_executable_from(
                Some(std::ffi::OsStr::new("relative-windows")),
                "cmd.exe"
            )
            .is_err(),
            "a relative SystemRoot must be rejected"
        );
        assert!(
            windows_system32_executable_from(Some(system_root.as_os_str()), "icacls.exe").is_err(),
            "a missing System32 executable must not fall back to a bare name"
        );
    }

    #[test]
    fn windows_git_resolver_rejects_unsafe_entries_and_selects_external_git() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = temp.path().join("workspace/subdir");
        let excluded_root = temp.path().join("workspace");
        let cwd_bin = cwd.join("bin");
        let excluded_bin = excluded_root.join("tools");
        let external_bin = temp.path().join("trusted-git/bin");
        for directory in [&cwd_bin, &excluded_bin, &external_bin] {
            std::fs::create_dir_all(directory).unwrap();
            std::fs::write(directory.join("git.exe"), b"").unwrap();
        }

        let unsafe_entries = vec![
            PathBuf::from("relative-bin"),
            cwd_bin.clone(),
            excluded_bin.clone(),
        ];
        assert!(
            resolve_windows_git_program_with_boundary(
                unsafe_entries.clone(),
                &cwd,
                &excluded_root,
                &excluded_root,
            )
            .is_err(),
            "relative, cwd-contained, and workspace-contained candidates must all be rejected"
        );

        let mut entries = unsafe_entries;
        entries.push(external_bin.clone());
        assert_eq!(
            resolve_windows_git_program_with_boundary(
                entries,
                &cwd,
                &excluded_root,
                &excluded_root,
            )
            .unwrap(),
            crate::display_path::canonicalise(&external_bin.join("git.exe")).unwrap(),
            "the first safe external absolute candidate must be selected"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_git_resolver_accepts_quoted_path_entries() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = temp.path().join("workspace");
        let external_bin = temp.path().join("Program Files/Git/cmd");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::create_dir_all(&external_bin).unwrap();
        std::fs::write(external_bin.join("git.exe"), b"").unwrap();

        let mut quoted_path = std::ffi::OsString::from("\"");
        quoted_path.push(external_bin.as_os_str());
        quoted_path.push("\"");
        let path_entries = std::env::split_paths(&quoted_path).collect::<Vec<_>>();

        assert_eq!(
            path_entries,
            vec![external_bin.clone()],
            "Windows split_paths must remove PATH quote delimiters"
        );
        assert_eq!(
            resolve_windows_git_program_with_boundary(path_entries, &cwd, &cwd, &cwd).unwrap(),
            crate::display_path::canonicalise(&external_bin.join("git.exe")).unwrap(),
        );
    }

    #[test]
    fn workspace_git_resolver_excludes_outermost_repo_from_nested_cwd() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let hostile_nested_repo = repo.join("nested");
        let cwd = hostile_nested_repo.join("deeper");
        let planted_bin = repo.join("tools");
        let external_bin = temp.path().join("trusted-git/bin");

        for directory in [
            repo.join(".git"),
            hostile_nested_repo.join(".git"),
            cwd.clone(),
            planted_bin.clone(),
            external_bin.clone(),
        ] {
            std::fs::create_dir_all(directory).unwrap();
        }
        std::fs::write(planted_bin.join("git.exe"), b"").unwrap();
        std::fs::write(external_bin.join("git.exe"), b"").unwrap();

        let repo_boundary = crate::display_path::canonicalise(&repo).unwrap();
        let nested_boundary = crate::display_path::canonicalise(&hostile_nested_repo).unwrap();
        assert_eq!(
            resolve_workspace_git_program([planted_bin, external_bin.clone()], &cwd, |ancestor| {
                ancestor == repo_boundary || ancestor == nested_boundary
            },)
            .unwrap(),
            crate::display_path::canonicalise(&external_bin.join("git.exe")).unwrap(),
            "a hostile nested .git marker must not shrink the excluded outer workspace"
        );
    }

    /// SDT-008: this renderer is now shared by `gate`, `audit` and planless
    /// `check`, so its exact shape is a cross-surface contract rather than one
    /// command's formatting choice.
    #[test]
    fn secret_coverage_suffix_is_empty_when_the_scan_covered_everything() {
        assert_eq!(secret_coverage_suffix(&[]), "");
    }

    #[test]
    fn secret_coverage_suffix_renders_every_note_as_its_own_bullet() {
        let rendered = secret_coverage_suffix(&[
            "1 file(s) could not be read".to_string(),
            "2 line(s) too long to scan".to_string(),
        ]);
        assert_eq!(
            rendered,
            "\n\n⚠ The scan could not cover everything it was asked to:\n  \
             - 1 file(s) could not be read\n  - 2 line(s) too long to scan"
        );
    }

    #[test]
    fn secret_scan_ext_matching_is_case_insensitive_and_shared() {
        assert!(secret_scan_ext_allowed("ts"));
        assert!(secret_scan_ext_allowed("TS"));
        assert!(secret_scan_ext_allowed("JsOn"));
        assert!(secret_scan_ext_bytes_allowed(b"ts"));
        assert!(secret_scan_ext_bytes_allowed(b"TS"));
        assert!(!secret_scan_ext_allowed("py"));
        assert!(!secret_scan_ext_bytes_allowed(b"PY"));
        // Canonical list is the only definition gate/audit share.
        assert_eq!(
            SECRET_SCAN_EXTS,
            &["ts", "js", "rs", "json", "yaml", "yml", "toml", "env"]
        );
    }

    #[test]
    fn git_generated_paths_honours_linguist_generated() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let initialised = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["init", "-q"])
            .status()
            .is_ok_and(|s| s.success());
        if !initialised {
            return; // git unavailable — the helper is best-effort, so skip.
        }
        std::fs::write(
            root.join(".gitattributes"),
            "*.gen.ts linguist-generated=true\nsrc/api.ts linguist-generated\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        for rel in ["routeTree.gen.ts", "src/api.ts", "src/app.ts"] {
            std::fs::write(root.join(rel), "x\n").unwrap();
        }

        let paths = vec![
            "routeTree.gen.ts".to_string(),
            "src/api.ts".to_string(),
            "src/app.ts".to_string(),
        ];
        let generated = git_generated_paths(root, &paths);
        assert!(
            generated.contains("routeTree.gen.ts"),
            "linguist-generated=true must be treated as generated"
        );
        assert!(
            generated.contains("src/api.ts"),
            "bare `linguist-generated` (set) must be treated as generated"
        );
        assert!(
            !generated.contains("src/app.ts"),
            "an unmarked file must not be excluded"
        );
    }

    #[test]
    fn git_generated_paths_empty_for_empty_input() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(git_generated_paths(tmp.path(), &[]).is_empty());
    }

    fn init_git_repo(root: &Path) -> bool {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["init", "-q"])
            .status()
            .is_ok_and(|s| s.success())
    }

    fn call_with_timeout(
        root: &Path,
        paths: Vec<String>,
    ) -> Result<std::collections::HashSet<String>, String> {
        let root = root.to_path_buf();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(git_generated_paths(&root, &paths));
        });
        rx.recv_timeout(std::time::Duration::from_secs(20))
            .map_err(|_| "git_generated_paths deadlocked on git check-attr pipes".to_string())
    }

    #[test]
    fn git_generated_paths_does_not_deadlock_on_large_stdin() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        if !init_git_repo(root) {
            return;
        }
        std::fs::write(
            root.join(".gitattributes"),
            "*.gen.ts linguist-generated=true\n",
        )
        .unwrap();

        let mut paths: Vec<String> = (0..3_000).map(|i| format!("src/f{i}.ts")).collect();
        paths.push("routeTree.gen.ts".to_string());

        let generated = call_with_timeout(root, paths).expect("must not deadlock");
        assert!(
            generated.contains("routeTree.gen.ts"),
            "the glob-marked generated file must still be excluded"
        );
        assert!(
            !generated.contains("src/f0.ts"),
            "unmarked files must not be excluded"
        );
    }

    #[test]
    fn git_generated_paths_skips_git_when_attribute_unused() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        if !init_git_repo(root) {
            return;
        }
        std::fs::write(root.join(".gitattributes"), "*.ts text eol=lf\n").unwrap();

        assert!(
            !attributes_declare_linguist_generated(root),
            "a gitattributes file without the attribute must not spawn check-attr"
        );

        let paths: Vec<String> = (0..3_000).map(|i| format!("src/f{i}.ts")).collect();
        let generated = call_with_timeout(root, paths).expect("must not deadlock");
        assert!(
            generated.is_empty(),
            "unused linguist-generated must exclude nothing"
        );
    }

    #[test]
    fn attributes_declare_linguist_generated_from_nested_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        if !init_git_repo(root) {
            return;
        }
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join(".gitattributes"), "*.ts text\n").unwrap();
        std::fs::write(
            root.join("src/.gitattributes"),
            "stubs.ts linguist-generated=true\n",
        )
        .unwrap();

        assert!(attributes_declare_linguist_generated(root));

        let generated = git_generated_paths(
            root,
            &["src/stubs.ts".to_string(), "src/app.ts".to_string()],
        );
        assert!(generated.contains("src/stubs.ts"));
        assert!(!generated.contains("src/app.ts"));
    }

    #[test]
    fn attributes_declare_linguist_generated_despite_gitignore() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        if !init_git_repo(root) {
            return;
        }
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join(".gitignore"), "src/\n").unwrap();
        std::fs::write(
            root.join("src/.gitattributes"),
            "stubs.ts linguist-generated=true\n",
        )
        .unwrap();

        assert!(
            attributes_declare_linguist_generated(root),
            "gitignored nested .gitattributes must still be seen, matching the source walker"
        );
    }

    #[test]
    fn attributes_declare_linguist_generated_skips_ignored_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        if !init_git_repo(root) {
            return;
        }
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::write(
            root.join("node_modules/.gitattributes"),
            "* linguist-generated=true\n",
        )
        .unwrap();

        assert!(
            !attributes_declare_linguist_generated(root),
            "canonical ignore-dirs must not trigger a check-attr spawn"
        );
    }

    fn suppression(provenance: AllowlistProvenance) -> Suppression {
        Suppression {
            file: "src/x.rs".to_string(),
            line: 1,
            rule_name: "High Entropy String".to_string(),
            redacted_match: "ab...yz".to_string(),
            provenance,
        }
    }

    #[test]
    fn suppression_note_is_none_when_empty() {
        assert!(secret_suppression_note(&[]).is_none());
    }

    #[test]
    fn suppression_note_counts_total_and_omits_operator_detail_for_builtins() {
        let note = secret_suppression_note(&[
            suppression(AllowlistProvenance::BuiltinShape),
            suppression(AllowlistProvenance::BuiltinKeyword),
        ])
        .expect("note for non-empty suppressions");
        assert!(note.contains("2 match(es)"), "got: {note}");
        assert!(
            !note.contains("project allowlist"),
            "built-in-only suppressions must not claim a project allowlist: {note}"
        );
    }

    #[test]
    fn suppression_note_breaks_out_operator_configured_count() {
        let note = secret_suppression_note(&[
            suppression(AllowlistProvenance::BuiltinShape),
            suppression(AllowlistProvenance::Custom {
                pattern: "diag_".to_string(),
            }),
        ])
        .expect("note");
        assert!(note.contains("2 match(es)"), "got: {note}");
        assert!(
            note.contains("1 via project allowlist"),
            "operator-configured suppressions must be called out: {note}"
        );
    }

    #[test]
    fn suppression_note_breaks_out_inline_ignore_count() {
        let note = secret_suppression_note(&[suppression(AllowlistProvenance::InlineIgnore {
            rule_id: "SECRET-HIGH-ENTROPY-STRING".to_string(),
            reason: "fixture".to_string(),
        })])
        .expect("note");
        assert!(note.contains("1 match(es)"), "got: {note}");
        assert!(
            note.contains("1 via @anvil-ignore, still listed"),
            "inline ignores must stay visible: {note}"
        );
    }

    #[test]
    fn is_ignored_dir_name_matches_full_list() {
        for entry in IGNORE_DIRS {
            assert!(is_ignored_dir_name(entry), "expected {entry} to be ignored");
        }
    }

    #[test]
    fn is_ignored_dir_name_rejects_unknown() {
        for name in ["src", "tests", "lib", "node_modules.bak", "Target", ""] {
            assert!(
                !is_ignored_dir_name(name),
                "expected {name} to not be ignored"
            );
        }
    }

    /// ADOPT-004: cli's `is_ignored_dir_name` is the public re-export of
    /// the kernel-owned canonical list. They must agree on every entry so
    /// audit/baseline/check/drift/gate (cli consumers) and the watcher
    /// (kernel consumer) cannot diverge.
    #[test]
    fn cli_helper_matches_kernel_canonical() {
        for entry in IGNORE_DIRS {
            assert!(
                is_ignored_dir_name(entry),
                "cli is_ignored_dir_name disagrees with kernel canonical for {entry}",
            );
        }
        for name in ["src", "tests", "lib", "node_modules.bak", "Target"] {
            assert!(
                !is_ignored_dir_name(name),
                "cli is_ignored_dir_name should not match {name}",
            );
        }
    }

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
    fn refuse_if_parent_is_symlink_blocks_symlinked_parent() {
        // LAUNCH-009.5 council remediation: a symlinked parent dir would
        // let `tempfile_in` write through the link to an unintended
        // location. Callers that need this guard (currently the MCP
        // install path) opt in before invoking atomic_write.
        use std::os::unix::fs::symlink;

        let real = tempfile::tempdir().unwrap();
        let real_dir = real.path().join("real-config-dir");
        std::fs::create_dir(&real_dir).unwrap();

        let staging = tempfile::tempdir().unwrap();
        let symlinked_parent = staging.path().join("editor-config");
        symlink(&real_dir, &symlinked_parent).unwrap();

        let path = symlinked_parent.join("mcp.json");
        let err = refuse_if_parent_is_symlink(&path).expect_err("symlinked parent must be refused");
        let msg = format!("{err:#}");
        assert!(msg.contains("symlink"), "error must mention symlink: {msg}");
    }

    #[test]
    fn refuse_if_parent_is_symlink_passes_through_real_dir() {
        // Regression guard: the helper must not produce false positives
        // on ordinary directories. atomic_write is used by `.anvilrc`,
        // baseline snapshots, etc. — those paths are NOT installed
        // through this guard, but we still verify the helper itself
        // behaves correctly on real dirs in case a future caller opts in.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.json");
        refuse_if_parent_is_symlink(&path).unwrap();
    }

    #[test]
    fn refuse_if_parent_is_symlink_passes_when_parent_does_not_exist() {
        // The helper should not error when the parent dir doesn't exist
        // yet — the caller will create it via create_dir_all and then
        // re-check (or the subsequent write will surface the I/O error).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent").join("file.json");
        refuse_if_parent_is_symlink(&path).unwrap();
    }

    #[test]
    fn atomic_write_succeeds_in_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("subdir").join("file.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        atomic_write(&path, b"ok").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "ok");
    }

    #[test]
    fn atomic_write_nofollow_creates_nested_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.json");
        atomic_write_nofollow(&path, b"{\"ok\":true}").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"ok\":true}");
    }

    #[test]
    fn atomic_write_nofollow_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "old").unwrap();
        atomic_write_nofollow(&path, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_nofollow_refuses_symlinked_parent_without_writing_outside() {
        use std::os::unix::fs::symlink;

        let outside = tempfile::tempdir().unwrap();
        let outside_marker = outside.path().join("leaked.json");

        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join(".cursor");
        symlink(outside.path(), &parent).unwrap();

        let path = parent.join("mcp.json");
        let err = atomic_write_nofollow(&path, b"secret").expect_err("symlinked parent");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error should identify symlink refusal: {msg}"
        );
        assert!(
            !outside_marker.exists(),
            "must not create a file outside the intended root"
        );
        assert!(
            std::fs::read_dir(outside.path()).unwrap().next().is_none(),
            "outside directory must remain untouched"
        );
    }

    #[cfg(unix)]
    #[test]
    fn create_dir_all_nofollow_walks_os_root_compat_directory_symlink() {
        // Nightly macOS Node tests failed because /var is a symlink to
        // /private/var (same class as Linux usr-merge /bin -> usr/bin).
        // The nofollow walk must follow that single root hop so tempfile
        // paths work, without following attacker-planted nested links.
        #[cfg(target_os = "macos")]
        {
            assert!(
                std::fs::symlink_metadata("/var").is_ok_and(|meta| meta.file_type().is_symlink()),
                "macOS /var must be a directory symlink; this is the nightly regression"
            );
        }
        let candidates = [Path::new("/bin"), Path::new("/var"), Path::new("/tmp")];
        let Some(compat) = candidates.into_iter().find(|path| {
            std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
        }) else {
            return;
        };
        create_dir_all_nofollow(compat).unwrap_or_else(|err| {
            panic!(
                "OS root compatibility symlink {} must be walkable: {err:#}",
                compat.display()
            )
        });
    }

    #[cfg(unix)]
    #[test]
    fn create_dir_all_nofollow_refuses_symlink_component_without_creating_outside() {
        use std::os::unix::fs::symlink;

        let outside = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let link = root.path().join("escape");
        symlink(outside.path(), &link).unwrap();

        let target = link.join("child");
        let err = create_dir_all_nofollow(&target).expect_err("symlink component");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error should mention symlink: {msg}"
        );
        assert!(
            msg.contains("escape"),
            "must refuse the nested planted name, not an ancestor OS hop: {msg}"
        );
        assert!(
            !outside.path().join("child").exists(),
            "must not create directories through the symlink"
        );
    }

    #[cfg(windows)]
    fn plant_junction(link: &std::path::Path, target: &std::path::Path) -> bool {
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .is_ok_and(|status| status.success())
    }

    #[cfg(windows)]
    #[test]
    fn atomic_write_nofollow_refuses_junctioned_parent_without_writing_outside() {
        let outside = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join(".cursor");
        assert!(
            plant_junction(&parent, outside.path()),
            "mklink /J creates a junction without privilege"
        );

        let path = parent.join("mcp.json");
        let err = atomic_write_nofollow(&path, b"secret").expect_err("junctioned parent");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error should identify symlink refusal: {msg}"
        );
        assert!(
            std::fs::read_dir(outside.path()).unwrap().next().is_none(),
            "outside directory must remain untouched"
        );
    }

    #[cfg(windows)]
    #[test]
    fn create_dir_all_nofollow_refuses_junction_component_without_creating_outside() {
        let outside = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let link = root.path().join("escape");
        assert!(
            plant_junction(&link, outside.path()),
            "mklink /J creates a junction without privilege"
        );

        let target = link.join("child");
        let err = create_dir_all_nofollow(&target).expect_err("junction component");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error should mention symlink: {msg}"
        );
        assert!(
            !outside.path().join("child").exists(),
            "must not create directories through the junction"
        );
    }

    #[test]
    fn remove_file_nofollow_removes_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doomed.txt");
        std::fs::write(&path, b"x").unwrap();
        remove_file_nofollow(&path).unwrap();
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn remove_file_nofollow_refuses_symlinked_parent_without_deleting_outside() {
        use std::os::unix::fs::symlink;

        let outside = tempfile::tempdir().unwrap();
        let marker = outside.path().join("victim.txt");
        std::fs::write(&marker, b"keep").unwrap();

        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join(".anvil");
        symlink(outside.path(), &parent).unwrap();

        let err = remove_file_nofollow(&parent.join("victim.txt")).expect_err("symlinked parent");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error should identify symlink: {msg}"
        );
        assert_eq!(std::fs::read(&marker).unwrap(), b"keep");
    }

    #[test]
    fn remove_dir_nofollow_removes_an_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("empty");
        std::fs::create_dir(&child).unwrap();
        remove_dir_nofollow(&child).unwrap();
        assert!(!child.exists());
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
    fn write_new_nofollow_refuses_symlinked_parent() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), root.path().join("anvil")).unwrap();

        let error = write_new_nofollow(&root.path().join("anvil/policy.yml"), b"policy")
            .expect_err("symlinked parent");

        assert!(format!("{error:#}").contains("symlink"));
        assert!(!outside.path().join("policy.yml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn exclusive_nofollow_create_publishes_complete_bytes_without_staging_debris() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config");

        write_new_nofollow(&path, b"complete").unwrap();
        let error = write_new_nofollow(&path, b"replacement").unwrap_err();

        assert_eq!(std::fs::read(&path).unwrap(), b"complete");
        assert!(format!("{error:#}").contains("without replacement"));
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn descriptor_bound_compare_and_swap_rejects_changed_bytes_and_identity() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config");
        std::fs::write(&path, b"before").unwrap();
        let observed = observe_regular_nofollow(&path).unwrap();

        std::fs::write(&path, b"changed").unwrap();
        assert!(compare_and_swap_nofollow(&path, &observed, b"after").is_err());

        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"before").unwrap();
        assert!(compare_and_swap_nofollow(&path, &observed, b"after").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"before");
    }

    #[cfg(unix)]
    #[test]
    fn descriptor_bound_compare_and_swap_and_guarded_remove_succeed() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config");
        std::fs::write(&path, b"before").unwrap();
        let before = observe_regular_nofollow(&path).unwrap();
        let published = compare_and_swap_nofollow(&path, &before, b"after").unwrap();
        assert_eq!(published.bytes, b"after");
        assert!(remove_if_unchanged(&path, &published).unwrap());
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn exclusive_create_removes_partial_leaf_after_payload_failure() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("partial");
        let error = write_new_nofollow_unix(
            root.path(),
            std::ffi::OsStr::new("partial"),
            &path,
            b"complete payload",
            |file, _| {
                file.write_all(b"prefix")?;
                Err(std::io::Error::other("injected payload failure"))
            },
        )
        .unwrap_err();

        assert!(
            format!("{error:#}").contains("injected payload failure"),
            "{error:#}"
        );
        assert!(!path.exists());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn config_mutation_lock_is_shared_and_non_blocking() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir(repo.path().join(".git")).unwrap();

        let first = ConfigMutationLock::try_acquire(repo.path()).expect("first lock");
        let error = ConfigMutationLock::try_acquire(repo.path()).expect_err("contended lock");

        assert!(
            format!("{error:#}").contains("already modifying project configuration"),
            "unexpected contention error: {error:#}"
        );
        drop(first);
        ConfigMutationLock::try_acquire(repo.path()).expect("released lock");
    }

    #[cfg(unix)]
    #[test]
    fn config_mutation_lock_refuses_symlinked_lock_directory() {
        use std::os::unix::fs::symlink;

        let repo = tempfile::tempdir().unwrap();
        let git_dir = repo.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), git_dir.join("anvil")).unwrap();

        let error = ConfigMutationLock::try_acquire(repo.path()).expect_err("symlink refused");

        assert!(format!("{error:#}").contains("symlink"));
        assert!(!outside.path().join("config-mutation.lock").exists());
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
        // Product uses display_path::canonicalise (dunce), not raw
        // std::fs::canonicalize — the latter yields \\?\C:\... on Windows while
        // the product returns the ordinary form (CIB-237 path honesty).
        if let Ok(canonical) = crate::display_path::canonicalise(&root) {
            assert_eq!(root, canonical);
        }
    }
}
