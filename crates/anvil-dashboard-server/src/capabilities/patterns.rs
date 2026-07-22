use std::path::Path;

use serde::Deserialize;

use crate::api::{DataState, PATTERN_CATALOGUE_SCHEMA, PatternCatalogue, PatternSummary};
use crate::{Workspace, WorkspaceReadError};

const REGISTRY_ARTEFACT: &str = "patterns/compiled/registry.json";

#[derive(Debug, Deserialize)]
struct CompiledRegistryFile {
    patterns: Vec<CompiledPattern>,
}

#[derive(Debug, Deserialize)]
struct CompiledPattern {
    id: String,
    #[serde(default)]
    family: String,
    title: String,
    #[serde(default)]
    severity: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
}

pub fn load_pattern_catalogue(workspace: &Workspace) -> PatternCatalogue {
    let bytes = match workspace.read(Path::new(REGISTRY_ARTEFACT)) {
        Ok(bytes) => bytes,
        Err(WorkspaceReadError::Missing { .. }) => {
            return PatternCatalogue::unavailable(
                "No compiled anti-pattern registry is available in this workspace.",
            );
        }
        Err(error) => {
            return PatternCatalogue::unavailable(format!(
                "The compiled anti-pattern registry could not be read: {error}"
            ));
        }
    };

    let parsed: CompiledRegistryFile = match serde_json::from_slice(&bytes) {
        Ok(parsed) => parsed,
        Err(_) => {
            return PatternCatalogue::unavailable(
                "The compiled anti-pattern registry has an unsupported shape.",
            );
        }
    };

    let patterns = parsed
        .patterns
        .into_iter()
        .map(|pattern| PatternSummary {
            id: pattern.id,
            title: pattern.title,
            family: if pattern.family.is_empty() {
                "unknown".to_owned()
            } else {
                pattern.family
            },
            severity: if pattern.severity.is_empty() {
                "warning".to_owned()
            } else {
                pattern.severity
            },
            enabled: pattern.enabled.unwrap_or(true),
            instance_count: 0,
            description: pattern.description.unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    if patterns.is_empty() {
        return PatternCatalogue::unavailable(
            "The compiled anti-pattern registry does not define any patterns.",
        );
    }

    PatternCatalogue {
        schema_version: PATTERN_CATALOGUE_SCHEMA.to_owned(),
        data_state: DataState::Complete,
        source_message: "Compiled anti-pattern registry loaded from the workspace.".to_owned(),
        patterns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn loads_compiled_registry_patterns() {
        let root = tempdir().expect("workspace");
        let dir = root.path().join("patterns/compiled");
        fs::create_dir_all(&dir).expect("dirs");
        fs::write(
            dir.join("registry.json"),
            br#"{
              "schema_version": 1,
              "patterns": [
                {
                  "id": "AP-001",
                  "family": "guardrail-suppression",
                  "title": "Broad eslint-disable added",
                  "severity": "warning",
                  "description": "Disables lint rules broadly."
                }
              ]
            }"#,
        )
        .expect("write");
        let workspace = Workspace::new(root.path()).expect("workspace");
        let catalogue = load_pattern_catalogue(&workspace);
        assert_eq!(catalogue.data_state, DataState::Complete);
        assert_eq!(catalogue.patterns.len(), 1);
        assert_eq!(catalogue.patterns[0].id, "AP-001");
        assert!(catalogue.patterns[0].enabled);
    }
}
