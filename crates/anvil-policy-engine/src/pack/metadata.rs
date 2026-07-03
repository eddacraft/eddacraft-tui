//! Policy and pack metadata schema (POLVAL-001).
//!
//! Validation runs before load at the facade boundary (ADR-040 D-2): a pack
//! whose policies carry incomplete metadata is rejected with an error naming
//! the policy, the field, and the fix, rather than failing silently at gate
//! evaluation. This module owns pure data shapes and their validation only —
//! no policy evaluation.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Severity band a policy declares for the findings it emits.
///
/// A dedicated four-level band (rather than the engine's binary
/// [`crate::Severity`]) lets pack authors and downstream reporting rank
/// policies. The wire form is lowercase (`low`/`medium`/`high`/`critical`); an
/// unrecognised value is a deserialisation error, so a typo fails closed
/// instead of defaulting to a silent band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicySeverity {
    /// Advisory; lowest rank.
    Low,
    /// Default operational concern.
    Medium,
    /// Should be addressed before release.
    High,
    /// Must not ship; highest rank.
    Critical,
}

/// Required metadata every policy in a pack must carry.
///
/// Every field is `#[serde(default)]` so an incomplete entry still
/// deserialises and is reported by [`PolicyMetadata::validate`] naming the
/// offending policy and field. A missing field is therefore a *validation*
/// error, not a parse error, which is what lets the message cite the policy id.
/// `severity` is the exception: a *present but invalid* band is a parse error
/// (fail closed on a typo), while an *absent* band is reported by
/// [`PolicyMetadata::validate`]. Unknown fields are rejected so a mistyped key
/// (e.g. `owners`) cannot silently drop a required value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyMetadata {
    /// Unique policy identifier within the pack.
    #[serde(default)]
    pub id: String,
    /// Human-readable one-line title.
    #[serde(default)]
    pub title: String,
    /// Declared severity band; absent until validated.
    #[serde(default)]
    pub severity: Option<PolicySeverity>,
    /// Accountable owner (team or individual).
    #[serde(default)]
    pub owner: String,
    /// Why the policy exists — the rationale surfaced to authors.
    #[serde(default)]
    pub rationale: String,
    /// What the policy applies to (its evaluation scope).
    #[serde(default)]
    pub scope: String,
    /// Classification tags; at least one non-blank tag is required.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A metadata completeness or uniqueness failure.
///
/// Every variant names the offending policy (where known) and the fix, so the
/// message is actionable without further context. User-facing text uses UK
/// spelling.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MetadataError {
    /// A policy entry has no `id`, so nothing else can be attributed to it.
    #[error("a policy entry is missing its `id`; give every policy a unique, non-blank id")]
    MissingId,
    /// A required field is absent or blank on an identified policy.
    #[error(
        "policy `{policy_id}` is missing required metadata field `{field}`; \
         set a non-blank `{field}` value"
    )]
    MissingField {
        /// The `id` of the offending policy.
        policy_id: String,
        /// The name of the missing field.
        field: &'static str,
    },
    /// Two policies in the same pack share an `id`.
    #[error("duplicate policy id `{0}` in pack; each policy id must be unique")]
    DuplicateId(String),
}

impl PolicyMetadata {
    /// Validate that every required field is present and non-blank.
    ///
    /// A blank or whitespace-only string counts as missing. `id` is checked
    /// first so that any subsequent error can cite it. `tags` must contain at
    /// least one non-blank entry. `severity` must be present (an invalid band
    /// is already rejected at deserialisation time).
    pub fn validate(&self) -> Result<(), MetadataError> {
        let id = self.id.trim();
        if id.is_empty() {
            return Err(MetadataError::MissingId);
        }

        for (field, value) in [
            ("title", self.title.as_str()),
            ("owner", self.owner.as_str()),
            ("rationale", self.rationale.as_str()),
            ("scope", self.scope.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(MetadataError::MissingField {
                    policy_id: id.to_string(),
                    field,
                });
            }
        }

        if self.severity.is_none() {
            return Err(MetadataError::MissingField {
                policy_id: id.to_string(),
                field: "severity",
            });
        }

        if self.tags.iter().all(|tag| tag.trim().is_empty()) {
            return Err(MetadataError::MissingField {
                policy_id: id.to_string(),
                field: "tags",
            });
        }

        Ok(())
    }
}

