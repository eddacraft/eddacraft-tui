//! Tier-0 claim-versus-effect evaluation.
//!
//! The v1 claim table is deliberately closed. Git proves explicit path,
//! documentation-only, and test-only claims. Graph-semantic claims remain
//! unsupported until a predicate-specific GV2 adapter can evaluate an actual,
//! revision-bound delta; binding metadata alone is never treated as success.

use std::collections::{BTreeMap, BTreeSet};

use anvil_graph_cache::GraphDelta;
use anvil_kernel_types::conformance::RawGitChangeRecord;
use anvil_kernel_types::diagnostics::KnownMode;
use anvil_kernel_types::{
    CONFORMANCE_SCHEMA_VERSION, Category, ClaimKind, ConformanceClaim, ConformanceContract,
    ConformanceOutcome, ConformanceVerdict, CoverageMember, DeclaredScope, Diagnostic,
    DiagnosticSource, EvaluationBinding, EvidenceDisposition, EvidenceGrade, EvidenceStrength,
    GitChangeStatus, GraphEvidenceBinding, IntentSource, IntentSourceKind, IntentTier, Location,
    Mode, ScopeAuthority, Severity,
};

use super::git::{
    ConventionalCommitEvidence, GitBudgetDiagnostics, GitCoverageMember, GitEvaluationIdentity,
    GitFootprintCommitEvidence, GitFootprintCommitExtraction, GitFootprintExtraction,
};
use super::pr_body::PrBodyContractParts;

/// Version of the closed Tier-0 claim table implemented here.
pub const CONFORMANCE_CLAIM_TABLE_VERSION: u32 = 1;

/// An actual GV2 delta plus every join-time identity absent from `GraphDelta`.
///
/// CONF-004 accepts this shape to preserve the future graph-evaluation seam,
/// but v1 defines no graph-semantic claim predicate. A matching binding is
/// therefore still unsupported rather than conformant.
#[derive(Debug)]
pub struct BoundGraphDelta<'a> {
    pub repository_id: &'a str,
    pub canonical_worktree_id: &'a str,
    pub binding: &'a GraphEvidenceBinding,
    pub delta: &'a GraphDelta,
}

/// Per-claim result retained so mixed evidence cannot collapse into one bit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimEvaluation {
    pub claim: ConformanceClaim,
    pub outcome: ConformanceOutcome,
    pub evidence_strength: EvidenceStrength,
    pub reasons: Vec<String>,
    pub coverage: Vec<CoverageMember>,
    pub uncovered_files: Vec<String>,
}

/// Reusable advisory result. Callers own process-exit and enforcement policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceEvaluation {
    pub verdict: ConformanceVerdict,
    pub claim_results: Vec<ClaimEvaluation>,
    pub findings: Vec<Diagnostic>,
    pub git_non_evaluations: Vec<GitCommitNonEvaluation>,
}

/// Safe per-commit audit record for a footprint that could not be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitNonEvaluation {
    pub commit_revision: Option<Box<str>>,
    pub reason: &'static str,
    pub stage: &'static str,
    pub observed: usize,
    pub limit: Option<usize>,
    pub raw_digest: Option<Box<str>>,
    pub budget: Option<Box<GitBudgetDiagnostics>>,
}

/// Evaluate one complete Git range against an extracted pull-request declaration.
#[must_use]
pub fn evaluate_pr_declaration(
    declaration: &PrBodyContractParts,
    extraction: &GitFootprintExtraction,
    identity: &GitEvaluationIdentity,
) -> ConformanceEvaluation {
    let contract = ConformanceContract {
        schema_version: CONFORMANCE_SCHEMA_VERSION.to_owned(),
        binding: EvaluationBinding {
            run_id: identity.run_id.clone(),
            repository_id: identity.repository_id.clone(),
            canonical_worktree_id: identity.canonical_worktree_id.clone(),
            base_revision: extraction.base_revision.clone(),
            head_revision: extraction.head_revision.clone(),
            commit_revision: extraction.head_revision.clone(),
        },
        sources: vec![declaration.source.clone()],
        declared_scopes: declaration.declared_scopes.clone(),
        claimed_changes: declaration.claims.clone(),
        acceptance_assertions: Vec::new(),
        graph_evidence: Vec::new(),
    };
    if let Err(error) = contract.validate() {
        return invalid_evaluation(&contract, format!("contract.{}", error.code()));
    }
    let git_non_evaluations = pr_git_footprint_non_evaluations(extraction, identity);
    let failures = pr_git_footprint_failures(extraction, identity);
    if !failures.is_empty() {
        return invalid_evaluation_reasons(&contract, failures, git_non_evaluations);
    }

    let mut coverage = Vec::new();
    for commit in &extraction.commits {
        if let GitFootprintCommitExtraction::Evaluated(commit) = commit {
            coverage.extend(commit.coverage.iter().cloned());
        }
    }

    evaluate_bound_contract(&contract, &coverage, &BTreeSet::new(), &[])
}

