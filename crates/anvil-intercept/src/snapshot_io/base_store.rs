//! GBASE-002 (ADR-105 §2/§5/§9): the content-addressed, write-once **shared
//! base** store and its single-flight production claim.
//!
//! Where the parent [`super`] module keys per-worktree snapshots by
//! [`super::snapshot_filename`], this submodule keys a **shared base** artefact by
//! its **merge-base sha** and layers three things onto the key-agnostic
//! [`super::store`] seam — never duplicating its disk I/O:
//!
//! - **Write-once publish.** [`publish_base`] writes the base payload at a
//!   sha-derived leaf `<sha>.base` under a `base/` directory beneath the
//!   graph-cache dir. If the artefact for that sha already exists and gates clean,
//!   a second produce is a **no-op success** ([`PublishOutcome::AlreadyPresent`]);
//!   the content is addressed by sha, so re-publishing identical bytes is never
//!   required.
//! - **Single-flight claim.** [`claim`] creates `.producing/<sha>.lock` via
//!   `O_EXCL` through the seam, stamped `{pid, start_time, nonce}` — the
//!   save-time-driver `{pid, start_time}` convention with a **PID-reuse guard**
//!   (a pid match alone is *not* liveness; the recorded `start_time` must match
//!   too, per [`crate::save_time_driver`]). The claim is held for the duration of
//!   production and released on success or failure.
//! - **Stale-claim reclaim, in the claim path** (no separate reaper). On an
//!   `O_EXCL` collision, the existing lock is read and reclaimed **iff** the
//!   stamped process is not live (dead pid, *or* alive-pid-with-mismatched-start
//!   = PID reuse) **or** the lock mtime exceeds [`STALE_CLAIM_MAX_AGE`].
//!
//! # Destruction invariant — the per-dir advisory guard
//!
//! **Any destruction of a claim record — reclaim's `unlink` of a stale lock, and
//! release's `unlink` of our own — happens ONLY while holding an exclusive
//! `flock` on the `.producing/.guard` file, and only after re-verifying the
//! lock's identity through an fd opened via the dirfd (never a path re-read),
//! immediately before the destructive syscall.** This closes two TOCTOU races a
//! bare "classify then `renameat`/`unlink`" had (an atomic rename is atomic per
//! *name*, not pinned to the inode examined):
//!
//! - **reclaim vs reclaim / vs a fresh claim:** without the guard, reclaimer A
//!   could rename a stale lock aside and `O_EXCL`-create a fresh legitimate
//!   claim, then reclaimer B's delayed rename consumes A's brand-new lock — two
//!   winners. Under the guard, only one reclaimer runs the classify→unlink
//!   section at a time, and it re-reads the inode under the guard before
//!   unlinking, so it never destroys a lock it did not classify as reclaimable.
//! - **release vs reclaim:** an owner whose claim was reclaimed (it stalled past
//!   the mtime bound) must not delete the peer's replacement lock. Release takes
//!   the same guard and re-verifies the on-disk nonce is still ours before
//!   unlinking.
//!
//! The guard file is created once and **never removed**, so it is a stable
//! rendezvous. The **hot path stays lock-free**: an `O_EXCL` create of a fresh
//! lock into an empty slot is an atomic single-winner and never takes the guard;
//! a lock-free creator can only ever *create* into an empty slot, never swap an
//! existing inode, so it can never make a guarded reclaimer destroy the wrong
//! lock.
//!
//! # Schema-epoch clause (load side, ADR-105 §9)
//!
//! [`load_base`] **refuses rather than returns** a mismatched payload: a base
//! whose magic, `format_version`, or `backing_schema_version` does not match this
//! build decodes to [`BaseLoadOutcome::Ignored`] (a cold-path, non-fatal ignore),
//! never a blocking error and never a partially-read graph. The artefact is left
//! in place — reclaiming an epoch-stale base at zero references is GBASE-008's GC
//! job, **not** implemented here. Because [`super::store`] and the base gate are
//! keyed on the `ANVILGB1` magic, a mixed-class (hence, via the shared version
//! policy, mixed-epoch) composition is impossible by construction.
//!
//! # Failure posture (ADR-105 §6)
//!
//! Every operation here is **non-fatal** to the caller. A claim collision by a
//! live producer is [`ClaimOutcome::Contended`] (serve cold / try later), an
//! unreadable/absent base is `Ignored`/`Absent` (cold scan serves), and a disk
//! error surfaces as `io::Error` for the caller to log-and-degrade — never a
//! panic, never a hard stop.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anvil_graph_cache::snapshot::{MAX_SNAPSHOT_BYTES, SnapshotPayload};

use super::store;

/// Subdirectory under the graph-cache dir that holds shared base artefacts.
const BASE_SUBDIR: &str = "base";
/// Extension for a published base artefact (`<sha>.base`).
const BASE_EXT: &str = "base";
/// Subdirectory (under the base dir) that holds in-flight production claims.
const PRODUCING_SUBDIR: &str = ".producing";
/// Extension for a single-flight production claim lock (`<sha>.lock`).
const LOCK_EXT: &str = "lock";
/// The per-`.producing`-dir advisory guard file. Created once (`0600`), **never
/// removed**, and `flock(LOCK_EX)`-held across every destructive claim-record
/// operation (reclaim and release) so classify→destroy is atomic w.r.t. the
/// inode examined. A separate file from any `<sha>.lock`, so guarding it never
/// blocks the lock-free hot-path `O_EXCL` create.
const GUARD_NAME: &str = ".guard";
/// Read cap for a claim lock body: `{pid}\n{start_time}\n{nonce}\n` is a few
/// dozen bytes; 256 is generous head-room and bounds a planted/oversized lock.
const MAX_LOCK_BYTES: u64 = 256;

