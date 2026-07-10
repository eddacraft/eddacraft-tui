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
//! # Crash-safety & read-safety (ADR-069 §4)
//!
//! The durable, symlink-safe **sealed I/O core** — temp create via
//! `O_CREAT | O_EXCL | O_NOFOLLOW` `0600`, `fsync` + atomic `renameat` publish +
//! parent-dir `fsync`; and the size-capped `openat2(RESOLVE_NO_SYMLINKS |
//! RESOLVE_BENEATH)` load with the `O_NOFOLLOW`-`openat` ladder fallback — is
//! extracted into the key-agnostic [`store`] submodule (ADR-105 §10). See its
//! module docs for the full hardening rationale; [`write_snapshot`] and
//! [`load_snapshot`] delegate their sealed core to `store::publish_sealed_at`
//! and [`store::load_sealed`] unchanged. This module keeps only the
//! **per-worktree** concerns layered on top: the [`snapshot_filename`] keying, the
//! `<hash>.root` companion, and the orphan sweep.
//!
//! # Privacy of the on-disk artifacts (PV-12, CIB-096)
//!
//! [`write_snapshot`] publishes two sibling files per worktree: the `<hash>.snap`
//! payload and a `<hash>.root` **companion** (CIB-096) that stores the worktree's
//! **absolute canonical root path in cleartext**, so the startup orphan sweep
//! ([`sweep_orphan_snapshots_on_start`]) can existence-check the root with no
//! session-registry keep-set (safe at cold boot). The `.root` is written `0600`
//! under the `0700`, owner-only graph-cache dir — same-uid, machine-local — which
//! is the existing PV-12 boundary: the snapshot **filename** is already an
//! unsalted, cross-machine-correlatable derivative of the root (the same exposure
//! class as a git blob hash).
//!
//! The companion's **read** matches the `.snap` discipline: it is opened anchored
//! beneath the validated graph-cache dirfd under `O_NOFOLLOW` /
//! `RESOLVE_NO_SYMLINKS` (a planted/swapped `.root` symlink is refused, not
//! followed) and the body is size-bounded by [`MAX_ROOT_BYTES`] (an over-cap
//! `.root` is rejected, not allocated) — so a tampered companion cannot redirect
//! the read or bomb the startup allocation. The existence check discriminates a
//! *proven* `NotFound` from any ambiguous stat error (EACCES / EIO / transient),
//! reclaiming only on the former.
//!
//! The companion stores the root *directly* (cleartext) rather than as a hash, so
//! an owner/PV sign-off on persisting the absolute root in cleartext may be wanted;
//! it does not cross the machine-local boundary.
#![cfg(unix)]

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anvil_graph_cache::snapshot::{
    MAX_SNAPSHOT_BYTES, SnapshotLoadError, SnapshotPayload, snapshot_filename,
};

/// The key-agnostic sealed-artifact I/O seam (ADR-105 §10): durable, symlink-safe
/// `write_sealed` / `load_sealed`, the dirfd + `openat2` helpers, and the size-cap
/// constant. Kept a named, `pub` submodule so the shared base-snapshot producer
/// (`anvil-cli` `graph_base_producer`) can reuse the exact hardening core the
/// per-worktree paths below layer their keying and `.root` companion onto.
pub mod store;

/// Extension for the `<hash>.root` companion (CIB-096): the cleartext canonical
/// root, sibling to the `.snap`, that the orphan sweep existence-checks.
const ROOT_EXT: &str = "root";
/// Extension for a published snapshot file (`<hash>.snap`).
const SNAP_EXT: &str = "snap";
/// Read cap for a `<hash>.root` companion (CIB-096 follow-up): the body is a
/// single absolute canonical path, bounded in practice by the OS `PATH_MAX`
/// (≈4 KiB on Linux); 64 KiB is generous head-room. The companion read is
/// anchored + `O_NOFOLLOW` (same discipline as the `.snap` load) and rejects an
/// over-cap body as `InvalidData`, so a planted/swapped multi-GB `.root` cannot
/// turn the startup sweep into an allocation bomb.
const MAX_ROOT_BYTES: u64 = 64 * 1024;

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
    store::ensure_dir(dir)?;

    let final_name = snapshot_filename(canonical_root);

    // ADR-069 §4 (CIB-097): open ONE real, fsync-able directory fd and reuse it
    // for BOTH the `.snap` sealed publish and the sibling `.root` companion, so a
    // single anchored create/rename/unlink discipline covers both. A REAL
    // `O_DIRECTORY` fd (NOT the read path's `O_PATH` one — `O_PATH` cannot be
    // `fsync`'d) serves three roles: the create anchor, the rename/unlink anchor,
    // and the post-rename directory-`fsync` target. `store::ensure_dir`'s
    // `validate_secure_dir` already rejects a symlinked / non-owned / group-writable
    // `dir`; opening with `O_NOFOLLOW` here is defence-in-depth.
    let dirfd = crate::path_safety::open_workspace_dir_for_fsync(dir)?;

    // Publish the `.snap` payload through the key-agnostic sealed core
    // (`create O_EXCL|O_NOFOLLOW 0600` → `write_all` → `fsync` → `renameat` →
    // parent-dir `fsync`). A pre-rename failure short-circuits here: the temp is
    // cleaned up inside `publish_sealed_at`, nothing is published, and the
    // companion below is not written.
    store::publish_sealed_at(&dirfd, &final_name, &payload.to_bytes())?;

    // CIB-096: publish the sibling `<hash>.root` companion (cleartext canonical
    // root) so the startup orphan sweep can existence-check the root without a
    // session-registry keep-set. The `.snap` is already published and IS the
    // source of truth, so a companion-write failure must NOT fail the write — a
    // missing/unreadable companion is treated as "keep" by the sweep (fail-safe),
    // so the snapshot simply cannot be auto-reclaimed until rewritten. Reuses the
    // SAME validated `dirfd` and the SAME sealed publish discipline.
    write_root_companion(&dirfd, &final_name, canonical_root);
    Ok(())
}

