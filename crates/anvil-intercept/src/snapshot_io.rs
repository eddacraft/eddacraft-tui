//! DSV-030 (ADR-069 §2/§4/§9/§10): the daemon-side warm-graph snapshot disk I/O.
//!
//! The sealed serialization core (the `SnapshotPayload` DTO, `postcard` codec,
//! magic/version/CRC integrity gate, privacy allowlist, golden round-trip
//! fixture) lives in [`anvil_graph_cache::snapshot`] (ADR-064: that crate stays
//! lean — `petgraph` + `serde` + the codec, no `nix`). This module is the part
//! the ADR assigns to the daemon: **timing and durable, symlink-safe disk I/O**,
//! which needs the platform syscall surface (`O_EXCL`, `O_NOFOLLOW`, `fsync`,
//! atomic `rename`) the lean graph-cache crate must not carry.
//!
//! Unix-only. On other platforms warm-start persistence is simply off (the
//! Windows daemon's persistence is a follow-up, mirroring the DSV-010/011
//! Windows-parity split); callers cfg-gate accordingly.
//!
//! # Crash-safety (ADR-069 §4)
//!
//! [`write_snapshot`] serialises to a temp file in the **same directory**,
//! created `O_CREAT | O_EXCL | O_NOFOLLOW` at mode `0600` with a randomised
//! suffix (defeating a same-uid pre-create / planted-symlink race), `fsync`s it,
//! `rename`s it over the target (atomic — temp and target share the dir, so
//! `EXDEV` cannot arise), then `fsync`s the parent directory so the rename is
//! durable across a crash. A crash at any point leaves the old snapshot or no
//! snapshot — never a torn one. A write failure unlinks the temp and propagates;
//! the caller degrades to no-persistence (never wedges).
//!
//! # Read-safety (ADR-069 §4)
//!
//! [`load_snapshot`] stats the file and rejects anything over
//! [`MAX_SNAPSHOT_BYTES`] **before** reading (no allocation bomb), opens it
//! `O_NOFOLLOW` (a planted symlink at the path cannot redirect the read), then
//! hands the bytes to the graph-cache integrity gate
//! ([`SnapshotPayload::from_bytes`]). Every anomaly maps to "discard and
//! cold-rebuild" — the load path never panics and never refuses to start.
#![cfg(unix)]

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use anvil_graph_cache::snapshot::{
    MAX_SNAPSHOT_BYTES, SnapshotLoadError, SnapshotPayload, snapshot_filename,
};

/// Owner-only mode for the snapshot directory (ADR-069 §2).
const DIR_MODE: u32 = 0o700;
/// Owner-only mode for a snapshot / temp file (ADR-069 §2/§4).
const FILE_MODE: u32 = 0o600;
/// Suffix for the in-progress temp file. Swept on start ([`sweep_orphan_temps`]).
const TMP_EXT: &str = "tmp";

/// Why a snapshot could not be loaded. Every variant ⇒ **cold rebuild** (ADR-069
/// §3); the caller logs per §10 (`NotFound` → DEBUG, `Rejected(VersionMismatch)`
/// → INFO, the rest → WARN). Distinguishes "no snapshot" (a normal cold start)
/// from "a snapshot was present but unusable", which [`SnapshotLoadError`] alone
/// cannot (it has no `NotFound`/`Io` variant — those are disk concerns the
/// lean graph-cache crate does not model).
#[derive(Debug)]
pub enum SnapshotReadError {
    /// No snapshot file for this key — the expected first-run / fresh-worktree
    /// case. Logged at DEBUG; a cold rebuild, not a problem.
    NotFound,
    /// A disk error opening/stat-ing/reading the file (not a decode failure).
    Io(io::Error),
    /// The file decoded-path rejected it: bad magic, version/schema mismatch,
    /// checksum/count mismatch, oversize, or a corrupt body (ADR-069 §1 gate).
    Rejected(SnapshotLoadError),
}

/// The persistent graph-cache directory, `<state-dir>/graph-cache` (ADR-069 §2).
///
/// Resolution mirrors the daemon's PID/socket resolver (ADR-060) but prefers the
/// **persistent** state dir, never the ephemeral runtime dir: `ANVIL_HOME` →
/// `<prefix>/graph-cache`; else `$XDG_STATE_HOME/anvil/graph-cache`; else
/// `$HOME/.local/state/anvil/graph-cache`. Returns `None` when no home can be
/// resolved (persistence then stays off).
#[must_use]
pub fn graph_cache_dir() -> Option<PathBuf> {
    graph_cache_dir_from(
        anvil_home_prefix_env(),
        non_empty_env("XDG_STATE_HOME"),
        non_empty_env("HOME").or_else(|| non_empty_env("USERPROFILE")),
    )
}

