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
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

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
        vec![
            change("docs/guide.md"),
            change("README.md"),
            change("LICENSE"),
        ],
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

fn git_with_env(repo: &Path, args: &[&str], extra_env: &[(&str, &str)]) -> String {
    let empty_config = tempfile::Builder::new()
        .prefix("anvil-conf-test-gitconfig-")
        .tempfile()
        .expect("empty gitconfig");
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_SYSTEM", empty_config.path())
        .env("GIT_CONFIG_GLOBAL", empty_config.path())
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_PARAMETERS");
    for (key, value) in extra_env {
        cmd.env(*key, *value);
    }
    let output = cmd.output().expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf-8 git output")
        .trim()
        .to_owned()
}

fn repository() -> TempDir {
    repository_with_env(&[])
}

fn repository_with_env(extra_env: &[(&str, &str)]) -> TempDir {
    let repo = tempfile::tempdir().expect("temporary repository");
    git_with_env(repo.path(), &["init", "-q"], extra_env);
    git_with_env(
        repo.path(),
        &["config", "user.name", "CONF test"],
        extra_env,
    );
    git_with_env(
        repo.path(),
        &["config", "user.email", "conf-test@example.invalid"],
        extra_env,
    );
    git_with_env(
        repo.path(),
        &["config", "commit.gpgsign", "false"],
        extra_env,
    );
    let empty_hooks = repo.path().join("empty-hooks");
    std::fs::create_dir_all(&empty_hooks).expect("create empty hooks directory");
    let hooks_path = empty_hooks.to_string_lossy().into_owned();
    git_with_env(
        repo.path(),
        &["config", "core.hooksPath", &hooks_path],
        extra_env,
    );
    repo
}

fn commit_file(repo: &Path, path: &str, contents: &str, message: &str) -> String {
    commit_file_with_env(repo, path, contents, message, &[])
}

fn commit_file_with_env(
    repo: &Path,
    path: &str,
    contents: &str,
    message: &str,
    extra_env: &[(&str, &str)],
) -> String {
    let full_path = repo.join(path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(full_path, contents).expect("write fixture");
    git_with_env(repo, &["add", "--", path], extra_env);
    git_with_env(repo, &["commit", "-q", "-m", message], extra_env);
    git_with_env(repo, &["rev-parse", "HEAD"], extra_env)
}

#[test]
fn hermetic_git_extraction_drives_tier0_evaluation() {
    let repository = repository();
    let revision = commit_file(
        repository.path(),
        "docs/guide.md",
        "hello\n",
        "docs(path:docs): add guide",
    );
    let extractor = GitExtractor::default();
    let identity = extractor
        .identity_for_repository(repository.path(), "run-conf-004-hermetic")
        .expect("derive canonical repository identity");

    let extraction = extractor.extract(
        repository.path(),
        GitSelection::Commit("HEAD".into()),
        &identity,
    );
    let GitExtractionOutcome::Evaluated(extraction) = extraction else {
        panic!("hermetic repository must select HEAD: {extraction:?}");
    };
    assert_eq!(extraction.head_revision, revision);
    assert_eq!(extraction.commits.len(), 1);
    let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("hermetic conventional commit must be evaluable");
    };

    let report = evaluate_tier0(&contract_from(commit, vec![]), commit, &[]);

    assert_eq!(commit.commit_revision, revision);
    assert_eq!(commit.coverage.len(), 1);
    assert_eq!(commit.coverage[0].new_path, "docs/guide.md");
    assert_eq!(report.verdict.outcome, ConformanceOutcome::Conformant);
    assert_eq!(report.verdict.evidence_strength, EvidenceStrength::Complete);
    assert!(report.findings.is_empty());
}

