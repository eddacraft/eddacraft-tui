//! INTL-008: blocked-launch UX.
//!
//! When the daemon refuses a launch (fenced worktree, daemon down,
//! handshake failure) the launcher prints an actionable message on
//! stderr and exits with one of the codes from
//! [`crate::exit_codes`]. Wrappers and CI harnesses can switch on
//! the exit code; humans can read the message.
//!
//! The text is single-paragraph by design — operators reading this
//! at 11pm get one signal and one suggested command, not a wall.

use std::path::{Path, PathBuf};

use crate::exit_codes::{EXIT_DAEMON_UNAVAILABLE, EXIT_FENCED};
use crate::preflight::PreflightDecision;

/// Reason the launcher refused to run the wrapped command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalReason {
    /// Daemon could not be reached on the per-user rendezvous, or
    /// the handshake failed. Carries the operator-facing message
    /// (built upstream by [`crate::preflight::refusal_message_for`]).
    DaemonUnavailable { message: String },
    /// Worktree is fenced — the daemon's fence store says so.
    Fenced {
        worktree: PathBuf,
        fence_reason: String,
    },
}

impl RefusalReason {
    /// Translate a [`PreflightDecision::Fenced`] into a refusal.
    pub fn from_preflight(decision: PreflightDecision) -> Option<Self> {
        match decision {
            PreflightDecision::Proceed => None,
            PreflightDecision::Fenced { worktree, reason } => Some(Self::Fenced {
                worktree,
                fence_reason: reason,
            }),
        }
    }
}

/// Stable exit code for `reason`.
#[must_use]
pub fn exit_code_for(reason: &RefusalReason) -> i32 {
    match reason {
        RefusalReason::DaemonUnavailable { .. } => EXIT_DAEMON_UNAVAILABLE,
        RefusalReason::Fenced { .. } => EXIT_FENCED,
    }
}

/// Render the operator-facing refusal text. Pure helper so tests
/// can pin the wording. Multi-line; ends with a trailing `\n`.
#[must_use]
pub fn render(reason: &RefusalReason) -> String {
    match reason {
        RefusalReason::DaemonUnavailable { message } => {
            format!("anvil-run: refusing to launch — {message}\n",)
        }
        RefusalReason::Fenced {
            worktree,
            fence_reason,
        } => format!(
            "anvil-run: refusing to launch — worktree {worktree} is fenced.\n  reason: {fence_reason}\n  unblock: `anvil intercept unblock {worktree}` (after addressing the reason)\n",
            worktree = display_path(worktree),
        ),
    }
}

fn display_path(p: &Path) -> String {
    p.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fenced_refusal_uses_the_fence_exit_code() {
        let reason = RefusalReason::Fenced {
            worktree: PathBuf::from("/tmp/wt"),
            fence_reason: "rule violation".into(),
        };
        assert_eq!(exit_code_for(&reason), EXIT_FENCED);
    }

    #[test]
    fn daemon_down_uses_the_unavailable_exit_code() {
        let reason = RefusalReason::DaemonUnavailable {
            message: "no socket".into(),
        };
        assert_eq!(exit_code_for(&reason), EXIT_DAEMON_UNAVAILABLE);
    }

    #[test]
    fn fenced_text_names_the_worktree_reason_and_unblock_command() {
        let reason = RefusalReason::Fenced {
            worktree: PathBuf::from("/work/api"),
            fence_reason: "L4 violation".into(),
        };
        let rendered = render(&reason);
        assert!(rendered.contains("worktree /work/api"));
        assert!(rendered.contains("L4 violation"));
        assert!(
            rendered.contains("anvil intercept unblock /work/api"),
            "operator-facing text must name the unblock command verbatim",
        );
    }

    #[test]
    fn daemon_unavailable_text_includes_the_upstream_message() {
        let reason = RefusalReason::DaemonUnavailable {
            message: "Start it with `anvil intercept start`".into(),
        };
        let rendered = render(&reason);
        assert!(rendered.contains("Start it with `anvil intercept start`"));
    }

    #[test]
    fn preflight_proceed_produces_no_refusal() {
        assert!(RefusalReason::from_preflight(PreflightDecision::Proceed).is_none());
    }
}
