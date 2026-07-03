//! Shared enforcement posture and the single posture→decision rule.
//!
//! Both guidance layers — [`crate::context::guidance`] (assertion violations)
//! and [`crate::io_risk::guidance`] (IO risk findings) — decide whether a
//! finding *blocks* using the same rule. That rule lives here, once, so the two
//! layers cannot drift. Each layer keeps a thin adapter mapping its own severity
//! vocabulary onto the [`decision_for_band`] boolean.

use anvil_kernel_types::diagnostics::ControlDecision;
use serde::{Deserialize, Serialize};

/// The enforcement posture a caller applies when turning findings into
/// decisions.
///
/// The default is [`EnforcementPosture::Warn`] (ADR-002, warnings over blocks):
/// nothing blocks until a caller opts into [`EnforcementPosture::Enforce`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnforcementPosture {
    /// Warnings-first: every finding is advisory; nothing blocks (exit 0).
    #[default]
    Warn,
    /// Enforce: high-signal findings block; lower bands stay advisory.
    Enforce,
}

/// The one posture→decision rule shared by every guidance layer.
///
/// Under [`EnforcementPosture::Warn`] every finding is [`ControlDecision::Warn`]
/// (ADR-002). Under [`EnforcementPosture::Enforce`] a high-signal finding
/// (`high_or_critical`) blocks; everything else warns. Callers map their own
/// severity vocabulary into the `high_or_critical` boolean via a thin adapter.
#[must_use]
pub(crate) fn decision_for_band(
    high_or_critical: bool,
    posture: EnforcementPosture,
) -> ControlDecision {
    match posture {
        EnforcementPosture::Warn => ControlDecision::Warn,
        EnforcementPosture::Enforce => {
            if high_or_critical {
                ControlDecision::Block
            } else {
                ControlDecision::Warn
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posture_default_is_warn() {
        assert_eq!(EnforcementPosture::default(), EnforcementPosture::Warn);
    }

    #[test]
    fn posture_warn_never_blocks() {
        assert_eq!(
            decision_for_band(true, EnforcementPosture::Warn),
            ControlDecision::Warn
        );
        assert_eq!(
            decision_for_band(false, EnforcementPosture::Warn),
            ControlDecision::Warn
        );
    }

    #[test]
    fn posture_enforce_blocks_only_high_band() {
        assert_eq!(
            decision_for_band(true, EnforcementPosture::Enforce),
            ControlDecision::Block
        );
        assert_eq!(
            decision_for_band(false, EnforcementPosture::Enforce),
            ControlDecision::Warn
        );
    }
}
