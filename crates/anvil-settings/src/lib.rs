//! Settings truth service (SETCON / ADR-132).
//!
//! Distinguishes configured, requested, resolved and active state. Surfaces
//! consume this crate; they do not read configuration files for settings
//! purposes and they do not write configuration except through the service.

pub mod bootstrap;
mod canonical_json;
pub mod catalogue;
pub mod constraints;
pub mod envelope;
pub mod exit_codes;
pub mod health;
pub mod mutate;
pub mod posture;
pub mod redaction;
pub mod resolver;
pub mod runtime_state;
pub mod seed;
pub mod service;
pub mod types;

pub use bootstrap::{BootstrapError, BootstrapMutation, BootstrapSetting};
pub use catalogue::{Catalogue, CatalogueEntry, CatalogueError, ProjectConfigTarget};
pub use constraints::{ApprovalEvidence, Constraint, ConstraintError, PolicyBundle};
pub use envelope::{Envelope, EnvelopeCommand, SCHEMA_VERSION};
pub use exit_codes::{SettingsOutcome, code_for};
pub use health::{Health, HealthStatus};
pub use mutate::{ClassAOp, apply_class_a};
pub use posture::{
    AcceptanceSource, AcceptanceVerb, ActiveCell, EnforcementSource, GateSource, LastAction,
    MappingLegend, POSTURE_SCALE, PostureCell, PostureInputs, PostureRow, PostureSnapshot,
    PostureSurface, SurfaceActive, VerbFamily, posture_snapshot,
};
pub use redaction::{RedactionError, fail_closed, redact_setting_value, redact_value};
pub use resolver::{
    Declaration, ProvenanceEvent, ResolutionEvent, ResolvedSetting, Resolver, ResolverError,
};
pub use runtime_state::{
    Attestation, EvidenceChannel, EvidenceTrust, RuntimeState, classify_runtime_state,
};
pub use seed::first_release_catalogue;
pub use service::{SettingRow, SettingsError, SettingsService, Snapshot, SnapshotRequest};
pub use types::{
    ConsequenceClass, EvidenceMode, HealthRelevance, MergeSemantics, PersistenceTarget, Posture,
    Scope, Sensitivity, SettingGroup, SettingKey, ValueType, WorkflowState,
};
