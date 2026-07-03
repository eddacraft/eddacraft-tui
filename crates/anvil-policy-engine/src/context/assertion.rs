//! Contextual policy assertion schema (CPOL-001).
//!
//! An *assertion* is an authored, declarative rule that must hold for a change
//! set given deterministic workflow context. It pairs a [scope](AssertionScope)
//! (which changed paths and workflow phases it applies to) with an ordered list
//! of [conditions](AssertionCondition) — a closed set of field matchers and
//! thresholds, deliberately *not* an expression language — and an
//! [outcome](crate::pack::PolicySeverity) declaring the severity band of the
//! finding a violation emits.
//!
//! This module owns pure data shapes and their validation only; it performs no
//! evaluation. Evaluation against a context payload lands in
//! [`crate::context::adapters`] (CPOL-002) and remediation output in
//! [`crate::context::guidance`] (CPOL-003).
//!
//! ## Wire contract
//!
//! Assertions are authored (YAML/JSON), so the schema fails *closed*: every
//! struct is `#[serde(deny_unknown_fields)]` and the condition set is a closed
//! enum, so a mistyped field or an unknown condition kind is rejected at
//! deserialisation rather than silently dropped — the same posture as
//! [`crate::pack::metadata`]. String fields default so a *missing* field is a
//! [`AssertionError`] naming the offending assertion and field (a friendlier
//! message than a bare parse error); `outcome` is required, so an assertion with
//! no declared severity fails closed at parse time.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::pack::PolicySeverity;

/// A phase of the Anvil workflow an assertion can be scoped to.
///
/// The wire form is kebab-case (`save`/`commit`/`push`/`manual`). Variants are
/// added, never renamed: the set is part of the authored-rule contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPhase {
    /// Save-time evaluation (the editor/agent write path).
    Save,
    /// Pre-commit evaluation.
    Commit,
    /// Pre-push evaluation.
    Push,
    /// Ad-hoc evaluation (`anvil policy eval`), outside a lifecycle hook.
    Manual,
}

/// The kind of change a path underwent in the change set.
///
/// Wire form is kebab-case (`added`/`modified`/`removed`/`renamed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    /// The path is new in the change set.
    Added,
    /// The path existed and its contents changed.
    Modified,
    /// The path was deleted.
    Removed,
    /// The path was moved/renamed.
    Renamed,
}

/// A numeric comparison operator for threshold conditions.
///
/// Wire form is kebab-case (`equal`/`less-than`/`at-most`/`greater-than`/
/// `at-least`). The comparison reads left-to-right: `actual <op> value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Comparison {
    /// `actual == value`.
    Equal,
    /// `actual < value`.
    LessThan,
    /// `actual <= value`.
    AtMost,
    /// `actual > value`.
    GreaterThan,
    /// `actual >= value`.
    AtLeast,
}

impl Comparison {
    /// Apply the comparison: `actual <op> value`.
    #[must_use]
    pub fn holds(self, actual: u64, value: u64) -> bool {
        match self {
            Self::Equal => actual == value,
            Self::LessThan => actual < value,
            Self::AtMost => actual <= value,
            Self::GreaterThan => actual > value,
            Self::AtLeast => actual >= value,
        }
    }
}

/// A path-glob condition payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathGlob {
    /// The glob matched against changed paths (`**`, `*`, `?` supported).
    pub glob: String,
}

/// A changed-path-count threshold payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangedPathCountSpec {
    /// How the count is compared against [`value`](Self::value).
    pub op: Comparison,
    /// The threshold the count is compared against.
    pub value: u64,
    /// When set, only changed paths of this kind are counted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_kind: Option<ChangeKind>,
}

/// A config key/value match payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigMatch {
    /// The config key that must be present.
    pub key: String,
    /// The value the config key must equal.
    pub value: String,
}

/// A config-key presence payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigKey {
    /// The config key that must be present and non-blank.
    pub key: String,
}

