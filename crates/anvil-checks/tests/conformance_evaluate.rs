use anvil_checks::conformance::evaluate::{
    BoundGraphDelta, CONFORMANCE_CLAIM_TABLE_VERSION, evaluate_tier0,
};
use anvil_checks::conformance::git::{
    ConventionalCommitEvidence, ConventionalCommitHeader, GitCommitExtraction, GitCoverageMember,
    GitExtractionOutcome, GitExtractor, GitSelection,
};
use anvil_graph_cache::GraphDelta;
use anvil_kernel_types::{
    CONFORMANCE_SCHEMA_VERSION, Category, ClaimKind, ConformanceClaim, ConformanceContract,
    ConformanceOutcome, DeclaredScope, EvaluationBinding, EvidenceDisposition, EvidenceGrade,
    EvidenceStrength, GitChangeStatus, GitObjectType, GraphEvidenceBinding, IntentSource,
    IntentSourceKind, IntentTier, ScopeAuthority, Severity,
};
use std::path::Path;

#[test]
fn explicit_path_scope_covers_exact_and_descendant_paths() {
    let commit = commit(
        vec![path_claim("src/api")],
        Some(explicit_scope("src/api")),
        vec![change("src/api"), change("src/api/routes.rs")],
    );
    let contract = contract_from(&commit, vec![]);

    let report = evaluate_tier0(&contract, &commit, &[]);

    assert_eq!(
        report.verdict.claim_table_version,
        CONFORMANCE_CLAIM_TABLE_VERSION
    );
    assert_eq!(report.verdict.outcome, ConformanceOutcome::Conformant);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Complete);
    assert!(
        report
            .verdict
            .coverage
            .iter()
            .all(|member| member.disposition == EvidenceDisposition::GitSufficient)
    );
    assert!(report.findings.is_empty());
}

#[test]
fn unbound_or_substituted_extractor_coverage_is_never_trusted() {
    let commit = commit(
        vec![path_claim("src")],
        Some(explicit_scope("src")),
        vec![change("src/lib.rs")],
    );
    let mut contract = contract_from(&commit, vec![]);
    contract.binding.commit_revision = "d".repeat(40);

    let report = evaluate_tier0(&contract, &commit, &[]);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::NotEvaluated);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Absent);
    assert!(report.verdict.coverage.is_empty());
    assert!(
        report
            .verdict
            .reasons
            .contains(&"binding.extracted-commit-mismatch".into())
    );
}

#[test]
fn structurally_invalid_contract_is_reason_coded_without_trusting_coverage() {
    let commit = commit(
        vec![path_claim("src")],
        Some(explicit_scope("src")),
        vec![change("src/lib.rs")],
    );
    let mut contract = contract_from(&commit, vec![]);
    contract.schema_version = "anvil.intent-conformance.v999".into();

    let report = evaluate_tier0(&contract, &commit, &[]);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::NotEvaluated);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Absent);
    assert!(report.verdict.coverage.is_empty());
    assert_eq!(report.verdict.reasons, vec!["contract.schema.unsupported"]);
}

#[test]
fn rename_requires_both_paths_to_match_the_authorised_prefix() {
    let mut renamed = change("src/api/new.rs");
    renamed.status = GitChangeStatus::Renamed;
    renamed.raw_status = "R100".into();
    renamed.rename_score = Some(100);
    renamed.old_path = Some("src/legacy.rs".into());
    let commit = commit(
        vec![path_claim("src/api")],
        Some(explicit_scope("src/api")),
        vec![renamed],
    );
    let contract = contract_from(&commit, vec![]);

    let report = evaluate_tier0(&contract, &commit, &[]);

    report
        .verdict
        .validate_output_shape()
        .expect("evaluator emits a structurally valid verdict");
    assert_eq!(report.verdict.outcome, ConformanceOutcome::NonConformant);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Complete);
    assert_eq!(report.findings[0].severity, Severity::Warning);
    assert_eq!(report.findings[0].category, Category::Policy);
    assert_eq!(report.verdict.coverage.len(), 2);
    assert_eq!(report.verdict.coverage[0].path, "src/api/new.rs");
    assert_eq!(report.verdict.coverage[1].path, "src/legacy.rs");
}

