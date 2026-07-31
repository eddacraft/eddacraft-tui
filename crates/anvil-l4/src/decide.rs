//! Map L4 policy resolution and validation into commit admit/block decisions.

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

/// MLP2-018: outcome of comparing a witness's recognised
/// `anvil_version` against the policy's `required_anvil_version`
/// floor.
///
/// The L4 server-side validation lane composes this with
/// [`CommitDecision`] (witness presence) and
/// [`crate::RulesShaOutcome`] (witness recognition) to produce the
/// final per-commit verdict. A parallel decision type — rather than a
/// new [`BlockKind`] variant — keeps `decide_commit`'s contract
/// stable for the pre-push hook (which only needs the witness-presence
/// answer) while letting the L4 caller pattern-match on the richer
/// floor diagnostic data (`required` + `observed` semver strings).
///
/// Mirrors `anvil-cli::commands::hook::VersionFloorOutcome` so the
/// hook-side (MLP2-020) and server-side (MLP2-018) check have parallel
/// names; the server-side variant carries `required` + `observed`
/// strings because the diagnostic line ADR-038 demands ("required
/// X.Y.Z, observed A.B.C") needs both.
///
/// ## Routing semantics
///
/// | Floor pinned? | Witness version present? | Floor parse | Witness parse | Compare      | Outcome              |
/// | ------------- | ------------------------ | ----------- | ------------- | ------------ | -------------------- |
/// | no            | —                        | —           | —             | —            | `Satisfied`          |
/// | yes           | no                       | ok          | —             | —            | `WitnessVersionAbsent` |
/// | yes           | yes                      | error       | —             | —            | `InvalidFloor`       |
/// | yes           | yes                      | ok          | error         | —            | `InvalidWitnessVersion` |
/// | yes           | yes                      | ok          | ok            | observed≥floor | `Satisfied`        |
/// | yes           | yes                      | ok          | ok            | observed<floor | `BelowFloor`       |
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionFloorOutcome {
    /// Either no floor was pinned, OR a parsed witness version is at
    /// or above the parsed floor.
    Satisfied,
    /// Floor pinned but witness carries no `anvil_version` to compare.
    /// The L4 caller chooses whether this is a hard block (strict
    /// branches), a degraded-signal admit, or a route to revalidation.
    /// Distinct from `Satisfied` so the routing is explicit.
    WitnessVersionAbsent,
    /// Parsed witness version is strictly less than the parsed floor.
    /// `required` and `observed` carry the operator-facing strings
    /// for the diagnostic line.
    BelowFloor { required: String, observed: String },
    /// The `required_anvil_version` in policy is not valid semver.
    /// The operator's remediation is fixing the policy file; the L4
    /// caller surfaces this as a policy error rather than a
    /// commit-level block.
    InvalidFloor { raw: String },
    /// The witness's `anvil_version` is not valid semver. Degraded
    /// witness data; the L4 caller decides whether to admit (legacy
    /// witness pre-`anvil_version` field) or block.
    InvalidWitnessVersion { raw: String },
}

