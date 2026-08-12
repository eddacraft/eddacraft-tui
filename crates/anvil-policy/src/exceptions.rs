use std::path::Path;
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

const EXCEPTION_SCHEMA_VERSION: &str = "anvil.exception.v1";

/// A policy finding that an exception can suppress.
///
/// A plain data struct describing a single violation (rule, file, message,
/// severity, and optional category / fingerprint). It moved here from the
/// deleted OPA `evaluator` module in ADR-098 PR-C; the exceptions store is
/// its only remaining consumer.
#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    pub policy_id: String,
    pub file: String,
    pub message: String,
    pub severity: String,
    pub category: Option<String>,
    pub fingerprint: Option<String>,
}

/// A policy exception that suppresses matching violations.
#[derive(Debug, Clone, Serialize)]
pub struct PolicyException {
    /// Versioned on-disk schema. v0 records without this field deserialize as v1.
    #[serde(default = "default_exception_schema_version")]
    pub schema_version: String,
    /// Stable exception identifier used by future grant/revoke surfaces.
    #[serde(default)]
    pub id: String,
    /// The policy ID to suppress (e.g. "AP-001").
    pub policy_id: String,
    /// Glob-style file pattern (e.g. "src/legacy/**"). Empty means all files.
    #[serde(default)]
    pub file_pattern: String,
    /// Optional stable finding hash for a concrete diagnostic instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding_hash: Option<String>,
    /// Human-readable justification for the exception.
    pub reason: String,
    /// Accountable team or owner for the exception.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Actor who created the exception.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// When the exception was created.
    pub created_at: DateTime<Utc>,
    /// Optional expiry — the exception is ignored after this date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Soft-delete audit trail. Revoked exceptions remain in the tracked store.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked: Option<ExceptionRevocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExceptionRevocation {
    /// When the exception was revoked.
    pub revoked_at: DateTime<Utc>,
    /// Actor who revoked the exception.
    pub revoked_by: String,
    /// Human-readable revocation reason.
    pub reason: String,
}

#[derive(Deserialize)]
struct RawPolicyException {
    #[serde(default = "default_exception_schema_version")]
    schema_version: String,
    #[serde(default)]
    id: String,
    policy_id: String,
    #[serde(default)]
    file_pattern: String,
    #[serde(default)]
    finding_hash: Option<String>,
    reason: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    created_by: Option<String>,
    created_at: DateTime<Utc>,
    #[serde(default)]
    expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    revoked: Option<ExceptionRevocation>,
}

impl<'de> Deserialize<'de> for PolicyException {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawPolicyException::deserialize(deserializer)?;
        let schema_version = if raw.schema_version.is_empty() {
            default_exception_schema_version()
        } else if raw.schema_version == EXCEPTION_SCHEMA_VERSION {
            raw.schema_version
        } else {
            return Err(serde::de::Error::custom(format!(
                "unsupported exception schema_version '{}'; expected '{}'",
                raw.schema_version, EXCEPTION_SCHEMA_VERSION
            )));
        };
        let id = if raw.id.is_empty() {
            exception_id_from_parts(
                &raw.policy_id,
                &raw.file_pattern,
                raw.created_at,
                raw.finding_hash.as_deref(),
            )
        } else {
            raw.id
        };
        Ok(Self {
            schema_version,
            id,
            policy_id: raw.policy_id,
            file_pattern: raw.file_pattern,
            finding_hash: raw.finding_hash,
            reason: raw.reason,
            owner: raw.owner,
            created_by: raw.created_by,
            created_at: raw.created_at,
            expires_at: raw.expires_at,
            revoked: raw.revoked,
        })
    }
}

fn default_exception_schema_version() -> String {
    EXCEPTION_SCHEMA_VERSION.to_string()
}

fn exception_id_from_parts(
    policy_id: &str,
    file_pattern: &str,
    created_at: DateTime<Utc>,
    finding_hash: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(policy_id.as_bytes());
    hasher.update([0]);
    hasher.update(file_pattern.as_bytes());
    hasher.update([0]);
    hasher.update(created_at.to_rfc3339().as_bytes());
    hasher.update([0]);
    if let Some(finding_hash) = finding_hash {
        hasher.update(finding_hash.as_bytes());
    }
    let digest = hasher.finalize();
    format!("exc_{}", hex::encode(&digest[..12]))
}

#[derive(Debug, thiserror::Error)]
pub enum ExceptionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("serialisation error: {0}")]
    Serialise(String),
    /// The in-memory store was loaded from the legacy local path; writing
    /// it to the tracked tree would silently promote local-only data into
    /// git. ADR-073 requires an explicit step — run
    /// [`ExceptionStore::migrate`] first, then reload.
    #[error(
        "store was loaded from the legacy `.anvil/exceptions.json`; run migrate() before \
         writing the tracked store (ADR-073 explicit-migration discipline)"
    )]
    LegacyOriginNotMigrated,
    /// A path component under the workspace's `anvil/` governance tree is
    /// a symlink — refusing to read or write through it (hostile-repo
    /// redirect-outside-worktree gadget; a committed symlink would also
    /// let gate reads consume content from an unreviewed location).
    /// Mirrors `anvil-witness`'s guard.
    #[error("refusing to access symlinked governance path: {path}")]
    SymlinkedPath {
        /// The offending symlinked component.
        path: std::path::PathBuf,
    },
    /// The store file exceeds [`MAX_STORE_BYTES`]. Bounding the read
    /// keeps the L3/L4 gate hot path (EXCEPT-006) from paying an
    /// unbounded allocation for an oversized or maliciously padded
    /// tracked store — the same discipline as
    /// `anvil_config::read_to_string_bounded` (MLP2-063).
    #[error(
        "exception store is {size} bytes; refusing to read past the {MAX_STORE_BYTES}-byte bound"
    )]
    Oversized {
        /// Size of the offending store file in bytes.
        size: u64,
    },
}

/// Upper bound on a readable exception store file. Mirrors
/// `anvil_config::MAX_CONFIG_FILE_BYTES` (1 MiB, MLP2-063) — far above
/// any legitimate store, low enough that gate evaluation never pays an
/// unbounded read.
pub const MAX_STORE_BYTES: u64 = 1024 * 1024;

/// Where a loaded store's data came from. Carried (non-serialised) on
/// [`ExceptionStore`] so write paths can refuse to silently promote
/// legacy-origin data into the tracked tree (ADR-073).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoreSource {
    /// Loaded from the tracked `anvil/exceptions/store.json`.
    Tracked,
    /// Loaded from the legacy `.anvil/exceptions.json` read-fallback.
    Legacy,
    /// No store file existed (or the store was constructed in memory).
    #[default]
    Fresh,
}

/// Outcome of a tracked-store write ([`ExceptionStore::save`] /
/// [`ExceptionStore::update`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum WriteOutcome {
    /// The tracked store was written.
    Written,
    /// The worktree is read-only — the write was skipped. Gate callers
    /// surface this as a warning, never a failure (ADR-002). `detail`
    /// carries the underlying I/O error text: the outcome deliberately
    /// conflates read-only checkouts with permission misconfiguration,
    /// so the diagnostic distinguishing them must stay reachable for
    /// verbose surfaces (2026-06-08 council, EXCEPT-004 contract).
    SkippedReadOnly {
        /// Text of the I/O error that triggered the degrade.
        detail: String,
    },
}

/// Outcome of [`ExceptionStore::migrate`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum MigrateOutcome {
    /// Legacy data was copied into the tracked store.
    Migrated,
    /// Nothing to do: no legacy store, or the tracked store already exists.
    NothingToDo,
    /// The worktree is read-only — the migration was skipped (ADR-002).
    /// `detail` carries the underlying I/O error, mirroring
    /// [`WriteOutcome::SkippedReadOnly`]: the read-only/permission
    /// conflation must stay diagnosable here too.
    SkippedReadOnly {
        /// Text of the I/O error that triggered the degrade.
        detail: String,
    },
}

/// Tracked store path (ADR-073). Exceptions are durable governance state that
/// must travel with the repository and be visible in PR review, so they live
/// under `anvil/`, not the gitignored `.anvil/` runtime tree. Public so gate
/// loaders reading the store from a commit tree (ADR-100) share the one
/// source of truth for the path.
pub const EXCEPTIONS_FILE: &str = "anvil/exceptions/store.json";

/// Legacy local store path. Read-only fallback for repositories written before
/// the ADR-073 migration; [`ExceptionStore::save`] never writes here, and
/// [`ExceptionStore::migrate`] performs the one-time, non-destructive move.
const LEGACY_EXCEPTIONS_FILE: &str = ".anvil/exceptions.json";

/// Name of the advisory lock file inside `anvil/exceptions/`, mirroring
/// the `anvil-witness` writer's flock discipline.
const LOCK_FILE_NAME: &str = ".lock";

/// Persistent store for policy exceptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionStore {
    pub exceptions: Vec<PolicyException>,
    /// Load provenance (EXCEPT-007). Never serialised; defaults to
    /// [`StoreSource::Fresh`] on construction and deserialisation —
    /// [`Self::load`] overwrites it with the real origin.
    #[serde(skip)]
    source: StoreSource,
}