#[test]
fn free_form_scope_never_invents_path_authority() {
    let commit = commit(
        vec![],
        Some(DeclaredScope {
            label: "api".into(),
            authority: ScopeAuthority::None,
            source_index: 0,
        }),
        vec![change("api/mod.rs")],
    );
    let contract = contract_from(&commit, vec![]);

    let report = evaluate_tier0(&contract, &commit, &[]);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::NotEvaluated);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Absent);
    assert_eq!(
        report.verdict.coverage[0].disposition,
        EvidenceDisposition::Unsupported
    );
    assert!(
        report
            .verdict
            .reasons
            .contains(&"claim.none-applicable".into())
    );
}

#[test]
fn closed_file_class_claims_use_the_complete_changed_file_set() {
    let docs_claim = ConformanceClaim {
        kind: ClaimKind::FileClass,
        value: "documentation-only".into(),
        source_index: 0,
    };
    let passing = commit(
        vec![docs_claim.clone()],
        None,
        vec![change("docs/guide.md"), change("README.md")],
    );
    assert_eq!(
        evaluate_tier0(&contract_from(&passing, vec![]), &passing, &[])
            .verdict
            .outcome,
        ConformanceOutcome::Conformant
    );

    let failing = commit(
        vec![docs_claim],
        None,
        vec![change("docs/guide.md"), change("src/lib.rs")],
    );
    let violation = evaluate_tier0(&contract_from(&failing, vec![]), &failing, &[]);
    assert_eq!(violation.verdict.outcome, ConformanceOutcome::NonConformant);
    assert_eq!(violation.verdict.uncovered_files, vec!["src/lib.rs"]);

    let test_claim = ConformanceClaim {
        kind: ClaimKind::FileClass,
        value: "test-only".into(),
        source_index: 0,
    };
    let tests = commit(
        vec![test_claim],
        None,
        vec![change("tests/cli.rs"), change("src/parser_test.rs")],
    );
    assert_eq!(
        evaluate_tier0(&contract_from(&tests, vec![]), &tests, &[])
            .verdict
            .outcome,
        ConformanceOutcome::Conformant
    );
}

#[test]
fn base_mapping_changes_are_visible_without_self_authorising() {
    let scope = DeclaredScope {
        label: "core".into(),
        authority: ScopeAuthority::BaseMapping {
            mapping_key: "core".into(),
            prefixes: vec!["src".into()],
            schema_version: 1,
            source_path: ".anvil.yaml".into(),
            source_digest: "sha256:base-config".into(),
        },
        source_index: 0,
    };
    let commit = commit(
        vec![path_claim("src")],
        Some(scope),
        vec![change("src/lib.rs"), change(".anvil.yaml")],
    );
    let contract = contract_from(&commit, vec![]);

    let report = evaluate_tier0(&contract, &commit, &[]);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::NonConformant);
    let policy_change = report
        .verdict
        .coverage
        .iter()
        .find(|member| member.path == ".anvil.yaml")
        .expect("contributing base configuration remains in coverage");
    assert!(policy_change.policy_change);
    assert_eq!(policy_change.disposition, EvidenceDisposition::PolicyChange);
}

#[test]
fn contributing_base_config_is_policy_change_under_explicit_authority() {
    let mut commit = commit(
        vec![path_claim(".anvil.yaml")],
        Some(explicit_scope(".anvil.yaml")),
        vec![change(".anvil.yaml")],
    );
    commit.contributing_base_config_paths = vec![".anvil.yaml".into()];
    let contract = contract_from(&commit, vec![]);

    let report = evaluate_tier0(&contract, &commit, &[]);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::Conformant);
    assert_eq!(report.verdict.coverage.len(), 1);
    assert!(report.verdict.coverage[0].policy_change);
    assert_eq!(
        report.verdict.coverage[0].disposition,
        EvidenceDisposition::PolicyChange
    );
}

#[test]
fn a_proven_path_violation_survives_unsupported_graph_evidence() {
    let commit = commit(
        vec![
            path_claim("src"),
            ConformanceClaim {
                kind: ClaimKind::GraphSemantic,
                value: "imports:stable".into(),
                source_index: 0,
            },
        ],
        Some(explicit_scope("src")),
        vec![change("README.md")],
    );
    let contract = contract_from(&commit, vec![]);

    let report = evaluate_tier0(&contract, &commit, &[]);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::NonConformant);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Partial);
    assert_eq!(
        report.verdict.coverage[0].disposition,
        EvidenceDisposition::Missing
    );
}

