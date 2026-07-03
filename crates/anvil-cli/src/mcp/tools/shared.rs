//! Shared helpers for MCP tools.
//!
//! The pre-RMCPF-010 status tool, plus the new check/gate tools, all share
//! the same workspace-containment, redaction, and warning-shape concerns.
//! Keeping the helpers in one place stops the per-tool copies from drifting
//! apart and makes any future change (e.g. tighter symlink containment, new
//! redaction rule) land in a single location.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use anvil_checks::antipattern::{Warning, WarningCategory, WarningSeverity};

/// Maximum number of entries any MCP tool will accept in a file array.
/// Stops a hostile or buggy caller from forcing an arbitrarily large
/// `Vec::with_capacity` and per-entry path-join allocation before the
/// scanner runs.
pub const MAX_FILE_ENTRIES: usize = 10_000;

/// Validate that `workspace_root` resolves to a directory inside
/// `server_root`. Returns the canonicalised `(server_root, workspace_root)`
/// pair on success.
pub fn validate_workspace_root(
    workspace_root: &Path,
    server_root: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    if !workspace_root.is_absolute() {
        return Err("workspaceRoot must be an absolute path".to_string());
    }
    if !workspace_root.exists() {
        return Err("workspaceRoot does not exist".to_string());
    }
    if !workspace_root.is_dir() {
        return Err("workspaceRoot must be a directory".to_string());
    }
    let server_root = server_root
        .canonicalize()
        .map_err(|err| format!("MCP server root is not accessible: {err}"))?;
    let workspace_root = workspace_root
        .canonicalize()
        .map_err(|err| format!("workspaceRoot is not accessible: {err}"))?;
    if !workspace_root.starts_with(&server_root) {
        return Err("workspaceRoot must be inside the MCP server root".to_string());
    }
    Ok((server_root, workspace_root))
}

/// Redact an absolute workspace path to a server-root-relative form.
pub fn redact_workspace_root(workspace_root: &Path, server_root: &Path) -> String {
    let relative = workspace_root
        .strip_prefix(server_root)
        .expect("workspace root containment must be validated before redaction");

    if relative.as_os_str().is_empty() {
        ".".to_string()
    } else {
        relative.to_string_lossy().replace('\\', "/")
    }
}

/// Collect the workspace-relative file list from a JSON array, rejecting
/// shapes that the MCP tool surface should not accept: non-strings, empty
/// entries, absolute paths, `..` escapes, and arrays larger than
/// [`MAX_FILE_ENTRIES`].
///
/// `field` is the name of the JSON property (e.g. `"files"` or
/// `"targetFiles"`) so the error messages stay parity-shaped with the
/// archived TS contract.
pub fn collect_relative_files(files: &[Value], field: &str) -> Result<Vec<String>, String> {
    if files.len() > MAX_FILE_ENTRIES {
        return Err(format!(
            "{field} must contain at most {MAX_FILE_ENTRIES} entries"
        ));
    }
    let mut out = Vec::with_capacity(files.len());
    for (index, entry) in files.iter().enumerate() {
        let path = entry
            .as_str()
            .ok_or_else(|| format!("{field}[{index}] must be a string"))?;
        if path.is_empty() {
            return Err(format!("{field}[{index}] must not be empty"));
        }
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            return Err(format!(
                "{field}[{index}] must be a workspace-relative path"
            ));
        }
        if candidate
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(format!(
                "{field}[{index}] must not escape the workspace via \"..\""
            ));
        }
        out.push(path.to_string());
    }
    Ok(out)
}