impl ExceptionStore {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            exceptions: Vec::new(),
            source: StoreSource::Fresh,
        }
    }

    /// Where this store's data was loaded from (EXCEPT-007 provenance).
    /// In-memory constructions report [`StoreSource::Fresh`].
    #[must_use]
    pub fn source(&self) -> StoreSource {
        self.source
    }

    /// Loads exceptions, preferring the tracked store
    /// (`{workspace_root}/anvil/exceptions/store.json`) and falling back to the
    /// legacy local store (`{workspace_root}/.anvil/exceptions.json`) for
    /// repositories not yet migrated (ADR-073). The origin is recorded on the
    /// returned store ([`Self::source`]); a legacy-origin store **cannot be
    /// saved** until [`Self::migrate`] runs (no silent promotion into git).
    ///
    /// Read-only: this never writes or migrates. Returns an empty
    /// [`StoreSource::Fresh`] store if neither file exists.
    pub fn load(workspace_root: &Path) -> Result<Self, ExceptionError> {
        // EXCEPT-006 / clawpatch TOCTOU: do not check-then-`File::open` on
        // path strings. Open each governed component with no-follow
        // semantics so a concurrent swap of a checked path for a
        // symlink cannot redirect gate reads to unreviewed content.
        match open_governed_store(workspace_root, GovernedStoreKind::Tracked) {
            Ok(file) => return Self::load_from_file(file, StoreSource::Tracked),
            Err(e) if is_not_found(&e) => {}
            Err(e) => return Err(e),
        }
        match open_governed_store(workspace_root, GovernedStoreKind::Legacy) {
            Ok(file) => return Self::load_from_file(file, StoreSource::Legacy),
            Err(e) if is_not_found(&e) => {}
            Err(e) => return Err(e),
        }
        Ok(Self::empty())
    }

    /// Reads and parses a store from an already-open file handle, tagging
    /// its origin. The read is bounded to [`MAX_STORE_BYTES`] + 1 (a
    /// `Read::take` cap, not a check-then-read on file metadata, so a
    /// file growing between stat and read cannot bypass the bound) and
    /// refuses oversized stores. Callers must open through
    /// [`open_governed_store`] (Unix: per-component `O_NOFOLLOW`) so the
    /// open cannot be retargeted by a post-validation symlink swap.
    fn load_from_file(file: std::fs::File, source: StoreSource) -> Result<Self, ExceptionError> {
        use std::io::Read;
        let mut content = String::new();
        file.take(MAX_STORE_BYTES + 1)
            .read_to_string(&mut content)?;
        let size = content.len() as u64;
        if size > MAX_STORE_BYTES {
            return Err(ExceptionError::Oversized { size });
        }
        let mut store: Self =
            serde_json::from_str(&content).map_err(|e| ExceptionError::Parse(e.to_string()))?;
        store.source = source;
        Ok(store)
    }

    /// Saves exceptions to the tracked store
    /// (`{workspace_root}/anvil/exceptions/store.json`) under an exclusive
    /// flock, with write-temp-then-rename atomicity.
    ///
    /// EXCEPT-007 contract:
    /// - **Refuses legacy-origin data** ([`ExceptionError::LegacyOriginNotMigrated`])
    ///   — run [`Self::migrate`] first, then reload; ADR-073 requires the
    ///   promotion into git to be an explicit step.
    /// - **Refuses symlinked governance paths**
    ///   ([`ExceptionError::SymlinkedPath`]).
    /// - **Read-only worktrees degrade**: returns
    ///   [`WriteOutcome::SkippedReadOnly`] instead of a propagated I/O error,
    ///   so a gate can warn-and-continue (ADR-002).
    ///
    /// Do not compose `load` → mutate → `save` across concurrent callers —
    /// use [`Self::update`], which holds the lock across the full cycle.
    pub fn save(&self, workspace_root: &Path) -> Result<WriteOutcome, ExceptionError> {
        if self.source == StoreSource::Legacy {
            return Err(ExceptionError::LegacyOriginNotMigrated);
        }
        refuse_symlinked_store_paths(workspace_root)?;
        match Self::locked(workspace_root, || self.write_tracked(workspace_root)) {
            Ok(()) => Ok(WriteOutcome::Written),
            Err(e) if is_readonly_io(&e) => Ok(WriteOutcome::SkippedReadOnly {
                detail: e.to_string(),
            }),
            Err(e) => Err(e),
        }
    }

    /// Load-modify-save under a single exclusive flock — the safe CRUD
    /// primitive for the EXCEPT-004 CLI. Two concurrent `update`s cannot
    /// lose each other's writes (the second loads the first's result).
    ///
    /// Same EXCEPT-007 refusals and read-only degrade as [`Self::save`];
    /// on [`WriteOutcome::SkippedReadOnly`] the mutation is discarded.
    pub fn update(
        workspace_root: &Path,
        mutate: impl FnOnce(&mut Self),
    ) -> Result<WriteOutcome, ExceptionError> {
        refuse_symlinked_store_paths(workspace_root)?;
        let result = Self::locked(workspace_root, || {
            let mut store = Self::load(workspace_root)?;
            if store.source == StoreSource::Legacy {
                return Err(ExceptionError::LegacyOriginNotMigrated);
            }
            mutate(&mut store);
            store.write_tracked(workspace_root)
        });
        match result {
            Ok(()) => Ok(WriteOutcome::Written),
            Err(e) if is_readonly_io(&e) => Ok(WriteOutcome::SkippedReadOnly {
                detail: e.to_string(),
            }),
            Err(e) => Err(e),
        }
    }

    /// One-time, non-destructive migration of the legacy local store
    /// (`.anvil/exceptions.json`) to the tracked store
    /// (`anvil/exceptions/store.json`), per ADR-073. This is the explicit
    /// promotion step that [`Self::save`] refuses to perform implicitly.
    ///
    /// Copies the legacy store to the tracked path when the legacy file
    /// exists and the tracked store does not yet. The tracked-store
    /// existence check is re-run **under the flock**, so concurrent
    /// migrations cannot race the exists→write window (one migrates, the
    /// rest see [`MigrateOutcome::NothingToDo`]). The legacy file is
    /// **left in place**; callers decide when to remove it.
    pub fn migrate(workspace_root: &Path) -> Result<MigrateOutcome, ExceptionError> {
        let tracked = workspace_root.join(EXCEPTIONS_FILE);
        let legacy = workspace_root.join(LEGACY_EXCEPTIONS_FILE);
        // Lock-free fast path: nothing to migrate, touch nothing.
        if tracked.exists() || !legacy.exists() {
            return Ok(MigrateOutcome::NothingToDo);
        }
        refuse_symlinked_store_paths(workspace_root)?;
        // 2026-07-04 council: migrate() reads the legacy store, so the
        // read-side legacy guard applies here exactly as it does on
        // load()'s fallback — without it a symlinked `.anvil/` smuggles
        // outside content into the tracked, git-visible store.
        refuse_symlinked_legacy_paths(workspace_root)?;
        let result = Self::locked(workspace_root, || {
            // Re-check under the lock: a concurrent migrate may have won.
            if tracked.exists() {
                return Ok(MigrateOutcome::NothingToDo);
            }
            // Re-open the legacy store under the flock with the same
            // no-follow discipline as load — the pre-lock exists/guard
            // check is not a substitute for a race-safe open.
            let mut store = match open_governed_store(workspace_root, GovernedStoreKind::Legacy) {
                Ok(file) => Self::load_from_file(file, StoreSource::Legacy)?,
                Err(e) if is_not_found(&e) => return Ok(MigrateOutcome::NothingToDo),
                Err(e) => return Err(e),
            };
            // The explicit-migration path is the one place a legacy-origin
            // store may be promoted; re-tag before the tracked write.
            store.source = StoreSource::Tracked;
            store.write_tracked(workspace_root)?;
            Ok(MigrateOutcome::Migrated)
        });
        match result {
            Ok(outcome) => Ok(outcome),
            Err(e) if is_readonly_io(&e) => Ok(MigrateOutcome::SkippedReadOnly {
                detail: e.to_string(),
            }),
            Err(e) => Err(e),
        }
    }

    /// Serialise and atomically write this store to the tracked path.
    /// Callers own refusal checks and locking.
    fn write_tracked(&self, workspace_root: &Path) -> Result<(), ExceptionError> {
        let path = workspace_root.join(EXCEPTIONS_FILE);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ExceptionError::Serialise(e.to_string()))?;
        atomic_write(&path, content.as_bytes())?;
        Ok(())
    }

    /// Run `body` while holding an exclusive flock on
    /// `anvil/exceptions/.lock`, mirroring `anvil-witness::WitnessWriter`:
    /// the lock is held only for the duration of the call, and released
    /// (via close) before returning. flock is per open-file-description,
    /// so this serialises writers across threads **and** processes.
    fn locked<T>(
        workspace_root: &Path,
        body: impl FnOnce() -> Result<T, ExceptionError>,
    ) -> Result<T, ExceptionError> {
        use fs2::FileExt;
        let _process_guard = exception_store_process_lock().lock().map_err(|_| {
            ExceptionError::Io(std::io::Error::other("exception store lock poisoned"))
        })?;
        let dir = workspace_root.join(EXCEPTIONS_FILE);
        let dir = dir.parent().expect("EXCEPTIONS_FILE has a parent");
        std::fs::create_dir_all(dir)?;
        // Check-create-check, mirroring WitnessWriter::ensure_tree: a hostile
        // process could race a symlink into any governed component between the
        // caller's guard and create_dir_all above. Re-check the whole path
        // before opening the lock or writing through it.
        refuse_symlinked_store_paths(workspace_root)?;
        let lock_path = dir.join(LOCK_FILE_NAME);
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;
        let result = body();
        // Unlock explicitly so the success path doesn't depend on Drop
        // order; close would release it regardless.
        let _ = fs2::FileExt::unlock(&lock_file);
        result
    }

    /// Returns only the exceptions that are currently active (not expired).
    #[must_use]
    pub fn active_exceptions(&self) -> Vec<&PolicyException> {
        self.active_exceptions_at(Utc::now())
    }

    /// Returns exceptions active at a specific point in time.
    #[must_use]
    pub fn active_exceptions_at(&self, now: DateTime<Utc>) -> Vec<&PolicyException> {
        self.exceptions
            .iter()
            .filter(|e| !is_revoked(e) && !is_expired(e, now))
            .collect()
    }

    /// Adds a new exception.
    pub fn add(&mut self, mut exception: PolicyException) {
        exception.ensure_schema_defaults();
        self.exceptions.push(exception);
    }

    /// Removes all exceptions for the given policy ID.
    pub fn remove_by_policy(&mut self, policy_id: &str) {
        self.exceptions.retain(|e| e.policy_id != policy_id);
    }
}

