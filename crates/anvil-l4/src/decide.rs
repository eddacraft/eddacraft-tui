//! Pre-push commit-level decision logic (MLP-004).
//!
//! Given the resolved [`BranchRule`] for the branch being pushed and
//! whether each commit carries an L3 witness, produce one
//! [`CommitDecision`] per commit. The pre-push CLI subcommand walks
//! the pushed range and calls this for every commit; the verdict
//! collapses to one user-facing line.
//!
//! Decision matrix (per ADR-037 §D-5):
//!
//! | `Requirement` | `has_witness` | `OnNoWitness`    | Result            |
//! | ------------- | ------------- | ---------------- | ----------------- |
//! | `L4OrL3`      | true          | —                | `Allow`           |
//! | `L4OrL3`      | false         | `Allow`          | `Allow`           |
//! | `L4OrL3`      | false         | `ValidateAtL4`   | `NeedsL4Validation` |
//! | `L4OrL3`      | false         | `Reject`         | `Block(Unwitnessed)` |
//! | `L4Only`      | —             | `Allow`          | `Allow` *(see note)* |
//! | `L4Only`      | —             | `ValidateAtL4`   | `NeedsL4Validation` |
//! | `L4Only`      | —             | `Reject`         | `Block(Unwitnessed)` |
//! | `L3Only`      | true          | —                | `Allow`           |
//! | `L3Only`      | false         | `Allow`          | `Allow`           |
//! | `L3Only`      | false         | `ValidateAtL4`   | `Block(Unwitnessed)` *(see note)* |
//! | `L3Only`      | false         | `Reject`         | `Block(Unwitnessed)` |
//!
//! ## Notes on the corner cases
//!
//! - `L4Only` ignores the L3 witness entirely: the branch contract is
//!   "every commit must pass server-side re-validation regardless of
//!   client claims." So `has_witness` doesn't change the answer; we
//!   route to `NeedsL4Validation` unless the branch explicitly opts
//!   out via `OnNoWitness::Allow`.
//! - `L3Only` refuses to accept L4 fallback: the branch contract is
//!   "every commit must come with witness evidence already." So when
//!   the commit lacks a witness, even an `OnNoWitness::ValidateAtL4`
//!   policy can't rescue it — the branch's `Requirement` is the
//!   stricter pin.

use crate::policy::{BranchRule, OnNoWitness, Requirement};

/// Per-commit verdict the pre-push subcommand emits before deciding
/// whether to allow or refuse the push as a whole.
///
/// The CLI maps `Block(_)` to `Verdict::Block { reason:
/// BlockReason::UnwitnessedCommit }`, propagates the offending
/// commit SHA into `witness_id`, and applies ADR-038 noise discipline.
/// `NeedsL4Validation` is handled by the future validate-at-l4
/// command (CLI lane); in MLP-004 v1 the CLI logs a single
/// `InternalError { class: TimedOut }` line and falls through to
/// allow (Serena rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitDecision {
    /// Witness present or policy opts out — admit the commit.
    Allow,
    /// Policy refuses this commit.
    Block(BlockKind),
    /// Policy demands server-side validation. The pre-push hook
    /// itself doesn't run the rule engine; it surfaces the demand to
    /// the caller, which decides whether to invoke `validate_at_l4`
    /// inline (future) or fall back to a noise-disciplined warning.
    NeedsL4Validation,
}

/// Why a commit was blocked at policy resolution time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// No L3 witness AND policy refuses to admit it (either via
    /// `OnNoWitness::Reject` or via `Requirement::L3Only` with no
    /// matching L3 record).
    UnwitnessedCommit,
}

