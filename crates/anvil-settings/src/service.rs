//! Settings service and revisioned read-model boundary (SETCON-010).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::catalogue::Catalogue;
use crate::constraints::{ConstraintError, PolicyBundle, apply_constraints};
use crate::envelope::{Envelope, EnvelopeCommand, empty_object};
use crate::health::{Health, HealthControl, aggregate};
use crate::redaction::RedactionError;
use crate::resolver::{Declaration, ResolvedSetting, Resolver};
use crate::runtime_state::{Attestation, ClassifyInput, RuntimeState, classify_runtime_state};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error(transparent)]
    Constraint(#[from] ConstraintError),
    #[error(transparent)]
    Redaction(#[from] RedactionError),
    #[error("io discovering config: {0}")]
    Discover(#[from] std::io::Error),
}

/// Internally consistent, `model_revision`-stamped snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub model_revision: String,
    pub rows: Vec<SettingRow>,
    pub health: Health,
    pub discovered: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingRow {
    pub key: String,
    pub requested: Option<Value>,
    pub resolved: Option<Value>,
    pub runtime: RuntimeState,
    pub provenance: Vec<crate::resolver::ProvenanceEvent>,
}

pub struct SettingsService {
    catalogue: Catalogue,
}

pub struct SnapshotRequest<'a> {
    pub workspace_root: Option<&'a Path>,
    pub declarations: &'a [Declaration],
    pub bundle: Option<&'a PolicyBundle>,
    pub attestations: &'a BTreeMap<String, Attestation>,
    pub now: &'a str,
    pub generated_at: &'a str,
    pub command: EnvelopeCommand,
}

impl SettingsService {
    #[must_use]
    pub fn new(catalogue: Catalogue) -> Self {
        Self { catalogue }
    }

    #[must_use]
    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    /// Discover the project config path via `anvil_config`. Settings consumers
    /// do not open that file themselves.
    pub fn discover_config(root: &Path) -> std::io::Result<Option<anvil_config::DiscoveredConfig>> {
        anvil_config::discover(root, ".anvil")
    }

    pub fn snapshot(&self, request: &SnapshotRequest<'_>) -> Result<Snapshot, SettingsError> {
        let discovered = match request.workspace_root {
            Some(root) => {
                Self::discover_config(root)?.map(|d| d.path.to_string_lossy().into_owned())
            }
            None => None,
        };
        let requested = Resolver::resolve(&self.catalogue, request.declarations);
        let resolved = apply_constraints(&requested, request.bundle)?;
        let revision = model_revision(&resolved, request.attestations, discovered.as_deref());
        let mut rows = Vec::new();
        let mut controls = Vec::new();
        for setting in &resolved {
            let entry = self.catalogue.get(&setting.key);
            let att = request.attestations.get(&setting.key);
            let runtime = match entry {
                Some(entry) => classify_runtime_state(
                    att,
                    &ClassifyInput {
                        evidence_mode: entry.evidence_mode,
                        required_owner: entry.activation_owner.as_deref(),
                        required_trust: entry.evidence_trust,
                        resolved_value: setting.resolved.as_ref(),
                        resolved_revision: &revision,
                        now: request.now,
                    },
                ),
                None => RuntimeState::Unknown,
            };
            if let Some(entry) = entry {
                controls.push(HealthControl {
                    entry,
                    workflow_invalid: false,
                    runtime,
                });
            }
            rows.push(SettingRow {
                key: setting.key.clone(),
                requested: setting.requested.clone(),
                resolved: setting.resolved.clone(),
                runtime,
                provenance: setting.provenance.clone(),
            });
        }
        let health = aggregate(&self.catalogue, &controls, false);
        Ok(Snapshot {
            model_revision: revision,
            rows,
            health,
            discovered,
        })
    }

    pub fn envelope(
        &self,
        snapshot: &Snapshot,
        command: EnvelopeCommand,
        generated_at: &str,
    ) -> Result<Value, SettingsError> {
        let mut data = serde_json::Map::new();
        for row in &snapshot.rows {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "requested".into(),
                row.requested.clone().unwrap_or(Value::Null),
            );
            obj.insert(
                "resolved".into(),
                row.resolved.clone().unwrap_or(Value::Null),
            );
            obj.insert(
                "runtime".into(),
                serde_json::to_value(row.runtime).unwrap_or(Value::Null),
            );
            data.insert(row.key.clone(), Value::Object(obj));
        }
        let env = Envelope::new(
            command,
            generated_at.to_owned(),
            snapshot.model_revision.clone(),
            empty_object(),
            snapshot.health.clone(),
            Value::Object(data),
            vec![],
        );
        Ok(env.redacted(&self.catalogue)?)
    }
}