#[test]
fn binding_metadata_and_a_graph_delta_cannot_launder_an_undefined_semantic_claim() {
    let mut member = change("src/lib.rs");
    member.new_object = "c".repeat(40);
    let graph_binding = GraphEvidenceBinding {
        run_id: "run-01".into(),
        path: "src/lib.rs".into(),
        revision: "b".repeat(40),
        blob: "c".repeat(40),
        schema_version: 1,
        generation: 7,
    };
    let commit = commit(
        vec![ConformanceClaim {
            kind: ClaimKind::GraphSemantic,
            value: "imports:stable".into(),
            source_index: 0,
        }],
        None,
        vec![member],
    );
    let contract = contract_from(&commit, vec![graph_binding.clone()]);
    let delta = GraphDelta {
        file: "src/lib.rs".into(),
        ..GraphDelta::default()
    };
    let bound = BoundGraphDelta {
        repository_id: "repo-01",
        canonical_worktree_id: "worktree-01",
        binding: &graph_binding,
        delta: &delta,
    };

    let report = evaluate_tier0(&contract, &commit, &[bound]);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::NotEvaluated);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Absent);
    assert_eq!(
        report.verdict.coverage[0].disposition,
        EvidenceDisposition::Unsupported
    );
    assert!(
        report
            .verdict
            .reasons
            .contains(&"claim.graph-semantic.unsupported".into())
    );
}

#[test]
fn applicable_empty_footprint_is_conformant_and_prefixes_are_exact() {
    let empty = commit(vec![path_claim("src")], Some(explicit_scope("src")), vec![]);
    let empty_report = evaluate_tier0(&contract_from(&empty, vec![]), &empty, &[]);
    assert_eq!(empty_report.verdict.outcome, ConformanceOutcome::Conformant);
    assert_eq!(
        empty_report.verdict.evidence_strength,
        EvidenceStrength::Complete
    );

    let case_mismatch = commit(
        vec![path_claim("src/Auth")],
        Some(explicit_scope("src/Auth")),
        vec![change("src/auth/mod.rs")],
    );
    assert_eq!(
        evaluate_tier0(&contract_from(&case_mismatch, vec![]), &case_mismatch, &[])
            .verdict
            .outcome,
        ConformanceOutcome::NonConformant
    );
}

#[test]
fn claim_and_coverage_input_permutations_produce_identical_results() {
    let claims = vec![
        path_claim("docs"),
        ConformanceClaim {
            kind: ClaimKind::FileClass,
            value: "documentation-only".into(),
            source_index: 0,
        },
    ];
    let first = commit(
        claims.clone(),
        Some(explicit_scope("docs")),
        vec![change("README.md"), change("docs/guide.md")],
    );
    let mut reversed_claims = claims;
    reversed_claims.reverse();
    let second = commit(
        reversed_claims,
        Some(explicit_scope("docs")),
        vec![change("docs/guide.md"), change("README.md")],
    );

    assert_eq!(
        evaluate_tier0(&contract_from(&first, vec![]), &first, &[]),
        evaluate_tier0(&contract_from(&second, vec![]), &second, &[])
    );
}

#[test]
fn mapped_prefixes_form_one_authorised_union() {
    let scope = DeclaredScope {
        label: "core".into(),
        authority: ScopeAuthority::BaseMapping {
            mapping_key: "core".into(),
            prefixes: vec!["crates/core".into(), "src/core".into()],
            schema_version: 1,
            source_path: ".anvil.yaml".into(),
            source_digest: "sha256:base-config".into(),
        },
        source_index: 0,
    };
    let commit = commit(
        vec![path_claim("crates/core"), path_claim("src/core")],
        Some(scope),
        vec![
            change("crates/core/src/lib.rs"),
            change("src/core/runtime.rs"),
        ],
    );

    let report = evaluate_tier0(&contract_from(&commit, vec![]), &commit, &[]);

    assert_eq!(report.verdict.outcome, ConformanceOutcome::Conformant);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Complete);
    assert!(report.verdict.uncovered_files.is_empty());
}

#[test]
fn closed_file_classes_preserve_exact_git_path_case() {
    let docs = commit(
        vec![ConformanceClaim {
            kind: ClaimKind::FileClass,
            value: "documentation-only".into(),
            source_index: 0,
        }],
        None,
        vec![change("DOCS/payload.rs"), change("guide.MD")],
    );
    assert_eq!(
        evaluate_tier0(&contract_from(&docs, vec![]), &docs, &[])
            .verdict
            .outcome,
        ConformanceOutcome::NonConformant
    );

    let tests = commit(
        vec![ConformanceClaim {
            kind: ClaimKind::FileClass,
            value: "test-only".into(),
            source_index: 0,
        }],
        None,
        vec![change("TESTS/runtime.rs"), change("src/runtime_TEST.rs")],
    );
    assert_eq!(
        evaluate_tier0(&contract_from(&tests, vec![]), &tests, &[])
            .verdict
            .outcome,
        ConformanceOutcome::NonConformant
    );
}

