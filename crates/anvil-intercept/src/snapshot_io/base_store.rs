//! GBASE-002 (ADR-105): content-addressed, write-once shared base store and
//! single-flight production claim.
//!
//! Bases are keyed by merge-base commit; claims serialise producers;
//! readers never mutate. GC is [`base_gc`].

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anvil_graph_cache::snapshot::{MAX_SNAPSHOT_BYTES, SnapshotPayload};

use super::store;

/// Subdirectory under the graph-cache dir that holds shared base artefacts.
const BASE_SUBDIR: &str = "base";
/// Extension for a published base artefact (`<sha>.base`). Shared with
/// [`super::base_gc`] so the GC enumerator keys on the same extension the store
/// writes.
pub(crate) const BASE_EXT: &str = "base";
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
///
/// **GBASE-010 graduation-gate calibration (2026-07-12 — reasoned keep).** The p95
/// was measured over a representative fixture (see
/// `graph_base_producer::base_production_p95_over_representative_fixture` and
/// `plans/audits/2026-07-12-gbase-graduation-gate.md`): fixture-scale production is
/// **tens of milliseconds**, so a literal `2 × p95` would be sub-second. That
/// value is **deliberately not adopted**: this mtime bound is only the *fallback*
/// for the ambiguous "present, unreadable, or start-time-less" case (the precise
/// [`ClaimProcs::is_live`] pid / PID-reuse check reclaims a dead producer
/// immediately), and it must not reclaim a genuinely-still-producing subprocess on
/// a **large monorepo**, whose real production time is seconds-to-minutes — far
/// above any fixture p95. Over-retention is the safe direction (a slower recovery
/// in a rare ambiguous case, never a stolen live claim), so the conservative 10 min
/// stands with wide margin over the largest plausible `2 × p95`. This is a
/// *reasoned* keep, not an unexamined one: revisit only if real-fleet telemetry
/// ever shows monorepo production approaching this bound.
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
    // macOS/APFS can spuriously fail concurrent `O_CREAT` opens of the SAME
    // name with `ENOENT` (kernel directory-entry race, reproduced on the CI
    // arm64 runners by racing claimants opening this create-once guard). The
    // guard is never removed once created, so a brief retry always converges;
    // any other error propagates immediately.
    let mut denied = nix::errno::Errno::ENOENT;
    for _ in 0..16 {
        match openat(dirfd.as_fd(), GUARD_NAME, flags, mode) {
            Ok(fd) => return Ok(File::from(fd)),
            Err(err @ nix::errno::Errno::ENOENT) => {
                denied = err;
                std::thread::yield_now();
            }
            Err(err) => return Err(io::Error::from(err)),
        }
    }
    Err(io::Error::from(denied))
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
    let file = open_guard(dirfd).map_err(tag_step("guard open"))?;
    nix::fcntl::Flock::lock(file, nix::fcntl::FlockArg::LockExclusive)
        .map_err(|(_file, errno)| tag_step("guard flock")(io::Error::from(errno)))
}

/// Tag an `io::Error` with the claim step that produced it, preserving its
/// [`io::ErrorKind`] and keeping the original error reachable via `source()`.
/// Claim errors are non-fatal — the caller degrades to cold and the error
/// surfaces only in logs — where the step name turns a bare "No such file or
/// directory" into a diagnosable report.
fn tag_step(step: &'static str) -> impl Fn(io::Error) -> io::Error {
    move |err| io::Error::new(err.kind(), StepError { step, source: err })
}

/// The payload [`tag_step`] wraps: `Display` renders `"{step}: {source}"`,
/// and the original `io::Error` stays on the `source()` chain for callers
/// that introspect past the message.
#[derive(Debug)]
struct StepError {
    step: &'static str,
    source: io::Error,
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.step, self.source)
    }
}

impl std::error::Error for StepError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
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

/// Atomically publish the fully-stamped temp `temp_name` at `final_name` under
/// `dirfd` with **exclusive** semantics: `linkat` hard-links the temp's inode to
/// `final_name`, failing `EEXIST` if `final_name` already exists. Because the temp
/// is fully written + `fsync`'d before this call, a lock file is **never visible
/// unstamped**, and the link's atomic-exclusive create preserves the single-winner
/// semantics the hot path relies on. `Ok(true)` = we published (won); `Ok(false)`
/// = `final_name` already exists (a peer holds it).
///
/// `linkat` is the portable POSIX atomic-exclusive primitive; Linux's
/// `renameat2(RENAME_NOREPLACE)` is glibc-Linux-only in `nix`, so `linkat` keeps
/// one audited publish path across every `cfg(unix)` target. On the rare
/// filesystem that refuses same-dir hardlinks (some FUSE/overlay configs) this
/// surfaces as a generic `io::Error` from `claim()` — non-fatal, the caller
/// degrades to cold per ADR-105 §6.
fn publish_exclusive(
    dirfd: &std::os::fd::OwnedFd,
    temp_name: &str,
    final_name: &str,
) -> io::Result<bool> {
    use nix::fcntl::AtFlags;
    use nix::unistd::linkat;
    // `AtFlags::empty()` = do not follow a symlink at the temp path (there is
    // none — the temp is a regular file we just created). `linkat` fails `EEXIST`
    // if `final_name` already exists, which is the atomic-exclusive guarantee.
    match linkat(dirfd, temp_name, dirfd, final_name, AtFlags::empty()) {
        Ok(()) => Ok(true),
        Err(nix::errno::Errno::EEXIST) => Ok(false),
        Err(err) => Err(io::Error::from(err)),
    }
}