impl BranchRule {
    /// Decide what to do with one commit given whether it has an L3
    /// witness.
    ///
    /// The `has_witness` argument is the answer to "is this commit's
    /// SHA recorded in `anvil/witness/active.ndjson` (or an archived
    /// segment) with a verifying chain?" — answered by the caller
    /// composing this rule with `anvil_witness::verify_chain` + a
    /// commit-SHA lookup.
    #[must_use]
    pub fn decide_commit(&self, has_witness: bool) -> CommitDecision {
        // `L4Only` ignores the L3 witness — the branch contract is
        // server-side re-validation regardless of client claims.
        if self.require == Requirement::L4Only {
            return match self.on_no_witness {
                OnNoWitness::Allow => CommitDecision::Allow,
                OnNoWitness::ValidateAtL4 => CommitDecision::NeedsL4Validation,
                OnNoWitness::Reject => CommitDecision::Block(BlockKind::UnwitnessedCommit),
            };
        }

        // For `L4OrL3` and `L3Only`, a present witness shortcuts.
        if has_witness {
            return CommitDecision::Allow;
        }

        // Missing witness — route by `on_no_witness`, except `L3Only`
        // refuses `ValidateAtL4` (the branch contract is "L3 evidence
        // present").
        match (self.require, self.on_no_witness) {
            (_, OnNoWitness::Allow) => CommitDecision::Allow,
            (Requirement::L3Only, OnNoWitness::ValidateAtL4) => {
                CommitDecision::Block(BlockKind::UnwitnessedCommit)
            }
            (_, OnNoWitness::ValidateAtL4) => CommitDecision::NeedsL4Validation,
            (_, OnNoWitness::Reject) => CommitDecision::Block(BlockKind::UnwitnessedCommit),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{OnBlock, OnWarn};

    fn rule(require: Requirement, on_no_witness: OnNoWitness) -> BranchRule {
        BranchRule {
            pattern: "test".to_string(),
            require,
            on_no_witness,
            on_block: OnBlock::Reject,
            on_warn: OnWarn::Allow,
        }
    }

    #[test]
    fn l4_or_l3_with_witness_allows() {
        let r = rule(Requirement::L4OrL3, OnNoWitness::Reject);
        assert_eq!(r.decide_commit(true), CommitDecision::Allow);
    }

    #[test]
    fn l4_or_l3_no_witness_validate_routes_to_l4() {
        let r = rule(Requirement::L4OrL3, OnNoWitness::ValidateAtL4);
        assert_eq!(r.decide_commit(false), CommitDecision::NeedsL4Validation);
    }

    #[test]
    fn l4_or_l3_no_witness_reject_blocks() {
        let r = rule(Requirement::L4OrL3, OnNoWitness::Reject);
        assert_eq!(
            r.decide_commit(false),
            CommitDecision::Block(BlockKind::UnwitnessedCommit)
        );
    }

    #[test]
    fn l4_or_l3_no_witness_allow_allows() {
        let r = rule(Requirement::L4OrL3, OnNoWitness::Allow);
        assert_eq!(r.decide_commit(false), CommitDecision::Allow);
    }

    #[test]
    fn l4_only_ignores_l3_witness_and_routes_to_validation() {
        // The whole point of L4Only is "do not trust client witness."
        let r = rule(Requirement::L4Only, OnNoWitness::ValidateAtL4);
        assert_eq!(r.decide_commit(true), CommitDecision::NeedsL4Validation);
        assert_eq!(r.decide_commit(false), CommitDecision::NeedsL4Validation);
    }

    #[test]
    fn l4_only_with_reject_blocks_regardless_of_witness() {
        let r = rule(Requirement::L4Only, OnNoWitness::Reject);
        assert_eq!(
            r.decide_commit(true),
            CommitDecision::Block(BlockKind::UnwitnessedCommit)
        );
        assert_eq!(
            r.decide_commit(false),
            CommitDecision::Block(BlockKind::UnwitnessedCommit)
        );
    }

    #[test]
    fn l4_only_with_allow_allows_regardless_of_witness() {
        let r = rule(Requirement::L4Only, OnNoWitness::Allow);
        assert_eq!(r.decide_commit(true), CommitDecision::Allow);
        assert_eq!(r.decide_commit(false), CommitDecision::Allow);
    }

    #[test]
    fn l3_only_with_witness_allows() {
        let r = rule(Requirement::L3Only, OnNoWitness::Reject);
        assert_eq!(r.decide_commit(true), CommitDecision::Allow);
    }

    #[test]
    fn l3_only_without_witness_blocks_even_with_validate_at_l4() {
        // L3Only's contract is "L3 evidence MUST exist." Falling back
        // to L4 would violate that — block instead.
        let r = rule(Requirement::L3Only, OnNoWitness::ValidateAtL4);
        assert_eq!(
            r.decide_commit(false),
            CommitDecision::Block(BlockKind::UnwitnessedCommit)
        );
    }

    #[test]
    fn l3_only_with_allow_admits_unwitnessed() {
        // Explicit allow overrides L3Only — the operator opted in.
        let r = rule(Requirement::L3Only, OnNoWitness::Allow);
        assert_eq!(r.decide_commit(false), CommitDecision::Allow);
    }

    #[test]
    fn l3_only_no_witness_reject_blocks() {
        let r = rule(Requirement::L3Only, OnNoWitness::Reject);
        assert_eq!(
            r.decide_commit(false),
            CommitDecision::Block(BlockKind::UnwitnessedCommit)
        );
    }
}