/// Upper bound on how long a `.producing/<sha>.lock` may sit before a peer is
/// allowed to reclaim it on the **mtime fallback alone** (ADR-105 §5).
///
/// ADR-105 §5 specifies the reclaim bound as **2× the p95 base-production time**.
/// That p95 is not yet measured — its calibration is an explicit deliverable of
/// the GBASE-010 graduation gate — so this is a deliberately **conservative**
/// placeholder: large enough that a genuinely-still-producing subprocess on a big
/// monorepo is never reclaimed out from under itself, at the cost of a slower
/// recovery in the rare case where the *precise* liveness check (dead pid /
/// PID-reuse via `start_time`) is unavailable because the platform could not read
/// a start time. The precise liveness path ([`ClaimProcs::is_live`]) handles the
/// common dead-producer case immediately; this mtime bound only backstops the
/// ambiguous "present, unreadable, or start-time-less" case. GBASE-010 replaces
/// this constant with the measured `2 × p95`.
const STALE_CLAIM_MAX_AGE: Duration = Duration::from_mins(10);

/// Liveness seam for the claim's PID-reuse guard, so reclaim tests never depend
/// on real process state. Mirrors the discriminator discipline of
/// [`crate::save_time_driver::ProcessControl`] but carries no `terminate` — a
/// claim is never signalled, only reclaimed.
pub trait ClaimProcs: Send + Sync {
    /// This process's pid — stamped into a freshly-acquired claim.
    fn current_pid(&self) -> u32;
    /// This process's PID-reuse discriminator (Linux `/proc` starttime, etc.), if
    /// readable — stamped alongside the pid so a later reclaimer can tell a
    /// recycled pid from a live producer.
    fn current_start_time(&self) -> Option<u64>;
    /// Whether the process stamped as `(pid, recorded_start_time)` is still that
    /// same live process. A pid match **alone** is not liveness: where both the
    /// recorded and current start times are readable they must match, so a
    /// recycled pid is never mistaken for a live producer. A missing discriminator
    /// falls back to bare pid liveness (best-effort, as the driver does).
    fn is_live(&self, pid: u32, recorded_start_time: Option<u64>) -> bool;
}

/// Production [`ClaimProcs`] over the crate's existing PID helpers.
pub struct SystemClaimProcs;

impl ClaimProcs for SystemClaimProcs {
    fn current_pid(&self) -> u32 {
        std::process::id()
    }

    fn current_start_time(&self) -> Option<u64> {
        crate::process_start_time(std::process::id())
    }

    fn is_live(&self, pid: u32, recorded_start_time: Option<u64>) -> bool {
        if !crate::process_exists(pid) {
            return false;
        }
        match (recorded_start_time, crate::process_start_time(pid)) {
            // Both discriminators readable: a mismatch is a **recycled pid** (PID
            // reuse) — not the producer we stamped, so not live.
            (Some(recorded), Some(current)) => recorded == current,
            // A missing discriminator on either side: fall back to bare pid
            // liveness, the best evidence available (same posture as the driver).
            _ => true,
        }
    }
}

/// The persistent shared-base directory, `<graph-cache>/base` (ADR-105 §2/§10).
/// `None` when no graph-cache dir resolves (persistence then stays off), mirroring
/// [`super::graph_cache_dir`].
#[must_use]
pub fn default_base_dir() -> Option<PathBuf> {
    super::graph_cache_dir().map(|dir| dir.join(BASE_SUBDIR))
}

/// The leaf name for a base artefact keyed by `sha` (`<sha>.base`). The sha is a
/// separator-free hex object name, so it passes the seam's `validate_leaf_name`.
fn base_leaf(sha: &str) -> String {
    format!("{sha}.{BASE_EXT}")
}

/// The leaf name for a production claim keyed by `sha` (`<sha>.lock`).
fn lock_leaf(sha: &str) -> String {
    format!("{sha}.{LOCK_EXT}")
}

/// The `.producing/` claim directory beneath `base_dir`.
fn producing_dir(base_dir: &Path) -> PathBuf {
    base_dir.join(PRODUCING_SUBDIR)
}

/// Outcome of a write-once [`publish_base`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    /// The base artefact was freshly written for this sha.
    Written,
    /// A clean base artefact for this sha already existed — a no-op success
    /// (write-once semantics; the content is addressed by sha).
    AlreadyPresent,
}

/// Outcome of loading a base for a sha (ADR-105 §9). The load API **refuses**
/// rather than returns a mismatched payload, so a `Loaded` value is always a
/// current-epoch, gate-clean base.
#[derive(Debug)]
pub enum BaseLoadOutcome {
    /// A current-epoch base decoded and passed the full integrity gate.
    Loaded(SnapshotPayload),
    /// No base artefact exists for this sha (the normal cold/first-run case).
    Absent,
    /// A base artefact exists but does not match this build — wrong magic
    /// (including an `ANVILGC1` per-worktree snapshot), a `format_version` /
    /// `backing_schema_version` mismatch (schema-epoch drift), a failed
    /// integrity check, a non-regular file, or an oversize body. **Ignored on the
    /// cold path**, never a blocking error; the artefact is left in place for
    /// GBASE-008 GC to reclaim once unreferenced.
    Ignored,
}

/// Load and gate the base artefact for `sha` from `base_dir` (ADR-105 §9).
///
/// Rides the key-agnostic [`store::load_sealed`] (stat + size-cap + anchored,
/// symlink-safe read) and then the `ANVILGB1` base gate
/// ([`SnapshotPayload::from_base_bytes`]). Any anomaly — absent, unreadable,
/// wrong class/epoch, corrupt — maps to [`BaseLoadOutcome`] without ever
/// returning a mismatched payload or a hard error.
#[must_use]
pub fn load_base(base_dir: &Path, sha: &str) -> BaseLoadOutcome {
    let leaf = base_leaf(sha);
    let bytes = match store::load_sealed(base_dir, &leaf, MAX_SNAPSHOT_BYTES as u64) {
        Ok(bytes) => bytes,
        Err(store::LoadSealedError::NotFound) => return BaseLoadOutcome::Absent,
        // A present-but-unusable artefact (disk error, non-regular file, oversize)
        // is a cold-path ignore, never a block (ADR-105 §6).
        Err(_) => return BaseLoadOutcome::Ignored,
    };
    match SnapshotPayload::from_base_bytes(&bytes) {
        Ok(payload) => BaseLoadOutcome::Loaded(payload),
        // Wrong magic / epoch / checksum / count / corrupt → discard-and-rebuild
        // posture: ignore, leave in place (ADR-105 §9).
        Err(_) => BaseLoadOutcome::Ignored,
    }
}