fn exception_store_process_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

impl PolicyException {
    /// Whether this exception's **scope** covers a finding identified
    /// by `rule_id`, a workspace-relative `file` path, and an optional
    /// stable finding hash (EXCEPT-006).
    ///
    /// Scope only: revocation, expiry, and attribution are
    /// [`verify_exception_at`]'s job — gate callers check both, so a
    /// revoked grant still *covers* its finding here but must never be
    /// offered to this method's consumers as applicable. A
    /// `finding_hash`-pinned grant covers only a finding reporting the
    /// identical hash; a finding with no hash is never covered by a
    /// pinned grant (fail-safe: instance-scoped grants don't widen).
    /// An unparseable glob covers nothing, mirroring
    /// [`ExceptionVerdict::InvalidScope`].
    #[must_use]
    pub fn covers_finding(&self, rule_id: &str, file: &str, finding_hash: Option<&str>) -> bool {
        if self.policy_id != rule_id {
            return false;
        }
        if let Some(required) = self.finding_hash.as_deref()
            && finding_hash != Some(required)
        {
            return false;
        }
        if !scope_is_valid(&self.file_pattern) {
            return false;
        }
        if self.file_pattern.is_empty() {
            return true;
        }
        glob_matches(&self.file_pattern, file)
    }

    fn ensure_schema_defaults(&mut self) {
        if self.schema_version.is_empty() {
            self.schema_version = default_exception_schema_version();
        }
        if self.id.is_empty() {
            self.id = exception_id_from_parts(
                &self.policy_id,
                &self.file_pattern,
                self.created_at,
                self.finding_hash.as_deref(),
            );
        }
    }
}

/// Refuse to write through a symlink at any component of the tracked
/// store's path (`anvil`, `anvil/exceptions`, the store file, the lock
/// file). A hostile repository shipping a symlinked governance dir would
/// otherwise redirect writes outside the worktree. Mirrors
/// `anvil-witness`'s `refuse_if_symlink` discipline; non-existent
/// components are fine — `create_dir_all` creates real directories.
fn refuse_symlinked_store_paths(workspace_root: &Path) -> Result<(), ExceptionError> {
    let store = workspace_root.join(EXCEPTIONS_FILE);
    let dir = store.parent().expect("EXCEPTIONS_FILE has a parent");
    let anvil = dir.parent().expect("exceptions dir has a parent");
    for path in [anvil, dir, &store, &dir.join(LOCK_FILE_NAME)] {
        // symlink_metadata (not exists()) so a *dangling* symlink — which
        // exists() reports as absent — is still refused.
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(ExceptionError::SymlinkedPath {
                    path: path.to_path_buf(),
                });
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Read-side twin of [`refuse_symlinked_store_paths`] for the legacy
/// local store: refuse a symlinked `.anvil/` directory or
/// `.anvil/exceptions.json` file. The legacy tree is conventionally
/// gitignored, but a hostile repository controls its own `.gitignore`
/// and could commit a symlink here too.
fn refuse_symlinked_legacy_paths(workspace_root: &Path) -> Result<(), ExceptionError> {
    let legacy = workspace_root.join(LEGACY_EXCEPTIONS_FILE);
    let dir = legacy
        .parent()
        .expect("LEGACY_EXCEPTIONS_FILE has a parent");
    for path in [dir, &legacy] {
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(ExceptionError::SymlinkedPath {
                    path: path.to_path_buf(),
                });
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Which on-disk exception store to open under a workspace root.
#[derive(Clone, Copy)]
enum GovernedStoreKind {
    Tracked,
    Legacy,
}

impl GovernedStoreKind {
    fn components(self) -> &'static [&'static str] {
        match self {
            Self::Tracked => &["anvil", "exceptions", "store.json"],
            Self::Legacy => &[".anvil", "exceptions.json"],
        }
    }

    #[cfg(not(unix))]
    fn relative_path(self) -> &'static str {
        match self {
            Self::Tracked => EXCEPTIONS_FILE,
            Self::Legacy => LEGACY_EXCEPTIONS_FILE,
        }
    }
}

/// Whether an error is a missing path component / file (empty-store case).
fn is_not_found(error: &ExceptionError) -> bool {
    matches!(error, ExceptionError::Io(e) if e.kind() == std::io::ErrorKind::NotFound)
}

/// Open a governed exception store relative to `workspace_root` without
/// following any path-component symlink.
///
/// On Unix this walks each component with `openat` + `O_NOFOLLOW` so a
/// concurrent replacement of a checked path for a symlink cannot redirect
/// the read after a path-string guard (TOCTOU). On non-Unix platforms the
/// path-string symlink guard still runs, then a normal open (best-effort;
/// same class as other non-Unix governance opens in this repo).
fn open_governed_store(
    workspace_root: &Path,
    kind: GovernedStoreKind,
) -> Result<std::fs::File, ExceptionError> {
    #[cfg(unix)]
    {
        open_nofollow_components(workspace_root, kind.components())
    }
    #[cfg(not(unix))]
    {
        match kind {
            GovernedStoreKind::Tracked => refuse_symlinked_store_paths(workspace_root)?,
            GovernedStoreKind::Legacy => refuse_symlinked_legacy_paths(workspace_root)?,
        }
        std::fs::File::open(workspace_root.join(kind.relative_path())).map_err(ExceptionError::from)
    }
}

/// Open `components` under `root` with per-hop no-follow semantics.
///
/// Intermediate components open as directories (`O_DIRECTORY |
/// O_NOFOLLOW`); the leaf opens as a regular read (`O_RDONLY |
/// O_NOFOLLOW`). A symlink at any hop surfaces as
/// [`ExceptionError::SymlinkedPath`].
///
/// Uses `nix::fcntl::open`/`openat` so this crate stays under the
/// workspace `forbid(unsafe_code)` lint (same pattern as gate-history
/// and usage-sidecar opens in `anvil-cli`).
#[cfg(unix)]
fn open_nofollow_components(
    root: &Path,
    components: &[&str],
) -> Result<std::fs::File, ExceptionError> {
    use std::os::fd::AsFd;

    use nix::fcntl::{OFlag, open, openat};
    use nix::sys::stat::Mode;

    if components.is_empty() {
        return Err(ExceptionError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "governed store path has no components",
        )));
    }

    // Workspace root is caller-supplied; open it as a directory fd that
    // anchors every subsequent openat. We do not O_NOFOLLOW the root —
    // the operator chose this workspace path intentionally.
    let mut dir = open(
        root,
        OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::from)
    .map_err(ExceptionError::from)?;

    let mut built = root.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        let is_last = index + 1 == components.len();
        built.push(component);
        let flags = if is_last {
            OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC
        } else {
            OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC
        };
        let next = match openat(dir.as_fd(), Path::new(component), flags, Mode::empty()) {
            Ok(fd) => fd,
            Err(nix::Error::ELOOP) => {
                return Err(ExceptionError::SymlinkedPath { path: built });
            }
            // Intermediate symlink + O_DIRECTORY can surface as ENOTDIR
            // when the link itself is not followed; re-check with
            // no-follow metadata and map to SymlinkedPath.
            Err(nix::Error::ENOTDIR) => {
                if std::fs::symlink_metadata(&built).is_ok_and(|m| m.file_type().is_symlink()) {
                    return Err(ExceptionError::SymlinkedPath { path: built });
                }
                return Err(ExceptionError::Io(std::io::Error::from(
                    nix::Error::ENOTDIR,
                )));
            }
            Err(err) => return Err(ExceptionError::Io(std::io::Error::from(err))),
        };
        if is_last {
            return Ok(std::fs::File::from(next));
        }
        dir = next;
    }
    unreachable!("components non-empty; loop always returns on last hop")
}

/// Whether an error is the read-only-worktree class that write paths
/// degrade on (ADR-002: warn, never block) rather than propagate.
fn is_readonly_io(error: &ExceptionError) -> bool {
    use std::io::ErrorKind;
    match error {
        ExceptionError::Io(e) => matches!(
            e.kind(),
            ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem
        ),
        _ => false,
    }
}