/// Build a fully-stamped `{pid, start_time, nonce}` record, write + `fsync` it to
/// a uniquely-named temp, then atomically publish it at `lock_name` with exclusive
/// semantics ([`publish_exclusive`]).
///
/// The lock is therefore **never visible unstamped** (the CI two-winner root
/// cause): a racing claimant either fails to see it at all (`Ok(None)` on the
/// `EEXIST` from a peer's link) or reads a complete, correct record — never an
/// empty or torn one that could mis-parse to a foreign pid and be wrongly judged
/// reclaimable. `Ok(Some(claim))` = we hold the claim; `Ok(None)` = a peer already
/// holds `lock_name`.
fn try_stamped_create(
    dirfd: &std::os::fd::OwnedFd,
    producing_dir: &Path,
    lock_name: &str,
    procs: &dyn ClaimProcs,
) -> io::Result<Option<BaseClaim>> {
    let nonce = fresh_nonce();
    let record = LockRecord {
        pid: procs.current_pid(),
        start_time: procs.current_start_time(),
        nonce: nonce.clone(),
    };
    // Unique temp name (nonce-suffixed) so the `O_EXCL` temp create never collides.
    let temp_name = format!("{lock_name}.{nonce}.tmp");

    // Create + fully stamp + fsync the temp BEFORE it can be published.
    let stamp = (|| -> io::Result<()> {
        let mut file =
            store::create_leaf_under_dirfd(dirfd, &temp_name).map_err(tag_step("stamp create"))?;
        file.write_all(&encode_lock(&record))
            .map_err(tag_step("stamp write"))?;
        file.sync_all().map_err(tag_step("stamp fsync"))
    })();
    if let Err(err) = stamp {
        let _ = store::unlink_at(dirfd, &temp_name);
        return Err(err);
    }

    let published = publish_exclusive(dirfd, &temp_name, lock_name);
    // The temp is now either an extra hard-link to the published inode (winner) or
    // a loser's discard — unlink it either way (best-effort; ignore a NotFound).
    let _ = store::unlink_at(dirfd, &temp_name);

    let published = match published {
        // macOS/APFS can spuriously report `ENOENT` from `linkat` when the
        // target directory is under concurrent entry churn (racing claimants
        // creating and unlinking temps beside the lock). The source temp
        // provably exists — just created, fsync'd, nonce-unique — so `ENOENT`
        // cannot mean a missing source. Decide by reading the published record
        // back: our full stamped identity (`{pid, start_time, nonce}` — not
        // the nonce alone, whose `getrandom`-failure fallback can collide)
        // means the link landed and we won; a peer's record or nothing at all
        // means this attempt lost, and the caller's guarded slow path
        // re-classifies and retries.
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            Ok(read_lock_record_at(dirfd, lock_name).is_some_and(|published| published == record))
        }
        other => other.map_err(tag_step("publish linkat")),
    };

    if published? {
        Ok(Some(BaseClaim {
            producing_dir: producing_dir.to_path_buf(),
            lock_name: lock_name.to_owned(),
            nonce,
            released: false,
        }))
    } else {
        Ok(None)
    }
}

