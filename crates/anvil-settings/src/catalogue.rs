//! Typed settings catalogue (SETCON-002).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::runtime_state::EvidenceTrust;
use crate::types::{
    ConsequenceClass, EvidenceMode, HealthRelevance, MergeSemantics, Scope, Sensitivity,
    SettingGroup, SettingKey, ValueType,
};

/// One inspectable setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueEntry {
    pub key: SettingKey,
    pub label: String,
    pub owner: String,
    pub group: SettingGroup,
    pub order: u32,
    pub value_type: ValueType,
    pub default: Option<Value>,
    pub supported_scopes: Vec<Scope>,
    pub precedence: Vec<Scope>,
    pub merge: MergeSemantics,
    pub mutability: Mutability,
    pub canonical_writer: String,
    pub consequence_class: ConsequenceClass,
    pub sensitivity: Sensitivity,
    pub evidence_mode: EvidenceMode,
    pub health_relevance: HealthRelevance,
    pub activation_owner: Option<String>,
    pub evidence_trust: EvidenceTrust,
    pub docs_ref: Option<String>,
    pub deprecated_aliases: Vec<String>,
    pub version_compatibility: String,
}

/// Who may write the setting, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mutability {
    ReadOnly,
    SettingsService,
}

/// Collision-failing typed catalogue.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalogue {
    entries: BTreeMap<String, CatalogueEntry>,
    aliases: BTreeMap<String, String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CatalogueError {
    #[error("catalogue key collision: {0}")]
    KeyCollision(String),
    #[error("catalogue alias collision: {0}")]
    AliasCollision(String),
    #[error("empty catalogue key")]
    EmptyKey,
    #[error("adapter/extension key {0} is not namespaced")]
    UnnamespacedExtension(String),
    #[error("evidence-mode none cannot declare activation owner {owner} on {key}")]
    EvidenceNoneWithOwner { key: String, owner: String },
}

impl Catalogue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an entry. A key or alias collision fails rather than picking a
    /// winner.
    pub fn register(&mut self, entry: CatalogueEntry) -> Result<(), CatalogueError> {
        let key = entry.key.0.trim();
        if key.is_empty() {
            return Err(CatalogueError::EmptyKey);
        }
        validate_namespace(key, &entry.owner)?;
        if entry.evidence_mode == EvidenceMode::None
            && let Some(owner) = entry.activation_owner.as_deref()
        {
            return Err(CatalogueError::EvidenceNoneWithOwner {
                key: key.to_owned(),
                owner: owner.to_owned(),
            });
        }
        if self.entries.contains_key(key) || self.aliases.contains_key(key) {
            return Err(CatalogueError::KeyCollision(key.to_owned()));
        }
        for alias in &entry.deprecated_aliases {
            if self.entries.contains_key(alias) || self.aliases.contains_key(alias) {
                return Err(CatalogueError::AliasCollision(alias.clone()));
            }
        }
        for alias in &entry.deprecated_aliases {
            self.aliases.insert(alias.clone(), key.to_owned());
        }
        self.entries.insert(key.to_owned(), entry);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&CatalogueEntry> {
        let canonical = self.aliases.get(key).map_or(key, String::as_str);
        self.entries.get(canonical)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &CatalogueEntry> {
        self.entries.values()
    }

    /// Fail closed if any entry violates catalogue invariants.
    pub fn validate(&self) -> Result<(), CatalogueError> {
        for entry in self.entries.values() {
            validate_namespace(entry.key.as_str(), &entry.owner)?;
            if entry.evidence_mode == EvidenceMode::None
                && let Some(owner) = entry.activation_owner.as_deref()
            {
                return Err(CatalogueError::EvidenceNoneWithOwner {
                    key: entry.key.0.clone(),
                    owner: owner.to_owned(),
                });
            }
        }
        Ok(())
    }
}

fn validate_namespace(key: &str, owner: &str) -> Result<(), CatalogueError> {
    let owner = owner.trim();
    if (owner.starts_with("adapter.") || owner.starts_with("ext.") || owner == "extension")
        && !(key.starts_with("adapter.") || key.starts_with("ext."))
    {
        return Err(CatalogueError::UnnamespacedExtension(key.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod catalogue_tests {
    use super::*;
    use crate::runtime_state::EvidenceTrust;

    fn entry(key: &str, owner: &str) -> CatalogueEntry {
        CatalogueEntry {
            key: SettingKey(key.to_owned()),
            label: key.to_owned(),
            owner: owner.to_owned(),
            group: SettingGroup::Protection,
            order: 0,
            value_type: ValueType::Boolean,
            default: None,
            supported_scopes: vec![Scope::Project],
            precedence: vec![Scope::Project],
            merge: MergeSemantics::Replace,
            mutability: Mutability::SettingsService,
            canonical_writer: "settings".to_owned(),
            consequence_class: ConsequenceClass::B,
            sensitivity: Sensitivity::Public,
            evidence_mode: EvidenceMode::None,
            health_relevance: HealthRelevance::None,
            activation_owner: None,
            evidence_trust: EvidenceTrust::None,
            docs_ref: None,
            deprecated_aliases: vec![],
            version_compatibility: "1".to_owned(),
        }
    }

    #[test]
    fn catalogue_collision_fails_rather_than_picking_a_winner() {
        let mut cat = Catalogue::new();
        cat.register(entry("protection.checks", "core")).unwrap();
        let err = cat
            .register(entry("protection.checks", "core"))
            .expect_err("collision");
        assert_eq!(
            err,
            CatalogueError::KeyCollision("protection.checks".into())
        );
    }

    #[test]
    fn catalogue_alias_collision_fails() {
        let mut cat = Catalogue::new();
        let mut first = entry("protection.checks", "core");
        first.deprecated_aliases = vec!["checks".into()];
        cat.register(first).unwrap();
        let mut second = entry("protection.enabled_checks", "core");
        second.deprecated_aliases = vec!["checks".into()];
        let err = cat.register(second).expect_err("alias collision");
        assert_eq!(err, CatalogueError::AliasCollision("checks".into()));
    }

    #[test]
    fn catalogue_extension_keys_must_be_namespaced() {
        let mut cat = Catalogue::new();
        let err = cat
            .register(entry("bare", "ext.packs"))
            .expect_err("unnamespaced");
        assert_eq!(err, CatalogueError::UnnamespacedExtension("bare".into()));
    }
}
