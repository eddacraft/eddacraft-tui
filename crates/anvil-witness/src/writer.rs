use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fs2::FileExt;
use sha2::{Digest, Sha256};

use crate::line::{LineHash, WitnessLine, compute_line_hash};

/// Per-acquire ceiling for the witness `.lock` flock (CIB-124). A normal append
/// holds the lock for milliseconds; this bound exists so a stalled holder (a hung
/// writer, or a wedged daemon holding the lock mid-append) cannot block a
/// concurrent `git commit` *forever* — the acquire times out to
/// [`WriterError::LockTimeout`] instead. It is generous enough that legitimate
/// contention between concurrent worktree hooks never trips it (each hold is
/// sub-millisecond, so this only fires against a genuine wedge).
///
/// This is the ceiling for **one** acquire. On the `git commit` hot path under
/// MLP2-005 phase 3 the effective wait can compound: the hook's daemon RPC has
/// its own ~2s socket timeout, and on a wedged lock the hook then falls back to
/// the embedded writer, which waits this bound against the *same* lock — so the
/// worst-case commit hang is roughly `2s + DEFAULT_LOCK_ACQUIRE_TIMEOUT`. (Note: `flock`
/// is resolved locally by the kernel even on NFS mounts — it does not route
/// through the NFS lock server — so a network pause manifests as a hung syscall,
/// not repeated `WouldBlock` retries; the wedge case this bounds is a live local
/// holder.) Operators on unusual storage or very high parallel-worktree volume
/// can override this via the [`LOCK_TIMEOUT_ENV`] environment variable — resolved
/// by the caller with [`lock_timeout_from_env`] and passed to
/// [`WitnessWriter::append_chained_with_lock_timeout`]. (A non-blocking daemon leg
/// is still tracked separately on CIB-124.)
pub const DEFAULT_LOCK_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
/// Environment variable to override [`DEFAULT_LOCK_ACQUIRE_TIMEOUT`] (whole
/// seconds). Read by the CLI/daemon callers, not this crate — see
/// [`lock_timeout_from_env`].
pub const LOCK_TIMEOUT_ENV: &str = "ANVIL_WITNESS_LOCK_TIMEOUT";
/// Initial poll interval for the bounded lock acquire; doubles up to
/// [`LOCK_RETRY_BACKOFF_MAX`] so contention resolves fast without a busy-spin.
const LOCK_RETRY_BACKOFF_START: Duration = Duration::from_millis(5);
/// Cap for the exponential backoff poll interval; see [`LOCK_RETRY_BACKOFF_START`].
const LOCK_RETRY_BACKOFF_MAX: Duration = Duration::from_millis(100);

/// Resolve a lock-acquire timeout from a raw [`LOCK_TIMEOUT_ENV`] value (whole
/// seconds). `None`/blank → [`DEFAULT_LOCK_ACQUIRE_TIMEOUT`]; a valid positive
/// integer → that many seconds; anything else → `Err(raw)` so the caller can warn
/// and fall back (never a silent default on a malformed value).
///
/// Kept pure — it does **not** read the environment — so it is unit-testable and
/// so this low-level crate stays free of env/logging concerns; the CLI and daemon
/// read the env and log the warning on `Err`.
///
/// # Errors
/// Returns the offending (trimmed) value when it is not a positive integer.
pub fn lock_timeout_from_env(raw: Option<&str>) -> Result<Duration, String> {
    match raw.map(str::trim) {
        None | Some("") => Ok(DEFAULT_LOCK_ACQUIRE_TIMEOUT),
        Some(v) => match v.parse::<u64>() {
            Ok(secs) if secs > 0 => Ok(Duration::from_secs(secs)),
            _ => Err(v.to_string()),
        },
    }
}

/// Filename of the durable "chain has been initialised" marker (CIB-126), a
/// sibling of `.lock` in the witness root. Its **presence** distinguishes a chain
/// that was already seeded from a genuinely fresh repo: an empty-or-absent
/// `active.ndjson` with no archives is otherwise indistinguishable from a new repo,
/// so without this marker a truncated-to-zero OR deleted active file would silently
/// reseed genesis over erased history. With the marker present, that state is
/// refused as `ChainBroken`
/// (ADR-038: never reseed). It is not `.ndjson`, so `witness_paths` skips it — it
/// never participates in the chain walk. (Deleting the marker AND truncating the
/// active file is a stronger, deliberate attack; this closes the accidental /
/// simple-truncation silent-reseed, complementing the non-empty-unparseable
/// hardening.)
const CHAIN_MARKER_FILE: &str = ".chain-initialised";
/// Body written into [`CHAIN_MARKER_FILE`] — a small identifiable sentinel with a
/// version for forward-compatibility. Only the file's presence is load-bearing.
const CHAIN_MARKER_BODY: &[u8] = b"anvil-witness-chain v1\n";

#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("serde_json: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("witness chain is corrupted: {0}")]
    Corruption(String),
    #[error("witness chain integrity check failed (broken chain); refusing to append")]
    ChainBroken,
    #[error("timed out after {0:?} waiting for the witness lock (it was held past the timeout)")]
    LockTimeout(Duration),
    #[error("witness root is a symlink; refusing to write: {path}")]
    SymlinkRoot { path: PathBuf },
    #[error(
        "scope mismatch: writer is configured for `{writer_scope}` but line.scope is `{line_scope}`"
    )]
    ScopeMismatch {
        writer_scope: String,
        line_scope: String,
    },
}

/// RAII guard for the witness `.lock` flock (CIB-124). Releasing on `Drop` — which
/// runs on a normal return **and** on an unwinding panic in the critical section —
/// makes the release explicit rather than relying on the fd being dropped, so a
/// future refactor cannot leave the lock held past its scope.
#[derive(Debug)]
struct LockGuard {
    file: File,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Best-effort explicit unlock. The OS also releases the flock when the fd
        // closes (on this `File`'s own drop, immediately after), so an error here
        // is harmless — the lock is released either way.
        let _ = FileExt::unlock(&self.file);
    }
}

/// Whether `err` is the "lock is contended" error a `try_lock_*` returns while
/// another writer holds the flock. Compares the raw OS error against `fs2`'s own
/// contention error — the portable, Rust-version-agnostic signal fs2 exposes for
/// exactly this — rather than the `io::ErrorKind::WouldBlock` mapping, which is
/// only wired for Windows `ERROR_LOCK_VIOLATION` on newer toolchains (CIB-124).
fn is_lock_contended(err: &io::Error) -> bool {
    err.raw_os_error() == fs2::lock_contended_error().raw_os_error()
}