/// A single declarative requirement over the assertion context payload.
///
/// Deliberately a *closed* enum of field matchers and thresholds rather than an
/// expression language: an assertion asserts that every one of its conditions
/// holds, and a violation names the first that did not. The wire form is
/// externally tagged in kebab-case, e.g.
///
/// ```yaml
/// - config-equals: { key: signed-commits, value: "true" }
/// - changed-paths-exclude: { glob: "Cargo.lock" }
/// ```
///
/// An unknown condition kind is an unknown-variant error (fail closed), and each
/// payload denies unknown fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssertionCondition {
    /// Every changed path must match the glob (changes confined to an area).
    ChangedPathsConfinedTo(PathGlob),
    /// No changed path may match the glob (a protected area is untouched).
    ChangedPathsExclude(PathGlob),
    /// The changed-path count (optionally filtered by kind) must satisfy the
    /// comparison.
    ChangedPathCount(ChangedPathCountSpec),
    /// A config value must be present and equal to the expected string.
    ConfigEquals(ConfigMatch),
    /// A config value must be present and non-blank.
    ConfigPresent(ConfigKey),
}

impl AssertionCondition {
    /// Check the condition's own fields for blanks, returning the offending
    /// field name when malformed. Does not touch any context.
    fn field_check(&self) -> Result<(), &'static str> {
        match self {
            Self::ChangedPathsConfinedTo(spec) | Self::ChangedPathsExclude(spec) => {
                blank_guard("glob", &spec.glob)
            }
            Self::ChangedPathCount(_) => Ok(()),
            Self::ConfigEquals(spec) => blank_guard("key", &spec.key),
            Self::ConfigPresent(spec) => blank_guard("key", &spec.key),
        }
    }
}

fn blank_guard(field: &'static str, value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() {
        Err(field)
    } else {
        Ok(())
    }
}

/// Which changed paths and workflow phases an assertion applies to.
///
/// An empty list on either axis means "no restriction on this axis": a default
/// [`AssertionScope`] applies to every path in every phase. A path is in scope
/// when it matches at least one of [`paths`](Self::paths); a phase is in scope
/// when it is one of [`phases`](Self::phases).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssertionScope {
    /// Path globs the assertion applies to; empty means every path.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Workflow phases the assertion applies to; empty means every phase.
    #[serde(default)]
    pub phases: Vec<WorkflowPhase>,
}

/// A contextual policy assertion.
///
/// See the [module docs](self) for the wire contract. Construct from an authored
/// document via serde, then call [`validate`](Self::validate) before use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion {
    /// Unique assertion identifier.
    #[serde(default)]
    pub id: String,
    /// Human-readable one-line title.
    #[serde(default)]
    pub title: String,
    /// Which paths and phases the assertion applies to.
    #[serde(default)]
    pub scope: AssertionScope,
    /// Ordered requirements; all must hold. Evaluated in order, a violation
    /// names the first unmet condition.
    #[serde(default)]
    pub conditions: Vec<AssertionCondition>,
    /// Severity band of the finding a violation emits. Reuses the pack severity
    /// vocabulary; required (a missing band fails closed at parse time).
    pub outcome: PolicySeverity,
    /// Why the assertion exists — surfaced to authors on a violation.
    #[serde(default)]
    pub rationale: String,
    /// How to resolve a violation — surfaced remediation-first.
    #[serde(default)]
    pub remediation: String,
}

/// A validation failure on an [`Assertion`].
///
/// Every variant names the offending assertion (where known) and the fix.
/// User-facing text uses UK spelling.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssertionError {
    /// The assertion has no `id`, so nothing else can be attributed to it.
    #[error("an assertion is missing its `id`; give every assertion a unique, non-blank id")]
    MissingId,
    /// A required top-level field is absent or blank on an identified assertion.
    #[error(
        "assertion `{assertion_id}` is missing required field `{field}`; \
         set a non-blank `{field}` value"
    )]
    MissingField {
        /// The `id` of the offending assertion.
        assertion_id: String,
        /// The name of the missing field.
        field: &'static str,
    },
    /// The assertion declares no conditions, so it can never be evaluated.
    #[error(
        "assertion `{assertion_id}` declares no conditions; \
         add at least one condition it must satisfy"
    )]
    NoConditions {
        /// The `id` of the offending assertion.
        assertion_id: String,
    },
    /// A scope path glob is blank.
    #[error(
        "assertion `{assertion_id}` has a blank scope path at index {index}; remove it or set a glob"
    )]
    BlankScopePath {
        /// The `id` of the offending assertion.
        assertion_id: String,
        /// The index of the blank scope path.
        index: usize,
    },
    /// A condition has a blank field.
    #[error(
        "assertion `{assertion_id}` condition {index} has a blank `{field}`; \
         set a non-blank `{field}` value"
    )]
    BlankConditionField {
        /// The `id` of the offending assertion.
        assertion_id: String,
        /// The index of the offending condition.
        index: usize,
        /// The blank field name.
        field: &'static str,
    },
}

