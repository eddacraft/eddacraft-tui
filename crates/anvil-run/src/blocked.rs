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

use std::borrow::Cow;
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
            format!("anvil-run: refusing to launch — {message}\n")
        }
        RefusalReason::Fenced {
            worktree,
            fence_reason,
        } => {
            let display = display_path(worktree);
            let quoted = shell_quote(&display);
            format!(
                "anvil-run: refusing to launch — worktree {display} is fenced.\n  reason: {fence_reason}\n  unblock: `anvil intercept unblock {quoted}` (after addressing the reason)\n",
            )
        }
    }
}

fn display_path(p: &Path) -> String {
    p.display().to_string()
}

/// POSIX shell-quote `s` so it can be pasted as a single argument.
///
/// Returns the input unchanged when it contains only characters that
/// every POSIX shell treats literally; otherwise wraps it in single
/// quotes, escaping any embedded single quotes via the standard
/// `'\''` close-escape-reopen sequence. Output is *not* portable to
/// `cmd.exe`, but the surrounding refusal text targets POSIX shells.
fn shell_quote(s: &str) -> Cow<'_, str> {
    if !s.is_empty()
        && s.bytes().all(|b| {
            matches!(b,
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
                | b'_' | b'-' | b'.' | b'/' | b':' | b'@' | b'%' | b'+' | b','
            )
        })
    {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    Cow::Owned(out)
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

    #[test]
    fn fenced_text_shell_quotes_worktree_with_spaces() {
        let reason = RefusalReason::Fenced {
            worktree: PathBuf::from("/work/some dir"),
            fence_reason: "x".into(),
        };
        let rendered = render(&reason);
        assert!(
            rendered.contains("anvil intercept unblock '/work/some dir'"),
            "spaces in path must be quoted in the unblock command:\n{rendered}",
        );
    }

    #[test]
    fn fenced_text_shell_escapes_embedded_single_quote() {
        let reason = RefusalReason::Fenced {
            worktree: PathBuf::from("/work/it's-fine"),
            fence_reason: "x".into(),
        };
        let rendered = render(&reason);
        assert!(
            rendered.contains("anvil intercept unblock '/work/it'\\''s-fine'"),
            "embedded single quotes must use the close/escape/reopen form:\n{rendered}",
        );
    }

    #[test]
    fn shell_quote_leaves_simple_paths_alone() {
        assert_eq!(shell_quote("/work/api"), "/work/api");
        assert_eq!(shell_quote("name-1.2_v3"), "name-1.2_v3");
    }

    #[test]
    fn shell_quote_wraps_paths_with_metacharacters() {
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("a$b"), "'a$b'");
        assert_eq!(shell_quote("a;b"), "'a;b'");
        assert_eq!(shell_quote("a`b"), "'a`b'");
        assert_eq!(shell_quote(""), "''");
    }
}
