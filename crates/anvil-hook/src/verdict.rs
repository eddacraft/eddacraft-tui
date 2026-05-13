/// Hook verdict — the closed set of outcomes a hook can emit, per
/// ADR-038 §D-1 + §D-6 noise discipline.
///
/// Every variant maps to exactly one terse stderr line (or none, for
/// the silent-success case) and exactly one exit code. The mapping
/// is implemented by [`render_verdict`]; this enum is the contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Validation passed cleanly. Silence is the win signal.
    Pass,
    /// Validation found warn-level findings; commit proceeds.
    /// `count` is the number of findings; `witness_id` is the
    /// short pointer the user types into `anvil show <id>`.
    Warn { count: usize, witness_id: String },
    /// Validation found block-level findings; commit refused.
    Block {
        count: usize,
        witness_id: String,
        reason: BlockReason,
    },
    /// An internal error (daemon unreachable, embedded fallback
    /// failed, hash chain corruption recovery) prevented a clean
    /// verdict. Per ADR-038 §D-6, the user is NOT held hostage to
    /// Anvil's health — commit proceeds; L4 picks up.
    InternalError { class: ErrorClass },
    /// Witness file write failed (disk full / permissions). The
    /// commit is refused: per ADR-038, "we don't claim what we
    /// can't witness."
    WitnessWriteFailed,
}

/// Why a commit was blocked. Kept narrow so the user-facing line
/// stays short.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    /// One or more findings exceeded `severity_threshold`.
    Findings,
    /// The witness chain's hash linkage was broken on inspection.
    ChainBroken,
    /// A pre-push range contained a commit with no L3 witness AND
    /// the resolved per-branch policy refused to admit it
    /// (`OnNoWitness::Reject`, or `Requirement::L3Only` with no
    /// matching witness).
    UnwitnessedCommit,
}

/// Class of internal error, used in the one-line stderr message and
/// for deduplication in [`crate::SuppressionLog`].
///
/// The discriminant is what gets compared for suppression purposes:
/// `DaemonUnreachable` should fire once per session, not per commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Daemon RPC failed; embedded fallback ran and succeeded.
    DaemonUnreachable,
    /// Daemon RPC failed AND embedded fallback errored.
    EmbeddedFailed,
    /// `std::panic::set_hook` fired during hook execution.
    Panic,
    /// Time budget exceeded; partial verdict surfaced.
    TimedOut,
}

impl ErrorClass {
    /// Component string used in `anvil: <component> errored …`.
    pub fn component(self) -> &'static str {
        match self {
            ErrorClass::DaemonUnreachable => "daemon",
            ErrorClass::EmbeddedFailed | ErrorClass::TimedOut => "validation",
            ErrorClass::Panic => "hook",
        }
    }
}

/// Output of [`render_verdict`]: the terse stderr line (empty for
/// silent-success) and the exit code.
///
/// Splitting these out keeps the function pure and unit-testable
/// without spawning a process. The CLI wrapper does the actual
/// `eprintln!` + `std::process::exit` at the boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedVerdict {
    /// Single line of stderr output (no trailing newline). Empty
    /// string means "silent — emit nothing."
    pub stderr_line: String,
    /// Exit code per ADR-038 §D-6.
    pub exit_code: i32,
}

