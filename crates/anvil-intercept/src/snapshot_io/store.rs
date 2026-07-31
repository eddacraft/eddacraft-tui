//! Key-agnostic sealed-artefact disk I/O (ADR-105 §10).
//!
//! Shared primitive under base-store and worktree snapshot paths.

use std::fs::{self, DirBuilder, File};
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::Path;

/// The size cap shared with the per-worktree snapshot class, re-exported on the
/// key-agnostic seam so callers (the parent module's `load_snapshot`, and the
/// future `graph_base_producer`) size-cap their [`load_sealed`] reads from one
/// authoritative constant. The value itself lives in the lean `anvil-graph-cache`
/// format crate (ADR-064).
pub use anvil_graph_cache::snapshot::MAX_SNAPSHOT_BYTES;

/// Owner-only mode for a sealed-artifact directory (ADR-069 §2).
pub const DIR_MODE: u32 = 0o700;
/// Owner-only mode for a sealed-artifact / temp file (ADR-069 §2/§4).
pub const FILE_MODE: u32 = 0o600;
/// Suffix for the in-progress temp file a sealed publish creates then renames
/// away. An interrupted publish leaves one behind; the parent module's temp sweep
/// reclaims it.
pub const TMP_EXT: &str = "tmp";

/// Why a sealed load could not return bytes. Every variant is a **discard +
/// cold-rebuild** signal for the caller, which maps it into its own richer,
/// format-aware error taxonomy (the parent module folds these into
/// `SnapshotReadError`). Distinguishes "no artefact" (a normal cold start) from a
/// disk error and from an artefact that is present but structurally unusable
/// (not a regular file, or over the size cap) — the format gate itself lives in
/// the caller, so this enum deliberately carries no decode variant.
#[derive(Debug)]
pub enum LoadSealedError {
    /// No file at the leaf — the expected first-run / fresh case.
    NotFound,
    /// A disk error opening / stat-ing / reading the file (not a decode failure).
    Io(io::Error),
    /// The leaf is a symlink or otherwise not a regular file — never a legitimate
    /// sealed artefact.
    NotRegularFile,
    /// The file exceeds the caller-supplied size cap (checked on the stat and,
    /// again, at the read fd against a grow-between-stat-and-open TOCTOU).
    Oversized,
}

/// Atomically and durably publish `bytes` at the separator-free leaf `final_name`
/// inside `dir` (ADR-069 §4). Creates `dir` (mode `0700`) if absent, then runs the
/// full sealed publish under a single validated directory fd. This is the
/// path-based public seam the shared base-snapshot producer uses; the parent
/// module's `write_snapshot`, which additionally publishes a `.root` companion
/// under the *same* dirfd, opens the dirfd itself and calls `publish_sealed_at`
/// directly.
///
/// # Errors
/// Any `io::Error` from the ensure-dir / open-dirfd / create / write / fsync /
/// rename path. On a pre-rename failure the temp is unlinked; nothing is
/// published.
pub fn write_sealed(dir: &Path, final_name: &str, bytes: &[u8]) -> io::Result<()> {
    validate_leaf_name(final_name)?;
    ensure_dir(dir)?;
    let dirfd = crate::path_safety::open_workspace_dir_for_fsync(dir)?;
    publish_sealed_at(&dirfd, final_name, bytes)
}

/// Reject any `name` that is not a single, separator-free path component
/// (`InvalidInput`). The `openat2(RESOLVE_BENEATH)` path refuses traversal by
/// construction, but the `O_NOFOLLOW`-`openat` **fallback** (non-Linux, or
/// `openat2` ENOSYS/EPERM) only guards the leaf — a `..` or `a/b` name would
/// resolve intermediate components there. The seam functions are `pub`, so the
/// documented invariant is enforced at runtime, not by caller convention;
/// internal callers always pass hash-derived basenames and are unaffected.
fn validate_leaf_name(name: &str) -> io::Result<()> {
    let invalid = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0');
    if invalid {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("sealed-store name must be a single separator-free component, got {name:?}"),
        ));
    }
    Ok(())
}

