//! Rule-set hashing and version-floor primitives (MLP-012).
//!
//! `rules_sha` locks the rule version used to produce a witness line
//! into the evidence stream, so L4 can verify that a recorded
//! validation matches a known, replayable rule set. ADR-037 §D-1
//! defines the witness envelope; ADR-038 makes the noise-discipline
//! consequences observable; this crate provides the computation.
//!
//! ## Scope (MLP-012 v1)
//!
//! - [`RulesShaInput`] — the four-field structured input
//!   (`anvil_version`, `config_sha`, `opa_runtime_version`,
//!   `rules`). Construct with [`RulesShaInput::try_new`], which sorts
//!   the rule list and validates `config_sha` shape + rule id form
//!   at the boundary.
//! - [`rules_sha`] — one-shot helper that builds + hashes.
//! - [`config_sha_from_canonical`] — convenience helper that takes
//!   canonical-JSON bytes (the only stable encoding) and returns the
//!   hex digest. Reuses anvil-config's canonical-JSON guarantee.
//! - [`RequiredAnvilVersion`] — semver floor parsing and comparison.
//!   The policy file pins a minimum `required_anvil_version`;
//!   consumers (MLP-003 hook, MLP-006 L4) check that the running
//!   anvil meets the floor before honouring a rule set. Callers
//!   should pass their own `env!("CARGO_PKG_VERSION")` for the
//!   current-version side of the comparison.
//!
//! ## Out of scope (deferred to consumers)
//!
//! - Daemon-side `(worktree_key, rules_sha) → ResolvedRuleSet` cache
//!   with `.anvil.*` watcher invalidation — owned by `anvil-intercept`
//!   when the daemon materialises (MLP-014 / INTD).
//! - In-flight evaluation pinning during config-update bursts — owned
//!   by the scheduler that drives evaluations.
//! - Hook-side `required_anvil_version` floor check at fire time —
//!   owned by MLP-003 (`anvil hook pre-commit`); it consumes the
//!   helpers here.
//! - L4 verification of witness `rules_sha` against a recognised
//!   version — owned by MLP-006 (anvil-l4 crate).
//! - Witness-writer wiring (setting `WitnessLine.rules_sha`) — the
//!   field exists today (MLP-002); the writer call site lives in the
//!   hook (MLP-003) where the rule set is resolved.
//!
//! ## Determinism model
//!
//! `rules_sha` is a SHA-256 of canonical JSON over four fields:
//!
//! ```text
//! sha256(canonical_json({
//!     "anvil_version":      <semver string>,
//!     "config_sha":         <64-char lowercase hex string>,
//!     "opa_runtime_version": <semver string>,
//!     "rules":              [<sorted ASCII rule ids>...],
//! }))
//! ```
//!
//! Top-level keys are sorted lexicographically by emitting through a
//! `BTreeMap` with named keys (not by round-tripping through
//! `serde_json::to_value`, which would be sensitive to the
//! `preserve_order` feature flag). The rule list is sorted explicitly
//! at input-construction time, not in the encoder — if a future
//! reviewer decides "the canonical encoder should sort all arrays"
//! they would collapse different rule-precedence configs to the same
//! hash, so the sort is local and visible where intent is local.
//!
//! Cross-format determinism rides on
//! `anvil_config::canonical_json_bytes`: feeding it equivalent
//! yaml / json / toml inputs yields byte-identical bytes, so
//! `config_sha` is the same across formats, so `rules_sha` is the
//! same across formats.

mod input;
mod version;

pub use input::{
    OPA_RUNTIME_VERSION, RulesShaError, RulesShaInput, config_sha_from_canonical, rules_sha,
};
pub use version::{RequiredAnvilVersion, VersionFloorError};