/// Threshold policy for active-file rollover.
///
/// Rollover fires when the active file crosses **either** threshold,
/// whichever happens first (ADR-037 §D-2). The check runs inside the
/// flock, so concurrent writers cannot race a half-archive into
/// existence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RolloverPolicy {
    /// Maximum lines per active file before rollover.
    pub max_lines: u64,
    /// Maximum bytes per active file before rollover.
    pub max_bytes: u64,
}

impl Default for RolloverPolicy {
    fn default() -> Self {
        Self {
            // Spec defaults: 1000 lines or 1 MB whichever first.
            max_lines: 1000,
            max_bytes: 1_048_576,
        }
    }
}

impl RolloverPolicy {
    /// Build a tighter policy useful for tests so rollover happens
    /// without writing a megabyte of synthetic data.
    pub const fn tight(max_lines: u64, max_bytes: u64) -> Self {
        Self {
            max_lines,
            max_bytes,
        }
    }
}

/// The verifiable head of the chain, as read under the writer's flock by
/// [`WitnessWriter::read_chain_head`] / [`WitnessWriter::append_chained`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainHead {
    /// No verifiable segment exists yet — the caller's genesis seed runs before
    /// the first real line.
    Empty,
    /// A verifiable tip: a new line uses `seq` and chains off `prev_line_hash`.
    Healthy {
        /// Sequence number the next appended line must carry (`line_count + 1`).
        seq: u64,
        /// Hash of the current tip line — the next line's `prev_line_hash`.
        prev_line_hash: String,
    },
}

/// Flock-serialised append-only writer for the witness chain.
///
/// Construct with [`WitnessWriter::open`]; one writer instance per `anvil/`
/// root. The writer holds NO long-lived locks — each [`WitnessWriter::append`]
/// call takes the
/// flock for the duration of the append + rollover decision and
/// releases it before returning. This avoids the classic "hold-the-
/// lock-while-the-process-stalls" hazard at the cost of one
/// `flock` syscall per line. The hook surface (MLP-003) writes one
/// line per commit, so the cost is paid at human cadence.
#[derive(Debug)]
pub struct WitnessWriter {
    root: PathBuf,
    scope: String,
    policy: RolloverPolicy,
}

impl WitnessWriter {
    /// `root` is the workspace root; the writer creates the
    /// `anvil/witness/` tree under it on first append.
    pub fn open(
        root: impl Into<PathBuf>,
        scope: impl Into<String>,
        policy: RolloverPolicy,
    ) -> Result<Self, WriterError> {
        let writer = Self {
            root: root.into(),
            scope: scope.into(),
            policy,
        };
        writer.ensure_tree()?;
        Ok(writer)
    }

    /// Append `line` to the active file under flock, performing
    /// rollover if the policy fires after the append.
    ///
    /// `line.prev_line_hash` must already be set by the caller to
    /// either the genesis anchor (for the first line) or the SHA-256
    /// of the immediately-prior line's canonical bytes. The writer
    /// does NOT mutate the line — chaining is the caller's
    /// responsibility, because the caller has visibility into the
    /// commit semantics (e.g. a merge commit needs `prev_line_hashes[]`
    /// rather than a single `prev_line_hash`).
    ///
    /// Returns the new active file's line count after the append, and
    /// the archive path if a rollover happened.
    ///
    /// This is the low-level primitive: the caller has already derived
    /// `(seq, prev_line_hash)` from the chain head. When that derivation and the
    /// append must be atomic against concurrent writers (a daemon and an embedded
    /// fallback, or concurrent worktree hooks), use [`WitnessWriter::append_chained`]
    /// instead — it reads the head and appends under a single flock hold.
    ///
    /// The flock acquire uses [`DEFAULT_LOCK_ACQUIRE_TIMEOUT`] and does **not**
    /// consult the [`LOCK_TIMEOUT_ENV`] operator override — that override is a
    /// property of the atomic append path; route production writes through
    /// [`WitnessWriter::append_chained_with_lock_timeout`] to honour it.
    pub fn append(&self, line: &WitnessLine) -> Result<AppendOutcome, WriterError> {
        // Fail fast on a misrouted line BEFORE acquiring the shared flock — a
        // scope mismatch is a caller bug, not something to take the lock for.
        // (`append_locked` re-checks as defence-in-depth.)
        if line.scope != self.scope {
            return Err(WriterError::ScopeMismatch {
                writer_scope: self.scope.clone(),
                line_scope: line.scope.clone(),
            });
        }
        // The guard releases the flock on drop (end of scope), including on a
        // panic inside `append_locked` (CIB-124).
        let _guard = self.acquire_lock()?;
        self.append_locked(line)
    }

    /// Atomically read the verifiable chain head and append the line that
    /// extends it, under a **single** flock hold (MLP2-005).
    ///
    /// This closes the read-head→append TOCTOU that [`append`](Self::append)
    /// leaves open: with `append`, a caller derives `(seq, prev)` from the chain
    /// *before* taking the lock, so two writers can read the same tip and fork the
    /// chain (same `seq`/`prev`) without corrupting the file. `append_chained`
    /// holds the flock across the whole read-head → derive → append sequence, so
    /// any second writer blocks until the first has extended the chain.
    ///
    /// - On a verifiable tip, `build(seq, prev_line_hash)` produces the line.
    /// - On an empty chain, `seed_genesis()` runs first (under the same lock); the
    ///   head is re-read, then `build` chains off the seeded genesis.
    /// - A chain that exists but fails verification yields
    ///   [`WriterError::ChainBroken`] — it is **never** reseeded (ADR-038).
    ///
    /// Returns the appended (non-genesis) line's hash.
    pub fn append_chained<G, F>(&self, seed_genesis: G, build: F) -> Result<LineHash, WriterError>
    where
        G: FnOnce() -> WitnessLine,
        F: FnOnce(u64, String) -> WitnessLine,
    {
        self.append_chained_with_lock_timeout(seed_genesis, build, DEFAULT_LOCK_ACQUIRE_TIMEOUT)
    }

    /// As [`append_chained`](Self::append_chained), but with the flock-acquire
    /// timeout supplied by the caller (CIB-124 env override). The CLI/daemon
    /// resolve `lock_timeout` from [`LOCK_TIMEOUT_ENV`] via [`lock_timeout_from_env`]
    /// so operators can tune the bound; everything else is identical.
    pub fn append_chained_with_lock_timeout<G, F>(
        &self,
        seed_genesis: G,
        build: F,
        lock_timeout: Duration,
    ) -> Result<LineHash, WriterError>
    where
        G: FnOnce() -> WitnessLine,
        F: FnOnce(u64, String) -> WitnessLine,
    {
        // The guard releases the flock on drop (end of scope), including on a
        // panic inside the closures / `append_chained_locked` (CIB-124).
        let _guard = self.acquire_lock_with_timeout(lock_timeout)?;
        self.append_chained_locked(seed_genesis, build)
    }