/// Pure resolver for [`graph_cache_dir`], taking the candidate roots explicitly
/// so it unit-tests without mutating the process environment.
#[must_use]
fn graph_cache_dir_from(
    anvil_home: Option<PathBuf>,
    xdg_state_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(prefix) = anvil_home {
        return Some(prefix.join("graph-cache"));
    }
    if let Some(state) = xdg_state_home {
        return Some(state.join("anvil").join("graph-cache"));
    }
    home.map(|h| {
        h.join(".local")
            .join("state")
            .join("anvil")
            .join("graph-cache")
    })
}

fn non_empty_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// `ANVIL_HOME` as an absolute prefix (mirrors `lib::anvil_home_prefix`): a
/// whitespace-only value is treated as unset; a relative value is absolutised
/// against the cwd, falling back to the raw value if the cwd is unavailable.
fn anvil_home_prefix_env() -> Option<PathBuf> {
    let raw = std::env::var_os("ANVIL_HOME").filter(|v| !v.is_empty())?;
    if raw.to_str().is_some_and(|s| s.trim().is_empty()) {
        return None;
    }
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        Some(p)
    } else {
        std::env::current_dir().map_or(Some(p.clone()), |cwd| Some(cwd.join(p)))
    }
}

/// Atomically and durably write `payload` for the worktree at `canonical_root`
/// into `dir` (ADR-069 §4). Creates `dir` (mode `0700`) if absent.
///
/// # Errors
/// Any `io::Error` from the create/write/fsync/rename path. The temp file is
/// unlinked on failure; the caller logs + degrades to no-persistence.
pub fn write_snapshot(
    dir: &Path,
    canonical_root: &Path,
    payload: &SnapshotPayload,
) -> io::Result<()> {
    ensure_dir(dir)?;

    let final_name = snapshot_filename(canonical_root);
    let tmp_name = temp_name(&final_name);
    let tmp_path = dir.join(&tmp_name);
    let final_path = dir.join(&final_name);

    // Deferred under CIB-092d: anchor the temp create + rename to the validated
    // `O_PATH` dirfd (`open_workspace_dirfd(dir)` + `openat(O_CREAT|O_EXCL|O_NOFOLLOW)`
    // + `renameat` + a dirfd `fsync`), mirroring the read path's openat2 discipline.
    // Deferred (not the read fix): the shipped `path_safety` helpers cover only the
    // *read* side (`read_under`/`read_under_openat2`), so the write would need a new
    // hand-rolled `openat`/`renameat` ladder — a larger change than this slice
    // carries. The current write is already symlink-safe at the leaf
    // (`O_EXCL | O_NOFOLLOW`, randomised temp) and `validate_secure_dir` rejects a
    // symlinked / non-owned / group-writable `dir` before any write, so the
    // residual gap is only the dir-component-swap atomicity the read path now closes.
    //
    // Create `O_CREAT | O_EXCL | O_NOFOLLOW` at 0600 from the first syscall — no
    // default-umask-then-chmod window, and a planted symlink/pre-created temp
    // fails the create rather than redirecting the write.
    //
    // Returns `Ok(())` once the `rename` has succeeded (the new snapshot is
    // **published** — visible at `final_path`). The closure's `?` short-circuits a
    // pre-rename failure into the `Err` arm below (nothing published; clean up the
    // temp); reaching `Ok(())` means the file IS durable enough to serve (see the
    // `fsync_dir` note below).
    let create_to_rename = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(FILE_MODE)
            .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC)
            .open(&tmp_path)?;
        file.write_all(&payload.to_bytes())?;
        file.sync_all()?;
        // Atomic publish: temp and target share `dir`, so `rename` is atomic and
        // cannot return EXDEV.
        fs::rename(&tmp_path, &final_path)?;
        Ok(())
    })();

    if let Err(err) = create_to_rename {
        // Failure *before* the rename published anything: best-effort cleanup
        // of the orphaned temp (the rename may already have consumed it on some
        // paths, so ignore a NotFound), then surface the failure.
        let _ = fs::remove_file(&tmp_path);
        Err(err)
    } else {
        // The rename succeeded — the snapshot is published at `final_path`.
        // Whether the directory fsync then succeeds or not, the published file
        // is durable-enough to serve (CIB-092g); `note_publish_durability` folds
        // a fsync failure to a WARN rather than a hard error.
        note_publish_durability(fsync_dir(dir));
        Ok(())
    }
}