/// Publish `base_bytes` for `sha` **write-once** (ADR-105 §2).
///
/// If a clean, current-epoch base already exists for `sha`, this is a no-op
/// success ([`PublishOutcome::AlreadyPresent`]); otherwise the bytes are written
/// through the seam's durable, symlink-safe sealed publish
/// ([`store::write_sealed`]) and the outcome is [`PublishOutcome::Written`]. The
/// caller is expected to hold the [`claim`] for `sha` across this call, but the
/// write-once presence check makes a redundant concurrent publish of identical,
/// content-addressed bytes harmless regardless.
///
/// # `AlreadyPresent` vs a present-but-`Ignored` artefact (ADR-105 §9)
///
/// Only a **currently loadable** base short-circuits to `AlreadyPresent`. A
/// present-but-[`BaseLoadOutcome::Ignored`] artefact (corrupt, or a stale-epoch /
/// wrong-magic base) does **not** — this fresh produce writes the correct
/// current-epoch bytes over it (atomic `renameat`, so a reader never sees a torn
/// file). ADR-105 §9's "left in place" governs the **read path**: a *loader* that
/// meets an epoch/magic mismatch ignores it and never returns a mismatched
/// payload (that is [`load_base`]'s contract). Refreshing a corrupt/stale-epoch
/// artefact at the same content-addressed sha is the **produce path's**
/// prerogative — the sha still names this exact content, the write is atomic, and
/// readers are fail-closed — so healing it here is correct, not a §9 violation.
///
/// # Errors
/// Any `io::Error` from the sealed publish (create / write / fsync / rename).
pub fn publish_base(base_dir: &Path, sha: &str, base_bytes: &[u8]) -> io::Result<PublishOutcome> {
    if matches!(load_base(base_dir, sha), BaseLoadOutcome::Loaded(_)) {
        return Ok(PublishOutcome::AlreadyPresent);
    }
    store::write_sealed(base_dir, &base_leaf(sha), base_bytes)?;
    Ok(PublishOutcome::Written)
}

/// Outcome of a single-flight [`claim`].
#[derive(Debug)]
pub enum ClaimOutcome {
    /// This caller holds the production claim for the sha. Held until the returned
    /// guard is [`BaseClaim::release`]d or dropped.
    Acquired(BaseClaim),
    /// Another **live** producer holds the claim — serve cold / retry later
    /// (ADR-105 §6). Nothing was written.
    Contended,
}

/// A held single-flight production claim (`.producing/<sha>.lock`). Releasing —
/// explicitly via [`Self::release`] or implicitly on `Drop` — removes the lock
/// **under the per-dir guard, and only if it still carries this claim's nonce**
/// (re-verified through a dirfd-anchored open, never a path re-read): if a peer
/// reclaimed the lock (because this producer stalled past [`STALE_CLAIM_MAX_AGE`]
/// or its start-time became unreadable), the lock now holds the reclaimer's stamp
/// and must not be removed by us. See the module-level destruction invariant.
#[derive(Debug)]
pub struct BaseClaim {
    producing_dir: PathBuf,
    lock_name: String,
    nonce: String,
    released: bool,
}

impl BaseClaim {
    /// Release the claim: remove `.producing/<sha>.lock` iff it still carries this
    /// claim's nonce (see the type docs). Idempotent; never panics.
    pub fn release(mut self) {
        self.release_inner();
    }

    fn release_inner(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        // Destruction of a claim record happens ONLY under the per-dir guard,
        // after re-verifying ownership through an open fd (never a path re-read).
        // Open a real dirfd on the producing dir; a vanished dir means there is
        // nothing to release (best-effort, never panics in `Drop`).
        let Ok(dirfd) = crate::path_safety::open_workspace_dir_for_fsync(&self.producing_dir)
        else {
            return;
        };
        // Hold the guard across the read-nonce → unlink critical section, so a
        // peer reclaim cannot interleave between our ownership check and the
        // unlink (the release-vs-reclaim TOCTOU). If the guard cannot be taken we
        // simply skip removal — leaving a lock in place is safe (it ages out or
        // is reclaimed), removing a peer's lock is not.
        let Ok(_guard) = lock_guard(&dirfd) else {
            return;
        };
        // Re-verify ownership through an fd opened via the dirfd, under the guard:
        // only remove the lock if it STILL carries this claim's nonce. A reclaimer
        // that stole a stalled claim replaced the lock with its own stamp; removing
        // that would drop a live peer's claim.
        if let Some(record) = read_lock_record_at(&dirfd, &self.lock_name)
            && record.nonce == self.nonce
        {
            let _ = store::unlink_at(&dirfd, &self.lock_name);
        }
    }
}

impl Drop for BaseClaim {
    fn drop(&mut self) {
        // Backstop for a panic between acquire and an explicit `release()`.
        self.release_inner();
    }
}

/// Stamped identity of a claim lock: `{pid, start_time, nonce}`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LockRecord {
    pid: u32,
    start_time: Option<u64>,
    nonce: String,
}

/// Serialise a [`LockRecord`] to the on-disk lock body:
/// `{pid}\n{start_time-or-empty}\n{nonce}\n`.
fn encode_lock(record: &LockRecord) -> Vec<u8> {
    let start = record.start_time.map_or(String::new(), |s| s.to_string());
    format!("{}\n{}\n{}\n", record.pid, start, record.nonce).into_bytes()
}

