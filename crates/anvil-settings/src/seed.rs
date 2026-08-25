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
    cat.validate()?;
    Ok(cat)
}

#[allow(clippy::too_many_lines)]
fn first_release_entries() -> Vec<CatalogueEntry> {
    vec![
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
                allowed: vec!["off".into(), "warn".into(), "enforce".into()],
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
        let secret = cat.get("privacy.license_token").unwrap();
        assert_eq!(secret.consequence_class, ConsequenceClass::D);
        assert_eq!(secret.sensitivity, Sensitivity::Secret);
    }
}
