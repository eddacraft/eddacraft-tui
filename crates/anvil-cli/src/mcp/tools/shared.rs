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

/// Caller-facing refusal when `workspaceRoot` is not the server cwd, a
/// path inside it, or a registered linked worktree of the same repository
/// (ADR-125).
pub const WORKSPACE_ROOT_NOT_ADMITTED: &str = "workspaceRoot must be inside the MCP server root or a linked git worktree of the same repository";

/// Validate that `workspace_root` is an admitted MCP workspace for
/// `server_root`. Returns the canonicalised `(server_root, workspace_root)`
/// pair on success.
///
/// Admitted roots are the server cwd, a directory inside it, or a
/// registered Git worktree of the same repository (ADR-125).
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
    let server_root = dunce::canonicalize(server_root)
        .map_err(|err| format!("MCP server root is not accessible: {err}"))?;
    let workspace_root = dunce::canonicalize(workspace_root)
        .map_err(|err| format!("workspaceRoot is not accessible: {err}"))?;
    if !workspace_root_is_admitted(&workspace_root, &server_root) {
        return Err(WORKSPACE_ROOT_NOT_ADMITTED.to_string());
    }
    Ok((server_root, workspace_root))
}

/// Whether a canonical `workspace_root` may be used as an MCP tool workspace
/// for this server cwd (ADR-125).
pub fn workspace_root_is_admitted(workspace_root: &Path, server_root: &Path) -> bool {
    if workspace_root == server_root || workspace_root.starts_with(server_root) {
        return true;
    }
    registered_worktree_roots(server_root)
        .iter()
        .any(|root| workspace_root.starts_with(root))
}

/// Redact an absolute workspace path to a server-root-relative form.
/// Linked worktrees that are not inside the server cwd redact to
/// `worktree:<basename>` so the response stays identity-only (ADR-125).
pub fn redact_workspace_root(workspace_root: &Path, server_root: &Path) -> String {
    if let Ok(relative) = workspace_root.strip_prefix(server_root) {
        if relative.as_os_str().is_empty() {
            return ".".to_string();
        }
        return relative.to_string_lossy().replace('\\', "/");
    }
    match workspace_root.file_name() {
        Some(name) => format!("worktree:{}", name.to_string_lossy()),
        None => "worktree".to_string(),
    }
}

/// Resolve `<root>/.git` as a directory or as a linked-worktree `gitdir:` file.
/// Portable file parse — intercept's copy is unix-gated (`graph_base_trigger`).
fn resolve_git_dir(repo_root: &Path) -> Option<PathBuf> {
    let dot_git = repo_root.join(".git");
    let meta = std::fs::symlink_metadata(&dot_git).ok()?;
    if meta.is_dir() {
        return Some(dot_git);
    }
    let contents = std::fs::read_to_string(&dot_git).ok()?;
    let line = contents.lines().find_map(|l| {
        l.trim()
            .strip_prefix("gitdir:")
            .map(str::trim)
            .filter(|p| !p.is_empty())
    })?;
    let git_dir = Path::new(line);
    Some(if git_dir.is_absolute() {
        git_dir.to_path_buf()
    } else {
        lexical_join(repo_root, git_dir)
    })
}

fn resolve_common_dir(git_dir: &Path) -> PathBuf {
    match std::fs::read_to_string(git_dir.join("commondir")) {
        Ok(raw) => {
            let rel = Path::new(raw.trim());
            if rel.is_absolute() {
                rel.to_path_buf()
            } else {
                lexical_join(git_dir, rel)
            }
        }
        Err(_) => git_dir.to_path_buf(),
    }
}

