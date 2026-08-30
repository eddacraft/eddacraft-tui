use anvil_checks::conformance::{
    GitBudgetDiagnostics, GitCoverageMember, GitEvaluationIdentity, GitFootprintCommitEvidence,
    GitFootprintCommitExtraction, GitFootprintExtraction, GitNonEvaluation, PrBodyContractParts,
    evaluate_pr_declaration, extract_pr_body_claims,
};
use anvil_kernel_types::{
    ConformanceOutcome, ConformanceVerdict, EvaluationBinding, EvidenceDisposition, EvidenceGrade,
    EvidenceStrength, GitChangeStatus, GitObjectType,
};

#[test]
fn complete_range_is_evaluated_as_one_pr_declaration_footprint() {
    let declaration = extract_pr_body_claims(
        "github:pull-request:42:body:sha256:fixture",
        "```anvil-claims\nclaim: documentation-only\nscope: path:docs\n```\n",
    )
    .into_contract_parts()
    .expect("valid PR declaration");
    let identity = identity();
    let base = "a".repeat(40);
    let head = "c".repeat(40);
    let mut extraction = GitFootprintExtraction {
        base_revision: base.clone(),
        head_revision: head.clone(),
        commits: vec![
            evaluated_commit("c", "docs/a-first.md", &identity, &base, &head),
            evaluated_commit("b", "docs/z-last.md", &identity, &base, &head),
        ],
    };
    let GitFootprintCommitExtraction::Evaluated(second_commit) = &mut extraction.commits[1] else {
        unreachable!("fixture commit is evaluated");
    };
    second_commit.coverage.push(change("docs/middle.md"));

    let report = evaluate_pr_declaration(&declaration, &extraction, &identity);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::Conformant);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Complete);
    assert_eq!(report.verdict.binding.base_revision, base);
    assert_eq!(report.verdict.binding.head_revision, head);
    assert_eq!(
        report.verdict.binding.commit_revision,
        report.verdict.binding.head_revision
    );
    assert_eq!(report.verdict.sources, vec![declaration.source.clone()]);
    assert_eq!(
        report.verdict.sources[0].evidence_grade,
        EvidenceGrade::Weak
    );
    assert_eq!(
        report
            .verdict
            .coverage
            .iter()
            .map(|member| (member.path.as_str(), member.disposition))
            .collect::<Vec<_>>(),
        vec![
            ("docs/a-first.md", EvidenceDisposition::GitSufficient),
            ("docs/middle.md", EvidenceDisposition::GitSufficient),
            ("docs/z-last.md", EvidenceDisposition::GitSufficient),
        ]
    );
    assert_eq!(report.verdict.git_records.len(), 3);
    assert_per_commit_evidence(&report.verdict, &base);
    assert!(report.findings.is_empty());
}

fn assert_per_commit_evidence(verdict: &ConformanceVerdict, base: &str) {
    assert_eq!(verdict.git_commits.len(), 2);
    assert_eq!(
        verdict
            .git_commits
            .iter()
            .map(|commit| (
                commit.binding.commit_revision.as_str(),
                commit.parent_revision.as_str(),
                commit.git_records.len(),
                commit.coverage[0].record_indices.as_slice(),
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                base,
                2,
                [0].as_slice(),
            ),
            (
                "cccccccccccccccccccccccccccccccccccccccc",
                base,
                1,
                [0].as_slice(),
            ),
        ]
    );
    assert_eq!(
        verdict.git_commits[0]
            .git_records
            .iter()
            .map(|record| record.new_path.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("docs/middle.md"), Some("docs/z-last.md")]
    );
    assert_eq!(
        verdict.git_commits[0]
            .coverage
            .iter()
            .map(|member| (member.path.as_str(), member.record_indices.as_slice()))
            .collect::<Vec<_>>(),
        vec![
            ("docs/middle.md", [0].as_slice()),
            ("docs/z-last.md", [1].as_slice()),
        ]
    );
    verdict
        .validate_output_shape()
        .expect("range and per-commit evidence stay internally bound");
}