/// Retain every per-commit extraction failure without unsafe diagnostic detail.
#[must_use]
pub fn pr_git_footprint_non_evaluations(
    extraction: &GitFootprintExtraction,
    identity: &GitEvaluationIdentity,
) -> Vec<GitCommitNonEvaluation> {
    let mut failures: Vec<_> = extraction
        .commits
        .iter()
        .filter_map(|result| match result {
            GitFootprintCommitExtraction::Evaluated(commit) => {
                footprint_commit_binding_failures(commit, extraction, identity)
                    .next()
                    .map(|reason| GitCommitNonEvaluation {
                        commit_revision: Some(commit.commit_revision.clone().into_boxed_str()),
                        reason,
                        stage: "binding",
                        observed: 0,
                        limit: None,
                        raw_digest: None,
                        budget: None,
                    })
            }
            GitFootprintCommitExtraction::NotEvaluated(failure) => Some(GitCommitNonEvaluation {
                commit_revision: failure.commit_revision.clone(),
                reason: failure.reason,
                stage: failure.stage,
                observed: failure.observed,
                limit: failure.limit,
                raw_digest: failure.raw_digest.clone(),
                budget: failure.budget.clone(),
            }),
        })
        .collect();
    failures.sort_by(|left, right| left.commit_revision.cmp(&right.commit_revision));
    failures
}

/// Return every deterministic reason a PR footprint cannot be trusted.
#[must_use]
pub fn pr_git_footprint_failures(
    extraction: &GitFootprintExtraction,
    identity: &GitEvaluationIdentity,
) -> Vec<String> {
    let mut failures = BTreeSet::new();
    if extraction.commits.is_empty() {
        failures.insert("selection.range-empty".to_owned());
    }

    for result in &extraction.commits {
        let commit = match result {
            GitFootprintCommitExtraction::Evaluated(commit) => commit,
            GitFootprintCommitExtraction::NotEvaluated(failure) => {
                failures.insert(format!("git.commit-not-evaluated.{}", failure.reason));
                continue;
            }
        };
        for reason in footprint_commit_binding_failures(commit, extraction, identity) {
            failures.insert(reason.to_owned());
        }
    }

    failures.into_iter().collect()
}

fn footprint_commit_binding_failures(
    commit: &GitFootprintCommitEvidence,
    extraction: &GitFootprintExtraction,
    identity: &GitEvaluationIdentity,
) -> impl Iterator<Item = &'static str> {
    [
        (commit.binding.run_id != identity.run_id).then_some("binding.run-id-mismatch"),
        (commit.binding.repository_id != identity.repository_id)
            .then_some("binding.repository-mismatch"),
        (commit.binding.canonical_worktree_id != identity.canonical_worktree_id)
            .then_some("binding.worktree-mismatch"),
        (commit.binding.base_revision != extraction.base_revision)
            .then_some("binding.base-revision-mismatch"),
        (commit.binding.head_revision != extraction.head_revision)
            .then_some("binding.head-revision-mismatch"),
        (commit.binding.commit_revision != commit.commit_revision)
            .then_some("binding.commit-revision-mismatch"),
    ]
    .into_iter()
    .flatten()
}