/// Parse a lock body. A malformed body yields `None` (treated as reclaimable
/// garbage by the reclaim classifier, gated by the mtime bound).
fn parse_lock(bytes: &[u8]) -> Option<LockRecord> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    let pid: u32 = lines.next()?.trim().parse().ok()?;
    let start_line = lines.next()?.trim();
    let start_time = if start_line.is_empty() {
        None
    } else {
        Some(start_line.parse().ok()?)
    };
    let nonce = lines.next()?.trim().to_owned();
    if nonce.is_empty() {
        return None;
    }
    Some(LockRecord {
        pid,
        start_time,
        nonce,
    })
}

/// Read a lock leaf's raw bytes + mtime **anchored beneath `dirfd`** (the seam's
/// `O_NOFOLLOW` / `RESOLVE_NO_SYMLINKS` open), never a path re-read — so a
/// swapped intermediate component or a planted symlink cannot redirect the read
/// in the classify→destroy or release→unlink windows.
///
/// # Errors
/// [`io::ErrorKind::NotFound`] when the leaf is gone; any other `io::Error` on a
/// disk/open failure.
fn read_lock_bytes_at(
    dirfd: &std::os::fd::OwnedFd,
    lock_name: &str,
) -> io::Result<(Vec<u8>, Option<SystemTime>)> {
    let fd = store::open_leaf_under_dirfd(dirfd, lock_name)?;
    let file = File::from(fd);
    let mtime = file.metadata().ok().and_then(|m| m.modified().ok());
    let mut bytes = Vec::new();
    file.take(MAX_LOCK_BYTES + 1).read_to_end(&mut bytes)?;
    Ok((bytes, mtime))
}

/// Read + parse a lock record anchored beneath `dirfd` (the ownership check used
/// under the guard by reclaim and release). A missing / unreadable / malformed
/// lock yields `None`.
fn read_lock_record_at(dirfd: &std::os::fd::OwnedFd, lock_name: &str) -> Option<LockRecord> {
    let (bytes, _mtime) = read_lock_bytes_at(dirfd, lock_name).ok()?;
    parse_lock(&bytes)
}

/// Open (creating once, `0600`, never removed) the per-`.producing`-dir advisory
/// guard file `.guard`, anchored beneath `dirfd` under `O_NOFOLLOW`. The returned
/// [`File`] owns the fd whose open-file-description the `flock` binds to.
fn open_guard(dirfd: &std::os::fd::OwnedFd) -> io::Result<File> {
    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::Mode;
    use std::os::fd::AsFd;
    // `O_CREAT` (not `O_EXCL`): the guard is created once and thereafter reused.
    // `O_NOFOLLOW` refuses a planted `.guard` symlink; the single, fixed
    // component cannot traverse. Mode `0600` from the first syscall.
    let flags = OFlag::O_RDWR | OFlag::O_CREAT | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let mode = Mode::from_bits_truncate(store::FILE_MODE as nix::libc::mode_t);
    let fd = openat(dirfd.as_fd(), GUARD_NAME, flags, mode).map_err(io::Error::from)?;
    Ok(File::from(fd))
}

/// Take the exclusive advisory guard (`flock(LOCK_EX)`) on the `.producing`
/// dir's `.guard` file, returned as an RAII [`nix::fcntl::Flock`] that releases
/// on drop. `flock` locks are per open-file-description, so independent opens —
/// across threads *and* processes — mutually exclude, which is exactly what
/// serialises the rare destructive sections (reclaim + release). The hot path
/// (`O_EXCL` create of a fresh lock) never takes this guard.
///
/// # Errors
/// Any `io::Error` from opening `.guard` or from the blocking `flock`.
fn lock_guard(dirfd: &std::os::fd::OwnedFd) -> io::Result<nix::fcntl::Flock<File>> {
    let file = open_guard(dirfd)?;
    nix::fcntl::Flock::lock(file, nix::fcntl::FlockArg::LockExclusive)
        .map_err(|(_file, errno)| io::Error::from(errno))
}

