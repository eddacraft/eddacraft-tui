//! Anvil witness chain primitive (MLP-002).
//!
//! In-tree, hash-chained ndjson record of which protection layers
//! fired on which commit. Designed to survive `git worktree add`
//! because it lives under `anvil/`, not `.anvil/`.
//!
//! ## v1 scope shipped here
//!
//! - [`WitnessLine`] — one record per line; canonical UTF-8 JSON
//!   with sorted keys so two machines emitting the same logical
//!   record produce byte-identical lines. (Strings carrying non-ASCII
//!   commit messages or UUIDs survive intact via standard JSON
//!   string semantics.)
//! - [`compute_line_hash`] — SHA-256 of the canonical bytes of a line
//!   (without its trailing newline); used as the `prev_line_hash` of
//!   the next line.
//! - [`GenesisAnchor`] — the two anchors (`GENESIS-FRESH` /
//!   `GENESIS-BASELINED:<commit_sha>`) that bootstrap the chain.
//! - [`WitnessWriter`] — flock-serialised append-only writer with
//!   automatic rollover when the active file crosses either the line
//!   count or byte size threshold. Rollover happens **inside** the
//!   lock so two concurrent writers cannot race a half-completed
//!   archive into existence. [`WitnessWriter::append`] reports the
//!   post-append state — and any archive path written on rollover —
//!   via [`AppendOutcome`].
//! - [`verify_chain`] — walk the active file (and optionally a list
//!   of archive files in sequence order) and confirm the hash chain
//!   is unbroken; surfaces tamper / deletion / genesis-mismatch.
//!   Linear-only contract; a thin wrapper over [`verify_chain_dag`].
//! - [`verify_chain_dag`] (MLP2-011) — DAG-aware walker that joins on
//!   merge nodes' `parent_commits[]` + `prev_line_hashes[]`. Strict
//!   superset of `verify_chain`: every linear chain that the legacy
//!   verifier accepted is still accepted, plus merge-shaped chains
//!   produced by `anvil hook post-merge` via
//!   [`anvil_hook::merge_witness_plan`][merge-plan].
//!
//! [merge-plan]: https://docs.rs/eddacraft-anvil-hook/latest/anvil_hook/fn.merge_witness_plan.html
//!
//! ## Deferred follow-ups
//! - **`merge=union -text` integration** — the orchestrator already
//!   pre-positions `.gitattributes` (MLP-001 step 1a-b). The witness
//!   crate itself stays format-stable so the union merge can
//!   trivially concatenate without producing duplicate lines.
//! - **Archive manifest event stream.** `anvil/witness/manifest/chain.ndjson`
//!   recording rollover events is filed as MLP-002b; the writer
//!   already returns the archive path on rollover so the manifest
//!   layer can be added without re-touching the writer.
//!
//! ## Design notes
//!
//! Canonical line encoding uses the same RFC 8785-style rules as
//! `anvil-config` (sorted object keys, no insignificant whitespace).
//! Reusing the rule rather than copying the implementation is
//! deliberate: a future refactor can pull both into a shared encoder
//! crate when the consumer count grows past two.

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
pub use writer::{AppendOutcome, RolloverPolicy, WitnessWriter, WriterError};
