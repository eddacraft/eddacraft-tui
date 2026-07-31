//! DSV-003 (ADR-061 §5): openat2-anchored path normalisation and workspace
//! directory opens for daemon reads.

use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::Path;

// `Errno` is only referenced by the Linux `openat2` fallback ladder (the
// `ENOSYS`/`EPERM` match) and by the tests (`ELOOP`); gate the import so a
// non-Linux build (e.g. the macOS cross matrix) does not flag it unused under
// `-D warnings`.
#[cfg(any(target_os = "linux", test))]
use nix::errno::Errno;
use nix::fcntl::{OFlag, openat};
use nix::sys::stat::Mode;
use thiserror::Error;

/// A workspace-root-relative path that has passed *structural* escape
/// validation: it is not absolute and contains no `..` component. Symlink
/// escape is a separate, read-time guarantee enforced by [`read_under`]
/// (openat2 / the `O_NOFOLLOW` ladder), not by this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelPath {
    /// Slash-joined normalised form (e.g. `src/lib.rs`). Never absolute,
    /// never carries `.`/empty/`..` segments.
    joined: String,
    /// The individual path components, in order — used by the fallback ladder
    /// to open one hop at a time.
    components: Vec<String>,
}

impl RelPath {
    /// The slash-joined normalised path (for openat2 and diagnostics).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.joined
    }
}

/// Why a client-supplied path was refused before any read was attempted.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Escape {
    /// The path began with `/` — absolute paths are never resolved against a
    /// workspace root.
    #[error("path is absolute, refusing to resolve against a workspace root: {0:?}")]
    Absolute(String),
    /// The path contained a `..` component, which could climb above the root.
    #[error("path escapes workspace root via `..`: {0:?}")]
    ParentEscape(String),
    /// The path normalised to nothing (e.g. `""`, `.`, `./`).
    #[error("path is empty after normalisation: {0:?}")]
    Empty(String),
    /// The path contained an interior NUL byte. A NUL would truncate the path
    /// at the C-string boundary, so it is refused outright rather than relying
    /// on the syscall layer to reject it.
    #[error("path contains an interior NUL byte: {0:?}")]
    NulByte(String),
}

/// Normalise and structurally validate a client-supplied, root-relative path.
///
/// Collapses `.` and empty segments, rejects absolute paths and any `..`
/// segment. The workspace root is intentionally **not** a parameter: the root
/// is the held dirfd in [`read_under`], not a string we re-join here — keeping
/// the two concerns separate is what makes a post-admission root swap
/// un-exploitable (security C2).
///
/// # Errors
/// Returns [`Escape`] if the path is absolute, climbs via `..`, is empty, or
/// contains an interior NUL byte.
pub fn normalise_rel(path: &str) -> Result<RelPath, Escape> {
    // Refuse a NUL explicitly. The `nix` `CString` boundary would reject it
    // too, but making it this module's contract means a future read path that
    // does not go through `nix` cannot silently inherit a truncated C string.
    if path.contains('\0') {
        return Err(Escape::NulByte(path.to_string()));
    }
    if path.starts_with('/') {
        return Err(Escape::Absolute(path.to_string()));
    }
    let mut components = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err(Escape::ParentEscape(path.to_string())),
            other => components.push(other.to_string()),
        }
    }
    if components.is_empty() {
        return Err(Escape::Empty(path.to_string()));
    }
    Ok(RelPath {
        joined: components.join("/"),
        components,
    })
}

/// Open the workspace root directory as the read anchor / identity.
///
/// On Linux this is an `O_PATH` directory fd (the lightest handle that still
/// works as an `*at` anchor); elsewhere a plain `O_DIRECTORY` fd. The fd is
/// opened **once** per admitted root and held: all subsequent reads anchor to
/// it, so a later retarget of the root path cannot redirect them.
///
/// # Errors
/// Propagates the underlying open error (e.g. the root does not exist or is
/// not a directory).
pub fn open_workspace_dirfd(root: &Path) -> io::Result<OwnedFd> {
    // `O_PATH` is the lightest handle that still works as an `*at` anchor, but it
    // is Linux-only; elsewhere a plain `O_DIRECTORY` fd. Two cfg-split bindings
    // (rather than `let mut` + a Linux-gated `|=`) so a non-Linux build does not
    // flag the unmutated `flags` under `-D warnings`.
    #[cfg(target_os = "linux")]
    let flags = OFlag::O_DIRECTORY | OFlag::O_CLOEXEC | OFlag::O_PATH;
    #[cfg(not(target_os = "linux"))]
    let flags = OFlag::O_DIRECTORY | OFlag::O_CLOEXEC;
    nix::fcntl::open(root, flags, Mode::empty()).map_err(io::Error::from)
}

