//! GCTX-024: per-workspace persisted snippet-egress consent.
//!
//! Source-text snippet egress is identity-only by default (PV-9 CE-1). An
//! operator opts a *single workspace* in with `anvil gctx egress enable`, which
//! records consent as operator **state** under the operator-owned state
//! directory (`ANVIL_HOME`, else `$XDG_STATE_HOME/anvil`, else
//! `$HOME/.local/state/anvil`) — never a repository-controlled worktree path.
//! A hostile checkout or any process that can only write the worktree cannot
//! plant consent and open source-text egress.
//!
//! The daemon reads this record on the snippet path and feeds it to
//! [`anvil_gctx_types::resolve_snippet_egress`] (the env var overrides it; the
//! CE-1 default applies when neither is set).
//!
//! Writes are TOCTOU-hardened (symlink-refusing, atomic temp+rename). Reads fold
//! an absent file into `Ok(None)` (no consent — the default), but propagate
//! symlink/IO/parse errors rather than silently defaulting (operator state must
//! never fail open by accident): the daemon logs and fails safe to
//! identity-only, the CLI surfaces the error to the operator.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Legacy worktree-relative path historically used for the consent record.
///
/// Records at this path are **not** honoured. Consent lives only in
/// operator-owned state (see [`consent_state_dir`]). Kept public so operators
/// and docs can name the retired location when cleaning up old checkouts.
pub const EGRESS_CONSENT_PATH: &str = "anvil/witness/gctx-egress.json";

/// Subdirectory under the operator state prefix holding egress consent records.
const CONSENT_STATE_SUBDIR: &str = "gctx-egress";

/// Current consent-record schema version (CE-12 audit).
const CONSENT_VERSION: u32 = 1;

/// Upper bound on the consent file the daemon will read. The record is a tiny
/// fixed-shape JSON object; anything larger is tampering. Bounding the read
/// keeps the per-request daemon path cheap and prevents a giant file planted in
/// operator state from exhausting daemon memory.
const MAX_CONSENT_BYTES: u64 = 4096;

#[derive(Debug, Error)]
pub enum EgressConsentError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("malformed gctx-egress consent at `{path}`: {source}")]
    Malformed {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("gctx-egress consent at `{path}` is {size} bytes; refusing (cap {MAX_CONSENT_BYTES})")]
    TooLarge { path: PathBuf, size: u64 },
    #[error("`{path}` is a symlink; refusing to read/write gctx-egress consent")]
    SymlinkRefusal { path: PathBuf },
    #[error(
        "cannot resolve operator state directory for gctx-egress consent \
         (set ANVIL_HOME, XDG_STATE_HOME, or HOME)"
    )]
    UnresolvableStateDir,
    #[error(
        "gctx-egress consent at `{path}` is bound to workspace `{bound}`, \
         not `{expected}`"
    )]
    WorkspaceMismatch {
        path: PathBuf,
        bound: String,
        expected: String,
    },
}

/// On-disk consent record. Minimal by design: the daemon only needs the boolean;
/// the version is for CE-12 audit / future migration; `workspace_root` binds the
/// record to the workspace it was enabled for so a swapped leaf cannot enable a
/// different tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EgressConsentRecord {
    /// The operator has consented to source-text snippet egress for this
    /// workspace (CE-12). An absent file means no consent — the CE-1 default.
    snippet_egress: bool,
    #[serde(default = "default_version")]
    consent_version: u32,
    /// Canonical absolute workspace path this consent applies to.
    workspace_root: String,
}

fn default_version() -> u32 {
    CONSENT_VERSION
}

/// Resolve the operator-owned directory that holds egress consent records.
///
/// Order (mirrors the graph-cache state dir): `ANVIL_HOME` →
/// `$XDG_STATE_HOME/anvil` → `$HOME/.local/state/anvil`. Returns `None` when no
/// home can be resolved — callers fail closed (no consent / cannot enable).
#[must_use]
pub fn consent_state_dir() -> Option<PathBuf> {
    consent_state_dir_from(
        non_empty_env("ANVIL_HOME"),
        non_empty_env("XDG_STATE_HOME"),
        non_empty_env("HOME").or_else(|| non_empty_env("USERPROFILE")),
    )
    .map(|prefix| prefix.join(CONSENT_STATE_SUBDIR))
}

