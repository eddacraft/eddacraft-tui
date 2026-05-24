//! `PolicyInput` v1 — the versioned data document every policy receives at
//! eval time (POLENG-002).
//!
//! This is a **stability contract**. Rego policies bind to the JSON shape
//! produced here (`input.repo_state.files`, `input.diff.new_edges`,
//! `input.baseline.findings`, …), so field names and nesting are part of the
//! public API. The schema-stability snapshot test in this module pins the
//! wire format; any intended change must update the snapshot deliberately and
//! follow the deprecation policy in `docs/specs/policy-input-v1.md`.
//!
//! The document is intentionally self-describing: it mirrors the shapes of
//! `anvil_kernel::graph::DependencyGraph` (files + edges) and
//! `anvil_baseline::BaselineFinding` (`rule_id` / `file_path` / `fingerprint`)
//! without taking a crate dependency on either, so the input contract can be
//! constructed, serialised, and snapshot-tested in isolation.

use serde::{Deserialize, Serialize};

/// Schema version of [`PolicyInput`]. Serialises to the string `"v1"` so
/// policies can branch defensively (`input.schema_version == "v1"`) and stay
/// forward-compatible across future revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SchemaVersion {
    /// Version 1 — the schema defined in this module.
    #[default]
    #[serde(rename = "v1")]
    V1,
}

/// A directed dependency edge in the repository import graph: `from` imports
/// `to`. Paths are repo-relative, matching `DependencyGraph`'s convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
}

/// Snapshot of repository structure feeding policy evaluation: the set of
/// known files and the dependency edges between them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoState {
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub edges: Vec<DependencyEdge>,
}

/// An APS plan file visible to policies. `id` and `status` are optional
/// because not every plan file carries machine-readable front matter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanFile {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// An architecture decision record entry drawn from the decision log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// The working-tree diff under evaluation: files that changed and dependency
/// edges introduced by the change set. `new_edges` is the substrate for
/// ADR-003 ("new edges only") — policies warn on these, not on the baseline.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diff {
    #[serde(default)]
    pub changed_files: Vec<String>,
    #[serde(default)]
    pub new_edges: Vec<DependencyEdge>,
}

/// A baselined finding fingerprint (ADR-003). Mirrors
/// `anvil_baseline::BaselineFinding` so policies can exclude pre-existing
/// findings from the new-edge set via `anvil.baseline_contains` (POLENG-003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineFinding {
    pub rule_id: String,
    pub file_path: String,
    pub fingerprint: String,
}

/// The baseline cohort: fingerprints of findings that existed before the
/// change set under evaluation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    #[serde(default)]
    pub findings: Vec<BaselineFinding>,
}

/// The versioned input data document every policy receives at eval time.
///
/// Serialises to a stable JSON object consumed by `regorus` as the policy
/// `input`. A defaulted `PolicyInput` is valid and produces a fully-populated
/// (but empty) object, so policies never have to guard against missing keys.
///
/// Every field is `#[serde(default)]` so a hand-written or partial input
/// document (e.g. `anvil policy eval --input`) can omit sections it does not
/// care about; serialisation still emits every field (the snapshot contract).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyInput {
    #[serde(default)]
    pub schema_version: SchemaVersion,
    #[serde(default)]
    pub repo_state: RepoState,
    #[serde(default)]
    pub plans: Vec<PlanFile>,
    #[serde(default)]
    pub decisions: Vec<DecisionEntry>,
    #[serde(default)]
    pub diff: Diff,
    #[serde(default)]
    pub baseline: Baseline,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, EngineConfig};

    /// A fully-populated fixture exercising every field of the v1 schema.
    /// Used by the round-trip and schema-stability snapshot tests.
    fn populated_fixture() -> PolicyInput {
        PolicyInput {
            schema_version: SchemaVersion::V1,
            repo_state: RepoState {
                files: vec!["src/app.rs".into(), "src/db.rs".into()],
                edges: vec![DependencyEdge {
                    from: "src/app.rs".into(),
                    to: "src/db.rs".into(),
                }],
            },
            plans: vec![PlanFile {
                path: "plans/modules/policy-engine.aps.md".into(),
                id: Some("POLENG".into()),
                status: Some("In Progress".into()),
            }],
            decisions: vec![DecisionEntry {
                id: "040".into(),
                title: Some("Adopt regorus as the Anvil Policy Engine".into()),
                status: Some("Accepted".into()),
            }],
            diff: Diff {
                changed_files: vec!["src/app.rs".into()],
                new_edges: vec![DependencyEdge {
                    from: "src/app.rs".into(),
                    to: "src/db.rs".into(),
                }],
            },
            baseline: Baseline {
                findings: vec![BaselineFinding {
                    rule_id: "anti-pattern:guardrail-suppression".into(),
                    file_path: "src/legacy.rs".into(),
                    fingerprint: "f00dcafe12345678".into(),
                }],
            },
        }
    }

    #[test]
    fn default_input_serialises_to_fully_populated_empty_object() {
        // A defaulted document must still carry every key so policies never
        // have to guard against `undefined` on a missing field.
        let json = serde_json::to_value(PolicyInput::default()).expect("serialise");
        let obj = json.as_object().expect("object");
        for key in [
            "schema_version",
            "repo_state",
            "plans",
            "decisions",
            "diff",
            "baseline",
        ] {
            assert!(obj.contains_key(key), "default input missing key `{key}`");
        }
        assert_eq!(obj["schema_version"], "v1");
    }

    #[test]
    fn input_round_trips_through_json() {
        let original = populated_fixture();
        let json = serde_json::to_string(&original).expect("serialise");
        let restored: PolicyInput = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(original, restored);
    }

    #[test]
    fn schema_version_serialises_as_v1_string() {
        let json = serde_json::to_string(&SchemaVersion::V1).expect("serialise");
        assert_eq!(json, "\"v1\"");
    }

    /// Schema-stability snapshot: pins the exact JSON wire format that
    /// policies bind to. Updating this snapshot is a deliberate contract
    /// change and must follow the deprecation policy in
    /// `docs/specs/policy-input-v1.md`.
    #[test]
    fn schema_stability_snapshot() {
        let json = serde_json::to_string_pretty(&populated_fixture()).expect("serialise");
        insta::assert_snapshot!(json);
    }

    /// The document must be readable by `regorus` as the policy `input` — the
    /// whole point of the contract. This proves the field names survive the
    /// round-trip into the engine and back out of a policy decision.
    #[test]
    fn policy_can_read_input_fields() {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        engine
            .add_policy(
                "edges.rego",
                r#"package edges
import rego.v1

# True when the change set introduces an edge into src/db.rs and the schema
# version is the one this policy was written against.
new_db_dependency if {
    input.schema_version == "v1"
    some edge in input.diff.new_edges
    edge.to == "src/db.rs"
}
"#,
            )
            .expect("add_policy");

        let result = engine
            .eval(&populated_fixture(), "data.edges.new_db_dependency")
            .expect("eval");
        assert_eq!(result.value, Some(serde_json::Value::Bool(true)));
    }
}
