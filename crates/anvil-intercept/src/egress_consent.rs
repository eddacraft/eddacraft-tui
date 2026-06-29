//! GCTX-024: per-workspace persisted snippet-egress consent.
//!
//! Source-text snippet egress is identity-only by default (PV-9 CE-1). An
//! operator opts a *single workspace* in with `anvil gctx egress enable`, which
//! records consent here as operator **state** (like the baseline) under the
//! repo-relative `anvil/` directory — never the hand-edited `.anvil.<ext>`
//! config. The daemon reads this record on the snippet path and feeds it to
//! [`anvil_gctx_types::resolve_snippet_egress`] (the env var overrides it; the
//! CE-1 default applies when neither is set).
//!
//! Writes are TOCTOU-hardened (symlink-refusing, atomic temp+rename), mirroring
//! the baseline writer. Reads fold an absent file into `Ok(None)` (no consent —
//! the default), but propagate symlink/IO/parse errors rather than silently
//! defaulting (operator state must never fail open by accident): the daemon logs
//! and fails safe to identity-only, the CLI surfaces the error to the operator.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Repo-relative path of the persisted snippet-egress consent record.
pub const EGRESS_CONSENT_PATH: &str = "anvil/gctx-egress.json";

/// Current consent-record schema version (CE-12 audit).
const CONSENT_VERSION: u32 = 1;

/// Upper bound on the consent file the daemon will read. The record is a tiny
/// fixed-shape JSON object; anything larger is tampering. Bounding the read
/// keeps the per-request daemon path cheap and prevents a giant file planted in
/// a (possibly hostile) repo from exhausting daemon memory.
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
    #[error("`{path}` is a symlink; refusing to read/write gctx-egress consent outside the repo")]
    SymlinkRefusal { path: PathBuf },
}

/// On-disk consent record. Minimal by design: the daemon only needs the boolean,
/// the version is for CE-12 audit / future migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EgressConsentRecord {
    /// The operator has consented to source-text snippet egress for this
    /// workspace (CE-12). An absent file means no consent — the CE-1 default.
    snippet_egress: bool,
    #[serde(default = "default_version")]
    consent_version: u32,
}

fn default_version() -> u32 {
    CONSENT_VERSION
}

/// Read the persisted snippet-egress consent for `repo_root`.
///
/// - absent file → `Ok(None)` (no consent recorded — the CE-1 default),
/// - present + valid → `Ok(Some(snippet_egress))`,
/// - symlinked path / IO error / malformed content → `Err` (never silently
///   folded to the default).
pub fn read_snippet_consent(repo_root: &Path) -> Result<Option<bool>, EgressConsentError> {
    let parent = repo_root.join("anvil");
    refuse_if_symlink(&parent)?;
    let path = repo_root.join(EGRESS_CONSENT_PATH);
    refuse_if_symlink(&path)?;
    if !path.exists() {
        return Ok(None);
    }
    let size = fs::metadata(&path)?.len();
    if size > MAX_CONSENT_BYTES {
        return Err(EgressConsentError::TooLarge {
            path: path.clone(),
            size,
        });
    }
    let bytes = fs::read(&path)?;
    let record: EgressConsentRecord =
        serde_json::from_slice(&bytes).map_err(|source| EgressConsentError::Malformed {
            path: path.clone(),
            source,
        })?;
    Ok(Some(record.snippet_egress))
}

