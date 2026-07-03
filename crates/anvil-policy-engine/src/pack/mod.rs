//! Policy pack admission: metadata schema, manifest loading, structural
//! validation, and test enforcement (POLVAL).
//!
//! Validation runs before load at the facade boundary (ADR-040 D-2). Metadata,
//! manifest, and validator layers are pure data shapes and I/O; the test runner
//! ([`test_runner`]) additionally executes each member's `*_test.rego` through
//! the facade [`crate::Engine`] to enforce that packs ship passing tests. Under
//! the POLRESET-002 retarget (ADR-098) pack admission lives here, in the
//! product-path engine crate, not in the OPA-era `anvil-policy` loader.

pub mod manifest;
pub mod metadata;
pub mod test_runner;
pub mod validator;

pub use manifest::{ManifestError, PackManifest, PolicyEntry, load_manifest};
pub use metadata::{MetadataError, PolicyMetadata, PolicySeverity, ensure_unique_ids};
pub use test_runner::{
    MemberTestResult, TestOutcome, TestRunError, TestRunReport, enforce_tests, run_pack_tests,
};
pub use validator::{IssueCode, IssueSeverity, ValidationIssue, ValidationReport, validate_pack};