fn hostile_host_gitconfig(dir: &Path) -> PathBuf {
    let hooks = dir.join("hooks");
    std::fs::create_dir_all(&hooks).expect("hostile hooks directory");
    let hook = hooks.join("pre-commit");
    std::fs::write(&hook, "#!/bin/sh\nexit 1\n").expect("failing pre-commit");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
            .expect("chmod failing hook");
    }
    let global = dir.join("gitconfig");
    std::fs::write(&global, "").expect("hostile gitconfig");
    let hooks_path = hooks.to_string_lossy().into_owned();
    let global_path = global.to_string_lossy().into_owned();
    for (key, value) in [
        ("commit.gpgsign", "true"),
        ("gpg.program", "/definitely/missing/clawopen-gpg"),
        ("core.hooksPath", hooks_path.as_str()),
    ] {
        let output = Command::new("git")
            .args(["config", "--file", &global_path, key, value])
            .output()
            .expect("write hostile gitconfig key");
        assert!(
            output.status.success(),
            "git config {key} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    global
}

#[test]
fn hermetic_git_extraction_survives_hostile_host_signing_and_hooks() {
    let host = tempfile::tempdir().expect("hostile host git directory");
    let global = hostile_host_gitconfig(host.path());
    let global_path = global.to_string_lossy().into_owned();
    let extra = [("GIT_CONFIG_GLOBAL", global_path.as_str())];

    let repository = repository_with_env(&extra);
    let revision = commit_file_with_env(
        repository.path(),
        "docs/guide.md",
        "hello\n",
        "docs(path:docs): add guide",
        &extra,
    );

    let extractor = GitExtractor::default();
    let identity = extractor
        .identity_for_repository(repository.path(), "run-conf-004-hostile-host")
        .expect("derive canonical repository identity");
    let extraction = extractor.extract(
        repository.path(),
        GitSelection::Commit("HEAD".into()),
        &identity,
    );
    let GitExtractionOutcome::Evaluated(extraction) = extraction else {
        panic!("hostile-host repository must select HEAD: {extraction:?}");
    };
    assert_eq!(extraction.head_revision, revision);
}

#[test]
fn fixture_git_commands_ignore_ambient_command_scope_config() {
    let host = tempfile::tempdir().expect("hostile command config directory");
    let _global = hostile_host_gitconfig(host.path());
    let hooks_path = host.path().join("hooks").to_string_lossy().into_owned();
    let cases = [
        (
            "GIT_CONFIG_COUNT",
            vec![
                ("GIT_CONFIG_COUNT", "1".to_owned()),
                ("GIT_CONFIG_KEY_0", "core.hooksPath".to_owned()),
                ("GIT_CONFIG_VALUE_0", hooks_path.clone()),
            ],
        ),
        (
            "GIT_CONFIG_PARAMETERS",
            vec![(
                "GIT_CONFIG_PARAMETERS",
                format!("'core.hooksPath={hooks_path}'"),
            )],
        ),
    ];

    let mut failures = Vec::new();
    for (name, environment) in cases {
        let mut child = Command::new(std::env::current_exe().expect("current test executable"));
        child
            .args([
                "--exact",
                "ambient_command_scope_git_config_child",
                "--nocapture",
            ])
            .env("ANVIL_CONF_AMBIENT_COMMAND_CONFIG", "1")
            .env_remove("GIT_CONFIG_COUNT")
            .env_remove("GIT_CONFIG_KEY_0")
            .env_remove("GIT_CONFIG_VALUE_0")
            .env_remove("GIT_CONFIG_PARAMETERS");
        for (key, value) in environment {
            child.env(key, value);
        }
        let output = child.output().expect("run isolated command-config child");
        if !output.status.success() {
            failures.push(format!(
                "{name} leaked into fixture Git commands:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "ambient command config was not isolated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn ambient_command_scope_git_config_child() {
    if std::env::var_os("ANVIL_CONF_AMBIENT_COMMAND_CONFIG").is_some() {
        let repository = repository();
        commit_file(
            repository.path(),
            "docs/guide.md",
            "hello\n",
            "docs: add guide",
        );
    }
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
