//! Policy constraint layer (SETCON-005).
//!
//! Constraints evaluate after ordinary resolution. A local, environment or
//! session declaration cannot escape a controlling constraint by having
//! higher precedence. Unverifiable, expired or incompatible bundles fail
//! closed and cannot select their own failure behaviour.

use serde::Serialize;
use serde_json::Value;

use crate::canonical_json::canonicalise;
use crate::resolver::ResolvedSetting;
use crate::types::{Posture, Scope};

/// One constraint on the permitted value space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Constraint {
    RequireValue { key: String, value: Value },
    ProhibitValue { key: String, value: Value },
    MinPosture { key: String, min: Posture },
    MaxPosture { key: String, max: Posture },
    MandateMember { key: String, member: Value },
    ForbidMember { key: String, member: Value },
    RestrictOverrideScope { key: String, allowed: Vec<Scope> },
    RequireApproval { key: String, authority: String },
}

/// Signed-bundle stand-in. SETCON does not author or distribute policy; it
/// decides whether a bundle may constrain a settings row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyBundle {
    pub id: String,
    pub verifiable: bool,
    pub expired: bool,
    pub compatible: bool,
    pub constraints: Vec<Constraint>,
}

impl PolicyBundle {
    /// Immutable digest of the bundle identity and ordered constraint content.
    #[must_use]
    pub fn content_digest(&self) -> String {
        use sha2::{Digest, Sha256};
        let payload = serde_json::json!({
            "id": self.id,
            "constraints": self.constraints,
        });
        let bytes = serde_json::to_vec(&canonicalise(&payload)).unwrap_or_default();
        hex::encode(Sha256::digest(bytes))
    }
}

/// Verified approval evidence supplied at the policy-evaluation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalEvidence {
    pub bundle_id: String,
    pub bundle_digest: String,
    pub key: String,
    pub authority: String,
    pub verified: bool,
    pub valid_until: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConstraintError {
    #[error("policy bundle {0} is unverifiable")]
    Unverifiable(String),
    #[error("policy bundle {0} is expired")]
    Expired(String),
    #[error("policy bundle {0} is incompatible")]
    Incompatible(String),
    #[error("approval required for {key} from {authority}")]
    ApprovalRequired { key: String, authority: String },
    #[error("constraint violation on {key}: {reason}")]
    Violated { key: String, reason: String },
}

/// Apply constraints to already-resolved (requested) values.
///
/// Returns the constrained resolved values, or a fail-closed error. Bundles
/// cannot choose to be ignored.
pub fn apply_constraints(
    requested: &[ResolvedSetting],
    bundle: Option<&PolicyBundle>,
) -> Result<Vec<ResolvedSetting>, ConstraintError> {
    apply_constraints_with_approvals(requested, bundle, &[], "")
}

/// Apply constraints with approval evidence already verified by the caller's
/// trust boundary.
pub fn apply_constraints_with_approvals(
    requested: &[ResolvedSetting],
    bundle: Option<&PolicyBundle>,
    approvals: &[ApprovalEvidence],
    now: &str,
) -> Result<Vec<ResolvedSetting>, ConstraintError> {
    let Some(bundle) = bundle else {
        return Ok(requested.to_vec());
    };
    if !bundle.verifiable {
        return Err(ConstraintError::Unverifiable(bundle.id.clone()));
    }
    if bundle.expired {
        return Err(ConstraintError::Expired(bundle.id.clone()));
    }
    if !bundle.compatible {
        return Err(ConstraintError::Incompatible(bundle.id.clone()));
    }

    let bundle_digest = bundle.content_digest();
    let mut out = requested.to_vec();
    for constraint in &bundle.constraints {
        apply_one(
            &mut out,
            constraint,
            &bundle.id,
            &bundle_digest,
            approvals,
            now,
        )?;
    }
    Ok(out)
}

