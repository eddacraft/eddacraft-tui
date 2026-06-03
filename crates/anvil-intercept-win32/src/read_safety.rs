//! DSV-010 / ADR-068: Windows read-safety for the daemon's save-time reads.
//!
//! The Windows analogue of the Unix `path_safety` guard
//! (`crates/anvil-intercept/src/path_safety.rs`). Every save-time read is
//! anchored at a **workspace directory handle** opened once at admission (the
//! workspace identity — security C2) and each path is resolved **one component
//! at a time** with `NtCreateFile` relative to the prior directory handle, with
//! `OBJ_DONT_REPARSE` set so a symlink **or** junction anywhere in the path
//! fails the open (`STATUS_REPARSE_POINT_ENCOUNTERED`) rather than being
//! followed. This is the Windows analogue of the Unix `openat` + `O_NOFOLLOW`
//! ladder under `RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH`, and mirrors the
//! mechanism Go's `os.Root` uses (golang/go#73080).
//!
//! Because the root handle is held open, a root-directory retarget *after*
//! admission cannot redirect reads: the handle is the identity, so reads either
//! hit the original object or fail closed — they never re-resolve the path
//! string against a swapped-in directory (C2).
//!
//! `..` / absolute / drive / UNC / device-prefix / alternate-data-stream /
//! reserved-name escapes are rejected **structurally** by [`normalise_rel`]
//! before any open (the Windows-hardened analogue of the Unix `normalise_rel`,
//! which only has to reject `..` / absolute / NUL on a single-separator
//! filesystem). Oversized files are **refused, never truncated** (B2).
//!
//! All `unsafe` FFI is quarantined here so `anvil-intercept` keeps
//! `#![forbid(unsafe_code)]`.

use std::io::{self, ErrorKind};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::null_mut;

use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_FOR_BACKUP_INTENT,
    FILE_SYNCHRONOUS_IO_NONALERT, NtCreateFile,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE, UNICODE_STRING,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, ReadFile,
};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

// `windows-sys` 0.61 does not re-export these from a module the daemon already
// depends on, so pin them inline (the same approach lib.rs uses for the
// `GENERIC_*` flags). The OBJ_* / SYNCHRONIZE values are frozen Win32/NT ABI
// constants.
const SYNCHRONIZE: u32 = 0x0010_0000;
const OBJ_CASE_INSENSITIVE: u32 = 0x0000_0040;
const OBJ_DONT_REPARSE: u32 = 0x0000_1000;

// NTSTATUS values (i32). `STATUS_REPARSE_POINT_ENCOUNTERED` is the
// `OBJ_DONT_REPARSE` rejection — the Windows ELOOP analogue.
const STATUS_SUCCESS: i32 = 0;
const STATUS_REPARSE_POINT_ENCOUNTERED: i32 = 0x8000_0016_u32 as i32;
const STATUS_OBJECT_NAME_NOT_FOUND: i32 = 0xC000_0034_u32 as i32;
const STATUS_OBJECT_PATH_NOT_FOUND: i32 = 0xC000_003A_u32 as i32;
const STATUS_ACCESS_DENIED: i32 = 0xC000_0022_u32 as i32;
const STATUS_NOT_A_DIRECTORY: i32 = 0xC000_0103_u32 as i32;

// Win32 error codes the relevant NTSTATUS values map to (for `raw_os_error`).
// `ERROR_CANT_RESOLVE_FILENAME` (1921) is what the OS maps
// `STATUS_REPARSE_POINT_ENCOUNTERED` to — the reparse-refused signal callers
// match on, mirroring the Unix guard's `ELOOP` `raw_os_error`.
const ERROR_FILE_NOT_FOUND: i32 = 2;
const ERROR_PATH_NOT_FOUND: i32 = 3;
const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_DIRECTORY: i32 = 267;
pub const ERROR_CANT_RESOLVE_FILENAME: i32 = 1921;

/// Hard upper bound on the bytes [`read_under`] will buffer for one file — the
/// memory-DoS ceiling. Mirrors the Unix `MAX_GUARDED_READ_BYTES` (64 MiB): a
/// file beyond it is refused (`FileTooLarge`), never truncated to a wrong,
/// hashable prefix (B2 — a truncated buffer would certify content that is not
/// what is on disk).
pub const MAX_GUARDED_READ_BYTES: u64 = 64 * 1024 * 1024;