fn lexical_join(base: &Path, rel: &Path) -> PathBuf {
    let mut out = base.to_path_buf();
    for comp in rel.components() {
        match comp {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Registered Git worktree roots of the repository that owns `server_root`,
/// including the main worktree. Empty when `server_root` is not a Git
/// checkout. Does not spawn `git`; reads the on-disk gitdir layout.
fn registered_worktree_roots(server_root: &Path) -> Vec<PathBuf> {
    let Some(git_dir) = resolve_git_dir(server_root) else {
        return Vec::new();
    };
    let common = resolve_common_dir(&git_dir);
    let Ok(common) = dunce::canonicalize(&common) else {
        return Vec::new();
    };

    let mut roots = Vec::new();
    if common.file_name().is_some_and(|name| name == ".git")
        && let Some(parent) = common.parent()
        && let Ok(main) = dunce::canonicalize(parent)
    {
        roots.push(main);
    }

    let Ok(entries) = std::fs::read_dir(common.join("worktrees")) else {
        return roots;
    };
    for entry in entries.flatten() {
        let admin = entry.path();
        if !admin.is_dir() {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(admin.join("gitdir")) else {
            continue;
        };
        let pointed = Path::new(raw.trim());
        let pointed = if pointed.is_absolute() {
            pointed.to_path_buf()
        } else {
            admin.join(pointed)
        };
        let Some(root) = pointed.parent() else {
            continue;
        };
        if let Ok(canon) = dunce::canonicalize(root)
            && !roots.iter().any(|existing| existing == &canon)
        {
            roots.push(canon);
        }
    }
    roots
}

/// Select whether the returned path is a policy key or a filesystem spelling.
#[derive(Clone, Copy)]
pub enum WorkspacePathKind {
    /// Normalise separators so architecture policy keys compare consistently.
    Policy,
    /// Preserve the caller's spelling so valid POSIX backslashes keep their
    /// filename identity; portable validation still recognises Windows syntax.
    Filesystem,
    /// Normalise native host separators and dot segments for filesystem APIs.
    /// On Unix, backslashes remain literal filename characters; on Windows,
    /// the returned spelling uses `/` separators.
    HostFilesystem,
}

fn has_portable_anchor(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

/// Lexically validate an untrusted workspace-relative path in a
/// host-OS-independent way, then return the representation selected by `kind`.
/// Windows separators and anchors are recognised even when the server is
/// running on Unix.
///
/// Validation uses a portable lexical representation that converts backslashes
/// to slashes and drops `.` and empty segments. It rejects NUL,
/// rooted/UNC/drive-prefixed paths, `..`, and values that become empty.
/// [`WorkspacePathKind::Policy`] returns that normalised representation;
/// [`WorkspacePathKind::Filesystem`] returns the original spelling so valid
/// POSIX backslashes retain their filename identity.
/// [`WorkspacePathKind::HostFilesystem`] normalises native host separators and
/// dot segments, returning `/`-separated output on Windows while preserving
/// literal backslashes on Unix. Filesystem containment is still enforced by
/// the caller after joining the result to its validated workspace root.
pub fn normalise_workspace_relative_path(
    field: &str,
    raw: &str,
    kind: WorkspacePathKind,
) -> Result<String, String> {
    if raw.contains('\0') {
        return Err(format!("{field} must not contain NUL characters"));
    }
    let unified = raw.replace('\\', "/");
    if has_portable_anchor(&unified) {
        return Err(format!("{field} must be a workspace-relative path"));
    }

    // Dropping leading current-directory segments can expose an anchor that
    // was not at byte zero in the caller's spelling (`./C:/x`, `./\\server`,
    // `./\root`). Parse that prefix from the original spelling so redundant
    // forward separators remain ordinary empty segments, while a backslash
    // still retains the anchor evidence that portable validation needs.
    let mut anchor_remainder = raw;
    let mut consumed_current_dir = false;
    while let Some(without_current_dir) = anchor_remainder
        .strip_prefix("./")
        .or_else(|| anchor_remainder.strip_prefix(".\\"))
    {
        consumed_current_dir = true;
        anchor_remainder = without_current_dir.trim_start_matches('/');
    }
    if consumed_current_dir && has_portable_anchor(&anchor_remainder.replace('\\', "/")) {
        return Err(format!("{field} must be a workspace-relative path"));
    }
    let mut segments = Vec::new();
    for segment in unified.split('/') {
        if segment == ".." {
            return Err(format!("{field} must not escape the workspace via \"..\""));
        }
        if !segment.is_empty() && segment != "." {
            segments.push(segment);
        }
    }
    let normalised = segments.join("/");
    if normalised.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if has_portable_anchor(&normalised) {
        return Err(format!("{field} must be a workspace-relative path"));
    }
    Ok(match kind {
        WorkspacePathKind::Policy => normalised,
        WorkspacePathKind::Filesystem => raw.to_string(),
        WorkspacePathKind::HostFilesystem => {
            let mut host_segments = Vec::new();
            for component in Path::new(raw).components() {
                match component {
                    std::path::Component::Normal(segment) => {
                        host_segments.push(segment.to_string_lossy().into_owned());
                    }
                    std::path::Component::CurDir => {}
                    std::path::Component::ParentDir => {
                        return Err(format!("{field} must not escape the workspace via \"..\""));
                    }
                    std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                        return Err(format!("{field} must be a workspace-relative path"));
                    }
                }
            }
            host_segments.join("/")
        }
    })
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
        out.push(normalise_workspace_relative_path(
            &format!("{field}[{index}]"),
            path,
            WorkspacePathKind::Filesystem,
        )?);
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
///
/// Goes through `nix`'s safe `F_GETPATH` wrapper (which owns the
/// `MAXPATHLEN` buffer handling) rather than a raw `libc::fcntl` call:
/// the workspace forbids `unsafe_code`, so the original hand-rolled
/// binding could never compile on this target — caught by the Cross
/// (x86_64/aarch64-apple-darwin) legs, not by Linux CI.
#[cfg(target_os = "macos")]
fn handle_real_path(handle: &File) -> Option<PathBuf> {
    use nix::fcntl::{FcntlArg, fcntl};

    let mut path = PathBuf::new();
    match fcntl(handle, FcntlArg::F_GETPATH(&mut path)) {
        Ok(_) => Some(path),
        Err(err) => {
            tracing::warn!(
                error = %err,
                "fcntl(F_GETPATH) failed; falling back to O_NOFOLLOW-only containment"
            );
            None
        }
    }
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
    fn normalises_workspace_relative_paths_independently_of_host_os() {
        assert_eq!(
            normalise_workspace_relative_path(
                "filePath",
                ".\\src//domain\\file.ts/",
                WorkspacePathKind::Policy,
            ),
            Ok("src/domain/file.ts".to_string())
        );
    }

    #[test]
    fn host_normalised_filesystem_paths_normalise_native_spelling() {
        assert_eq!(
            normalise_workspace_relative_path(
                "filePath",
                "./src//x.ts",
                WorkspacePathKind::HostFilesystem,
            ),
            Ok("src/x.ts".to_string())
        );

        #[cfg(unix)]
        assert_eq!(
            normalise_workspace_relative_path(
                "filePath",
                r"src/a\b.ts",
                WorkspacePathKind::HostFilesystem,
            ),
            Ok(r"src/a\b.ts".to_string())
        );
    }

    #[test]
    fn host_normalised_filesystem_paths_drop_redundant_forward_separators_after_current_dir() {
        for path in [".//src/x.ts", ".//./src/x.ts"] {
            assert_eq!(
                normalise_workspace_relative_path(
                    "filePath",
                    path,
                    WorkspacePathKind::HostFilesystem,
                ),
                Ok("src/x.ts".to_string()),
                "path {path:?} should be accepted"
            );
        }
    }

    #[test]
    fn host_normalised_filesystem_paths_reject_portable_hazards() {
        for path in [
            r"C:\outside.ts",
            r".\\server\share\outside.ts",
            r".\\\server\share\outside.ts",
            r".\\outside.ts",
            r".\\?\C:\outside.ts",
            r".\\\?\C:\outside.ts",
            r".\\.\pipe\name",
            r".\\\.\pipe\name",
            r".\C:\outside.ts",
            r".\C:relative",
            "./C:/outside.ts",
            "./C:relative",
            r"./\\server\share\outside.ts",
            r"./\outside.ts",
            r"./\\?\C:\outside.ts",
            r"./\\.\pipe\name",
            r"\\server\share\outside.ts",
            "../outside.ts",
            r"src\..\outside.ts",
            "src/evil\0name.ts",
        ] {
            assert!(
                normalise_workspace_relative_path(
                    "filePath",
                    path,
                    WorkspacePathKind::HostFilesystem,
                )
                .is_err(),
                "path {path:?} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_windows_anchors_on_non_windows_hosts() {
        for path in [r"C:\Windows\system32", "C:relative", r"\\server\share"] {
            assert_eq!(
                normalise_workspace_relative_path("filePath", path, WorkspacePathKind::Policy),
                Err("filePath must be a workspace-relative path".to_string())
            );
        }
    }

    #[test]
    fn rejects_parent_and_nul_components() {
        assert_eq!(
            normalise_workspace_relative_path(
                "filePath",
                "src/../escape",
                WorkspacePathKind::Filesystem,
            ),
            Err("filePath must not escape the workspace via \"..\"".to_string())
        );
        assert_eq!(
            normalise_workspace_relative_path(
                "filePath",
                "src/evil\0name",
                WorkspacePathKind::Filesystem,
            ),
            Err("filePath must not contain NUL characters".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn filesystem_paths_preserve_literal_posix_backslashes() {
        assert_eq!(
            normalise_workspace_relative_path(
                "filePath",
                r"src/a\b.ts",
                WorkspacePathKind::Filesystem,
            ),
            Ok(r"src/a\b.ts".to_string())
        );
    }

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

    fn linked_worktree_layout(root: &Path) -> (PathBuf, PathBuf) {
        let main = root.join("main");
        let common = main.join(".git");
        std::fs::create_dir_all(common.join("refs")).expect("git refs dir");
        std::fs::write(common.join("HEAD"), b"ref: refs/heads/main\n").expect("HEAD");
        std::fs::create_dir_all(&main).expect("main worktree");

        let admin = common.join("worktrees").join("linked");
        std::fs::create_dir_all(&admin).expect("worktree admin dir");
        std::fs::write(admin.join("HEAD"), b"ref: refs/heads/feature\n").expect("linked HEAD");
        std::fs::write(admin.join("commondir"), b"../..\n").expect("commondir");

        let linked = root.join("linked");
        std::fs::create_dir_all(&linked).expect("linked worktree");
        let git_file = linked.join(".git");
        std::fs::write(&git_file, format!("gitdir: {}\n", admin.display())).expect(".git file");
        std::fs::write(admin.join("gitdir"), format!("{}\n", git_file.display()))
            .expect("gitdir back-pointer");

        let main = dunce::canonicalize(&main).expect("main canonicalises");
        let linked = dunce::canonicalize(&linked).expect("linked canonicalises");
        (main, linked)
    }

    #[test]
    fn admits_registered_linked_worktree_of_the_same_repository() {
        let root = tempfile::tempdir().expect("fixture root");
        let (main, linked) = linked_worktree_layout(root.path());

        let (server, workspace) =
            validate_workspace_root(&linked, &main).expect("linked worktree is admitted");
        assert_eq!(server, main);
        assert_eq!(workspace, linked);
        assert_eq!(redact_workspace_root(&linked, &main), "worktree:linked");
    }

    #[test]
    fn admits_main_checkout_when_server_is_a_linked_worktree() {
        let root = tempfile::tempdir().expect("fixture root");
        let (main, linked) = linked_worktree_layout(root.path());

        validate_workspace_root(&main, &linked)
            .expect("main checkout is admitted from a linked-worktree server");
    }

    #[test]
    fn admits_directory_inside_a_registered_linked_worktree() {
        let root = tempfile::tempdir().expect("fixture root");
        let (main, linked) = linked_worktree_layout(root.path());
        let nested = linked.join("src");
        std::fs::create_dir_all(&nested).expect("nested dir");

        validate_workspace_root(&nested, &main).expect("nested path inside linked worktree");
    }

    #[test]
    fn refuses_unrelated_sibling_directory() {
        let root = tempfile::tempdir().expect("fixture root");
        let (main, _linked) = linked_worktree_layout(root.path());
        let other = tempfile::tempdir().expect("other repo");

        let err = validate_workspace_root(other.path(), &main).expect_err("foreign root refused");
        assert_eq!(err, WORKSPACE_ROOT_NOT_ADMITTED);
    }

    #[test]
    fn refuses_forged_gitdir_pointer_that_is_not_registered() {
        let root = tempfile::tempdir().expect("fixture root");
        let (main, _linked) = linked_worktree_layout(root.path());
        let fake = root.path().join("forged");
        std::fs::create_dir_all(&fake).expect("forged dir");
        std::fs::write(
            fake.join(".git"),
            format!("gitdir: {}\n", main.join(".git").display()),
        )
        .expect("forged gitdir pointer");

        let err = validate_workspace_root(&fake, &main).expect_err("forged pointer refused");
        assert_eq!(err, WORKSPACE_ROOT_NOT_ADMITTED);
    }

    #[test]
    fn still_admits_nested_directory_inside_the_server_root() {
        let cwd = std::env::current_dir().expect("test cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let nested = workspace.path().join("pkg");
        std::fs::create_dir_all(&nested).expect("nested");

        validate_workspace_root(&nested, workspace.path()).expect("nested under server root");
        assert_eq!(
            redact_workspace_root(
                &dunce::canonicalize(&nested).expect("nested canonicalises"),
                &dunce::canonicalize(workspace.path()).expect("workspace canonicalises"),
            ),
            "pkg"
        );
    }
}
