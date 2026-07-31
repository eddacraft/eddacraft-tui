//! MLP2-016: `validate_at_l4` pipeline entry for pre-push / l4-validate.

use std::path::{Path, PathBuf};

use crate::policy::BranchRule;

/// MLP2-016: per-commit validation request the pre-push hook submits
/// to a [`ValidationEngine`]. Pure data — the engine resolves
/// everything it needs from the (`commit_sha`, `repo_root`,
/// `branch_rule`) triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationRequest {
    /// Full 40-hex commit SHA the engine must validate. The hook
    /// resolves this from `git rev-list` over the pushed range before
    /// dispatching — the engine never has to walk git itself.
    pub commit_sha: String,
    /// Branch rule that produced the `NeedsL4Validation` decision —
    /// carries the `Requirement` / `on_block` / `on_warn` knobs the
    /// engine needs to decide severity-to-verdict mapping.
    pub branch_rule: BranchRule,
    /// Repository root the hook is running in. Engines that need to
    /// shell out to git or read the commit's tree resolve paths
    /// relative to this.
    pub repo_root: PathBuf,
    /// Commit whose tree carries suppression authority: the tip of
    /// the pushed range (pre-push `local_sha`, `l4-validate` range
    /// head) or, for audit-chain rescans, the audited commit list's
    /// own tip (the `--branch` target — not the checkout's HEAD).
    /// Exceptions apply only if committed in this tree —
    /// configuration may be local, authority must be committed
    /// (ADR-100). `None` applies no exceptions (fail-safe: findings
    /// stand).
    pub exceptions_tip_sha: Option<String>,
}

/// MLP2-016: outcome of a server-side `validate_at_l4` call.
///
/// Closed set of three variants — anything not strictly Allow or
/// Block is `EngineUnavailable` so the pre-push hook never has to
/// invent a fall-through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationVerdict {
    /// Engine ran the rules and the commit passed.
    Allow,
    /// Engine ran the rules and the commit failed. `diagnostics` is
    /// the per-rule findings the operator sees in the hook's stderr
    /// line; the hook collapses them into a single
    /// `Verdict::Block { reason: UnwitnessedCommit }` plus per-rule
    /// detail lines.
    Block {
        diagnostics: Vec<ValidationDiagnostic>,
    },
    /// Engine could not execute. The hook preserves the pre-MLP2-016
    /// surface (emit `InternalError { TimedOut }` once via
    /// suppression, admit the push).
    EngineUnavailable { reason: EngineUnavailableReason },
}

/// Why a [`ValidationEngine`] declined to run. Closed set so the
/// pre-push hook can route on the reason without parsing strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineUnavailableReason {
    /// No real engine is wired in yet — the default
    /// [`NoOpValidationEngine`] returns this. Downstream follow-ups
    /// replace the no-op with a real impl that runs `anvil-checks`
    /// against the commit's tree.
    NotImplemented,
    /// The engine's required tooling is missing on the operator's
    /// machine (e.g. `regorus` runtime not installed). Reserved for
    /// the real-engine integration; the no-op never returns this.
    BinaryMissing,
    /// The engine ran past its time budget. Reserved for the
    /// real-engine integration; surfaces as
    /// `InternalError { TimedOut }` upstream.
    Timeout,
    /// The engine could not read or materialise required data because
    /// local I/O failed (temporary directory allocation, git object
    /// reads, disk-full writes, or permission errors). Distinct from
    /// missing tooling so observability does not misclassify an
    /// infrastructure outage as an install problem.
    IoError,
}

/// MLP2-016: per-rule diagnostic carried by [`ValidationVerdict::Block`].
/// The pre-push hook renders these as detail lines under the
/// `Verdict::Block` headline so operators see *which* rule refused
/// the commit, not just "Anvil said no."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationDiagnostic {
    /// Rule identifier — e.g. `secret-detection.aws-key`. Mirrors
    /// the rule ids `anvil-rules::rules_sha` hashes over.
    pub rule_id: String,
    /// Per-rule severity (`Block` / `Warn`). The branch rule's
    /// `on_warn` knob controls whether a `Warn` upgrades to a block.
    pub severity: Severity,
    /// Operator-facing message. Single-line; ≤200 chars (the hook
    /// truncates longer messages per ADR-038 noise discipline).
    pub message: String,
}

