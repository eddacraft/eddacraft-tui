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
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
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

    // Create `O_CREAT | O_EXCL | O_NOFOLLOW` at 0600 from the first syscall — no
    // default-umask-then-chmod window, and a planted symlink/pre-created temp
    // fails the create rather than redirecting the write.
    let write_result = (|| -> io::Result<()> {
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
        // Durably commit the directory entry so the rename survives a crash (a
        // file fsync alone does not guarantee this on ext4).
        fsync_dir(dir)?;
        Ok(())
    })();

    if write_result.is_err() {
        // Best-effort cleanup of the orphaned temp; the rename may already have
        // consumed it, so ignore a NotFound.
        let _ = fs::remove_file(&tmp_path);
    }
    write_result
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
    let path = dir.join(snapshot_filename(canonical_root));

    let metadata = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(SnapshotReadError::NotFound);
        }
        Err(err) => return Err(SnapshotReadError::Io(err)),
    };
    // A symlink at the snapshot path is never a legitimate snapshot — refuse it
    // (the O_NOFOLLOW open would also fail, but reject early + explicitly).
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(SnapshotReadError::Rejected(SnapshotLoadError::Corrupt));
    }
    // Size cap BEFORE reading — a crafted length cannot trigger a huge read.
    if metadata.len() > MAX_SNAPSHOT_BYTES as u64 {
        return Err(SnapshotReadError::Rejected(SnapshotLoadError::Oversized));
    }

    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC)
        .open(&path)
        .map_err(SnapshotReadError::Io)?;
    // `metadata.len()` is already proven `<= MAX_SNAPSHOT_BYTES` above; the
    // `try_from` keeps the cast honest on a 32-bit target.
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    file.read_to_end(&mut bytes)
        .map_err(SnapshotReadError::Io)?;

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

/// Create `dir` (and parents) at owner-only mode `0700` if absent (ADR-069 §2).
fn ensure_dir(dir: &Path) -> io::Result<()> {
    if dir.is_dir() {
        return Ok(());
    }
    DirBuilder::new().recursive(true).mode(DIR_MODE).create(dir)
}

/// `fsync` a directory so a rename into it is durable. Opening a directory
/// read-only and `fsync`-ing the fd is the portable POSIX idiom.
fn fsync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// `<final>.<rand-hex>.tmp` — a randomised suffix so a same-uid attacker cannot
/// pre-create the temp path (combined with `O_EXCL`, the create then fails
/// closed rather than being redirected).
fn temp_name(final_name: &str) -> String {
    let mut rand = [0u8; 8];
    // A randomness failure is implausible on supported hosts; fall back to the
    // pid so we never block a write on it (O_EXCL still guards correctness).
    if getrandom::fill(&mut rand).is_err() {
        let pid = std::process::id().to_le_bytes();
        rand[..4].copy_from_slice(&pid);
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

    #[test]
    fn write_then_load_round_trips_through_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let root = Path::new("/some/workspace/root");
        let want = payload();

        write_snapshot(tmp.path(), root, &want).expect("write");
        let got = load_snapshot(tmp.path(), root).expect("load");

        // `SnapshotPayload: PartialEq` is byte-equality (the golden-fixture
        // property), so this proves the on-disk round-trip is lossless.
        assert_eq!(got, want);
    }

    #[test]
    fn write_is_owner_only_and_leaves_no_temp() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = Path::new("/ws");
        write_snapshot(tmp.path(), root, &payload()).expect("write");

        let file = tmp.path().join(snapshot_filename(root));
        let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, FILE_MODE, "snapshot must be owner-only 0600");

        // No leftover temp after a successful write.
        let temps = fs::read_dir(tmp.path())
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
    fn remove_snapshot_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = Path::new("/ws");
        write_snapshot(tmp.path(), root, &payload()).unwrap();
        remove_snapshot(tmp.path(), root).expect("first remove");
        // A second remove (already gone) is not an error.
        remove_snapshot(tmp.path(), root).expect("idempotent remove");
        assert!(!tmp.path().join(snapshot_filename(root)).exists());
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
