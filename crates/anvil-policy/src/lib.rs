//! Anvil policy support crate.
//!
//! The legacy OPA-binary-subprocess evaluation path (`opa`, `evaluator`,
//! `loader`, `bundle`, `library`, `profiles`, `config_view`) was removed in
//! ADR-098 PR-C; the product evaluation path now lives in
//! `anvil-policy-engine` (regorus). What remains here is the crate's durable
//! role:
//!
//! - [`exceptions`] — the git-native policy exceptions store.
//! - [`eval`] — the eval-regression harness.
//! - [`config`] — `.anvil.yaml` policy configuration loading.
//! - [`attack`] — the prompt-attack regression pack runner (PATT-002).
//! - [`adversarial`] — the adversarial testing catalog: probe packs (ATC-002)
//!   and their eval-harness integration (ATC-003).

pub mod adversarial;
pub mod attack;
pub mod config;
pub mod eval;
pub mod exceptions;
