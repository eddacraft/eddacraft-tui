//! DSV-010b (ADR-070 step 2): the platform-neutral workspace read anchor.
//!
//! The verdict path reads *arbitrary on-disk paths a client names*, so the read
//! is anchored at a workspace directory handle opened **once** at admission (the
//! workspace identity, security C2) and every path is resolved beneath it with a
//! no-symlink / no-reparse, stay-beneath-root guarantee. Each platform has a
//! real anchor with that guarantee, behind one type:
//!
//! - **Unix:** a held `O_PATH` dirfd ([`crate::path_safety::open_workspace_dirfd`])
//!   read via `openat2(RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH)` or the
//!   `O_NOFOLLOW` ladder ([`crate::path_safety::read_under`]).
//! - **Windows:** a held directory `HANDLE`
//!   ([`anvil_intercept_win32::read_safety::WorkspaceDir`]) read via a
//!   per-component `NtCreateFile` + `OBJ_DONT_REPARSE` ladder (ADR-068).
//!
//! Both refuse a symlink/junction anywhere in the path, structurally reject
//! `..`/absolute/escape forms before any open, and **refuse — never truncate —**
//! a file over the 64 MiB guarded-read ceiling (B2). Because the handle is held,
//! a root-directory retarget *after* admission cannot redirect reads: the handle
//! is the identity, so reads either hit the original object or fail closed (C2).
//!
//! `forbid(unsafe_code)` is inherited from the crate lint — the Unix path goes
//! through `nix`'s safe wrappers and the Windows path quarantines all `unsafe`
//! FFI in `anvil-intercept-win32`.

use std::io;
use std::path::Path;

/// A held workspace-root read anchor and workspace identity (security C2).
///
/// Opened once per admitted root via [`WorkspaceAnchor::open`] and held; every
/// [`WorkspaceAnchor::read_rel`] resolves beneath the held handle, so a later
/// retarget of the root path cannot redirect reads. Closes its handle on drop.
#[derive(Debug)]
pub struct WorkspaceAnchor {
    #[cfg(unix)]
    dirfd: std::os::fd::OwnedFd,
    #[cfg(windows)]
    dir: anvil_intercept_win32::read_safety::WorkspaceDir,
}

impl WorkspaceAnchor {
    /// Open `root` as the held read anchor / identity.
    ///
    /// # Errors
    /// Propagates the underlying open error (root missing, not a directory, or
    /// access-denied).
    pub fn open(root: &Path) -> io::Result<Self> {
        #[cfg(unix)]
        {
            Ok(Self {
                dirfd: crate::path_safety::open_workspace_dirfd(root)?,
            })
        }
        #[cfg(windows)]
        {
            Ok(Self {
                dir: anvil_intercept_win32::read_safety::WorkspaceDir::open(root)?,
            })
        }
    }

    /// Read the full bytes of the client-supplied, root-relative `rel`, resolved
    /// beneath the held anchor. The path is first normalised + structurally
    /// validated (absolute / `..` / NUL — plus, on Windows, backslash / drive /
    /// UNC / device / alternate-data-stream / trailing-dot / reserved-name), then
    /// read with a per-platform no-symlink / no-reparse guarantee. A symlink or
    /// junction anywhere in the path fails the read; a file over the 64 MiB
    /// ceiling is refused (`FileTooLarge`), never truncated.
    ///
    /// # Errors
    /// Returns an [`io::Error`]: `InvalidInput` for a structurally-refused path,
    /// a symlink/reparse rejection (Unix `ELOOP`; Windows
    /// `ERROR_CANT_RESOLVE_FILENAME`), `FileTooLarge` for an over-ceiling file, or
    /// any other open/read failure (e.g. `NotFound`).
    pub fn read_rel(&self, rel: &str) -> io::Result<Vec<u8>> {
        #[cfg(unix)]
        {
            self.read_rel_capped(rel, crate::path_safety::MAX_GUARDED_READ_BYTES)
        }
        #[cfg(windows)]
        {
            self.read_rel_capped(
                rel,
                anvil_intercept_win32::read_safety::MAX_GUARDED_READ_BYTES,
            )
        }
    }

