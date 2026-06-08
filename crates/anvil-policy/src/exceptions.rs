use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::evaluator::Violation;

const EXCEPTION_SCHEMA_VERSION: &str = "anvil.exception.v1";

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
    /// a symlink — refusing to write through it (hostile-repo
    /// write-outside-worktree gadget). Mirrors `anvil-witness`'s guard.
    #[error("refusing to write through symlinked governance path: {path}")]
    SymlinkedPath {
        /// The offending symlinked component.
        path: std::path::PathBuf,
    },
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum WriteOutcome {
    /// The tracked store was written.
    Written,
    /// The worktree is read-only — the write was skipped. Gate callers
    /// surface this as a warning, never a failure (ADR-002).
    SkippedReadOnly,
}

/// Outcome of [`ExceptionStore::migrate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum MigrateOutcome {
    /// Legacy data was copied into the tracked store.
    Migrated,
    /// Nothing to do: no legacy store, or the tracked store already exists.
    NothingToDo,
    /// The worktree is read-only — the migration was skipped (ADR-002).
    SkippedReadOnly,
}

/// Tracked store path (ADR-073). Exceptions are durable governance state that
/// must travel with the repository and be visible in PR review, so they live
/// under `anvil/`, not the gitignored `.anvil/` runtime tree.
const EXCEPTIONS_FILE: &str = "anvil/exceptions/store.json";

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
        let tracked = workspace_root.join(EXCEPTIONS_FILE);
        if tracked.exists() {
            return Self::load_from(&tracked, StoreSource::Tracked);
        }
        let legacy = workspace_root.join(LEGACY_EXCEPTIONS_FILE);
        if legacy.exists() {
            return Self::load_from(&legacy, StoreSource::Legacy);
        }
        Ok(Self::empty())
    }

    /// Reads and parses a store from an explicit path, tagging its origin.
    fn load_from(path: &Path, source: StoreSource) -> Result<Self, ExceptionError> {
        let content = std::fs::read_to_string(path)?;
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
            Err(e) if is_readonly_io(&e) => Ok(WriteOutcome::SkippedReadOnly),
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
            Err(e) if is_readonly_io(&e) => Ok(WriteOutcome::SkippedReadOnly),
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
        let result = Self::locked(workspace_root, || {
            // Re-check under the lock: a concurrent migrate may have won.
            if tracked.exists() {
                return Ok(MigrateOutcome::NothingToDo);
            }
            let mut store = Self::load_from(&legacy, StoreSource::Legacy)?;
            // The explicit-migration path is the one place a legacy-origin
            // store may be promoted; re-tag before the tracked write.
            store.source = StoreSource::Tracked;
            store.write_tracked(workspace_root)?;
            Ok(MigrateOutcome::Migrated)
        });
        match result {
            Ok(outcome) => Ok(outcome),
            Err(e) if is_readonly_io(&e) => Ok(MigrateOutcome::SkippedReadOnly),
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

impl PolicyException {
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
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(ExceptionError::Io(e));
            }
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
        if ex.file_pattern.is_empty() {
            return true;
        }
        glob_matches(&ex.file_pattern, &violation.file)
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
        let ex = PolicyException {
            schema_version: default_exception_schema_version(),
            id: exception_id_from_parts("AP-001", "", Utc::now() - Duration::days(30), None),
            policy_id: "AP-001".to_string(),
            file_pattern: String::new(),
            finding_hash: None,
            reason: "temporary".to_string(),
            owner: None,
            created_by: None,
            created_at: Utc::now() - Duration::days(30),
            expires_at: Some(Utc::now() - Duration::days(1)),
            revoked: None,
        };
        assert!(!is_suppressed(&v, &[ex]));
    }

    #[test]
    fn non_expired_exception_applies() {
        let v = make_violation("AP-001", "src/foo.ts");
        let ex = PolicyException {
            schema_version: default_exception_schema_version(),
            id: exception_id_from_parts("AP-001", "", Utc::now(), None),
            policy_id: "AP-001".to_string(),
            file_pattern: String::new(),
            finding_hash: None,
            reason: "temporary".to_string(),
            owner: None,
            created_by: None,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::days(7)),
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
        assert_eq!(outcome, WriteOutcome::SkippedReadOnly);

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
        assert_eq!(outcome, WriteOutcome::SkippedReadOnly);

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
        assert_eq!(outcome, MigrateOutcome::SkippedReadOnly);
        assert!(!tmp.path().join("anvil/exceptions/store.json").exists());

        perms.set_mode(0o755);
        std::fs::set_permissions(tmp.path(), perms).unwrap();
    }
}