impl Assertion {
    /// Validate that every required field is present and non-blank, that at
    /// least one condition is declared, and that no scope path or condition
    /// field is blank.
    ///
    /// `id` is checked first so any subsequent error can cite it. Returns the
    /// first failure in a deterministic field order.
    pub fn validate(&self) -> Result<(), AssertionError> {
        let id = self.id.trim();
        if id.is_empty() {
            return Err(AssertionError::MissingId);
        }

        for (field, value) in [
            ("title", self.title.as_str()),
            ("rationale", self.rationale.as_str()),
            ("remediation", self.remediation.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(AssertionError::MissingField {
                    assertion_id: id.to_string(),
                    field,
                });
            }
        }

        for (index, path) in self.scope.paths.iter().enumerate() {
            if path.trim().is_empty() {
                return Err(AssertionError::BlankScopePath {
                    assertion_id: id.to_string(),
                    index,
                });
            }
        }

        if self.conditions.is_empty() {
            return Err(AssertionError::NoConditions {
                assertion_id: id.to_string(),
            });
        }

        for (index, condition) in self.conditions.iter().enumerate() {
            if let Err(field) = condition.field_check() {
                return Err(AssertionError::BlankConditionField {
                    assertion_id: id.to_string(),
                    index,
                    field,
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_assertion() -> Assertion {
        Assertion {
            id: "confine-changes-to-src".into(),
            title: "Confine changes to the source tree".into(),
            scope: AssertionScope {
                paths: vec!["crates/**".into()],
                phases: vec![WorkflowPhase::Commit, WorkflowPhase::Push],
            },
            conditions: vec![
                AssertionCondition::ChangedPathsConfinedTo(PathGlob {
                    glob: "crates/**".into(),
                }),
                AssertionCondition::ChangedPathsExclude(PathGlob {
                    glob: "**/Cargo.lock".into(),
                }),
                AssertionCondition::ChangedPathCount(ChangedPathCountSpec {
                    op: Comparison::AtMost,
                    value: 50,
                    change_kind: Some(ChangeKind::Modified),
                }),
                AssertionCondition::ConfigEquals(ConfigMatch {
                    key: "signed-commits".into(),
                    value: "true".into(),
                }),
                AssertionCondition::ConfigPresent(ConfigKey {
                    key: "owner".into(),
                }),
            ],
            outcome: PolicySeverity::High,
            rationale: "Scoped, small, signed changes keep the blast radius small.".into(),
            remediation: "Split unrelated changes out and sign your commits.".into(),
        }
    }

    #[test]
    fn assertion_schema_valid_assertion_passes_validation() {
        assert_eq!(valid_assertion().validate(), Ok(()));
    }

    #[test]
    fn assertion_schema_round_trips_through_json() {
        let original = valid_assertion();
        let json = serde_json::to_string(&original).expect("serialise");
        let restored: Assertion = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(original, restored);
    }

    #[test]
    fn assertion_schema_round_trips_through_yaml() {
        let original = valid_assertion();
        let yaml = serde_yaml::to_string(&original).expect("serialise");
        let restored: Assertion = serde_yaml::from_str(&yaml).expect("deserialise");
        assert_eq!(original, restored);
    }

    #[test]
    fn assertion_schema_outcome_reuses_pack_severity_vocabulary() {
        // The declared band is the pack severity vocabulary, on the wire in its
        // lowercase form.
        let yaml = serde_yaml::to_string(&valid_assertion()).expect("serialise");
        assert!(yaml.contains("outcome: high"), "got:\n{yaml}");
    }

    #[test]
    fn assertion_schema_condition_wire_form_is_kebab_case_externally_tagged() {
        let json = serde_json::to_string(&AssertionCondition::ConfigEquals(ConfigMatch {
            key: "k".into(),
            value: "v".into(),
        }))
        .expect("serialise");
        assert_eq!(json, r#"{"config-equals":{"key":"k","value":"v"}}"#);
    }

    #[test]
    fn assertion_schema_unknown_top_level_field_is_rejected() {
        let yaml = r"
id: a1
title: t
conditions:
  - config-present: { key: owner }
outcome: high
rationale: r
remediation: fix it
extra: nope
";
        let parsed: Result<Assertion, _> = serde_yaml::from_str(yaml);
        assert!(parsed.is_err(), "a mistyped top-level key must be rejected");
    }

    #[test]
    fn assertion_schema_unknown_condition_kind_is_rejected() {
        let yaml = r"
id: a1
title: t
conditions:
  - no-such-condition: { key: owner }
outcome: high
rationale: r
remediation: fix it
";
        let parsed: Result<Assertion, _> = serde_yaml::from_str(yaml);
        assert!(
            parsed.is_err(),
            "an unknown condition kind must fail closed"
        );
    }

    #[test]
    fn assertion_schema_unknown_condition_field_is_rejected() {
        let yaml = r"
id: a1
title: t
conditions:
  - config-equals: { key: k, value: v, typo: x }
outcome: high
rationale: r
remediation: fix it
";
        let parsed: Result<Assertion, _> = serde_yaml::from_str(yaml);
        assert!(
            parsed.is_err(),
            "a mistyped condition field must be rejected"
        );
    }

    #[test]
    fn assertion_schema_missing_outcome_fails_closed_at_parse() {
        let yaml = r"
id: a1
title: t
conditions:
  - config-present: { key: owner }
rationale: r
remediation: fix it
";
        let parsed: Result<Assertion, _> = serde_yaml::from_str(yaml);
        assert!(
            parsed.is_err(),
            "an assertion with no declared outcome must fail closed"
        );
    }

    #[test]
    fn assertion_schema_missing_id_is_reported() {
        let assertion = Assertion {
            id: "   ".into(),
            ..valid_assertion()
        };
        assert_eq!(assertion.validate(), Err(AssertionError::MissingId));
    }

    #[test]
    fn assertion_schema_missing_title_cites_assertion_and_field() {
        let assertion = Assertion {
            title: String::new(),
            ..valid_assertion()
        };
        assert_eq!(
            assertion.validate(),
            Err(AssertionError::MissingField {
                assertion_id: "confine-changes-to-src".into(),
                field: "title",
            })
        );
    }

    #[test]
    fn assertion_schema_missing_remediation_is_reported() {
        let assertion = Assertion {
            remediation: "  ".into(),
            ..valid_assertion()
        };
        assert!(matches!(
            assertion.validate(),
            Err(AssertionError::MissingField {
                field: "remediation",
                ..
            })
        ));
    }

    #[test]
    fn assertion_schema_missing_rationale_is_reported() {
        let assertion = Assertion {
            rationale: String::new(),
            ..valid_assertion()
        };
        assert!(matches!(
            assertion.validate(),
            Err(AssertionError::MissingField {
                field: "rationale",
                ..
            })
        ));
    }

    #[test]
    fn assertion_schema_no_conditions_is_reported() {
        let assertion = Assertion {
            conditions: vec![],
            ..valid_assertion()
        };
        assert_eq!(
            assertion.validate(),
            Err(AssertionError::NoConditions {
                assertion_id: "confine-changes-to-src".into(),
            })
        );
    }

    #[test]
    fn assertion_schema_blank_scope_path_is_reported() {
        let assertion = Assertion {
            scope: AssertionScope {
                paths: vec!["crates/**".into(), "  ".into()],
                phases: vec![],
            },
            ..valid_assertion()
        };
        assert_eq!(
            assertion.validate(),
            Err(AssertionError::BlankScopePath {
                assertion_id: "confine-changes-to-src".into(),
                index: 1,
            })
        );
    }

    #[test]
    fn assertion_schema_blank_condition_field_cites_index_and_field() {
        let assertion = Assertion {
            conditions: vec![
                AssertionCondition::ConfigPresent(ConfigKey { key: "ok".into() }),
                AssertionCondition::ConfigEquals(ConfigMatch {
                    key: "  ".into(),
                    value: "v".into(),
                }),
            ],
            ..valid_assertion()
        };
        assert_eq!(
            assertion.validate(),
            Err(AssertionError::BlankConditionField {
                assertion_id: "confine-changes-to-src".into(),
                index: 1,
                field: "key",
            })
        );
    }

    #[test]
    fn assertion_schema_comparison_semantics_hold() {
        assert!(Comparison::Equal.holds(3, 3));
        assert!(!Comparison::Equal.holds(3, 4));
        assert!(Comparison::LessThan.holds(2, 3));
        assert!(Comparison::AtMost.holds(3, 3));
        assert!(Comparison::GreaterThan.holds(4, 3));
        assert!(Comparison::AtLeast.holds(3, 3));
    }

    #[test]
    fn assertion_schema_change_kind_wire_form_is_kebab_case() {
        let json = serde_json::to_string(&ChangeKind::Renamed).expect("serialise");
        assert_eq!(json, r#""renamed""#);
    }
}
