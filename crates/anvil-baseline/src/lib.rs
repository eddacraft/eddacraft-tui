//! Baseline store (MLP-007): on-disk `anvil/baseline.json` shape and
//! move-resistant fingerprinting for new-edges-only comparison.
//!
//! Load/store/compare only — policy on what counts as a violation lives elsewhere.

mod diff;
mod finding;
mod io;
mod store;

pub use diff::{
    BaselineDiff, BaselineDiffEntry, DEGRADED_REASON as REFRESH_DEGRADED_REASON, RefreshSuspicion,
    SuspicionThresholds, analyze_refresh,
};
pub use finding::{BaselineFinding, FingerprintError, compute_fingerprint, normalize_snippet};
pub use io::{
    BASELINE_PATH, BASELINE_VALIDATION_AT, BaselineIoError, load, save, save_with_genesis,
};
pub use store::{Baseline, BaselineMetadata, FORMAT_VERSION, FormatError};
