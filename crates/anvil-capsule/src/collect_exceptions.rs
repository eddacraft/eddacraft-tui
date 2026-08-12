//! EXCEPT-009 collector: exception grants recorded into the capsule.
//!
//! Collects the **active** grants from the tracked exception store
//! (`anvil/exceptions/store.json`, ADR-073) so the capsule names the
//! exceptions the change could have relied on, and `anvil capsule
//! verify` re-verifies them (scope/expiry/revocation/attribution) via
//! the existing `exceptions` check.
//!
//! Honesty note: "applied" is approximated by "active at collect
//! time". A faithful relied-upon subset needs the gate's
//! applied-exception record joined against a diagnostics source, and
//! capsule create has no diagnostics source wired yet —
//! `diagnostics.sarif` is an empty document for the same reason. The
//! approximation is conservative for verification: every grant the
//! gate *could* have applied is in the capsule and gets re-verified,
//! so a revoked-after-the-fact or expiring grant still degrades or
//! blocks `capsule verify`.
//!
//! Collection semantics mirror the crate's evidence discipline:
//! - Store absent → empty collection (absence is honest; the verify
//!   check reports "no applied exceptions" and passes).
//! - Store present but unreadable (unparseable / oversized /
//!   symlinked) → [`CapsuleError::Collect`], loudly — a capsule must
//!   not misrepresent present-but-broken governance state as absence.
//! - Revoked and expired grants are **not** collected: they cannot
//!   apply at the gate, and the store keeps them for audit — the
//!   capsule is evidence of what could apply, not a store dump.

use std::path::Path;

use anvil_policy::exceptions::{ExceptionStore, PolicyException};
use chrono::{DateTime, Utc};

use crate::canonical::canonical_json_bytes;
use crate::errors::CapsuleError;

/// The collected active grants, ready for `exceptions.json`.
#[derive(Debug, Clone)]
pub struct CollectedExceptions {
    /// Active grants at collect time, in store order.
    pub exceptions: Vec<PolicyException>,
}

impl CollectedExceptions {
    /// Canonical bytes for `exceptions.json` — the `Vec<PolicyException>`
    /// shape the verifier's `exceptions` check reads back.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CapsuleError> {
        let value = serde_json::to_value(&self.exceptions)
            .map_err(|e| CapsuleError::Serialise(e.to_string()))?;
        canonical_json_bytes(&value).map_err(|e| CapsuleError::Serialise(e.to_string()))
    }
}

/// Collect active exception grants from the tracked store at the
/// current time. See the module docs for absence/error semantics.
pub fn collect_exceptions(repo_root: &Path) -> Result<CollectedExceptions, CapsuleError> {
    collect_exceptions_at(repo_root, Utc::now())
}