/// Pure resolver for [`consent_state_dir`], taking candidate roots explicitly so
/// unit tests do not need to mutate the process environment for path math.
#[must_use]
fn consent_state_dir_from(
    anvil_home: Option<PathBuf>,
    xdg_state_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(prefix) = anvil_home {
        return Some(absolutise_prefix(prefix));
    }
    if let Some(state) = xdg_state_home {
        return Some(state.join("anvil"));
    }
    home.map(|h| h.join(".local").join("state").join("anvil"))
}

fn absolutise_prefix(p: PathBuf) -> PathBuf {
    if p.is_absolute() {
        p
    } else {
        std::env::current_dir().map_or(p.clone(), |cwd| cwd.join(p))
    }
}

fn non_empty_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Canonical workspace path used as the consent key and binding.
fn canonical_workspace(repo_root: &Path) -> PathBuf {
    dunce::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf())
}

/// Deterministic leaf name for a workspace under the operator consent dir.
fn consent_leaf_name(canonical_root: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_root.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    format!("{hex}.json")
}

fn consent_file_path(state_dir: &Path, canonical_root: &Path) -> PathBuf {
    state_dir.join(consent_leaf_name(canonical_root))
}

/// Read the persisted snippet-egress consent for `repo_root`.
///
/// - no operator state dir / absent file → `Ok(None)` (no consent — CE-1 default),
/// - present + valid + bound to this workspace → `Ok(Some(snippet_egress))`,
/// - symlinked path / IO error / malformed content / workspace mismatch → `Err`
///   (never silently folded to the default).
///
/// A regular JSON file planted under the worktree (the legacy
/// [`EGRESS_CONSENT_PATH`]) is **ignored** — it is not operator state.
pub fn read_snippet_consent(repo_root: &Path) -> Result<Option<bool>, EgressConsentError> {
    let Some(state_dir) = consent_state_dir() else {
        return Ok(None);
    };
    read_snippet_consent_in(repo_root, &state_dir)
}

fn read_snippet_consent_in(
    repo_root: &Path,
    state_dir: &Path,
) -> Result<Option<bool>, EgressConsentError> {
    refuse_if_symlink(state_dir)?;
    let canonical = canonical_workspace(repo_root);
    let path = consent_file_path(state_dir, &canonical);
    refuse_if_symlink(&path)?;
    if !path.exists() {
        return Ok(None);
    }
    let record = load_record(&path)?;
    let expected = canonical.to_string_lossy();
    if record.workspace_root != expected {
        return Err(EgressConsentError::WorkspaceMismatch {
            path: path.clone(),
            bound: record.workspace_root,
            expected: expected.into_owned(),
        });
    }
    Ok(Some(record.snippet_egress))
}