#[test]
fn source_canonicalisation_remaps_multi_source_provenance() {
    let commit = commit(
        vec![path_claim("src")],
        Some(explicit_scope("src")),
        vec![change("src/lib.rs")],
    );
    let adapter = IntentSource {
        tier: IntentTier::Tier2,
        kind: IntentSourceKind::PlanAdapter,
        reference: "adapter:a".into(),
        digest: "sha256:adapter".into(),
        evidence_grade: EvidenceGrade::Strong,
        producer_schema: "adapter.v1".into(),
        producer_record_id: Some("record-1".into()),
    };

    let mut commit_first = contract_from(&commit, vec![]);
    commit_first.sources.push(adapter.clone());

    let mut adapter_first = contract_from(&commit, vec![]);
    adapter_first.sources = vec![adapter, commit.source.clone()];
    adapter_first.claimed_changes[0].source_index = 1;
    adapter_first.declared_scopes[0].source_index = 1;

    let first = evaluate_tier0(&commit_first, &commit, &[]);
    let second = evaluate_tier0(&adapter_first, &commit, &[]);

    assert_eq!(first, second);
    let source_index = first.verdict.evaluated_claims[0].source_index;
    assert_eq!(
        first.verdict.sources[usize::try_from(source_index).unwrap()],
        commit.source
    );
    assert_eq!(first.verdict.declared_scopes[0].source_index, source_index);
}

#[test]
#[allow(clippy::too_many_lines)] // One shared fixture makes every graph-binding axis comparable.
fn graph_binding_failures_are_visible_before_unsupported_semantics() {
    let mut member = change("src/lib.rs");
    member.new_object = "c".repeat(40);
    let expected = GraphEvidenceBinding {
        run_id: "run-01".into(),
        path: "src/lib.rs".into(),
        revision: "b".repeat(40),
        blob: "c".repeat(40),
        schema_version: 1,
        generation: 7,
    };
    let commit = commit(
        vec![ConformanceClaim {
            kind: ClaimKind::GraphSemantic,
            value: "imports:stable".into(),
            source_index: 0,
        }],
        None,
        vec![member],
    );
    let contract = contract_from(&commit, vec![expected.clone()]);
    let delta = GraphDelta {
        file: "src/lib.rs".into(),
        ..GraphDelta::default()
    };

    let mut stale_binding = expected.clone();
    stale_binding.blob = "d".repeat(40);
    let stale = evaluate_tier0(
        &contract,
        &commit,
        &[BoundGraphDelta {
            repository_id: "repo-01",
            canonical_worktree_id: "worktree-01",
            binding: &stale_binding,
            delta: &delta,
        }],
    );
    assert_eq!(
        stale.verdict.coverage[0].disposition,
        EvidenceDisposition::Stale
    );
    assert!(
        stale
            .verdict
            .reasons
            .contains(&"binding.graph-stale".into())
    );

    let mismatched = evaluate_tier0(
        &contract,
        &commit,
        &[BoundGraphDelta {
            repository_id: "different-repository",
            canonical_worktree_id: "worktree-01",
            binding: &expected,
            delta: &delta,
        }],
    );
    assert_eq!(
        mismatched.verdict.coverage[0].disposition,
        EvidenceDisposition::Mismatched
    );
    assert!(
        mismatched
            .verdict
            .reasons
            .contains(&"binding.graph-mismatched".into())
    );

    assert_graph_disposition(
        &contract,
        &commit,
        "repo-01",
        "different-worktree",
        &expected,
        &delta,
        EvidenceDisposition::Mismatched,
    );

    let mut wrong_run = expected.clone();
    wrong_run.run_id = "different-run".into();
    assert_graph_disposition(
        &contract,
        &commit,
        "repo-01",
        "worktree-01",
        &wrong_run,
        &delta,
        EvidenceDisposition::Mismatched,
    );

    let mut wrong_schema = expected.clone();
    wrong_schema.schema_version += 1;
    assert_graph_disposition(
        &contract,
        &commit,
        "repo-01",
        "worktree-01",
        &wrong_schema,
        &delta,
        EvidenceDisposition::Mismatched,
    );

    let mut stale_revision = expected.clone();
    stale_revision.revision = "d".repeat(40);
    assert_graph_disposition(
        &contract,
        &commit,
        "repo-01",
        "worktree-01",
        &stale_revision,
        &delta,
        EvidenceDisposition::Stale,
    );

    let mut stale_generation = expected.clone();
    stale_generation.generation += 1;
    assert_graph_disposition(
        &contract,
        &commit,
        "repo-01",
        "worktree-01",
        &stale_generation,
        &delta,
        EvidenceDisposition::Stale,
    );
}