#[test]
fn explicit_path_scopes_from_one_declaration_are_evaluated_as_a_union() {
    let declaration = extract_pr_body_claims(
        "github:pull-request:42:body:sha256:fixture",
        "```anvil-claims\nscope: path:docs\nscope: path:tests\n```\n",
    )
    .into_contract_parts()
    .expect("valid multi-scope PR declaration");
    let identity = identity();
    let base = "a".repeat(40);
    let head = "c".repeat(40);
    let extraction = GitFootprintExtraction {
        base_revision: base.clone(),
        head_revision: head.clone(),
        commits: vec![
            evaluated_commit("b", "docs/guide.md", &identity, &base, &head),
            evaluated_commit("c", "tests/guide_test.rs", &identity, &base, &head),
        ],
    };

    let report = evaluate_pr_declaration(&declaration, &extraction, &identity);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::Conformant);
    assert_eq!(report.claim_results.len(), 2);
    assert!(report.claim_results.iter().all(|result| {
        result.outcome == ConformanceOutcome::Conformant && result.uncovered_files.is_empty()
    }));
    assert!(report.verdict.uncovered_files.is_empty());
    assert!(report.findings.is_empty());
}

#[test]
fn incomplete_or_mismatched_range_is_reason_coded_not_evaluated() {
    let declaration = docs_declaration();
    let identity = identity();
    let base = "a".repeat(40);
    let head = "c".repeat(40);

    assert_not_evaluated(
        &declaration,
        &GitFootprintExtraction {
            base_revision: base.clone(),
            head_revision: head.clone(),
            commits: Vec::new(),
        },
        &identity,
        "selection.range-empty",
    );
    assert_not_evaluated(
        &declaration,
        &GitFootprintExtraction {
            base_revision: base.clone(),
            head_revision: head.clone(),
            commits: vec![GitFootprintCommitExtraction::NotEvaluated(
                GitNonEvaluation {
                    commit_revision: Some("b".repeat(40).into_boxed_str()),
                    reason: "budget.records",
                    stage: "diff",
                    observed: 2,
                    limit: Some(1),
                    detail: "fixture overflow".into(),
                    raw_digest: None,
                    budget: None,
                },
            )],
        },
        &identity,
        "git.commit-not-evaluated.budget.records",
    );

    for (field, expected_reason) in [
        ("run", "binding.run-id-mismatch"),
        ("repository", "binding.repository-mismatch"),
        ("worktree", "binding.worktree-mismatch"),
        ("base", "binding.base-revision-mismatch"),
        ("head", "binding.head-revision-mismatch"),
        ("commit", "binding.commit-revision-mismatch"),
    ] {
        let mut extraction = GitFootprintExtraction {
            base_revision: base.clone(),
            head_revision: head.clone(),
            commits: vec![evaluated_commit(
                "c",
                "docs/guide.md",
                &identity,
                &base,
                &head,
            )],
        };
        let GitFootprintCommitExtraction::Evaluated(commit) = &mut extraction.commits[0] else {
            unreachable!("fixture commit is evaluated");
        };
        match field {
            "run" => commit.binding.run_id = "substituted-run".into(),
            "repository" => commit.binding.repository_id = "substituted-repo".into(),
            "worktree" => commit.binding.canonical_worktree_id = "substituted-worktree".into(),
            "base" => commit.binding.base_revision = "d".repeat(40),
            "head" => commit.binding.head_revision = "d".repeat(40),
            "commit" => commit.binding.commit_revision = "d".repeat(40),
            _ => unreachable!("closed fixture case"),
        }
        assert_not_evaluated(&declaration, &extraction, &identity, expected_reason);
        let report = evaluate_pr_declaration(&declaration, &extraction, &identity);
        let [failure] = report.git_non_evaluations.as_slice() else {
            panic!("one binding failure must be retained per commit");
        };
        assert_eq!(failure.reason, expected_reason);
        assert_eq!(failure.stage, "binding");
        assert_eq!(
            failure.commit_revision.as_deref(),
            Some("cccccccccccccccccccccccccccccccccccccccc")
        );
    }
}