/// Atomically write `content` to `path` via a temp file + rename.
fn atomic_write(path: &Path, content: &[u8]) -> Result<(), ExceptionError> {
    use std::io::Write;

    let dir = path.parent().ok_or_else(|| {
        ExceptionError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("no parent directory for {}", path.display()),
        ))
    })?;

    let mut tmp = tempfile::Builder::new()
        .tempfile_in(dir)
        .map_err(ExceptionError::Io)?;

    tmp.write_all(content).map_err(ExceptionError::Io)?;
    tmp.flush().map_err(ExceptionError::Io)?;

    let tmp_path = tmp.into_temp_path();

    // On Windows, TempPath::persist uses std::fs::rename, which fails if the
    // destination already exists. Remove the existing file first.
    #[cfg(windows)]
    {
        if let Err(e) = std::fs::remove_file(path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(ExceptionError::Io(e));
        }
    }

    tmp_path
        .persist(path)
        .map_err(|e| ExceptionError::Io(e.error))?;

    Ok(())
}

/// Checks whether a violation is suppressed by any active exception.
#[must_use]
pub fn is_suppressed(violation: &Violation, exceptions: &[PolicyException]) -> bool {
    is_suppressed_at(violation, exceptions, Utc::now())
}

/// Checks suppression at a specific point in time (useful for testing).
#[must_use]
pub fn is_suppressed_at(
    violation: &Violation,
    exceptions: &[PolicyException],
    now: DateTime<Utc>,
) -> bool {
    exceptions.iter().any(|ex| {
        if is_revoked(ex) {
            return false;
        }
        if is_expired(ex, now) {
            return false;
        }
        if ex.policy_id != violation.policy_id {
            return false;
        }
        if let Some(finding_hash) = ex.finding_hash.as_deref()
            && violation.fingerprint.as_deref() != Some(finding_hash)
        {
            return false;
        }
        // An unparseable scope must not suppress. Classify it explicitly
        // so enforcement and `verify_exception_at`'s `InvalidScope`
        // verdict agree by construction, not by the accident of
        // `glob_matches`'s error arm. Keep the diagnostic the
        // short-circuit would otherwise swallow (council EXCEPT-005).
        if !scope_is_valid(&ex.file_pattern) {
            eprintln!(
                "warning: invalid exception glob pattern '{}'; exception does not apply",
                ex.file_pattern
            );
            return false;
        }
        // Delegate the scope decision to the shared matcher so the OPA
        // evaluator path and the L3/L4 gate path (EXCEPT-006) cannot
        // drift; the guards above only preserve this arm's diagnostic
        // ordering.
        ex.covers_finding(
            &violation.policy_id,
            &violation.file,
            violation.fingerprint.as_deref(),
        )
    })
}

/// Filters a list of violations, removing any that are suppressed.
#[must_use]
pub fn filter_suppressed(
    violations: Vec<Violation>,
    exceptions: &[PolicyException],
) -> Vec<Violation> {
    violations
        .into_iter()
        .filter(|v| !is_suppressed(v, exceptions))
        .collect()
}

fn is_expired(exception: &PolicyException, now: DateTime<Utc>) -> bool {
    exception.expires_at.is_some_and(|exp| now > exp)
}

fn is_revoked(exception: &PolicyException) -> bool {
    exception.revoked.is_some()
}

/// Glob matching using the `glob` crate's `Pattern` type.
///
/// Normalises path separators to `/` before matching so patterns
/// work consistently across platforms.
fn glob_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.replace('\\', "/");
    let path = path.replace('\\', "/");

    let opts = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    match glob::Pattern::new(&pattern) {
        Ok(p) => p.matches_with(&path, opts),
        Err(e) => {
            eprintln!("warning: invalid exception glob pattern '{pattern}': {e}");
            false
        }
    }
}

/// Verdict for a single exception at evaluation time (EXCEPT-005).
///
/// A single, precedence-ordered classification — the first failing check
/// wins, most-terminal first: an exception that is both revoked and
/// expired reports `Revoked`. The precedence is
/// `Revoked` > `Expired` > `InvalidScope` > `Unattributed` > `Active`:
/// deliberately-dead grants (revoked, then expired) before structural
/// faults (an unparseable scope glob) before trust faults (a v0-shape
/// grant with no attribution) before a clean `Active`.
///
/// This type *classifies*; it does not enforce. Only
/// [`Active`](Self::Active) and [`Unattributed`](Self::Unattributed)
/// [`applies()`](Self::applies); the latter also [`is_downgrade()`](Self::is_downgrade),
/// the signal a consumer uses to surface `warn`/`degraded` instead of a
/// clean `pass` (ADR-073). Acting on that signal — refusing to silently
/// honour an unattributed grant during L3/L4 evaluation — is wired by
/// EXCEPT-006; the legacy [`is_suppressed_at`] path does not yet consult
/// this verdict. Expired / revoked / invalid-scope grants do not apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionVerdict {
    /// Soft-deleted via [`PolicyException::revoked`]; does not apply.
    Revoked,
    /// Past [`PolicyException::expires_at`] at evaluation time; does not apply.
    Expired,
    /// `file_pattern` is not a parseable glob; cannot be safely matched,
    /// so it does not apply.
    InvalidScope,
    /// Valid and in-date but **unattributed** — no `owner` and no
    /// `created_by` (the v0 shape). [`applies()`](Self::applies) is true,
    /// but [`is_downgrade()`](Self::is_downgrade) flags it so a verdict-aware
    /// consumer surfaces `warn`/`degraded` rather than honouring it
    /// silently (the enforcement wiring is EXCEPT-006).
    Unattributed,
    /// Valid, in-date, in-scope, and attributed. Applies cleanly.
    Active,
}

impl ExceptionVerdict {
    /// Whether an exception with this verdict suppresses a matching
    /// finding. `Active` and `Unattributed` apply; the rest do not.
    #[must_use]
    pub fn applies(self) -> bool {
        matches!(self, Self::Active | Self::Unattributed)
    }

    /// Whether this verdict is a downgrade signal: the exception applies
    /// but the evaluation must not report a clean `pass`. True only for
    /// [`Unattributed`](Self::Unattributed).
    #[must_use]
    pub fn is_downgrade(self) -> bool {
        matches!(self, Self::Unattributed)
    }

    /// Stable lowercase token for diagnostics / capsule verdicts.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Revoked => "revoked",
            Self::Expired => "expired",
            Self::InvalidScope => "invalid-scope",
            Self::Unattributed => "unattributed",
            Self::Active => "active",
        }
    }
}

/// An exception paired with its verification verdict.
///
/// Borrows the source record — zero-copy and the full
/// [`PolicyException`] stays in reach (id, reason, owner, expiry, …),
/// so a consumer never has to re-project fields as needs grow.
#[derive(Debug, Clone, Copy)]
pub struct VerifiedException<'a> {
    /// The verified exception.
    pub exception: &'a PolicyException,
    /// Its verdict at the `now` it was verified against.
    pub verdict: ExceptionVerdict,
}

/// Verify one exception at the current time. See [`verify_exception_at`].
#[must_use]
pub fn verify_exception(exception: &PolicyException) -> ExceptionVerdict {
    verify_exception_at(exception, Utc::now())
}

/// Classify `exception`'s validity at `now` into an [`ExceptionVerdict`].
///
/// Validates revocation, expiry, scope-glob well-formedness, and
/// attribution — **not** whether it matches any particular violation
/// (that is [`is_suppressed_at`]). The precedence is documented on
/// [`ExceptionVerdict`].
#[must_use]
pub fn verify_exception_at(exception: &PolicyException, now: DateTime<Utc>) -> ExceptionVerdict {
    if is_revoked(exception) {
        return ExceptionVerdict::Revoked;
    }
    if is_expired(exception, now) {
        return ExceptionVerdict::Expired;
    }
    if !scope_is_valid(&exception.file_pattern) {
        return ExceptionVerdict::InvalidScope;
    }
    if is_unattributed(exception) {
        return ExceptionVerdict::Unattributed;
    }
    ExceptionVerdict::Active
}

/// Verify every exception in `exceptions` at `now`.
#[must_use]
pub fn verify_exceptions_at(
    exceptions: &[PolicyException],
    now: DateTime<Utc>,
) -> Vec<VerifiedException<'_>> {
    exceptions
        .iter()
        .map(|ex| VerifiedException {
            exception: ex,
            verdict: verify_exception_at(ex, now),
        })
        .collect()
}

/// Whether `file_pattern` is a usable scope: empty (= all files) or a
/// well-formed glob. Mirrors [`glob_matches`]'s separator normalisation
/// so the validity check and the match use the same parser.
fn scope_is_valid(file_pattern: &str) -> bool {
    file_pattern.is_empty() || glob::Pattern::new(&file_pattern.replace('\\', "/")).is_ok()
}

/// A grant is unattributed when it carries neither an `owner` nor a
/// `created_by` — the v0 schema shape, which predates ADR-073's
/// attribution requirement. A **blank** (empty, whitespace-only, or
/// invisible-character-only) value counts as absent: `owner: ""` — or
/// `owner: "\u{200B}"` — is attribution in name only and must not let
/// a v0-shape grant masquerade as `Active` (council EXCEPT-005;
/// invisible-unicode hardening 2026-07-04 council).
fn is_unattributed(exception: &PolicyException) -> bool {
    let blank = |s: &String| is_visibly_blank(s);
    exception.owner.as_ref().is_none_or(blank) && exception.created_by.as_ref().is_none_or(blank)
}