/// Acquire the single-flight production claim for `sha` under `base_dir`
/// (ADR-105 §5).
///
/// **Hot path (lock-free):** [`try_stamped_create`] — write a fully-stamped lock
/// to a temp, then atomically `linkat` it into place with exclusive semantics.
/// Exactly one racer's link wins; the rest see `Ok(None)` and fall through. The
/// lock is never visible unstamped, so no racer can read an empty/torn record.
///
/// **Slow path (guarded):** when the lock already exists, take the per-dir
/// advisory guard ([`lock_guard`]) and run the classify→reclaim critical section
/// under it. A **live** holder ⇒ [`ClaimOutcome::Contended`]; a **dead /
/// PID-reused / timed-out / stale-garbage** lock is reclaimed by
/// [`store::unlink_at`] + retry. The guard serialises all destruction, and the
/// classify reads through an fd opened via the dirfd immediately before the
/// unlink, so the inode classified is the inode destroyed — closing the
/// classify→destroy TOCTOU. A lock-free creator can only ever *publish* into an
/// empty slot (its `linkat` fails `EEXIST` otherwise), never swap an existing
/// inode, so it cannot make a guarded reclaimer destroy a fresh legitimate claim.
///
/// # Errors
/// A disk error from the ensure-dir / dirfd / temp-create / publish / guard /
/// unlink path (not an `EEXIST` collision, which is handled internally). The
/// caller treats an error as non-fatal and serves cold (ADR-105 §6).
pub fn claim(base_dir: &Path, sha: &str, procs: &dyn ClaimProcs) -> io::Result<ClaimOutcome> {
    let dir = producing_dir(base_dir);
    store::ensure_dir(&dir).map_err(tag_step("ensure producing dir"))?;
    let dirfd = crate::path_safety::open_workspace_dir_for_fsync(&dir)
        .map_err(tag_step("open producing dir"))?;
    let lock_name = lock_leaf(sha);

    // Hot path: publish a fully-stamped lock with no guard. Exactly one racer's
    // exclusive link wins; the rest see `Ok(None)`.
    if let Some(claim) = try_stamped_create(&dirfd, &dir, &lock_name, procs)? {
        return Ok(ClaimOutcome::Acquired(claim));
    }

    // Slow path: a lock exists. Serialise ALL destruction through the per-dir
    // advisory guard so classify→destroy is atomic w.r.t. the inode examined
    // (BLOCKING-1 TOCTOU fix). Held for the whole loop; released on return.
    let _guard = lock_guard(&dirfd)?;

    // Bounded retry: each iteration either acquires, concedes, or unlinks a stale
    // lock (after which the next iteration re-publishes). The cap prevents an
    // unbounded loop if a lock-free creator keeps re-winning the vacated slot;
    // over the cap we concede (non-fatal).
    for _ in 0..8 {
        if let Some(claim) = try_stamped_create(&dirfd, &dir, &lock_name, procs)? {
            return Ok(ClaimOutcome::Acquired(claim));
        }
        match classify_existing(&dirfd, &lock_name, procs) {
            Existing::Live | Existing::Unknown => return Ok(ClaimOutcome::Contended),
            // Vanished between the failed publish and the read — retry.
            Existing::Vanished => {}
            Existing::Reclaimable => {
                // Under the guard the classified inode cannot be swapped by another
                // reclaimer or releaser; unlink exactly it, then retry the publish.
                // A concurrent lock-free creator that wins the vacated slot is
                // handled by the next iteration re-classifying its (live, fully
                // stamped) lock as Contended.
                match store::unlink_at(&dirfd, &lock_name) {
                    Ok(()) => {}
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                    Err(err) => return Err(tag_step("reclaim unlink")(err)),
                }
            }
        }
    }
    // Persistent churn: concede rather than spin (non-fatal, ADR-105 §6).
    Ok(ClaimOutcome::Contended)
}

/// Outcome of a GBASE-008 GC reclaim attempt for one base sha. Consumed only by
/// the sibling [`super::base_gc`] orchestrator, hence crate-private.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BaseReclaimOutcome {
    /// The base artefact was unlinked — it was unreferenced (the caller's keep-set
    /// decision) and no live production claim held its sha.
    Reclaimed,
    /// A live (or un-decidable) production claim holds this sha, so the base is
    /// **under active production** and must not be removed (ADR-105 §5). Any stale
    /// lock is left for the claim path to reclaim — GC never touches the lock.
    SkippedClaimed,
    /// No base artefact existed for this sha (already gone, or a raced peer removed
    /// it) — a no-op success.
    Absent,
}