    /// Read `rel` beneath the held workspace anchor, refusing any symlink,
    /// reparse, or structural escape and refusing input larger than
    /// `max_bytes`.
    ///
    /// The limit is enforced while reading the already-guarded handle, so a
    /// consumer can apply a smaller capability-specific allocation ceiling
    /// without weakening workspace containment. Oversized input is refused,
    /// never returned as a truncated prefix.
    ///
    /// # Errors
    /// Returns the same path/open errors as [`Self::read_rel`], or
    /// [`io::ErrorKind::FileTooLarge`] when the file exceeds `max_bytes`.
    pub fn read_rel_capped(&self, rel: &str, max_bytes: u64) -> io::Result<Vec<u8>> {
        #[cfg(unix)]
        {
            use std::os::fd::AsFd;
            let parsed = crate::path_safety::normalise_rel(rel).map_err(|escape| {
                io::Error::new(io::ErrorKind::InvalidInput, format!("{escape:?}"))
            })?;
            crate::path_safety::read_under_capped(self.dirfd.as_fd(), &parsed, max_bytes)
        }
        #[cfg(windows)]
        {
            let parsed =
                anvil_intercept_win32::read_safety::normalise_rel(rel).map_err(|escape| {
                    io::Error::new(io::ErrorKind::InvalidInput, format!("{escape:?}"))
                })?;
            anvil_intercept_win32::read_safety::read_under_capped(&self.dir, &parsed, max_bytes)
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn reads_a_real_file_beneath_the_anchor() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), b"fn main() {}").unwrap();

        let anchor = WorkspaceAnchor::open(tmp.path()).expect("open anchor");
        assert_eq!(
            anchor.read_rel("src/lib.rs").expect("read"),
            b"fn main() {}"
        );
    }

    #[test]
    fn caller_limit_is_enforced_before_returning_guarded_bytes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("ten.bin"), b"0123456789").expect("write 10 bytes");
        let anchor = WorkspaceAnchor::open(tmp.path()).expect("open anchor");

        let err = anchor
            .read_rel_capped("ten.bin", 5)
            .expect_err("a guarded read over the caller limit must be refused");

        assert_eq!(err.kind(), io::ErrorKind::FileTooLarge);
        assert_eq!(
            anchor
                .read_rel_capped("ten.bin", 10)
                .expect("exact-limit read"),
            b"0123456789"
        );
    }

    #[test]
    fn structurally_refuses_escape_before_any_read() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let anchor = WorkspaceAnchor::open(tmp.path()).expect("open anchor");
        // `..` is refused structurally (InvalidInput), not as a read failure.
        let err = anchor
            .read_rel("../etc/passwd")
            .expect_err("escape refused");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "{err}");
    }

    #[test]
    fn refuses_a_symlink_in_the_resolution_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        symlink("/etc/hostname", tmp.path().join("escape")).unwrap();
        let anchor = WorkspaceAnchor::open(tmp.path()).expect("open anchor");
        let err = anchor
            .read_rel("escape")
            .expect_err("a symlink in the path must be refused");
        assert_eq!(
            err.raw_os_error(),
            Some(nix::errno::Errno::ELOOP as i32),
            "{err}"
        );
    }

    #[test]
    fn held_anchor_fails_closed_after_root_retarget() {
        // C2: the held handle is the workspace identity. After admission,
        // replacing the root directory must NOT redirect reads.
        let parent = tempfile::tempdir().expect("tempdir");
        let root = parent.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("marker"), b"original").unwrap();

        let anchor = WorkspaceAnchor::open(&root).expect("open anchor");

        let moved = parent.path().join("root-old");
        std::fs::rename(&root, &moved).unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("marker"), b"attacker-swapped").unwrap();

        assert_eq!(
            anchor.read_rel("marker").expect("read via held anchor"),
            b"original",
            "the held anchor must hit the original inode, not the swapped-in root",
        );
    }
}