/// [`collect_exceptions`] at an explicit `now` — deterministic
/// expiry-boundary evaluation for tests and crate-internal callers.
pub(crate) fn collect_exceptions_at(
    repo_root: &Path,
    now: DateTime<Utc>,
) -> Result<CollectedExceptions, CapsuleError> {
    let store = ExceptionStore::load(repo_root).map_err(|e| CapsuleError::Collect {
        // Name the store that actually failed: load() falls back to
        // the legacy local file when the tracked store is absent.
        path: if repo_root.join("anvil/exceptions/store.json").exists() {
            "anvil/exceptions/store.json".to_string()
        } else {
            ".anvil/exceptions.json".to_string()
        },
        detail: e.to_string(),
    })?;
    Ok(CollectedExceptions {
        exceptions: store
            .active_exceptions_at(now)
            .into_iter()
            .cloned()
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_policy::exceptions::StoreSource;
    use chrono::Duration;
    use tempfile::TempDir;

    fn grant(policy_id: &str, expires_at: Option<DateTime<Utc>>) -> PolicyException {
        PolicyException {
            schema_version: String::new(),
            id: String::new(),
            policy_id: policy_id.to_string(),
            file_pattern: "src/**".to_string(),
            finding_hash: None,
            reason: "test grant".to_string(),
            owner: Some("team-platform".to_string()),
            created_by: Some("alice@example.test".to_string()),
            created_at: Utc::now(),
            expires_at,
            revoked: None,
        }
    }

    fn save(root: &Path, grants: Vec<PolicyException>) {
        let mut store = ExceptionStore::empty();
        for g in grants {
            store.add(g).unwrap();
        }
        let outcome = store.save(root).expect("write tracked store");
        assert!(matches!(
            outcome,
            anvil_policy::exceptions::WriteOutcome::Written
        ));
    }

    /// Active grants are collected; the canonical bytes parse back as
    /// the `Vec<PolicyException>` shape the verifier reads.
    #[test]
    fn collect_exceptions_captures_active_grants() {
        let tmp = TempDir::new().unwrap();
        save(tmp.path(), vec![grant("AP-001", None)]);
        let collected = collect_exceptions(tmp.path()).unwrap();
        assert_eq!(collected.exceptions.len(), 1);
        assert_eq!(collected.exceptions[0].policy_id, "AP-001");
        let bytes = collected.to_canonical_bytes().unwrap();
        let round: Vec<PolicyException> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(round.len(), 1);
    }

    /// Absent store → empty collection, no error (absence is honest).
    #[test]
    fn collect_exceptions_absent_store_is_empty() {
        let tmp = TempDir::new().unwrap();
        let collected = collect_exceptions(tmp.path()).unwrap();
        assert!(collected.exceptions.is_empty());
        assert_eq!(collected.to_canonical_bytes().unwrap(), b"[]");
    }

    /// Revoked and expired grants cannot apply at the gate and are not
    /// collected — the capsule is evidence, not a store dump.
    #[test]
    fn collect_exceptions_skips_revoked_and_expired() {
        let tmp = TempDir::new().unwrap();
        let mut revoked = grant("AP-001", None);
        revoked.revoked = Some(anvil_policy::exceptions::ExceptionRevocation {
            revoked_at: Utc::now(),
            revoked_by: "bob".to_string(),
            reason: "done".to_string(),
        });
        let expired = grant("AP-002", Some(Utc::now() - Duration::days(1)));
        let live = grant("AP-003", Some(Utc::now() + Duration::days(30)));
        save(tmp.path(), vec![revoked, expired, live]);
        let collected = collect_exceptions(tmp.path()).unwrap();
        assert_eq!(collected.exceptions.len(), 1);
        assert_eq!(collected.exceptions[0].policy_id, "AP-003");
    }

    /// Deterministic expiry boundary: a grant expiring exactly at
    /// `now` is still active (expiry is strict `>`), one second past
    /// is not. Drives `collect_exceptions_at` directly.
    #[test]
    fn collect_exceptions_at_expiry_boundary_is_deterministic() {
        let tmp = TempDir::new().unwrap();
        let now = Utc::now();
        save(tmp.path(), vec![grant("AP-001", Some(now))]);
        let at_boundary = collect_exceptions_at(tmp.path(), now).unwrap();
        assert_eq!(at_boundary.exceptions.len(), 1, "expiry is exclusive");
        let past = collect_exceptions_at(tmp.path(), now + Duration::seconds(1)).unwrap();
        assert!(past.exceptions.is_empty(), "past expiry must not collect");
    }

    /// A broken LEGACY store names the legacy file in the error, not
    /// the tracked path that does not exist.
    #[test]
    fn collect_exceptions_broken_legacy_store_names_legacy_path() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(tmp.path().join(".anvil/exceptions.json"), "{not json").unwrap();
        let err = collect_exceptions(tmp.path()).expect_err("broken legacy must fail");
        assert!(
            matches!(err, CapsuleError::Collect { ref path, .. } if path == ".anvil/exceptions.json"),
            "{err:?}",
        );
    }

    /// A present-but-broken store fails collection loudly — the
    /// capsule must not misrepresent broken governance state as
    /// absence.
    #[test]
    fn collect_exceptions_unreadable_store_errors() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("anvil/exceptions")).unwrap();
        std::fs::write(tmp.path().join("anvil/exceptions/store.json"), "{not json").unwrap();
        let err = collect_exceptions(tmp.path()).expect_err("broken store must fail collection");
        assert!(
            matches!(err, CapsuleError::Collect { ref path, .. } if path.contains("exceptions")),
            "{err:?}",
        );
    }

    /// A legacy-only store still collects (read fallback) — the grants
    /// exist and could apply at the gate, so the capsule names them.
    #[test]
    fn collect_exceptions_reads_legacy_fallback() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        let store_json = serde_json::json!({ "exceptions": [grant("AP-009", None)] });
        std::fs::write(
            tmp.path().join(".anvil/exceptions.json"),
            serde_json::to_vec(&store_json).unwrap(),
        )
        .unwrap();
        let collected = collect_exceptions(tmp.path()).unwrap();
        assert_eq!(collected.exceptions.len(), 1);
        assert_eq!(collected.exceptions[0].policy_id, "AP-009");
        // Source provenance sanity: the fallback really was legacy.
        let store = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(store.source(), StoreSource::Legacy);
    }
}