fn load_record(path: &Path) -> Result<EgressConsentRecord, EgressConsentError> {
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::with_capacity(usize::try_from(MAX_CONSENT_BYTES).unwrap_or(4096) + 1);
    let size = Read::by_ref(&mut file)
        .take(MAX_CONSENT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if u64::try_from(size).unwrap_or(u64::MAX) > MAX_CONSENT_BYTES {
        return Err(EgressConsentError::TooLarge {
            path: path.to_path_buf(),
            size: u64::try_from(size).unwrap_or(u64::MAX),
        });
    }
    serde_json::from_slice(&bytes).map_err(|source| EgressConsentError::Malformed {
        path: path.to_path_buf(),
        source,
    })
}

/// Persist the operator's consent to snippet egress for `repo_root` (atomic
/// temp+rename, symlink-refusing). Idempotent — re-enabling overwrites the same
/// record. Writes only under operator-owned state; never the worktree.
pub fn enable_snippet_consent(repo_root: &Path) -> Result<(), EgressConsentError> {
    let state_dir = consent_state_dir().ok_or(EgressConsentError::UnresolvableStateDir)?;
    enable_snippet_consent_in(repo_root, &state_dir)
}

fn enable_snippet_consent_in(repo_root: &Path, state_dir: &Path) -> Result<(), EgressConsentError> {
    refuse_if_symlink(state_dir)?;
    fs::create_dir_all(state_dir)?;
    // TOCTOU: re-check after create so a racing process cannot swap the state
    // directory for a symlink between the pre-check and the write.
    refuse_if_symlink(state_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(state_dir, fs::Permissions::from_mode(0o700));
    }

    let canonical = canonical_workspace(repo_root);
    let final_path = consent_file_path(state_dir, &canonical);
    refuse_if_symlink(&final_path)?;

    // Process-unique temp name so two concurrent `enable` invocations do not
    // stomp the same scratch file.
    let tmp_path = state_dir.join(format!(
        ".{}.{}.tmp",
        consent_leaf_name(&canonical),
        std::process::id()
    ));
    refuse_if_symlink(&tmp_path)?;

    let record = EgressConsentRecord {
        snippet_egress: true,
        consent_version: CONSENT_VERSION,
        workspace_root: canonical.to_string_lossy().into_owned(),
    };
    let bytes = serde_json::to_vec_pretty(&record).expect("egress consent record serialises");
    {
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600));
    }
    refuse_if_symlink(&final_path)?;
    // `std::fs::rename` replaces an existing destination atomically on both Unix
    // and Windows, so a re-enable over an existing record is safe and idempotent.
    fs::rename(&tmp_path, &final_path)?;
    Ok(())
}

/// Revoke consent for `repo_root` by removing the operator-state record — a clean
/// revert to the CE-1 identity-only default. Idempotent: a no-op when no record
/// exists or no operator state directory can be resolved.
pub fn disable_snippet_consent(repo_root: &Path) -> Result<(), EgressConsentError> {
    let Some(state_dir) = consent_state_dir() else {
        // Nothing to revoke when there is nowhere consent could have been stored.
        return Ok(());
    };
    disable_snippet_consent_in(repo_root, &state_dir)
}

fn disable_snippet_consent_in(
    repo_root: &Path,
    state_dir: &Path,
) -> Result<(), EgressConsentError> {
    refuse_if_symlink(state_dir)?;
    let canonical = canonical_workspace(repo_root);
    let path = consent_file_path(state_dir, &canonical);
    refuse_if_symlink(&path)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(EgressConsentError::Io(e)),
    }
}