/// Reserved DOS device names (case-insensitive, with or without an extension).
/// A path component equal to one of these — or one of these followed by `.ext` —
/// is refused: on Windows such a name resolves to a device, not a file beneath
/// the root.
const RESERVED_DOS_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// A workspace-root-relative path that has passed Windows-hardened *structural*
/// escape validation. Reparse (symlink/junction) escape is a separate,
/// open-time guarantee enforced by [`read_under`] (`OBJ_DONT_REPARSE`), not by
/// this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinRelPath {
    /// Slash-joined normalised form (for diagnostics).
    joined: String,
    /// Individual components, in order — the ladder opens one per hop.
    components: Vec<String>,
}

impl WinRelPath {
    /// The slash-joined normalised path (for diagnostics).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.joined
    }
}

/// Why a client-supplied path was refused before any open was attempted. A
/// superset of the Unix `Escape`: Windows has more ambient ways for a string to
/// name something other than a file beneath the root.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WinEscape {
    /// Absolute path (`/x`, `\x`, or `C:\x`).
    #[error("path is absolute, refusing to resolve against a workspace root: {0:?}")]
    Absolute(String),
    /// Contains a `..` component.
    #[error("path escapes workspace root via `..`: {0:?}")]
    ParentEscape(String),
    /// Normalised to nothing.
    #[error("path is empty after normalisation: {0:?}")]
    Empty(String),
    /// Interior NUL byte.
    #[error("path contains an interior NUL byte: {0:?}")]
    NulByte(String),
    /// Backslash separator — the wire is slash-only; a backslash would be a
    /// second separator Windows honours but the normaliser does not split on.
    #[error("path contains a backslash separator (the wire is slash-only): {0:?}")]
    Backslash(String),
    /// Drive letter (`C:`) or drive-relative (`C:foo`) prefix.
    #[error("path carries a drive specifier: {0:?}")]
    Drive(String),
    /// `\\?\` / `\\.\` device-namespace prefix or UNC root.
    #[error("path is a device-namespace or UNC path: {0:?}")]
    DevicePath(String),
    /// Alternate-data-stream colon (`file:stream`).
    #[error("path names an alternate data stream: {0:?}")]
    AltDataStream(String),
    /// A component ends in a dot or space (Windows strips these, so the named
    /// file is not the byte-exact component).
    #[error("path component has a trailing dot or space: {0:?}")]
    TrailingDotOrSpace(String),
    /// A component is a reserved DOS device name (`CON`, `NUL`, `COM1`, …).
    #[error("path component is a reserved DOS device name: {0:?}")]
    ReservedName(String),
}

/// Normalise and structurally validate a client-supplied, root-relative path for
/// Windows. The wire is slash-separated; this rejects every ambient Windows way
/// a string could name something other than a plain file beneath the root.
///
/// # Errors
/// Returns [`WinEscape`] for any of the rejected forms above.
pub fn normalise_rel(path: &str) -> Result<WinRelPath, WinEscape> {
    let owned = path.to_string();
    if path.contains('\0') {
        return Err(WinEscape::NulByte(owned));
    }
    if path.contains('\\') {
        return Err(WinEscape::Backslash(owned));
    }
    // `\\?\` / `\\.\` would already be caught by the backslash check, but a
    // forward-slash device form (`//?/`) would not — refuse both shapes.
    if path.starts_with("//") {
        return Err(WinEscape::DevicePath(owned));
    }
    if path.starts_with('/') {
        return Err(WinEscape::Absolute(owned));
    }
    // Drive specifier: a colon in the first component is a drive letter
    // (`C:`/`C:foo`); a colon anywhere else is an alternate-data-stream.
    let first = path.split('/').next().unwrap_or("");
    if first.len() >= 2 && first.as_bytes()[1] == b':' {
        return Err(WinEscape::Drive(owned));
    }
    if path.contains(':') {
        return Err(WinEscape::AltDataStream(owned));
    }

    let mut components = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err(WinEscape::ParentEscape(owned)),
            other => {
                let trimmed = other.trim_end_matches([' ', '.']);
                if trimmed.len() != other.len() {
                    return Err(WinEscape::TrailingDotOrSpace(owned));
                }
                let stem = other.split('.').next().unwrap_or(other);
                if RESERVED_DOS_NAMES.contains(&stem.to_ascii_lowercase().as_str()) {
                    return Err(WinEscape::ReservedName(owned));
                }
                components.push(other.to_string());
            }
        }
    }
    if components.is_empty() {
        return Err(WinEscape::Empty(owned));
    }
    Ok(WinRelPath {
        joined: components.join("/"),
        components,
    })
}