/// GBASE-008 (ADR-105 §5/§9): reclaim the base artefact for `sha` **iff no live
/// production claim holds it**. The caller ([`super::base_gc`]) has already proven
/// `sha` is referenced by no live registered worktree's merge-base; this function
/// owns the claim-respecting, guard-honest destruction half.
///
/// # Extends the module destruction invariant to base artefacts
///
/// The per-`.producing`-dir `.guard` `flock` guards **all** destruction in this
/// module (claim reclaim + release). GC's base unlink rides the **same** guard:
/// the classify (is a live claim producing this sha?) and the `unlinkat` of
/// `<sha>.base` run under one held `.guard` `flock`, so a base unlink never
/// interleaves with a guarded claim reclaim/release. The **hot-path** claim
/// (`O_EXCL`/`linkat`) is deliberately lock-free and does not take the guard; a
/// producer that claims-and-finishes a sha entirely within the tiny window between
/// GC's under-guard classify and its unlink would see its fresh base reclaimed —
/// but that only occurs for a sha a worktree just rebased **onto** (so the keep-set
/// GC resolved a moment earlier is stale by one pass), and the next ref-change
/// trigger re-produces it while the cold path serves (ADR-105 §6). Under-production
/// is caught: a live claim classifies [`Existing::Live`] and skips.
///
/// A **stale** claim lock ([`Existing::Reclaimable`]) does **not** block base
/// reclaim, but GC does **not** unlink it either — lock reclaim is the claim
/// path's job (a subsequent `claim()` for the sha reclaims it), never GC's.
///
/// # Errors
/// Any `io::Error` from ensuring/opening the guarded `.producing` dir, opening the
/// base dir fd, or the base `unlinkat` (a clean `NotFound` on the base is
/// [`BaseReclaimOutcome::Absent`], not an error). The caller treats an error as
/// non-fatal and logs-and-degrades (ADR-105 §6).
///
/// Crate-private: the sole caller is the sibling [`super::base_gc`] orchestrator.
pub(crate) fn reclaim_unreferenced_base(
    base_dir: &Path,
    sha: &str,
    procs: &dyn ClaimProcs,
) -> io::Result<BaseReclaimOutcome> {
    // The guard lives under `.producing`; ensure it exists so the rendezvous file
    // has a home even when no production ever claimed this store. Without the guard
    // we decline to destroy anything (conservative).
    let producing = producing_dir(base_dir);
    store::ensure_dir(&producing)?;
    let pdirfd = crate::path_safety::open_workspace_dir_for_fsync(&producing)?;
    let _guard = lock_guard(&pdirfd)?;

    // Under the guard, re-verify no live claim is producing this sha. A live or
    // un-decidable (present-but-unreadable) claim ⇒ respect active production.
    match classify_existing(&pdirfd, &lock_leaf(sha), procs) {
        Existing::Live | Existing::Unknown => return Ok(BaseReclaimOutcome::SkippedClaimed),
        // No claim, a vanished lock, or a provably-stale one: none is active
        // production, so the base is reclaimable. The stale lock (if any) is the
        // claim path's to reclaim, not ours.
        Existing::Vanished | Existing::Reclaimable => {}
    }

    // Unlink the base artefact under the guard, anchored at the base dir fd
    // (symlink-safe `unlinkat`, CIB-097 discipline) — the same guard held across
    // the classify above, so classify→destroy is atomic w.r.t. guarded claim ops.
    let base_dirfd = crate::path_safety::open_workspace_dir_for_fsync(base_dir)?;
    match store::unlink_at(&base_dirfd, &base_leaf(sha)) {
        Ok(()) => Ok(BaseReclaimOutcome::Reclaimed),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(BaseReclaimOutcome::Absent),
        Err(err) => Err(err),
    }
}

/// Operator classification of one `.producing/<sha>.lock` (CIB-344).
///
/// Distinct from [`Existing`]: a live producer that has been running longer
/// than [`STALE_CLAIM_MAX_AGE`] is still **live** here. The mtime fallback is
/// only for the claim path (steal a stuck producer). Doctor / start must not
/// unlink an in-flight producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProduceLockClass {
    /// Stamped process is still that same live producer.
    Live,
    /// Pid is dead or PID-reused. Safe to unlink.
    Stale,
    /// Unreadable / malformed / mid-stamp. Leave alone.
    Unknown,
}

/// One observed `.producing/<sha>.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProduceLock {
    /// Leaf name (`<sha>.lock`).
    pub name: String,
    /// Stamped pid, when the body parsed.
    pub pid: Option<u32>,
    /// Stamped start-time discriminator, when present.
    pub start_time: Option<u64>,
    /// Operator liveness class (dead-pid only; no mtime steal).
    pub class: ProduceLockClass,
}

/// Scan `base_dir/.producing/*.lock` (never `.guard`, never temps).
///
/// A missing producing dir is empty. Order is sorted by leaf name.
///
/// # Errors
/// Opening the producing dir failed for a reason other than `NotFound`
/// (permission, symlink refusal).
pub fn list_produce_locks(base_dir: &Path, procs: &dyn ClaimProcs) -> io::Result<Vec<ProduceLock>> {
    let dir = producing_dir(base_dir);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let dirfd = crate::path_safety::open_workspace_dir_for_fsync(&dir)?;
    let mut out = Vec::new();
    for name in lock_leaf_names(&dir) {
        let record = read_lock_record_at(&dirfd, &name);
        out.push(ProduceLock {
            name: name.clone(),
            pid: record.as_ref().map(|r| r.pid),
            start_time: record.as_ref().and_then(|r| r.start_time),
            class: classify_operator_lock(&dirfd, &name, procs),
        });
    }
    Ok(out)
}