/// Resolve workspace-relative file entries to canonical absolute paths
/// that are guaranteed to live inside `workspace_root`. Symlinks are
/// followed and their targets are re-validated, so a relative entry that
/// points at a symlink targeting `/etc/passwd` is rejected rather than
/// silently read.
///
/// Missing files are tolerated with the empty string at their slot so the
/// scanner's own "file not found" path can produce a parity-shaped
/// response — exactly mirroring the archived TS tool, which delegates to
/// `GateRunner.analyzeFiles` without pre-checking existence.
pub fn resolve_workspace_files(
    workspace_root: &Path,
    relative: &[String],
    field: &str,
) -> Result<Vec<String>, String> {
    let mut absolute = Vec::with_capacity(relative.len());
    for (index, rel) in relative.iter().enumerate() {
        let joined = workspace_root.join(rel);
        let resolved = match joined.canonicalize() {
            Ok(path) => path,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                // Forward the workspace-joined path verbatim. The scanner
                // surfaces a per-file read error in its own envelope.
                absolute.push(joined.to_string_lossy().to_string());
                continue;
            }
            Err(err) => {
                return Err(format!("{field}[{index}] is not accessible: {err}"));
            }
        };
        if !resolved.starts_with(workspace_root) {
            return Err(format!("{field}[{index}] resolves outside workspaceRoot"));
        }
        absolute.push(resolved.to_string_lossy().to_string());
    }
    Ok(absolute)
}

/// Open an already-canonicalised, containment-checked file for read+write
/// so an in-place read-modify-write cannot be redirected by a symlink
/// swapped in **after** the containment check — the classic check-then-use
/// (TOCTOU) window that a path-based `fs::write(resolved, …)` re-opens every
/// time it re-walks the path string.
///
/// Callers that may perform a NO-OP (read, decide nothing needs writing)
/// should read through [`open_contained_ro_handle`] first and only escalate
/// to this read+write open when a change is actually required — opening
/// read+write up front would turn a no-op on a read-only file into a
/// permission error. Each open is independently hardened, so escalating to a
/// second open preserves the containment invariant.
///
/// See [`open_contained_handle`] for the full contract and the per-platform
/// residual table.
pub fn open_contained_rw_handle(resolved: &Path, workspace_root: &Path) -> Result<File, String> {
    open_contained_handle(resolved, workspace_root, true)
}

/// Read-only counterpart to [`open_contained_rw_handle`], hardened the same
/// way (`O_NOFOLLOW` + best-effort fd path re-check). Use this to read a file
/// whose modification is conditional, so a no-op does not require write
/// permission.
pub fn open_contained_ro_handle(resolved: &Path, workspace_root: &Path) -> Result<File, String> {
    open_contained_handle(resolved, workspace_root, false)
}

