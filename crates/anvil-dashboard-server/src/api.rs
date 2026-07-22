use anvil_kernel_types::protection_claim::ProtectionClaim;
use serde::{Deserialize, Serialize};

pub const PROTECTION_OVERVIEW_SCHEMA: &str = "anvil.dashboard.protection.v1";
pub const PLAN_DRIVER_SCHEMA: &str = "anvil.dashboard.plans.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub access: String,
}

impl HealthResponse {
    pub fn ready() -> Self {
        Self {
            status: "ok".to_owned(),
            access: "read-only".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DataState {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateCheckSummary {
    pub name: String,
    pub status: String,
    pub score: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateRunSummary {
    pub id: String,
    pub result: String,
    pub label: String,
    pub score: Option<f64>,
    pub warning_count: usize,
    pub duration_seconds: Option<f64>,
    pub started_at: Option<String>,
    pub new_warning_count: Option<usize>,
    pub changed_file_count: Option<usize>,
    /// Check tree for this run when the gate artefact includes rows.
    #[serde(default)]
    pub checks: Vec<GateCheckSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceLine {
    pub number: usize,
    pub text: String,
    pub highlighted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarningSummary {
    pub id: String,
    pub severity: String,
    pub category: String,
    pub message: String,
    pub file_path: Option<String>,
    pub age_label: String,
    pub evidence_id: String,
    pub rule: String,
    pub line: Option<usize>,
    pub explanation: String,
    pub matched_pattern: String,
    pub evidence_excerpt: Vec<EvidenceLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttentionItem {
    pub title: String,
    pub detail: String,
    pub evidence_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedFile {
    pub path: String,
    pub highest_severity: String,
    pub warning_count: usize,
    pub first_seen: String,
    pub last_seen: String,
    pub warning_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssuranceSummary {
    pub state: String,
    pub reason: Option<String>,
    pub generation: u64,
    pub last_full_scan: Option<String>,
    pub scanned_files: Option<u64>,
    pub total_files: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveTimeSummary {
    pub state: String,
    pub active: bool,
    pub failure_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataGap {
    pub component: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectionOverview {
    pub schema_version: String,
    pub data_state: DataState,
    pub source_message: String,
    pub claim: Option<ProtectionClaim>,
    pub assurance: Option<AssuranceSummary>,
    pub save_time: Option<SaveTimeSummary>,
    pub observed_at_unix: Option<u64>,
    pub latest_run: Option<GateRunSummary>,
    pub recent_runs: Vec<GateRunSummary>,
    pub next_attention: Option<AttentionItem>,
    pub warnings_state: DataState,
    pub warnings: Vec<WarningSummary>,
    pub affected_files_state: DataState,
    pub affected_files: Vec<AffectedFile>,
    pub gaps: Vec<DataGap>,
}

impl ProtectionOverview {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            schema_version: PROTECTION_OVERVIEW_SCHEMA.to_owned(),
            data_state: DataState::Unavailable,
            source_message: message.into(),
            claim: None,
            assurance: None,
            save_time: None,
            observed_at_unix: None,
            latest_run: None,
            recent_runs: Vec::new(),
            next_attention: None,
            warnings_state: DataState::Unavailable,
            warnings: Vec::new(),
            affected_files_state: DataState::Unavailable,
            affected_files: Vec::new(),
            gaps: Vec::new(),
        }
    }
}


pub const PATTERN_CATALOGUE_SCHEMA: &str = "anvil.dashboard.patterns.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternSummary {
    pub id: String,
    pub title: String,
    pub family: String,
    pub severity: String,
    pub enabled: bool,
    pub instance_count: usize,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternCatalogue {
    pub schema_version: String,
    pub data_state: DataState,
    pub source_message: String,
    pub patterns: Vec<PatternSummary>,
}

impl PatternCatalogue {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            schema_version: PATTERN_CATALOGUE_SCHEMA.to_owned(),
            data_state: DataState::Unavailable,
            source_message: message.into(),
            patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSummary {
    pub id: String,
    pub scope: String,
    pub title: String,
    pub status: String,
    pub progress: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanDetail {
    pub schema_version: String,
    pub summary: PlanSummary,
    pub purpose: String,
    pub actions_enabled: bool,
    pub action_message: String,
    pub timeline: Vec<PlanTimelineEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTimelineEntry {
    pub id: String,
    pub title: String,
    pub status: String,
    pub validation_contract: Option<String>,
    pub readiness: bool,
}

impl PlanDetail {
    pub fn read_only(
        summary: PlanSummary,
        purpose: String,
        timeline: Vec<PlanTimelineEntry>,
    ) -> Self {
        Self {
            schema_version: PLAN_DRIVER_SCHEMA.to_owned(),
            summary,
            purpose,
            actions_enabled: false,
            action_message: "Approval and execution actions are deferred beyond read-only Wave 1."
                .to_owned(),
            timeline,
        }
    }
}