fn assert_graph_disposition(
    contract: &ConformanceContract,
    commit: &ConventionalCommitEvidence,
    repository_id: &str,
    canonical_worktree_id: &str,
    binding: &GraphEvidenceBinding,
    delta: &GraphDelta,
    expected_disposition: EvidenceDisposition,
) {
    let report = evaluate_tier0(
        contract,
        commit,
        &[BoundGraphDelta {
            repository_id,
            canonical_worktree_id,
            binding,
            delta,
        }],
    );
    assert_eq!(report.verdict.coverage[0].disposition, expected_disposition);
}

#[test]
#[ignore = "requires the source repository's full committed history"]
fn dogfoods_a_merged_conf_commit_from_this_repository() {
    const DOGFOOD_COMMIT: &str = "6f7c4caa08f762de4e943495ae19a714d3ca8626";
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical repository path");
    let extractor = GitExtractor::default();
    let identity = extractor
        .identity_for_repository(&repository, "run-conf-004-dogfood")
        .expect("derive canonical repository identity");

    let extraction = extractor.extract(
        &repository,
        GitSelection::Commit(DOGFOOD_COMMIT.into()),
        &identity,
    );
    let GitExtractionOutcome::Evaluated(extraction) = extraction else {
        panic!("repository-history dogfood must select the pinned commit: {extraction:?}");
    };
    let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("pinned conventional commit must be evaluable");
    };

    let report = evaluate_tier0(&contract_from(commit, vec![]), commit, &[]);

    assert_eq!(commit.commit_revision, DOGFOOD_COMMIT);
    assert!(!commit.coverage.is_empty());
    assert_eq!(report.verdict.outcome, ConformanceOutcome::Conformant);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Complete);
    assert!(report.findings.is_empty());
}

fn path_claim(prefix: &str) -> ConformanceClaim {
    ConformanceClaim {
        kind: ClaimKind::PathPrefix,
        value: prefix.into(),
        source_index: 0,
    }
}

fn explicit_scope(prefix: &str) -> DeclaredScope {
    DeclaredScope {
        label: format!("path:{prefix}"),
        authority: ScopeAuthority::ExplicitPath {
            prefix: prefix.into(),
        },
        source_index: 0,
    }
}

fn contract_from(
    commit: &ConventionalCommitEvidence,
    graph_evidence: Vec<GraphEvidenceBinding>,
) -> ConformanceContract {
    ConformanceContract {
        schema_version: CONFORMANCE_SCHEMA_VERSION.into(),
        binding: commit.binding.clone(),
        sources: vec![commit.source.clone()],
        declared_scopes: commit.declared_scope.clone().into_iter().collect(),
        claimed_changes: commit.claims.clone(),
        acceptance_assertions: vec![],
        graph_evidence,
    }
}

fn commit(
    claims: Vec<ConformanceClaim>,
    declared_scope: Option<DeclaredScope>,
    coverage: Vec<GitCoverageMember>,
) -> ConventionalCommitEvidence {
    ConventionalCommitEvidence {
        commit_revision: "b".repeat(40),
        parent_revision: "a".repeat(40),
        binding: EvaluationBinding {
            run_id: "run-01".into(),
            repository_id: "repo-01".into(),
            canonical_worktree_id: "worktree-01".into(),
            base_revision: "a".repeat(40),
            head_revision: "b".repeat(40),
            commit_revision: "b".repeat(40),
        },
        header: ConventionalCommitHeader {
            commit_type: "docs".into(),
            scope: None,
            breaking: false,
            description: "fixture".into(),
        },
        source: IntentSource {
            tier: IntentTier::Tier0,
            kind: IntentSourceKind::ConventionalCommit,
            reference: "git:commit:bbbb".into(),
            digest: "sha256:message".into(),
            evidence_grade: EvidenceGrade::Weak,
            producer_schema: "git.conventional-commit.v1".into(),
            producer_record_id: None,
        },
        declared_scope,
        claims,
        coverage,
        contributing_base_config_paths: vec![],
    }
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
        old_object: "a".repeat(40),
        new_object: "b".repeat(40),
    }
}