/// Evaluate one CONF-003 extracted commit against its canonical contract.
#[must_use]
pub fn evaluate_tier0(
    contract: &ConformanceContract,
    commit: &ConventionalCommitEvidence,
    graph: &[BoundGraphDelta<'_>],
) -> ConformanceEvaluation {
    if let Err(error) = contract.validate() {
        return invalid_evaluation(contract, format!("contract.{}", error.code()));
    }
    if !contract_matches_commit(contract, commit) {
        return invalid_evaluation(contract, "binding.extracted-commit-mismatch".to_owned());
    }

    let policy_sources = contributing_policy_sources(
        &contract.declared_scopes,
        &commit.contributing_base_config_paths,
    );
    evaluate_bound_contract(contract, &commit.coverage, &policy_sources, graph)
}

fn evaluate_bound_contract(
    contract: &ConformanceContract,
    coverage: &[GitCoverageMember],
    policy_sources: &BTreeSet<String>,
    graph: &[BoundGraphDelta<'_>],
) -> ConformanceEvaluation {
    let mut claims = contract.claimed_changes.clone();
    claims.sort_by(claim_order);
    let mut raw_coverage = coverage.to_vec();
    raw_coverage.sort_by(coverage_order);

    let mut claim_results: Vec<_> = claims
        .into_iter()
        .map(|claim| evaluate_claim(contract, &claim, &raw_coverage, policy_sources, graph))
        .collect();
    let (sources, source_index_map) = canonical_sources(contract);
    for result in &mut claim_results {
        remap_claim_source(&mut result.claim, &source_index_map);
    }
    claim_results.sort_by(|left, right| claim_order(&left.claim, &right.claim));

    let mut reasons: Vec<String> = claim_results
        .iter()
        .flat_map(|result| result.reasons.iter().cloned())
        .collect();
    let mut uncovered_files: Vec<String> = claim_results
        .iter()
        .flat_map(|result| result.uncovered_files.iter().cloned())
        .collect();
    sort_deduplicate(&mut reasons);
    sort_deduplicate(&mut uncovered_files);

    let outcome = if claim_results
        .iter()
        .any(|result| result.outcome == ConformanceOutcome::NonConformant)
    {
        ConformanceOutcome::NonConformant
    } else if !claim_results.is_empty()
        && claim_results
            .iter()
            .all(|result| result.outcome == ConformanceOutcome::Conformant)
    {
        ConformanceOutcome::Conformant
    } else {
        ConformanceOutcome::NotEvaluated
    };

    let evidence_strength = aggregate_strength(&claim_results);
    if claim_results.is_empty() {
        reasons.push("claim.none-applicable".to_owned());
    }
    sort_deduplicate(&mut reasons);

    let coverage = aggregate_coverage(&raw_coverage, &claim_results, policy_sources);
    let verdict = ConformanceVerdict {
        claim_table_version: CONFORMANCE_CLAIM_TABLE_VERSION,
        binding: contract.binding.clone(),
        sources,
        evaluated_claims: claim_results
            .iter()
            .map(|result| result.claim.clone())
            .collect(),
        declared_scopes: canonical_scopes(contract, &source_index_map),
        outcome,
        evidence_strength,
        reasons: reasons.clone(),
        git_records: canonical_git_records(&raw_coverage),
        coverage,
        uncovered_files: uncovered_files.clone(),
    };
    let findings = advisory_findings(&verdict);

    ConformanceEvaluation {
        verdict,
        claim_results,
        findings,
        git_non_evaluations: Vec::new(),
    }
}

fn evaluate_claim(
    contract: &ConformanceContract,
    claim: &ConformanceClaim,
    raw_coverage: &[GitCoverageMember],
    policy_sources: &BTreeSet<String>,
    graph: &[BoundGraphDelta<'_>],
) -> ClaimEvaluation {
    match claim.kind {
        ClaimKind::FileClass => {
            if !matches!(claim.value.as_str(), "documentation-only" | "test-only") {
                return unsupported_claim(
                    claim,
                    raw_coverage,
                    policy_sources,
                    "claim.file-class.unsupported",
                );
            }
            evaluate_git_claim(
                claim,
                raw_coverage,
                policy_sources,
                "claim.file-class.outside-class",
                &if claim.value == "documentation-only" {
                    GitClaimRule::Documentation
                } else {
                    GitClaimRule::Test
                },
            )
        }
        ClaimKind::PathPrefix => {
            let mut prefixes = Vec::new();
            for scope in &contract.declared_scopes {
                if scope.source_index != claim.source_index {
                    continue;
                }
                match &scope.authority {
                    ScopeAuthority::ExplicitPath { prefix } if prefix == &claim.value => {
                        prefixes.push(prefix.as_str());
                    }
                    ScopeAuthority::BaseMapping {
                        prefixes: mapped, ..
                    } if mapped.iter().any(|prefix| prefix == &claim.value) => {
                        prefixes.extend(mapped.iter().map(String::as_str));
                    }
                    ScopeAuthority::None
                    | ScopeAuthority::ExplicitPath { .. }
                    | ScopeAuthority::BaseMapping { .. } => {}
                }
            }
            prefixes.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
            prefixes.dedup();
            if prefixes.is_empty() {
                return unsupported_claim(
                    claim,
                    raw_coverage,
                    policy_sources,
                    "claim.path.no-authority",
                );
            }
            evaluate_git_claim(
                claim,
                raw_coverage,
                policy_sources,
                "claim.path.outside-scope",
                &GitClaimRule::Prefixes(prefixes),
            )
        }
        ClaimKind::GraphSemantic => {
            evaluate_graph_claim(contract, claim, raw_coverage, policy_sources, graph)
        }
    }
}

enum GitClaimRule<'a> {
    Documentation,
    Test,
    Prefixes(Vec<&'a str>),
}

fn evaluate_git_claim(
    claim: &ConformanceClaim,
    raw_coverage: &[GitCoverageMember],
    policy_sources: &BTreeSet<String>,
    violation_reason: &str,
    rule: &GitClaimRule<'_>,
) -> ClaimEvaluation {
    let coverage = coverage_members(raw_coverage, policy_sources, |_| {
        EvidenceDisposition::GitSufficient
    });

    let mut uncovered_files = Vec::new();
    for path in canonical_coverage_paths(raw_coverage).keys() {
        let endpoint_conforms = match rule {
            GitClaimRule::Documentation => is_documentation_path(path),
            GitClaimRule::Test => is_test_path(path),
            GitClaimRule::Prefixes(prefixes) => prefixes
                .iter()
                .any(|prefix| path_matches_prefix(path, prefix)),
        };
        if !endpoint_conforms {
            uncovered_files.push(path.clone());
        }
    }
    let violated = !uncovered_files.is_empty();
    ClaimEvaluation {
        claim: claim.clone(),
        outcome: if violated {
            ConformanceOutcome::NonConformant
        } else {
            ConformanceOutcome::Conformant
        },
        evidence_strength: EvidenceStrength::Complete,
        reasons: if violated {
            vec![violation_reason.to_owned()]
        } else {
            Vec::new()
        },
        coverage,
        uncovered_files,
    }
}

fn evaluate_graph_claim(
    contract: &ConformanceContract,
    claim: &ConformanceClaim,
    raw_coverage: &[GitCoverageMember],
    policy_sources: &BTreeSet<String>,
    graph: &[BoundGraphDelta<'_>],
) -> ClaimEvaluation {
    let mut reasons = Vec::new();
    let coverage = canonical_coverage_paths(raw_coverage)
        .into_iter()
        .map(|(path, record_indices)| {
            let (disposition, reason) =
                graph_path_disposition(contract, &path, &record_indices, raw_coverage, graph);
            reasons.push(reason.to_owned());
            CoverageMember {
                policy_change: policy_sources.contains(&path),
                path,
                record_indices,
                disposition,
            }
        })
        .collect();
    if reasons.is_empty() {
        reasons.push("claim.graph-semantic.unsupported".to_owned());
    }
    sort_deduplicate(&mut reasons);

    ClaimEvaluation {
        claim: claim.clone(),
        outcome: ConformanceOutcome::NotEvaluated,
        evidence_strength: EvidenceStrength::Absent,
        reasons,
        coverage,
        uncovered_files: Vec::new(),
    }
}

fn graph_path_disposition(
    contract: &ConformanceContract,
    path: &str,
    record_indices: &[u32],
    raw_coverage: &[GitCoverageMember],
    graph: &[BoundGraphDelta<'_>],
) -> (EvidenceDisposition, &'static str) {
    let expected: Vec<_> = contract
        .graph_evidence
        .iter()
        .filter(|binding| binding.path == path)
        .collect();
    let [expected] = expected.as_slice() else {
        return if expected.is_empty() {
            (EvidenceDisposition::Missing, "binding.graph-missing")
        } else {
            (EvidenceDisposition::Mismatched, "binding.graph-mismatched")
        };
    };

    let Some(evaluated_object) = evaluated_object_for_path(path, record_indices, raw_coverage)
    else {
        return (EvidenceDisposition::Mismatched, "binding.graph-mismatched");
    };
    if expected.blob != evaluated_object {
        return (EvidenceDisposition::Stale, "binding.graph-stale");
    }

    let candidates: Vec<_> = graph
        .iter()
        .filter(|record| record.binding.path == path || record.delta.file == path)
        .collect();
    let [record] = candidates.as_slice() else {
        return if candidates.is_empty() {
            (EvidenceDisposition::Missing, "binding.graph-missing")
        } else {
            (EvidenceDisposition::Mismatched, "binding.graph-mismatched")
        };
    };

    if record.repository_id != contract.binding.repository_id
        || record.canonical_worktree_id != contract.binding.canonical_worktree_id
        || record.binding.run_id != contract.binding.run_id
        || record.binding.path != path
        || record.delta.file != path
        || record.binding.schema_version != record.delta.schema_version
    {
        return (EvidenceDisposition::Mismatched, "binding.graph-mismatched");
    }
    if record.binding.revision != expected.revision
        || record.binding.blob != expected.blob
        || record.binding.generation != expected.generation
    {
        return (EvidenceDisposition::Stale, "binding.graph-stale");
    }
    if record.binding != *expected {
        return (EvidenceDisposition::Mismatched, "binding.graph-mismatched");
    }
    if !record.delta.errors.is_empty() {
        return (EvidenceDisposition::Unsupported, "binding.graph-incomplete");
    }

    (
        EvidenceDisposition::Unsupported,
        "claim.graph-semantic.unsupported",
    )
}

fn unsupported_claim(
    claim: &ConformanceClaim,
    raw_coverage: &[GitCoverageMember],
    policy_sources: &BTreeSet<String>,
    reason: &str,
) -> ClaimEvaluation {
    ClaimEvaluation {
        claim: claim.clone(),
        outcome: ConformanceOutcome::NotEvaluated,
        evidence_strength: EvidenceStrength::Absent,
        reasons: vec![reason.to_owned()],
        coverage: coverage_members(raw_coverage, policy_sources, |_| {
            EvidenceDisposition::Unsupported
        }),
        uncovered_files: Vec::new(),
    }
}

fn aggregate_strength(results: &[ClaimEvaluation]) -> EvidenceStrength {
    if results.is_empty()
        || results
            .iter()
            .all(|result| result.evidence_strength == EvidenceStrength::Absent)
    {
        EvidenceStrength::Absent
    } else if results
        .iter()
        .all(|result| result.evidence_strength == EvidenceStrength::Complete)
    {
        EvidenceStrength::Complete
    } else {
        EvidenceStrength::Partial
    }
}

fn aggregate_coverage(
    raw_coverage: &[GitCoverageMember],
    results: &[ClaimEvaluation],
    policy_sources: &BTreeSet<String>,
) -> Vec<CoverageMember> {
    coverage_members(raw_coverage, policy_sources, |path| {
        let dispositions: Vec<_> = results
            .iter()
            .flat_map(|result| result.coverage.iter())
            .filter(|coverage| coverage.path == path)
            .map(|coverage| coverage.disposition)
            .collect();
        if dispositions.contains(&EvidenceDisposition::Stale) {
            EvidenceDisposition::Stale
        } else if dispositions.contains(&EvidenceDisposition::Mismatched) {
            EvidenceDisposition::Mismatched
        } else if dispositions.contains(&EvidenceDisposition::Missing) {
            EvidenceDisposition::Missing
        } else if dispositions.is_empty()
            || dispositions.contains(&EvidenceDisposition::Unsupported)
        {
            EvidenceDisposition::Unsupported
        } else if dispositions.contains(&EvidenceDisposition::Gv2Bound) {
            EvidenceDisposition::Gv2Bound
        } else {
            EvidenceDisposition::GitSufficient
        }
    })
}

fn coverage_members(
    raw_coverage: &[GitCoverageMember],
    policy_sources: &BTreeSet<String>,
    disposition_for_path: impl Fn(&str) -> EvidenceDisposition,
) -> Vec<CoverageMember> {
    canonical_coverage_paths(raw_coverage)
        .into_iter()
        .map(|(path, record_indices)| {
            let policy_change = policy_sources.contains(&path);
            let mut disposition = disposition_for_path(&path);
            if policy_change && disposition == EvidenceDisposition::GitSufficient {
                disposition = EvidenceDisposition::PolicyChange;
            }
            CoverageMember {
                path,
                record_indices,
                policy_change,
                disposition,
            }
        })
        .collect()
}

fn canonical_coverage_paths(raw_coverage: &[GitCoverageMember]) -> BTreeMap<String, Vec<u32>> {
    let mut paths = BTreeMap::<String, Vec<u32>>::new();
    for (index, member) in raw_coverage.iter().enumerate() {
        let index = u32::try_from(index).expect("CONF-003 bounds records below u32::MAX");
        for path in endpoints(member) {
            let indices = paths.entry(path.to_owned()).or_default();
            if indices.last() != Some(&index) {
                indices.push(index);
            }
        }
    }
    paths
}

fn canonical_git_records(raw_coverage: &[GitCoverageMember]) -> Vec<RawGitChangeRecord> {
    raw_coverage
        .iter()
        .map(|member| RawGitChangeRecord {
            status: member.status,
            raw_status: member.raw_status.clone(),
            rename_score: member.rename_score,
            old_path: match member.status {
                GitChangeStatus::Deleted => Some(member.new_path.clone()),
                _ => member.old_path.clone(),
            },
            new_path: match member.status {
                GitChangeStatus::Deleted => None,
                _ => Some(member.new_path.clone()),
            },
            old_mode: member.old_mode.clone(),
            new_mode: member.new_mode.clone(),
            old_object_type: member.old_object_type,
            new_object_type: member.new_object_type,
            old_object: member.old_object.clone(),
            new_object: member.new_object.clone(),
        })
        .collect()
}

fn evaluated_object_for_path<'a>(
    path: &str,
    record_indices: &[u32],
    raw_coverage: &'a [GitCoverageMember],
) -> Option<&'a str> {
    let mut object = None;
    for index in record_indices {
        let member = raw_coverage.get(usize::try_from(*index).ok()?)?;
        let candidate = if member.old_path.as_deref() == Some(path)
            || (member.status == GitChangeStatus::Deleted && member.new_path == path)
        {
            member.old_object.as_str()
        } else if member.new_path == path {
            member.new_object.as_str()
        } else {
            return None;
        };
        if object.is_some_and(|object| object != candidate) {
            return None;
        }
        object = Some(candidate);
    }
    object
}

fn contract_matches_commit(
    contract: &ConformanceContract,
    commit: &ConventionalCommitEvidence,
) -> bool {
    if contract.binding != commit.binding
        || commit.commit_revision != commit.binding.commit_revision
    {
        return false;
    }

    let source_indices: Vec<_> = contract
        .sources
        .iter()
        .enumerate()
        .filter_map(|(index, source)| (source == &commit.source).then_some(index))
        .collect();
    let [source_index] = source_indices.as_slice() else {
        return false;
    };
    let Ok(source_index) = u32::try_from(*source_index) else {
        return false;
    };
    if commit.claims.iter().any(|claim| claim.source_index != 0)
        || commit
            .declared_scope
            .iter()
            .any(|scope| scope.source_index != 0)
    {
        return false;
    }

    let mut contract_claims: Vec<_> = contract
        .claimed_changes
        .iter()
        .filter(|claim| claim.source_index == source_index)
        .cloned()
        .collect();
    let mut commit_claims = commit.claims.clone();
    for claim in &mut commit_claims {
        claim.source_index = source_index;
    }
    contract_claims.sort_by(claim_order);
    commit_claims.sort_by(claim_order);
    if contract_claims != commit_claims {
        return false;
    }

    let mut contract_scopes: Vec<_> = contract
        .declared_scopes
        .iter()
        .filter(|scope| scope.source_index == source_index)
        .cloned()
        .collect();
    let mut commit_scopes: Vec<_> = commit.declared_scope.clone().into_iter().collect();
    for scope in &mut commit_scopes {
        scope.source_index = source_index;
    }
    contract_scopes.sort_by(scope_order);
    commit_scopes.sort_by(scope_order);
    contract_scopes == commit_scopes
}

fn invalid_evaluation(contract: &ConformanceContract, reason: String) -> ConformanceEvaluation {
    invalid_evaluation_reasons(contract, vec![reason], Vec::new())
}

fn invalid_evaluation_reasons(
    contract: &ConformanceContract,
    mut reasons: Vec<String>,
    git_non_evaluations: Vec<GitCommitNonEvaluation>,
) -> ConformanceEvaluation {
    sort_deduplicate(&mut reasons);
    let (sources, source_index_map) = canonical_sources(contract);
    let verdict = ConformanceVerdict {
        claim_table_version: CONFORMANCE_CLAIM_TABLE_VERSION,
        binding: contract.binding.clone(),
        sources,
        evaluated_claims: Vec::new(),
        declared_scopes: canonical_scopes(contract, &source_index_map),
        outcome: ConformanceOutcome::NotEvaluated,
        evidence_strength: EvidenceStrength::Absent,
        reasons,
        git_records: Vec::new(),
        coverage: Vec::new(),
        uncovered_files: Vec::new(),
    };
    let findings = advisory_findings(&verdict);
    ConformanceEvaluation {
        verdict,
        claim_results: Vec::new(),
        findings,
        git_non_evaluations,
    }
}

fn advisory_findings(verdict: &ConformanceVerdict) -> Vec<Diagnostic> {
    if verdict.outcome == ConformanceOutcome::Conformant
        && verdict.evidence_strength == EvidenceStrength::Complete
    {
        return Vec::new();
    }
    let path = verdict
        .uncovered_files
        .first()
        .or_else(|| verdict.coverage.first().map(|member| &member.path))
        .cloned()
        .unwrap_or_else(|| ".".to_owned());
    verdict
        .reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            Diagnostic::new(
                format!("CONF-004-{index}"),
                Severity::Warning,
                format!("intent conformance: {reason}"),
                Location {
                    file: path.clone(),
                    line: None,
                    column: None,
                    end_line: None,
                    end_column: None,
                },
                Category::Policy,
                DiagnosticSource {
                    rule_id: "CONF-004".to_owned(),
                    source_module: "anvil-checks::conformance".to_owned(),
                },
                Mode::known(KnownMode::Gate),
            )
        })
        .collect()
}

fn contributing_policy_sources(
    scopes: &[DeclaredScope],
    extracted_sources: &[String],
) -> BTreeSet<String> {
    let mut sources: BTreeSet<_> = scopes
        .iter()
        .filter_map(|scope| match &scope.authority {
            ScopeAuthority::BaseMapping { source_path, .. } => Some(source_path.clone()),
            ScopeAuthority::None | ScopeAuthority::ExplicitPath { .. } => None,
        })
        .collect();
    sources.extend(extracted_sources.iter().cloned());
    sources
}

fn endpoints(member: &GitCoverageMember) -> impl Iterator<Item = &str> {
    member
        .old_path
        .as_deref()
        .into_iter()
        .chain(std::iter::once(member.new_path.as_str()))
}

fn path_matches_prefix(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|remainder| remainder.starts_with('/'))
}