/// Shared opener behind [`open_contained_rw_handle`] /
/// [`open_contained_ro_handle`].
///
/// The contract is deliberately narrow: `resolved` MUST be the canonical
/// path returned by a prior `canonicalize()` + `starts_with(workspace_root)`
/// check (so its final component is a real file, never a symlink, at check
/// time). Callers MUST perform every filesystem operation on the target
/// through a handle from this opener, never by re-touching the path string.
/// Both access shapes do this: `anvil_suppress` reads and writes through one
/// read-write handle, while `anvil_fix` reads through a read-only handle and,
/// only if a change is needed, escalates to a second, independently-hardened
/// read-write open (each open re-verifies containment, so the escalation adds
/// no window). Because a single open handle is pinned to one inode/file object
/// for its whole lifetime, no path is re-resolved between the read and the
/// write within a handle, so an attacker who swaps the target for a symlink in
/// that window cannot redirect the write — it lands on the originally-opened
/// inode (see the handle-pinning test). This wide read→write window is closed
/// on **every** platform, including Windows, purely by handle-pinning.
///
/// The only residual is the *narrow* canonicalise→open window, where an
/// attacker could swap a path component before we open. Per-platform status:
///
/// | Platform     | Final component swap      | Intermediate dir swap        |
/// | ------------ | ------------------------- | ---------------------------- |
/// | Linux        | blocked (`O_NOFOLLOW`)    | blocked (`/proc/self/fd`)    |
/// | macOS        | blocked (`O_NOFOLLOW`)    | blocked (`fcntl F_GETPATH`)  |
/// | other Unix   | blocked (`O_NOFOLLOW`)    | **open residual** (no fd→path)|
/// | Windows      | **open residual**         | **open residual**            |
///
/// On Windows there is no portable `O_NOFOLLOW` and this code does not (yet)
/// wire `GetFinalPathNameByHandleW`, so neither the final-component nor the
/// intermediate-component swap in the narrow window is guarded. Note this is
/// **not** gated by symlink privilege: NTFS *directory junctions*
/// (`mklink /J`) require no elevation and are followed transparently by
/// `std::fs`, so the vector is real. Closing it needs
/// `GetFinalPathNameByHandleW` (a direct analogue of the Linux/macOS
/// fd→path re-derivation); wiring it is deferred as an owner-decidable
/// residual to avoid pulling a new `windows-sys` feature edge (Hakari churn).
/// The wide read→write window — the one this change primarily targets — is
/// closed on Windows regardless.
fn open_contained_handle(
    resolved: &Path,
    workspace_root: &Path,
    writable: bool,
) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    if writable {
        options.write(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let handle = options
        .open(resolved)
        .map_err(|err| format!("filePath is not accessible: {err}"))?;

    // Confirm the OPENED inode's real path is still inside the workspace,
    // closing the narrow canonicalise→open window for an intermediate
    // directory component swapped for a symlink/junction. `handle_real_path`
    // returns `None` on platforms without an fd→path primitive, in which
    // case we fall back to `O_NOFOLLOW` + handle-pinning alone.
    if let Some(real) = handle_real_path(&handle)
        && !real.starts_with(workspace_root)
    {
        return Err("filePath resolves outside workspaceRoot".to_string());
    }

    Ok(handle)
}

/// Best-effort re-derivation of the real filesystem path an open handle
/// currently resolves to, using each platform's direct analogue of Linux
/// `/proc/self/fd`. Returns `None` when the platform offers no such
/// primitive, or the query fails (the caller then relies on `O_NOFOLLOW` +
/// handle-pinning).
#[cfg(target_os = "linux")]
fn handle_real_path(handle: &File) -> Option<PathBuf> {
    use std::os::unix::io::AsRawFd as _;
    let fd_link = format!("/proc/self/fd/{}", handle.as_raw_fd());
    match std::fs::read_link(&fd_link) {
        Ok(real) => Some(real),
        Err(err) => {
            tracing::warn!(
                error = %err,
                "read_link(/proc/self/fd) failed; falling back to O_NOFOLLOW-only containment"
            );
            None
        }
    }
}

/// macOS exposes the path backing an open fd via `fcntl(F_GETPATH)`, the
/// platform analogue of Linux `/proc/self/fd`.
#[cfg(target_os = "macos")]
fn handle_real_path(handle: &File) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt as _;
    use std::os::unix::io::AsRawFd as _;

    // F_GETPATH writes a NUL-terminated path of at most `MAXPATHLEN`
    // (== PATH_MAX == 1024) bytes into the caller-provided buffer.
    let mut buf = vec![0_u8; libc::PATH_MAX as usize];
    // SAFETY: `buf` is `PATH_MAX` bytes and stays live for the call; the fd
    // is valid for the lifetime of `handle`. F_GETPATH only writes within
    // the buffer and NUL-terminates.
    let rc = unsafe {
        libc::fcntl(
            handle.as_raw_fd(),
            libc::F_GETPATH,
            buf.as_mut_ptr().cast::<libc::c_char>(),
        )
    };
    if rc != 0 {
        tracing::warn!("fcntl(F_GETPATH) failed; falling back to O_NOFOLLOW-only containment");
        return None;
    }
    let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(nul);
    Some(PathBuf::from(OsString::from_vec(buf)))
}

/// Platforms without a fd→path primitive wired: rely on `O_NOFOLLOW`
/// (final-component, Unix) + handle-pinning. See the residual table on
/// [`open_contained_handle`].
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn handle_real_path(_handle: &File) -> Option<PathBuf> {
    None
}