fn apply_one(
    rows: &mut [ResolvedSetting],
    constraint: &Constraint,
    bundle_id: &str,
    bundle_digest: &str,
    approvals: &[ApprovalEvidence],
    now: &str,
) -> Result<(), ConstraintError> {
    match constraint {
        Constraint::RequireValue { key, value } => {
            let row = row_mut(rows, key)?;
            if row.resolved.as_ref() != Some(value) {
                row.resolved = Some(value.clone());
            }
            Ok(())
        }
        Constraint::ProhibitValue { key, value } => {
            let row = row_mut(rows, key)?;
            if row.resolved.as_ref() == Some(value) {
                return Err(ConstraintError::Violated {
                    key: key.clone(),
                    reason: "prohibited value".into(),
                });
            }
            Ok(())
        }
        Constraint::MinPosture { key, min } => {
            let row = row_mut(rows, key)?;
            let Some(current) = row.resolved.as_ref().and_then(as_posture) else {
                row.resolved = Some(Value::String(posture_name(*min).into()));
                return Ok(());
            };
            if current < *min {
                row.resolved = Some(Value::String(posture_name(*min).into()));
            }
            Ok(())
        }
        Constraint::MaxPosture { key, max } => {
            let row = row_mut(rows, key)?;
            if let Some(current) = row.resolved.as_ref().and_then(as_posture)
                && current > *max
            {
                return Err(ConstraintError::Violated {
                    key: key.clone(),
                    reason: "exceeds maximum posture".into(),
                });
            }
            Ok(())
        }
        Constraint::MandateMember { key, member } => {
            let row = row_mut(rows, key)?;
            let mut items = list_of(row.resolved.clone());
            if !items.contains(member) {
                items.push(member.clone());
            }
            row.resolved = Some(Value::Array(items));
            Ok(())
        }
        Constraint::ForbidMember { key, member } => {
            let row = row_mut(rows, key)?;
            let items: Vec<Value> = list_of(row.resolved.clone())
                .into_iter()
                .filter(|item| item != member)
                .collect();
            row.resolved = Some(Value::Array(items));
            Ok(())
        }
        Constraint::RestrictOverrideScope { key, allowed } => {
            let row =
                rows.iter()
                    .find(|r| r.key == *key)
                    .ok_or_else(|| ConstraintError::Violated {
                        key: key.clone(),
                        reason: "missing key".into(),
                    })?;
            let disallowed: Vec<&str> = row
                .provenance
                .iter()
                .filter(|p| !allowed.contains(&p.scope))
                .map(|p| p.source_id.as_str())
                .collect();
            if !disallowed.is_empty() {
                return Err(ConstraintError::Violated {
                    key: key.clone(),
                    reason: format!("override scope not permitted ({})", disallowed.join(",")),
                });
            }
            Ok(())
        }
        Constraint::RequireApproval { key, authority } => {
            row_mut(rows, key)?;
            if approvals.iter().any(|evidence| {
                approval_is_current(evidence, bundle_id, bundle_digest, key, authority, now)
            }) {
                return Ok(());
            }
            Err(ConstraintError::ApprovalRequired {
                key: key.clone(),
                authority: authority.clone(),
            })
        }
    }
}

fn approval_is_current(
    evidence: &ApprovalEvidence,
    bundle_id: &str,
    bundle_digest: &str,
    key: &str,
    authority: &str,
    now: &str,
) -> bool {
    use chrono::DateTime;
    evidence.verified
        && evidence.bundle_id == bundle_id
        && evidence.bundle_digest == bundle_digest
        && evidence.key == key
        && evidence.authority == authority
        && DateTime::parse_from_rfc3339(now)
            .ok()
            .zip(DateTime::parse_from_rfc3339(&evidence.valid_until).ok())
            .is_some_and(|(now, valid_until)| now < valid_until)
}

fn row_mut<'a>(
    rows: &'a mut [ResolvedSetting],
    key: &str,
) -> Result<&'a mut ResolvedSetting, ConstraintError> {
    rows.iter_mut()
        .find(|r| r.key == key)
        .ok_or_else(|| ConstraintError::Violated {
            key: key.to_owned(),
            reason: "missing key".into(),
        })
}

fn as_posture(value: &Value) -> Option<Posture> {
    value.as_str().and_then(Posture::parse)
}

fn posture_name(posture: Posture) -> &'static str {
    posture.as_str()
}

fn list_of(value: Option<Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items,
        Some(other) => vec![other],
        None => Vec::new(),
    }
}

#[cfg(test)]
mod constraints_tests {
    use super::*;
    use crate::resolver::{ProvenanceEvent, ResolutionEvent};
    use crate::types::Scope;

    fn row(key: &str, value: Value, scope: Scope) -> ResolvedSetting {
        ResolvedSetting {
            key: key.into(),
            requested: Some(value.clone()),
            resolved: Some(value.clone()),
            provenance: vec![ProvenanceEvent {
                source_id: "project".into(),
                scope,
                event: ResolutionEvent::Set(value),
                overridden: false,
            }],
        }
    }

    #[test]
    fn constraints_unverified_bundle_is_never_ignored() {
        let bundle = PolicyBundle {
            id: "org-1".into(),
            verifiable: false,
            expired: false,
            compatible: true,
            constraints: vec![],
        };
        let err = apply_constraints(&[], Some(&bundle)).unwrap_err();
        assert_eq!(err, ConstraintError::Unverifiable("org-1".into()));
    }

    #[test]
    fn constraints_expired_bundle_is_never_ignored() {
        let bundle = PolicyBundle {
            id: "org-1".into(),
            verifiable: true,
            expired: true,
            compatible: true,
            constraints: vec![],
        };
        assert_eq!(
            apply_constraints(&[], Some(&bundle)).unwrap_err(),
            ConstraintError::Expired("org-1".into())
        );
    }

    #[test]
    fn constraints_incompatible_bundle_is_never_ignored() {
        let bundle = PolicyBundle {
            id: "org-1".into(),
            verifiable: true,
            expired: false,
            compatible: false,
            constraints: vec![],
        };
        assert_eq!(
            apply_constraints(&[], Some(&bundle)).unwrap_err(),
            ConstraintError::Incompatible("org-1".into())
        );
    }

