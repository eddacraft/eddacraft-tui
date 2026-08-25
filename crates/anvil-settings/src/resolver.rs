//! Precedence and composite resolution with provenance (SETCON-004).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::catalogue::Catalogue;
use crate::types::{MergeSemantics, Scope};

/// A configured declaration (or deletion / exclusion) at one scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Declaration {
    pub key: String,
    pub scope: Scope,
    pub source_id: String,
    pub event: ResolutionEvent,
}

/// First-class resolution events. Deletions and exclusions remain visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionEvent {
    Set(Value),
    Delete,
    Exclude(Value),
}

/// One step that contributed to (or was overridden in) the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceEvent {
    pub source_id: String,
    pub scope: Scope,
    pub event: ResolutionEvent,
    pub overridden: bool,
}

/// Resolved value plus complete provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSetting {
    pub key: String,
    pub requested: Option<Value>,
    pub resolved: Option<Value>,
    pub provenance: Vec<ProvenanceEvent>,
}

/// Resolves declarations across scopes. Policy constraints are applied later.
#[derive(Debug, Clone, Copy)]
pub struct Resolver;

impl Resolver {
    /// Default precedence (highest last): org < team < project < user <
    /// environment < session. A catalogue entry may override this list.
    pub const DEFAULT_PRECEDENCE: [Scope; 6] = [
        Scope::Org,
        Scope::Team,
        Scope::Project,
        Scope::User,
        Scope::Environment,
        Scope::Session,
    ];

    #[must_use]
    pub fn resolve(catalogue: &Catalogue, declarations: &[Declaration]) -> Vec<ResolvedSetting> {
        let mut keys = Vec::new();
        for decl in declarations {
            if !keys.iter().any(|k| k == &decl.key) {
                keys.push(decl.key.clone());
            }
        }
        for entry in catalogue.iter() {
            if !keys.iter().any(|k| k == entry.key.as_str()) {
                keys.push(entry.key.0.clone());
            }
        }
        keys.sort();
        keys.into_iter()
            .map(|key| resolve_one(catalogue, declarations, &key))
            .collect()
    }
}

fn resolve_one(catalogue: &Catalogue, declarations: &[Declaration], key: &str) -> ResolvedSetting {
    let entry = catalogue.get(key);
    let precedence = entry
        .map(|e| e.precedence.as_slice())
        .filter(|p| !p.is_empty())
        .unwrap_or(Resolver::DEFAULT_PRECEDENCE.as_slice());
    let merge = entry.map_or(MergeSemantics::Replace, |e| e.merge);
    let default = entry.and_then(|e| e.default.clone());

    let mut ranked: Vec<&Declaration> = declarations.iter().filter(|d| d.key == key).collect();
    ranked.sort_by_key(|d| {
        precedence
            .iter()
            .position(|s| *s == d.scope)
            .unwrap_or(usize::MAX)
    });

    let mut provenance = Vec::new();
    let mut current: Option<Value> = default;
    for (idx, decl) in ranked.iter().enumerate() {
        let is_last = idx + 1 == ranked.len();
        match (&decl.event, merge) {
            (ResolutionEvent::Delete, _) => {
                provenance.push(ProvenanceEvent {
                    source_id: decl.source_id.clone(),
                    scope: decl.scope,
                    event: decl.event.clone(),
                    overridden: !is_last,
                });
                if is_last {
                    current = None;
                }
            }
            (ResolutionEvent::Exclude(value), MergeSemantics::Union | MergeSemantics::Append) => {
                provenance.push(ProvenanceEvent {
                    source_id: decl.source_id.clone(),
                    scope: decl.scope,
                    event: decl.event.clone(),
                    overridden: false,
                });
                current = Some(exclude_member(current, value));
            }
            (ResolutionEvent::Exclude(value), _) => {
                provenance.push(ProvenanceEvent {
                    source_id: decl.source_id.clone(),
                    scope: decl.scope,
                    event: decl.event.clone(),
                    overridden: !is_last,
                });
                if is_last && current.as_ref() == Some(value) {
                    current = None;
                }
            }
            (ResolutionEvent::Set(value), MergeSemantics::Replace) => {
                if current.is_some()
                    && !is_last
                    && let Some(prev) = provenance.last_mut()
                {
                    prev.overridden = true;
                }
                provenance.push(ProvenanceEvent {
                    source_id: decl.source_id.clone(),
                    scope: decl.scope,
                    event: decl.event.clone(),
                    overridden: !is_last,
                });
                current = Some(value.clone());
            }
            (ResolutionEvent::Set(value), MergeSemantics::Append | MergeSemantics::Union) => {
                provenance.push(ProvenanceEvent {
                    source_id: decl.source_id.clone(),
                    scope: decl.scope,
                    event: decl.event.clone(),
                    overridden: false,
                });
                current = Some(merge_list(current, value, merge == MergeSemantics::Union));
            }
            (ResolutionEvent::Set(value), MergeSemantics::KeyedMerge) => {
                provenance.push(ProvenanceEvent {
                    source_id: decl.source_id.clone(),
                    scope: decl.scope,
                    event: decl.event.clone(),
                    overridden: false,
                });
                current = Some(merge_map(current, value));
            }
        }
    }

    ResolvedSetting {
        key: key.to_owned(),
        requested: current.clone(),
        resolved: current,
        provenance,
    }
}

