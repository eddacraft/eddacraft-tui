//! Shared types for the `reasoning/` rule family.
//!
//! Reasoning-pattern rules (AI-001 onwards) flag prose in source comments
//! that justifies dubious code with appeals to authority, social proof, or
//! deflection — rather than technical reasoning. These rules consume comment
//! regions only and emit canonical [`Diagnostic`] values
//! (`anvil.diagnostic.v1`) tagged with [`Category::Reasoning`].
//!
//! The check-level `ReasoningCheckResult` mirrors the `passed` / `score` /
//! `message` shape used by the secret and anti-pattern checks so the surface
//! API stays uniform across rule families.

use serde::{Deserialize, Serialize};

use anvil_kernel_types::Diagnostic;

/// Configuration for the reasoning check entry point.
///
/// Currently empty besides defaults — kept as a dedicated type so future
/// rules (AI-002+) can grow tunables without breaking the surface.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningCheckConfig {
    /// Optional opt-in: when populated, restrict the run to this rule set.
    /// Empty (the default) means "every reasoning rule that ships enabled".
    #[serde(default)]
    pub rule_ids: Vec<String>,
}

/// Outcome of a reasoning-rule run, mirroring the secret / antipattern
/// `*CheckResult` shapes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningCheckResult {
    pub passed: bool,
    pub score: u8,
    pub message: String,
    pub findings: Vec<Diagnostic>,
}

impl ReasoningCheckResult {
    /// Build a clean-pass result with no findings.
    #[must_use]
    pub fn clean() -> Self {
        Self {
            passed: true,
            score: 100,
            message: "No reasoning-pattern issues detected".to_string(),
            findings: Vec::new(),
        }
    }
}