/// Unlink stale produce-locks only (dead pid / PID-reuse). Live and unknown
/// locks are retained. Uses the per-dir `.guard` so classify→destroy is
/// atomic w.r.t. claim reclaim/release.
///
/// # Errors
/// Opening the producing dir, taking the guard, or a non-NotFound unlink.
/// A missing producing dir is `Ok(0)` — nothing to reap.
pub fn reap_stale_produce_locks(base_dir: &Path, procs: &dyn ClaimProcs) -> io::Result<usize> {
    let dir = producing_dir(base_dir);
    if !dir.is_dir() {
        return Ok(0);
    }
    let dirfd = crate::path_safety::open_workspace_dir_for_fsync(&dir)?;
    let _guard = lock_guard(&dirfd)?;
    let mut removed = 0usize;
    for name in lock_leaf_names(&dir) {
        if classify_operator_lock(&dirfd, &name, procs) != ProduceLockClass::Stale {
            continue;
        }
        match store::unlink_at(&dirfd, &name) {
            Ok(()) => removed += 1,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(removed)
}

/// [`list_produce_locks`] over [`default_base_dir`] and [`SystemClaimProcs`].
///
/// # Errors
/// Same as [`list_produce_locks`]. Missing default dir is empty.
pub fn list_default_produce_locks() -> io::Result<Vec<ProduceLock>> {
    let Some(dir) = default_base_dir() else {
        return Ok(Vec::new());
    };
    list_produce_locks(&dir, &SystemClaimProcs)
}

/// [`reap_stale_produce_locks`] over the default store.
///
/// # Errors
/// Same as [`reap_stale_produce_locks`]. Missing default dir is `Ok(0)`.
pub fn reap_default_stale_produce_locks() -> io::Result<usize> {
    let Some(dir) = default_base_dir() else {
        return Ok(0);
    };
    reap_stale_produce_locks(&dir, &SystemClaimProcs)
}

fn lock_leaf_names(producing: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(producing) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("lock"))
        {
            continue;
        }
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_file() {
            continue;
        }
        names.push(name.to_owned());
    }
    names.sort();
    names
}