/// Note the post-rename durability outcome (CIB-092g / ADR-069 §4). The rename has
/// already published the snapshot at its final path, so a failing **directory**
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

/// Load and validate the snapshot for `canonical_root` from `dir` (ADR-069
/// §1/§4). Stats + size-caps before reading; opens `O_NOFOLLOW`.
///
/// # Errors
/// [`SnapshotReadError::NotFound`] when there is no snapshot (normal cold start);
/// [`SnapshotReadError::Io`] on a disk error; [`SnapshotReadError::Rejected`]
/// when the integrity gate rejects the bytes. Every case ⇒ cold rebuild.
pub fn load_snapshot(
    dir: &Path,
    canonical_root: &Path,
) -> Result<SnapshotPayload, SnapshotReadError> {
    let filename = snapshot_filename(canonical_root);
    let path = dir.join(&filename);

    let metadata = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(SnapshotReadError::NotFound);
        }
        Err(err) => return Err(SnapshotReadError::Io(err)),
    };
    // A symlink at the snapshot path is never a legitimate snapshot — refuse it
    // (the O_NOFOLLOW open below would also fail; reject early + explicitly).
    // `O_NOFOLLOW` on the open is the actual security guard against a same-uid
    // symlink swap in the stat→open window; this early check is just a clear
    // fast-path rejection.
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(SnapshotReadError::Rejected(SnapshotLoadError::Corrupt));
    }
    // Size cap on the stat (cheap pre-check).
    if metadata.len() > MAX_SNAPSHOT_BYTES as u64 {
        return Err(SnapshotReadError::Rejected(SnapshotLoadError::Oversized));
    }

    // ADR-069 §4 (CIB-092d): anchor the read to an `O_PATH` dirfd held on the
    // graph-cache dir and open the single-component snapshot name **relative to
    // it** under `openat2(RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH)` (with the
    // `O_NOFOLLOW`-`openat` ladder fallback where `openat2` is unavailable), rather
    // than a path-based `open` with only a leaf-`O_NOFOLLOW`. This refuses a
    // symlinked snapshot *and* a symlinked/`..`-escaping dir component, and cannot
    // be redirected by a same-uid swap of an intermediate directory in the
    // stat→open window. The `metadata.len()` pre-cap above is a cheap fast-reject;
    // the held fd below is the security-bearing read.
    let file = open_snapshot_for_read(dir, &filename).map_err(SnapshotReadError::Io)?;
    // Cap the actual READ at the open fd, not just the pre-stat size: a file that
    // grew between `symlink_metadata` and `open` (a TOCTOU on a network/FUSE
    // mount) cannot drive `read_to_end` past the cap. `take(MAX + 1)` lets a
    // genuine over-cap file be detected and rejected rather than truncated.
    let cap = MAX_SNAPSHOT_BYTES as u64 + 1;
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len().min(cap)).unwrap_or(0));
    file.take(cap)
        .read_to_end(&mut bytes)
        .map_err(SnapshotReadError::Io)?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SnapshotReadError::Rejected(SnapshotLoadError::Oversized));
    }

    SnapshotPayload::from_bytes(&bytes).map_err(SnapshotReadError::Rejected)
}

/// Remove the snapshot for `canonical_root` (ADR-069 §10: a key's snapshot is
/// dropped when its workspace is unregistered/evicted). A missing file is not an
/// error.
///
/// # Errors
/// A disk error other than the file already being absent.
pub fn remove_snapshot(dir: &Path, canonical_root: &Path) -> io::Result<()> {
    let path = dir.join(snapshot_filename(canonical_root));
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// Sweep orphaned `*.tmp` files left by an interrupted write (ADR-069 §10),
/// returning the count removed. Best-effort: an unreadable dir or a file that
/// vanishes mid-sweep is skipped, never fatal. A missing `dir` is a no-op.
pub fn sweep_orphan_temps(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some(TMP_EXT)
            && fs::remove_file(&path).is_ok()
        {
            removed += 1;
        }
    }
    removed
}