/// Persist the operator's consent to snippet egress for `repo_root` (atomic
/// temp+rename, symlink-refusing). Idempotent — re-enabling overwrites the same
/// record.
pub fn enable_snippet_consent(repo_root: &Path) -> Result<(), EgressConsentError> {
    let parent = repo_root.join("anvil");
    refuse_if_symlink(&parent)?;
    fs::create_dir_all(&parent)?;
    // TOCTOU: re-check after create so a racing process cannot swap `anvil/`
    // for a symlink between the pre-check and the write.
    refuse_if_symlink(&parent)?;

    let final_path = repo_root.join(EGRESS_CONSENT_PATH);
    refuse_if_symlink(&final_path)?;

    // Process-unique temp name so two concurrent `enable` invocations do not
    // stomp the same scratch file (both write the identical record, but a shared
    // temp name would race the rename to a spurious ENOENT).
    let tmp_path = parent.join(format!(".gctx-egress.json.{}.tmp", std::process::id()));
    refuse_if_symlink(&tmp_path)?;

    let record = EgressConsentRecord {
        snippet_egress: true,
        consent_version: CONSENT_VERSION,
    };
    let bytes = serde_json::to_vec_pretty(&record).expect("egress consent record serialises");
    {
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    refuse_if_symlink(&final_path)?;
    // `std::fs::rename` replaces an existing destination atomically on both Unix
    // (`rename(2)`) and Windows (`MoveFileExW(MOVEFILE_REPLACE_EXISTING)`), so a
    // re-enable over an existing record is safe and idempotent.
    fs::rename(&tmp_path, &final_path)?;
    Ok(())
}

/// Revoke consent for `repo_root` by removing the record — a clean revert to the
/// CE-1 identity-only default. Idempotent: a no-op when no record exists.
pub fn disable_snippet_consent(repo_root: &Path) -> Result<(), EgressConsentError> {
    let parent = repo_root.join("anvil");
    refuse_if_symlink(&parent)?;
    let path = repo_root.join(EGRESS_CONSENT_PATH);
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

    #[test]
    fn read_absent_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_snippet_consent(dir.path()).unwrap(), None);
    }

    #[test]
    fn enable_then_read_is_some_true() {
        let dir = tempfile::tempdir().unwrap();
        enable_snippet_consent(dir.path()).unwrap();
        assert_eq!(read_snippet_consent(dir.path()).unwrap(), Some(true));
        assert!(dir.path().join(EGRESS_CONSENT_PATH).is_file());
    }

    #[test]
    fn disable_removes_record_and_reverts_to_default() {
        let dir = tempfile::tempdir().unwrap();
        enable_snippet_consent(dir.path()).unwrap();
        disable_snippet_consent(dir.path()).unwrap();
        assert_eq!(read_snippet_consent(dir.path()).unwrap(), None);
        assert!(!dir.path().join(EGRESS_CONSENT_PATH).exists());
    }

    #[test]
    fn disable_is_idempotent_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        // No record yet — disable must not error.
        disable_snippet_consent(dir.path()).unwrap();
    }

    #[test]
    fn enable_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        enable_snippet_consent(dir.path()).unwrap();
        enable_snippet_consent(dir.path()).unwrap();
        assert_eq!(read_snippet_consent(dir.path()).unwrap(), Some(true));
    }

    #[test]
    fn malformed_record_is_err_not_default() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        fs::write(dir.path().join(EGRESS_CONSENT_PATH), b"{ not json").unwrap();
        let err = read_snippet_consent(dir.path()).unwrap_err();
        assert!(matches!(err, EgressConsentError::Malformed { .. }));
    }

    #[test]
    fn oversized_record_is_rejected_not_read() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        let blob = vec![b' '; usize::try_from(MAX_CONSENT_BYTES).unwrap() + 1];
        fs::write(dir.path().join(EGRESS_CONSENT_PATH), &blob).unwrap();
        let err = read_snippet_consent(dir.path()).unwrap_err();
        assert!(matches!(err, EgressConsentError::TooLarge { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn read_refuses_symlinked_consent_file() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        let other = tempfile::tempdir().unwrap();
        fs::write(
            other.path().join("legit.json"),
            b"{\"snippet_egress\":true}",
        )
        .unwrap();
        // The consent file itself is a symlink out of the repo — must be refused,
        // not followed (a hostile worktree could otherwise fabricate consent).
        symlink(
            other.path().join("legit.json"),
            dir.path().join(EGRESS_CONSENT_PATH),
        )
        .unwrap();
        let err = read_snippet_consent(dir.path()).unwrap_err();
        assert!(matches!(err, EgressConsentError::SymlinkRefusal { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn read_refuses_symlinked_anvil_dir() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        symlink(other.path(), dir.path().join("anvil")).unwrap();
        let err = read_snippet_consent(dir.path()).unwrap_err();
        assert!(matches!(err, EgressConsentError::SymlinkRefusal { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn enable_refuses_symlinked_anvil_dir() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        symlink(other.path(), dir.path().join("anvil")).unwrap();
        let err = enable_snippet_consent(dir.path()).unwrap_err();
        assert!(matches!(err, EgressConsentError::SymlinkRefusal { .. }));
    }
}