    fn append_chained_locked<G, F>(
        &self,
        seed_genesis: G,
        build: F,
    ) -> Result<LineHash, WriterError>
    where
        G: FnOnce() -> WitnessLine,
        F: FnOnce(u64, String) -> WitnessLine,
    {
        let (seq, prev) = match self.read_chain_head()? {
            ChainHead::Healthy {
                seq,
                prev_line_hash,
            } => (seq, prev_line_hash),
            ChainHead::Empty => {
                // Fresh chain — seed genesis under the held lock, then chain off
                // the just-written tip (re-read, still under the same lock).
                self.append_locked(&seed_genesis())?;
                // We just wrote genesis. If it does not read back as a healthy
                // 1-line chain, that is a write-pipeline failure (disk /
                // serialisation), NOT tampering — surface it as `Corruption` so the
                // caller maps it to a generic write failure, never as `ChainBroken`
                // (which would block the commit with a "do not reseed" tamper
                // message for a chain we just authored).
                let reread = self.read_chain_head().map_err(|err| match err {
                    WriterError::ChainBroken => WriterError::Corruption(
                        "genesis failed to re-verify immediately after seeding".to_string(),
                    ),
                    other => other,
                })?;
                match reread {
                    ChainHead::Healthy {
                        seq,
                        prev_line_hash,
                    } => (seq, prev_line_hash),
                    ChainHead::Empty => {
                        return Err(WriterError::Corruption(
                            "genesis append left the chain empty".to_string(),
                        ));
                    }
                }
            }
        };
        // CIB-126: the chain is now known-initialised (freshly seeded above, or an
        // existing Healthy tip). Persist the marker (idempotent; backfills chains
        // created before the marker existed) so a later zero-byte active is refused,
        // not reseeded. Under the flock, so it is atomic with the head read/append.
        self.ensure_chain_marker()?;
        let line = build(seq, prev);
        // Hash the canonical bytes before the write so a serialise failure
        // surfaces here; the writer serialises the same bytes on disk.
        let canonical = line.to_canonical_bytes()?;
        let line_hash = compute_line_hash(&canonical);
        self.append_locked(&line)?;
        Ok(line_hash)
    }

    /// Read and verify the chain head from disk. Walks `witness_paths` so archive
    /// segments participate across rollover boundaries.
    ///
    /// `Empty` → no chain yet (the caller seeds genesis); `Healthy` → the tip to
    /// chain off; [`WriterError::ChainBroken`] → a chain exists but fails
    /// verification (ADR-038: refuse, do **not** reseed).
    ///
    /// **`pub(crate)` by design.** For an atomic head-then-append the flock must
    /// be held across *both* — [`append_chained`](Self::append_chained) does that.
    /// Exposing a standalone public read would let an out-of-crate caller rebuild
    /// the read-head-then-`append` pattern outside the lock — the very TOCTOU this
    /// module closes. The daemon witness leg must route through `append_chained`.
    pub(crate) fn read_chain_head(&self) -> Result<ChainHead, WriterError> {
        let paths = crate::paths::witness_paths(&self.root);
        if paths.is_empty() {
            // No segment files at all. CIB-126: if the chain-init marker says this
            // chain was seeded before, the active file was *deleted* (not just
            // truncated) — refuse rather than reseed genesis over erased history
            // (ADR-038). No marker ⇒ genuinely fresh repo.
            return if self.chain_marker_exists() {
                Err(WriterError::ChainBroken)
            } else {
                Ok(ChainHead::Empty)
            };
        }
        let path_refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        match crate::verify::verify_chain_dag(&path_refs) {
            Ok(dag) if dag.line_count == 0 => {
                // Zero parseable lines. A genuinely fresh start has only absent or
                // zero-byte segment files. But if a segment exists and is
                // non-empty, its bytes are unrecognisable to the verifier — treat
                // that as a broken chain, NOT Empty, so a corrupt/garbled active
                // file can never trigger a silent genesis reseed over real history
                // (ADR-038).
                let any_nonempty = paths
                    .iter()
                    .any(|p| fs::metadata(p).is_ok_and(|m| m.len() > 0));
                if any_nonempty {
                    Err(WriterError::ChainBroken)
                } else if self.chain_marker_exists() {
                    // CIB-126: all segments are present-but-empty, but the marker
                    // says this chain was seeded before — the active file was
                    // truncated to zero with no archives to fall back on. Refuse
                    // rather than reseed genesis over the erased history (ADR-038).
                    Err(WriterError::ChainBroken)
                } else {
                    // No marker ⇒ genuinely fresh repo. Seed genesis.
                    Ok(ChainHead::Empty)
                }
            }
            Ok(dag) => {
                let seq = dag.line_count.saturating_add(1);
                let prev_line_hash = dag.tip_hash.unwrap_or_else(|| {
                    crate::genesis::GenesisAnchor::Fresh
                        .anchor_string()
                        .to_string()
                });
                Ok(ChainHead::Healthy {
                    seq,
                    prev_line_hash,
                })
            }
            Err(_) => Err(WriterError::ChainBroken),
        }
    }

    /// Acquire the exclusive flock with the [`DEFAULT_LOCK_ACQUIRE_TIMEOUT`].
    fn acquire_lock(&self) -> Result<LockGuard, WriterError> {
        self.acquire_lock_with_timeout(DEFAULT_LOCK_ACQUIRE_TIMEOUT)
    }