fn is_documentation_path(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    path.starts_with("docs/")
        || path.starts_with("plans/")
        || matches!(
            file,
            "readme"
                | "readme.md"
                | "readme.mdx"
                | "changelog"
                | "changelog.md"
                | "contributing.md"
                | "LICENSE"
                | "license"
                | "license.md"
                | "notice"
                | "notice.md"
        )
        || [".md", ".mdx", ".rst", ".adoc"]
            .iter()
            .any(|extension| path.ends_with(extension))
}

fn is_test_path(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    path.split('/')
        .any(|segment| matches!(segment, "test" | "tests" | "__tests__" | "fixtures"))
        || file.starts_with("test_")
        || file.ends_with("_test.rs")
        || file.ends_with("_test.go")
        || [
            ".test.js",
            ".test.ts",
            ".test.tsx",
            ".spec.js",
            ".spec.ts",
            ".spec.tsx",
            ".snap",
        ]
        .iter()
        .any(|suffix| file.ends_with(suffix))
}

fn canonical_sources(contract: &ConformanceContract) -> (Vec<IntentSource>, Vec<u32>) {
    let mut indexed: Vec<_> = contract.sources.iter().cloned().enumerate().collect();
    indexed.sort_by(|(left_index, left), (right_index, right)| {
        source_order(left, right).then_with(|| left_index.cmp(right_index))
    });
    let mut source_index_map = vec![0; indexed.len()];
    for (canonical_index, (original_index, _)) in indexed.iter().enumerate() {
        source_index_map[*original_index] =
            u32::try_from(canonical_index).expect("contract source index fits u32");
    }
    (
        indexed.into_iter().map(|(_, source)| source).collect(),
        source_index_map,
    )
}