/// The `<hash>.root` companion name for a published `<hash>.snap` (CIB-096): the
/// same FNV hash stem, `.root` extension. `snapshot_name` is a separator-free
/// `snapshot_filename` basename ending in `.snap`.
#[must_use]
fn root_companion_name(snapshot_name: &str) -> String {
    let stem = snapshot_name
        .strip_suffix(&format!(".{SNAP_EXT}"))
        .unwrap_or(snapshot_name);
    format!("{stem}.{ROOT_EXT}")
}

/// Encode a canonical root path to the bytes stored in its `.root` companion.
/// Uses [`std::os::unix::ffi::OsStrExt::as_bytes`] — on Unix this is the exact,
/// lossless byte representation of the path (no UTF-8 round-trip risk for a
/// non-UTF-8 root), the same fidelity `snapshot_filename` hashes over.
fn encode_companion_root(canonical_root: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    canonical_root.as_os_str().as_bytes().to_vec()
}

/// Decode the bytes stored in a `.root` companion back to a path. The exact
/// inverse of [`encode_companion_root`] (`OsStrExt::from_bytes`); lossless on
/// Unix. An empty body decodes to an empty path, which the sweep treats as
/// unparseable (fail-safe keep).
fn decode_companion_root(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

/// Read + decode the `<hash>.root` companion `name` (a single, separator-free
/// leaf) **anchored beneath `dirfd`** (CIB-096 follow-up). Mirrors the disciplined
/// `.snap` load rather than a path-based `fs::read`:
/// - the leaf is opened via [`store::open_leaf_under_dirfd`] (anchored, `O_NOFOLLOW` /
///   `RESOLVE_NO_SYMLINKS`), so a planted/swapped `.root` **symlink** is refused
///   (`ELOOP`) and an intermediate-component swap cannot redirect the read;
/// - the read is bounded by `take(MAX_ROOT_BYTES + 1)` and an over-cap body is
///   rejected as `InvalidData`, so a multi-GB `.root` cannot be an allocation
///   bomb at daemon boot.
///
/// `Ok(root)` on a non-empty, in-bounds body; `Err` when the body is empty
/// (unparseable), over-cap, the leaf is a symlink, or any read fails — every
/// `Err` is a **fail-safe keep** at the call site.
fn read_companion_root(dirfd: &std::os::fd::OwnedFd, name: &str) -> io::Result<PathBuf> {
    let leaf = store::open_leaf_under_dirfd(dirfd, name)?;
    // `take(MAX + 1)` lets a genuinely over-cap body be detected (not silently
    // truncated into a valid-looking shorter path) and rejected.
    let cap = MAX_ROOT_BYTES + 1;
    let mut bytes = Vec::new();
    File::from(leaf).take(cap).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ROOT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "oversized .root companion",
        ));
    }
    let root = decode_companion_root(&bytes);
    if root.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "empty .root companion",
        ));
    }
    Ok(root)
}

