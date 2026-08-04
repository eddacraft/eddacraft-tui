//! ACTTUI-019: shared protection facts for `anvil start` and `anvil status`.
//!
//! Start and status use different top-level vocabulary words (`protecting`
//! vs worktree claim states like `warming`). Both must still name the **same
//! subordinate facts** (MCP live/wired, daemon attestation, save-time) so
//! operators never see contradictory stories without an explicit layer line.

use super::diagnostic::ActivationDiagnostic;

/// MCP pre-write posture shared by start and status fact lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpPosture {
    Live,
    WiredRestartPending,
    NotLive,
}

impl McpPosture {
    #[must_use]
    pub fn from_diagnostic(diag: &ActivationDiagnostic) -> Self {
        if diag.mcp_pre_write_live() {
            Self::Live
        } else if diag.mcp_pre_write_wired_or_live() {
            Self::WiredRestartPending
        } else {
            Self::NotLive
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::WiredRestartPending => "wired (restart pending)",
            Self::NotLive => "not live",
        }
    }

    #[must_use]
    pub const fn is_live(self) -> bool {
        matches!(self, Self::Live)
    }

    #[must_use]
    pub const fn is_wired_or_live(self) -> bool {
        matches!(self, Self::Live | Self::WiredRestartPending)
    }
}

/// Overlapping protection facts both start and status must name consistently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedPostureFacts {
    /// Start-surface protection word (`protecting`, `ready_restart_required`, …).
    pub protection_label: String,
    pub mcp: McpPosture,
    pub daemon_attests: bool,
    pub save_time_attached: bool,
}

impl SharedPostureFacts {
    /// Project from the activation diagnostic (used by `anvil start` and by
    /// `anvil status`, which already loads the same diagnostic).
    #[must_use]
    pub fn from_diagnostic(diag: &ActivationDiagnostic) -> Self {
        Self {
            protection_label: diag.protection_state().label().to_string(),
            mcp: McpPosture::from_diagnostic(diag),
            daemon_attests: diag.daemon_attestation.attests_worktree(),
            save_time_attached: diag.save_time_driver_attached,
        }
    }

    /// Canonical subordinate fact lines — byte-identical on start and status.
    #[must_use]
    pub fn fact_lines(&self) -> Vec<String> {
        vec![
            format!("mcp: {}", self.mcp.label()),
            format!(
                "daemon: {}",
                if self.daemon_attests {
                    "attesting worktree"
                } else {
                    "not attesting"
                }
            ),
            format!(
                "save-time: {}",
                if self.save_time_attached {
                    "attached"
                } else {
                    "not attached"
                }
            ),
        ]
    }

    /// When status's worktree-claim word differs from start's protection word,
    /// explain using the same subordinate facts both surfaces print.
    #[must_use]
    pub fn meaning_for_status_claim(&self, claim: &str) -> Option<String> {
        let facts = self.fact_lines().join("; ");
        match claim {
            "warming" if self.mcp.is_live() => Some(format!(
                "Protection is warming even though MCP is live. Current posture: {facts}"
            )),
            "warming" if self.mcp.is_wired_or_live() => Some(format!(
                "Protection is warming because MCP is configured but not attached. Current posture: {facts}"
            )),
            "warming" => Some(format!(
                "Protection is warming because MCP is not attached. Current posture: {facts}"
            )),
            "full" | "pre_write_daemon" if !self.save_time_attached => Some(format!(
                "Protection is {claim}, but save-time is not attached. Current posture: {facts}"
            )),
            _ => {
                // Always surface facts when start protection word is not a
                // simple synonym of the claim (protecting vs full is fine).
                if claim != self.protection_label
                    && !(claim == "full" && self.protection_label == "protecting")
                    && !(claim == "pre_write_daemon" && self.protection_label == "protecting")
                {
                    Some(format!(
                        "Protection layers report different stages for {claim}. Current posture: {facts}"
                    ))
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activation::daemon_evidence::DaemonAttestation;
    use crate::activation::diagnostic::{
        ActivationDiagnostic, ConfigStatus, McpClientId, McpTier, WatchTier,
    };
    use crate::activation::language_profile::RepoLanguageProfile;
    use crate::activation::state::ProtectionState;
    use std::collections::BTreeMap;

    fn base_diag() -> ActivationDiagnostic {
        ActivationDiagnostic {
            config: ConfigStatus::Valid,
            mcp: BTreeMap::new(),
            watch: WatchTier::NotRequested,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: RepoLanguageProfile::default(),
            daemon_attestation: DaemonAttestation::NotProbed,
            save_time_driver_attached: false,
        }
    }

    #[test]
    fn fact_lines_are_stable_for_live_mcp() {
        let mut d = base_diag();
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::LiveValidation.into());
        d.daemon_attestation = DaemonAttestation::Enforced;
        d.save_time_driver_attached = true;
        let facts = SharedPostureFacts::from_diagnostic(&d);
        assert!(facts.mcp.is_live());
        assert_eq!(
            facts.fact_lines(),
            vec![
                "mcp: live".to_string(),
                "daemon: attesting worktree".to_string(),
                "save-time: attached".to_string(),
            ]
        );
        assert_eq!(facts.protection_label, ProtectionState::Protecting.label());
    }

    #[test]
    fn warming_with_live_mcp_meaning_names_same_facts_as_start() {
        let mut d = base_diag();
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::LiveValidation.into());
        let facts = SharedPostureFacts::from_diagnostic(&d);
        let meaning = facts.meaning_for_status_claim("warming").unwrap();
        assert!(meaning.contains("warming"));
        assert!(meaning.contains("mcp: live"));
        for line in facts.fact_lines() {
            assert!(
                meaning.contains(&line),
                "meaning must include fact line {line:?}: {meaning}"
            );
        }
    }

    #[test]
    fn wired_not_live_is_named_explicitly() {
        let mut d = base_diag();
        d.mcp
            .insert(McpClientId::Cursor, McpTier::RestartRequired.into());
        let facts = SharedPostureFacts::from_diagnostic(&d);
        assert!(!facts.mcp.is_live());
        assert!(facts.mcp.is_wired_or_live());
        assert!(facts.fact_lines()[0].contains("wired (restart pending)"));
    }
}