fn canonical_scopes(
    contract: &ConformanceContract,
    source_index_map: &[u32],
) -> Vec<DeclaredScope> {
    let mut scopes = contract.declared_scopes.clone();
    for scope in &mut scopes {
        if let Some(index) = usize::try_from(scope.source_index)
            .ok()
            .and_then(|index| source_index_map.get(index))
        {
            scope.source_index = *index;
        }
    }
    scopes.sort_by(scope_order);
    scopes
}

fn remap_claim_source(claim: &mut ConformanceClaim, source_index_map: &[u32]) {
    if let Some(index) = usize::try_from(claim.source_index)
        .ok()
        .and_then(|index| source_index_map.get(index))
    {
        claim.source_index = *index;
    }
}

fn source_order(left: &IntentSource, right: &IntentSource) -> std::cmp::Ordering {
    intent_tier_order(left.tier)
        .cmp(&intent_tier_order(right.tier))
        .then_with(|| intent_kind_order(left.kind).cmp(&intent_kind_order(right.kind)))
        .then_with(|| left.reference.as_bytes().cmp(right.reference.as_bytes()))
        .then_with(|| left.digest.as_bytes().cmp(right.digest.as_bytes()))
        .then_with(|| {
            evidence_grade_order(left.evidence_grade)
                .cmp(&evidence_grade_order(right.evidence_grade))
        })
        .then_with(|| {
            left.producer_schema
                .as_bytes()
                .cmp(right.producer_schema.as_bytes())
        })
        .then_with(|| left.producer_record_id.cmp(&right.producer_record_id))
}