/// Rule-level severity. Mirrors the closed set used by
/// `anvil-checks::Severity` so downstream wiring is a 1:1 map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Rule blocks the commit. Upgrades to a hard refusal regardless
    /// of `on_warn`.
    Block,
    /// Rule warns. The branch rule's `on_warn` decides whether this
    /// upgrades to a block (`OnWarn::Reject`) or is admitted with
    /// the diagnostic (`OnWarn::Allow`).
    Warn,
}

/// MLP2-016: pluggable rule-engine dispatch. The pre-push hook owns
/// the trait object; tests substitute fixtures; production wires the
/// real engine once `anvil-checks` integration lands.
pub trait ValidationEngine {
    /// Validate one commit. The hook iterates over unwitnessed
    /// commits and calls this once per commit so the engine's
    /// per-commit cost is bounded.
    fn validate(&self, request: &ValidationRequest) -> ValidationVerdict;
}

/// MLP2-016 default engine: returns
/// [`ValidationVerdict::EngineUnavailable`] with reason
/// [`EngineUnavailableReason::NotImplemented`]. Pre-push uses this
/// until a real engine is wired in.
///
/// The pre-push hook's behaviour with this engine matches the
/// pre-MLP2-016 surface byte-for-byte (single
/// `InternalError { TimedOut }` emit via suppression, push admitted).
/// The point of MLP2-016 is the **typed pipeline + trait dispatch +
/// test fixtures** — the engine swap is the load-bearing follow-up.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpValidationEngine;

impl ValidationEngine for NoOpValidationEngine {
    fn validate(&self, _request: &ValidationRequest) -> ValidationVerdict {
        ValidationVerdict::EngineUnavailable {
            reason: EngineUnavailableReason::NotImplemented,
        }
    }
}

/// Convenience entry point: run a single [`ValidationRequest`]
/// through `engine`. Identical to `engine.validate(request)`; named
/// so call sites read like the prose ("validate at L4").
#[must_use]
pub fn validate_at_l4<E: ValidationEngine + ?Sized>(
    engine: &E,
    request: &ValidationRequest,
) -> ValidationVerdict {
    engine.validate(request)
}

/// Convenience entry point for a commit range. Returns one verdict
/// per commit in the input order. The hook uses this for the common
/// case of "every unwitnessed commit in the pushed range needs L4
/// validation" — the per-commit ordering is preserved so the hook
/// can correlate verdicts with commit SHAs by position.
#[must_use]
pub fn validate_range<E, I>(engine: &E, requests: I) -> Vec<ValidationVerdict>
where
    E: ValidationEngine + ?Sized,
    I: IntoIterator<Item = ValidationRequest>,
{
    requests
        .into_iter()
        .map(|req| engine.validate(&req))
        .collect()
}