/// A random 64-bit nonce rendered as 16 lowercase hex chars, disambiguating one
/// claim attempt from another (including two threads of the *same* pid in tests)
/// so release/reclaim can prove lock ownership.
fn fresh_nonce() -> String {
    let mut raw = [0u8; 8];
    if getrandom::fill(&mut raw).is_err() {
        // A randomness failure is implausible on supported hosts; fall back to the
        // pid (widened to 8 bytes) so we never block a claim — the same fallback
        // the seam's `temp_name` uses. Ownership proof degrades to best-effort,
        // which only matters for the rare stalled-reclaim case.
        raw = u64::from(std::process::id()).to_le_bytes();
    }
    let mut hex = String::with_capacity(16);
    for b in raw {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Classification of an existing claim lock on an `O_EXCL` collision.
enum Existing {
    /// The lock vanished between the failed create and this read — retry.
    Vanished,
    /// A live producer holds it — [`ClaimOutcome::Contended`].
    Live,
    /// Dead / PID-reused / mtime-exceeded / stale garbage — reclaimable.
    Reclaimable,
    /// Present but un-decidable (unreadable body *and* un-stattable mtime) —
    /// conservatively treated as [`ClaimOutcome::Contended`] rather than risk
    /// stealing a claim we cannot reason about.
    Unknown,
}

/// Whether `mtime` is older than [`STALE_CLAIM_MAX_AGE`] relative to now. An
/// unreadable or future mtime is **not** treated as exceeded (conservative: do
/// not reclaim on a clock we cannot trust).
fn mtime_exceeded(mtime: Option<SystemTime>) -> bool {
    match mtime {
        Some(mtime) => SystemTime::now()
            .duration_since(mtime)
            .is_ok_and(|age| age > STALE_CLAIM_MAX_AGE),
        None => false,
    }
}

/// Classify the already-read lock `(bytes, mtime)` under `procs`. Split from the
/// read so the caller reads once (anchored, under the guard) and classifies the
/// exact bytes it will act on — no second open between decision and destruction.
fn classify_bytes(bytes: &[u8], mtime: Option<SystemTime>, procs: &dyn ClaimProcs) -> Existing {
    match parse_lock(bytes) {
        Some(record) => {
            // Reclaimable iff the stamped process is not live (dead pid, or a
            // recycled pid whose start_time no longer matches) OR the lock is
            // older than the mtime bound (ADR-105 §5).
            if !procs.is_live(record.pid, record.start_time) || mtime_exceeded(mtime) {
                Existing::Reclaimable
            } else {
                Existing::Live
            }
        }
        // Garbage body: reclaimable only once it is old enough to be clearly
        // abandoned, never immediately (a peer might be mid-lifecycle).
        None => {
            if mtime_exceeded(mtime) {
                Existing::Reclaimable
            } else {
                Existing::Unknown
            }
        }
    }
}

/// Read + classify the existing lock `lock_name` beneath `dirfd` (anchored,
/// symlink-safe via the seam), deciding reclaimability under `procs`. Called
/// **under the guard** in the reclaim path, so the classified inode cannot be
/// swapped before the caller acts on it.
fn classify_existing(
    dirfd: &std::os::fd::OwnedFd,
    lock_name: &str,
    procs: &dyn ClaimProcs,
) -> Existing {
    match read_lock_bytes_at(dirfd, lock_name) {
        Ok((bytes, mtime)) => classify_bytes(&bytes, mtime, procs),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Existing::Vanished,
        // Unreadable (not a clean NotFound): conservatively decline to reclaim a
        // lock we cannot even read.
        Err(_) => Existing::Unknown,
    }
}

/// Stamp `file` (a freshly `O_EXCL`-created lock) and return the held claim.
fn finish_acquire(
    mut file: File,
    producing_dir: &Path,
    lock_name: String,
    procs: &dyn ClaimProcs,
) -> io::Result<ClaimOutcome> {
    let nonce = fresh_nonce();
    let record = LockRecord {
        pid: procs.current_pid(),
        start_time: procs.current_start_time(),
        nonce: nonce.clone(),
    };
    file.write_all(&encode_lock(&record))?;
    file.sync_all()?;
    Ok(ClaimOutcome::Acquired(BaseClaim {
        producing_dir: producing_dir.to_path_buf(),
        lock_name,
        nonce,
        released: false,
    }))
}

/// Acquire the single-flight production claim for `sha` under `base_dir`
/// (ADR-105 §5).
///
/// **Hot path (lock-free):** an `O_EXCL` `create_leaf_under_dirfd` of a fresh
/// `.producing/<sha>.lock`. When no lock exists this wins outright — `O_EXCL` is
/// itself an atomic single-winner, so the common case takes no guard.
///
/// **Slow path (guarded):** on an `O_EXCL` collision, take the per-dir advisory
/// guard ([`lock_guard`]) and run the classify→reclaim critical section under it.
/// A **live** holder ⇒ [`ClaimOutcome::Contended`]; a **dead / PID-reused /
/// timed-out / stale-garbage** lock is reclaimed by [`store::unlink_at`] + retry.
/// The guard serialises all destruction, and the classify reads through an fd
/// opened via the dirfd immediately before the unlink, so the inode classified is
/// the inode destroyed — closing the classify→destroy TOCTOU (a lock-free creator
/// can only ever *create* into an empty slot via `O_EXCL`, never swap an existing
/// inode, so it cannot make us destroy a fresh legitimate claim).
///
/// # Errors
/// A disk error from the ensure-dir / dirfd / create / guard / unlink path (not
/// an `O_EXCL` collision, which is handled internally). The caller treats an
/// error as non-fatal and serves cold (ADR-105 §6).
pub fn claim(base_dir: &Path, sha: &str, procs: &dyn ClaimProcs) -> io::Result<ClaimOutcome> {
    let dir = producing_dir(base_dir);
    store::ensure_dir(&dir)?;
    let dirfd = crate::path_safety::open_workspace_dir_for_fsync(&dir)?;
    let lock_name = lock_leaf(sha);

    // Hot path: no lock present ⇒ `O_EXCL` create wins with no guard.
    match store::create_leaf_under_dirfd(&dirfd, &lock_name) {
        Ok(file) => return finish_acquire(file, &dir, lock_name, procs),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
        Err(err) => return Err(err),
    }

    // Slow path: a lock exists. Serialise ALL destruction through the per-dir
    // advisory guard so classify→destroy is atomic w.r.t. the inode examined
    // (BLOCKING-1 TOCTOU fix). Held for the whole loop; released on return.
    let _guard = lock_guard(&dirfd)?;

    // Bounded retry: each iteration either acquires, concedes, or unlinks a stale
    // lock (after which the next iteration re-creates). The cap prevents an
    // unbounded loop if a lock-free creator keeps re-winning the vacated slot;
    // over the cap we concede (non-fatal).
    for _ in 0..8 {
        match store::create_leaf_under_dirfd(&dirfd, &lock_name) {
            Ok(file) => return finish_acquire(file, &dir, lock_name, procs),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                match classify_existing(&dirfd, &lock_name, procs) {
                    Existing::Live | Existing::Unknown => return Ok(ClaimOutcome::Contended),
                    // Vanished between the failed create and the read — retry.
                    Existing::Vanished => {}
                    Existing::Reclaimable => {
                        // Under the guard the classified inode cannot be swapped by
                        // another reclaimer or releaser; unlink exactly it, then
                        // retry the `O_EXCL` create. A concurrent lock-free creator
                        // that wins the vacated slot is handled by the next
                        // iteration re-classifying its (live) lock as Contended.
                        match store::unlink_at(&dirfd, &lock_name) {
                            Ok(()) => {}
                            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                            Err(err) => return Err(err),
                        }
                    }
                }
            }
            Err(err) => return Err(err),
        }
    }
    // Persistent churn: concede rather than spin (non-fatal, ADR-105 §6).
    Ok(ClaimOutcome::Contended)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A deterministic [`ClaimProcs`] fake: a fixed current identity plus an
    /// explicit set of "live" `(pid, start_time)` pairs.
    struct FakeProcs {
        pid: u32,
        start_time: Option<u64>,
        live: std::sync::Mutex<Vec<(u32, Option<u64>)>>,
    }

    impl FakeProcs {
        fn new(pid: u32, start_time: Option<u64>) -> Self {
            Self {
                pid,
                start_time,
                live: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn with_live(pid: u32, start_time: Option<u64>, live: &[(u32, Option<u64>)]) -> Self {
            let me = Self::new(pid, start_time);
            *me.live.lock().unwrap() = live.to_vec();
            me
        }
    }

    impl ClaimProcs for FakeProcs {
        fn current_pid(&self) -> u32 {
            self.pid
        }
        fn current_start_time(&self) -> Option<u64> {
            self.start_time
        }
        fn is_live(&self, pid: u32, recorded_start_time: Option<u64>) -> bool {
            self.live
                .lock()
                .unwrap()
                .iter()
                .any(|(p, s)| *p == pid && *s == recorded_start_time)
        }
    }

    fn base_dir(tmp: &tempfile::TempDir) -> PathBuf {
        tmp.path().join("graph-cache").join(BASE_SUBDIR)
    }

    /// A gate-clean base payload for a two-file fixture, serialised as `ANVILGB1`.
    fn base_bytes() -> Vec<u8> {
        use anvil_graph_cache::{DependencyGraph, SymbolGraph};
        use anvil_kernel_types::{SymbolKind, TrustLevel, Visibility};
        let mut sym = SymbolGraph::new();
        sym.add_symbol(anvil_kernel_types::SymbolNode {
            id: 1,
            kind: SymbolKind::Function,
            name: "a".to_owned(),
            visibility: Visibility::Public,
            file: "src/a.ts".to_owned(),
            trust_level: TrustLevel::Internal,
            span: None,
        })
        .unwrap();
        let dep = DependencyGraph::new();
        SnapshotPayload::from_graphs(&sym, &dep)
            .unwrap()
            .to_base_bytes()
    }

    #[test]
    fn publish_is_write_once_second_produce_is_noop_success() {
        // (a) two produces for the same sha ⇒ one artefact; the second is a no-op
        // success, and the artefact still gates clean on load.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "a".repeat(40);
        let bytes = base_bytes();

        assert_eq!(
            publish_base(&dir, &sha, &bytes).unwrap(),
            PublishOutcome::Written,
            "first publish writes the artefact"
        );
        assert_eq!(
            publish_base(&dir, &sha, &bytes).unwrap(),
            PublishOutcome::AlreadyPresent,
            "second publish for the same sha is a no-op success"
        );

        // Exactly one artefact on disk.
        let count = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some(BASE_EXT))
            .count();
        assert_eq!(count, 1, "write-once: a single base artefact");

        assert!(matches!(load_base(&dir, &sha), BaseLoadOutcome::Loaded(_)));
    }

    #[test]
    fn concurrent_claim_is_single_flight_exactly_one_winner() {
        // (b) many claimants race for a fresh lock; O_EXCL admits exactly one.
        // Repeated many times so a race window (the original two-winner bug class)
        // is caught deterministically-ish rather than ~1-in-N.
        let all_live: Vec<(u32, Option<u64>)> =
            (0..8u32).map(|i| (1000 + i, Some(u64::from(i)))).collect();

        for round in 0..120u32 {
            let tmp = tempfile::tempdir().unwrap();
            let dir = base_dir(&tmp);
            // A fresh sha per round so no artefact/lock survives between rounds.
            let sha = Arc::new(format!("{round:040x}"));
            let acquired = Arc::new(AtomicU32::new(0));

            std::thread::scope(|scope| {
                for i in 0..8u32 {
                    let dir = dir.clone();
                    let sha = Arc::clone(&sha);
                    let acquired = Arc::clone(&acquired);
                    // Every racer sees every other racer as LIVE, so the ONLY gate
                    // is the `O_EXCL` create — no peer ever reclaims the holder.
                    let all_live = all_live.clone();
                    scope.spawn(move || {
                        let procs = FakeProcs::with_live(1000 + i, Some(u64::from(i)), &all_live);
                        if let ClaimOutcome::Acquired(claim) = claim(&dir, &sha, &procs).unwrap() {
                            acquired.fetch_add(1, Ordering::SeqCst);
                            // Hold briefly so peers race the *held* lock.
                            std::thread::sleep(Duration::from_millis(2));
                            claim.release();
                        }
                    });
                }
            });

            assert_eq!(
                acquired.load(Ordering::SeqCst),
                1,
                "round {round}: exactly one claimant must win the single-flight race",
            );
        }
    }

    #[test]
    fn claim_contends_against_a_live_holder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "c".repeat(40);

        // Holder pid 4242 / start 7 is live.
        let holder = FakeProcs::with_live(4242, Some(7), &[(4242, Some(7))]);
        let held = match claim(&dir, &sha, &holder).unwrap() {
            ClaimOutcome::Acquired(c) => c,
            ClaimOutcome::Contended => panic!("first claim must acquire"),
        };

        // A peer that sees the holder as live must contend, not steal.
        let peer = FakeProcs::with_live(5, Some(9), &[(4242, Some(7)), (5, Some(9))]);
        assert!(matches!(
            claim(&dir, &sha, &peer).unwrap(),
            ClaimOutcome::Contended
        ));
        held.release();
    }

    #[test]
    fn reclaim_on_dead_pid() {
        // (c) dead-pid reclaim path.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "d".repeat(40);

        // A dead holder writes the lock (holder considers itself live to acquire).
        let dead = FakeProcs::with_live(9001, Some(1), &[(9001, Some(1))]);
        let held = match claim(&dir, &sha, &dead).unwrap() {
            ClaimOutcome::Acquired(c) => c,
            ClaimOutcome::Contended => panic!("acquire"),
        };
        // Forget the guard without releasing so the lock stays on disk.
        std::mem::forget(held);

        // The reclaimer sees NO live processes → the dead holder is reclaimable.
        let reclaimer = FakeProcs::new(9002, Some(2));
        assert!(matches!(
            claim(&dir, &sha, &reclaimer).unwrap(),
            ClaimOutcome::Acquired(_)
        ));
    }

    #[test]
    fn reclaim_on_pid_reuse_alive_but_wrong_start_time() {
        // (c) PID-reuse reclaim path: the pid is alive but its start_time differs
        // from the stamped one, so it is NOT the producer we recorded.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "e".repeat(40);

        let holder = FakeProcs::with_live(7000, Some(100), &[(7000, Some(100))]);
        let held = match claim(&dir, &sha, &holder).unwrap() {
            ClaimOutcome::Acquired(c) => c,
            ClaimOutcome::Contended => panic!("acquire"),
        };
        std::mem::forget(held);

        // Reclaimer: pid 7000 is alive but now with start_time 999 — a recycled
        // pid. The stamped (7000, Some(100)) is therefore NOT live.
        let reclaimer = FakeProcs::with_live(7000, Some(100), &[(7000, Some(999))]);
        // is_live((7000, Some(100))) is false because live set only has (7000,
        // Some(999)); reclaim proceeds.
        assert!(matches!(
            claim(&dir, &sha, &reclaimer).unwrap(),
            ClaimOutcome::Acquired(_)
        ));
    }

    #[test]
    fn reclaim_on_mtime_timeout_even_when_pid_looks_live() {
        // (c) mtime-timeout reclaim path: the holder still looks live, but the
        // lock has aged past STALE_CLAIM_MAX_AGE, so it is reclaimable anyway.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "f".repeat(40);

        let holder = FakeProcs::with_live(8000, Some(3), &[(8000, Some(3))]);
        let held = match claim(&dir, &sha, &holder).unwrap() {
            ClaimOutcome::Acquired(c) => c,
            ClaimOutcome::Contended => panic!("acquire"),
        };
        std::mem::forget(held);

        // Age the lock past the bound by back-dating its mtime.
        let lock_path = producing_dir(&dir).join(lock_leaf(&sha));
        let old = SystemTime::now() - (STALE_CLAIM_MAX_AGE + Duration::from_mins(1));
        File::options()
            .write(true)
            .open(&lock_path)
            .unwrap()
            .set_modified(old)
            .unwrap();

        // The reclaimer still considers the holder live, but the mtime bound
        // forces reclaim.
        let reclaimer = FakeProcs::with_live(8001, Some(4), &[(8000, Some(3)), (8001, Some(4))]);
        assert!(matches!(
            claim(&dir, &sha, &reclaimer).unwrap(),
            ClaimOutcome::Acquired(_)
        ));
    }

    #[test]
    fn reclaim_race_has_exactly_one_winner() {
        // (d) two reclaimers race a single stale (dead-holder) lock; the guarded
        // classify→unlink admits exactly one. Looped many times: the original
        // unguarded "classify then renameat-aside" failed ~2/20 with TWO winners,
        // so a single pass under-tested it — this runs ≥100 rounds.
        //
        // Each reclaimer sees the DEAD holder (6000) as not live, but sees the
        // OTHER reclaimer as live — so once one wins, the other must contend
        // against the fresh lock rather than re-steal it.
        let reclaimers_live: Vec<(u32, Option<u64>)> =
            (0..2u32).map(|i| (6100 + i, Some(u64::from(i)))).collect();

        for round in 0..150u32 {
            let tmp = tempfile::tempdir().unwrap();
            let dir = base_dir(&tmp);
            let sha = Arc::new(format!("{round:040x}"));

            // Plant a stale lock from a now-dead holder (forget the guard so the
            // lock stays on disk for the reclaimers to find).
            let dead = FakeProcs::with_live(6000, Some(5), &[(6000, Some(5))]);
            std::mem::forget(match claim(&dir, &sha, &dead).unwrap() {
                ClaimOutcome::Acquired(c) => c,
                ClaimOutcome::Contended => panic!("round {round}: plant acquire"),
            });

            let winners = Arc::new(AtomicU32::new(0));
            std::thread::scope(|scope| {
                for i in 0..2u32 {
                    let dir = dir.clone();
                    let sha = Arc::clone(&sha);
                    let winners = Arc::clone(&winners);
                    let reclaimers_live = reclaimers_live.clone();
                    scope.spawn(move || {
                        let procs =
                            FakeProcs::with_live(6100 + i, Some(u64::from(i)), &reclaimers_live);
                        if let ClaimOutcome::Acquired(claim) = claim(&dir, &sha, &procs).unwrap() {
                            winners.fetch_add(1, Ordering::SeqCst);
                            std::thread::sleep(Duration::from_millis(2));
                            claim.release();
                        }
                    });
                }
            });

            assert_eq!(
                winners.load(Ordering::SeqCst),
                1,
                "round {round}: exactly one reclaimer must win the stale-lock steal",
            );
        }
    }

    #[test]
    fn release_removes_only_our_own_lock() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "2".repeat(40);
        let lock_path = producing_dir(&dir).join(lock_leaf(&sha));

        let procs = FakeProcs::new(3000, Some(1));
        let held = match claim(&dir, &sha, &procs).unwrap() {
            ClaimOutcome::Acquired(c) => c,
            ClaimOutcome::Contended => panic!("acquire"),
        };
        assert!(lock_path.exists());
        held.release();
        assert!(!lock_path.exists(), "release removes our own lock");

        // A foreign lock (different nonce) is NOT removed by a stale guard's
        // release: simulate by writing a foreign lock then releasing a guard that
        // believes it owns this sha.
        fs::create_dir_all(producing_dir(&dir)).unwrap();
        fs::write(
            &lock_path,
            encode_lock(&LockRecord {
                pid: 9999,
                start_time: Some(1),
                nonce: "deadbeefdeadbeef".to_owned(),
            }),
        )
        .unwrap();
        let foreign_guard = BaseClaim {
            producing_dir: producing_dir(&dir),
            lock_name: lock_leaf(&sha),
            nonce: "our-different-nonce".to_owned(),
            released: false,
        };
        foreign_guard.release();
        assert!(
            lock_path.exists(),
            "a guard must not remove a lock carrying a different nonce"
        );
    }

    #[test]
    fn release_does_not_remove_a_peers_reclaimed_lock() {
        // Release-vs-reclaim interleaving (the release TOCTOU): an owner holds a
        // claim that ages past the mtime bound; a peer reclaims + recreates the
        // lock with its own nonce; the owner's LATER release() must NOT delete the
        // peer's fresh lock.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "7".repeat(40);
        let lock_path = producing_dir(&dir).join(lock_leaf(&sha));

        // Owner claims.
        let owner_procs = FakeProcs::with_live(4000, Some(1), &[(4000, Some(1))]);
        let owner = match claim(&dir, &sha, &owner_procs).unwrap() {
            ClaimOutcome::Acquired(c) => c,
            ClaimOutcome::Contended => panic!("owner acquires"),
        };
        let owner_nonce = owner.nonce.clone();

        // Age the lock past the bound so the peer may reclaim despite the owner
        // still looking live.
        let old = SystemTime::now() - (STALE_CLAIM_MAX_AGE + Duration::from_mins(1));
        File::options()
            .write(true)
            .open(&lock_path)
            .unwrap()
            .set_modified(old)
            .unwrap();

        // Peer reclaims (via the mtime bound) and recreates with its own nonce.
        let peer_procs = FakeProcs::with_live(4001, Some(2), &[(4000, Some(1)), (4001, Some(2))]);
        let peer = match claim(&dir, &sha, &peer_procs).unwrap() {
            ClaimOutcome::Acquired(c) => c,
            ClaimOutcome::Contended => panic!("peer reclaims the aged lock"),
        };
        let peer_nonce = peer.nonce.clone();
        assert_ne!(owner_nonce, peer_nonce, "peer stamped a distinct nonce");

        // The on-disk lock now carries the peer's nonce.
        let on_disk = parse_lock(&fs::read(&lock_path).unwrap()).unwrap();
        assert_eq!(on_disk.nonce, peer_nonce, "peer's lock is on disk");

        // The owner's late release() must NOT remove the peer's lock.
        owner.release();
        assert!(
            lock_path.exists(),
            "owner release must not delete the peer's reclaimed lock"
        );
        let after = parse_lock(&fs::read(&lock_path).unwrap()).unwrap();
        assert_eq!(
            after.nonce, peer_nonce,
            "the peer's lock survives release with its own nonce"
        );

        // The peer's own release cleans it up.
        peer.release();
        assert!(!lock_path.exists(), "peer release removes its own lock");
    }

    #[test]
    fn load_ignores_epoch_or_magic_mismatch() {
        use anvil_graph_cache::{DependencyGraph, SymbolGraph};

        // (e) a magic/epoch mismatch load ⇒ typed ignore, never a payload.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);

        // Absent.
        assert!(matches!(
            load_base(&dir, &"3".repeat(40)),
            BaseLoadOutcome::Absent
        ));

        // A per-worktree (ANVILGC1) artefact written under a base leaf is refused
        // as a base (wrong magic) → Ignored, never Loaded.
        let gc_bytes = SnapshotPayload::from_graphs(&SymbolGraph::new(), &DependencyGraph::new())
            .unwrap()
            .to_bytes(); // ANVILGC1
        let sha = "4".repeat(40);
        store::write_sealed(&dir, &base_leaf(&sha), &gc_bytes).unwrap();
        assert!(
            matches!(load_base(&dir, &sha), BaseLoadOutcome::Ignored),
            "an ANVILGC1 artefact loaded as a base must be refused (Ignored)"
        );

        // Corrupt bytes → Ignored, not a panic.
        let sha2 = "5".repeat(40);
        store::write_sealed(&dir, &base_leaf(&sha2), b"not a snapshot at all").unwrap();
        assert!(matches!(load_base(&dir, &sha2), BaseLoadOutcome::Ignored));
    }

    #[test]
    fn default_base_dir_is_under_graph_cache() {
        // Structural: the base dir sits beneath the graph-cache dir when one
        // resolves. Drive it deterministically via ANVIL_HOME through the parent
        // resolver by asserting the suffix shape rather than mutating env here.
        if let Some(dir) = default_base_dir() {
            assert!(dir.ends_with(BASE_SUBDIR));
        }
    }

    #[test]
    fn parse_lock_roundtrip_and_rejects_garbage() {
        let rec = LockRecord {
            pid: 77,
            start_time: Some(123),
            nonce: "abcd1234abcd1234".to_owned(),
        };
        assert_eq!(parse_lock(&encode_lock(&rec)), Some(rec.clone()));

        let no_start = LockRecord {
            start_time: None,
            ..rec
        };
        assert_eq!(parse_lock(&encode_lock(&no_start)), Some(no_start));

        assert_eq!(parse_lock(b"garbage"), None);
        assert_eq!(parse_lock(b"77\n\n"), None, "missing nonce is garbage");
        assert_eq!(parse_lock(b""), None);
    }
}