    /// Acquire the exclusive flock, refusing symlinks at every path we are about to
    /// write through (the witness root, the lock file, the active file, and the
    /// chain-init marker).
    ///
    /// CIB-124: retries `try_lock_exclusive` with capped backoff until it wins the
    /// lock or `timeout` elapses, returning [`WriterError::LockTimeout`] rather
    /// than blocking indefinitely — a stalled holder (or an NFS lock-server hang)
    /// no longer wedges a concurrent `git commit`. The lock is released by the
    /// returned [`LockGuard`] on drop (including on panic). `timeout` is a
    /// parameter so the timeout path is testable without a multi-second wait.
    fn acquire_lock_with_timeout(&self, timeout: Duration) -> Result<LockGuard, WriterError> {
        refuse_if_symlink(&self.witness_root())?;
        let lock_path = self.lock_path();
        refuse_if_symlink(&lock_path)?;
        refuse_if_symlink(&self.active_path())?;
        // CIB-126: refuse a symlink squatted at the chain-init marker path here —
        // before any head read or genesis seed — so a symlinked marker can neither
        // silently disable the protection (a dangling link the write path would
        // swallow) nor be read as a false "present" marker.
        refuse_if_symlink(&self.chain_marker_path())?;

        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;

        let deadline = Instant::now() + timeout;
        let mut backoff = LOCK_RETRY_BACKOFF_START;
        loop {
            match lock_file.try_lock_exclusive() {
                Ok(()) => return Ok(LockGuard { file: lock_file }),
                // Contended — another writer holds it. Wait (bounded) and retry.
                // Compare the raw OS error against `fs2`'s own contention error
                // rather than `err.kind()`: on Windows the `ERROR_LOCK_VIOLATION`
                // → `WouldBlock` `ErrorKind` mapping only exists on newer Rust, so
                // a `kind()` check would silently fall through to a hard `Io` error
                // (making the retry loop dead) on an older toolchain.
                Err(err) if is_lock_contended(&err) => {
                    let now = Instant::now();
                    if now >= deadline {
                        return Err(WriterError::LockTimeout(timeout));
                    }
                    // Bound the sleep by the remaining budget. This does not make
                    // the *acquire* exact — after the sleep the loop makes one more
                    // `try_lock_exclusive` attempt (which may land just past the
                    // deadline) before the next iteration returns `LockTimeout`;
                    // that extra attempt is harmless and grabs the lock if it just
                    // freed.
                    std::thread::sleep(backoff.min(deadline - now));
                    backoff = (backoff * 2).min(LOCK_RETRY_BACKOFF_MAX);
                }
                Err(err) => return Err(WriterError::Io(err)),
            }
        }
    }

    /// Append one line; **the flock must already be held**. Scope-checks the line
    /// (a misrouted hook must not push into the wrong archive scope), writes +
    /// `sync_all`s, then applies the rollover policy.
    fn append_locked(&self, line: &WitnessLine) -> Result<AppendOutcome, WriterError> {
        if line.scope != self.scope {
            return Err(WriterError::ScopeMismatch {
                writer_scope: self.scope.clone(),
                line_scope: line.scope.clone(),
            });
        }

        let active_path = self.active_path();
        let bytes = line.to_ndjson_line()?;
        let mut active = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&active_path)?;
        active.write_all(&bytes)?;
        active.sync_all()?;

        // Decide on rollover. Cheap line count + byte count.
        let size_after = active.metadata()?.len();
        let lines_after = count_lines(&mut active)?;
        let outcome = if lines_after >= self.policy.max_lines || size_after >= self.policy.max_bytes
        {
            let archive_path = self.rollover(&active_path, line.seq)?;
            AppendOutcome {
                active_lines: 0,
                active_bytes: 0,
                rolled_over_to: Some(archive_path),
            }
        } else {
            AppendOutcome {
                active_lines: lines_after,
                active_bytes: size_after,
                rolled_over_to: None,
            }
        };
        Ok(outcome)
    }

    fn rollover(&self, active_path: &Path, seq_at_rollover: u64) -> Result<PathBuf, WriterError> {
        // Compute a content-addressed name for the archive so two
        // mirrored repos produce the same archive filename if they
        // share the same content. `merkle` here is just SHA-256 of
        // the active file bytes — sufficient for content addressing.
        let mut bytes = Vec::new();
        let mut active = File::open(active_path)?;
        active.read_to_end(&mut bytes)?;
        let merkle = hex::encode(Sha256::digest(&bytes));
        let archive_dir = self.witness_root().join("archive");
        fs::create_dir_all(&archive_dir)?;
        let archive_name = format!(
            "{scope}-{seq:020}-{merkle}.ndjson",
            scope = self.scope,
            seq = seq_at_rollover,
            merkle = &merkle[..16],
        );
        let archive_path = archive_dir.join(archive_name);

        // Content-addressed naming means two writers producing
        // identical content would compute the same archive name. On
        // POSIX `fs::rename` silently replaces the existing file; on
        // Windows it fails with AlreadyExists. Both behaviours are
        // wrong for our use case: we want the rollover to be
        // idempotent (the archive already exists with the same
        // content, so we just need to remove the active file). We
        // verify the destination's content matches before treating it
        // as a no-op so a stale or corrupt file at the destination is
        // never silently accepted.
        match fs::rename(active_path, &archive_path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let existing = fs::read(&archive_path).map_err(WriterError::Io)?;
                if existing == bytes {
                    // Content matches — safe to drop the active file.
                    fs::remove_file(active_path)?;
                } else {
                    return Err(WriterError::Corruption(format!(
                        "archive {} exists with different content; refusing to overwrite",
                        archive_path.display(),
                    )));
                }
            }
            Err(e) => return Err(e.into()),
        }

        // MLP2-012: record the rollover in the manifest stream so
        // consumers can tail archive transitions without polling the
        // archive dir. The append is idempotent — re-rolling onto an
        // archive that already exists leaves the manifest with the
        // same single entry.
        #[allow(clippy::naive_bytecount)]
        // Avoid pulling in `bytecount` for a once-per-rollover count.
        let line_count = bytes.iter().filter(|&&b| b == b'\n').count() as u64;
        let entry = crate::manifest::ManifestEntry {
            archive_path: archive_path.clone(),
            merkle,
            line_count,
            seq_at_rollover,
        };
        crate::manifest::append_manifest_entry(&self.witness_root(), &entry)?;

        Ok(archive_path)
    }

    fn ensure_tree(&self) -> Result<(), WriterError> {
        let root = self.witness_root();
        refuse_if_symlink(&root)?;
        fs::create_dir_all(&root)?;
        refuse_if_symlink(&root)?;
        Ok(())
    }
}

/// Refuse to write through a symlink at `path`. The TOCTOU hardening
/// matches MLP-001's pattern: check, create, re-check. Kept as a
/// module-private free function — it doesn't depend on writer state.
///
/// Uses `symlink_metadata` (does NOT follow the link) rather than `path.exists()`
/// (which follows): a **dangling** symlink resolves to a missing target, so
/// `exists()` returns false and would let the link slip through the guard. This
/// hardens every caller — the witness root, `.lock`, active file, and the CIB-126
/// chain-init marker.
fn refuse_if_symlink(path: &Path) -> Result<(), WriterError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(WriterError::SymlinkRoot {
            path: path.to_path_buf(),
        }),
        Ok(_) => Ok(()),                                         // regular file/dir
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()), // absent — fine
        Err(e) => Err(WriterError::Io(e)),
    }
}