/// MLP2-018: server-side `required_anvil_version` floor check.
///
/// Daemon-side mirror of MLP2-020's hook-side `check_version_floor`.
/// The hook runs the floor check against the running binary's version;
/// the server-side runs it against each commit's witness-claimed
/// `anvil_version` (e.g. recovered from
/// [`crate::RecognisedRulesRegistry`] via the witness's `rules_sha`).
///
/// `policy_floor` is `policy.required_anvil_version.as_deref()` —
/// `None` means no floor pinned, which short-circuits to
/// `Satisfied`. `witness_anvil_version` is `None` when the witness
/// carries no version claim.
///
/// Semver parsing uses `semver::Version` directly so the parsing
/// shape matches `anvil_rules::RequiredAnvilVersion::parse` byte for
/// byte. Both versions are parsed strictly — a leading `v`, a `>=`
/// constraint operator, or any other non-version syntax routes to
/// `InvalidFloor` / `InvalidWitnessVersion`.
#[must_use]
pub fn evaluate_version_floor(
    policy_floor: Option<&str>,
    witness_anvil_version: Option<&str>,
) -> VersionFloorOutcome {
    let Some(floor_raw) = policy_floor else {
        return VersionFloorOutcome::Satisfied;
    };
    let Ok(floor) = semver::Version::parse(floor_raw) else {
        return VersionFloorOutcome::InvalidFloor {
            raw: floor_raw.to_string(),
        };
    };
    let Some(observed_raw) = witness_anvil_version else {
        return VersionFloorOutcome::WitnessVersionAbsent;
    };
    let Ok(observed) = semver::Version::parse(observed_raw) else {
        return VersionFloorOutcome::InvalidWitnessVersion {
            raw: observed_raw.to_string(),
        };
    };
    if observed >= floor {
        VersionFloorOutcome::Satisfied
    } else {
        VersionFloorOutcome::BelowFloor {
            required: floor_raw.to_string(),
            observed: observed_raw.to_string(),
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

    #[test]
    fn version_floor_no_floor_short_circuits_to_satisfied() {
        // No `required_anvil_version` pinned in policy → check is a
        // no-op regardless of the witness's claim.
        let outcome = evaluate_version_floor(None, Some("0.6.0"));
        assert_eq!(outcome, VersionFloorOutcome::Satisfied);
        let outcome = evaluate_version_floor(None, None);
        assert_eq!(outcome, VersionFloorOutcome::Satisfied);
    }

    #[test]
    fn version_floor_observed_above_floor_satisfied() {
        // Boundary plus a strict-above sample.
        let outcome = evaluate_version_floor(Some("0.7.0"), Some("0.7.0"));
        assert_eq!(outcome, VersionFloorOutcome::Satisfied);
        let outcome = evaluate_version_floor(Some("0.7.0"), Some("0.7.1"));
        assert_eq!(outcome, VersionFloorOutcome::Satisfied);
        let outcome = evaluate_version_floor(Some("0.7.0"), Some("1.0.0"));
        assert_eq!(outcome, VersionFloorOutcome::Satisfied);
    }

    #[test]
    fn version_floor_observed_below_floor_blocks() {
        // The diagnostic strings come from the raw input — preserves
        // the operator-typed values so the message is byte-faithful
        // to what they put in the policy.
        let outcome = evaluate_version_floor(Some("0.7.0"), Some("0.6.9"));
        match outcome {
            VersionFloorOutcome::BelowFloor { required, observed } => {
                assert_eq!(required, "0.7.0");
                assert_eq!(observed, "0.6.9");
            }
            other => panic!("expected BelowFloor, got {other:?}"),
        }
    }

    #[test]
    fn version_floor_prerelease_precedence_matches_semver() {
        // 0.7.0-beta < 0.7.0 per standard semver — pin so a future
        // dep upgrade can't silently re-interpret prerelease ordering.
        let outcome = evaluate_version_floor(Some("0.7.0"), Some("0.7.0-beta"));
        match outcome {
            VersionFloorOutcome::BelowFloor { required, observed } => {
                assert_eq!(required, "0.7.0");
                assert_eq!(observed, "0.7.0-beta");
            }
            other => panic!("expected BelowFloor, got {other:?}"),
        }
        // Reverse: a prerelease floor admits the release.
        let outcome = evaluate_version_floor(Some("0.7.0-beta"), Some("0.7.0"));
        assert_eq!(outcome, VersionFloorOutcome::Satisfied);
        let outcome = evaluate_version_floor(Some("0.7.0-beta"), Some("0.7.0-beta"));
        assert_eq!(outcome, VersionFloorOutcome::Satisfied);
    }

    #[test]
    fn version_floor_witness_absent_routes_distinctly() {
        // Floor pinned but witness has no anvil_version. The caller
        // sees `WitnessVersionAbsent` (not `Satisfied`) so a strict
        // branch can choose to block.
        let outcome = evaluate_version_floor(Some("0.7.0"), None);
        assert_eq!(outcome, VersionFloorOutcome::WitnessVersionAbsent);
    }

    #[test]
    fn version_floor_invalid_floor_routes_to_invalid_floor() {
        // `v0.7` is not valid semver — the leading `v` is rejected.
        let outcome = evaluate_version_floor(Some("v0.7"), Some("0.7.0"));
        match outcome {
            VersionFloorOutcome::InvalidFloor { raw } => assert_eq!(raw, "v0.7"),
            other => panic!("expected InvalidFloor, got {other:?}"),
        }
        // Constraint operators (>=) are also rejected — the policy
        // pin is a literal version, not a range.
        let outcome = evaluate_version_floor(Some(">=0.7.0"), Some("0.7.0"));
        assert!(matches!(outcome, VersionFloorOutcome::InvalidFloor { .. }));
    }

    #[test]
    fn version_floor_invalid_witness_version_routes_distinctly() {
        // Witness value is malformed — surface separately so the
        // caller distinguishes "operator policy bug" (InvalidFloor)
        // from "degraded upstream witness data" (InvalidWitnessVersion).
        let outcome = evaluate_version_floor(Some("0.7.0"), Some("not-a-version"));
        match outcome {
            VersionFloorOutcome::InvalidWitnessVersion { raw } => {
                assert_eq!(raw, "not-a-version");
            }
            other => panic!("expected InvalidWitnessVersion, got {other:?}"),
        }
    }

    #[test]
    fn version_floor_invalid_floor_takes_precedence_over_invalid_witness() {
        // Both unparseable: caller cares about the policy bug first
        // (operator can fix it); witness invalidity is downstream.
        let outcome = evaluate_version_floor(Some("v0.7"), Some("not-a-version"));
        assert!(matches!(outcome, VersionFloorOutcome::InvalidFloor { .. }));
    }

    #[test]
    fn version_floor_build_metadata_is_ignored_per_semver_spec() {
        // semver spec §10: build metadata is ignored when determining
        // precedence. A CI-stamped witness like `1.0.0+ci.42` must
        // still satisfy a floor of `1.0.0`. Pin against an accidental
        // dep upgrade that surfaces build metadata into ordering.
        let outcome = evaluate_version_floor(Some("1.0.0"), Some("1.0.0+ci.42"));
        assert_eq!(outcome, VersionFloorOutcome::Satisfied);
        let outcome = evaluate_version_floor(Some("0.7.0"), Some("0.7.0+build.5"));
        assert_eq!(outcome, VersionFloorOutcome::Satisfied);
        // A below-floor with build metadata still blocks.
        let outcome = evaluate_version_floor(Some("1.0.0"), Some("0.9.9+ci.42"));
        match outcome {
            VersionFloorOutcome::BelowFloor { required, observed } => {
                assert_eq!(required, "1.0.0");
                assert_eq!(observed, "0.9.9+ci.42");
            }
            other => panic!("expected BelowFloor, got {other:?}"),
        }
    }
}