/// Dead-pid / PID-reuse only. Does **not** treat mtime age as stale.
fn classify_operator_lock(
    dirfd: &std::os::fd::OwnedFd,
    lock_name: &str,
    procs: &dyn ClaimProcs,
) -> ProduceLockClass {
    match read_lock_bytes_at(dirfd, lock_name) {
        Ok((bytes, _mtime)) => match parse_lock(&bytes) {
            Some(record) => {
                if procs.is_live(record.pid, record.start_time) {
                    ProduceLockClass::Live
                } else {
                    ProduceLockClass::Stale
                }
            }
            None => ProduceLockClass::Unknown,
        },
        Err(_) => ProduceLockClass::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;

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
    fn claim_concedes_to_an_unstamped_lock_mid_stamp_window() {
        // Regression for the CI two-winner root cause: a lock that is present but
        // NOT yet stamped (the mid-stamp window a non-atomic create exposed) must
        // make a claimant CONCEDE, never steal — even a claimant that sees nothing
        // as live. Simulate the window by directly creating an empty lock via the
        // seam, then run claim().
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "8".repeat(40);
        let pdir = producing_dir(&dir);
        store::ensure_dir(&pdir).unwrap();
        let dirfd = crate::path_safety::open_workspace_dir_for_fsync(&pdir).unwrap();
        // An empty, UNSTAMPED lock at the final name (mid-stamp state).
        drop(store::create_leaf_under_dirfd(&dirfd, &lock_leaf(&sha)).unwrap());
        let lock_path = pdir.join(lock_leaf(&sha));
        assert!(lock_path.exists() && fs::read(&lock_path).unwrap().is_empty());

        // A claimant that sees NOTHING as live must still concede (owner presumed
        // mid-stamp) — not reclaim the empty lock.
        let procs = FakeProcs::new(5000, Some(1));
        assert!(
            matches!(claim(&dir, &sha, &procs).unwrap(), ClaimOutcome::Contended),
            "an empty/mid-stamp lock must be conceded, never stolen",
        );
        // The unstamped lock is untouched (not reclaimed).
        assert!(
            lock_path.exists() && fs::read(&lock_path).unwrap().is_empty(),
            "claim must not have unlinked the mid-stamp lock",
        );

        // Once a real stamp lands, a peer still contends against the live owner.
        fs::write(
            &lock_path,
            encode_lock(&LockRecord {
                pid: 5000,
                start_time: Some(1),
                nonce: "abcdef0123456789".to_owned(),
            }),
        )
        .unwrap();
        let peer = FakeProcs::with_live(5001, Some(2), &[(5000, Some(1)), (5001, Some(2))]);
        assert!(matches!(
            claim(&dir, &sha, &peer).unwrap(),
            ClaimOutcome::Contended
        ));
    }

    #[test]
    fn concurrent_claim_is_single_flight_exactly_one_winner() {
        // (b) many claimants race for a fresh lock; the atomic-exclusive linkat
        // publish admits exactly one. Repeated many times so a race window (the
        // original two-winner bug class) is caught deterministically-ish.
        //
        // Every acquired claim is HELD (pushed into `held`) until the whole burst
        // has joined, then released together. Releasing a winning claim *mid-burst*
        // would legitimately let a still-contending loser acquire the freed slot —
        // that is not a single-flight violation (the write-once store prevents
        // redundant production; the claim only guarantees ≤1 holder at an instant),
        // but it would inflate the concurrent-winner count. Holding across the burst
        // asserts the true invariant: never two simultaneous holders.
        let all_live: Vec<(u32, Option<u64>)> =
            (0..8u32).map(|i| (1000 + i, Some(u64::from(i)))).collect();

        for round in 0..120u32 {
            let tmp = tempfile::tempdir().unwrap();
            let dir = base_dir(&tmp);
            // A fresh sha per round so no artefact/lock survives between rounds.
            let sha = Arc::new(format!("{round:040x}"));
            let held = Arc::new(std::sync::Mutex::new(Vec::new()));

            std::thread::scope(|scope| {
                for i in 0..8u32 {
                    let dir = dir.clone();
                    let sha = Arc::clone(&sha);
                    let held = Arc::clone(&held);
                    // Every racer sees every other racer as LIVE, so the ONLY gate
                    // is the atomic-exclusive publish — no peer ever reclaims.
                    let all_live = all_live.clone();
                    scope.spawn(move || {
                        let procs = FakeProcs::with_live(1000 + i, Some(u64::from(i)), &all_live);
                        if let ClaimOutcome::Acquired(claim) = claim(&dir, &sha, &procs).unwrap() {
                            // Keep the claim alive until the burst completes.
                            held.lock().unwrap().push(claim);
                        }
                    });
                }
            });

            let winners = held.lock().unwrap().len();
            assert_eq!(
                winners, 1,
                "round {round}: exactly one claimant may hold the lock concurrently",
            );
            // `held` (and the winning claim) drops here, after the burst joined.
        }
    }

    // ========================================================================
    // GBASE-010 graduation-gate evidence — herd-miss single-flight (fleet shape)
    // ========================================================================

    /// **GBASE-010 §11 criterion: herd-miss under single-flight (fleet-shaped).**
    ///
    /// The existing single-flight tests prove the *producer* invariant (≤1
    /// concurrent claim holder) for an 8-way burst. This extends it to the ADR-105
    /// §11 fleet scenario — **many worktrees of the same repo simultaneously
    /// discovering the same fresh merge-base sha** (a fleet rebasing onto fresh
    /// `main`) — and adds the *consumer* dimension the producer tests do not cover:
    ///
    /// 1. **exactly-one-producer** — of `N` racers for one fresh sha, precisely one
    ///    `Acquired`; every other is `Contended` (a live peer holds the claim, so
    ///    none reclaims). No thundering herd of redundant producers.
    /// 2. **all-eventually-cold-served** — while production is in flight, every
    ///    non-winner that `load_base`s sees `Absent` (serve cold, non-fatal — the
    ///    ADR-105 §6 fallback), never a torn/partial artefact.
    /// 3. **all-eventually-warm** — once the single producer publishes and releases,
    ///    every consumer `load_base`s the identical shared artefact (`Loaded`) — the
    ///    O(1)-production, O(N)-share win the whole design exists for.
    ///
    /// Run at a fleet-scale `N = 48` (far above the 8-way burst) across 20 rounds
    /// with a fresh sha per round (the CI concurrency-harness multiplier, and
    /// `taskset -c 0,1`-shaped when pinned).
    #[test]
    fn herd_miss_single_flight_fleet_shaped() {
        const FLEET: u32 = 48;
        const ROUNDS: u32 = 20;

        // Every worktree in the fleet sees every other as LIVE, so the ONLY gate is
        // the atomic-exclusive publish — no peer ever reclaims a live producer.
        let all_live: Vec<(u32, Option<u64>)> =
            (0..FLEET).map(|i| (2000 + i, Some(u64::from(i)))).collect();

        for round in 0..ROUNDS {
            let tmp = tempfile::tempdir().unwrap();
            let dir = base_dir(&tmp);
            let sha = Arc::new(format!("{round:040x}"));
            let winners = Arc::new(std::sync::atomic::AtomicU32::new(0));
            let contended = Arc::new(std::sync::atomic::AtomicU32::new(0));
            // Consumers that read the store DURING production must never see a torn
            // artefact — only `Absent` (produce not yet published) is acceptable cold.
            let cold_absent = Arc::new(std::sync::atomic::AtomicU32::new(0));
            // The single winning claim, captured so the burst can publish + release
            // exactly once after the herd has fully joined.
            let winner = Arc::new(std::sync::Mutex::new(None::<BaseClaim>));

            std::thread::scope(|scope| {
                for i in 0..FLEET {
                    let dir = dir.clone();
                    let sha = Arc::clone(&sha);
                    let winners = Arc::clone(&winners);
                    let contended = Arc::clone(&contended);
                    let cold_absent = Arc::clone(&cold_absent);
                    let winner = Arc::clone(&winner);
                    let all_live = all_live.clone();
                    scope.spawn(move || {
                        let procs = FakeProcs::with_live(2000 + i, Some(u64::from(i)), &all_live);
                        match claim(&dir, &sha, &procs).unwrap() {
                            ClaimOutcome::Acquired(claim) => {
                                winners.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                // Hold the winner until the herd joins; it must be the
                                // sole producer.
                                *winner.lock().unwrap() = Some(claim);
                            }
                            ClaimOutcome::Contended => {
                                contended.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                // A loser serves cold: the base is not yet published,
                                // so a concurrent read must be `Absent`, never torn.
                                if matches!(load_base(&dir, &sha), BaseLoadOutcome::Absent) {
                                    cold_absent.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                }
                            }
                        }
                    });
                }
            });

            // (1) exactly-one-producer.
            assert_eq!(
                winners.load(std::sync::atomic::Ordering::SeqCst),
                1,
                "round {round}: a fleet of {FLEET} must elect exactly one producer",
            );
            assert_eq!(
                contended.load(std::sync::atomic::Ordering::SeqCst),
                FLEET - 1,
                "round {round}: every non-winner must contend (no redundant producer)",
            );
            // (2) all-eventually-cold-served: every loser that read mid-flight saw a
            // clean `Absent`, never a torn artefact.
            assert_eq!(
                cold_absent.load(std::sync::atomic::Ordering::SeqCst),
                FLEET - 1,
                "round {round}: a mid-flight consumer must cold-serve `Absent`, never torn",
            );

            // The single producer publishes the shared base once, then releases.
            let claim = winner.lock().unwrap().take().expect("one winner");
            assert_eq!(
                publish_base(&dir, &sha, &base_bytes()).unwrap(),
                PublishOutcome::Written,
                "round {round}: the sole producer writes the shared artefact once",
            );
            claim.release();

            // (3) all-eventually-warm: every worktree in the fleet now loads the
            // identical shared artefact — one production, N-way share.
            for i in 0..FLEET {
                assert!(
                    matches!(load_base(&dir, &sha), BaseLoadOutcome::Loaded(_)),
                    "round {round}: consumer {i} must warm from the shared base",
                );
            }
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

            // Winners are HELD until the burst joins (see the note in
            // `concurrent_claim_...`): releasing mid-burst would let the loser
            // legitimately take the freed slot and inflate the count.
            let held = Arc::new(std::sync::Mutex::new(Vec::new()));
            std::thread::scope(|scope| {
                for i in 0..2u32 {
                    let dir = dir.clone();
                    let sha = Arc::clone(&sha);
                    let held = Arc::clone(&held);
                    let reclaimers_live = reclaimers_live.clone();
                    scope.spawn(move || {
                        let procs =
                            FakeProcs::with_live(6100 + i, Some(u64::from(i)), &reclaimers_live);
                        if let ClaimOutcome::Acquired(claim) = claim(&dir, &sha, &procs).unwrap() {
                            held.lock().unwrap().push(claim);
                        }
                    });
                }
            });

            assert_eq!(
                held.lock().unwrap().len(),
                1,
                "round {round}: exactly one reclaimer may hold the stolen lock concurrently",
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

    fn plant_lock(dir: &Path, sha: &str, pid: u32, start_time: Option<u64>) -> PathBuf {
        let pdir = producing_dir(dir);
        store::ensure_dir(&pdir).unwrap();
        let path = pdir.join(lock_leaf(sha));
        fs::write(
            &path,
            encode_lock(&LockRecord {
                pid,
                start_time,
                nonce: "cib344lockcib344".to_owned(),
            }),
        )
        .unwrap();
        path
    }

    #[test]
    fn list_names_dead_pid_stale_and_live_pid_live() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let dead_sha = "a".repeat(40);
        let live_sha = "b".repeat(40);
        plant_lock(&dir, &dead_sha, 9001, Some(1));
        plant_lock(&dir, &live_sha, 42, Some(2));
        let procs = FakeProcs::with_live(1, Some(0), &[(42, Some(2))]);

        let listed = list_produce_locks(&dir, &procs).expect("list produce locks");
        assert_eq!(listed.len(), 2, "{listed:?}");
        let dead = listed.iter().find(|l| l.pid == Some(9001)).unwrap();
        assert_eq!(dead.class, ProduceLockClass::Stale);
        assert_eq!(dead.name, lock_leaf(&dead_sha));
        let live = listed.iter().find(|l| l.pid == Some(42)).unwrap();
        assert_eq!(live.class, ProduceLockClass::Live);
    }

    #[test]
    fn reap_removes_dead_pid_and_retains_live_and_unknown() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let dead_sha = "c".repeat(40);
        let live_sha = "d".repeat(40);
        let garbage_sha = "e".repeat(40);
        let dead_path = plant_lock(&dir, &dead_sha, 9001, Some(1));
        let live_path = plant_lock(&dir, &live_sha, 42, Some(2));
        let pdir = producing_dir(&dir);
        let garbage_path = pdir.join(lock_leaf(&garbage_sha));
        fs::write(&garbage_path, b"not-a-lock").unwrap();
        // `.guard` must survive; claim/reap creates it, plant it explicitly too.
        let guard_path = pdir.join(GUARD_NAME);
        fs::write(&guard_path, b"").unwrap();

        let procs = FakeProcs::with_live(1, Some(0), &[(42, Some(2))]);
        assert_eq!(reap_stale_produce_locks(&dir, &procs).unwrap(), 1);
        assert!(!dead_path.exists(), "dead-pid lock must be removed");
        assert!(live_path.exists(), "live producer lock must be retained");
        assert!(garbage_path.exists(), "malformed lock must be left alone");
        assert!(guard_path.exists(), ".guard must never be reaped");
    }

    #[test]
    fn operator_reap_does_not_steal_ancient_live_producer() {
        // Claim-path mtime fallback would reclaim this; doctor/start must not.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "f".repeat(40);
        let path = plant_lock(&dir, &sha, 8000, Some(3));
        let old = SystemTime::now() - (STALE_CLAIM_MAX_AGE + Duration::from_mins(1));
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old)
            .unwrap();

        let procs = FakeProcs::with_live(1, Some(0), &[(8000, Some(3))]);
        let listed = list_produce_locks(&dir, &procs).expect("list produce locks");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].class, ProduceLockClass::Live);
        assert_eq!(reap_stale_produce_locks(&dir, &procs).unwrap(), 0);
        assert!(path.exists(), "ancient but live producer must be retained");
    }

    #[test]
    fn pid_reuse_is_stale_for_operator_reap() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let sha = "1".repeat(40);
        let path = plant_lock(&dir, &sha, 7000, Some(100));
        // Same pid, different start_time — recycled, not the producer.
        let procs = FakeProcs::with_live(7000, Some(999), &[(7000, Some(999))]);
        assert_eq!(
            list_produce_locks(&dir, &procs).expect("list produce locks")[0].class,
            ProduceLockClass::Stale
        );
        assert_eq!(reap_stale_produce_locks(&dir, &procs).unwrap(), 1);
        assert!(!path.exists());
    }

    #[test]
    fn system_procs_reaps_exited_pid_and_keeps_self() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let child = std::process::Command::new("true")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let dead_pid = child.id();
        let _ = child.wait_with_output();
        let live_pid = std::process::id();
        let dead_sha = "a".repeat(40);
        let live_sha = "b".repeat(40);
        let dead_path = plant_lock(&dir, &dead_sha, dead_pid, None);
        let live_path = plant_lock(&dir, &live_sha, live_pid, None);

        let listed = list_produce_locks(&dir, &SystemClaimProcs).expect("list produce locks");
        let dead = listed.iter().find(|l| l.pid == Some(dead_pid)).unwrap();
        assert_eq!(dead.class, ProduceLockClass::Stale);
        let live = listed.iter().find(|l| l.pid == Some(live_pid)).unwrap();
        assert_eq!(live.class, ProduceLockClass::Live);

        assert_eq!(
            reap_stale_produce_locks(&dir, &SystemClaimProcs).unwrap(),
            1
        );
        assert!(!dead_path.exists());
        assert!(live_path.exists());
    }

    #[test]
    fn missing_producing_dir_is_empty_and_reap_is_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let procs = FakeProcs::new(1, Some(0));
        assert!(
            list_produce_locks(&dir, &procs)
                .expect("list produce locks")
                .is_empty()
        );
        assert_eq!(reap_stale_produce_locks(&dir, &procs).unwrap(), 0);
        assert!(
            !producing_dir(&dir).exists(),
            "reap must not create the dir"
        );
    }
}
