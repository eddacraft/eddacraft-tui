//! Witness chain (MLP-002): hash-chained ndjson of which protection layers
//! ran and under which rule set.
//!
//! Append-only integrity primitive for status / doctor / release gates.

mod genesis;
mod line;
mod manifest;
mod paths;
mod verify;
mod writer;

pub use genesis::GenesisAnchor;
pub use line::{LineHash, WitnessLine, WitnessRecord, compute_line_hash};
pub use manifest::{ManifestEntry, append_manifest_entry, manifest_path, manifest_tail};
pub use paths::witness_paths;
#[allow(deprecated)]
// Re-export the linear-only wrapper for out-of-tree callers; in-tree callers should use `verify_chain_dag`.
pub use verify::{ChainReport, DagVerification, VerifyError, verify_chain, verify_chain_dag};
pub use writer::{
    AppendOutcome, ChainHead, DEFAULT_LOCK_ACQUIRE_TIMEOUT, LOCK_TIMEOUT_ENV, RolloverPolicy,
    WitnessWriter, WriterError, lock_timeout_from_env,
};