#[test]
fn every_per_commit_failure_is_retained_with_safe_diagnostics_in_canonical_order() {
    let declaration = docs_declaration();
    let identity = identity();
    let extraction = GitFootprintExtraction {
        base_revision: "a".repeat(40),
        head_revision: "d".repeat(40),
        commits: vec![
            budget_footprint_failure("d", 12, 10, "sha256:d"),
            footprint_failure("c", "git.path-invalid-utf8"),
            budget_footprint_failure("b", 11, 10, "sha256:b"),
        ],
    };

    let report = evaluate_pr_declaration(&declaration, &extraction, &identity);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::NotEvaluated);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Absent);
    assert_eq!(
        report.verdict.reasons,
        vec![
            "git.commit-not-evaluated.budget.records",
            "git.commit-not-evaluated.git.path-invalid-utf8",
        ]
    );
    assert_eq!(report.git_non_evaluations.len(), 3);
    assert_eq!(
        report
            .git_non_evaluations
            .iter()
            .map(|failure| failure.commit_revision.as_deref())
            .collect::<Vec<_>>(),
        vec![
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            Some("cccccccccccccccccccccccccccccccccccccccc"),
            Some("dddddddddddddddddddddddddddddddddddddddd"),
        ]
    );
    let failures_with_same_reason: Vec<_> = report
        .git_non_evaluations
        .iter()
        .filter(|failure| failure.reason == "budget.records")
        .collect();
    assert_eq!(failures_with_same_reason.len(), 2);
    assert_eq!(failures_with_same_reason[0].stage, "diff");
    assert_eq!(failures_with_same_reason[0].observed, 11);
    assert_eq!(failures_with_same_reason[0].limit, Some(10));
    assert_eq!(
        failures_with_same_reason[0].raw_digest.as_deref(),
        Some("sha256:b")
    );
    let budget = failures_with_same_reason[0]
        .budget
        .as_deref()
        .expect("safe budget diagnostics");
    assert_eq!(budget.configured_limit, 10);
    assert_eq!(budget.records, Some(11));
    assert_eq!(budget.raw_bytes, Some(101));
    assert_eq!(budget.raw_output_digest.as_deref(), Some("sha256:b"));
}

#[test]
fn every_binding_mismatched_commit_is_retained_once_in_canonical_order() {
    let declaration = docs_declaration();
    let identity = identity();
    let base = "a".repeat(40);
    let head = "c".repeat(40);
    let mut extraction = GitFootprintExtraction {
        base_revision: base.clone(),
        head_revision: head.clone(),
        commits: vec![
            evaluated_commit("c", "docs/c.md", &identity, &base, &head),
            evaluated_commit("b", "docs/b.md", &identity, &base, &head),
        ],
    };
    for result in &mut extraction.commits {
        let GitFootprintCommitExtraction::Evaluated(commit) = result else {
            unreachable!("fixture commits are evaluated");
        };
        commit.binding.run_id = "substituted-run".to_owned();
    }

    let report = evaluate_pr_declaration(&declaration, &extraction, &identity);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::NotEvaluated);
    assert_eq!(report.verdict.reasons, vec!["binding.run-id-mismatch"]);
    assert_eq!(report.git_non_evaluations.len(), 2);
    assert_eq!(
        report
            .git_non_evaluations
            .iter()
            .map(|failure| failure.commit_revision.as_deref())
            .collect::<Vec<_>>(),
        vec![
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            Some("cccccccccccccccccccccccccccccccccccccccc"),
        ]
    );
    assert!(
        report
            .git_non_evaluations
            .iter()
            .all(|failure| failure.reason == "binding.run-id-mismatch"
                && failure.stage == "binding"
                && failure.observed == 0
                && failure.limit.is_none()
                && failure.raw_digest.is_none()
                && failure.budget.is_none())
    );
}