/// The sealed publish core, anchored at an already-open, validated **real**
/// (fsync-able) directory fd (ADR-069 §4 / CIB-097). Creates the temp via
/// `openat`/`openat2` RELATIVE to `dirfd` using only the temp BASENAME, with
/// `O_CREAT | O_EXCL | O_NOFOLLOW` at `0600` from the first syscall (no
/// default-umask-then-chmod window, and a planted symlink / pre-created temp fails
/// the create rather than redirecting it); `write_all` + `sync_all`; then publishes
/// via `renameat(dirfd, tmp, dirfd, final_name)` — atomic within the same dir
/// (temp and target share `dir`, so `EXDEV` cannot arise) and not symlink-following.
///
/// Returns `Ok(())` once the rename has succeeded (the artefact is **published** —
/// visible at the final name). A pre-rename failure unlinks the orphaned temp
/// (anchored at the same dirfd; a `NotFound` is ignored) and surfaces the original
/// error. Reaching the publish means the file IS durable enough to serve: the
/// subsequent parent-directory `fsync` failing only leaves the dentry's
/// crash-durability unconfirmed (worst case: one cold rebuild after an ill-timed
/// crash), which `note_publish_durability` folds to a WARN rather than a hard
/// error — so a published write still reports `Ok`.
///
/// The `dirfd` must be a real `O_DIRECTORY` fd (not an `O_PATH` one — `O_PATH`
/// cannot be `fsync`'d); [`crate::path_safety::open_workspace_dir_for_fsync`]
/// yields exactly that.
pub(crate) fn publish_sealed_at(dirfd: &OwnedFd, final_name: &str, bytes: &[u8]) -> io::Result<()> {
    let tmp_name = temp_name(final_name);

    // The closure's `?` short-circuits a pre-rename failure into the `Err` arm
    // below (nothing published; clean up the temp).
    let create_to_rename = (|| -> io::Result<()> {
        let mut file = create_leaf_under_dirfd(dirfd, &tmp_name)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        // Atomic publish anchored at the same dirfd.
        nix::fcntl::renameat(dirfd, tmp_name.as_str(), dirfd, final_name)
            .map_err(io::Error::from)?;
        Ok(())
    })();

    if let Err(err) = create_to_rename {
        // Pre-rename failure only: the temp still exists (or never got created) —
        // best-effort cleanup via `unlinkat` anchored at the same dirfd, then
        // surface the original failure.
        let _ = nix::unistd::unlinkat(
            dirfd,
            tmp_name.as_str(),
            nix::unistd::UnlinkatFlags::NoRemoveDir,
        );
        Err(err)
    } else {
        // The rename succeeded — the artefact is published at its final name.
        // Whether the directory fsync then succeeds or not, the published file is
        // durable-enough to serve (CIB-092g). The same real dirfd is the fsync
        // target (an `O_PATH` fd could not be `fsync`'d).
        note_publish_durability(nix::unistd::fsync(dirfd).map_err(io::Error::from));
        Ok(())
    }
}