/// The workspace root directory handle — the read anchor / identity (C2). Opened
/// once per admitted root and held; all reads anchor to it, so a later retarget
/// of the root path cannot redirect them. Closes on drop.
#[derive(Debug)]
pub struct WorkspaceDir(HANDLE);

// SAFETY: a Win32 directory HANDLE is a kernel-object reference safe to move
// between threads; the kernel synchronises its own access. We do not implement
// Sync (the daemon holds one per connection and uses it single-threaded for a
// verdict).
unsafe impl Send for WorkspaceDir {}

impl WorkspaceDir {
    /// Open `root` as the held directory handle. `FILE_FLAG_BACKUP_SEMANTICS` is
    /// required to obtain a *directory* handle from `CreateFileW`;
    /// `FILE_FLAG_OPEN_REPARSE_POINT` means the root itself is opened as-named
    /// (a retargeted/symlinked root is opened as the link object, not silently
    /// followed — admission decided this exact root).
    ///
    /// # Errors
    /// Propagates the open error (root missing / not a directory / access).
    pub fn open(root: &Path) -> io::Result<Self> {
        let wide: Vec<u16> = root.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 path; all other arguments
        // are plain scalars; the returned handle is owned by this `WorkspaceDir`
        // and closed in `Drop`.
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(handle))
    }

    /// Borrow the raw root handle (for the ladder's first hop). Callers must not
    /// close it — drop the `WorkspaceDir` instead.
    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for WorkspaceDir {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE && !self.0.is_null() {
            // SAFETY: owned handle from `CreateFileW`, closed exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// An owned intermediate handle from the ladder — closed on drop. The root
/// handle is never wrapped in this (it is owned by [`WorkspaceDir`]).
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE && !self.0.is_null() {
            // SAFETY: owned handle from `NtCreateFile`, closed exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// Read the full bytes of `rel` resolved beneath `dir`, refusing any reparse
/// point (symlink/junction) in the path. Component-by-component `NtCreateFile`
/// anchored at the prior handle with `OBJ_DONT_REPARSE` — the Windows analogue
/// of the Unix `O_NOFOLLOW` ladder.
///
/// # Errors
/// `ErrorKind::FilesystemLoop` for a reparse rejection
/// (`STATUS_REPARSE_POINT_ENCOUNTERED`), `ErrorKind::FileTooLarge` for an
/// over-ceiling file, or the underlying open/read error otherwise.
pub fn read_under(dir: &WorkspaceDir, rel: &WinRelPath) -> io::Result<Vec<u8>> {
    let (last, parents) = rel
        .components
        .split_last()
        .expect("WinRelPath is never empty by construction");

    // Descend the parent directories, each anchored at the previous handle and
    // refusing a reparse component. The root handle anchors the first hop and is
    // not owned here; each intermediate handle is dropped when the next replaces
    // it.
    let mut current: Option<OwnedHandle> = None;
    for component in parents {
        let anchor = current.as_ref().map_or_else(|| dir.raw(), |h| h.0);
        let next = nt_open_at(anchor, component, true)?;
        current = Some(OwnedHandle(next));
    }

    let anchor = current.as_ref().map_or_else(|| dir.raw(), |h| h.0);
    let file = OwnedHandle(nt_open_at(anchor, last, false)?);
    read_handle_capped(file.0, MAX_GUARDED_READ_BYTES)
}

/// Open a single path `component` relative to `parent`, refusing a reparse point
/// via `OBJ_DONT_REPARSE`. `directory` selects `FILE_DIRECTORY_FILE` vs
/// `FILE_NON_DIRECTORY_FILE`, so an intermediate that is not a directory (or a
/// leaf that is) fails closed.
fn nt_open_at(parent: HANDLE, component: &str, directory: bool) -> io::Result<HANDLE> {
    // UNICODE_STRING is counted (no NUL); `Length`/`MaximumLength` are byte
    // counts of the UTF-16 buffer.
    let mut name: Vec<u16> = component.encode_utf16().collect();
    let byte_len = u16::try_from(name.len() * 2)
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "path component too long"))?;
    let mut unicode = UNICODE_STRING {
        Length: byte_len,
        MaximumLength: byte_len,
        Buffer: name.as_mut_ptr(),
    };

    // SAFETY: OBJECT_ATTRIBUTES is a plain C struct; zero-init then fill the
    // members. `ObjectName` borrows `unicode`, which borrows `name`; all three
    // outlive the `NtCreateFile` call below.
    let mut attrs: OBJECT_ATTRIBUTES = unsafe { std::mem::zeroed() };
    attrs.Length = u32::try_from(std::mem::size_of::<OBJECT_ATTRIBUTES>())
        .expect("OBJECT_ATTRIBUTES size fits u32");
    attrs.RootDirectory = parent;
    attrs.ObjectName = &mut unicode;
    attrs.Attributes = OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE;

    let create_options = if directory {
        FILE_DIRECTORY_FILE
    } else {
        FILE_NON_DIRECTORY_FILE
    } | FILE_SYNCHRONOUS_IO_NONALERT
        | FILE_OPEN_FOR_BACKUP_INTENT;

    let mut handle: HANDLE = null_mut();
    let mut iosb: IO_STATUS_BLOCK = unsafe { std::mem::zeroed() };
    // SAFETY: `handle`/`iosb`/`attrs` are valid out/in pointers held for the
    // call; `name` backs `unicode` backs `attrs.ObjectName`. Null AllocationSize
    // / EaBuffer match an open (FILE_OPEN) with no extended attributes.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            GENERIC_READ | SYNCHRONIZE,
            &attrs,
            &mut iosb,
            null_mut(),
            FILE_ATTRIBUTE_NORMAL,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_OPEN,
            create_options,
            null_mut(),
            0,
        )
    };
    // Touch `name` after the call so the buffer provably outlives `NtCreateFile`.
    drop(name);

    match status {
        STATUS_SUCCESS => Ok(handle),
        // The `OBJ_DONT_REPARSE` rejection. Surface it as the Win32
        // `ERROR_CANT_RESOLVE_FILENAME` (1921) `raw_os_error` — the Windows
        // analogue of the Unix guard's `ELOOP` signal — so the daemon can map a
        // refused symlink/junction to `symlink-retarget` stale. (`io_error_more`'s
        // `ErrorKind::FilesystemLoop` is still unstable, so we signal via the OS
        // error code, not the kind.)
        STATUS_REPARSE_POINT_ENCOUNTERED => {
            Err(io::Error::from_raw_os_error(ERROR_CANT_RESOLVE_FILENAME))
        }
        other => Err(nt_status_to_io(other)),
    }
}