/// Build the `warnings` array shared by `anvil_check` and `anvil_gate`.
pub fn build_warnings_array(warnings: &[Warning]) -> Vec<Value> {
    warnings
        .iter()
        .map(|w| {
            json!({
                "id": w.id,
                "severity": severity_str(w.severity),
                "title": w.title,
                "message": w.message,
                "explanation": w.explanation,
                "suggestion": w.suggestion,
                "nudge": w.nudge,
                "category": category_str(w.category),
                "location": {
                    "file": w.location.file,
                    "line": w.location.line,
                    "column": w.location.column,
                    "endLine": w.location.end_line,
                    "endColumn": w.location.end_column,
                }
            })
        })
        .collect()
}

#[must_use]
pub fn severity_str(severity: WarningSeverity) -> &'static str {
    match severity {
        WarningSeverity::Error => "error",
        WarningSeverity::Warning => "warning",
        WarningSeverity::Info => "info",
    }
}

#[must_use]
pub fn category_str(category: WarningCategory) -> &'static str {
    match category {
        WarningCategory::AntiPattern => "anti-pattern",
        WarningCategory::Boundary => "boundary",
        WarningCategory::Architecture => "architecture",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_rejects_arrays_larger_than_max() {
        let oversize: Vec<Value> = (0..=MAX_FILE_ENTRIES)
            .map(|i| Value::String(format!("src/file_{i}.ts")))
            .collect();

        let err = collect_relative_files(&oversize, "files").unwrap_err();

        assert!(err.contains("at most"));
    }

    #[test]
    fn collect_rejects_non_string_entries() {
        let err = collect_relative_files(&[Value::Bool(true)], "files").unwrap_err();

        assert_eq!(err, "files[0] must be a string");
    }

    #[test]
    fn collect_rejects_empty_entries() {
        let err =
            collect_relative_files(&[Value::String(String::new())], "targetFiles").unwrap_err();

        assert_eq!(err, "targetFiles[0] must not be empty");
    }

    #[test]
    fn collect_rejects_parent_dir_components() {
        let err =
            collect_relative_files(&[Value::String("../escape".to_string())], "files").unwrap_err();

        assert_eq!(err, "files[0] must not escape the workspace via \"..\"");
    }

    #[test]
    fn resolve_rejects_symlink_targets_outside_workspace() {
        #[cfg(unix)]
        {
            let cwd = std::env::current_dir().expect("test cwd is accessible");
            let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
            let outside_dir = tempfile::tempdir_in(&cwd).expect("outside dir exists");
            let outside_file = outside_dir.path().join("secret.txt");
            std::fs::write(&outside_file, "shh").expect("outside file writes");

            let link = workspace.path().join("link.ts");
            std::os::unix::fs::symlink(&outside_file, &link).expect("symlink created");

            let workspace_canonical = workspace
                .path()
                .canonicalize()
                .expect("workspace canonicalises");
            let err =
                resolve_workspace_files(&workspace_canonical, &["link.ts".to_string()], "files")
                    .unwrap_err();

            assert_eq!(err, "files[0] resolves outside workspaceRoot");
        }
    }

    #[test]
    fn resolve_tolerates_missing_files_so_scanner_emits_parity_error() {
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let workspace_canonical = workspace
            .path()
            .canonicalize()
            .expect("workspace canonicalises");

        let resolved = resolve_workspace_files(
            &workspace_canonical,
            &["src/does-not-exist.ts".to_string()],
            "files",
        )
        .expect("missing file is tolerated");

        let expected = workspace_canonical
            .join("src/does-not-exist.ts")
            .to_string_lossy()
            .to_string();
        assert_eq!(resolved, vec![expected]);
    }

    #[cfg(unix)]
    #[test]
    fn handle_write_pins_inode_against_post_open_symlink_swap() {
        use std::io::{Seek as _, Write as _};

        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let ws = workspace
            .path()
            .canonicalize()
            .expect("workspace canonicalises");
        let target = ws.join("target.ts");
        std::fs::write(&target, "original\n").expect("target writes");

        // The file an attacker wants the delayed write to clobber.
        let outside_dir = tempfile::tempdir_in(&cwd).expect("outside dir exists");
        let victim = outside_dir.path().join("victim.txt");
        std::fs::write(&victim, "DO NOT OVERWRITE").expect("victim writes");

        let mut handle = open_contained_rw_handle(&target, &ws).expect("opens contained handle");

        // Attacker swaps target.ts -> symlink pointing at the victim AFTER we
        // hold the fd. A path-based `fs::write(target, …)` here would follow
        // the new symlink and clobber the victim.
        std::fs::remove_file(&target).expect("unlink original");
        std::os::unix::fs::symlink(&victim, &target).expect("swap in symlink");

        // Write through the pinned handle.
        handle.rewind().expect("rewind");
        handle.set_len(0).expect("truncate");
        handle.write_all(b"rewritten").expect("write via handle");
        drop(handle);

        // The victim is untouched: the write followed the pinned fd, not the
        // swapped-in symlink.
        assert_eq!(
            std::fs::read_to_string(&victim).expect("victim readable"),
            "DO NOT OVERWRITE"
        );
    }

    #[cfg(unix)]
    #[test]
    fn open_contained_rw_handle_rejects_symlinked_final_component() {
        // Simulates the narrow canonicalise→open window: the path handed to
        // the opener has a symlink as its final component. O_NOFOLLOW must
        // refuse to follow it rather than escape the workspace.
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let ws = workspace
            .path()
            .canonicalize()
            .expect("workspace canonicalises");
        let outside_dir = tempfile::tempdir_in(&cwd).expect("outside dir exists");
        let outside = outside_dir.path().join("secret.txt");
        std::fs::write(&outside, "shh").expect("outside file writes");

        let link = ws.join("link.ts");
        std::os::unix::fs::symlink(&outside, &link).expect("symlink created");

        let err = open_contained_rw_handle(&link, &ws).expect_err("symlink rejected");
        assert!(err.contains("not accessible"), "got: {err}");
    }

    #[test]
    fn open_contained_rw_handle_reads_and_writes_regular_file() {
        use std::io::{Read as _, Seek as _, Write as _};

        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let ws = workspace
            .path()
            .canonicalize()
            .expect("workspace canonicalises");
        let target = ws.join("file.ts");
        std::fs::write(&target, "before\n").expect("target writes");

        let mut handle = open_contained_rw_handle(&target, &ws).expect("opens contained handle");
        let mut content = String::new();
        handle
            .read_to_string(&mut content)
            .expect("read via handle");
        assert_eq!(content, "before\n");

        handle.rewind().expect("rewind");
        handle.set_len(0).expect("truncate");
        handle.write_all(b"after\n").expect("write via handle");
        drop(handle);

        assert_eq!(
            std::fs::read_to_string(&target).expect("target readable"),
            "after\n"
        );
    }

    #[test]
    fn resolve_keeps_inside_workspace_symlinks() {
        #[cfg(unix)]
        {
            let cwd = std::env::current_dir().expect("test cwd is accessible");
            let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
            let workspace_canonical = workspace
                .path()
                .canonicalize()
                .expect("workspace canonicalises");
            let real = workspace_canonical.join("real.ts");
            std::fs::write(&real, "export const x = 1;\n").expect("real file writes");
            let link = workspace_canonical.join("link.ts");
            std::os::unix::fs::symlink(&real, &link).expect("symlink created");

            let resolved =
                resolve_workspace_files(&workspace_canonical, &["link.ts".to_string()], "files")
                    .expect("in-workspace symlink resolves");

            assert_eq!(resolved.len(), 1);
            assert!(resolved[0].ends_with("real.ts"));
        }
    }
}
