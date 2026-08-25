//! Health aggregation over required controls only (SETCON-007).

use serde::{Deserialize, Serialize};

use crate::catalogue::{Catalogue, CatalogueEntry};
use crate::runtime_state::RuntimeState;
use crate::types::HealthRelevance;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    pub status: HealthStatus,
    pub reasons: Vec<String>,
}

/// One control considered for health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthControl<'a> {
    pub entry: &'a CatalogueEntry,
    pub workflow_invalid: bool,
    pub runtime: RuntimeState,
}

/// Derive overall health. Advisory findings stay visible without failing
/// health. An unavailable optional integration does not fail health merely
/// by being inspectable.
#[must_use]
pub fn aggregate(
    catalogue: &Catalogue,
    controls: &[HealthControl<'_>],
    mandatory_bundle_invalid: bool,
) -> Health {
    let _ = catalogue;
    let mut reasons = Vec::new();
    let mut unhealthy = false;
    let mut indeterminate = false;

    if mandatory_bundle_invalid {
        unhealthy = true;
        reasons.push("mandatory policy bundle is invalid".into());
    }

    for control in controls {
        let required = control.entry.health_relevance == HealthRelevance::Required;
        if !required {
            continue;
        }
        if control.workflow_invalid
            || control.runtime == RuntimeState::Failed
            || control.runtime == RuntimeState::Drift
        {
            unhealthy = true;
            let kind = if control.workflow_invalid {
                "invalid"
            } else if control.runtime == RuntimeState::Failed {
                "failed"
            } else {
                "drift"
            };
            reasons.push(format!("{} is {kind}", control.entry.key.as_str()));
        } else if control.runtime == RuntimeState::Unknown || control.runtime == RuntimeState::Stale
        {
            indeterminate = true;
            reasons.push(format!(
                "{} is {:?}",
                control.entry.key.as_str(),
                control.runtime
            ));
        }
    }

    let status = if unhealthy {
        HealthStatus::Unhealthy
    } else if indeterminate {
        HealthStatus::Indeterminate
    } else {
        HealthStatus::Healthy
    };
    Health { status, reasons }
}

#[cfg(test)]
mod health_tests {
    use super::*;
    use crate::catalogue::{Catalogue, CatalogueEntry, Mutability};
    use crate::runtime_state::EvidenceTrust;
    use crate::types::{
        ConsequenceClass, EvidenceMode, MergeSemantics, Scope, Sensitivity, SettingGroup,
        SettingKey, ValueType,
    };

    fn entry(key: &str, relevance: HealthRelevance) -> CatalogueEntry {
        CatalogueEntry {
            key: SettingKey(key.into()),
            label: key.into(),
            owner: "core".into(),
            group: SettingGroup::Protection,
            order: 1,
            value_type: ValueType::Boolean,
            default: None,
            supported_scopes: vec![Scope::Project],
            precedence: vec![Scope::Project],
            merge: MergeSemantics::Replace,
            mutability: Mutability::SettingsService,
            canonical_writer: "settings".into(),
            consequence_class: ConsequenceClass::C,
            sensitivity: Sensitivity::Public,
            evidence_mode: EvidenceMode::Value,
            health_relevance: relevance,
            activation_owner: Some("intercept".into()),
            evidence_trust: EvidenceTrust::DaemonAttested,
            docs_ref: None,
            deprecated_aliases: vec![],
            version_compatibility: "1".into(),
        }
    }

    #[test]
    fn health_advisory_does_not_fail_and_optional_unknown_is_ignored() {
        let mut cat = Catalogue::new();
        let required = entry("protection.checks", HealthRelevance::Required);
        let advisory = entry("interface.compact", HealthRelevance::Advisory);
        cat.register(required.clone()).unwrap();
        cat.register(advisory.clone()).unwrap();
        let health = aggregate(
            &cat,
            &[
                HealthControl {
                    entry: cat.get("protection.checks").unwrap(),
                    workflow_invalid: false,
                    runtime: RuntimeState::Active,
                },
                HealthControl {
                    entry: cat.get("interface.compact").unwrap(),
                    workflow_invalid: false,
                    runtime: RuntimeState::Unknown,
                },
            ],
            false,
        );
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[test]
    fn health_required_unknown_is_indeterminate() {
        let mut cat = Catalogue::new();
        cat.register(entry("protection.checks", HealthRelevance::Required))
            .unwrap();
        let health = aggregate(
            &cat,
            &[HealthControl {
                entry: cat.get("protection.checks").unwrap(),
                workflow_invalid: false,
                runtime: RuntimeState::Unknown,
            }],
            false,
        );
        assert_eq!(health.status, HealthStatus::Indeterminate);
    }

    #[test]
    fn health_required_drift_is_unhealthy() {
        let mut cat = Catalogue::new();
        cat.register(entry("protection.checks", HealthRelevance::Required))
            .unwrap();
        let health = aggregate(
            &cat,
            &[HealthControl {
                entry: cat.get("protection.checks").unwrap(),
                workflow_invalid: false,
                runtime: RuntimeState::Drift,
            }],
            false,
        );
        assert_eq!(health.status, HealthStatus::Unhealthy);
    }
}