impl WitnessWriter {
    pub fn witness_root(&self) -> PathBuf {
        self.root.join("anvil").join("witness")
    }

    pub fn active_path(&self) -> PathBuf {
        // ADR-037 §D-3 pins the active file inside the witness tree
        // at `anvil/witness/active.ndjson`. Keeping it under `witness/`
        // (rather than its sibling at `anvil/witnessed.ndjson`) means
        // the whole chain — active + archives + manifest — lives in
        // one directory that callers can crawl or `git diff` as a unit.
        self.witness_root().join("active.ndjson")
    }

    fn lock_path(&self) -> PathBuf {
        self.witness_root().join(".lock")
    }

    fn chain_marker_path(&self) -> PathBuf {
        self.witness_root().join(CHAIN_MARKER_FILE)
    }

    /// Whether the durable chain-init marker is present (CIB-126). Self-safe: uses
    /// `symlink_metadata` (does NOT follow the link), so a **dangling** symlink
    /// squatted at the path counts as present — never misread as "absent", which
    /// would let the erased-chain path reseed. Only a definitive `NotFound` means
    /// truly absent; any other IO error is treated conservatively as present, so
    /// uncertainty never triggers a silent reseed. (`acquire_lock` also refuses a
    /// symlinked marker up-front; this stays correct independently of that.)
    fn chain_marker_exists(&self) -> bool {
        !matches!(
            fs::symlink_metadata(self.chain_marker_path()),
            Err(e) if e.kind() == io::ErrorKind::NotFound
        )
    }

    /// Idempotently write the chain-init marker under the held flock (CIB-126).
    /// A no-op once present, so it is written once at genesis and **backfilled** on
    /// the next `append_chained` for any chain created before this marker existed.
    /// The one-time `sync_all` makes the marker durable across a crash — the point
    /// of the marker is to survive the same event that truncates the active file.
    fn ensure_chain_marker(&self) -> Result<(), WriterError> {
        let path = self.chain_marker_path();
        // Self-safe: refuse a squatted symlink (dangling included — `refuse_if_symlink`
        // uses `symlink_metadata`) at the marker path, even though `acquire_lock`
        // already guards it upstream. After this passes, `path` is not a symlink, so
        // the `exists()` short-circuit below is reliable.
        refuse_if_symlink(&path)?;
        if path.exists() {
            return Ok(());
        }
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                f.write_all(CHAIN_MARKER_BODY)?;
                f.sync_all()?;
                Ok(())
            }
            // Created between our `exists()` check and here by external, non-flock
            // filesystem activity (a concurrent `append_chained` cannot reach here —
            // the flock serialises us). Benign: the marker is what we wanted.
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(WriterError::Io(e)),
        }
    }
}

/// Result returned by [`WitnessWriter::append`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendOutcome {
    /// Lines remaining in the active file after this append. Zero
    /// when rollover occurred.
    pub active_lines: u64,
    /// Active file size in bytes after the append. Zero when
    /// rollover occurred.
    pub active_bytes: u64,
    /// Archive path written if rollover fired during this append.
    pub rolled_over_to: Option<PathBuf>,
}