/// Load the raw bytes of the separator-free leaf `name` from `dir`, size-capped at
/// `max_bytes` and symlink-safe (ADR-069 §1/§4). Stats + size-caps before reading;
/// opens anchored beneath an `O_PATH` dirfd under `openat2(RESOLVE_NO_SYMLINKS |
/// RESOLVE_BENEATH)` (with the `O_NOFOLLOW`-`openat` ladder fallback). The caller
/// runs its own format / integrity gate over the returned bytes.
///
/// # Errors
/// [`LoadSealedError::NotFound`] when there is no file (normal cold start);
/// [`LoadSealedError::Io`] on a disk error; [`LoadSealedError::NotRegularFile`]
/// when the leaf is a symlink or not a regular file; [`LoadSealedError::Oversized`]
/// when the body exceeds `max_bytes`.
pub fn load_sealed(dir: &Path, name: &str, max_bytes: u64) -> Result<Vec<u8>, LoadSealedError> {
    validate_leaf_name(name).map_err(LoadSealedError::Io)?;
    let path = dir.join(name);

    let metadata = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(LoadSealedError::NotFound);
        }
        Err(err) => return Err(LoadSealedError::Io(err)),
    };
    // A symlink at the leaf is never a legitimate artefact — refuse it (the
    // O_NOFOLLOW open below would also fail; reject early + explicitly). The
    // anchored open is the actual security guard against a same-uid symlink swap
    // in the stat→open window; this early check is just a clear fast-path
    // rejection.
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LoadSealedError::NotRegularFile);
    }
    // Size cap on the stat (cheap pre-check).
    if metadata.len() > max_bytes {
        return Err(LoadSealedError::Oversized);
    }

    // Anchor the read to an `O_PATH` dirfd held on `dir` and open the
    // single-component leaf **relative to it** under `openat2(RESOLVE_NO_SYMLINKS |
    // RESOLVE_BENEATH)` (with the `O_NOFOLLOW`-`openat` ladder fallback), rather
    // than a path-based `open` with only a leaf-`O_NOFOLLOW`. This refuses a
    // symlinked leaf *and* a symlinked / `..`-escaping dir component, and cannot be
    // redirected by a same-uid swap of an intermediate directory in the stat→open
    // window. The `metadata.len()` pre-cap above is a cheap fast-reject; the held
    // fd below is the security-bearing read.
    let file = open_sealed_for_read(dir, name).map_err(LoadSealedError::Io)?;
    // Cap the actual READ at the open fd, not just the pre-stat size: a file that
    // grew between `symlink_metadata` and `open` (a TOCTOU on a network/FUSE mount)
    // cannot drive `read_to_end` past the cap. `take(MAX + 1)` lets a genuine
    // over-cap file be detected and rejected rather than truncated.
    let cap = max_bytes + 1;
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len().min(cap)).unwrap_or(0));
    file.take(cap)
        .read_to_end(&mut bytes)
        .map_err(LoadSealedError::Io)?;
    if bytes.len() as u64 > max_bytes {
        return Err(LoadSealedError::Oversized);
    }

    Ok(bytes)
}

/// Note the post-rename durability outcome (CIB-092g / ADR-069 §4). The rename has
/// already published the artefact at its final path, so a failing **directory**
/// fsync is a *semi-success*: the file is durably written and visible; only the
/// directory entry's crash-durability is unconfirmed (worst case: one cold rebuild
/// after an ill-timed crash, which the ADR accepts). It is therefore logged at WARN
/// and never propagated as a "persistence failed" write error for an
/// already-published file — so the write call still reports `Ok`. Returns nothing
/// (the write is already a success) but is split out so the fold is unit-testable
/// without forcing a real `fsync` failure.
fn note_publish_durability(fsync_dir_result: io::Result<()>) {
    if let Err(err) = fsync_dir_result {
        tracing::warn!(
            target: "anvil_intercept::snapshot",
            error = %err,
            "snapshot published but directory fsync failed; durable-but-not-crash-guaranteed (ADR-069 §4)",
        );
    }
}

/// Create `dir` (and parents) at owner-only mode `0700` if absent, then validate a
/// pre-existing one (ADR-069 §2).
///
/// # Errors
/// Any `io::Error` from the create, or `validate_secure_dir` refusing an
/// externally-planted symlinked / non-owned / group-or-other-accessible dir.
pub(crate) fn ensure_dir(dir: &Path) -> io::Result<()> {
    // `recursive(true)` is idempotent on a pre-existing dir, so no `is_dir`
    // pre-check (which would only add a TOCTOU window).
    DirBuilder::new()
        .recursive(true)
        .mode(DIR_MODE)
        .create(dir)?;
    // Validate the dir's security properties (mirrors the fence store's owner-only
    // state-dir discipline): a pre-existing dir that is a symlink, not owned by us,
    // or group/other-accessible means a redirected / tampered `ANVIL_HOME` /
    // `XDG_STATE_HOME` — refuse to write there rather than undermine the owner-only
    // / symlink-safe contract. The caller degrades to no-persistence. (A dir we
    // just created is `0700` and owned by us; this only ever rejects an
    // externally-planted one.)
    validate_secure_dir(dir)
}

