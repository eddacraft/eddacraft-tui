//! Settings truth service (SETCON / ADR-132).
//!
//! Distinguishes configured, requested, resolved and active state. Surfaces
//! consume this crate; they do not read configuration files for settings
//! purposes and they do not write configuration except through the service.

pub mod catalogue;
pub mod constraints;
pub mod envelope;
pub mod exit_codes;
pub mod health;
pub mod redaction;
pub mod resolver;
pub mod runtime_state;
pub mod seed;
pub mod service;
pub mod types;

pub use catalogue::{Catalogue, CatalogueEntry, CatalogueError};
pub use constraints::{Constraint, ConstraintError, PolicyBundle};
pub use envelope::{Envelope, EnvelopeCommand, SCHEMA_VERSION};
pub use exit_codes::{SettingsOutcome, code_for};
pub use health::{Health, HealthStatus};
pub use redaction::{RedactionError, fail_closed, redact_setting_value, redact_value};
pub use resolver::{Declaration, ProvenanceEvent, ResolutionEvent, ResolvedSetting, Resolver};
pub use runtime_state::{
    Attestation, EvidenceChannel, EvidenceTrust, RuntimeState, classify_runtime_state,
};
pub use seed::first_release_catalogue;
pub use service::{SettingsError, SettingsService, Snapshot};
pub use types::{
    ConsequenceClass, EvidenceMode, HealthRelevance, MergeSemantics, Scope, Sensitivity,
    SettingGroup, SettingKey, ValueType, WorkflowState,
};
