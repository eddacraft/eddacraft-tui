//! First-release catalogue groups (SETCON-011).

use serde_json::json;

use crate::catalogue::{Catalogue, CatalogueEntry, CatalogueError, Mutability};
use crate::runtime_state::EvidenceTrust;
use crate::types::{
    ConsequenceClass, EvidenceMode, HealthRelevance, MergeSemantics, Scope, Sensitivity,
    SettingGroup, SettingKey, ValueType,
};

/// Populate Protection, Agents, Privacy, Integrations and Interface.
pub fn first_release_catalogue() -> Result<Catalogue, CatalogueError> {
    let mut cat = Catalogue::new();
    for entry in first_release_entries() {
        cat.register(entry)?;
    }
    for (key, pointer) in [
        ("project.schema_version", "/schema_version"),
        ("project.format", "/format"),
        ("project.planning.dir", "/planning_dir"),
        ("project.architecture.source", "/architecture/source"),
        ("protection.checks", "/checks"),
        ("protection.enforcement.mode", "/enforcement/mode"),
    ] {
        cat.register_project_config_target(key, pointer)?;
    }
    cat.validate()?;
    Ok(cat)
}

#[allow(clippy::too_many_lines)]
fn first_release_entries() -> Vec<CatalogueEntry> {
    vec![
        entry(
            "project.schema_version",
            "Project config schema version",
            SettingGroup::Project,
            10,
            ValueType::String,
            Some(json!("1.0.0")),
            MergeSemantics::Replace,
            ConsequenceClass::C,
            Sensitivity::Public,
            EvidenceMode::Value,
            HealthRelevance::Required,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "project.format",
            "Project config format",
            SettingGroup::Project,
            20,
            ValueType::Enum {
                allowed: vec!["yaml".into(), "json".into(), "toml".into()],
            },
            Some(json!("yaml")),
            MergeSemantics::Replace,
            ConsequenceClass::C,
            Sensitivity::Public,
            EvidenceMode::Value,
            HealthRelevance::Required,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "project.planning.dir",
            "Planning directory",
            SettingGroup::Project,
            30,
            ValueType::String,
            Some(json!("plans")),
            MergeSemantics::Replace,
            ConsequenceClass::B,
            Sensitivity::Public,
            EvidenceMode::Value,
            HealthRelevance::Advisory,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "project.architecture.source",
            "Architecture definition source",
            SettingGroup::Project,
            40,
            ValueType::String,
            None,
            MergeSemantics::Replace,
            ConsequenceClass::B,
            Sensitivity::Public,
            EvidenceMode::Value,
            HealthRelevance::Advisory,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "protection.checks",
            "Enabled checks",
            SettingGroup::Protection,
            10,
            ValueType::List,
            Some(json!([
                "secret-detection",
                "import-boundaries",
                "antipattern-scan"
            ])),
            MergeSemantics::Union,
            ConsequenceClass::C,
            Sensitivity::Public,
            EvidenceMode::Value,
            HealthRelevance::Required,
            Some("intercept"),
            EvidenceTrust::DaemonAttested,
        ),
        entry(
            "protection.enforcement.mode",
            "Enforcement mode",
            SettingGroup::Protection,
            20,
            ValueType::Enum {
                allowed: vec![
                    "off".into(),
                    "warn".into(),
                    "fence".into(),
                    "interrupt".into(),
                ],
            },
            Some(json!("warn")),
            MergeSemantics::Replace,
            ConsequenceClass::C,
            Sensitivity::Public,
            EvidenceMode::Value,
            HealthRelevance::Required,
            Some("intercept"),
            EvidenceTrust::DaemonAttested,
        ),
        entry(
            "protection.fail_closed",
            "Fail closed on gate errors",
            SettingGroup::Protection,
            30,
            ValueType::Boolean,
            Some(json!(false)),
            MergeSemantics::Replace,
            ConsequenceClass::C,
            Sensitivity::Public,
            EvidenceMode::Conformance,
            HealthRelevance::Required,
            Some("intercept"),
            EvidenceTrust::DaemonAttested,
        ),
        entry(
            "agents.approvals.required",
            "Approval requirements",
            SettingGroup::Agents,
            10,
            ValueType::Boolean,
            Some(json!(false)),
            MergeSemantics::Replace,
            ConsequenceClass::C,
            Sensitivity::Public,
            EvidenceMode::None,
            HealthRelevance::Advisory,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "agents.mcp.enabled",
            "MCP pre-write validation",
            SettingGroup::Agents,
            20,
            ValueType::Boolean,
            Some(json!(true)),
            MergeSemantics::Replace,
            ConsequenceClass::B,
            Sensitivity::Public,
            EvidenceMode::Conformance,
            HealthRelevance::Advisory,
            Some("intercept"),
            EvidenceTrust::DaemonAttested,
        ),
        entry(
            "privacy.telemetry",
            "Anonymous usage telemetry",
            SettingGroup::Privacy,
            10,
            ValueType::Enum {
                allowed: vec!["off".into(), "anonymous".into()],
            },
            Some(json!("anonymous")),
            MergeSemantics::Replace,
            ConsequenceClass::C,
            Sensitivity::Public,
            EvidenceMode::None,
            HealthRelevance::Advisory,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "privacy.gctx_egress",
            "Graph-context snippet egress",
            SettingGroup::Privacy,
            20,
            ValueType::Boolean,
            Some(json!(false)),
            MergeSemantics::Replace,
            ConsequenceClass::C,
            Sensitivity::Public,
            EvidenceMode::None,
            HealthRelevance::Advisory,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "privacy.observation_include_paths",
            "Record paths in usage observations",
            SettingGroup::Privacy,
            30,
            ValueType::Boolean,
            Some(json!(false)),
            MergeSemantics::Replace,
            ConsequenceClass::C,
            Sensitivity::Internal,
            EvidenceMode::None,
            HealthRelevance::None,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "privacy.license_token",
            "Licence token",
            SettingGroup::Privacy,
            40,
            ValueType::String,
            None,
            MergeSemantics::Replace,
            ConsequenceClass::D,
            Sensitivity::Secret,
            EvidenceMode::None,
            HealthRelevance::None,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "integrations.mcp.clients",
            "Registered MCP clients",
            SettingGroup::Integrations,
            10,
            ValueType::List,
            Some(json!([])),
            MergeSemantics::Union,
            ConsequenceClass::B,
            Sensitivity::Public,
            EvidenceMode::None,
            HealthRelevance::None,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "integrations.hooks.mode",
            "Git hook installation mode",
            SettingGroup::Integrations,
            20,
            ValueType::Enum {
                allowed: vec!["off".into(), "config".into(), "core".into()],
            },
            Some(json!("off")),
            MergeSemantics::Replace,
            ConsequenceClass::B,
            Sensitivity::Public,
            EvidenceMode::None,
            HealthRelevance::None,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "interface.compact",
            "Compact display",
            SettingGroup::Interface,
            10,
            ValueType::Boolean,
            Some(json!(false)),
            MergeSemantics::Replace,
            ConsequenceClass::A,
            Sensitivity::Public,
            EvidenceMode::None,
            HealthRelevance::None,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "interface.timestamps",
            "Show timestamps",
            SettingGroup::Interface,
            20,
            ValueType::Boolean,
            Some(json!(true)),
            MergeSemantics::Replace,
            ConsequenceClass::A,
            Sensitivity::Public,
            EvidenceMode::None,
            HealthRelevance::None,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "interface.motion",
            "Motion preference",
            SettingGroup::Interface,
            30,
            ValueType::Enum {
                allowed: vec!["full".into(), "reduced".into(), "off".into()],
            },
            Some(json!("full")),
            MergeSemantics::Replace,
            ConsequenceClass::A,
            Sensitivity::Public,
            EvidenceMode::None,
            HealthRelevance::None,
            None,
            EvidenceTrust::None,
        ),
        entry(
            "interface.hints",
            "Contextual hints",
            SettingGroup::Interface,
            40,
            ValueType::Boolean,
            Some(json!(true)),
            MergeSemantics::Replace,
            ConsequenceClass::A,
            Sensitivity::Public,
            EvidenceMode::None,
            HealthRelevance::None,
            None,
            EvidenceTrust::None,
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn entry(
    key: &str,
    label: &str,
    group: SettingGroup,
    order: u32,
    value_type: ValueType,
    default: Option<serde_json::Value>,
    merge: MergeSemantics,
    consequence_class: ConsequenceClass,
    sensitivity: Sensitivity,
    evidence_mode: EvidenceMode,
    health_relevance: HealthRelevance,
    activation_owner: Option<&str>,
    evidence_trust: EvidenceTrust,
) -> CatalogueEntry {
    CatalogueEntry {
        key: SettingKey(key.into()),
        label: label.into(),
        owner: "core".into(),
        group,
        order,
        value_type,
        default,
        supported_scopes: vec![
            Scope::Org,
            Scope::Team,
            Scope::Project,
            Scope::User,
            Scope::Environment,
            Scope::Session,
        ],
        precedence: ResolverPrecedence::all().to_vec(),
        merge,
        mutability: Mutability::SettingsService,
        canonical_writer: "settings-service".into(),
        consequence_class,
        sensitivity,
        evidence_mode,
        health_relevance,
        activation_owner: activation_owner.map(str::to_owned),
        evidence_trust,
        docs_ref: Some(format!("https://docs.eddacraft.ai/anvil/settings#{key}")),
        deprecated_aliases: vec![],
        version_compatibility: "anvil.settings.v1".into(),
    }
}

struct ResolverPrecedence;

impl ResolverPrecedence {
    fn all() -> [Scope; 6] {
        crate::resolver::Resolver::DEFAULT_PRECEDENCE
    }
}

#[cfg(test)]
mod catalogue_seed_tests {
    use super::*;

    #[test]
    fn catalogue_seed_covers_first_release_groups() {
        let cat = first_release_catalogue().expect("seed");
        assert!(cat.len() >= 5);
        for key in [
            "protection.checks",
            "agents.mcp.enabled",
            "privacy.telemetry",
            "integrations.mcp.clients",
            "interface.compact",
        ] {
            assert!(cat.get(key).is_some(), "missing {key}");
        }
        let checks = cat.get("protection.checks").unwrap();
        assert_eq!(checks.health_relevance, HealthRelevance::Required);
        assert_eq!(checks.activation_owner.as_deref(), Some("intercept"));
        let compact = cat.get("interface.compact").unwrap();
        assert_eq!(compact.evidence_mode, EvidenceMode::None);
        assert!(compact.activation_owner.is_none());
        let hints = cat.get("interface.hints").expect("hints");
        assert_eq!(hints.consequence_class, ConsequenceClass::A);
        assert_eq!(hints.value_type, ValueType::Boolean);
        assert_eq!(hints.default, Some(json!(true)));
        assert_eq!(hints.evidence_mode, EvidenceMode::None);
        assert!(hints.activation_owner.is_none());
        let secret = cat.get("privacy.license_token").unwrap();
        assert_eq!(secret.consequence_class, ConsequenceClass::D);
        assert_eq!(secret.sensitivity, Sensitivity::Secret);
    }

    #[test]
    fn catalogue_seed_class_a_keys_are_the_agreed_interface_set() {
        let cat = first_release_catalogue().expect("seed");
        let mut class_a: Vec<&str> = cat
            .iter()
            .filter(|entry| entry.consequence_class == ConsequenceClass::A)
            .map(|entry| entry.key.as_str())
            .collect();
        class_a.sort_unstable();
        assert_eq!(
            class_a,
            [
                "interface.compact",
                "interface.hints",
                "interface.motion",
                "interface.timestamps",
            ]
        );
    }

    #[test]
    fn catalogue_seed_enforcement_mode_uses_posture_ladder() {
        let cat = first_release_catalogue().expect("seed");
        let entry = cat.get("protection.enforcement.mode").expect("mode");
        match &entry.value_type {
            ValueType::Enum { allowed } => {
                assert_eq!(
                    allowed,
                    &vec![
                        "off".to_string(),
                        "warn".to_string(),
                        "fence".to_string(),
                        "interrupt".to_string(),
                    ]
                );
                assert!(!allowed.iter().any(|value| value == "enforce"));
            }
            other => panic!("expected enum, got {other:?}"),
        }
        assert_eq!(entry.default, Some(json!("warn")));
    }

    #[test]
    fn catalogue_seed_rejects_enforce_and_accepts_ladder_values() {
        use crate::resolver::{Declaration, ResolutionEvent, Resolver};
        use crate::types::Scope;
        use serde_json::Value;

        let cat = first_release_catalogue().expect("seed");
        let err = Resolver::resolve(
            &cat,
            &[Declaration {
                key: "protection.enforcement.mode".into(),
                scope: Scope::Project,
                source_id: "project".into(),
                event: ResolutionEvent::Set(Value::String("enforce".into())),
            }],
        )
        .expect_err("enforce is a rule-mode value, not a posture");
        let crate::resolver::ResolverError::InvalidValue { key, .. } = err;
        assert_eq!(key, "protection.enforcement.mode");

        for value in ["off", "warn", "fence", "interrupt"] {
            let rows = Resolver::resolve(
                &cat,
                &[Declaration {
                    key: "protection.enforcement.mode".into(),
                    scope: Scope::Project,
                    source_id: "project".into(),
                    event: ResolutionEvent::Set(Value::String(value.into())),
                }],
            )
            .unwrap_or_else(|err| panic!("{value} should be accepted: {err}"));
            let row = rows
                .iter()
                .find(|row| row.key == "protection.enforcement.mode")
                .expect("row");
            assert_eq!(row.resolved, Some(Value::String(value.into())));
        }
    }

    #[test]
    fn catalogue_seed_min_posture_uses_enforcement_mode_order() {
        use crate::constraints::{Constraint, PolicyBundle, apply_constraints};
        use crate::resolver::{ProvenanceEvent, ResolutionEvent, ResolvedSetting};
        use crate::types::{Posture, Scope};
        use serde_json::Value;

        let requested = vec![ResolvedSetting {
            key: "protection.enforcement.mode".into(),
            requested: Some(Value::String("warn".into())),
            resolved: Some(Value::String("warn".into())),
            provenance: vec![ProvenanceEvent {
                source_id: "project".into(),
                scope: Scope::Project,
                event: ResolutionEvent::Set(Value::String("warn".into())),
                overridden: false,
            }],
        }];
        let bundle = PolicyBundle {
            id: "org-1".into(),
            verifiable: true,
            expired: false,
            compatible: true,
            constraints: vec![Constraint::MinPosture {
                key: "protection.enforcement.mode".into(),
                min: Posture::Fence,
            }],
        };
        let out = apply_constraints(&requested, Some(&bundle)).unwrap();
        assert_eq!(out[0].resolved, Some(Value::String("fence".into())));

        let bundle = PolicyBundle {
            constraints: vec![Constraint::MinPosture {
                key: "protection.enforcement.mode".into(),
                min: Posture::Interrupt,
            }],
            ..bundle
        };
        let out = apply_constraints(&requested, Some(&bundle)).unwrap();
        assert_eq!(out[0].resolved, Some(Value::String("interrupt".into())));
    }

    #[test]
    fn project_bootstrap_targets_are_canonical_settings_metadata() {
        let cat = first_release_catalogue().expect("seed");
        for (key, pointer) in [
            ("project.schema_version", "/schema_version"),
            ("project.format", "/format"),
            ("project.planning.dir", "/planning_dir"),
            ("project.architecture.source", "/architecture/source"),
            ("protection.checks", "/checks"),
            ("protection.enforcement.mode", "/enforcement/mode"),
        ] {
            let target = cat
                .project_config_target(key)
                .unwrap_or_else(|| panic!("missing {key}"));
            assert_eq!(target.pointer, pointer);
            assert_eq!(cat.get(key).unwrap().canonical_writer, "settings-service");
        }
    }
}