/// Reject a sealed-artifact dir that is a symlink, not a directory, not owned by
/// the current euid, or accessible by group/other (`mode & 0o077 != 0`).
fn validate_secure_dir(dir: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(dir)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "snapshot dir is a symlink or not a directory",
        ));
    }
    if meta.uid() != nix::unistd::geteuid().as_raw() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "snapshot dir is not owned by the current user",
        ));
    }
    if meta.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "snapshot dir is group/other-accessible",
        ));
    }
    Ok(())
}

/// Open the leaf `name` (a single, separator-free component) **relative to an
/// `O_PATH` dirfd** held on `dir`, for reading (ADR-069 §4 / CIB-092d). Uses the
/// shipped [`open_workspace_dirfd`](crate::path_safety::open_workspace_dirfd) as
/// the anchor, then [`open_leaf_under_dirfd`] on top. A symlinked leaf or a
/// symlinked dir is refused (`ELOOP`); the name cannot escape `dir`.
fn open_sealed_for_read(dir: &Path, name: &str) -> io::Result<File> {
    let dirfd = crate::path_safety::open_workspace_dirfd(dir)?;
    let leaf_fd = open_leaf_under_dirfd(&dirfd, name)?;
    // `File::from(OwnedFd)` takes sole ownership of the fd — no `unsafe`.
    Ok(File::from(leaf_fd))
}