/// Open `dir` as a **real, fsync-able** directory fd for use as a write anchor.
///
/// Unlike [`open_workspace_dirfd`] — which returns an `O_PATH` fd on Linux (the
/// lightest `*at` anchor, but one that **cannot be `fsync`'d**: `fsync` on an
/// `O_PATH` fd returns `EBADF`) — this opens the directory `O_DIRECTORY |
/// O_RDONLY` so the single fd serves three roles at once: the `openat`/`openat2`
/// create anchor, the `renameat`/`unlinkat` anchor, AND the post-rename
/// directory `fsync` target (ADR-069 §4 / CIB-097). A plain `O_DIRECTORY` fd is
/// an equally valid `*at` anchor as an `O_PATH` one — only the read path needs
/// `O_PATH` for its `openat2` anchor — so this is the natural fd for the write
/// path's durability discipline.
///
/// `O_NOFOLLOW` refuses a symlinked `dir` (defence-in-depth alongside the
/// caller's [`validate_secure_dir`](crate::snapshot_io) check, which already
/// rejects a symlinked snapshot dir before any write).
///
/// The opened fd is then `fstat`-validated (owner-only mode, owned by the current
/// euid) on the **actual inode we hold** — closing the TOCTOU between the caller's
/// path-based `validate_secure_dir` and this open, where a same-uid attacker could
/// swap an intermediate component to a different (non-symlink) directory.
///
/// # Errors
/// Propagates the underlying open error (the dir does not exist, is not a
/// directory, or is a symlink — `ELOOP`), or `PermissionDenied` if the opened
/// directory is not owned by the current user or is group/other-accessible.
pub fn open_workspace_dir_for_fsync(dir: &Path) -> io::Result<OwnedFd> {
    // A REAL directory fd (no `O_PATH`): `O_PATH` cannot be `fsync`'d, and the
    // write path must `fsync` this fd after the rename to make the publish
    // durable. `O_NOFOLLOW` refuses a symlinked dir.
    let flags = OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let fd = nix::fcntl::open(dir, flags, Mode::empty()).map_err(io::Error::from)?;

    // Validate the inode we actually opened (not the looked-up path): re-check
    // ownership + owner-only mode via `fstat`, so a swapped-in directory that is
    // not ours / is group- or other-accessible is rejected even if it slipped in
    // between `validate_secure_dir` and this open.
    let st = nix::sys::stat::fstat(&fd).map_err(io::Error::from)?;
    if st.st_uid != nix::unistd::geteuid().as_raw() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "snapshot dir fd is not owned by the current user",
        ));
    }
    if st.st_mode & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "snapshot dir fd is group/other-accessible",
        ));
    }
    Ok(fd)
}

/// Read the full bytes of `rel` resolved beneath `dirfd`, refusing any symlink
/// or escape during resolution.
///
/// On Linux this uses a single `openat2` with `RESOLVE_NO_SYMLINKS |
/// RESOLVE_BENEATH`; where `openat2` is unavailable — `ENOSYS` on pre-5.6
/// kernels, or `EPERM` when a seccomp profile blocks the unknown syscall — it
/// falls back to the `O_NOFOLLOW` component ladder, as it does on non-Linux.
/// Applies equally to a change's `path` and a rename's `from` side — both are
/// normalised and read through this guard.
///
/// # Errors
/// Returns an [`io::Error`] for a symlink rejection (`ELOOP`), a missing file
/// (`ENOENT`), or any other read failure. A `..`/absolute escape (`EXDEV`
/// under `RESOLVE_BENEATH`) is not observable here: [`normalise_rel`], a
/// precondition of constructing a [`RelPath`], rejects those before any read.
pub fn read_under(dirfd: BorrowedFd<'_>, rel: &RelPath) -> io::Result<Vec<u8>> {
    read_under_capped(dirfd, rel, MAX_GUARDED_READ_BYTES)
}