/// Helper for callers that have a commit SHA + branch rule but need
/// to build a [`ValidationRequest`] without remembering the field
/// order.
pub fn request_for(
    commit_sha: impl Into<String>,
    branch_rule: BranchRule,
    repo_root: impl AsRef<Path>,
    exceptions_tip_sha: Option<String>,
) -> ValidationRequest {
    ValidationRequest {
        commit_sha: commit_sha.into(),
        branch_rule,
        repo_root: repo_root.as_ref().to_path_buf(),
        exceptions_tip_sha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{OnBlock, OnNoWitness, OnWarn, Requirement};

    fn rule() -> BranchRule {
        BranchRule {
            pattern: "main".to_string(),
            require: Requirement::L4OrL3,
            on_no_witness: OnNoWitness::ValidateAtL4,
            on_block: OnBlock::Reject,
            on_warn: OnWarn::Allow,
        }
    }

    fn request(sha: &str) -> ValidationRequest {
        request_for(sha, rule(), Path::new("/tmp/test-repo"), None)
    }

    /// Default no-op engine returns `EngineUnavailable { NotImplemented }`.
    /// The pre-push hook treats this as the pre-MLP2-016 fall-through.
    #[test]
    fn noop_engine_reports_not_implemented() {
        let engine = NoOpValidationEngine;
        let req = request("a".repeat(40).as_str());
        let verdict = validate_at_l4(&engine, &req);
        assert_eq!(
            verdict,
            ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::NotImplemented,
            }
        );
    }

    /// A fixture engine that always allows produces the `Allow`
    /// verdict — pins the wire shape so the hook's "no emit on
    /// allow" path is testable.
    #[test]
    fn allowing_engine_returns_allow() {
        struct AllowingEngine;
        impl ValidationEngine for AllowingEngine {
            fn validate(&self, _request: &ValidationRequest) -> ValidationVerdict {
                ValidationVerdict::Allow
            }
        }
        let verdict = validate_at_l4(&AllowingEngine, &request("b".repeat(40).as_str()));
        assert_eq!(verdict, ValidationVerdict::Allow);
    }

    /// A fixture engine that blocks produces `Block` with the
    /// diagnostics intact — pins the wire shape so the hook's
    /// per-rule detail rendering is testable.
    #[test]
    fn blocking_engine_returns_block_with_diagnostics() {
        struct BlockingEngine;
        impl ValidationEngine for BlockingEngine {
            fn validate(&self, _request: &ValidationRequest) -> ValidationVerdict {
                ValidationVerdict::Block {
                    diagnostics: vec![ValidationDiagnostic {
                        rule_id: "secret-detection.aws-key".to_string(),
                        severity: Severity::Block,
                        message: "AWS access key leaked".to_string(),
                    }],
                }
            }
        }
        let verdict = validate_at_l4(&BlockingEngine, &request("c".repeat(40).as_str()));
        let ValidationVerdict::Block { diagnostics } = verdict else {
            panic!("expected Block, got {verdict:?}");
        };
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "secret-detection.aws-key");
        assert_eq!(diagnostics[0].severity, Severity::Block);
    }

    /// `validate_range` preserves per-commit ordering so the hook can
    /// correlate verdicts with commit SHAs by position. Pin against a
    /// future "sort by severity" refactor that would silently break
    /// position-based correlation.
    #[test]
    fn validate_range_preserves_input_order() {
        struct PerShaEngine;
        impl ValidationEngine for PerShaEngine {
            fn validate(&self, request: &ValidationRequest) -> ValidationVerdict {
                if request.commit_sha.starts_with('a') {
                    ValidationVerdict::Allow
                } else {
                    ValidationVerdict::Block {
                        diagnostics: vec![],
                    }
                }
            }
        }
        let reqs = vec![
            request(&"a".repeat(40)),
            request(&"b".repeat(40)),
            request(&"a".repeat(40)),
        ];
        let verdicts = validate_range(&PerShaEngine, reqs);
        assert_eq!(verdicts.len(), 3);
        assert_eq!(verdicts[0], ValidationVerdict::Allow);
        assert!(matches!(verdicts[1], ValidationVerdict::Block { .. }));
        assert_eq!(verdicts[2], ValidationVerdict::Allow);
    }

    /// `request_for` builds a `ValidationRequest` without forcing
    /// callers to remember positional fields — pinned so a future
    /// field reorder cannot break call sites silently.
    #[test]
    fn request_for_carries_caller_inputs_unchanged() {
        let req = request_for("deadbeef", rule(), Path::new("/work/repo"), None);
        assert_eq!(req.commit_sha, "deadbeef");
        assert_eq!(req.branch_rule.pattern, "main");
        assert_eq!(req.repo_root, Path::new("/work/repo"));
    }

    /// `EngineUnavailableReason` variants are pairwise distinct so the
    /// hook can match on a specific reason rather than a stringly-
    /// compared blob. Pin the closed set against an accidental
    /// duplicate.
    #[test]
    fn engine_unavailable_reasons_are_distinct() {
        assert_ne!(
            EngineUnavailableReason::NotImplemented,
            EngineUnavailableReason::BinaryMissing,
        );
        assert_ne!(
            EngineUnavailableReason::BinaryMissing,
            EngineUnavailableReason::Timeout,
        );
        assert_ne!(
            EngineUnavailableReason::NotImplemented,
            EngineUnavailableReason::Timeout,
        );
        assert_ne!(
            EngineUnavailableReason::IoError,
            EngineUnavailableReason::BinaryMissing,
        );
    }
}