/// Open a single, separator-free `name` for reading beneath `dirfd`, refusing a
/// symlinked leaf or escape. Linux: one `openat2` (with the `O_NOFOLLOW`-`openat`
/// fallback on `ENOSYS`/`EPERM`); other Unix: `O_NOFOLLOW` `openat`. Mirrors the
/// platform discipline in [`crate::path_safety::read_under`]. Part of the
/// key-agnostic seam: the base-snapshot producer reads its companion metadata
/// through the same anchored open.
pub fn open_leaf_under_dirfd(
    dirfd: &std::os::fd::OwnedFd,
    name: &str,
) -> io::Result<std::os::fd::OwnedFd> {
    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::Mode;
    use std::os::fd::AsFd;

    validate_leaf_name(name)?;

    let nofollow_openat = |fd: std::os::fd::BorrowedFd<'_>| -> io::Result<std::os::fd::OwnedFd> {
        openat(
            fd,
            name,
            OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(io::Error::from)
    };

    #[cfg(target_os = "linux")]
    {
        use nix::errno::Errno;
        use nix::fcntl::{OpenHow, ResolveFlag, openat2};
        let how = OpenHow::new()
            .flags(OFlag::O_RDONLY | OFlag::O_CLOEXEC)
            .resolve(ResolveFlag::RESOLVE_NO_SYMLINKS | ResolveFlag::RESOLVE_BENEATH);
        match openat2(dirfd.as_fd(), name, how) {
            // `openat2` absent (pre-5.6 kernel ENOSYS, or a seccomp EPERM on the
            // unknown syscall) ⇒ fall back to the `O_NOFOLLOW` `openat`, exactly as
            // `path_safety::read_under` does.
            Err(err)
                if matches!(
                    err as i32,
                    code if code == Errno::ENOSYS as i32 || code == Errno::EPERM as i32
                ) =>
            {
                nofollow_openat(dirfd.as_fd())
            }
            other => other.map_err(io::Error::from),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        nofollow_openat(dirfd.as_fd())
    }
}

/// Create a single, separator-free temp `name` beneath `dirfd` with
/// `O_CREAT | O_EXCL | O_WRONLY | O_NOFOLLOW` at mode `0600`, returning it as a
/// writable [`File`] (ADR-069 §4 / CIB-097). The WRITE-side mirror of
/// [`open_leaf_under_dirfd`]: on Linux one `openat2(RESOLVE_NO_SYMLINKS |
/// RESOLVE_BENEATH)` carrying the create flags + mode (with the `O_NOFOLLOW`
/// `openat` ladder fallback on `ENOSYS`/`EPERM`); other Unix uses the
/// `O_NOFOLLOW` `openat` create directly.
///
/// `O_EXCL` means a planted symlink / pre-created temp at `name` fails the
/// create (fails closed) rather than redirecting the write, and the `0600` mode
/// is applied from the first syscall — no default-umask-then-chmod window.
pub fn create_leaf_under_dirfd(dirfd: &std::os::fd::OwnedFd, name: &str) -> io::Result<File> {
    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::Mode;
    use std::os::fd::AsFd;

    // Enforce the documented single-component invariant at runtime: the
    // `O_NOFOLLOW` `openat` fallback only guards the *leaf*, so a multi-component
    // `name` would let a caller traverse intermediate symlinked components
    // unsafely. Internal callers pass separator-free
    // `snapshot_filename`/`temp_name` basenames and are unaffected.
    validate_leaf_name(name)?;

    let mode = Mode::from_bits_truncate(FILE_MODE as nix::libc::mode_t);
    let create_flags =
        OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_WRONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;

    let nofollow_openat = |fd: std::os::fd::BorrowedFd<'_>| -> io::Result<std::os::fd::OwnedFd> {
        openat(fd, name, create_flags, mode).map_err(io::Error::from)
    };

    #[cfg(target_os = "linux")]
    let leaf_fd = {
        use nix::errno::Errno;
        use nix::fcntl::{OpenHow, ResolveFlag, openat2};
        // `openat2` supports creation flags, so the create can ride the same
        // RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH anchor the read path uses.
        let how = OpenHow::new()
            .flags(create_flags)
            .mode(mode)
            .resolve(ResolveFlag::RESOLVE_NO_SYMLINKS | ResolveFlag::RESOLVE_BENEATH);
        match openat2(dirfd.as_fd(), name, how) {
            // `openat2` absent (pre-5.6 ENOSYS, or a seccomp EPERM on the unknown
            // syscall) ⇒ fall back to the `O_NOFOLLOW` `openat` create, exactly as
            // `open_leaf_under_dirfd` does for the read side.
            Err(err)
                if matches!(
                    err as i32,
                    code if code == Errno::ENOSYS as i32 || code == Errno::EPERM as i32
                ) =>
            {
                nofollow_openat(dirfd.as_fd())?
            }
            other => other.map_err(io::Error::from)?,
        }
    };
    #[cfg(not(target_os = "linux"))]
    let leaf_fd = nofollow_openat(dirfd.as_fd())?;

    // `File::from(OwnedFd)` takes sole ownership of the fd — no `unsafe`, keeping
    // the crate's `forbid(unsafe_code)` honest while reusing the existing
    // `write_all` + `sync_all` (file fsync) code above.
    Ok(File::from(leaf_fd))
}

/// `unlinkat(dirfd, name, 0)` for a single separator-free leaf, surfacing the
/// error as `io::Result`. Anchored at the validated dirfd so an intermediate
/// directory swap cannot redirect the unlink (CIB-097 discipline).
pub(crate) fn unlink_at(dirfd: &std::os::fd::OwnedFd, name: &str) -> io::Result<()> {
    nix::unistd::unlinkat(dirfd, name, nix::unistd::UnlinkatFlags::NoRemoveDir)
        .map_err(io::Error::from)
}

