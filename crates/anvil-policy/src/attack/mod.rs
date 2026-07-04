//! Prompt-attack regression packs — deterministic runner (PATT-002).
//!
//! Loads a versioned [`AttackPack`](runner::AttackPack) of self-contained
//! prompt-attack fixtures (PATT-001 [`AttackScenario`]s) and executes each one
//! through a [`DefenceObserver`](runner::DefenceObserver) — the injected
//! defence-under-test — producing normalised per-scenario outcomes with bounded
//! confidence metadata. Deterministic (fixtures are self-contained: no clock, no
//! network at eval time) and fail-closed (an unrecognised observed behaviour is
//! never treated as safe). The PATT-003 fail-policy maps a
//! [`PackRunReport`](runner::PackRunReport) to a gate decision.
//!
//! [`AttackScenario`]: anvil_kernel_types::attack_scenario::AttackScenario

pub mod runner;

pub use runner::{
    AttackPack, ConformanceObserver, DefenceObserver, Observation, PackLoadError, PackRunReport,
    ScenarioOutcome, load_pack, run_pack,
};
