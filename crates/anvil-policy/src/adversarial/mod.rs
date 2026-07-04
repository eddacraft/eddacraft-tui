//! Adversarial testing catalog — probe packs, execution, and reporting (ATC).
//!
//! Built on the pure taxonomy in
//! [`anvil_kernel_types::adversarial`](anvil_kernel_types::adversarial)
//! (ATC-001), this module owns the reference-crate substrate for adversarial
//! *test assets*:
//!
//! - [`registry`] — ATC-002: loadable, versioned probe packs, containment-safe
//!   discovery, and selection by risk profile.
//! - [`execution`] — ATC-003: deterministic probe execution folded into the
//!   frozen eval-harness [`EvalRunSummary`](crate::eval::EvalRunSummary) shape so
//!   probe runs appear alongside policy suites without touching the eval
//!   `--json` v1 contract.
//!
//! Nothing here scans, attacks, or makes a policy decision: it defines and runs
//! deterministic probe fixtures for regression tracking.

pub mod execution;
pub mod registry;

pub use execution::{
    PROBE_SUITE_PREFIX, ProbeExecutor, ProbeOutcome, ProbeRunReport, category_from_suite,
    probe_suite_name, run_probe_pack,
};
pub use registry::{
    DiscoveryError, LoadedProbePack, ProbePack, ProbePackDiscovery, ProbePackError, ProbePackRef,
    ProbeRegistry, RegistryLoadError, RejectedEntry, RejectionReason, RiskProfile,
    discover_and_load, discover_probe_packs, load_probe_pack,
};