/// `<final>.<rand-hex>.tmp` — a randomised suffix so a same-uid attacker cannot
/// pre-create the temp path (combined with `O_EXCL`, the create then fails
/// closed rather than being redirected).
fn temp_name(final_name: &str) -> String {
    let mut rand = [0u8; 8];
    // A randomness failure is implausible on supported hosts; fall back to the
    // pid (widened to 8 bytes) so we never block a write on it. `O_EXCL` is the
    // actual correctness guard — a colliding temp name fails the create.
    if getrandom::fill(&mut rand).is_err() {
        rand = u64::from(std::process::id()).to_le_bytes();
    }
    let mut suffix = String::with_capacity(16);
    for b in rand {
        use std::fmt::Write as _;
        let _ = write!(suffix, "{b:02x}");
    }
    format!("{final_name}.{suffix}.{TMP_EXT}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // The dir the sealed write creates is owner-only 0700; in production it is a
    // `graph-cache` subdir, never the (possibly group-readable) state-dir root, so
    // the write tests use a fresh subdir the writer creates + validates.
    fn gc(tmp: &tempfile::TempDir) -> std::path::PathBuf {
        tmp.path().join("graph-cache")
    }

    #[test]
    fn write_sealed_then_load_sealed_round_trips_raw_bytes() {
        // The key-agnostic seam round-trips an opaque payload through disk with no
        // knowledge of its format — the property the base-snapshot producer relies
        // on.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        let bytes = b"opaque-sealed-artifact-bytes";
        write_sealed(&dir, "artefact.bin", bytes).expect("write");
        let got = load_sealed(&dir, "artefact.bin", MAX_SNAPSHOT_BYTES as u64).expect("load");
        assert_eq!(got, bytes);
    }

    #[test]
    fn load_sealed_missing_is_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            load_sealed(tmp.path(), "absent.bin", MAX_SNAPSHOT_BYTES as u64),
            Err(LoadSealedError::NotFound)
        ));
    }

    #[test]
    fn load_sealed_refuses_a_symlink_at_the_leaf() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("real-secret");
        fs::write(&target, b"secret").unwrap();
        std::os::unix::fs::symlink(&target, tmp.path().join("link.bin")).unwrap();
        // A planted symlink is refused, never followed.
        assert!(matches!(
            load_sealed(tmp.path(), "link.bin", MAX_SNAPSHOT_BYTES as u64),
            Err(LoadSealedError::NotRegularFile)
        ));
    }

    #[test]
    fn load_sealed_rejects_an_oversized_body() {
        // The cap is enforced both on the stat pre-check and at the read fd; a body
        // over the cap is Oversized, never truncated.
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("big.bin"), vec![b'x'; 32]).unwrap();
        assert!(matches!(
            load_sealed(tmp.path(), "big.bin", 16),
            Err(LoadSealedError::Oversized)
        ));
    }

    #[test]
    fn note_publish_durability_never_panics_on_fsync_failure() {
        // CIB-092g: after the rename has published the file, a failing directory
        // fsync is folded to a WARN (durable-but-not-crash-guaranteed), never a hard
        // write error — `note_publish_durability` returns `()` either way, so the
        // surrounding sealed publish still reports `Ok`.
        note_publish_durability(Ok(()));
        note_publish_durability(Err(io::Error::other("fsync EIO")));
    }

    #[test]
    fn open_sealed_for_read_is_anchored_and_refuses_a_symlinked_leaf() {
        // CIB-092d: the read opens the leaf relative to an O_PATH dirfd under
        // openat2(RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH) (or the O_NOFOLLOW openat
        // fallback). A real file opens; a symlinked leaf is refused.
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("real.snap"), b"bytes").unwrap();
        let mut got = Vec::new();
        open_sealed_for_read(dir, "real.snap")
            .expect("a real leaf opens under the dirfd")
            .read_to_end(&mut got)
            .unwrap();
        assert_eq!(got, b"bytes");

        // A symlinked leaf is refused (ELOOP under RESOLVE_NO_SYMLINKS / O_NOFOLLOW).
        std::os::unix::fs::symlink(dir.join("real.snap"), dir.join("link.snap")).unwrap();
        assert!(
            open_sealed_for_read(dir, "link.snap").is_err(),
            "a symlinked snapshot leaf must be refused by the anchored open",
        );
    }

    #[test]
    fn create_leaf_under_dirfd_is_anchored_and_refuses_a_planted_symlink() {
        // CIB-097: the WRITE path creates the temp leaf relative to a real
        // (fsync-able) directory fd under openat2(RESOLVE_NO_SYMLINKS |
        // RESOLVE_BENEATH) with O_CREAT|O_EXCL (or the O_NOFOLLOW openat
        // fallback), mirroring the read path's anchored-open discipline. A
        // fresh name is created 0600; a planted symlink at the leaf is refused
        // (O_EXCL fails closed — the create never follows it).
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        ensure_dir(&dir).unwrap();
        let dirfd =
            crate::path_safety::open_workspace_dir_for_fsync(&dir).expect("open writable dirfd");

        // A fresh name is created from the first syscall at 0600.
        {
            let mut f =
                create_leaf_under_dirfd(&dirfd, "fresh.snap.tmp").expect("create fresh leaf");
            f.write_all(b"payload").unwrap();
        }
        let created = dir.join("fresh.snap.tmp");
        assert_eq!(
            fs::metadata(&created).unwrap().permissions().mode() & 0o777,
            FILE_MODE,
            "the anchored create must be owner-only 0600 from the first syscall",
        );

        // A planted symlink where the leaf would be created is refused — the
        // create fails closed (O_EXCL / O_NOFOLLOW / RESOLVE_NO_SYMLINKS) rather
        // than following the symlink and writing through it.
        let secret = tmp.path().join("secret");
        fs::write(&secret, b"do-not-clobber").unwrap();
        std::os::unix::fs::symlink(&secret, dir.join("evil.snap.tmp")).unwrap();
        assert!(
            create_leaf_under_dirfd(&dirfd, "evil.snap.tmp").is_err(),
            "a planted symlink at the temp leaf must be refused by the anchored create",
        );
        // The symlink target was never written through.
        assert_eq!(fs::read(&secret).unwrap(), b"do-not-clobber");
    }

    /// Every `pub` seam entry point rejects a non-leaf `name` with
    /// `InvalidInput` at runtime — the `O_NOFOLLOW`-`openat` fallback path only
    /// guards the leaf, so traversal/escape names must never reach a syscall.
    #[test]
    fn seam_rejects_non_leaf_names() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("store");
        fs::create_dir_all(&dir).unwrap();
        let dirfd = crate::path_safety::open_workspace_dirfd(&dir).unwrap();

        for bad in ["", ".", "..", "a/b", "a\\b", "../escape", "a\0b"] {
            let err = write_sealed(&dir, bad, b"x").unwrap_err();
            assert_eq!(
                err.kind(),
                io::ErrorKind::InvalidInput,
                "write_sealed({bad:?})"
            );

            match load_sealed(&dir, bad, 1024) {
                Err(LoadSealedError::Io(err)) => {
                    assert_eq!(
                        err.kind(),
                        io::ErrorKind::InvalidInput,
                        "load_sealed({bad:?})"
                    );
                }
                other => {
                    panic!("load_sealed({bad:?}) must reject with Io(InvalidInput), got {other:?}")
                }
            }

            let err = open_leaf_under_dirfd(&dirfd, bad).unwrap_err();
            assert_eq!(
                err.kind(),
                io::ErrorKind::InvalidInput,
                "open_leaf_under_dirfd({bad:?})"
            );

            let err = create_leaf_under_dirfd(&dirfd, bad).unwrap_err();
            assert_eq!(
                err.kind(),
                io::ErrorKind::InvalidInput,
                "create_leaf_under_dirfd({bad:?})"
            );
        }

        // Nothing escaped: the parent of `dir` contains only `dir` itself.
        let entries: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("store")]);
    }
}