/// Best-effort publish of the `<hash>.root` companion for an already-published
/// snapshot (CIB-096). Mirrors the snapshot publish: a randomised temp created
/// `O_CREAT | O_EXCL | O_NOFOLLOW` 0600 under the SAME `dirfd`, `write_all` +
/// `sync_all`, then `renameat` to the final companion name. A failure at any step
/// is logged at WARN and swallowed — the snapshot is the source of truth and is
/// already durable; the sweep keeps any snapshot whose companion is missing.
fn write_root_companion(
    dirfd: &std::os::fd::OwnedFd,
    snapshot_final_name: &str,
    canonical_root: &Path,
) {
    let companion_name = root_companion_name(snapshot_final_name);
    // The companion is just another sealed publish anchored at the SAME `dirfd` —
    // reuse the key-agnostic core (`create O_EXCL|O_NOFOLLOW 0600` → `write_all` →
    // `fsync` → `renameat` → parent-dir `fsync`, cleaning up the temp on a
    // pre-rename failure). On any failure, WARN and continue: the `.snap` is
    // published and IS the source of truth, and the sweep treats a missing
    // companion as "keep".
    if let Err(err) = store::publish_sealed_at(
        dirfd,
        &companion_name,
        &encode_companion_root(canonical_root),
    ) {
        tracing::warn!(
            target: "anvil_intercept::snapshot",
            error = %err,
            "snapshot .root companion write failed; snapshot published but not auto-reclaimable until rewritten (CIB-096)",
        );
    }
}

/// Load and validate the snapshot for `canonical_root` from `dir` (ADR-069
/// §1/§4). Delegates the stat + size-cap + anchored, symlink-safe read to the
/// key-agnostic [`store::load_sealed`], then runs the graph-cache integrity gate
/// over the returned bytes.
///
/// # Errors
/// [`SnapshotReadError::NotFound`] when there is no snapshot (normal cold start);
/// [`SnapshotReadError::Io`] on a disk error; [`SnapshotReadError::Rejected`]
/// when a non-regular file / oversize body / the integrity gate rejects the
/// bytes. Every case ⇒ cold rebuild.
pub fn load_snapshot(
    dir: &Path,
    canonical_root: &Path,
) -> Result<SnapshotPayload, SnapshotReadError> {
    let filename = snapshot_filename(canonical_root);

    // The sealed load caps at `MAX_SNAPSHOT_BYTES` and refuses a symlinked leaf /
    // symlinked-or-escaping dir component (ADR-069 §4 / CIB-092d). Map its
    // format-agnostic error taxonomy into this module's richer one: a
    // non-regular-file or oversize body is `Corrupt`/`Oversized` on the read side,
    // exactly as the previous inline checks reported.
    let bytes = match store::load_sealed(dir, &filename, MAX_SNAPSHOT_BYTES as u64) {
        Ok(bytes) => bytes,
        Err(store::LoadSealedError::NotFound) => return Err(SnapshotReadError::NotFound),
        Err(store::LoadSealedError::Io(err)) => return Err(SnapshotReadError::Io(err)),
        Err(store::LoadSealedError::NotRegularFile) => {
            return Err(SnapshotReadError::Rejected(SnapshotLoadError::Corrupt));
        }
        Err(store::LoadSealedError::Oversized) => {
            return Err(SnapshotReadError::Rejected(SnapshotLoadError::Oversized));
        }
    };

    SnapshotPayload::from_bytes(&bytes).map_err(SnapshotReadError::Rejected)
}

/// Remove the snapshot for `canonical_root` (ADR-069 §10: a key's snapshot is
/// dropped when its workspace is unregistered/evicted). A missing file is not an
/// error.
///
/// # Errors
/// A disk error other than the file already being absent.
pub fn remove_snapshot(dir: &Path, canonical_root: &Path) -> io::Result<()> {
    let snapshot_name = snapshot_filename(canonical_root);
    // Anchor the unlinks to a validated owner-only dirfd (CIB-102), so a same-uid
    // swap of a `dir` component cannot redirect the delete, and a symlink at a
    // leaf is unlinked as the symlink (never followed). A non-existent `dir` means
    // there is nothing to remove — idempotent success (mirrors the old NotFound
    // tolerance of the path-based `fs::remove_file`).
    let dirfd = match crate::path_safety::open_workspace_dir_for_fsync(dir) {
        Ok(fd) => fd,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    // Order matters (CIB-096 follow-up): remove the `.snap` FIRST, then best-effort
    // the `.root`. A crash *between* the two unlinks must not leave a `.snap` with
    // no companion — the startup sweep keeps any `.snap` whose companion is missing
    // (fail-safe), so that snapshot would linger forever, un-reclaimable. Dropping
    // the `.snap` first means a mid-crash leaves only a stray `.root`, which the
    // sweep already cleans up on the next boot.
    let snap_result = match store::unlink_at(&dirfd, &snapshot_name) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    };
    // Best-effort drop the `.root` companion alongside (CIB-096); a missing
    // companion is fine (older snapshot, or companion-write had failed).
    let companion_name = root_companion_name(&snapshot_name);
    if let Err(err) = store::unlink_at(&dirfd, &companion_name)
        && err.kind() != io::ErrorKind::NotFound
    {
        tracing::warn!(
            target: "anvil_intercept::snapshot",
            companion = %companion_name,
            error = %err,
            "failed to remove .root companion alongside snapshot (CIB-096)",
        );
    }
    snap_result
}

