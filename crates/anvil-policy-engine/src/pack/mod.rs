//! Policy pack admission: metadata schema and manifest loading (POLVAL).
//!
//! Validation runs before load at the facade boundary (ADR-040 D-2). This
//! module owns pure data shapes and manifest I/O only — no policy evaluation.
//! Under the POLRESET-002 retarget (ADR-098) pack admission lives here, in the
//! product-path engine crate, not in the OPA-era `anvil-policy` loader.

pub mod manifest;
pub mod metadata;
pub mod validator;

pub use manifest::{ManifestError, PackManifest, PolicyEntry, load_manifest};
pub use metadata::{MetadataError, PolicyMetadata, PolicySeverity, ensure_unique_ids};
pub use validator::{IssueCode, IssueSeverity, ValidationIssue, ValidationReport, validate_pack};