/// Sweep stale `*.snap` snapshots on daemon start (ADR-069 §10, CIB-092c): remove
/// any snapshot whose worktree key is **not** in `registered_roots`, returning the
/// count removed. A worktree deleted while the daemon was down (so its unregister
/// hook never fired) otherwise leaves its `.snap` on disk forever; this reclaims
/// it. Snapshots for currently-registered worktrees are kept.
///
/// `registered_roots` are the roots the daemon knows about. Each is **canonicalized
/// inside the sweep before hashing** (mirroring the write path, which hashes a
/// canonicalized root) — a caller that passes a non-canonical form (trailing slash,
/// unresolved symlink) must not cause a live snapshot to be misclassified as an
/// orphan and deleted. When a root cannot be canonicalized (e.g. it transiently
/// vanished) the raw form is kept in the keep-set as well, so the failure can only
/// ever *retain* a snapshot, never delete a live one. Each kept name is the on-disk
/// name [`snapshot_filename`] would own; a `.snap` whose name matches no registered
/// root is an orphan and is removed.
///
/// **Runtime empty-guard (CIB-092 council survivor):** if `registered_roots` is
/// empty the sweep returns `0` immediately **without deleting anything**. The
/// function name `on_start` is exactly when the daemon's session registry is empty,
/// and an empty keep-set would otherwise reclaim *every* warm-start snapshot — the
/// opposite of the intent. A faithful, non-empty registered set is the caller's
/// contract; an empty one is treated as "nothing is known yet, touch nothing".
///
/// Best-effort: an unreadable dir or a file that vanishes mid-sweep is skipped,
/// never fatal; a missing `dir` is a no-op. `*.tmp` files are left to
/// [`sweep_orphan_temps`] — this sweep only touches `*.snap`.
pub fn sweep_stale_snapshots_on_start<I, P>(dir: &Path, registered_roots: I) -> usize
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    use std::collections::HashSet;
    // Canonicalize each root before hashing (the write path canonicalizes), but
    // keep the raw form too so a canonicalize failure can only ever retain — never
    // delete — a snapshot.
    let keep: HashSet<String> = registered_roots
        .into_iter()
        .flat_map(|root| {
            let raw = root.as_ref().to_path_buf();
            let canonical = fs::canonicalize(&raw).unwrap_or_else(|_| raw.clone());
            [snapshot_filename(&raw), snapshot_filename(&canonical)]
        })
        .collect();
    // Empty-guard: no registered roots ⇒ delete nothing (a doc-comment is not a
    // guard; the on-start registry is empty at exactly the catastrophic moment).
    if keep.is_empty() {
        return 0;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        // Only `*.snap` files; `*.tmp` is the temp-sweep's job, and a subdir is
        // never a snapshot.
        if path.extension().and_then(|e| e.to_str()) != Some("snap") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // A registered worktree's snapshot is kept; anything else is an orphan.
        if keep.contains(name) {
            continue;
        }
        if fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Create `dir` (and parents) at owner-only mode `0700` if absent (ADR-069 §2).
fn ensure_dir(dir: &Path) -> io::Result<()> {
    // `recursive(true)` is idempotent on a pre-existing dir, so no `is_dir`
    // pre-check (which would only add a TOCTOU window).
    DirBuilder::new()
        .recursive(true)
        .mode(DIR_MODE)
        .create(dir)?;
    // Validate the dir's security properties (mirrors the fence store's
    // owner-only state-dir discipline): a pre-existing `graph-cache` that is a
    // symlink, not owned by us, or group/other-accessible means a redirected /
    // tampered `ANVIL_HOME`/`XDG_STATE_HOME` — refuse to write there rather than
    // undermine the owner-only / symlink-safe contract. The caller degrades to
    // no-persistence. (A dir we just created is `0700` and owned by us; this only
    // ever rejects an externally-planted one.)
    validate_secure_dir(dir)
}

/// Reject a snapshot dir that is a symlink, not a directory, not owned by the
/// current euid, or accessible by group/other (`mode & 0o077 != 0`).
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

/// `fsync` a directory so a rename into it is durable. Opening a directory
/// read-only and `fsync`-ing the fd is the portable POSIX idiom.
fn fsync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// Open the snapshot `name` (a single, separator-free component) **relative to an
/// `O_PATH` dirfd** held on `dir`, for reading (ADR-069 §4 / CIB-092d). Uses the
/// shipped [`open_workspace_dirfd`](crate::path_safety::open_workspace_dirfd) as
/// the anchor, then `openat2(RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH)` on Linux
/// (falling back to an `O_NOFOLLOW` `openat` where `openat2` is unavailable —
/// `ENOSYS`/`EPERM` — and on non-Linux Unix). A symlinked leaf or a symlinked dir
/// is refused (`ELOOP`); the name cannot escape `dir`.
fn open_snapshot_for_read(dir: &Path, name: &str) -> io::Result<File> {
    let dirfd = crate::path_safety::open_workspace_dirfd(dir)?;
    let leaf_fd = open_leaf_under_dirfd(&dirfd, name)?;
    // `File::from(OwnedFd)` takes sole ownership of the fd — no `unsafe`.
    Ok(File::from(leaf_fd))
}

/// Open a single, separator-free `name` for reading beneath `dirfd`, refusing a
/// symlinked leaf or escape. Linux: one `openat2` (with the `O_NOFOLLOW`-`openat`
/// fallback on `ENOSYS`/`EPERM`); other Unix: `O_NOFOLLOW` `openat`. Mirrors the
/// platform discipline in [`crate::path_safety::read_under`].
fn open_leaf_under_dirfd(
    dirfd: &std::os::fd::OwnedFd,
    name: &str,
) -> io::Result<std::os::fd::OwnedFd> {
    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::Mode;
    use std::os::fd::AsFd;

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
    use anvil_graph_cache::snapshot::snapshot_filename;
    use anvil_graph_cache::{DependencyGraph, SymbolGraph};
    use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};

    fn payload() -> SnapshotPayload {
        let mut sym = SymbolGraph::new();
        sym.add_symbol(SymbolNode {
            id: 1,
            kind: SymbolKind::Function,
            name: "alpha".to_owned(),
            visibility: Visibility::Public,
            file: "src/a.ts".to_owned(),
            trust_level: TrustLevel::Internal,
        })
        .unwrap();
        let mut dep = DependencyGraph::new();
        dep.add_dependency("src/a.ts".to_owned(), "src/b.ts".to_owned());
        SnapshotPayload::from_graphs(&sym, &dep).unwrap()
    }

    // The snapshot dir `write_snapshot` creates is owner-only 0700; in production
    // it is a `graph-cache` subdir, never the (possibly group-readable) state-dir
    // root, so the write tests use a fresh subdir the writer creates + validates.
    fn gc(tmp: &tempfile::TempDir) -> PathBuf {
        tmp.path().join("graph-cache")
    }

    #[test]
    fn write_then_load_round_trips_through_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        let root = Path::new("/some/workspace/root");
        let want = payload();

        write_snapshot(&dir, root, &want).expect("write");
        let got = load_snapshot(&dir, root).expect("load");

        // `SnapshotPayload: PartialEq` is byte-equality (the golden-fixture
        // property), so this proves the on-disk round-trip is lossless.
        assert_eq!(got, want);
    }

    #[test]
    fn write_is_owner_only_and_leaves_no_temp() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        let root = Path::new("/ws");
        write_snapshot(&dir, root, &payload()).expect("write");

        let file = dir.join(snapshot_filename(root));
        let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, FILE_MODE, "snapshot must be owner-only 0600");
        // The created dir is owner-only 0700.
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            DIR_MODE
        );

        // No leftover temp after a successful write.
        let temps = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some(TMP_EXT))
            .count();
        assert_eq!(temps, 0, "a successful write leaves no .tmp");
    }

    #[test]
    fn load_missing_is_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            load_snapshot(tmp.path(), Path::new("/ws")),
            Err(SnapshotReadError::NotFound)
        ));
    }

    #[test]
    fn load_garbage_is_rejected_not_panicked() {
        let tmp = tempfile::tempdir().unwrap();
        let root = Path::new("/ws");
        fs::write(tmp.path().join(snapshot_filename(root)), b"not a snapshot").unwrap();
        assert!(matches!(
            load_snapshot(tmp.path(), root),
            Err(SnapshotReadError::Rejected(_))
        ));
    }

    #[test]
    fn load_refuses_a_symlink_at_the_snapshot_path() {
        let tmp = tempfile::tempdir().unwrap();
        let root = Path::new("/ws");
        let target = tmp.path().join("real-secret");
        fs::write(&target, b"secret").unwrap();
        std::os::unix::fs::symlink(&target, tmp.path().join(snapshot_filename(root))).unwrap();
        // A planted symlink is refused, never followed.
        assert!(matches!(
            load_snapshot(tmp.path(), root),
            Err(SnapshotReadError::Rejected(_))
        ));
    }

    #[test]
    fn sweep_orphan_temps_removes_only_temps() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.snap"), b"keep").unwrap();
        fs::write(tmp.path().join("a.snap.deadbeef.tmp"), b"orphan").unwrap();
        fs::write(tmp.path().join("b.snap.cafef00d.tmp"), b"orphan").unwrap();

        assert_eq!(sweep_orphan_temps(tmp.path()), 2);
        assert!(tmp.path().join("a.snap").exists(), "real snapshot kept");
        assert!(!tmp.path().join("a.snap.deadbeef.tmp").exists());
    }

    #[test]
    fn sweep_stale_snapshots_removes_only_unregistered() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let registered = Path::new("/ws/registered");
        let orphan = Path::new("/ws/deleted-while-down");
        // Two real snapshot files (one for a still-registered worktree, one for a
        // worktree deleted while the daemon was down) + a temp the .snap sweep
        // must not touch.
        fs::write(dir.join(snapshot_filename(registered)), b"keep").unwrap();
        fs::write(dir.join(snapshot_filename(orphan)), b"orphan").unwrap();
        let leftover_tmp = dir.join(format!("{}.deadbeef.tmp", snapshot_filename(orphan)));
        fs::write(&leftover_tmp, b"temp").unwrap();

        let removed = sweep_stale_snapshots_on_start(dir, [registered]);

        assert_eq!(removed, 1, "only the unregistered .snap is removed");
        assert!(
            dir.join(snapshot_filename(registered)).exists(),
            "a registered worktree's snapshot is kept",
        );
        assert!(
            !dir.join(snapshot_filename(orphan)).exists(),
            "the orphaned snapshot is reclaimed",
        );
        assert!(
            leftover_tmp.exists(),
            "the .snap sweep must not touch *.tmp files"
        );
    }

    #[test]
    fn sweep_stale_snapshots_with_no_registered_roots_deletes_nothing() {
        // CIB-092 council survivor (item 2): an EMPTY registered-root set is the
        // catastrophic case — the function name `on_start` is exactly when the
        // session registry is empty. A runtime guard must short-circuit to 0
        // WITHOUT deleting anything, so a fresh-boot call cannot wipe every
        // warm-start snapshot. (This INVERTS the prior destructive behaviour.)
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join(snapshot_filename(Path::new("/a"))), b"a").unwrap();
        fs::write(dir.join(snapshot_filename(Path::new("/b"))), b"b").unwrap();
        let empty: [&Path; 0] = [];
        assert_eq!(
            sweep_stale_snapshots_on_start(dir, empty),
            0,
            "an empty registered set must delete NOTHING (empty-guard)",
        );
        assert_eq!(
            fs::read_dir(dir).unwrap().count(),
            2,
            "no registered roots ⇒ the guard keeps every snapshot on disk",
        );
    }

    #[test]
    fn sweep_stale_snapshots_keeps_a_root_passed_with_a_trailing_slash() {
        // CIB-092 council survivor (item 2): the write path hashes a *canonicalized*
        // root, but the sweep is handed whatever roots a caller has. A caller that
        // passes a non-canonical form (trailing slash) must NOT cause the LIVE
        // snapshot — written under the canonical name — to be misclassified as an
        // orphan and deleted. The sweep canonicalizes each root before hashing.
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("graph-cache");
        // A real, canonical worktree directory that exists (so canonicalize resolves).
        let worktree = tmp.path().join("live-worktree");
        fs::create_dir(&worktree).unwrap();
        let canonical = fs::canonicalize(&worktree).unwrap();
        // The snapshot on disk is written under the CANONICAL filename (write path).
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(snapshot_filename(&canonical)), b"live").unwrap();

        // The caller hands the sweep the same root but with a trailing-slash
        // (non-canonical) form. The live snapshot must be kept, not reclaimed.
        let with_slash = PathBuf::from(format!("{}/", worktree.display()));
        let removed = sweep_stale_snapshots_on_start(&dir, [with_slash]);
        assert_eq!(
            removed, 0,
            "a trailing-slash root must still keep its snapshot"
        );
        assert!(
            dir.join(snapshot_filename(&canonical)).exists(),
            "the live snapshot survives a non-canonical registered root",
        );
    }

    #[test]
    fn sweep_stale_snapshots_missing_dir_is_a_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("does-not-exist");
        assert_eq!(
            sweep_stale_snapshots_on_start(&missing, [Path::new("/ws")]),
            0,
        );
    }

    #[test]
    fn open_snapshot_for_read_is_anchored_and_refuses_a_symlinked_leaf() {
        // CIB-092d: the read opens the leaf relative to an O_PATH dirfd under
        // openat2(RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH) (or the O_NOFOLLOW openat
        // fallback). A real file opens; a symlinked leaf is refused.
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("real.snap"), b"bytes").unwrap();
        let mut got = Vec::new();
        open_snapshot_for_read(dir, "real.snap")
            .expect("a real leaf opens under the dirfd")
            .read_to_end(&mut got)
            .unwrap();
        assert_eq!(got, b"bytes");

        // A symlinked leaf is refused (ELOOP under RESOLVE_NO_SYMLINKS / O_NOFOLLOW).
        std::os::unix::fs::symlink(dir.join("real.snap"), dir.join("link.snap")).unwrap();
        assert!(
            open_snapshot_for_read(dir, "link.snap").is_err(),
            "a symlinked snapshot leaf must be refused by the anchored open",
        );
    }

    #[test]
    fn note_publish_durability_never_panics_on_fsync_failure() {
        // CIB-092g: after the rename has published the file, a failing directory
        // fsync is folded to a WARN (durable-but-not-crash-guaranteed), never a hard
        // write error — `note_publish_durability` returns `()` either way, so the
        // surrounding `write_snapshot` still reports `Ok`.
        note_publish_durability(Ok(()));
        note_publish_durability(Err(io::Error::other("fsync EIO")));
    }

    #[test]
    fn write_reports_ok_and_publishes_despite_post_rename_fsync() {
        // The happy path publishes a loadable snapshot AND reports Ok — the 092g
        // regression guard: a post-rename fsync_dir failure is no longer a write
        // failure (and the rename, not the fsync, is what makes the file readable).
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        let root = Path::new("/ws-092g");
        write_snapshot(&dir, root, &payload()).expect("a published write reports Ok");
        assert!(
            dir.join(snapshot_filename(root)).exists(),
            "snapshot published"
        );
        load_snapshot(&dir, root).expect("the published snapshot loads");
    }

    #[test]
    fn remove_snapshot_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        let root = Path::new("/ws");
        write_snapshot(&dir, root, &payload()).unwrap();
        remove_snapshot(&dir, root).expect("first remove");
        // A second remove (already gone) is not an error.
        remove_snapshot(&dir, root).expect("idempotent remove");
        assert!(!dir.join(snapshot_filename(root)).exists());
    }

    #[test]
    fn write_refuses_a_symlinked_snapshot_dir() {
        // A planted symlink at the snapshot dir (redirected ANVIL_HOME) is
        // refused, never written through (mirrors the fence-store owner-only
        // discipline).
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real");
        fs::create_dir(&real).unwrap();
        let link = tmp.path().join("graph-cache");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let err = write_snapshot(&link, Path::new("/ws"), &payload())
            .expect_err("a symlinked snapshot dir must be refused");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        // Nothing was written into the redirected target.
        assert_eq!(fs::read_dir(&real).unwrap().count(), 0);
    }

    #[test]
    fn write_refuses_a_group_accessible_dir() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("graph-cache");
        fs::create_dir(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        let err = write_snapshot(&dir, Path::new("/ws"), &payload())
            .expect_err("a group/other-accessible dir must be refused");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn graph_cache_dir_prefers_anvil_home_then_xdg_then_home() {
        assert_eq!(
            graph_cache_dir_from(Some(PathBuf::from("/anvilhome")), None, None),
            Some(PathBuf::from("/anvilhome/graph-cache")),
        );
        assert_eq!(
            graph_cache_dir_from(None, Some(PathBuf::from("/xdg/state")), None),
            Some(PathBuf::from("/xdg/state/anvil/graph-cache")),
        );
        assert_eq!(
            graph_cache_dir_from(None, None, Some(PathBuf::from("/home/u"))),
            Some(PathBuf::from("/home/u/.local/state/anvil/graph-cache")),
        );
        assert_eq!(graph_cache_dir_from(None, None, None), None);
    }
}
