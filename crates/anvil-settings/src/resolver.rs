//! Precedence and composite resolution with provenance (SETCON-004).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::catalogue::Catalogue;
use crate::types::{MergeSemantics, Scope, ValueType};

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

/// Deterministic, value-redacting resolution failures.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ResolverError {
    #[error("invalid value for {key}; expected {expected:?}")]
    InvalidValue {
        key: String,
        source_id: String,
        expected: ValueType,
    },
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

    pub fn resolve(
        catalogue: &Catalogue,
        declarations: &[Declaration],
    ) -> Result<Vec<ResolvedSetting>, ResolverError> {
        let mut keys = Vec::new();
        for decl in declarations {
            let canonical = catalogue
                .get(&decl.key)
                .map_or_else(|| decl.key.clone(), |e| e.key.0.clone());
            if !keys.iter().any(|k| k == &canonical) {
                keys.push(canonical);
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

#[allow(clippy::too_many_lines)]
fn resolve_one(
    catalogue: &Catalogue,
    declarations: &[Declaration],
    key: &str,
) -> Result<ResolvedSetting, ResolverError> {
    let entry = catalogue.get(key);
    let precedence = entry
        .map(|e| e.precedence.as_slice())
        .filter(|p| !p.is_empty())
        .unwrap_or(Resolver::DEFAULT_PRECEDENCE.as_slice());
    let merge = entry.map_or(MergeSemantics::Replace, |e| e.merge);
    let default = entry.and_then(|e| e.default.clone());

    let mut ranked: Vec<&Declaration> = declarations
        .iter()
        .filter(|d| {
            let canonical = catalogue
                .get(&d.key)
                .map_or(d.key.as_str(), |e| e.key.as_str());
            canonical == key
        })
        .filter(|d| entry.is_none_or(|e| e.supported_scopes.contains(&d.scope)))
        .collect();
    ranked.sort_by_key(|d| {
        precedence
            .iter()
            .position(|s| *s == d.scope)
            .unwrap_or(usize::MAX)
    });

    if let Some(entry) = entry {
        if let Some(value) = default.as_ref() {
            validate_value(key, "catalogue:default", &entry.value_type, value)?;
        }
        for declaration in &ranked {
            if let ResolutionEvent::Set(value) = &declaration.event {
                validate_value(key, &declaration.source_id, &entry.value_type, value)?;
            }
        }
    }

    let mut provenance: Vec<ProvenanceEvent> = Vec::new();
    let mut current: Option<Value> = default;
    for (idx, decl) in ranked.iter().enumerate() {
        let is_last = idx + 1 == ranked.len();
        match (&decl.event, merge) {
            (ResolutionEvent::Delete, _) => {
                for prior in &mut provenance {
                    prior.overridden = true;
                }
                provenance.push(ProvenanceEvent {
                    source_id: decl.source_id.clone(),
                    scope: decl.scope,
                    event: decl.event.clone(),
                    overridden: !is_last,
                });
                current = None;
            }
            (ResolutionEvent::Exclude(value), MergeSemantics::Union | MergeSemantics::Append) => {
                provenance.push(ProvenanceEvent {
                    source_id: decl.source_id.clone(),
                    scope: decl.scope,
                    event: decl.event.clone(),
                    overridden: false,
                });
                current = exclude_member(current, value);
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

    if let (Some(entry), Some(value)) = (entry, current.as_ref()) {
        validate_value(key, "resolver:merged", &entry.value_type, value)?;
    }

    Ok(ResolvedSetting {
        key: key.to_owned(),
        requested: current.clone(),
        resolved: current,
        provenance,
    })
}

fn validate_value(
    key: &str,
    source_id: &str,
    expected: &ValueType,
    value: &Value,
) -> Result<(), ResolverError> {
    let valid = match expected {
        ValueType::Boolean => value.is_boolean(),
        ValueType::String => value.is_string(),
        ValueType::Integer => value
            .as_number()
            .is_some_and(|number| number.is_i64() || number.is_u64()),
        ValueType::Enum { allowed } => value
            .as_str()
            .is_some_and(|candidate| allowed.iter().any(|item| item == candidate)),
        ValueType::List | ValueType::Set => value.is_array(),
        ValueType::Map => value.is_object(),
    };
    if valid {
        return Ok(());
    }
    Err(ResolverError::InvalidValue {
        key: key.to_owned(),
        source_id: source_id.to_owned(),
        expected: expected.clone(),
    })
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

fn exclude_member(base: Option<Value>, member: &Value) -> Option<Value> {
    match base {
        Some(Value::Array(items)) => Some(Value::Array(
            items.into_iter().filter(|item| item != member).collect(),
        )),
        other => other,
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
        )
        .unwrap();
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
        )
        .unwrap();
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

    #[test]
    fn resolver_delete_resets_accumulated_collection_state() {
        for merge in [MergeSemantics::Append, MergeSemantics::Union] {
            let mut list = list_entry();
            list.merge = merge;
            list.supported_scopes = vec![Scope::Org, Scope::Team, Scope::Project, Scope::User];
            list.precedence = list.supported_scopes.clone();
            let mut catalogue = Catalogue::new();
            catalogue.register(list).unwrap();
            let list_declarations = [
                Declaration {
                    key: "protection.checks".into(),
                    scope: Scope::Org,
                    source_id: "org".into(),
                    event: ResolutionEvent::Set(serde_json::json!(["before-delete"])),
                },
                Declaration {
                    key: "protection.checks".into(),
                    scope: Scope::Team,
                    source_id: "team".into(),
                    event: ResolutionEvent::Delete,
                },
                Declaration {
                    key: "protection.checks".into(),
                    scope: Scope::Project,
                    source_id: "project".into(),
                    event: ResolutionEvent::Exclude(serde_json::json!("irrelevant")),
                },
                Declaration {
                    key: "protection.checks".into(),
                    scope: Scope::User,
                    source_id: "user".into(),
                    event: ResolutionEvent::Set(serde_json::json!(["after-delete"])),
                },
            ];

            let list_row = Resolver::resolve(&catalogue, &list_declarations)
                .unwrap()
                .pop()
                .unwrap();
            assert_eq!(list_row.resolved, Some(serde_json::json!(["after-delete"])));
            assert!(
                list_row
                    .provenance
                    .iter()
                    .any(|event| matches!(event.event, ResolutionEvent::Delete))
            );
        }

        let mut map = list_entry();
        map.value_type = ValueType::Map;
        map.default = Some(serde_json::json!({}));
        map.merge = MergeSemantics::KeyedMerge;
        map.supported_scopes = vec![Scope::Org, Scope::Team, Scope::Project];
        map.precedence = map.supported_scopes.clone();
        let mut catalogue = Catalogue::new();
        catalogue.register(map).unwrap();
        let map_declarations = [
            Declaration {
                key: "protection.checks".into(),
                scope: Scope::Org,
                source_id: "org".into(),
                event: ResolutionEvent::Set(serde_json::json!({
                    "before-delete": true
                })),
            },
            Declaration {
                key: "protection.checks".into(),
                scope: Scope::Team,
                source_id: "team".into(),
                event: ResolutionEvent::Delete,
            },
            Declaration {
                key: "protection.checks".into(),
                scope: Scope::Project,
                source_id: "project".into(),
                event: ResolutionEvent::Set(serde_json::json!({
                    "after-delete": true
                })),
            },
        ];

        let map_row = Resolver::resolve(&catalogue, &map_declarations)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(
            map_row.resolved,
            Some(serde_json::json!({"after-delete": true}))
        );
        assert!(
            map_row
                .provenance
                .iter()
                .any(|event| matches!(event.event, ResolutionEvent::Delete))
        );
    }

    #[test]
    fn resolver_rejects_values_that_contradict_the_catalogue_type() {
        let cases = [
            (ValueType::Boolean, Value::String("true".into())),
            (ValueType::String, Value::Bool(true)),
            (ValueType::Integer, serde_json::json!(1.5)),
            (
                ValueType::Enum {
                    allowed: vec!["warn".into(), "enforce".into()],
                },
                Value::String("off".into()),
            ),
            (ValueType::List, serde_json::json!({"not": "a list"})),
            (ValueType::Map, serde_json::json!(["not", "a", "map"])),
            (ValueType::Set, Value::String("not-a-set".into())),
        ];

        for (value_type, value) in cases {
            let mut entry = list_entry();
            entry.value_type = value_type.clone();
            entry.default = None;
            entry.merge = MergeSemantics::Replace;
            let mut catalogue = Catalogue::new();
            catalogue.register(entry).unwrap();
            let declaration = Declaration {
                key: "protection.checks".into(),
                scope: Scope::Project,
                source_id: "project".into(),
                event: ResolutionEvent::Set(value),
            };

            assert_eq!(
                Resolver::resolve(&catalogue, &[declaration]).unwrap_err(),
                ResolverError::InvalidValue {
                    key: "protection.checks".into(),
                    source_id: "project".into(),
                    expected: value_type,
                }
            );
        }
    }

    #[test]
    fn resolver_accepts_values_that_match_the_catalogue_type() {
        let cases = [
            (ValueType::Boolean, Value::Bool(true)),
            (ValueType::String, Value::String("value".into())),
            (ValueType::Integer, serde_json::json!(42)),
            (
                ValueType::Enum {
                    allowed: vec!["warn".into(), "enforce".into()],
                },
                Value::String("warn".into()),
            ),
            (ValueType::List, serde_json::json!(["lint"])),
            (ValueType::Map, serde_json::json!({"lint": true})),
            (ValueType::Set, serde_json::json!(["lint"])),
        ];

        for (value_type, value) in cases {
            let mut entry = list_entry();
            entry.value_type = value_type;
            entry.default = None;
            entry.merge = MergeSemantics::Replace;
            let mut catalogue = Catalogue::new();
            catalogue.register(entry).unwrap();
            let declaration = Declaration {
                key: "protection.checks".into(),
                scope: Scope::Project,
                source_id: "project".into(),
                event: ResolutionEvent::Set(value),
            };

            Resolver::resolve(&catalogue, &[declaration]).expect("matching value type");
        }
    }

    #[test]
    fn resolver_error_display_redacts_path_shaped_source_ids() {
        let mut entry = list_entry();
        entry.value_type = ValueType::Boolean;
        entry.default = None;
        entry.merge = MergeSemantics::Replace;
        let mut catalogue = Catalogue::new();
        catalogue.register(entry).unwrap();
        let source_id = "/home/alice/private/.anvil.json";
        let error = Resolver::resolve(
            &catalogue,
            &[Declaration {
                key: "protection.checks".into(),
                scope: Scope::Project,
                source_id: source_id.into(),
                event: ResolutionEvent::Set(Value::String("not-a-boolean".into())),
            }],
        )
        .unwrap_err();

        assert!(!error.to_string().contains(source_id));
        assert!(error.to_string().contains("protection.checks"));
        assert!(matches!(
            error,
            ResolverError::InvalidValue { source_id: actual, .. } if actual == source_id
        ));
    }
}