/// Sweep orphaned `*.tmp` files left by an interrupted write (ADR-069 §10),
/// returning the count removed. Best-effort: an unreadable dir, a file that
/// vanishes mid-sweep, or a dir the dirfd anchor **refuses as insecure**
/// (non-existent, symlinked, or not owner-only `0700`) all yield `0` and are
/// never fatal — the production graph-cache dir is always `0700`, so a refused
/// dir means a tampered environment in which there is nothing safe to sweep.
///
/// Unlinks are anchored to a validated owner-only dirfd (CIB-102) via `unlinkat`,
/// so a swapped `dir` component cannot redirect the delete and a symlinked `.tmp`
/// leaf is unlinked as the symlink, never followed. The `read_dir` enumeration is
/// path-based — the security-bearing operation is the per-leaf anchored unlink.
pub fn sweep_orphan_temps(dir: &Path) -> usize {
    let Ok(dirfd) = crate::path_safety::open_workspace_dir_for_fsync(dir) else {
        return 0;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(store::TMP_EXT) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if store::unlink_at(&dirfd, name).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Sweep orphaned `*.snap` snapshots on daemon start (ADR-069 §10, CIB-096):
/// reclaim any snapshot whose worktree was deleted while the daemon was down (so
/// its unregister hook never fired and its `.snap` lingers forever). Returns the
/// count of **snapshots** removed.
///
/// Unlike the prior keep-set sweep, this uses the per-snapshot `<hash>.root`
/// companion (written by [`write_snapshot`]) — so it needs **no session-registry
/// keep-set and is SAFE at cold boot**: it can never wipe a live (not-yet-attached)
/// snapshot, because the decision is "does this snapshot's stored root still exist
/// on disk", not "is this root in the (empty-at-boot) registry".
///
/// For each `<hash>.snap`:
/// - find its `<hash>.root` companion; if it is **absent, unreadable, or its body
///   does not parse to a path** → **KEEP** the snapshot (fail-safe: never delete a
///   snapshot we cannot *prove* is an orphan);
/// - else read the canonical root and `symlink_metadata` it. Only a **proven
///   `NotFound`** (the path is definitively gone) is a true orphan → remove BOTH
///   the `.snap` and its `.root`, counting one reclaimed snapshot. Any **other**
///   stat error (`EACCES` from a tightened parent, `EIO`/`ENOTCONN`/a transient
///   unmount at boot) is ambiguous → **KEEP** (fail-safe); and a successful stat
///   (root still exists) → keep.
///
/// A stray `<hash>.root` with **no matching `.snap`** (harmless leftover, e.g. a
/// companion published just before a crash that never wrote the snapshot) is also
/// cleaned up, but is **not** counted as a reclaimed snapshot.
///
/// Deletes are anchored to the validated, owner-only directory fd
/// ([`crate::path_safety::open_workspace_dir_for_fsync`]) via `unlinkat`, mirroring
/// the CIB-097 write-path discipline so an intermediate-component swap cannot
/// redirect an unlink; enumeration uses a path-based `read_dir` (anchoring readdir
/// is awkward and the per-leaf `unlinkat` carries the security-bearing anchor).
///
/// Best-effort: an unreadable/missing `dir` is a no-op; a file that vanishes
/// mid-sweep is skipped, never fatal. `*.tmp` files are left to
/// [`sweep_orphan_temps`].
pub fn sweep_orphan_snapshots_on_start(dir: &Path) -> usize {
    // Anchor unlinks to a validated owner-only dirfd (CIB-097 discipline). If the
    // dir is absent / unopenable / insecure, there is nothing safe to sweep.
    let Ok(dirfd) = crate::path_safety::open_workspace_dir_for_fsync(dir) else {
        return 0;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };

    // Collect names first so we can cross-reference `.snap` ↔ `.root` (a stray
    // `.root` with no `.snap` is cleaned up).
    let mut snaps: Vec<String> = Vec::new();
    let mut roots: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        match path.extension().and_then(|e| e.to_str()) {
            Some(SNAP_EXT) => snaps.push(name.to_owned()),
            Some(ROOT_EXT) => {
                roots.insert(name.to_owned());
            }
            _ => {}
        }
    }

    let mut removed = 0;
    for snap_name in &snaps {
        let companion_name = root_companion_name(snap_name);
        // Mark this companion as having a matching snapshot (so it is not later
        // treated as a stray `.root`).
        roots.remove(&companion_name);

        // Fail-safe: a missing / unreadable / over-cap / symlinked / unparseable
        // companion ⇒ KEEP. The read is anchored beneath the same validated
        // `dirfd` (O_NOFOLLOW + size-capped), so existence-check, read, and unlink
        // all share one anchor.
        let Ok(root) = read_companion_root(&dirfd, &companion_name) else {
            continue;
        };
        // Discriminate the existence check (CIB-096 follow-up): only a *proven*
        // `NotFound` reclaims. Any other outcome — the root still stats `Ok` (live),
        // or stat fails with anything other than `NotFound` (EACCES from a tightened
        // parent dir, EIO / ENOTCONN / a transient unmount at daemon boot) — is a
        // fail-safe KEEP, never a delete on uncertainty. (A non-`NotFound` error
        // misread as "root gone" would wipe a LIVE worktree's warm snapshot.) This
        // matches the `load_snapshot` / `remove_snapshot` NotFound discipline.
        let proven_gone = matches!(
            fs::symlink_metadata(&root),
            Err(e) if e.kind() == io::ErrorKind::NotFound
        );
        if !proven_gone {
            continue;
        }
        // True orphan: the stored root is gone. Remove BOTH the `.snap` and its
        // `.root`, anchored at the validated dirfd.
        if store::unlink_at(&dirfd, snap_name).is_ok() {
            removed += 1;
            // Best-effort drop the companion too; a failure here just leaves a
            // stray `.root` a later sweep will reclaim.
            let _ = store::unlink_at(&dirfd, &companion_name);
        }
    }

    // Clean up stray `.root` companions with no matching `.snap` (not counted as
    // reclaimed snapshots — they hold no graph state).
    for stray in &roots {
        let _ = store::unlink_at(&dirfd, stray);
    }

    removed
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
            span: None,
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
        assert_eq!(mode, store::FILE_MODE, "snapshot must be owner-only 0600");
        // CIB-096: the `<hash>.root` companion is created via the same sealed
        // `store::create_leaf_under_dirfd` path, so it is owner-only 0600 too.
        let companion = dir.join(root_companion_name(&snapshot_filename(root)));
        let companion_mode = fs::metadata(&companion).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            companion_mode,
            store::FILE_MODE,
            "the .root companion must be owner-only 0600"
        );
        // The created dir is owner-only 0700.
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            store::DIR_MODE
        );

        // No leftover temp after a successful write.
        let temps = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some(store::TMP_EXT))
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
        // Use a 0700 owner-only dir (the production graph-cache posture): the sweep
        // now anchors its unlinks via the validated dirfd (CIB-102), which refuses
        // a group/other-accessible dir.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        fs::write(dir.join("a.snap"), b"keep").unwrap();
        fs::write(dir.join("a.snap.deadbeef.tmp"), b"orphan").unwrap();
        fs::write(dir.join("b.snap.cafef00d.tmp"), b"orphan").unwrap();

        assert_eq!(sweep_orphan_temps(&dir), 2);
        assert!(dir.join("a.snap").exists(), "real snapshot kept");
        assert!(!dir.join("a.snap.deadbeef.tmp").exists());
    }

    #[test]
    fn sweep_orphan_temps_unlinks_a_symlinked_tmp_without_following_it() {
        // CIB-102: the temp sweep's unlink is anchored via `unlinkat` on the
        // validated dirfd, and `unlink` never follows the final symlink — so a
        // symlink planted at a `<...>.tmp` path is removed as the symlink itself;
        // the outside target it points at is left untouched.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let secret = tmp.path().join("outside-secret");
        fs::write(&secret, b"secret-bytes").unwrap();
        std::os::unix::fs::symlink(&secret, dir.join("planted.tmp")).unwrap();

        assert_eq!(sweep_orphan_temps(&dir), 1, "the symlinked .tmp is swept");
        assert!(
            !dir.join("planted.tmp").exists(),
            "the symlink entry is removed",
        );
        assert_eq!(
            fs::read(&secret).unwrap(),
            b"secret-bytes",
            "the symlink target is never followed / deleted",
        );
    }

    #[test]
    fn remove_snapshot_unlinks_a_symlinked_snap_without_following_it() {
        // CIB-102: `remove_snapshot` anchors its unlinks via the dirfd; a symlink
        // planted at the `.snap` path is removed as the symlink, never followed.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let root = Path::new("/ws-remove-symlink");
        let secret = tmp.path().join("outside-secret");
        fs::write(&secret, b"secret-bytes").unwrap();
        std::os::unix::fs::symlink(&secret, dir.join(snapshot_filename(root))).unwrap();

        remove_snapshot(&dir, root).expect("remove succeeds");
        assert!(
            !dir.join(snapshot_filename(root)).exists(),
            "the symlink entry is removed",
        );
        assert_eq!(
            fs::read(&secret).unwrap(),
            b"secret-bytes",
            "the symlink target is never followed / deleted",
        );
    }

    #[test]
    fn write_snapshot_produces_a_root_companion_holding_the_canonical_root() {
        // CIB-096: alongside the published `.snap`, the write must publish a sibling
        // `<hash>.root` companion holding the canonical root path, so the startup
        // sweep can existence-check the root without a registry keep-set.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        let root = Path::new("/some/workspace/root");
        write_snapshot(&dir, root, &payload()).expect("write");

        let companion_name = root_companion_name(&snapshot_filename(root));
        assert!(
            dir.join(&companion_name).exists(),
            "the .root companion is published"
        );
        let dirfd = crate::path_safety::open_workspace_dir_for_fsync(&dir).expect("open dirfd");
        let stored = read_companion_root(&dirfd, &companion_name).expect("companion parses");
        assert_eq!(stored, root, "the companion holds the canonical root");
    }

    #[test]
    fn sweep_orphan_removes_snap_and_root_when_root_gone() {
        // CIB-096: a worktree deleted while the daemon was down leaves a `.snap`
        // whose companion `.root` points at a path that no longer exists — a true
        // orphan. The sweep removes BOTH the `.snap` and its `.root`.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        // A root directory that existed at write time but is now gone.
        let gone = tmp.path().join("gone-worktree");
        fs::create_dir(&gone).unwrap();
        let canonical = fs::canonicalize(&gone).unwrap();
        write_snapshot(&dir, &canonical, &payload()).expect("write");
        fs::remove_dir(&canonical).unwrap();

        let snap = dir.join(snapshot_filename(&canonical));
        let companion = dir.join(root_companion_name(&snapshot_filename(&canonical)));
        assert!(snap.exists() && companion.exists());

        let removed = sweep_orphan_snapshots_on_start(&dir);
        assert_eq!(removed, 1, "the orphaned snapshot is reclaimed");
        assert!(!snap.exists(), "the orphan .snap is removed");
        assert!(!companion.exists(), "the orphan .root is removed too");
    }

    #[test]
    fn sweep_orphan_keeps_snap_when_root_exists() {
        // CIB-096: a snapshot whose companion root still exists on disk is a live
        // worktree (not yet reattached) — keep it.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let live = tmp.path().join("live-worktree");
        fs::create_dir(&live).unwrap();
        let canonical = fs::canonicalize(&live).unwrap();
        write_snapshot(&dir, &canonical, &payload()).expect("write");

        let removed = sweep_orphan_snapshots_on_start(&dir);
        assert_eq!(removed, 0, "a live root keeps its snapshot");
        assert!(dir.join(snapshot_filename(&canonical)).exists());
        assert!(
            dir.join(root_companion_name(&snapshot_filename(&canonical)))
                .exists()
        );
    }

    #[test]
    fn sweep_orphan_keeps_snap_with_missing_companion_fail_safe() {
        // CIB-096 fail-safe: a `.snap` whose `.root` companion is absent (e.g.
        // written by an older daemon, or the companion write failed) cannot be
        // proven an orphan — KEEP it, never delete on uncertainty.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let orphan = Path::new("/ws/no-companion");
        fs::write(dir.join(snapshot_filename(orphan)), b"snap-no-root").unwrap();

        let removed = sweep_orphan_snapshots_on_start(&dir);
        assert_eq!(removed, 0, "a .snap with no .root is kept (fail-safe)");
        assert!(dir.join(snapshot_filename(orphan)).exists());
    }

    #[test]
    fn sweep_orphan_cleans_a_stray_root_with_no_snap() {
        // CIB-096: a stray `.root` with no matching `.snap` is harmless leftover —
        // clean it up. It is not counted as a reclaimed *snapshot*.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let stray = root_companion_name(&snapshot_filename(Path::new("/ws/orphan-root")));
        fs::write(dir.join(&stray), b"/ws/orphan-root").unwrap();

        let removed = sweep_orphan_snapshots_on_start(&dir);
        assert_eq!(removed, 0, "a stray .root is not a reclaimed snapshot");
        assert!(!dir.join(&stray).exists(), "the stray .root is cleaned up");
    }

    #[test]
    fn sweep_orphan_keeps_snap_with_unreadable_companion_fail_safe() {
        // CIB-096 fail-safe: a `.root` whose contents do not parse to a path is
        // treated as "cannot prove orphan" — KEEP the snapshot. (An empty
        // companion parses to an empty path, which `exists()` reports false for;
        // guard against deleting on that by requiring a non-empty parse.)
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let root = Path::new("/ws/empty-companion");
        fs::write(dir.join(snapshot_filename(root)), b"snap").unwrap();
        // An empty companion file — no path bytes to existence-check.
        fs::write(dir.join(root_companion_name(&snapshot_filename(root))), b"").unwrap();

        let removed = sweep_orphan_snapshots_on_start(&dir);
        assert_eq!(removed, 0, "an empty/unparseable .root keeps the snapshot");
        assert!(dir.join(snapshot_filename(root)).exists());
    }

    #[test]
    fn sweep_orphan_keeps_snap_when_root_stat_is_permission_denied_not_not_found() {
        // CIB-096 follow-up (HIGH, data-loss): the existence check must discriminate
        // `NotFound` from other stat errors. If the worktree's PARENT dir has had its
        // execute/search bit removed (tightened perms, or a transient condition at
        // daemon boot), `symlink_metadata(root)` returns PermissionDenied — NOT
        // NotFound. The root is still LIVE; the snapshot must be KEPT, never deleted
        // on an ambiguous stat error.
        use std::os::unix::fs::PermissionsExt;
        // root bypasses DAC, so a 0o600 parent still stats Ok — the EACCES scenario
        // is unconstructable as root (common in CI containers). Skip there.
        if nix::unistd::geteuid().is_root() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();

        // A real, existing worktree nested under a parent dir we will lock down.
        let parent = tmp.path().join("locked-parent");
        fs::create_dir(&parent).unwrap();
        let live = parent.join("worktree");
        fs::create_dir(&live).unwrap();
        let canonical = fs::canonicalize(&live).unwrap();
        write_snapshot(&dir, &canonical, &payload()).expect("write");

        // Strip the execute/search bit on the parent so stat-ing the child path
        // fails with EACCES (PermissionDenied), not ENOENT — the child still exists.
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            fs::symlink_metadata(&canonical).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied,
            "precondition: the stat is EACCES, not NotFound",
        );

        let removed = sweep_orphan_snapshots_on_start(&dir);

        // Restore perms first so the tempdir cleans up regardless of assertions.
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(
            removed, 0,
            "a PermissionDenied stat must NOT be misread as 'root gone' (fail-safe keep)",
        );
        assert!(
            dir.join(snapshot_filename(&canonical)).exists(),
            "the live worktree's snapshot must be kept",
        );
    }

    #[test]
    fn sweep_orphan_keeps_snap_with_oversized_companion_fail_safe() {
        // CIB-096 follow-up (MEDIUM, startup DoS): an over-cap `.root` body is
        // rejected as InvalidData by the size-bounded companion read, so it cannot
        // be an allocation bomb — and the snapshot is KEPT (fail-safe: an unreadable
        // companion can't prove an orphan).
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let root = Path::new("/ws/oversized-companion");
        fs::write(dir.join(snapshot_filename(root)), b"snap").unwrap();
        // A companion larger than MAX_ROOT_BYTES.
        let big = vec![b'/'; usize::try_from(MAX_ROOT_BYTES + 1).unwrap()];
        fs::write(
            dir.join(root_companion_name(&snapshot_filename(root))),
            &big,
        )
        .unwrap();

        let removed = sweep_orphan_snapshots_on_start(&dir);
        assert_eq!(
            removed, 0,
            "an over-cap .root keeps the snapshot (fail-safe)"
        );
        assert!(
            dir.join(snapshot_filename(root)).exists(),
            "the snapshot is kept when its companion is over-cap",
        );
    }

    #[test]
    fn sweep_orphan_keeps_snap_with_symlinked_companion_not_followed() {
        // CIB-096 follow-up (MEDIUM): a `.root` that is a SYMLINK must not be
        // followed by the companion read (anchored O_NOFOLLOW / RESOLVE_NO_SYMLINKS),
        // even if the symlink target would decode to a gone path. The read fails →
        // fail-safe KEEP, and the symlink target is never read through.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let root = Path::new("/ws/symlinked-companion");
        fs::write(dir.join(snapshot_filename(root)), b"snap").unwrap();
        // A target holding a path that does NOT exist (would be a "reclaim" if read).
        let target = tmp.path().join("companion-target");
        fs::write(&target, b"/definitely/does/not/exist/anywhere").unwrap();
        std::os::unix::fs::symlink(
            &target,
            dir.join(root_companion_name(&snapshot_filename(root))),
        )
        .unwrap();

        let removed = sweep_orphan_snapshots_on_start(&dir);
        assert_eq!(
            removed, 0,
            "a symlinked .root must not be followed; the snapshot is kept",
        );
        assert!(
            dir.join(snapshot_filename(root)).exists(),
            "the snapshot is kept when its companion is a symlink",
        );
    }

    #[test]
    fn read_companion_root_rejects_an_oversized_body() {
        // The size cap is enforced at the read, not just a pre-stat: a body over
        // MAX_ROOT_BYTES is rejected as InvalidData (no allocation bomb, no silent
        // truncation into a valid-looking shorter path).
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let name = "oversize.root";
        fs::write(
            dir.join(name),
            vec![b'/'; usize::try_from(MAX_ROOT_BYTES + 1).unwrap()],
        )
        .unwrap();
        let dirfd = crate::path_safety::open_workspace_dir_for_fsync(&dir).expect("open dirfd");
        let err = read_companion_root(&dirfd, name).expect_err("over-cap body is rejected");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_companion_root_accepts_an_at_cap_body() {
        // A body exactly at the cap (MAX_ROOT_BYTES) is accepted; only > cap is
        // rejected. Use an absolute path padded to exactly the cap length.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let name = "atcap.root";
        let body = vec![b'/'; usize::try_from(MAX_ROOT_BYTES).unwrap()];
        fs::write(dir.join(name), &body).unwrap();
        let dirfd = crate::path_safety::open_workspace_dir_for_fsync(&dir).expect("open dirfd");
        let root = read_companion_root(&dirfd, name).expect("an at-cap body is accepted");
        assert_eq!(root.as_os_str().len() as u64, MAX_ROOT_BYTES);
    }

    #[test]
    fn sweep_orphan_missing_dir_is_a_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("does-not-exist");
        assert_eq!(sweep_orphan_snapshots_on_start(&missing), 0);
    }

    #[test]
    fn root_companion_name_swaps_the_snap_extension_for_root() {
        let snap = snapshot_filename(Path::new("/ws/x"));
        let companion = root_companion_name(&snap);
        assert_eq!(
            Path::new(&snap).extension().and_then(|e| e.to_str()),
            Some(SNAP_EXT),
        );
        assert_eq!(
            Path::new(&companion).extension().and_then(|e| e.to_str()),
            Some(ROOT_EXT),
        );
        assert_eq!(
            Path::new(&companion).file_stem().and_then(|s| s.to_str()),
            Path::new(&snap).file_stem().and_then(|s| s.to_str()),
            "the companion shares the snapshot's hash stem",
        );
    }

    #[test]
    fn write_replaces_a_symlink_at_the_final_path_without_writing_through_it() {
        // CIB-097: WRITE-side counterpart to `load_refuses_a_symlink_at_the_snapshot_path`.
        // The publish does NOT *refuse* a symlink at the final path — `renameat`
        // atomically replaces the symlink dentry with the renamed regular file
        // (rename is not symlink-following). The security property is that the
        // symlink's TARGET is never written through, and the published snapshot is
        // a real, loadable file with the bytes we wrote.
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        store::ensure_dir(&dir).unwrap();
        let root = Path::new("/ws-symlink-final");
        let secret = tmp.path().join("outside-secret");
        fs::write(&secret, b"secret-bytes").unwrap();
        // Plant a symlink at the final snapshot path pointing outside the dir.
        std::os::unix::fs::symlink(&secret, dir.join(snapshot_filename(root))).unwrap();

        write_snapshot(&dir, root, &payload()).expect("write publishes over the planted symlink");

        // The outside target was NOT written through (the publish did not follow
        // the symlink).
        assert_eq!(
            fs::read(&secret).unwrap(),
            b"secret-bytes",
            "the publish must not write through a symlink at the final path",
        );
        // The published path is now a real, owner-only, loadable snapshot — not a symlink.
        let meta = fs::symlink_metadata(dir.join(snapshot_filename(root))).unwrap();
        assert!(
            !meta.file_type().is_symlink(),
            "the published path is a regular file"
        );
        let loaded = load_snapshot(&dir, root).expect("the published snapshot loads");
        assert_eq!(
            loaded.to_bytes(),
            payload().to_bytes(),
            "the published snapshot must hold exactly the bytes we wrote, not the \
             symlink target's content or a truncated file",
        );
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
    fn remove_snapshot_removes_both_snap_and_root_companion() {
        // CIB-096 follow-up: `remove_snapshot` drops the `.snap` AND its `.root`
        // companion. Order is `.snap` first, then best-effort `.root`, so a crash
        // between the two leaves only a stray `.root` (which the sweep cleans up),
        // never a `.snap` with no companion (which the sweep would keep forever).
        let tmp = tempfile::tempdir().unwrap();
        let dir = gc(&tmp);
        let root = Path::new("/ws-remove-order");
        write_snapshot(&dir, root, &payload()).unwrap();
        let snap = dir.join(snapshot_filename(root));
        let companion = dir.join(root_companion_name(&snapshot_filename(root)));
        assert!(snap.exists() && companion.exists(), "both published");

        remove_snapshot(&dir, root).expect("remove");
        assert!(!snap.exists(), "the .snap is removed");
        assert!(!companion.exists(), "the .root companion is removed too");
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
