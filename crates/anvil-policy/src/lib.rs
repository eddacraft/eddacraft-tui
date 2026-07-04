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

pub mod config;
pub mod eval;
pub mod exceptions;