/// Whether a string carries no visible content: whitespace, control
/// characters, and zero-width/format characters (zero-width spaces,
/// joiners, bidi marks, soft hyphen, BOM) all count as blank. `trim()`
/// alone misses the format class — `"\u{200B}"` survives it — which
/// let invisible strings masquerade as attribution.
#[must_use]
pub fn is_visibly_blank(s: &str) -> bool {
    s.chars().all(|c| {
        c.is_whitespace()
            || c.is_control()
            || matches!(c,
                '\u{00AD}'
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{2069}'
                | '\u{FEFF}')
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_violation(policy_id: &str, file: &str) -> Violation {
        Violation {
            policy_id: policy_id.to_string(),
            file: file.to_string(),
            message: "test violation".to_string(),
            severity: "warning".to_string(),
            category: None,
            fingerprint: None,
        }
    }

    fn make_exception(policy_id: &str, file_pattern: &str) -> PolicyException {
        let created_at = Utc::now();
        PolicyException {
            schema_version: default_exception_schema_version(),
            id: exception_id_from_parts(policy_id, file_pattern, created_at, None),
            policy_id: policy_id.to_string(),
            file_pattern: file_pattern.to_string(),
            finding_hash: None,
            reason: "legacy code".to_string(),
            owner: None,
            created_by: None,
            created_at,
            expires_at: None,
            revoked: None,
        }
    }

    /// An attributed, in-date, valid-scope grant (an `Active` baseline).
    fn make_attributed(policy_id: &str, file_pattern: &str) -> PolicyException {
        let mut ex = make_exception(policy_id, file_pattern);
        ex.owner = Some("team-platform".to_string());
        ex.created_by = Some("alice@example.test".to_string());
        ex
    }

    // --- EXCEPT-006 read-path hardening ---

    /// A committed symlink at `anvil/exceptions` must refuse the read —
    /// gate verdicts must never consume store content from an
    /// unreviewed redirect target (read-side twin of the EXCEPT-007
    /// write guard).
    #[cfg(unix)]
    #[test]
    fn load_refuses_symlinked_exceptions_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("store.json"), r#"{"exceptions":[]}"#).unwrap();
        std::fs::create_dir_all(root.join("anvil")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("anvil/exceptions")).unwrap();
        let err = ExceptionStore::load(root).expect_err("symlinked dir must refuse");
        assert!(
            matches!(err, ExceptionError::SymlinkedPath { .. }),
            "{err:?}"
        );
    }

    /// A symlinked legacy `.anvil/exceptions.json` must refuse the
    /// read-fallback for the same reason.
    #[cfg(unix)]
    #[test]
    fn load_refuses_symlinked_legacy_store() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let outside = tmp.path().join("outside.json");
        std::fs::write(&outside, r#"{"exceptions":[]}"#).unwrap();
        std::fs::create_dir_all(root.join(".anvil")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join(".anvil/exceptions.json")).unwrap();
        let err = ExceptionStore::load(root).expect_err("symlinked legacy must refuse");
        assert!(
            matches!(err, ExceptionError::SymlinkedPath { .. }),
            "{err:?}"
        );
    }

    /// Leaf symlink at the tracked store file must refuse — and must
    /// never surface the external target's exceptions (the classic
    /// check-then-open race: after a path-string guard, a concurrent
    /// swap of store.json for a symlink would feed unreviewed content
    /// into gate decisions if the open followed links).
    #[cfg(unix)]
    #[test]
    fn load_refuses_symlinked_store_file_without_reading_target() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let outside = tmp.path().join("outside-store.json");
        // External content with a grant that must never be applied.
        std::fs::write(
            &outside,
            r#"{"exceptions":[{"policy_id":"AP-EVIL","file_pattern":"","reason":"smuggled","created_at":"2020-01-01T00:00:00Z"}]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("anvil/exceptions")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("anvil/exceptions/store.json")).unwrap();

        let err = ExceptionStore::load(root).expect_err("symlinked store leaf must refuse");
        assert!(
            matches!(err, ExceptionError::SymlinkedPath { .. }),
            "expected SymlinkedPath, got {err:?}"
        );
        // Defence-in-depth: if a future regression followed the link and
        // returned Ok, the smuggled grant would appear here.
        if let Ok(store) = ExceptionStore::load(root) {
            assert!(
                store.exceptions.is_empty(),
                "must not load external symlink target content"
            );
        }
    }

    /// Intermediate-component no-follow: even when `store.json` is a
    /// real file, a symlinked parent directory must not let load read
    /// through to an external tree (openat + `O_NOFOLLOW` on the dir hop).
    #[cfg(unix)]
    #[test]
    fn load_refuses_symlinked_anvil_dir_without_reading_target() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let outside = tmp.path().join("outside-tree");
        std::fs::create_dir_all(outside.join("exceptions")).unwrap();
        std::fs::write(
            outside.join("exceptions/store.json"),
            r#"{"exceptions":[{"policy_id":"AP-EVIL","file_pattern":"","reason":"smuggled","created_at":"2020-01-01T00:00:00Z"}]}"#,
        )
        .unwrap();
        std::os::unix::fs::symlink(&outside, root.join("anvil")).unwrap();

        let err = ExceptionStore::load(root).expect_err("symlinked anvil/ must refuse");
        assert!(
            matches!(err, ExceptionError::SymlinkedPath { .. }),
            "expected SymlinkedPath, got {err:?}"
        );
    }

    /// Happy path still binds a real tracked store through the no-follow
    /// ladder (regression guard for the openat wiring).
    #[cfg(unix)]
    #[test]
    fn load_opens_real_tracked_store_via_nofollow_ladder() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("anvil/exceptions")).unwrap();
        std::fs::write(
            root.join("anvil/exceptions/store.json"),
            r#"{"exceptions":[{"policy_id":"AP-001","file_pattern":"src/**","reason":"ok","created_at":"2020-01-01T00:00:00Z"}]}"#,
        )
        .unwrap();
        let store = ExceptionStore::load(root).expect("real store must load");
        assert_eq!(store.source(), StoreSource::Tracked);
        assert_eq!(store.exceptions.len(), 1);
        assert_eq!(store.exceptions[0].policy_id, "AP-001");
    }

    /// A store past [`MAX_STORE_BYTES`] refuses before allocating —
    /// the gate hot path never pays an unbounded read (MLP2-063
    /// discipline).
    #[test]
    fn load_refuses_oversized_store() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("anvil/exceptions")).unwrap();
        let padding = "x".repeat(usize::try_from(MAX_STORE_BYTES).unwrap() + 1);
        std::fs::write(root.join("anvil/exceptions/store.json"), padding).unwrap();
        let err = ExceptionStore::load(root).expect_err("oversized store must refuse");
        assert!(matches!(err, ExceptionError::Oversized { .. }), "{err:?}");
    }

    // --- EXCEPT-004 hardening (2026-07-04 council) ---

    /// Zero-width/format characters must not masquerade as attribution:
    /// a grant whose owner is a zero-width space is Unattributed.
    #[test]
    fn invisible_unicode_owner_is_unattributed() {
        let mut ex = make_exception("AP-001", "");
        ex.owner = Some("\u{200B}\u{2060}".to_string());
        ex.created_by = Some("\u{FEFF}".to_string());
        assert_eq!(verify_exception(&ex), ExceptionVerdict::Unattributed);
        assert!(is_visibly_blank("\u{200B}"));
        assert!(is_visibly_blank(" \t\n"));
        assert!(is_visibly_blank(""));
        assert!(!is_visibly_blank("alice"));
        assert!(!is_visibly_blank("\u{200B}a"));
    }

    /// `migrate` refuses a symlinked legacy tree — the read-side guard
    /// applies to the migration read exactly as it does to `load()`'s
    /// fallback, so outside content cannot be smuggled into the
    /// tracked, git-visible store.
    #[cfg(unix)]
    #[test]
    fn migrate_refuses_symlinked_legacy_store() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("exceptions.json"), r#"{"exceptions":[]}"#).unwrap();
        std::os::unix::fs::symlink(&outside, root.join(".anvil")).unwrap();
        let err = ExceptionStore::migrate(root).expect_err("symlinked legacy must refuse");
        assert!(
            matches!(err, ExceptionError::SymlinkedPath { .. }),
            "{err:?}"
        );
        assert!(
            !root.join("anvil/exceptions/store.json").exists(),
            "nothing may be written on refusal",
        );
    }

    // --- EXCEPT-006 scope matching for gate findings ---

    #[test]
    fn covers_finding_matches_rule_id_with_empty_pattern() {
        let ex = make_attributed("AP-001", "");
        assert!(ex.covers_finding("AP-001", "src/a.ts", None));
    }

    #[test]
    fn covers_finding_rejects_rule_id_mismatch() {
        let ex = make_attributed("AP-001", "");
        assert!(!ex.covers_finding("AP-002", "src/a.ts", None));
    }

    #[test]
    fn covers_finding_matches_glob_scope() {
        let ex = make_attributed("AP-001", "src/legacy/**");
        assert!(ex.covers_finding("AP-001", "src/legacy/old.ts", None));
        assert!(!ex.covers_finding("AP-001", "src/new/fresh.ts", None));
    }

    #[test]
    fn covers_finding_requires_literal_separator() {
        // `*` must not cross `/` — mirrors glob_matches's MatchOptions.
        let ex = make_attributed("AP-001", "src/*");
        assert!(ex.covers_finding("AP-001", "src/a.ts", None));
        assert!(!ex.covers_finding("AP-001", "src/nested/a.ts", None));
    }

    #[test]
    fn covers_finding_pinned_hash_requires_identical_hash() {
        let mut ex = make_attributed("AP-001", "");
        ex.finding_hash = Some("abc123".to_string());
        assert!(ex.covers_finding("AP-001", "src/a.ts", Some("abc123")));
        assert!(!ex.covers_finding("AP-001", "src/a.ts", Some("zzz999")));
        // A finding with no hash is never covered by a pinned grant.
        assert!(!ex.covers_finding("AP-001", "src/a.ts", None));
    }

    #[test]
    fn covers_finding_unpinned_grant_ignores_finding_hash() {
        let ex = make_attributed("AP-001", "");
        assert!(ex.covers_finding("AP-001", "src/a.ts", Some("abc123")));
    }

    #[test]
    fn covers_finding_invalid_glob_covers_nothing() {
        let ex = make_attributed("AP-001", "src/[invalid");
        assert!(!ex.covers_finding("AP-001", "src/a.ts", None));
    }

    #[test]
    fn covers_finding_is_scope_only_ignores_validity() {
        // A revoked grant still *covers* its finding — validity is
        // verify_exception_at's job, and gate callers check both.
        let mut ex = make_attributed("AP-001", "src/**");
        ex.revoked = Some(ExceptionRevocation {
            revoked_at: Utc::now(),
            revoked_by: "bob".to_string(),
            reason: "no longer needed".to_string(),
        });
        assert!(ex.covers_finding("AP-001", "src/a.ts", None));
    }

    // --- EXCEPT-005 scope/expiry/revocation verification ---

    #[test]
    fn exception_verify_active_for_attributed_in_scope_grant() {
        let ex = make_attributed("AP-001", "src/legacy/**");
        let verdict = verify_exception(&ex);
        assert_eq!(verdict, ExceptionVerdict::Active);
        assert!(verdict.applies());
        assert!(!verdict.is_downgrade());
    }

    #[test]
    fn exception_verify_empty_scope_is_valid() {
        // Empty file_pattern means "all files" — a valid scope, not InvalidScope.
        let ex = make_attributed("AP-001", "");
        assert_eq!(verify_exception(&ex), ExceptionVerdict::Active);
    }

    #[test]
    fn exception_verify_expired_does_not_apply() {
        let now = Utc::now();
        let mut ex = make_attributed("AP-001", "src/**");
        ex.created_at = now - Duration::days(30);
        ex.expires_at = Some(now - Duration::days(1));
        let verdict = verify_exception_at(&ex, now);
        assert_eq!(verdict, ExceptionVerdict::Expired);
        assert!(!verdict.applies());
    }

    #[test]
    fn exception_verify_revoked_does_not_apply() {
        let mut ex = make_attributed("AP-001", "src/**");
        ex.revoked = Some(ExceptionRevocation {
            revoked_at: Utc::now(),
            revoked_by: "bob@example.test".to_string(),
            reason: "no longer needed".to_string(),
        });
        let verdict = verify_exception(&ex);
        assert_eq!(verdict, ExceptionVerdict::Revoked);
        assert!(!verdict.applies());
    }

    #[test]
    fn exception_verify_invalid_scope_glob_does_not_apply() {
        // An unclosed character class is not a parseable glob.
        let ex = make_attributed("AP-001", "src/[unclosed");
        let verdict = verify_exception(&ex);
        assert_eq!(verdict, ExceptionVerdict::InvalidScope);
        assert!(!verdict.applies());
    }

    /// The core EXCEPT-005 contract: a v0-shape grant (no `owner`, no
    /// `created_by`) applies but as a downgrade — never silently honoured.
    #[test]
    fn exception_verify_unattributed_v0_grant_downgrades() {
        let ex = make_exception("AP-001", "src/**"); // owner + created_by None
        let verdict = verify_exception(&ex);
        assert_eq!(verdict, ExceptionVerdict::Unattributed);
        assert!(verdict.applies(), "still applies");
        assert!(verdict.is_downgrade(), "but flagged as a downgrade");
    }

    #[test]
    fn exception_verify_owner_alone_is_attributed() {
        let mut ex = make_exception("AP-001", "src/**");
        ex.owner = Some("team-platform".to_string());
        assert_eq!(verify_exception(&ex), ExceptionVerdict::Active);
    }

    #[test]
    fn exception_verify_created_by_alone_is_attributed() {
        let mut ex = make_exception("AP-001", "src/**");
        ex.created_by = Some("alice@example.test".to_string());
        assert_eq!(verify_exception(&ex), ExceptionVerdict::Active);
    }

    /// Precedence: revoked beats expired beats invalid-scope beats
    /// unattributed. A grant that trips every check reports `Revoked`.
    #[test]
    fn exception_verify_precedence_revoked_over_expired_over_scope_over_attribution() {
        let now = Utc::now();
        // Unattributed + invalid scope + expired + revoked → Revoked.
        let mut ex = make_exception("AP-001", "src/[bad");
        ex.expires_at = Some(now - Duration::days(1));
        ex.revoked = Some(ExceptionRevocation {
            revoked_at: now,
            revoked_by: "bob".to_string(),
            reason: "x".to_string(),
        });
        assert_eq!(verify_exception_at(&ex, now), ExceptionVerdict::Revoked);

        // Drop revocation → Expired wins over invalid-scope + attribution.
        ex.revoked = None;
        assert_eq!(verify_exception_at(&ex, now), ExceptionVerdict::Expired);

        // Drop expiry → InvalidScope wins over attribution.
        ex.expires_at = None;
        assert_eq!(
            verify_exception_at(&ex, now),
            ExceptionVerdict::InvalidScope
        );
    }

    #[test]
    fn exception_verify_verdict_tokens_are_stable() {
        assert_eq!(ExceptionVerdict::Revoked.as_str(), "revoked");
        assert_eq!(ExceptionVerdict::Expired.as_str(), "expired");
        assert_eq!(ExceptionVerdict::InvalidScope.as_str(), "invalid-scope");
        assert_eq!(ExceptionVerdict::Unattributed.as_str(), "unattributed");
        assert_eq!(ExceptionVerdict::Active.as_str(), "active");
    }

    #[test]
    fn exception_verify_batch_classifies_each() {
        let now = Utc::now();
        let mut revoked = make_attributed("AP-002", "src/**");
        revoked.revoked = Some(ExceptionRevocation {
            revoked_at: now,
            revoked_by: "bob".to_string(),
            reason: "x".to_string(),
        });
        let exceptions = vec![
            make_attributed("AP-001", "src/**"),
            revoked,
            make_exception("AP-003", "src/**"),
        ];
        let verified = verify_exceptions_at(&exceptions, now);
        assert_eq!(verified.len(), 3);
        assert_eq!(verified[0].verdict, ExceptionVerdict::Active);
        assert_eq!(verified[1].verdict, ExceptionVerdict::Revoked);
        assert_eq!(verified[1].exception.policy_id, "AP-002");
        assert_eq!(verified[2].verdict, ExceptionVerdict::Unattributed);
    }

    /// Expiry is exclusive at the exact boundary: `now == expires_at` is
    /// not yet expired. Pins the contract GITGOV-009 replays against.
    #[test]
    fn exception_verify_at_exact_expiry_boundary_is_not_yet_expired() {
        let t = Utc::now();
        let mut ex = make_attributed("AP-001", "src/**");
        ex.expires_at = Some(t);
        assert_eq!(verify_exception_at(&ex, t), ExceptionVerdict::Active);
        assert_eq!(
            verify_exception_at(&ex, t + Duration::nanoseconds(1)),
            ExceptionVerdict::Expired
        );
    }

    /// Blank attribution (`Some("")` / whitespace) is attribution in name
    /// only — still `Unattributed`, never `Active`.
    #[test]
    fn exception_verify_blank_attribution_is_unattributed() {
        let mut ex = make_exception("AP-001", "src/**");
        ex.owner = Some("   ".to_string());
        ex.created_by = Some(String::new());
        assert_eq!(verify_exception(&ex), ExceptionVerdict::Unattributed);
    }

    /// Once revoked/expired/invalid-scope are cleared, a valid-but-
    /// unattributed grant lands on `Unattributed` (the last precedence
    /// step), guarding the scope-vs-attribution check order.
    #[test]
    fn exception_verify_precedence_lands_on_unattributed_when_only_attribution_missing() {
        let now = Utc::now();
        let ex = make_exception("AP-001", "src/**"); // valid scope, no attribution
        assert_eq!(
            verify_exception_at(&ex, now),
            ExceptionVerdict::Unattributed
        );
    }

    // --- EXCEPT-003 enriched schema ---

    #[test]
    fn exception_schema_serialises_v1_attribution_and_revocation() {
        let created_at = DateTime::parse_from_rfc3339("2026-06-08T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let revoked_at = DateTime::parse_from_rfc3339("2026-06-09T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let exception = PolicyException {
            schema_version: "anvil.exception.v1".to_string(),
            id: "exc_01jz0example".to_string(),
            policy_id: "AP-001".to_string(),
            file_pattern: "src/legacy/**".to_string(),
            finding_hash: Some("sha256:abc123".to_string()),
            reason: "intentional legacy boundary".to_string(),
            owner: Some("team-platform".to_string()),
            created_by: Some("alice@example.test".to_string()),
            created_at,
            expires_at: None,
            revoked: Some(ExceptionRevocation {
                revoked_at,
                revoked_by: "bob@example.test".to_string(),
                reason: "migration complete".to_string(),
            }),
        };

        let value = serde_json::to_value(&exception).unwrap();

        assert_eq!(value["schema_version"], "anvil.exception.v1");
        assert_eq!(value["id"], "exc_01jz0example");
        assert_eq!(value["finding_hash"], "sha256:abc123");
        assert_eq!(value["owner"], "team-platform");
        assert_eq!(value["created_by"], "alice@example.test");
        assert_eq!(value["revoked"]["revoked_by"], "bob@example.test");
    }

    #[test]
    fn exception_schema_deserialises_v0_shape_with_defaults() {
        let exception: PolicyException = serde_json::from_str(
            r#"{
              "policy_id": "AP-001",
              "file_pattern": "src/**",
              "reason": "legacy code",
              "created_at": "2026-06-08T10:00:00Z"
            }"#,
        )
        .unwrap();

        assert_eq!(exception.schema_version, "anvil.exception.v1");
        assert!(!exception.id.is_empty());
        assert_eq!(exception.finding_hash, None);
        assert_eq!(exception.owner, None);
        assert_eq!(exception.created_by, None);
        assert_eq!(exception.revoked, None);
    }

    #[test]
    fn exception_schema_normalises_empty_schema_version_to_v1() {
        let exception: PolicyException = serde_json::from_str(
            r#"{
              "schema_version": "",
              "policy_id": "AP-001",
              "file_pattern": "src/**",
              "reason": "legacy code",
              "created_at": "2026-06-08T10:00:00Z"
            }"#,
        )
        .unwrap();

        assert_eq!(exception.schema_version, EXCEPTION_SCHEMA_VERSION);
    }

    #[test]
    fn exception_schema_rejects_unknown_schema_version() {
        let err = serde_json::from_str::<PolicyException>(
            r#"{
              "schema_version": "anvil.exception.v2",
              "policy_id": "AP-001",
              "file_pattern": "src/**",
              "reason": "legacy code",
              "created_at": "2026-06-08T10:00:00Z"
            }"#,
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("unsupported exception schema_version")
        );
    }

    #[test]
    fn exception_schema_keeps_flat_store_layout() {
        let store_json = serde_json::to_value(ExceptionStore {
            exceptions: vec![make_exception("AP-001", "src/**")],
            source: StoreSource::Fresh,
        })
        .unwrap();

        assert!(store_json.get("exceptions").is_some());
        assert!(store_json.get("active").is_none());
        assert!(store_json.get("revoked").is_none());
    }

    #[test]
    fn exception_schema_revoked_entries_are_inactive_without_erasure() {
        let v = make_violation("AP-001", "src/foo.ts");
        let mut ex = make_exception("AP-001", "src/**");
        ex.revoked = Some(ExceptionRevocation {
            revoked_at: Utc::now(),
            revoked_by: "alice@example.test".to_string(),
            reason: "no longer needed".to_string(),
        });

        assert!(!is_suppressed(&v, &[ex.clone()]));
        assert!(ex.revoked.is_some());
    }

    #[test]
    fn exception_schema_finding_hash_limits_suppression_to_matching_fingerprint() {
        let mut v = make_violation("AP-001", "src/foo.ts");
        v.fingerprint = Some("sha256:match".to_string());
        let mut ex = make_exception("AP-001", "src/**");
        ex.finding_hash = Some("sha256:match".to_string());

        assert!(is_suppressed(&v, &[ex]));
    }

    #[test]
    fn exception_schema_finding_hash_does_not_suppress_different_fingerprint() {
        let mut v = make_violation("AP-001", "src/foo.ts");
        v.fingerprint = Some("sha256:other".to_string());
        let mut ex = make_exception("AP-001", "src/**");
        ex.finding_hash = Some("sha256:match".to_string());

        assert!(!is_suppressed(&v, &[ex]));
    }

    #[test]
    fn suppresses_matching_policy_id() {
        let v = make_violation("AP-001", "src/foo.ts");
        let ex = make_exception("AP-001", "");
        assert!(is_suppressed(&v, &[ex]));
    }

    #[test]
    fn does_not_suppress_different_policy() {
        let v = make_violation("AP-001", "src/foo.ts");
        let ex = make_exception("AP-002", "");
        assert!(!is_suppressed(&v, &[ex]));
    }

    #[test]
    fn suppresses_with_matching_glob() {
        let v = make_violation("AP-001", "src/legacy/old.ts");
        let ex = make_exception("AP-001", "src/legacy/**");
        assert!(is_suppressed(&v, &[ex]));
    }

    #[test]
    fn does_not_suppress_non_matching_glob() {
        let v = make_violation("AP-001", "src/new/fresh.ts");
        let ex = make_exception("AP-001", "src/legacy/**");
        assert!(!is_suppressed(&v, &[ex]));
    }

    #[test]
    fn expired_exception_is_ignored() {
        let v = make_violation("AP-001", "src/foo.ts");
        // Pin one `now` so the derived `id` and `created_at` agree (a
        // second `Utc::now()` could tick between them).
        let now = Utc::now();
        let created_at = now - Duration::days(30);
        let ex = PolicyException {
            schema_version: default_exception_schema_version(),
            id: exception_id_from_parts("AP-001", "", created_at, None),
            policy_id: "AP-001".to_string(),
            file_pattern: String::new(),
            finding_hash: None,
            reason: "temporary".to_string(),
            owner: None,
            created_by: None,
            created_at,
            expires_at: Some(now - Duration::days(1)),
            revoked: None,
        };
        assert!(!is_suppressed(&v, &[ex]));
    }

    #[test]
    fn non_expired_exception_applies() {
        let v = make_violation("AP-001", "src/foo.ts");
        // One `now` so the derived `id` matches `created_at`.
        let now = Utc::now();
        let ex = PolicyException {
            schema_version: default_exception_schema_version(),
            id: exception_id_from_parts("AP-001", "", now, None),
            policy_id: "AP-001".to_string(),
            file_pattern: String::new(),
            finding_hash: None,
            reason: "temporary".to_string(),
            owner: None,
            created_by: None,
            created_at: now,
            expires_at: Some(now + Duration::days(7)),
            revoked: None,
        };
        assert!(is_suppressed(&v, &[ex]));
    }

    #[test]
    fn filter_removes_suppressed() {
        let violations = vec![
            make_violation("AP-001", "src/a.ts"),
            make_violation("AP-002", "src/b.ts"),
            make_violation("AP-001", "src/c.ts"),
        ];
        let exceptions = vec![make_exception("AP-001", "")];
        let remaining = filter_suppressed(violations, &exceptions);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].policy_id, "AP-002");
    }

    #[test]
    fn glob_star_matches_single_segment() {
        assert!(glob_matches("src/*.ts", "src/foo.ts"));
        assert!(!glob_matches("src/*.ts", "src/sub/foo.ts"));
    }

    #[test]
    fn glob_doublestar_matches_depth() {
        assert!(glob_matches("src/**", "src/a/b/c.ts"));
        assert!(glob_matches("**/test.ts", "a/b/test.ts"));
        assert!(glob_matches("src/**/test.ts", "src/deep/nested/test.ts"));
    }

    #[test]
    fn glob_question_mark() {
        assert!(glob_matches("src/?.ts", "src/a.ts"));
        assert!(!glob_matches("src/?.ts", "src/ab.ts"));
    }

    #[test]
    fn store_load_save_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = ExceptionStore::empty();
        store.add(make_exception("AP-001", "src/**"));
        store.add(make_exception("AP-003", ""));

        assert_eq!(store.save(tmp.path()).unwrap(), WriteOutcome::Written);
        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.exceptions.len(), 2);
        assert_eq!(loaded.exceptions[0].policy_id, "AP-001");
    }

    #[test]
    fn store_load_returns_empty_when_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = ExceptionStore::load(tmp.path()).unwrap();
        assert!(store.exceptions.is_empty());
    }

    #[test]
    fn store_remove_by_policy() {
        let mut store = ExceptionStore::empty();
        store.add(make_exception("AP-001", ""));
        store.add(make_exception("AP-002", ""));
        store.add(make_exception("AP-001", "src/**"));

        store.remove_by_policy("AP-001");
        assert_eq!(store.exceptions.len(), 1);
        assert_eq!(store.exceptions[0].policy_id, "AP-002");
    }

    #[test]
    fn store_active_filters_expired() {
        let mut store = ExceptionStore::empty();
        let now = Utc::now();

        store.add(PolicyException {
            schema_version: default_exception_schema_version(),
            id: exception_id_from_parts("AP-001", "", now, None),
            policy_id: "AP-001".to_string(),
            file_pattern: String::new(),
            finding_hash: None,
            reason: "still valid".to_string(),
            owner: None,
            created_by: None,
            created_at: now,
            expires_at: Some(now + Duration::days(7)),
            revoked: None,
        });
        store.add(PolicyException {
            schema_version: default_exception_schema_version(),
            id: exception_id_from_parts("AP-002", "", now - Duration::days(30), None),
            policy_id: "AP-002".to_string(),
            file_pattern: String::new(),
            finding_hash: None,
            reason: "expired".to_string(),
            owner: None,
            created_by: None,
            created_at: now - Duration::days(30),
            expires_at: Some(now - Duration::days(1)),
            revoked: None,
        });

        let active = store.active_exceptions();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].policy_id, "AP-001");
    }

    // --- ADR-073 storage-path migration (EXCEPT-001/002) ---

    fn write_store_at(root: &Path, rel: &str, store: &ExceptionStore) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let content = serde_json::to_string_pretty(store).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn save_writes_tracked_path_not_legacy() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = ExceptionStore::empty();
        store.add(make_exception("AP-001", "src/**"));
        assert_eq!(store.save(tmp.path()).unwrap(), WriteOutcome::Written);

        assert!(tmp.path().join("anvil/exceptions/store.json").exists());
        assert!(!tmp.path().join(".anvil/exceptions.json").exists());
    }

    #[test]
    fn load_prefers_tracked_over_legacy() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut tracked = ExceptionStore::empty();
        tracked.add(make_exception("AP-TRACKED", ""));
        write_store_at(tmp.path(), "anvil/exceptions/store.json", &tracked);

        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-LEGACY", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.exceptions.len(), 1);
        assert_eq!(loaded.exceptions[0].policy_id, "AP-TRACKED");
    }

    #[test]
    fn load_falls_back_to_legacy_when_tracked_absent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-LEGACY", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.exceptions.len(), 1);
        assert_eq!(loaded.exceptions[0].policy_id, "AP-LEGACY");
    }

    #[test]
    fn migrate_copies_legacy_to_tracked_non_destructive() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-001", "src/**"));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        let migrated = ExceptionStore::migrate(tmp.path()).unwrap();
        assert_eq!(migrated, MigrateOutcome::Migrated);

        // Tracked store now exists and carries the data.
        assert!(tmp.path().join("anvil/exceptions/store.json").exists());
        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.exceptions.len(), 1);
        assert_eq!(loaded.exceptions[0].policy_id, "AP-001");

        // Legacy file is left in place (non-destructive).
        assert!(tmp.path().join(".anvil/exceptions.json").exists());
    }

    #[test]
    fn migrate_is_idempotent_and_noop_without_legacy() {
        let tmp = tempfile::TempDir::new().unwrap();

        // Nothing to migrate when no legacy store exists.
        assert_eq!(
            ExceptionStore::migrate(tmp.path()).unwrap(),
            MigrateOutcome::NothingToDo
        );

        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-001", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        // First migration moves the data.
        assert_eq!(
            ExceptionStore::migrate(tmp.path()).unwrap(),
            MigrateOutcome::Migrated
        );
        // Second call is a no-op because the tracked store now exists.
        assert_eq!(
            ExceptionStore::migrate(tmp.path()).unwrap(),
            MigrateOutcome::NothingToDo
        );
    }

    // --- EXCEPT-007 write-path hardening ---

    #[test]
    fn load_reports_provenance() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(
            ExceptionStore::load(tmp.path()).unwrap().source(),
            StoreSource::Fresh
        );

        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-001", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);
        assert_eq!(
            ExceptionStore::load(tmp.path()).unwrap().source(),
            StoreSource::Legacy
        );

        write_store_at(tmp.path(), "anvil/exceptions/store.json", &legacy);
        assert_eq!(
            ExceptionStore::load(tmp.path()).unwrap().source(),
            StoreSource::Tracked
        );
    }

    /// The silent-promotion hole the council flagged: load (legacy
    /// fallback) → save (tracked) must refuse, not quietly copy
    /// local-only entries into git.
    #[test]
    fn save_refuses_legacy_origin_store() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-001", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        let err = loaded.save(tmp.path()).unwrap_err();
        assert!(matches!(err, ExceptionError::LegacyOriginNotMigrated));
        // Nothing was promoted.
        assert!(!tmp.path().join("anvil/exceptions/store.json").exists());
    }

    #[test]
    fn migrate_then_reload_allows_save() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-001", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        assert_eq!(
            ExceptionStore::migrate(tmp.path()).unwrap(),
            MigrateOutcome::Migrated
        );
        let mut reloaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(reloaded.source(), StoreSource::Tracked);
        reloaded.add(make_exception("AP-002", ""));
        assert_eq!(reloaded.save(tmp.path()).unwrap(), WriteOutcome::Written);
    }

    #[test]
    fn update_applies_mutation_under_lock() {
        let tmp = tempfile::TempDir::new().unwrap();
        let outcome = ExceptionStore::update(tmp.path(), |store| {
            store.add(make_exception("AP-001", ""));
        })
        .unwrap();
        assert_eq!(outcome, WriteOutcome::Written);
        let outcome = ExceptionStore::update(tmp.path(), |store| {
            store.add(make_exception("AP-002", ""));
        })
        .unwrap();
        assert_eq!(outcome, WriteOutcome::Written);
        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.exceptions.len(), 2);
    }

    /// The lost-write race the council flagged: concurrent
    /// load-modify-save cycles must not drop each other's entries.
    /// `update` holds the flock across the full cycle, so every
    /// thread's exception survives.
    #[test]
    fn update_concurrent_writers_lose_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let threads: Vec<_> = (0..8)
            .map(|i| {
                let root = root.clone();
                std::thread::spawn(move || {
                    ExceptionStore::update(&root, |store| {
                        store.add(make_exception(&format!("AP-{i:03}"), ""));
                    })
                    .unwrap()
                })
            })
            .collect();
        for handle in threads {
            assert_eq!(handle.join().unwrap(), WriteOutcome::Written);
        }
        let loaded = ExceptionStore::load(&root).unwrap();
        assert_eq!(loaded.exceptions.len(), 8);
    }

    #[test]
    fn update_refuses_unmigrated_legacy_store() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-001", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        let err = ExceptionStore::update(tmp.path(), |store| {
            store.add(make_exception("AP-002", ""));
        })
        .unwrap_err();
        assert!(matches!(err, ExceptionError::LegacyOriginNotMigrated));
    }

    /// Hostile-repo gadget: `anvil/exceptions` as a symlink pointing
    /// outside the worktree must be refused, not written through.
    #[cfg(unix)]
    #[test]
    fn save_refuses_symlinked_exceptions_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let outside = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("anvil")).unwrap();
        std::os::unix::fs::symlink(outside.path(), tmp.path().join("anvil/exceptions")).unwrap();

        let store = ExceptionStore::empty();
        let err = store.save(tmp.path()).unwrap_err();
        assert!(matches!(err, ExceptionError::SymlinkedPath { .. }));
        // Nothing escaped into the symlink target.
        assert!(!outside.path().join("store.json").exists());
    }

    /// A dangling symlink reports `exists() == false`; the guard must
    /// still refuse it rather than letting `create_dir_all` follow it.
    #[cfg(unix)]
    #[test]
    fn save_refuses_dangling_symlinked_store_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("anvil/exceptions")).unwrap();
        std::os::unix::fs::symlink(
            "/nonexistent/elsewhere.json",
            tmp.path().join("anvil/exceptions/store.json"),
        )
        .unwrap();

        let store = ExceptionStore::empty();
        let err = store.save(tmp.path()).unwrap_err();
        assert!(matches!(err, ExceptionError::SymlinkedPath { .. }));
    }

    /// ADR-002: a read-only worktree (bare CI checkout) degrades to a
    /// typed skip the gate can warn on — never a propagated I/O error.
    #[cfg(unix)]
    #[test]
    fn save_skips_readonly_worktree() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::TempDir::new().unwrap();
        let mut perms = std::fs::metadata(tmp.path()).unwrap().permissions();
        perms.set_mode(0o555);
        std::fs::set_permissions(tmp.path(), perms.clone()).unwrap();

        let store = ExceptionStore::empty();
        let outcome = store.save(tmp.path()).unwrap();
        assert!(
            matches!(outcome, WriteOutcome::SkippedReadOnly { ref detail } if !detail.is_empty()),
            "expected SkippedReadOnly with detail, got {outcome:?}",
        );

        // Restore so TempDir cleanup can delete the tree.
        perms.set_mode(0o755);
        std::fs::set_permissions(tmp.path(), perms).unwrap();
    }

    /// The realistic CI shape: the tracked structure already exists
    /// (committed store, cloned read-only), so `create_dir_all`
    /// succeeds and the failure surfaces at the lock-file open /
    /// temp-file create instead — must still degrade, not error.
    #[cfg(unix)]
    #[test]
    fn save_skips_readonly_when_structure_exists() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::TempDir::new().unwrap();
        let mut tracked = ExceptionStore::empty();
        tracked.add(make_exception("AP-001", ""));
        write_store_at(tmp.path(), "anvil/exceptions/store.json", &tracked);

        let dir = tmp.path().join("anvil/exceptions");
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_mode(0o555);
        std::fs::set_permissions(&dir, perms.clone()).unwrap();

        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.source(), StoreSource::Tracked);
        let outcome = loaded.save(tmp.path()).unwrap();
        assert!(
            matches!(outcome, WriteOutcome::SkippedReadOnly { ref detail } if !detail.is_empty()),
            "expected SkippedReadOnly with detail, got {outcome:?}",
        );

        perms.set_mode(0o755);
        std::fs::set_permissions(&dir, perms).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn migrate_skips_readonly_worktree() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::TempDir::new().unwrap();
        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-001", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        let mut perms = std::fs::metadata(tmp.path()).unwrap().permissions();
        perms.set_mode(0o555);
        std::fs::set_permissions(tmp.path(), perms.clone()).unwrap();

        let outcome = ExceptionStore::migrate(tmp.path()).unwrap();
        assert!(
            matches!(outcome, MigrateOutcome::SkippedReadOnly { ref detail } if !detail.is_empty()),
            "expected SkippedReadOnly with detail, got {outcome:?}",
        );
        assert!(!tmp.path().join("anvil/exceptions/store.json").exists());

        perms.set_mode(0o755);
        std::fs::set_permissions(tmp.path(), perms).unwrap();
    }
}