fn merge_list(base: Option<Value>, incoming: &Value, unique: bool) -> Value {
    let mut out: Vec<Value> = match base {
        Some(Value::Array(items)) => items,
        Some(other) => vec![other],
        None => Vec::new(),
    };
    match incoming {
        Value::Array(items) => {
            for item in items {
                if !unique || !out.contains(item) {
                    out.push(item.clone());
                }
            }
        }
        other => {
            if !unique || !out.contains(other) {
                out.push(other.clone());
            }
        }
    }
    Value::Array(out)
}

fn merge_map(base: Option<Value>, incoming: &Value) -> Value {
    let mut out: Map<String, Value> = match base {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    if let Value::Object(map) = incoming {
        for (k, v) in map {
            out.insert(k.clone(), v.clone());
        }
    }
    Value::Object(out)
}

fn exclude_member(base: Option<Value>, member: &Value) -> Value {
    match base {
        Some(Value::Array(items)) => {
            Value::Array(items.into_iter().filter(|item| item != member).collect())
        }
        other => other.unwrap_or(Value::Null),
    }
}

#[cfg(test)]
mod resolver_tests {
    use super::*;
    use crate::catalogue::{Catalogue, CatalogueEntry, Mutability};
    use crate::runtime_state::EvidenceTrust;
    use crate::types::{
        ConsequenceClass, EvidenceMode, HealthRelevance, Sensitivity, SettingGroup, SettingKey,
        ValueType,
    };

    fn list_entry() -> CatalogueEntry {
        CatalogueEntry {
            key: SettingKey("protection.checks".into()),
            label: "checks".into(),
            owner: "core".into(),
            group: SettingGroup::Protection,
            order: 1,
            value_type: ValueType::List,
            default: Some(Value::Array(vec![])),
            supported_scopes: vec![Scope::Org, Scope::Project],
            precedence: vec![Scope::Org, Scope::Project],
            merge: MergeSemantics::Union,
            mutability: Mutability::SettingsService,
            canonical_writer: "settings".into(),
            consequence_class: ConsequenceClass::C,
            sensitivity: Sensitivity::Public,
            evidence_mode: EvidenceMode::Value,
            health_relevance: HealthRelevance::Required,
            activation_owner: Some("intercept".into()),
            evidence_trust: EvidenceTrust::DaemonAttested,
            docs_ref: None,
            deprecated_aliases: vec![],
            version_compatibility: "1".into(),
        }
    }

    #[test]
    fn resolver_union_keeps_member_level_provenance() {
        let mut cat = Catalogue::new();
        cat.register(list_entry()).unwrap();
        let resolved = Resolver::resolve(
            &cat,
            &[
                Declaration {
                    key: "protection.checks".into(),
                    scope: Scope::Org,
                    source_id: "org".into(),
                    event: ResolutionEvent::Set(Value::Array(vec![Value::String(
                        "secret-detection".into(),
                    )])),
                },
                Declaration {
                    key: "protection.checks".into(),
                    scope: Scope::Project,
                    source_id: "project".into(),
                    event: ResolutionEvent::Set(Value::Array(vec![Value::String(
                        "antipattern-scan".into(),
                    )])),
                },
            ],
        );
        let row = resolved
            .iter()
            .find(|r| r.key == "protection.checks")
            .unwrap();
        assert_eq!(
            row.resolved,
            Some(Value::Array(vec![
                Value::String("secret-detection".into()),
                Value::String("antipattern-scan".into()),
            ]))
        );
        assert_eq!(row.provenance.len(), 2);
        assert!(!row.provenance[0].overridden);
        assert!(!row.provenance[1].overridden);
    }

    #[test]
    fn resolver_delete_and_exclude_remain_visible() {
        let mut cat = Catalogue::new();
        cat.register(list_entry()).unwrap();
        let resolved = Resolver::resolve(
            &cat,
            &[
                Declaration {
                    key: "protection.checks".into(),
                    scope: Scope::Org,
                    source_id: "org".into(),
                    event: ResolutionEvent::Set(Value::Array(vec![
                        Value::String("secret-detection".into()),
                        Value::String("lint".into()),
                    ])),
                },
                Declaration {
                    key: "protection.checks".into(),
                    scope: Scope::Project,
                    source_id: "project".into(),
                    event: ResolutionEvent::Exclude(Value::String("lint".into())),
                },
            ],
        );
        let row = resolved
            .iter()
            .find(|r| r.key == "protection.checks")
            .unwrap();
        assert_eq!(
            row.resolved,
            Some(Value::Array(vec![Value::String("secret-detection".into())]))
        );
        assert!(
            row.provenance
                .iter()
                .any(|p| matches!(p.event, ResolutionEvent::Exclude(_)))
        );
    }
}
