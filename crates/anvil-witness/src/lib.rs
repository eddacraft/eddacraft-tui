//! Anvil witness chain primitive (MLP-002).
//!
//! In-tree, hash-chained ndjson record of which protection layers
//! fired on which commit. Designed to survive `git worktree add`
//! because it lives under `anvil/`, not `.anvil/`.
//!
//! ## v1 scope shipped here
//!
//! - [`WitnessLine`] — one record per line; ASCII-safe canonical JSON
//!   with sorted keys so two machines emitting the same logical
//!   record produce byte-identical lines.
//! - [`compute_line_hash`] — SHA-256 of the canonical bytes of a line
//!   (without its trailing newline); used as the `prev_line_hash` of
//!   the next line.
//! - [`GenesisAnchor`] — the two anchors (`GENESIS-FRESH` /
//!   `GENESIS-BASELINED:<commit_sha>`) that bootstrap the chain.
//! - [`WitnessWriter`] — flock-serialised append-only writer with
//!   automatic rollover when the active file crosses either the line
//!   count or byte size threshold. Rollover happens **inside** the
//!   lock so two concurrent writers cannot race a half-completed
//!   archive into existence.
//! - [`verify_chain`] — walk the active file (and optionally a list
//!   of archive files in sequence order) and confirm the hash chain
//!   is unbroken; surfaces tamper / deletion / genesis-mismatch.
//!
//! ## Deferred follow-ups
//!
//! - **DAG-aware merge verification.** Merge commits will carry
//!   `parent_commits[]` and a `prev_line_hashes[]` array; the verifier
//!   needs a graph walk rather than a strict linear chain. Lands with
//!   MLP-005 post-merge hook.
//! - **`merge=union -text` integration** — the orchestrator already
//!   pre-positions `.gitattributes` (MLP-001 step 1a-b). The witness
//!   crate itself stays format-stable so the union merge can
//!   trivially concatenate without producing duplicate lines.
//! - **Archive manifest event stream.** `anvil/witness/manifest/chain.ndjson`
//!   recording rollover events is filed as MLP-002b; the writer
//!   already returns the archive path on rollover so the manifest
//!   layer can be added without re-touching the writer.
//! - **80-writer stress test.** Concurrency safety is exercised at 16
//!   writers in `tests/concurrency.rs`; CI hardware can comfortably
//!   handle that without flaking. The 80-writer harness is documented
//!   in the module test file as a stress benchmark, gated behind
//!   `--ignored` so developers can run it on demand.
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
mod verify;
mod writer;

pub use genesis::GenesisAnchor;
pub use line::{LineHash, WitnessLine, WitnessRecord, compute_line_hash};
pub use verify::{ChainReport, VerifyError, verify_chain};
pub use writer::{RolloverPolicy, WitnessWriter, WriterError};
