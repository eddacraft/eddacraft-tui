use anvil_checks::conformance::{
    GitCoverageMember, GitEvaluationIdentity, GitFootprintCommitEvidence,
    GitFootprintCommitExtraction, GitFootprintExtraction, GitNonEvaluation, PrBodyContractParts,
    evaluate_pr_declaration, extract_pr_body_claims,
};
use anvil_kernel_types::{
    ConformanceOutcome, EvaluationBinding, EvidenceDisposition, EvidenceGrade, EvidenceStrength,
    GitChangeStatus, GitObjectType,
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
    let extraction = GitFootprintExtraction {
        base_revision: base.clone(),
        head_revision: head.clone(),
        commits: vec![
            evaluated_commit("b", "docs/z-last.md", &identity, &base, &head),
            evaluated_commit("c", "docs/a-first.md", &identity, &base, &head),
        ],
    };

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
            ("docs/z-last.md", EvidenceDisposition::GitSufficient),
        ]
    );
    assert_eq!(report.verdict.git_records.len(), 2);
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
    }
}

#[test]
fn every_per_commit_failure_is_reported_once_in_canonical_order() {
    let declaration = docs_declaration();
    let identity = identity();
    let extraction = GitFootprintExtraction {
        base_revision: "a".repeat(40),
        head_revision: "d".repeat(40),
        commits: vec![
            footprint_failure("b", "git.path-invalid-utf8"),
            footprint_failure("c", "budget.records"),
            footprint_failure("d", "git.path-invalid-utf8"),
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
    GitEvaluationIdentity {
        run_id: "run-pr-01".into(),
        repository_id: "repo-01".into(),
        canonical_worktree_id: "worktree-01".into(),
    }
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