    #[test]
    fn constraints_org_floor_beats_higher_precedence_project_declaration() {
        let requested = vec![row(
            "protection.enforcement.mode",
            Value::String("warn".into()),
            Scope::Project,
        )];
        let bundle = PolicyBundle {
            id: "org-1".into(),
            verifiable: true,
            expired: false,
            compatible: true,
            constraints: vec![Constraint::MinPosture {
                key: "protection.enforcement.mode".into(),
                min: Posture::Interrupt,
            }],
        };
        let out = apply_constraints(&requested, Some(&bundle)).unwrap();
        assert_eq!(out[0].resolved, Some(Value::String("interrupt".into())));
    }

    #[test]
    fn constraints_session_cannot_override_forbidden_scope() {
        let requested = vec![row(
            "privacy.gctx_egress",
            Value::Bool(true),
            Scope::Session,
        )];
        let bundle = PolicyBundle {
            id: "org-1".into(),
            verifiable: true,
            expired: false,
            compatible: true,
            constraints: vec![Constraint::RestrictOverrideScope {
                key: "privacy.gctx_egress".into(),
                allowed: vec![Scope::Org, Scope::Team],
            }],
        };
        let err = apply_constraints(&requested, Some(&bundle)).unwrap_err();
        match err {
            ConstraintError::Violated { key, .. } => {
                assert_eq!(key, "privacy.gctx_egress");
            }
            other => panic!("expected violated, got {other:?}"),
        }
    }

    const APPROVAL_NOW: &str = "2026-08-29T00:00:00Z";

    fn approval_bundle() -> PolicyBundle {
        PolicyBundle {
            id: "org-1".into(),
            verifiable: true,
            expired: false,
            compatible: true,
            constraints: vec![Constraint::RequireApproval {
                key: "privacy.gctx_egress".into(),
                authority: "security".into(),
            }],
        }
    }

    fn valid_approval() -> ApprovalEvidence {
        ApprovalEvidence {
            bundle_id: "org-1".into(),
            bundle_digest: approval_bundle().content_digest(),
            key: "privacy.gctx_egress".into(),
            authority: "security".into(),
            verified: true,
            valid_until: "2026-08-29T01:00:00Z".into(),
        }
    }

    fn assert_approval_required(evidence: Option<&ApprovalEvidence>) {
        let requested = vec![row(
            "privacy.gctx_egress",
            Value::Bool(false),
            Scope::Project,
        )];
        let bundle = approval_bundle();
        let approvals: &[ApprovalEvidence] = evidence.map(std::slice::from_ref).unwrap_or_default();
        assert_eq!(
            apply_constraints_with_approvals(&requested, Some(&bundle), approvals, APPROVAL_NOW,)
                .unwrap_err(),
            ConstraintError::ApprovalRequired {
                key: "privacy.gctx_egress".into(),
                authority: "security".into(),
            }
        );
    }

    #[test]
    fn constraints_reject_missing_or_wrong_authority_approval() {
        assert_approval_required(None);
        let mut evidence = valid_approval();
        evidence.authority = "operations".into();
        assert_approval_required(Some(&evidence));
    }

    #[test]
    fn constraints_reject_expired_or_unverified_approval() {
        let mut evidence = valid_approval();
        evidence.valid_until = "2026-08-28T23:59:59Z".into();
        assert_approval_required(Some(&evidence));
        evidence.valid_until = "2026-08-29T01:00:00Z".into();
        evidence.verified = false;
        assert_approval_required(Some(&evidence));
    }

    #[test]
    fn constraints_bind_approval_to_bundle_and_key() {
        let mut evidence = valid_approval();
        evidence.bundle_id = "different-bundle".into();
        assert_approval_required(Some(&evidence));
        evidence.bundle_id = "org-1".into();
        evidence.key = "different.key".into();
        assert_approval_required(Some(&evidence));
    }

    #[test]
    fn constraints_accept_verified_current_approval_evidence() {
        let requested = vec![row(
            "privacy.gctx_egress",
            Value::Bool(false),
            Scope::Project,
        )];
        let bundle = approval_bundle();
        assert!(
            apply_constraints_with_approvals(
                &requested,
                Some(&bundle),
                &[valid_approval()],
                APPROVAL_NOW,
            )
            .is_ok()
        );
    }

    #[test]
    fn constraints_bind_approval_to_immutable_bundle_content() {
        let original = approval_bundle();
        let evidence = ApprovalEvidence {
            bundle_digest: original.content_digest(),
            ..valid_approval()
        };
        let mut changed = original;
        changed.constraints.insert(
            0,
            Constraint::RequireValue {
                key: "privacy.gctx_egress".into(),
                value: Value::Bool(false),
            },
        );
        let requested = vec![row(
            "privacy.gctx_egress",
            Value::Bool(false),
            Scope::Project,
        )];

        assert_eq!(
            apply_constraints_with_approvals(
                &requested,
                Some(&changed),
                &[evidence],
                APPROVAL_NOW,
            )
            .unwrap_err(),
            ConstraintError::ApprovalRequired {
                key: "privacy.gctx_egress".into(),
                authority: "security".into(),
            }
        );
    }
}