const fn intent_tier_order(tier: IntentTier) -> u8 {
    match tier {
        IntentTier::Tier0 => 0,
        IntentTier::Tier1 => 1,
        IntentTier::Tier2 => 2,
    }
}

const fn intent_kind_order(kind: IntentSourceKind) -> u8 {
    match kind {
        IntentSourceKind::ConventionalCommit => 0,
        IntentSourceKind::PullRequest => 1,
        IntentSourceKind::SessionIntent => 2,
        IntentSourceKind::PlanAdapter => 3,
    }
}

const fn evidence_grade_order(grade: EvidenceGrade) -> u8 {
    match grade {
        EvidenceGrade::Strong => 0,
        EvidenceGrade::Moderate => 1,
        EvidenceGrade::Weak => 2,
    }
}

fn claim_order(left: &ConformanceClaim, right: &ConformanceClaim) -> std::cmp::Ordering {
    claim_kind_order(left.kind)
        .cmp(&claim_kind_order(right.kind))
        .then_with(|| left.value.as_bytes().cmp(right.value.as_bytes()))
        .then_with(|| left.source_index.cmp(&right.source_index))
}

const fn claim_kind_order(kind: ClaimKind) -> u8 {
    match kind {
        ClaimKind::FileClass => 0,
        ClaimKind::PathPrefix => 1,
        ClaimKind::GraphSemantic => 2,
    }
}

fn scope_order(left: &DeclaredScope, right: &DeclaredScope) -> std::cmp::Ordering {
    left.label
        .as_bytes()
        .cmp(right.label.as_bytes())
        .then_with(|| left.source_index.cmp(&right.source_index))
}

fn coverage_order(left: &GitCoverageMember, right: &GitCoverageMember) -> std::cmp::Ordering {
    left.old_path
        .as_deref()
        .unwrap_or(&left.new_path)
        .as_bytes()
        .cmp(
            right
                .old_path
                .as_deref()
                .unwrap_or(&right.new_path)
                .as_bytes(),
        )
        .then_with(|| left.new_path.as_bytes().cmp(right.new_path.as_bytes()))
        .then_with(|| left.raw_status.as_bytes().cmp(right.raw_status.as_bytes()))
}

fn sort_deduplicate(values: &mut Vec<String>) {
    values.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    values.dedup();
}
