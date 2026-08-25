//! Policy constraint layer (SETCON-005).
//!
//! Constraints evaluate after ordinary resolution. A local, environment or
//! session declaration cannot escape a controlling constraint by having
//! higher precedence. Unverifiable, expired or incompatible bundles fail
//! closed and cannot select their own failure behaviour.

use serde_json::Value;

use crate::resolver::ResolvedSetting;
use crate::types::{Posture, Scope};

/// One constraint on the permitted value space.
#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConstraintError {
    #[error("policy bundle {0} is unverifiable")]
    Unverifiable(String),
    #[error("policy bundle {0} is expired")]
    Expired(String),
    #[error("policy bundle {0} is incompatible")]
    Incompatible(String),
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

    let mut out = requested.to_vec();
    for constraint in &bundle.constraints {
        apply_one(&mut out, constraint)?;
    }
    Ok(out)
}

fn apply_one(rows: &mut [ResolvedSetting], constraint: &Constraint) -> Result<(), ConstraintError> {
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
        Constraint::RequireApproval { .. } => {
            // Approval is recorded on the row; SETGOV owns the workflow.
            Ok(())
        }
    }
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
    match posture {
        Posture::Off => "off",
        Posture::Warn => "warn",
        Posture::Enforce => "enforce",
    }
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
                min: Posture::Enforce,
            }],
        };
        let out = apply_constraints(&requested, Some(&bundle)).unwrap();
        assert_eq!(out[0].resolved, Some(Value::String("enforce".into())));
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
}