/// Read the full bytes of `rel` beneath `dirfd`, refusing symlink/escape
/// resolution and refusing input larger than the caller-provided `max_bytes`.
///
/// This is the capability-facing form of [`read_under`]: consumers with a
/// smaller artefact budget can apply it while retaining the same held-handle
/// containment. The read buffers at most `max_bytes + 1` bytes so an oversized
/// file is detected and refused rather than truncated.
///
/// # Errors
/// Returns the same path/open errors as [`read_under`], or
/// [`io::ErrorKind::FileTooLarge`] when the file exceeds `max_bytes`.
pub fn read_under_capped(
    dirfd: BorrowedFd<'_>,
    rel: &RelPath,
    max_bytes: u64,
) -> io::Result<Vec<u8>> {
    #[cfg(target_os = "linux")]
    {
        match read_under_openat2(dirfd, rel, max_bytes) {
            // `openat2` unavailable: ENOSYS on old kernels, EPERM when a
            // seccomp filter rejects the unknown syscall. Both mean "syscall
            // absent", not "this path is forbidden" — fall back to the ladder.
            Err(err)
                if matches!(
                    err.raw_os_error(),
                    Some(code) if code == Errno::ENOSYS as i32 || code == Errno::EPERM as i32
                ) =>
            {
                read_under_ladder_capped(dirfd, rel, max_bytes)
            }
            other => other,
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        read_under_ladder_capped(dirfd, rel, max_bytes)
    }
}

#[cfg(target_os = "linux")]
fn read_under_openat2(dirfd: BorrowedFd<'_>, rel: &RelPath, max_bytes: u64) -> io::Result<Vec<u8>> {
    use nix::fcntl::{OpenHow, ResolveFlag, openat2};

    let how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_CLOEXEC)
        .resolve(ResolveFlag::RESOLVE_NO_SYMLINKS | ResolveFlag::RESOLVE_BENEATH);
    let fd = openat2(dirfd, rel.as_str(), how).map_err(io::Error::from)?;
    read_fd_capped(fd, max_bytes)
}

/// Fallback for kernels/platforms without `openat2`: walk one component at a
/// time from `dirfd`, refusing a symlink at every hop with `O_NOFOLLOW`.
///
/// Equivalent to `openat2`'s `RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH` for
/// **symlink and `..` rejection** (per-hop `O_NOFOLLOW`; `..` is pre-rejected
/// by [`normalise_rel`]; each hop anchored at the prior dirfd stays beneath the
/// root). It is **not** equivalent in *atomicity*: `openat2` resolves the whole
/// path in one syscall, whereas this walks N opens, leaving a window in which a
/// same-uid writer could swap a real (non-symlink) intermediate directory.
/// That window is in-model — the trust boundary is `SO_PEERCRED` same-uid
/// (contract §4) — and only reachable on pre-5.6 Linux / non-Linux.
#[cfg(test)]
fn read_under_ladder(dirfd: BorrowedFd<'_>, rel: &RelPath) -> io::Result<Vec<u8>> {
    read_under_ladder_capped(dirfd, rel, MAX_GUARDED_READ_BYTES)
}

