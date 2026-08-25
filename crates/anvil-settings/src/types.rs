//! Canonical settings vocabulary (ADR-132).
//!
//! The word `effective` is banned: it collapses resolved and active.

use serde::{Deserialize, Serialize};

/// Stable namespaced catalogue key (`protection.checks`, `ext.foo.bar`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SettingKey(pub String);

impl SettingKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SettingKey {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// First-release information-architecture group (spec §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingGroup {
    Protection,
    Agents,
    Privacy,
    Integrations,
    Interface,
}

/// Value type recorded on a catalogue entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Boolean,
    String,
    Integer,
    Enum { allowed: Vec<String> },
    List,
    Map,
    Set,
}

/// Configuration source scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Org,
    Team,
    Project,
    User,
    Environment,
    Session,
}

/// How structured values combine across scopes. There is no implicit global
/// collection rule — each entry declares its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeSemantics {
    Replace,
    Append,
    Union,
    KeyedMerge,
}

/// Spec §13 consequence class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsequenceClass {
    A,
    B,
    C,
    D,
}

/// Sensitivity classification. Values lacking a trusted classification are
/// treated as unclassified and hidden by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Public,
    Internal,
    Secret,
    Unclassified,
}

/// How (if at all) the key can be proven active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMode {
    None,
    Value,
    ClassifiedDigest,
    Conformance,
}

/// Whether the key participates in overall health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthRelevance {
    Required,
    Advisory,
    None,
}

/// Configuration / workflow state — **not** a runtime-state value (ADR-132).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowState {
    Invalid,
    Locked,
    PendingActivation,
}

/// Ordered enforcement posture used by min/max constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Posture {
    Off,
    Warn,
    Enforce,
}

impl Posture {
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "off" => Some(Self::Off),
            "warn" => Some(Self::Warn),
            "enforce" => Some(Self::Enforce),
            _ => None,
        }
    }
}