/// Refuse a symlink at `path` (TOCTOU defence), treating absence as fine.
fn refuse_if_symlink(path: &Path) -> Result<(), EgressConsentError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(EgressConsentError::SymlinkRefusal {
            path: path.to_path_buf(),
        }),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(EgressConsentError::Io(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Isolate operator state under a fresh temp `ANVIL_HOME` so unit tests never
    /// touch the real user's state directory.
    fn with_operator_home<T>(f: impl FnOnce(&Path) -> T) -> T {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().to_path_buf();
        temp_env::with_var("ANVIL_HOME", Some(path.as_os_str()), || f(&path))
    }

    #[test]
    fn read_absent_is_none() {
        with_operator_home(|_| {
            let dir = tempfile::tempdir().unwrap();
            assert_eq!(read_snippet_consent(dir.path()).unwrap(), None);
        });
    }

    #[test]
    fn enable_then_read_is_some_true() {
        with_operator_home(|home| {
            let dir = tempfile::tempdir().unwrap();
            enable_snippet_consent(dir.path()).unwrap();
            assert_eq!(read_snippet_consent(dir.path()).unwrap(), Some(true));
            // Consent lives under operator state, never the worktree.
            assert!(!dir.path().join(EGRESS_CONSENT_PATH).exists());
            let state = home.join(CONSENT_STATE_SUBDIR);
            let leaf = consent_leaf_name(&canonical_workspace(dir.path()));
            assert!(state.join(leaf).is_file());
        });
    }

    #[test]
    fn planted_worktree_consent_record_cannot_enable_snippet_egress() {
        // Clawpatch fnd_sig-feat-library-40c2cde0d8-ee12_b4c4c89514:
        // a hostile checkout must not open source-text egress by planting a
        // regular JSON file at the legacy worktree path.
        with_operator_home(|_| {
            let dir = tempfile::tempdir().unwrap();
            fs::create_dir_all(dir.path().join("anvil/witness")).unwrap();
            fs::write(
                dir.path().join(EGRESS_CONSENT_PATH),
                br#"{"snippet_egress":true,"consent_version":1,"workspace_root":"ignored"}"#,
            )
            .unwrap();
            assert_eq!(
                read_snippet_consent(dir.path()).unwrap(),
                None,
                "a repository-controlled regular consent record must not enable egress"
            );
        });
    }

    #[test]
    fn disable_removes_record_and_reverts_to_default() {
        with_operator_home(|_| {
            let dir = tempfile::tempdir().unwrap();
            enable_snippet_consent(dir.path()).unwrap();
            disable_snippet_consent(dir.path()).unwrap();
            assert_eq!(read_snippet_consent(dir.path()).unwrap(), None);
        });
    }

    #[test]
    fn disable_is_idempotent_when_absent() {
        with_operator_home(|_| {
            let dir = tempfile::tempdir().unwrap();
            disable_snippet_consent(dir.path()).unwrap();
        });
    }

    #[test]
    fn enable_is_idempotent() {
        with_operator_home(|_| {
            let dir = tempfile::tempdir().unwrap();
            enable_snippet_consent(dir.path()).unwrap();
            enable_snippet_consent(dir.path()).unwrap();
            assert_eq!(read_snippet_consent(dir.path()).unwrap(), Some(true));
        });
    }

    #[test]
    fn enable_fails_when_operator_state_unresolvable() {
        temp_env::with_vars(
            [
                ("ANVIL_HOME", None::<&str>),
                ("XDG_STATE_HOME", None),
                ("HOME", None),
                ("USERPROFILE", None),
            ],
            || {
                let dir = tempfile::tempdir().unwrap();
                let err = enable_snippet_consent(dir.path()).unwrap_err();
                assert!(matches!(err, EgressConsentError::UnresolvableStateDir));
            },
        );
    }

    #[test]
    fn consent_is_per_workspace() {
        with_operator_home(|_| {
            let a = tempfile::tempdir().unwrap();
            let b = tempfile::tempdir().unwrap();
            enable_snippet_consent(a.path()).unwrap();
            assert_eq!(read_snippet_consent(a.path()).unwrap(), Some(true));
            assert_eq!(read_snippet_consent(b.path()).unwrap(), None);
        });
    }

    #[test]
    fn malformed_record_is_err_not_default() {
        with_operator_home(|home| {
            let dir = tempfile::tempdir().unwrap();
            let state = home.join(CONSENT_STATE_SUBDIR);
            fs::create_dir_all(&state).unwrap();
            let path = consent_file_path(&state, &canonical_workspace(dir.path()));
            fs::write(&path, b"{ not json").unwrap();
            let err = read_snippet_consent(dir.path()).unwrap_err();
            assert!(matches!(err, EgressConsentError::Malformed { .. }));
        });
    }

    #[test]
    fn oversized_record_is_rejected_not_read() {
        with_operator_home(|home| {
            let dir = tempfile::tempdir().unwrap();
            let state = home.join(CONSENT_STATE_SUBDIR);
            fs::create_dir_all(&state).unwrap();
            let path = consent_file_path(&state, &canonical_workspace(dir.path()));
            let blob = vec![b' '; usize::try_from(MAX_CONSENT_BYTES).unwrap() + 1];
            fs::write(&path, &blob).unwrap();
            let err = read_snippet_consent(dir.path()).unwrap_err();
            assert!(matches!(err, EgressConsentError::TooLarge { .. }));
        });
    }

    #[test]
    fn workspace_mismatch_is_rejected() {
        with_operator_home(|home| {
            let dir = tempfile::tempdir().unwrap();
            let state = home.join(CONSENT_STATE_SUBDIR);
            fs::create_dir_all(&state).unwrap();
            let canonical = canonical_workspace(dir.path());
            let path = consent_file_path(&state, &canonical);
            let forged = EgressConsentRecord {
                snippet_egress: true,
                consent_version: CONSENT_VERSION,
                workspace_root: "/some/other/workspace".into(),
            };
            fs::write(&path, serde_json::to_vec(&forged).unwrap()).unwrap();
            let err = read_snippet_consent(dir.path()).unwrap_err();
            assert!(matches!(err, EgressConsentError::WorkspaceMismatch { .. }));
        });
    }

    #[test]
    fn consent_state_dir_prefers_anvil_home_then_xdg_then_home() {
        let anvil = PathBuf::from("/tmp/anvil-home-prefix");
        let xdg = PathBuf::from("/tmp/xdg-state");
        let home = PathBuf::from("/tmp/home");
        assert_eq!(
            consent_state_dir_from(Some(anvil.clone()), Some(xdg.clone()), Some(home.clone())),
            Some(anvil)
        );
        assert_eq!(
            consent_state_dir_from(None, Some(xdg.clone()), Some(home.clone())),
            Some(xdg.join("anvil"))
        );
        assert_eq!(
            consent_state_dir_from(None, None, Some(home.clone())),
            Some(home.join(".local").join("state").join("anvil"))
        );
        assert_eq!(consent_state_dir_from(None, None, None), None);
    }

    #[cfg(unix)]
    #[test]
    fn read_refuses_symlinked_consent_file() {
        use std::os::unix::fs::symlink;
        with_operator_home(|home| {
            let dir = tempfile::tempdir().unwrap();
            let state = home.join(CONSENT_STATE_SUBDIR);
            fs::create_dir_all(&state).unwrap();
            let other = tempfile::tempdir().unwrap();
            let legit = other.path().join("legit.json");
            let record = EgressConsentRecord {
                snippet_egress: true,
                consent_version: CONSENT_VERSION,
                workspace_root: canonical_workspace(dir.path())
                    .to_string_lossy()
                    .into_owned(),
            };
            fs::write(&legit, serde_json::to_vec(&record).unwrap()).unwrap();
            let path = consent_file_path(&state, &canonical_workspace(dir.path()));
            symlink(&legit, &path).unwrap();
            let err = read_snippet_consent(dir.path()).unwrap_err();
            assert!(matches!(err, EgressConsentError::SymlinkRefusal { .. }));
        });
    }

    #[cfg(unix)]
    #[test]
    fn read_refuses_symlinked_state_dir() {
        use std::os::unix::fs::symlink;
        with_operator_home(|home| {
            let dir = tempfile::tempdir().unwrap();
            let other = tempfile::tempdir().unwrap();
            let state = home.join(CONSENT_STATE_SUBDIR);
            // Parent of state dir is ANVIL_HOME (= home); plant the state subdir
            // itself as a symlink out of operator control.
            symlink(other.path(), &state).unwrap();
            let err = read_snippet_consent(dir.path()).unwrap_err();
            assert!(matches!(err, EgressConsentError::SymlinkRefusal { .. }));
        });
    }

    #[cfg(unix)]
    #[test]
    fn enable_refuses_symlinked_state_dir() {
        use std::os::unix::fs::symlink;
        with_operator_home(|home| {
            let dir = tempfile::tempdir().unwrap();
            let other = tempfile::tempdir().unwrap();
            let state = home.join(CONSENT_STATE_SUBDIR);
            symlink(other.path(), &state).unwrap();
            let err = enable_snippet_consent(dir.path()).unwrap_err();
            assert!(matches!(err, EgressConsentError::SymlinkRefusal { .. }));
        });
    }
}
