//! Anvil Review Capsule v0 — schema types for the `anvil.capsule.v1`
//! manifest and `anvil.capsule-verification.v1` verdict document
//! (ADR-074, GITGOV-003).
//!
//! A review capsule is a file-first, inspectable directory packaging a
//! commit range's governance evidence so a reviewer, auditor, or
//! supplier can verify it locally without trusting Anvil Cloud
//! (ADR-072). This crate owns the frozen schema surface; collectors
//! (GITGOV-005..008) and the verification engine (GITGOV-009) build on
//! it without re-modelling evidence — witness lines stay verbatim
//! `anvil-witness::WitnessLine`, rule identity stays
//! `anvil_rules::rules_sha`.

pub mod canonical;
pub mod collect;
pub mod errors;
pub mod manifest;
pub mod verification;

pub use canonical::{canonical_json_bytes, sha256_hex};
pub use collect::{COMMITS_SCHEMA, CommitEntry, CommitsDocument, collect_commits};
pub use errors::CapsuleError;
pub use manifest::{CAPSULE_SCHEMA, CapsuleManifest, CapsuleRange, Producer, REQUIRED_FILES};
pub use verification::{CapsuleVerification, CheckResult, VERIFICATION_SCHEMA, Verdict};

/// Probe a document's `schema` field and gate it against `expected`
/// **before** strict deserialisation, so version mismatch surfaces as
/// [`CapsuleError::SchemaMismatch`] rather than an opaque
/// unknown-field parse error from `deny_unknown_fields`.
pub(crate) fn schema_gate(bytes: &[u8], expected: &'static str) -> Result<(), CapsuleError> {
    #[derive(serde::Deserialize)]
    struct SchemaProbe {
        schema: String,
    }
    let probe: SchemaProbe =
        serde_json::from_slice(bytes).map_err(|e| CapsuleError::Parse(e.to_string()))?;
    if probe.schema != expected {
        return Err(CapsuleError::SchemaMismatch {
            expected,
            found: probe.schema,
        });
    }
    Ok(())
}