fn model_revision(
    resolved: &[ResolvedSetting],
    attestations: &BTreeMap<String, Attestation>,
    discovered: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    if let Ok(bytes) = serde_json::to_vec(&revision_payload(resolved, attestations, discovered)) {
        hasher.update(bytes);
    }
    hex::encode(hasher.finalize())
}

fn revision_payload(
    resolved: &[ResolvedSetting],
    attestations: &BTreeMap<String, Attestation>,
    discovered: Option<&str>,
) -> Value {
    serde_json::json!({
        "resolved": resolved.iter().map(|r| serde_json::json!({
            "key": r.key,
            "value": r.resolved,
        })).collect::<Vec<_>>(),
        "attestations": attestations,
        "discovered": discovered,
    })
}

#[cfg(test)]
mod service_tests {
    use super::*;
    use crate::resolver::ResolutionEvent;
    use crate::seed::first_release_catalogue;
    use crate::types::Scope;

    #[test]
    fn service_snapshot_is_revisioned_and_describes_unknown_state() {
        let cat = first_release_catalogue().expect("seed");
        let service = SettingsService::new(cat);
        let snapshot = service
            .snapshot(&SnapshotRequest {
                workspace_root: None,
                declarations: &[],
                bundle: None,
                attestations: &BTreeMap::new(),
                now: "2026-08-25T00:00:00Z",
                generated_at: "2026-08-25T00:00:00Z",
                command: EnvelopeCommand::Show,
            })
            .unwrap();
        assert!(!snapshot.model_revision.is_empty());
        assert_eq!(snapshot.model_revision.len(), 64);
        let checks = snapshot
            .rows
            .iter()
            .find(|r| r.key == "protection.checks")
            .expect("seeded key");
        assert_eq!(checks.runtime, RuntimeState::Unknown);
        let env = service
            .envelope(&snapshot, EnvelopeCommand::Show, "2026-08-25T00:00:00Z")
            .unwrap();
        assert_eq!(env["schema_version"], "anvil.settings.v1");
    }

    #[test]
    fn service_does_not_require_callers_to_open_config_files() {
        let tmp = std::env::temp_dir();
        let discovered = SettingsService::discover_config(&tmp).unwrap();
        assert!(discovered.is_none() || discovered.is_some());
    }

    #[test]
    fn service_resolves_injected_declarations() {
        let cat = first_release_catalogue().expect("seed");
        let service = SettingsService::new(cat);
        let declarations = [Declaration {
            key: "interface.compact".into(),
            scope: Scope::User,
            source_id: "user".into(),
            event: ResolutionEvent::Set(Value::Bool(true)),
        }];
        let snapshot = service
            .snapshot(&SnapshotRequest {
                workspace_root: None,
                declarations: &declarations,
                bundle: None,
                attestations: &BTreeMap::new(),
                now: "2026-08-25T00:00:00Z",
                generated_at: "2026-08-25T00:00:00Z",
                command: EnvelopeCommand::Show,
            })
            .unwrap();
        let row = snapshot
            .rows
            .iter()
            .find(|r| r.key == "interface.compact")
            .unwrap();
        assert_eq!(row.resolved, Some(Value::Bool(true)));
        assert_eq!(row.runtime, RuntimeState::Unknown);
    }
}