/// Read `handle` to EOF, refusing more than `max_bytes`. The buffer never grows
/// past the ceiling: a file over it is refused (`FileTooLarge`), never truncated
/// to a wrong, hashable prefix (B2).
fn read_handle_capped(handle: HANDLE, max_bytes: u64) -> io::Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let mut read: u32 = 0;
        // SAFETY: `handle` is a live read handle; `chunk` is a valid mutable
        // buffer of `len` bytes; `&mut read` is a valid out parameter; null
        // OVERLAPPED matches the synchronous handle.
        let ok = unsafe {
            ReadFile(
                handle,
                chunk.as_mut_ptr(),
                chunk.len() as u32,
                &mut read,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if read == 0 {
            break; // EOF
        }
        if buf.len() as u64 + u64::from(read) > max_bytes {
            return Err(io::Error::new(
                ErrorKind::FileTooLarge,
                format!("file exceeds the {max_bytes}-byte guarded-read ceiling"),
            ));
        }
        buf.extend_from_slice(&chunk[..read as usize]);
    }
    Ok(buf)
}

/// Map an `NTSTATUS` open failure to an `io::Error`. The handful of codes the
/// guard cares about carry a real Win32 `raw_os_error`; anything else surfaces
/// the raw status in the message. (Hand-mapped rather than via
/// `RtlNtStatusToDosError` so the FFI surface stays minimal.)
fn nt_status_to_io(status: i32) -> io::Error {
    let win32 = match status {
        STATUS_OBJECT_NAME_NOT_FOUND => ERROR_FILE_NOT_FOUND,
        STATUS_OBJECT_PATH_NOT_FOUND => ERROR_PATH_NOT_FOUND,
        STATUS_ACCESS_DENIED => ERROR_ACCESS_DENIED,
        STATUS_NOT_A_DIRECTORY => ERROR_DIRECTORY,
        _ => {
            return io::Error::other(format!("NtCreateFile failed: NTSTATUS {status:#010x}"));
        }
    };
    io::Error::from_raw_os_error(win32)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- normalise_rel (pure; runs wherever the crate compiles) ----

    #[test]
    fn accepts_and_collapses() {
        let rel = normalise_rel("src/./lib.rs").expect("clean path");
        assert_eq!(rel.as_str(), "src/lib.rs");
        assert_eq!(rel.components, vec!["src", "lib.rs"]);
    }

    #[test]
    fn rejects_absolute_and_drive_and_unc() {
        assert!(matches!(
            normalise_rel("/etc/x"),
            Err(WinEscape::Absolute(_))
        ));
        assert!(matches!(normalise_rel("C:/x"), Err(WinEscape::Drive(_))));
        assert!(matches!(normalise_rel("C:x"), Err(WinEscape::Drive(_))));
        assert!(matches!(
            normalise_rel("//server/share"),
            Err(WinEscape::DevicePath(_))
        ));
    }

    #[test]
    fn rejects_backslash_and_parent_and_nul() {
        assert!(matches!(
            normalise_rel("src\\lib.rs"),
            Err(WinEscape::Backslash(_))
        ));
        assert!(matches!(
            normalise_rel("../etc"),
            Err(WinEscape::ParentEscape(_))
        ));
        assert!(matches!(
            normalise_rel("a/../b"),
            Err(WinEscape::ParentEscape(_))
        ));
        assert!(matches!(
            normalise_rel("a/\0/b"),
            Err(WinEscape::NulByte(_))
        ));
    }

    #[test]
    fn rejects_alt_data_stream() {
        assert!(matches!(
            normalise_rel("src/lib.rs:secret"),
            Err(WinEscape::AltDataStream(_))
        ));
    }

    #[test]
    fn rejects_trailing_dot_or_space() {
        assert!(matches!(
            normalise_rel("src/lib.rs "),
            Err(WinEscape::TrailingDotOrSpace(_))
        ));
        assert!(matches!(
            normalise_rel("src/evil."),
            Err(WinEscape::TrailingDotOrSpace(_))
        ));
    }

    #[test]
    fn rejects_reserved_dos_names() {
        assert!(matches!(
            normalise_rel("CON"),
            Err(WinEscape::ReservedName(_))
        ));
        assert!(matches!(
            normalise_rel("src/nul.txt"),
            Err(WinEscape::ReservedName(_))
        ));
        assert!(matches!(
            normalise_rel("COM1"),
            Err(WinEscape::ReservedName(_))
        ));
        // A name that merely contains a reserved stem as a substring is fine.
        assert!(normalise_rel("console.ts").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(normalise_rel(""), Err(WinEscape::Empty(_))));
        assert!(matches!(normalise_rel("."), Err(WinEscape::Empty(_))));
        assert!(matches!(normalise_rel("./"), Err(WinEscape::Empty(_))));
    }

    // ---- read_under (real filesystem; Windows-only behaviour) ----

    #[test]
    fn reads_real_nested_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(tmp.path().join("a/b")).unwrap();
        std::fs::write(tmp.path().join("a/b/c.rs"), b"nested").unwrap();

        let dir = WorkspaceDir::open(tmp.path()).expect("open root");
        let rel = normalise_rel("a/b/c.rs").unwrap();
        assert_eq!(read_under(&dir, &rel).expect("read"), b"nested");
    }

    #[test]
    fn refuses_symlinked_leaf() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("real.txt"), b"x").unwrap();
        // Requires Developer Mode or admin; skip cleanly where unprivileged.
        if std::os::windows::fs::symlink_file("real.txt", tmp.path().join("link.txt")).is_err() {
            eprintln!("skipping: symlink creation requires privilege on this runner");
            return;
        }
        let dir = WorkspaceDir::open(tmp.path()).expect("open root");
        let rel = normalise_rel("link.txt").unwrap();
        let err = read_under(&dir, &rel).expect_err("a symlinked leaf must be refused");
        assert_eq!(
            err.raw_os_error(),
            Some(ERROR_CANT_RESOLVE_FILENAME),
            "reparse rejection must surface ERROR_CANT_RESOLVE_FILENAME: {err}"
        );
    }

    #[test]
    fn refuses_symlinked_parent_component() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(tmp.path().join("real")).unwrap();
        std::fs::write(tmp.path().join("real/secret"), b"x").unwrap();
        if std::os::windows::fs::symlink_dir("real", tmp.path().join("link")).is_err() {
            eprintln!("skipping: symlink creation requires privilege on this runner");
            return;
        }
        let dir = WorkspaceDir::open(tmp.path()).expect("open root");
        let rel = normalise_rel("link/secret").unwrap();
        let err = read_under(&dir, &rel).expect_err("a symlinked parent must be refused");
        assert_eq!(
            err.raw_os_error(),
            Some(ERROR_CANT_RESOLVE_FILENAME),
            "reparse rejection must surface ERROR_CANT_RESOLVE_FILENAME: {err}"
        );
    }

    #[test]
    fn stale_root_handle_fails_closed_not_reresolved() {
        // C2: the held handle is the workspace identity. After admission,
        // retargeting the root path must NOT redirect reads — the held handle
        // still anchors to the original directory object. Mirrors the Unix
        // `stale_root_dirfd_fails_closed_not_reresolved` test.
        let parent = tempfile::tempdir().expect("tempdir");
        let root = parent.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("marker"), b"original").unwrap();

        // Admission: open the handle once (shared for delete/rename).
        let dir = WorkspaceDir::open(&root).expect("open root");

        // Retarget: move the original aside, plant a new dir whose `marker`
        // differs. Renaming a directory with a live handle needs the runner to
        // permit it (the handle is opened FILE_SHARE_DELETE); skip cleanly if
        // the platform refuses.
        let moved = parent.path().join("root-old");
        if std::fs::rename(&root, &moved).is_err() {
            eprintln!("skipping: runner does not permit renaming a dir with an open handle");
            return;
        }
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("marker"), b"attacker-swapped").unwrap();

        // Reads through the held handle still see the ORIGINAL content — proof
        // the path string was not re-resolved against the swapped-in directory.
        let rel = normalise_rel("marker").unwrap();
        let bytes = read_under(&dir, &rel).expect("read via held handle");
        assert_eq!(
            bytes, b"original",
            "held handle must anchor to the original object, not the swapped-in root"
        );
    }

    #[test]
    fn read_handle_capped_refuses_oversized_without_truncating() {
        // B2: an over-ceiling file is refused, never truncated to a wrong
        // hashable prefix. Exercised directly against `read_handle_capped` with
        // a tiny cap (the real 64 MiB ceiling is impractical to hit in a test),
        // mirroring the Unix `read_fd_capped_refuses_oversized_without_truncating`.
        use std::os::windows::io::AsRawHandle;
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("ten.bin");
        std::fs::write(&path, b"0123456789").expect("write 10 bytes");
        let open = || std::fs::File::open(&path).expect("open");

        // Under the cap: full content.
        let f = open();
        assert_eq!(
            read_handle_capped(f.as_raw_handle() as HANDLE, 100).expect("small read"),
            b"0123456789"
        );
        // Exactly at the cap: allowed.
        let f = open();
        assert_eq!(
            read_handle_capped(f.as_raw_handle() as HANDLE, 10)
                .expect("at-cap read")
                .len(),
            10
        );
        // Over the cap: refused, never a truncated prefix.
        let f = open();
        let err = read_handle_capped(f.as_raw_handle() as HANDLE, 5).expect_err("over-cap refused");
        assert_eq!(err.kind(), ErrorKind::FileTooLarge);
    }
}