/// Count newlines in an open file. Uses a small buffer rather than
/// reading the whole file into memory; witness lines are short and
/// the active file is bounded by the rollover policy, so this is
/// cheap.
fn count_lines(file: &mut File) -> io::Result<u64> {
    file.seek(SeekFrom::Start(0))?;
    let mut buf = [0u8; 4096];
    let mut total: u64 = 0;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            if b == b'\n' {
                total += 1;
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genesis::GenesisAnchor;
    use crate::line::compute_line_hash;
    use tempfile::TempDir;

    fn fresh_line(seq: u64, prev: &str) -> WitnessLine {
        WitnessLine {
            seq,
            scope: "active".to_string(),
            kind: "witness".to_string(),
            prev_line_hash: prev.to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
            commit_sha: Some(format!("commit-{seq}")),
            parent_commits: Vec::new(),
            prev_line_hashes: Vec::new(),
            agent_tag: None,
            rules_sha: None,
            cutoff_commit: None,
            ts: "2026-05-13T00:00:00Z".to_string(),
            validation_at: "pre-commit".to_string(),
        }
    }

    #[test]
    fn append_creates_tree_and_writes_line() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let outcome = writer
            .append(&fresh_line(1, GenesisAnchor::Fresh.anchor_string()))
            .unwrap();
        assert!(outcome.rolled_over_to.is_none());
        assert_eq!(outcome.active_lines, 1);
        assert!(writer.active_path().exists());
        assert!(writer.witness_root().exists());
    }

    #[test]
    fn append_chains_lines() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let first = fresh_line(1, GenesisAnchor::Fresh.anchor_string());
        writer.append(&first).unwrap();
        let first_hash = compute_line_hash(&first.to_canonical_bytes().unwrap());
        let second = fresh_line(2, &first_hash);
        let outcome = writer.append(&second).unwrap();
        assert_eq!(outcome.active_lines, 2);

        let on_disk = fs::read_to_string(writer.active_path()).unwrap();
        let lines: Vec<&str> = on_disk.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn rollover_on_line_count_threshold() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(
            dir.path(),
            "active",
            RolloverPolicy::tight(/* max_lines = */ 3, /* max_bytes = */ 1_000_000),
        )
        .unwrap();

        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        let mut archive_seen = None;
        for seq in 1..=3 {
            let line = fresh_line(seq, &prev);
            let outcome = writer.append(&line).unwrap();
            prev = compute_line_hash(&line.to_canonical_bytes().unwrap());
            if let Some(arch) = outcome.rolled_over_to {
                archive_seen = Some(arch);
            }
        }

        let archive = archive_seen.expect("rollover should have fired on the 3rd append");
        assert!(archive.exists(), "archive path should be present on disk");
        assert!(
            !writer.active_path().exists(),
            "active file is renamed into the archive; next append recreates it"
        );
    }

    #[test]
    fn rollover_on_byte_threshold() {
        let dir = TempDir::new().unwrap();
        // Lines are >100 bytes once serialised, so 200 bytes triggers
        // rollover on the 2nd append.
        let writer = WitnessWriter::open(
            dir.path(),
            "active",
            RolloverPolicy::tight(1_000_000, /* max_bytes = */ 200),
        )
        .unwrap();
        let mut prev = GenesisAnchor::Fresh.anchor_string().to_string();
        let mut saw_rollover = false;
        for seq in 1..=5 {
            let line = fresh_line(seq, &prev);
            let outcome = writer.append(&line).unwrap();
            prev = compute_line_hash(&line.to_canonical_bytes().unwrap());
            if outcome.rolled_over_to.is_some() {
                saw_rollover = true;
                break;
            }
        }
        assert!(
            saw_rollover,
            "byte-size rollover should fire on the 2nd append"
        );
    }

    /// MLP2-012: a tight `RolloverPolicy` produces one manifest entry
    /// per archive in the same order as the rollovers fire. The
    /// manifest's `seq_at_rollover` matches the final `seq` written
    /// before the active file was renamed.
    #[test]
    fn rollover_emits_ordered_manifest_entries() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(
            dir.path(),
            "active",
            RolloverPolicy::tight(/* max_lines = */ 2, /* max_bytes = */ 1_000_000),
        )
        .unwrap();
        let mut prev = crate::genesis::GenesisAnchor::Fresh
            .anchor_string()
            .to_string();
        let mut archive_seqs = Vec::new();
        for seq in 1..=5 {
            let line = fresh_line(seq, &prev);
            let outcome = writer.append(&line).unwrap();
            prev = crate::line::compute_line_hash(&line.to_canonical_bytes().unwrap());
            if let Some(archive) = outcome.rolled_over_to {
                archive_seqs.push((seq, archive));
            }
        }
        // tight policy rolls at line 2 + line 4 -> 2 archives.
        assert_eq!(archive_seqs.len(), 2, "expected 2 rollovers");

        let manifest = crate::manifest::manifest_tail(&writer.witness_root()).unwrap();
        assert_eq!(manifest.len(), 2, "manifest should mirror rollover count");
        for (i, (seq, archive)) in archive_seqs.iter().enumerate() {
            assert_eq!(manifest[i].archive_path, *archive);
            assert_eq!(manifest[i].seq_at_rollover, *seq);
            assert!(
                manifest[i].line_count >= 1,
                "archive must record a non-zero line count",
            );
            assert_eq!(
                manifest[i].merkle.len(),
                64,
                "manifest carries the full SHA-256 hex",
            );
        }
    }

    /// MLP2-012 idempotency: when rollover lands on an archive whose
    /// content matches an existing archive (content-addressed rename
    /// no-op path), the manifest still records exactly one entry per
    /// rollover. Pin against a regression where the no-op rename
    /// branch silently skips the manifest append.
    #[test]
    fn manifest_records_one_entry_per_distinct_archive_even_on_renorm() {
        let dir = TempDir::new().unwrap();
        let writer =
            WitnessWriter::open(dir.path(), "active", RolloverPolicy::tight(2, 1_000_000)).unwrap();
        let mut prev = crate::genesis::GenesisAnchor::Fresh
            .anchor_string()
            .to_string();
        for seq in 1..=4 {
            let line = fresh_line(seq, &prev);
            writer.append(&line).unwrap();
            prev = crate::line::compute_line_hash(&line.to_canonical_bytes().unwrap());
        }
        let initial = crate::manifest::manifest_tail(&writer.witness_root()).unwrap();
        // 4 lines / 2-per-archive -> 2 manifest entries.
        assert_eq!(initial.len(), 2);
    }

    #[test]
    #[cfg(unix)]
    fn refuses_when_witness_root_is_symlink() {
        let dir = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        // Pre-create the anvil/ dir as a regular dir, then put a
        // symlink at anvil/witness/.
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), dir.path().join("anvil").join("witness"))
            .unwrap();
        let err = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap_err();
        assert!(matches!(err, WriterError::SymlinkRoot { .. }));
    }

    // ── MLP2-005: atomic read-head + append ──────────────────────────────────

    fn genesis_seed() -> WitnessLine {
        fresh_line(1, GenesisAnchor::Fresh.anchor_string())
    }

    #[test]
    fn append_chained_seeds_genesis_then_chains_off_it() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();

        // Empty chain: genesis is seeded, then `build` chains off it at seq 2.
        let mut seen = None;
        let hash = writer
            .append_chained(genesis_seed, |seq, prev| {
                seen = Some((seq, prev.clone()));
                fresh_line(seq, &prev)
            })
            .unwrap();

        let (seq, prev) = seen.expect("build ran");
        assert_eq!(seq, 2, "genesis is line 1, so the first real line is seq 2");
        let genesis_hash = compute_line_hash(&genesis_seed().to_canonical_bytes().unwrap());
        assert_eq!(
            prev, genesis_hash,
            "real line must chain off the genesis tip"
        );

        // Two lines on disk, and the returned hash is the real line's hash.
        let on_disk = fs::read_to_string(writer.active_path()).unwrap();
        assert_eq!(on_disk.lines().count(), 2);
        let expected =
            compute_line_hash(&fresh_line(2, &genesis_hash).to_canonical_bytes().unwrap());
        assert_eq!(hash, expected);

        // The whole chain verifies Healthy.
        let paths = crate::paths::witness_paths(dir.path());
        let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        assert_eq!(
            crate::verify::verify_chain_dag(&refs).unwrap().line_count,
            2
        );
    }

    #[test]
    fn append_chained_extends_an_existing_healthy_tip() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .unwrap();

        // Second call must find a Healthy tip — genesis seed must NOT run again.
        let mut seeded = false;
        writer
            .append_chained(
                || {
                    seeded = true;
                    genesis_seed()
                },
                |seq, prev| {
                    assert_eq!(seq, 3, "third line after genesis(1) + first real(2)");
                    fresh_line(seq, &prev)
                },
            )
            .unwrap();
        assert!(!seeded, "genesis must not be reseeded onto a healthy chain");

        let paths = crate::paths::witness_paths(dir.path());
        let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        assert_eq!(
            crate::verify::verify_chain_dag(&refs).unwrap().line_count,
            3
        );
    }

    #[test]
    fn append_chained_refuses_a_broken_chain_without_writing() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .unwrap();

        // Tamper: append a line with a bogus prev-hash link, breaking verification.
        let mut tampered = fresh_line(3, "deadbeef-not-a-real-prev-hash");
        tampered.commit_sha = Some("tampered".into());
        let raw = tampered.to_ndjson_line().unwrap();
        let mut f = OpenOptions::new()
            .append(true)
            .open(writer.active_path())
            .unwrap();
        f.write_all(&raw).unwrap();
        let before = fs::read_to_string(writer.active_path()).unwrap();

        let mut built = false;
        let err = writer
            .append_chained(genesis_seed, |seq, prev| {
                built = true;
                fresh_line(seq, &prev)
            })
            .unwrap_err();

        assert!(matches!(err, WriterError::ChainBroken));
        assert!(
            !built,
            "build must not run on a broken chain (ADR-038: never reseed)"
        );
        assert_eq!(
            fs::read_to_string(writer.active_path()).unwrap(),
            before,
            "a broken-chain refusal must not append anything",
        );
    }

    // ── CIB-124: bounded lock acquire + RAII release ────────────────────────

    #[test]
    fn lock_timeout_from_env_resolves_or_defaults() {
        // Unset / blank → default.
        assert_eq!(
            lock_timeout_from_env(None),
            Ok(DEFAULT_LOCK_ACQUIRE_TIMEOUT)
        );
        assert_eq!(
            lock_timeout_from_env(Some("")),
            Ok(DEFAULT_LOCK_ACQUIRE_TIMEOUT)
        );
        assert_eq!(
            lock_timeout_from_env(Some("   ")),
            Ok(DEFAULT_LOCK_ACQUIRE_TIMEOUT)
        );
        // Valid positive seconds (whitespace tolerated).
        assert_eq!(
            lock_timeout_from_env(Some("10")),
            Ok(Duration::from_secs(10))
        );
        assert_eq!(
            lock_timeout_from_env(Some(" 30 ")),
            Ok(Duration::from_secs(30))
        );
        // Malformed → Err(the offending value), never a silent default.
        assert_eq!(lock_timeout_from_env(Some("0")), Err("0".to_string()));
        assert_eq!(lock_timeout_from_env(Some("abc")), Err("abc".to_string()));
        assert_eq!(lock_timeout_from_env(Some("-5")), Err("-5".to_string()));
        assert_eq!(lock_timeout_from_env(Some("1.5")), Err("1.5".to_string()));
        // Overflow (> u64::MAX) is a parse error, not a silent wrap.
        let huge = "99999999999999999999";
        assert_eq!(lock_timeout_from_env(Some(huge)), Err(huge.to_string()));
    }

    #[test]
    fn append_chained_with_lock_timeout_honours_the_bound() {
        // A held lock makes the injected short timeout fire, mapping to LockTimeout.
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let _held = writer.acquire_lock().expect("hold the lock");
        let err = writer
            .append_chained_with_lock_timeout(
                genesis_seed,
                |seq, prev| fresh_line(seq, &prev),
                Duration::from_millis(150),
            )
            .unwrap_err();
        assert!(matches!(err, WriterError::LockTimeout(_)), "got {err:?}");
    }

    #[test]
    fn acquire_lock_times_out_when_another_writer_holds_it() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();

        // Hold the lock. A second acquire cannot win — it must TIME OUT (bounded),
        // not block indefinitely (the operations-HIGH the old `lock_exclusive` left).
        let held = writer.acquire_lock().expect("first acquire wins");
        let start = Instant::now();
        let err = writer
            .acquire_lock_with_timeout(Duration::from_millis(150))
            .unwrap_err();
        let waited = start.elapsed();
        assert!(matches!(err, WriterError::LockTimeout(_)), "got {err:?}");
        assert!(
            waited >= Duration::from_millis(150) && waited < Duration::from_secs(3),
            "must wait ~the timeout then give up, not hang: waited {waited:?}",
        );

        // Releasing the holder lets a fresh acquire win again.
        drop(held);
        writer
            .acquire_lock()
            .expect("acquire wins once the holder releases");
    }

    #[test]
    fn panic_in_build_closure_releases_the_lock() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        // Seed a healthy chain so the next append reaches the `build` closure.
        writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .unwrap();

        // A panic inside `build` must not leave the flock wedged. This asserts the
        // invariant (lock released on panic-unwind) — held by the `LockGuard`'s
        // `Drop` and, redundantly, by the fd closing as the guard's `File` drops;
        // the test cannot distinguish the two, and does not need to.
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            writer.append_chained(genesis_seed, |_seq, _prev| panic!("boom in build"))
        }));
        assert!(panicked.is_err(), "the closure panic must propagate");

        // If the lock were still held this would time out; it must win promptly.
        writer
            .acquire_lock_with_timeout(Duration::from_millis(500))
            .expect("the lock was released on panic-unwind, so a fresh acquire wins");
    }

    // ── CIB-126: chain-init marker (zero-byte-active reseed detection) ───────

    /// Seed a chain via `append_chained` and return its writer + root.
    fn seed_chain(dir: &TempDir) -> WitnessWriter {
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .unwrap();
        writer
    }

    #[test]
    fn append_chained_writes_the_chain_init_marker() {
        let dir = TempDir::new().unwrap();
        let writer = seed_chain(&dir);
        assert!(
            writer.chain_marker_exists(),
            "seeding a chain must persist the chain-init marker",
        );
        // The marker is NOT part of the chain walk (not `.ndjson`).
        let paths = crate::paths::witness_paths(dir.path());
        assert!(paths.iter().all(|p| !p.ends_with(CHAIN_MARKER_FILE)));
    }

    #[test]
    fn zero_byte_active_after_genesis_is_refused_not_reseeded() {
        let dir = TempDir::new().unwrap();
        let writer = seed_chain(&dir);
        let active = writer.active_path();

        // Simulate the residual: the active file is truncated to zero and there are
        // no archives to fall back on. Before CIB-126 this looked like a fresh repo.
        fs::write(&active, b"").unwrap();
        assert_eq!(fs::metadata(&active).unwrap().len(), 0);

        // The marker (durable, separate inode) survives the truncation, so the next
        // append must REFUSE (ChainBroken), never silently reseed genesis.
        let mut built = false;
        let err = writer
            .append_chained(genesis_seed, |seq, prev| {
                built = true;
                fresh_line(seq, &prev)
            })
            .unwrap_err();
        assert!(matches!(err, WriterError::ChainBroken), "got {err:?}");
        assert!(!built, "build must not run over an erased chain (ADR-038)");
        assert_eq!(
            fs::metadata(&active).unwrap().len(),
            0,
            "a refusal must not write a new genesis",
        );
    }

    #[test]
    fn zero_byte_active_without_marker_is_a_fresh_repo() {
        // A zero-byte active with NO marker is a genuinely fresh repo (e.g. someone
        // `touch`ed the file) — it must seed, not refuse. Guards the fresh path.
        let dir = TempDir::new().unwrap();
        // `open` creates `anvil/witness/` (not the active file), so writing the
        // zero-byte active via `active_path()` reproduces the "touched" precondition.
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        fs::write(writer.active_path(), b"").unwrap();
        assert!(!writer.chain_marker_exists(), "precondition: no marker");
        writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .expect("a marker-less zero-byte active is a fresh repo, must seed");
        assert!(writer.chain_marker_exists(), "seeding writes the marker");
    }

    #[test]
    fn marker_is_backfilled_for_a_pre_marker_healthy_chain() {
        let dir = TempDir::new().unwrap();
        let writer = seed_chain(&dir);
        // Simulate a chain created before the marker existed by deleting it.
        fs::remove_file(writer.chain_marker_path()).unwrap();
        assert!(!writer.chain_marker_exists());

        // A normal append to the still-Healthy chain must succeed AND backfill the
        // marker, so the chain is protected from the next append onward.
        writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .expect("append to a Healthy chain must succeed");
        assert!(
            writer.chain_marker_exists(),
            "the marker must be backfilled for a legacy chain",
        );
    }

    #[test]
    fn deleted_active_after_genesis_is_refused_not_reseeded() {
        // CIB-126 (adversarial F1): the residual covers DELETION too, not just
        // truncation. Remove active.ndjson entirely (no archives) — `witness_paths`
        // is then empty — the surviving marker must still force ChainBroken.
        let dir = TempDir::new().unwrap();
        let writer = seed_chain(&dir);
        fs::remove_file(writer.active_path()).unwrap();
        assert!(crate::paths::witness_paths(dir.path()).is_empty());

        let err = writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .unwrap_err();
        assert!(matches!(err, WriterError::ChainBroken), "got {err:?}");
    }

    #[test]
    fn zero_byte_active_after_rollover_is_healthy_not_broken() {
        // Kernel/code-quality: after a rollover the chain lives in an archive and
        // the active is empty/absent — `read_chain_head` must return Healthy (the
        // archive carries lines), NOT hit the marker branch. Pins that the marker
        // check only fires when EVERY segment is empty.
        let dir = TempDir::new().unwrap();
        // Roll over on every append so history moves to an archive.
        let writer =
            WitnessWriter::open(dir.path(), "active", RolloverPolicy::tight(1, 1_000_000)).unwrap();
        for _ in 0..3 {
            writer
                .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
                .expect("append across rollover");
        }
        // Zero the active file; the archive still holds the chain.
        fs::write(writer.active_path(), b"").unwrap();
        writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .expect("an archived chain with an empty active must stay Healthy");
    }

    #[test]
    #[cfg(unix)]
    fn symlinked_marker_is_refused_not_bypassed() {
        // CIB-126 (adversarial/code-quality F2): a symlink squatted at the marker
        // path — live or dangling — must be refused (SymlinkRoot), never silently
        // treated as "present" (false ChainBroken) or "absent" (silent reseed).
        let dir = TempDir::new().unwrap();
        let writer = seed_chain(&dir); // establishes a real chain + real marker
        fs::remove_file(writer.chain_marker_path()).unwrap();
        // Dangling symlink (target does not exist) — the case `path.exists()` misses.
        std::os::unix::fs::symlink(
            dir.path().join("nonexistent-target"),
            writer.chain_marker_path(),
        )
        .unwrap();

        let err = writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .unwrap_err();
        assert!(
            matches!(err, WriterError::SymlinkRoot { .. }),
            "a symlinked marker must be refused, got {err:?}",
        );
    }

    #[test]
    fn append_chained_concurrent_writers_linearize_one_chain() {
        // The MLP2-005 regression: with the old read-head-then-append (head read
        // OUTSIDE the flock), two writers could read the same tip and fork the
        // chain (duplicate seq/prev). `append_chained` holds the flock across the
        // whole read→append, so N concurrent writers produce ONE linear chain.
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let threads = 4;
        let per_thread = 5;

        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let root = root.clone();
                std::thread::spawn(move || {
                    // Each line is already distinguished by its unique `seq` (derived
                    // under the lock), so the worker needs no per-thread tag.
                    for _ in 0..per_thread {
                        let writer =
                            WitnessWriter::open(&root, "active", RolloverPolicy::default())
                                .unwrap();
                        writer
                            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
                            .unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        // One genesis + every appended line, as a single verifiable chain.
        let paths = crate::paths::witness_paths(&root);
        let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        let dag = crate::verify::verify_chain_dag(&refs)
            .expect("concurrent appends must form one verifiable chain, not a fork");
        assert_eq!(dag.line_count, 1 + threads * per_thread);
    }

    #[test]
    fn append_chained_recovers_the_tip_across_a_real_rollover() {
        // `tight(1, …)` rolls the active file to an archive after every single
        // line, so after the first call the chain lives entirely in archive
        // segments and `active.ndjson` is gone. `read_chain_head` must walk
        // `witness_paths` and recover the archived tip; a second `append_chained`
        // must chain off it, NOT reseed a fresh genesis.
        let dir = TempDir::new().unwrap();
        let writer =
            WitnessWriter::open(dir.path(), "active", RolloverPolicy::tight(1, 1_000_000)).unwrap();

        writer
            .append_chained(genesis_seed, |seq, prev| fresh_line(seq, &prev))
            .unwrap();
        // Genesis(1) + first record(2) each rolled to the archive.
        match writer.read_chain_head().unwrap() {
            ChainHead::Healthy { seq, .. } => assert_eq!(seq, 3),
            ChainHead::Empty => panic!("must recover the archived tip after rollover"),
        }

        writer
            .append_chained(
                || panic!("must not reseed genesis onto an archived chain"),
                |seq, prev| fresh_line(seq, &prev),
            )
            .unwrap();

        let paths = crate::paths::witness_paths(dir.path());
        let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        let dag = crate::verify::verify_chain_dag(&refs)
            .expect("one continuous chain across archive + active after rollover");
        assert_eq!(
            dag.line_count, 3,
            "genesis + 2 records, exactly one genesis"
        );
    }

    #[test]
    fn append_refuses_scope_mismatch_before_locking() {
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        let mut line = fresh_line(1, GenesisAnchor::Fresh.anchor_string());
        line.scope = "other".to_string();
        let err = writer.append(&line).unwrap_err();
        assert!(matches!(err, WriterError::ScopeMismatch { .. }));
    }

    #[test]
    fn read_chain_head_treats_nonempty_unparseable_active_as_broken() {
        // A non-empty active.ndjson that the verifier cannot parse into any line
        // must be ChainBroken, never Empty — otherwise a garbled/truncated-with-
        // junk active would trigger a silent genesis reseed over real history.
        let dir = TempDir::new().unwrap();
        let writer = WitnessWriter::open(dir.path(), "active", RolloverPolicy::default()).unwrap();
        fs::write(
            writer.active_path(),
            b"this is not ndjson witness content\n",
        )
        .unwrap();
        assert!(matches!(
            writer.read_chain_head(),
            Err(WriterError::ChainBroken)
        ));
    }
}