fn read_under_ladder_capped(
    dirfd: BorrowedFd<'_>,
    rel: &RelPath,
    max_bytes: u64,
) -> io::Result<Vec<u8>> {
    // Intermediate directory hops are opened WITHOUT `O_PATH`: with `O_PATH`,
    // `O_NOFOLLOW` on a symlink succeeds (it returns a handle to the symlink
    // itself) instead of failing `ELOOP`, which would defeat the per-hop
    // symlink rejection. A plain `O_DIRECTORY | O_NOFOLLOW` open is what makes
    // a symlinked component fail closed.
    let dir_flags = OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;

    let (last, parents) = rel
        .components
        .split_last()
        .expect("RelPath is never empty by construction");

    // Descend through the parent directories, refusing a symlinked component
    // at each hop. Each open is anchored at the previous directory fd.
    let mut current: Option<OwnedFd> = None;
    for component in parents {
        let anchor = current.as_ref().map_or(dirfd, AsFd::as_fd);
        let next = openat(anchor, component.as_str(), dir_flags, Mode::empty())
            .map_err(io::Error::from)?;
        current = Some(next);
    }

    let anchor = current.as_ref().map_or(dirfd, AsFd::as_fd);
    let file_fd = openat(
        anchor,
        last.as_str(),
        OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(io::Error::from)?;
    read_fd_capped(file_fd, max_bytes)
}

/// Hard upper bound on the bytes [`read_under`] will buffer for one file — the
/// daemon's memory-DoS ceiling (DSV-006 / Task 11).
///
/// A same-uid peer must not be able to make the verdict path allocate an
/// unbounded buffer by pointing it at an enormous file. This ceiling sits
/// **above** the configurable parse-size cap
/// ([`DosCaps::max_parse_bytes`](crate::workspace_pool::DosCaps::max_parse_bytes),
/// default 2 MiB): a file between the two is read and then skipped-with-a-
/// diagnostic by the verdict path, while a file beyond this ceiling is refused
/// at the read with `FileTooLarge` before the buffer grows past it. 64 MiB is
/// far larger than any file the antipattern family meaningfully scans, so the
/// ceiling only ever trips on pathological input.
pub const MAX_GUARDED_READ_BYTES: u64 = 64 * 1024 * 1024;

/// Read at most `max_bytes` of `fd`, refusing a file that delivers more.
///
/// The allocation is bounded to `max_bytes + 1`: a file over the ceiling is
/// *refused* (`FileTooLarge`), never truncated to a wrong, hashable prefix — a
/// truncated buffer would yield a verdict over content that is not what is on
/// disk. The `+ 1` distinguishes "exactly at the ceiling" (allowed) from "over
/// it" (refused) without a separate fstat that could race the read.
fn read_fd_capped(fd: OwnedFd, max_bytes: u64) -> io::Result<Vec<u8>> {
    let probe_bytes = max_bytes.checked_add(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "guarded-read ceiling must be less than u64::MAX",
        )
    })?;
    let file = File::from(fd);
    let mut buf = Vec::new();
    let read = file.take(probe_bytes).read_to_end(&mut buf)?;
    if read as u64 > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            format!("file exceeds the {max_bytes}-byte guarded-read ceiling"),
        ));
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    // ---- read ceiling (DSV-006 / Task 11 memory-DoS guard) ----

    #[test]
    fn read_fd_capped_refuses_oversized_without_truncating() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("ten.bin");
        std::fs::write(&path, b"0123456789").expect("write 10 bytes");

        let open = || OwnedFd::from(File::open(&path).expect("open"));

        // Under the cap: full content returned.
        assert_eq!(
            read_fd_capped(open(), 100).expect("small read"),
            b"0123456789"
        );
        // Exactly at the cap: allowed (the `+ 1` headroom makes the boundary
        // inclusive).
        assert_eq!(read_fd_capped(open(), 10).expect("at-cap read").len(), 10);
        // Over the cap: refused with FileTooLarge, never a truncated prefix.
        let err = read_fd_capped(open(), 5).expect_err("over-cap refused");
        assert_eq!(err.kind(), io::ErrorKind::FileTooLarge);
    }

    #[test]
    fn read_under_capped_enforces_the_caller_limit_on_the_guarded_handle() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("ten.bin"), b"0123456789").expect("write 10 bytes");
        let dirfd = open_workspace_dirfd(tmp.path()).expect("open root");
        let rel = normalise_rel("ten.bin").expect("normalise path");

        let err = read_under_capped(dirfd.as_fd(), &rel, 5)
            .expect_err("a guarded read over the caller limit must be refused");

        assert_eq!(err.kind(), io::ErrorKind::FileTooLarge);
        assert_eq!(
            read_under_capped(dirfd.as_fd(), &rel, 10).expect("exact-limit read"),
            b"0123456789"
        );
    }

    // ---- normalise_rel ----

    #[test]
    fn normalise_rel_accepts_and_collapses() {
        let rel = normalise_rel("src/./lib.rs").expect("clean path");
        assert_eq!(rel.as_str(), "src/lib.rs");
        assert_eq!(rel.components, vec!["src", "lib.rs"]);
    }

    #[test]
    fn normalise_rel_rejects_absolute() {
        assert_eq!(
            normalise_rel("/etc/passwd"),
            Err(Escape::Absolute("/etc/passwd".to_string()))
        );
    }

    #[test]
    fn normalise_rel_rejects_parent_escape() {
        assert_eq!(
            normalise_rel("../../etc/passwd"),
            Err(Escape::ParentEscape("../../etc/passwd".to_string()))
        );
        // A `..` buried mid-path is also refused.
        assert_eq!(
            normalise_rel("src/../../etc"),
            Err(Escape::ParentEscape("src/../../etc".to_string()))
        );
    }

    #[test]
    fn normalise_rel_rejects_empty() {
        assert_eq!(normalise_rel(""), Err(Escape::Empty(String::new())));
        assert_eq!(normalise_rel("."), Err(Escape::Empty(".".to_string())));
        assert_eq!(normalise_rel("./"), Err(Escape::Empty("./".to_string())));
    }

    #[test]
    fn normalise_rel_rejects_interior_nul() {
        let bad = "src/\0evil.rs";
        assert_eq!(normalise_rel(bad), Err(Escape::NulByte(bad.to_string())));
    }

    // ---- read_under ----

    #[test]
    fn read_under_reads_real_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), b"fn main() {}").unwrap();

        let dirfd = open_workspace_dirfd(tmp.path()).expect("open root");
        let rel = normalise_rel("src/lib.rs").unwrap();
        let bytes = read_under(dirfd.as_fd(), &rel).expect("read");
        assert_eq!(bytes, b"fn main() {}");
    }

    #[test]
    fn read_under_rejects_symlink_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // A symlink inside the root pointing at an absolute path outside it.
        symlink("/etc/hostname", tmp.path().join("escape")).unwrap();

        let dirfd = open_workspace_dirfd(tmp.path()).expect("open root");
        let rel = normalise_rel("escape").unwrap();
        let err = read_under(dirfd.as_fd(), &rel)
            .expect_err("a symlink in the resolution path must be refused");
        // openat2 RESOLVE_NO_SYMLINKS ⇒ ELOOP; the O_NOFOLLOW ladder ⇒ ELOOP too.
        assert_eq!(err.raw_os_error(), Some(Errno::ELOOP as i32), "{err}");
    }

    #[test]
    fn read_under_rejects_symlink_escape_for_renamed_from() {
        // C1: the SAME guard rejects an escaping `renamed.from`, not just
        // `path`. There is one read path, so a symlinked `from` is refused
        // identically — we simply drive the guard with the `from` value.
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(tmp.path().join("nested")).unwrap();
        symlink("/etc/hostname", tmp.path().join("nested/from-link")).unwrap();

        let dirfd = open_workspace_dirfd(tmp.path()).expect("open root");
        let from = normalise_rel("nested/from-link").unwrap();
        let err = read_under(dirfd.as_fd(), &from)
            .expect_err("an escaping renamed.from must be refused by the same guard");
        assert_eq!(err.raw_os_error(), Some(Errno::ELOOP as i32), "{err}");
    }

    #[test]
    fn read_under_rejects_symlinked_parent_component() {
        // A symlinked *directory* component (not just the leaf) is refused.
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(tmp.path().join("real")).unwrap();
        std::fs::write(tmp.path().join("real/secret"), b"x").unwrap();
        symlink("real", tmp.path().join("link")).unwrap();

        let dirfd = open_workspace_dirfd(tmp.path()).expect("open root");
        let rel = normalise_rel("link/secret").unwrap();
        let err = read_under(dirfd.as_fd(), &rel)
            .expect_err("a symlinked parent component must be refused");
        // Linux `openat2(RESOLVE_NO_SYMLINKS)` refuses a symlinked directory
        // component with `ELOOP`; the non-Linux `O_NOFOLLOW` ladder opens the
        // component `O_DIRECTORY | O_NOFOLLOW`, so the unfollowed symlink is
        // rejected as `ENOTDIR` (e.g. macOS). Both mean "fail closed" — accept
        // either, matching `ladder_rejects_symlinked_parent_component`.
        let code = err.raw_os_error();
        assert!(
            code == Some(Errno::ELOOP as i32) || code == Some(Errno::ENOTDIR as i32),
            "expected ELOOP or ENOTDIR, got {err}"
        );
    }

    #[test]
    fn stale_root_dirfd_fails_closed_not_reresolved() {
        // C2: the dirfd is the workspace identity. After admission, replacing
        // the root directory must NOT redirect reads to the new directory —
        // the held fd still anchors to the original inode.
        let parent = tempfile::tempdir().expect("tempdir");
        let root = parent.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("marker"), b"original").unwrap();

        // Admission: open the dirfd once.
        let dirfd = open_workspace_dirfd(&root).expect("open root");

        // Retarget the root path: move the original aside, plant a new dir
        // whose `marker` has different content.
        let moved = parent.path().join("root-old");
        std::fs::rename(&root, &moved).unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("marker"), b"attacker-swapped").unwrap();

        // Reads through the held fd still see the ORIGINAL content — proof the
        // path string was not re-resolved against the swapped-in directory.
        let rel = normalise_rel("marker").unwrap();
        let bytes = read_under(dirfd.as_fd(), &rel).expect("read via held fd");
        assert_eq!(
            bytes, b"original",
            "held dirfd must anchor to the original inode, not the swapped-in root"
        );
    }

    // ---- ladder fallback (exercised directly: on Linux 5.6+ `read_under`
    // always takes the openat2 path, so the security-critical fallback would
    // otherwise be untested on this platform) ----

    #[test]
    fn ladder_reads_real_nested_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(tmp.path().join("a/b")).unwrap();
        std::fs::write(tmp.path().join("a/b/c.rs"), b"nested").unwrap();
        let dirfd = open_workspace_dirfd(tmp.path()).expect("open root");
        let rel = normalise_rel("a/b/c.rs").unwrap();
        assert_eq!(
            read_under_ladder(dirfd.as_fd(), &rel).expect("read"),
            b"nested"
        );
    }

    #[test]
    fn ladder_rejects_symlinked_leaf() {
        let tmp = tempfile::tempdir().expect("tempdir");
        symlink("/etc/hostname", tmp.path().join("leaf")).unwrap();
        let dirfd = open_workspace_dirfd(tmp.path()).expect("open root");
        let rel = normalise_rel("leaf").unwrap();
        let err = read_under_ladder(dirfd.as_fd(), &rel)
            .expect_err("the ladder must refuse a symlinked leaf via O_NOFOLLOW");
        assert_eq!(err.raw_os_error(), Some(Errno::ELOOP as i32), "{err}");
    }

    #[test]
    fn ladder_rejects_symlinked_parent_component() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(tmp.path().join("real")).unwrap();
        std::fs::write(tmp.path().join("real/secret"), b"x").unwrap();
        symlink("real", tmp.path().join("link")).unwrap();
        let dirfd = open_workspace_dirfd(tmp.path()).expect("open root");
        let rel = normalise_rel("link/secret").unwrap();
        let err = read_under_ladder(dirfd.as_fd(), &rel)
            .expect_err("the ladder must refuse a symlinked parent component");
        // Either refusal is fine: `O_NOFOLLOW` does not follow the trailing
        // symlink, so `O_DIRECTORY` rejects the symlink-itself as ENOTDIR
        // (rather than ELOOP). Both mean "fail closed".
        let code = err.raw_os_error();
        assert!(
            code == Some(Errno::ELOOP as i32) || code == Some(Errno::ENOTDIR as i32),
            "expected ELOOP or ENOTDIR, got {err}"
        );
    }

    // ---- write anchor (CIB-097) ----

    #[test]
    fn dir_for_fsync_opens_an_owner_only_dir() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("graph-cache");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        // Succeeds and yields a real (non-`O_PATH`) fd that can actually be fsync'd.
        let fd = open_workspace_dir_for_fsync(&dir).expect("open owner-only dir");
        nix::unistd::fsync(&fd).expect("a real O_DIRECTORY fd is fsync-able");
    }

    #[test]
    fn dir_for_fsync_rejects_a_group_or_other_accessible_dir() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("graph-cache");
        std::fs::create_dir(&dir).unwrap();
        // `validate_secure_dir` is path-based and runs earlier in `write_snapshot`;
        // this asserts the fd-level `fstat` check independently rejects a
        // group/other-accessible directory (the TOCTOU-closing guard).
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let err = open_workspace_dir_for_fsync(&dir)
            .expect_err("a group/other-accessible dir must be refused by the fd fstat check");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied, "{err}");
    }
}
