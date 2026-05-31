//! Anvil baseline store (MLP-007).
//!
//! Owns the `anvil/baseline.json` on-disk shape, the move-resistant
//! fingerprint algorithm, and the comparison primitive that lets a
//! later scan partition findings into "already in the baseline" vs.
//! "new since adoption". This is the adoption mechanism for the
//! defense-in-depth model: an existing repo with pre-existing
//! findings can adopt Anvil without a warning storm, because every
//! finding present at adoption time enters the baseline and only
//! *new* findings escalate (CLAUDE.md "warnings over blocks",
//! "new edges only").
//!
//! ## Scope (MLP-007 v1)
//!
//! - [`BaselineFinding`] / [`Baseline`] — on-disk schema (versioned
//!   via `format_version`) with stable, sorted serialisation.
//! - [`compute_fingerprint`] — sha256-derived 16-hex-char digest over
//!   `(rule_id, normalised_snippet)`. Move-resistant: the same
//!   finding moving lines or files keeps the same fingerprint as
//!   long as the snippet is unchanged.
//! - [`load`] / [`save`] — TOCTOU-hardened I/O against
//!   `anvil/baseline.json` (symlink refusal pattern matches MLP-001).
//! - [`Baseline::diff`] — partitions a new scan into `unchanged`,
//!   `added`, `removed` relative to the baseline.
//! - [`save_with_genesis`] — MLP2-013: save + emit the first
//!   witness chain line with `GENESIS-BASELINED` (`baseline.cutoff_commit`
//!   on the line body) or `GENESIS-FRESH` (no cutoff). Idempotent
//!   on an existing chain so re-runs do not duplicate the genesis.
//! - [`analyze_refresh`] / [`RefreshSuspicion`] / [`SuspicionThresholds`]
//!   — adversarial-refresh detection: compares a proposed baseline
//!   refresh against the current baseline and flags a suspicious drop
//!   or churn of findings as [`REFRESH_DEGRADED_REASON`]
//!   (`degraded:baseline-suspicious`). The thresholds are caller-tunable.
//!
//! ## Out of scope (deferred to consumers / follow-up)
//!
//! - `anvil baseline` CLI command — lands with MLP-003 hook lane,
//!   which is where the rule engine is invoked. The library here
//!   is the building block.
//! - Scanner integration — populating findings from
//!   `anvil-checks` runs through that crate's own pipeline; the
//!   baseline crate is engine-agnostic.
//! - `cutoff_commit` pinning into `anvil/policy.yml` — owned by
//!   MLP-006 (L4 policy framework) and implemented in
//!   `anvil_l4::pin_cutoff_commit` (MLP2-031). The baseline crate
//!   exposes the SHA on the on-disk record itself for round-trip;
//!   the higher-level `anvil baseline` orchestrator (MLP2-032)
//!   calls `anvil_l4::pin_cutoff_commit` after `save()` so that
//!   `anvil/baseline.json` and `anvil/policy.yml` agree on the
//!   cutoff in one flow.
//! - Hook installation — MLP-003 / MLP-008 own framework-specific
//!   install paths.
//! - Async continuation for >100k files — performance work item.
//!
//! ## Format
//!
//! `anvil/baseline.json` is canonical-ish JSON with sorted object
//! keys at the top level and the `findings` array sorted by
//! `(rule_id, file_path, fingerprint)` so two adopters of the same
//! tree produce byte-identical output (deterministic per CLAUDE.md
//! "same input, same output"):
//!
//! ```json
//! {
//!   "cutoff_commit": null,
//!   "findings": [
//!     {"file_path":"src/lib.rs","fingerprint":"f00d…","rule_id":"anti-pattern:guardrail-suppression"}
//!   ],
//!   "format_version": 1,
//!   "metadata": {
//!     "created_at": "2026-05-13T00:00:00Z",
//!     "created_by_version": "0.7.0-beta",
//!     "project_uuid": "01997e4a-1b2c-7345-8901-abcdef123456"
//!   }
//! }
//! ```

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