/// Render a verdict into ADR-038-compliant stderr text + exit code.
///
/// Format pinned by ADR-038 §D-1:
///
/// - Pass: silent, exit 0.
/// - Warn: `anvil: N warning(s) (commit allowed) — anvil show <id>`, exit 0.
/// - Block (findings): `anvil: N finding(s) (block) — anvil show <id>`, exit 1.
/// - Block (chain): `anvil: chain integrity broken — anvil show <id>`, exit 1.
/// - `InternalError`: `anvil: <component> errored (anvil doctor for details)`, exit 0.
/// - `WitnessWriteFailed`: `anvil: witness write failed — refused`, exit 1.
pub fn render_verdict(verdict: &Verdict) -> RenderedVerdict {
    match verdict {
        Verdict::Pass => RenderedVerdict {
            stderr_line: String::new(),
            exit_code: 0,
        },
        Verdict::Warn { count, witness_id } => RenderedVerdict {
            stderr_line: format!(
                "anvil: {count} warning(s) (commit allowed) — anvil show {witness_id}"
            ),
            exit_code: 0,
        },
        Verdict::Block {
            count,
            witness_id,
            reason: BlockReason::Findings,
        } => RenderedVerdict {
            stderr_line: format!("anvil: {count} finding(s) (block) — anvil show {witness_id}"),
            exit_code: 1,
        },
        Verdict::Block {
            witness_id,
            reason: BlockReason::ChainBroken,
            ..
        } => RenderedVerdict {
            stderr_line: format!("anvil: chain integrity broken — anvil show {witness_id}"),
            exit_code: 1,
        },
        Verdict::Block {
            witness_id,
            reason: BlockReason::UnwitnessedCommit,
            ..
        } => RenderedVerdict {
            stderr_line: format!(
                "anvil: unwitnessed commit refused by policy — anvil show {witness_id}"
            ),
            exit_code: 1,
        },
        Verdict::InternalError { class } => RenderedVerdict {
            stderr_line: format!(
                "anvil: {} errored (anvil doctor for details)",
                class.component()
            ),
            exit_code: 0,
        },
        Verdict::WitnessWriteFailed => RenderedVerdict {
            stderr_line: "anvil: witness write failed — refused".to_string(),
            exit_code: 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_is_silent_and_exits_zero() {
        let r = render_verdict(&Verdict::Pass);
        assert!(r.stderr_line.is_empty(), "pass MUST be silent");
        assert_eq!(r.exit_code, 0);
    }

    #[test]
    fn warn_emits_one_line_exits_zero() {
        let r = render_verdict(&Verdict::Warn {
            count: 3,
            witness_id: "abc123".to_string(),
        });
        assert_eq!(
            r.stderr_line,
            "anvil: 3 warning(s) (commit allowed) — anvil show abc123"
        );
        assert_eq!(r.exit_code, 0);
        assert!(!r.stderr_line.contains('\n'));
    }

    #[test]
    fn block_findings_emits_one_line_exits_one() {
        let r = render_verdict(&Verdict::Block {
            count: 2,
            witness_id: "def456".to_string(),
            reason: BlockReason::Findings,
        });
        assert_eq!(
            r.stderr_line,
            "anvil: 2 finding(s) (block) — anvil show def456"
        );
        assert_eq!(r.exit_code, 1);
    }

    #[test]
    fn block_unwitnessed_commit_emits_one_line_exits_one() {
        let r = render_verdict(&Verdict::Block {
            count: 0,
            witness_id: "deadbeef".to_string(),
            reason: BlockReason::UnwitnessedCommit,
        });
        assert_eq!(
            r.stderr_line,
            "anvil: unwitnessed commit refused by policy — anvil show deadbeef"
        );
        assert_eq!(r.exit_code, 1);
        assert!(!r.stderr_line.contains('\n'));
    }

    #[test]
    fn block_chain_broken_emits_one_line_exits_one() {
        let r = render_verdict(&Verdict::Block {
            count: 0,
            witness_id: "xyz789".to_string(),
            reason: BlockReason::ChainBroken,
        });
        assert_eq!(
            r.stderr_line,
            "anvil: chain integrity broken — anvil show xyz789"
        );
        assert_eq!(r.exit_code, 1);
    }

    #[test]
    fn internal_error_exits_zero_per_serena_rule() {
        // Per ADR-038 §D-6: "failure that's Anvil's fault doesn't
        // block the user." All InternalError variants must allow the
        // commit to proceed.
        for class in [
            ErrorClass::DaemonUnreachable,
            ErrorClass::EmbeddedFailed,
            ErrorClass::Panic,
            ErrorClass::TimedOut,
        ] {
            let r = render_verdict(&Verdict::InternalError { class });
            assert_eq!(
                r.exit_code, 0,
                "internal error class {class:?} must not block the user"
            );
            assert!(
                r.stderr_line.starts_with("anvil: "),
                "internal error line must be anvil-prefixed: {:?}",
                r.stderr_line
            );
            assert!(r.stderr_line.contains("errored"));
            assert!(r.stderr_line.contains("anvil doctor"));
        }
    }

    #[test]
    fn error_class_components_are_distinct_where_meaningful() {
        // daemon vs validation matters: the user looks at the
        // component name to decide which surface to investigate.
        assert_eq!(ErrorClass::DaemonUnreachable.component(), "daemon");
        assert_eq!(ErrorClass::EmbeddedFailed.component(), "validation");
        assert_eq!(ErrorClass::Panic.component(), "hook");
        assert_eq!(ErrorClass::TimedOut.component(), "validation");
    }

    #[test]
    fn witness_write_failed_refuses_commit() {
        // ADR-038: "we don't claim what we can't witness."
        let r = render_verdict(&Verdict::WitnessWriteFailed);
        assert_eq!(r.exit_code, 1);
        assert!(r.stderr_line.contains("witness write failed"));
    }

    #[test]
    fn no_verdict_produces_multiline_stderr() {
        // ADR-038: "one terse line." Multi-line is forbidden.
        let verdicts = [
            Verdict::Pass,
            Verdict::Warn {
                count: 1,
                witness_id: "x".to_string(),
            },
            Verdict::Block {
                count: 1,
                witness_id: "x".to_string(),
                reason: BlockReason::Findings,
            },
            Verdict::Block {
                count: 0,
                witness_id: "x".to_string(),
                reason: BlockReason::ChainBroken,
            },
            Verdict::InternalError {
                class: ErrorClass::DaemonUnreachable,
            },
            Verdict::WitnessWriteFailed,
        ];
        for v in &verdicts {
            let r = render_verdict(v);
            assert!(
                !r.stderr_line.contains('\n'),
                "verdict {v:?} produced multi-line output: {:?}",
                r.stderr_line
            );
        }
    }

    #[test]
    fn no_verdict_uses_loud_color_or_emoji() {
        // ADR-038 §D-1: "no colour escalation by default. Reserve
        // loud formatting for genuine block decisions." We don't
        // emit ANSI escapes at all from the library — colour is the
        // CLI wrapper's choice if it ever wants to add one. Pin
        // current behaviour: no escape characters in any rendered
        // line.
        let verdicts = [
            Verdict::Pass,
            Verdict::Warn {
                count: 1,
                witness_id: "x".to_string(),
            },
            Verdict::Block {
                count: 1,
                witness_id: "x".to_string(),
                reason: BlockReason::Findings,
            },
            Verdict::InternalError {
                class: ErrorClass::Panic,
            },
            Verdict::WitnessWriteFailed,
        ];
        for v in &verdicts {
            let r = render_verdict(v);
            assert!(
                !r.stderr_line.contains('\x1b'),
                "verdict {v:?} emitted ANSI escape: {:?}",
                r.stderr_line
            );
        }
    }
}