/// Confirm that a set of policy metadata entries have unique, non-blank ids.
///
/// Ids are compared after trimming. Returns the first
/// [`MetadataError::DuplicateId`] in iteration order (deterministic), or
/// [`MetadataError::MissingId`] if any entry lacks an id.
pub fn ensure_unique_ids(entries: &[PolicyMetadata]) -> Result<(), MetadataError> {
    let mut seen = std::collections::BTreeSet::new();
    for entry in entries {
        let id = entry.id.trim();
        if id.is_empty() {
            return Err(MetadataError::MissingId);
        }
        if !seen.insert(id) {
            return Err(MetadataError::DuplicateId(id.to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_metadata() -> PolicyMetadata {
        PolicyMetadata {
            id: "no-network-imports".into(),
            title: "Disallow new network imports".into(),
            severity: Some(PolicySeverity::High),
            owner: "platform-security".into(),
            rationale: "New network edges widen the blast radius of a breach.".into(),
            scope: "src/**/*.rs".into(),
            tags: vec!["security".into(), "imports".into()],
        }
    }

    #[test]
    fn policy_metadata_valid_entry_passes() {
        assert_eq!(valid_metadata().validate(), Ok(()));
    }

    #[test]
    fn policy_metadata_severity_round_trips_lowercase() {
        let yaml = serde_yaml::to_string(&PolicySeverity::Critical).expect("serialise");
        assert_eq!(yaml.trim(), "critical");
        let parsed: PolicySeverity = serde_yaml::from_str("high").expect("deserialise");
        assert_eq!(parsed, PolicySeverity::High);
    }

    #[test]
    fn policy_metadata_missing_id_is_reported() {
        let meta = PolicyMetadata {
            id: "   ".into(),
            ..valid_metadata()
        };
        assert_eq!(meta.validate(), Err(MetadataError::MissingId));
    }

    #[test]
    fn policy_metadata_missing_title_cites_policy_and_field() {
        let meta = PolicyMetadata {
            title: String::new(),
            ..valid_metadata()
        };
        assert_eq!(
            meta.validate(),
            Err(MetadataError::MissingField {
                policy_id: "no-network-imports".into(),
                field: "title",
            })
        );
    }

    #[test]
    fn policy_metadata_missing_owner_is_reported() {
        let meta = PolicyMetadata {
            owner: "  ".into(),
            ..valid_metadata()
        };
        assert!(matches!(
            meta.validate(),
            Err(MetadataError::MissingField { field: "owner", .. })
        ));
    }

    #[test]
    fn policy_metadata_missing_rationale_is_reported() {
        let meta = PolicyMetadata {
            rationale: String::new(),
            ..valid_metadata()
        };
        assert!(matches!(
            meta.validate(),
            Err(MetadataError::MissingField {
                field: "rationale",
                ..
            })
        ));
    }

    #[test]
    fn policy_metadata_missing_scope_is_reported() {
        let meta = PolicyMetadata {
            scope: String::new(),
            ..valid_metadata()
        };
        assert!(matches!(
            meta.validate(),
            Err(MetadataError::MissingField { field: "scope", .. })
        ));
    }

    #[test]
    fn policy_metadata_missing_severity_is_reported() {
        let meta = PolicyMetadata {
            severity: None,
            ..valid_metadata()
        };
        assert!(matches!(
            meta.validate(),
            Err(MetadataError::MissingField {
                field: "severity",
                ..
            })
        ));
    }

    #[test]
    fn policy_metadata_blank_tags_are_reported() {
        let meta = PolicyMetadata {
            tags: vec!["   ".into()],
            ..valid_metadata()
        };
        assert!(matches!(
            meta.validate(),
            Err(MetadataError::MissingField { field: "tags", .. })
        ));

        let empty = PolicyMetadata {
            tags: vec![],
            ..valid_metadata()
        };
        assert!(matches!(
            empty.validate(),
            Err(MetadataError::MissingField { field: "tags", .. })
        ));
    }

    #[test]
    fn policy_metadata_invalid_severity_is_a_parse_error() {
        let yaml = r"
id: p1
title: t
severity: catastrophic
owner: o
rationale: r
scope: s
tags: [a]
";
        let parsed: Result<PolicyMetadata, _> = serde_yaml::from_str(yaml);
        assert!(
            parsed.is_err(),
            "an unrecognised severity band must fail closed, not default"
        );
    }

    #[test]
    fn policy_metadata_unknown_field_is_rejected() {
        let yaml = r"
id: p1
title: t
severity: low
owner: o
rationale: r
scope: s
tags: [a]
owners: typo-of-owner
";
        let parsed: Result<PolicyMetadata, _> = serde_yaml::from_str(yaml);
        assert!(parsed.is_err(), "a mistyped field key must be rejected");
    }

    #[test]
    fn policy_metadata_duplicate_ids_are_detected() {
        let a = valid_metadata();
        let b = valid_metadata();
        assert_eq!(
            ensure_unique_ids(&[a, b]),
            Err(MetadataError::DuplicateId("no-network-imports".into()))
        );
    }

    #[test]
    fn policy_metadata_unique_ids_pass() {
        let a = valid_metadata();
        let b = PolicyMetadata {
            id: "other".into(),
            ..valid_metadata()
        };
        assert_eq!(ensure_unique_ids(&[a, b]), Ok(()));
    }

    #[test]
    fn policy_metadata_ensure_unique_ids_reports_missing_id() {
        let a = PolicyMetadata {
            id: String::new(),
            ..valid_metadata()
        };
        assert_eq!(ensure_unique_ids(&[a]), Err(MetadataError::MissingId));
    }
}