#[test]
fn graph_semantic_pr_claim_stays_not_evaluated_without_bound_graph_evidence() {
    let declaration = extract_pr_body_claims(
        "github:pull-request:42:body:sha256:fixture",
        "```anvil-claims\nclaim: no-behaviour-change\n```\n",
    )
    .into_contract_parts()
    .expect("valid graph-semantic declaration");
    let identity = identity();
    let base = "a".repeat(40);
    let head = "c".repeat(40);
    let extraction = GitFootprintExtraction {
        base_revision: base.clone(),
        head_revision: head.clone(),
        commits: vec![evaluated_commit("c", "src/lib.rs", &identity, &base, &head)],
    };

    let report = evaluate_pr_declaration(&declaration, &extraction, &identity);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::NotEvaluated);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Absent);
    assert!(
        report
            .verdict
            .reasons
            .contains(&"binding.graph-missing".into())
    );
}

fn docs_declaration() -> PrBodyContractParts {
    extract_pr_body_claims(
        "github:pull-request:42:body:sha256:fixture",
        "```anvil-claims\nclaim: documentation-only\n```\n",
    )
    .into_contract_parts()
    .expect("valid documentation-only declaration")
}

fn assert_not_evaluated(
    declaration: &PrBodyContractParts,
    extraction: &GitFootprintExtraction,
    identity: &GitEvaluationIdentity,
    expected_reason: &str,
) {
    let report = evaluate_pr_declaration(declaration, extraction, identity);
    assert_eq!(report.verdict.outcome, ConformanceOutcome::NotEvaluated);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Absent);
    assert!(report.verdict.coverage.is_empty());
    assert_eq!(report.verdict.reasons, vec![expected_reason]);
}

fn identity() -> GitEvaluationIdentity {
    GitEvaluationIdentity::from_unverified_parts("run-pr-01", "repo-01", "worktree-01")
        .expect("fixture identity")
}

fn footprint_failure(revision_seed: &str, reason: &'static str) -> GitFootprintCommitExtraction {
    GitFootprintCommitExtraction::NotEvaluated(GitNonEvaluation {
        commit_revision: Some(revision_seed.repeat(40).into_boxed_str()),
        reason,
        stage: "fixture",
        observed: 0,
        limit: None,
        detail: "fixture failure".into(),
        raw_digest: None,
        budget: None,
    })
}

fn budget_footprint_failure(
    revision_seed: &str,
    observed: usize,
    limit: usize,
    digest: &str,
) -> GitFootprintCommitExtraction {
    GitFootprintCommitExtraction::NotEvaluated(GitNonEvaluation {
        commit_revision: Some(revision_seed.repeat(40).into_boxed_str()),
        reason: "budget.records",
        stage: "diff",
        observed,
        limit: Some(limit),
        detail: "unsafe raw detail must not escape".into(),
        raw_digest: Some(digest.into()),
        budget: Some(Box::new(GitBudgetDiagnostics {
            configured_limit: limit,
            elapsed_millis: Some(7),
            commits: Some(3),
            records: Some(observed),
            rename_sources: Some(2),
            rename_targets: Some(1),
            raw_bytes: Some(101),
            decoded_bytes: Some(89),
            raw_output_digest: Some(digest.into()),
        })),
    })
}

fn evaluated_commit(
    revision_seed: &str,
    path: &str,
    identity: &GitEvaluationIdentity,
    base: &str,
    head: &str,
) -> GitFootprintCommitExtraction {
    let revision = revision_seed.repeat(40);
    GitFootprintCommitExtraction::Evaluated(Box::new(GitFootprintCommitEvidence {
        commit_revision: revision.clone(),
        parent_revision: base.to_owned(),
        binding: EvaluationBinding {
            run_id: identity.run_id.clone(),
            repository_id: identity.repository_id.clone(),
            canonical_worktree_id: identity.canonical_worktree_id.clone(),
            base_revision: base.to_owned(),
            head_revision: head.to_owned(),
            commit_revision: revision,
        },
        coverage: vec![change(path)],
    }))
}

fn change(path: &str) -> GitCoverageMember {
    GitCoverageMember {
        status: GitChangeStatus::Modified,
        raw_status: "M".into(),
        rename_score: None,
        old_path: None,
        new_path: path.into(),
        old_mode: "100644".into(),
        new_mode: "100644".into(),
        old_object_type: GitObjectType::Blob,
        new_object_type: GitObjectType::Blob,
        old_object: "1".repeat(40),
        new_object: "2".repeat(40),
    }
}
