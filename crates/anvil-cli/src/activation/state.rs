//! Activation protection state vocabulary.
//!
//! The vocabulary is fixed and small (six variants) so user-facing copy
//! across surfaces (start, status, doctor, tutorial) can never drift.

use serde::{Deserialize, Serialize};

/// The single word a surface prints to describe the user's current
/// activation state. This is the ONLY allowed vocabulary for activation
/// outcomes — surfaces must never invent ad-hoc strings like
/// "config written", "ready to go", or "almost protected" because those
/// over-claim what was actually verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionState {
    /// Live MCP `anvil_validate_write` evidence has been observed
    /// inside this repo. Protection is literal — pre-write validation
    /// is happening at save time.
    Protecting,

    /// MCP config is written safely and the server starts, but the
    /// editor or agent must be restarted before validation can run.
    /// This is **not** protection — restart is still required.
    ReadyRestartRequired,

    /// The save-time watcher is running; pre-write MCP attachment is
    /// not in evidence. This is the honest fallback label — it is
    /// weaker than `Protecting` and surfaces must say so.
    Watching,

    /// Activation cannot make a literal protection claim and the user
    /// has actionable next steps (install missing config, accept a
    /// pending change, run a setup command, etc.).
    NeedsAction,

    /// Activation cannot claim coverage in this repo because the
    /// platform, language profile, or environment is out of scope for
    /// the current release. The surface must name the gap, not pretend
    /// coverage.
    Unsupported,

    /// Activation hit a hard error before it could establish any
    /// state. Surfaces should print the cause and a concrete repair.
    Error,
}

impl ProtectionState {
    /// The literal vocabulary word users see (`snake_case`) — also
    /// the JSON serialisation key.
    pub fn label(self) -> &'static str {
        match self {
            ProtectionState::Protecting => "protecting",
            ProtectionState::ReadyRestartRequired => "ready_restart_required",
            ProtectionState::Watching => "watching",
            ProtectionState::NeedsAction => "needs_action",
            ProtectionState::Unsupported => "unsupported",
            ProtectionState::Error => "error",
        }
    }

    /// One-line human-readable description of the state. Surfaces use
    /// this directly so the wording cannot drift between commands.
    pub fn headline(self) -> &'static str {
        match self {
            ProtectionState::Protecting => {
                "Protecting — pre-write validation is live in this repo."
            }
            ProtectionState::ReadyRestartRequired => {
                "Ready, restart required — restart your editor or agent so the MCP server attaches."
            }
            ProtectionState::Watching => {
                "Watching — save-time fallback only; this is weaker than pre-write validation."
            }
            ProtectionState::NeedsAction => {
                "Needs action — finish activation before anvil can protect this repo."
            }
            ProtectionState::Unsupported => {
                "Unsupported — this repo or platform is outside the current release's coverage."
            }
            ProtectionState::Error => {
                "Error — activation could not be established; see the diagnostic for details."
            }
        }
    }
}

impl std::fmt::Display for ProtectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_snake_case_and_unique() {
        let labels = [
            ProtectionState::Protecting.label(),
            ProtectionState::ReadyRestartRequired.label(),
            ProtectionState::Watching.label(),
            ProtectionState::NeedsAction.label(),
            ProtectionState::Unsupported.label(),
            ProtectionState::Error.label(),
        ];
        for label in &labels {
            assert!(
                label.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "label {label} is not snake_case"
            );
        }
        let mut seen = labels.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 6, "labels must be unique");
    }

    #[test]
    fn json_serialises_to_label() {
        let s = serde_json::to_string(&ProtectionState::ReadyRestartRequired).unwrap();
        assert_eq!(s, "\"ready_restart_required\"");
    }

    #[test]
    fn watching_headline_is_explicitly_weaker_than_protecting() {
        let watching = ProtectionState::Watching.headline().to_lowercase();
        // The watch fallback must explicitly say it is weaker — the
        // adversarial-review concern was that "watching" could be
        // misread as full protection.
        assert!(
            watching.contains("fallback") || watching.contains("weaker"),
            "watching headline must mark itself as fallback / weaker than \
             pre-write validation, got: {}",
            ProtectionState::Watching.headline()
        );
    }

    #[test]
    fn protecting_headline_is_literal() {
        let h = ProtectionState::Protecting.headline().to_lowercase();
        // The literal claim word must appear so surfaces cannot dilute
        // it with hedge wording.
        assert!(
            h.contains("pre-write"),
            "protecting headline must reference pre-write validation, got: {}",
            ProtectionState::Protecting.headline()
        );
    }
}
