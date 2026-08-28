use anvil_kernel_types::conformance::RawGitChangeRecord;
use anvil_kernel_types::{
    AcceptanceAssertion, CONFORMANCE_SCHEMA_VERSION, ClaimKind, ConformanceClaim,
    ConformanceContract, ConformanceOutcome, ConformanceVerdict, CoverageMember, DeclaredScope,
    EvaluationBinding, EvidenceDisposition, EvidenceGrade, EvidenceStrength, GitChangeStatus,
    GitObjectType, GraphEvidenceBinding, IntentSource, IntentSourceKind, IntentTier,
    ScopeAuthority, conformance_json_schema,
};

#[test]
fn canonical_contract_round_trips_with_all_binding_axes() {
    let contract = ConformanceContract {
        schema_version: CONFORMANCE_SCHEMA_VERSION.to_string(),
        binding: EvaluationBinding {
            run_id: "run-01".into(),
            repository_id: "repo-01".into(),
            canonical_worktree_id: "worktree-01".into(),
            base_revision: "a".repeat(40),
            head_revision: "b".repeat(40),
            commit_revision: "b".repeat(40),
        },
        sources: vec![IntentSource {
            tier: IntentTier::Tier0,
            kind: IntentSourceKind::ConventionalCommit,
            reference: "git:commit:bbbb".into(),
            digest: "sha256:message".into(),
            evidence_grade: EvidenceGrade::Strong,
            producer_schema: "conventional-commits.v1".into(),
            producer_record_id: None,
        }],
        declared_scopes: vec![DeclaredScope {
            label: "path:src".into(),
            authority: ScopeAuthority::ExplicitPath {
                prefix: "src".into(),
            },
            source_index: 0,
        }],
        claimed_changes: vec![ConformanceClaim {
            kind: ClaimKind::PathPrefix,
            value: "src".into(),
            source_index: 0,
        }],
        acceptance_assertions: vec![AcceptanceAssertion {
            id: "accept-1".into(),
            statement: "all changed paths remain under src".into(),
            source_index: 0,
        }],
        graph_evidence: vec![GraphEvidenceBinding {
            run_id: "run-01".into(),
            path: "src/lib.rs".into(),
            revision: "b".repeat(40),
            blob: "c".repeat(40),
            schema_version: 1,
            generation: 7,
        }],
    };

    let value = serde_json::to_value(&contract).expect("serialise contract");
    assert_eq!(value["schema_version"], CONFORMANCE_SCHEMA_VERSION);
    assert_eq!(value["sources"][0]["tier"], "tier-0");
    assert_eq!(value["sources"][0]["kind"], "conventional-commit");
    assert_eq!(
        value["declared_scopes"][0]["authority"]["kind"],
        "explicit-path"
    );
    assert_eq!(value["graph_evidence"][0]["generation"], 7);

    let decoded: ConformanceContract = serde_json::from_value(value).expect("deserialise contract");
    assert_eq!(decoded, contract);
}

#[test]
fn contract_validation_rejects_graph_evidence_from_another_run_or_revision() {
    let mut contract = sample_contract();
    contract.graph_evidence[0].run_id = "other-run".into();
    let error = contract.validate().expect_err("mismatched run must fail");
    assert_eq!(error.code(), "binding.graph-run-mismatch");

    contract.graph_evidence[0].run_id = contract.binding.run_id.clone();
    contract.graph_evidence[0].revision = "d".repeat(40);
    let error = contract
        .validate()
        .expect_err("mismatched revision must fail");
    assert_eq!(error.code(), "binding.graph-revision-mismatch");
}

#[test]
fn verdict_wire_shape_keeps_outcome_separate_from_evidence_strength() {
    let verdict = ConformanceVerdict {
        claim_table_version: 1,
        binding: sample_contract().binding,
        sources: sample_contract().sources,
        evaluated_claims: vec![],
        declared_scopes: vec![],
        outcome: ConformanceOutcome::NonConformant,
        evidence_strength: EvidenceStrength::Partial,
        reasons: vec!["claim.path.outside-scope".into()],
        git_records: vec![RawGitChangeRecord {
            status: GitChangeStatus::Modified,
            raw_status: "M".into(),
            rename_score: None,
            old_path: None,
            new_path: Some("src/lib.rs".into()),
            old_mode: "100644".into(),
            new_mode: "100644".into(),
            old_object_type: GitObjectType::Blob,
            new_object_type: GitObjectType::Blob,
            old_object: "a".repeat(40),
            new_object: "b".repeat(40),
        }],
        coverage: vec![CoverageMember {
            path: "src/lib.rs".into(),
            record_indices: vec![0],
            policy_change: false,
            disposition: EvidenceDisposition::Missing,
        }],
        uncovered_files: vec!["src/lib.rs".into()],
    };

    let value = serde_json::to_value(&verdict).expect("serialise verdict");
    assert_eq!(value["outcome"], "non-conformant");
    assert_eq!(value["claim_table_version"], 1);
    assert_eq!(value["binding"]["run_id"], "run-01");
    assert_eq!(value["sources"][0]["kind"], "conventional-commit");
    assert_eq!(value["evidence_strength"], "partial");
    assert_eq!(value["coverage"][0]["disposition"], "missing");
}

#[test]
fn verdict_wire_shape_separates_raw_git_records_from_per_path_coverage() {
    let contract = sample_contract();
    let verdict = ConformanceVerdict {
        claim_table_version: 1,
        binding: contract.binding,
        sources: contract.sources,
        evaluated_claims: vec![],
        declared_scopes: vec![],
        outcome: ConformanceOutcome::NotEvaluated,
        evidence_strength: EvidenceStrength::Partial,
        reasons: vec!["coverage.rename-endpoint-missing".into()],
        git_records: vec![RawGitChangeRecord {
            status: GitChangeStatus::Renamed,
            raw_status: "R087".into(),
            rename_score: Some(87),
            old_path: Some("src/old.rs".into()),
            new_path: Some("src/new.rs".into()),
            old_mode: "100644".into(),
            new_mode: "100644".into(),
            old_object_type: GitObjectType::Blob,
            new_object_type: GitObjectType::Blob,
            old_object: "a".repeat(40),
            new_object: "b".repeat(40),
        }],
        coverage: vec![
            CoverageMember {
                path: "src/new.rs".into(),
                record_indices: vec![0],
                policy_change: false,
                disposition: EvidenceDisposition::Missing,
            },
            CoverageMember {
                path: "src/old.rs".into(),
                record_indices: vec![0],
                policy_change: false,
                disposition: EvidenceDisposition::GitSufficient,
            },
        ],
        uncovered_files: vec!["src/new.rs".into()],
    };

    verdict
        .validate_output_shape()
        .expect("canonical verdict output");

    let mut missing_rename_endpoint = verdict.clone();
    missing_rename_endpoint.coverage.remove(0);
    assert_eq!(
        missing_rename_endpoint
            .validate_output_shape()
            .expect_err("every rename endpoint must appear in coverage")
            .code(),
        "coverage.record-endpoint-missing"
    );

    let value = serde_json::to_value(&verdict).expect("serialise verdict");
    assert_eq!(value["git_records"][0]["raw_status"], "R087");
    assert_eq!(value["git_records"][0]["old_path"], "src/old.rs");
    assert_eq!(value["git_records"][0]["new_path"], "src/new.rs");

    let coverage = value["coverage"].as_array().expect("coverage array");
    assert_eq!(coverage.len(), 2);
    assert_eq!(coverage[0]["path"], "src/new.rs");
    assert_eq!(coverage[1]["path"], "src/old.rs");
    assert_eq!(coverage[0]["record_indices"], serde_json::json!([0]));
    assert_eq!(coverage[1]["record_indices"], serde_json::json!([0]));
    assert_eq!(coverage[0]["disposition"], "missing");
    assert_eq!(coverage[1]["disposition"], "git-sufficient");
    assert!(coverage[0].get("raw_status").is_none());
    assert!(coverage[0].get("old_path").is_none());
    assert!(coverage[0].get("new_path").is_none());
}

#[test]
fn verdict_output_validation_requires_backed_canonical_coverage_references() {
    let contract = sample_contract();
    let verdict = ConformanceVerdict {
        claim_table_version: 1,
        binding: contract.binding,
        sources: contract.sources,
        evaluated_claims: vec![],
        declared_scopes: vec![],
        outcome: ConformanceOutcome::Conformant,
        evidence_strength: EvidenceStrength::Complete,
        reasons: vec![],
        git_records: vec![
            raw_modified_record("src/lib.rs", 'a', 'b'),
            raw_modified_record("src/lib.rs", 'b', 'c'),
        ],
        coverage: vec![CoverageMember {
            path: "src/lib.rs".into(),
            record_indices: vec![0, 1],
            policy_change: false,
            disposition: EvidenceDisposition::GitSufficient,
        }],
        uncovered_files: vec![],
    };

    verdict
        .validate_output_shape()
        .expect("deduplicated canonical coverage is valid");

    let value = serde_json::to_value(&verdict).expect("serialise verdict");
    assert_eq!(value["coverage"].as_array().expect("coverage").len(), 1);
    assert_eq!(value["coverage"][0]["path"], "src/lib.rs");
    assert_eq!(
        value["coverage"][0]["record_indices"],
        serde_json::json!([0, 1])
    );

    let mut unbacked = verdict.clone();
    unbacked.coverage[0].path = "src/other.rs".into();
    assert_eq!(
        unbacked
            .validate_output_shape()
            .expect_err("coverage path must be backed by referenced records")
            .code(),
        "coverage.path-not-backed-by-record"
    );

    let mut incomplete_indices = verdict.clone();
    incomplete_indices.coverage[0].record_indices = vec![0];
    assert_eq!(
        incomplete_indices
            .validate_output_shape()
            .expect_err("coverage must name every backing raw record")
            .code(),
        "coverage.record-indices-incomplete"
    );

    let mut omitted_record = verdict.clone();
    omitted_record
        .git_records
        .push(raw_modified_record("src/other.rs", 'c', 'd'));
    assert_eq!(
        omitted_record
            .validate_output_shape()
            .expect_err("every ordinary record path must appear in coverage")
            .code(),
        "coverage.record-endpoint-missing"
    );

    let mut duplicate = verdict.clone();
    duplicate.coverage.push(CoverageMember {
        path: "src/lib.rs".into(),
        record_indices: vec![1],
        policy_change: false,
        disposition: EvidenceDisposition::Missing,
    });
    assert_eq!(
        duplicate
            .validate_output_shape()
            .expect_err("duplicate coverage paths are not canonical")
            .code(),
        "coverage.paths-not-canonical"
    );
}

#[test]
fn verdict_output_accepts_legal_git_path_metacharacters() {
    let contract = sample_contract();
    let path = r"docs/[guide]*?.md\draft";
    let verdict = ConformanceVerdict {
        claim_table_version: 1,
        binding: contract.binding,
        sources: contract.sources,
        evaluated_claims: vec![],
        declared_scopes: vec![],
        outcome: ConformanceOutcome::Conformant,
        evidence_strength: EvidenceStrength::Complete,
        reasons: vec![],
        git_records: vec![raw_modified_record(path, 'a', 'b')],
        coverage: vec![CoverageMember {
            path: path.into(),
            record_indices: vec![0],
            policy_change: false,
            disposition: EvidenceDisposition::GitSufficient,
        }],
        uncovered_files: vec![],
    };

    verdict
        .validate_output_shape()
        .expect("legal repository-relative Git paths remain valid evidence");
}

#[test]
fn external_producer_schema_names_the_contract_and_binding_axes() {
    let schema: serde_json::Value =
        serde_json::from_str(conformance_json_schema()).expect("valid JSON schema");
    assert_eq!(schema["$id"], CONFORMANCE_SCHEMA_VERSION);
    assert_eq!(schema["title"], "ConformanceContract");
    let binding_required = schema["$defs"]["EvaluationBinding"]["required"]
        .as_array()
        .expect("binding required fields");
    for field in [
        "run_id",
        "repository_id",
        "canonical_worktree_id",
        "base_revision",
        "head_revision",
        "commit_revision",
    ] {
        assert!(binding_required.iter().any(|value| value == field));
    }
}

#[test]
fn contract_validation_rejects_absent_required_bindings_and_provenance() {
    let mut contract = sample_contract();
    contract.sources.clear();
    assert_eq!(
        contract
            .validate()
            .expect_err("a source is required")
            .code(),
        "source.none"
    );

    let mut contract = sample_contract();
    contract.binding.repository_id.clear();
    assert_eq!(
        contract
            .validate()
            .expect_err("repository id is required")
            .code(),
        "binding.repository-id-missing"
    );

    let mut contract = sample_contract();
    contract.graph_evidence[0].blob.clear();
    assert_eq!(
        contract
            .validate()
            .expect_err("graph blob is required")
            .code(),
        "binding.graph-blob-missing"
    );

    let mut contract = sample_contract();
    contract.sources[0].digest.clear();
    assert_eq!(
        contract
            .validate()
            .expect_err("source digest is required")
            .code(),
        "source.digest-missing"
    );
}

#[test]
fn contract_validation_rejects_unversioned_or_non_canonical_scope_authority() {
    let mut contract = sample_contract();
    contract.declared_scopes = vec![DeclaredScope {
        label: "core".into(),
        authority: ScopeAuthority::BaseMapping {
            mapping_key: "core".into(),
            prefixes: vec!["src/z".into(), "src/a".into()],
            schema_version: 1,
            source_path: ".anvil.yaml".into(),
            source_digest: "sha256:config".into(),
        },
        source_index: 0,
    }];
    assert_eq!(
        contract
            .validate()
            .expect_err("prefixes must be sorted")
            .code(),
        "scope.mapping-prefixes-not-canonical"
    );

    let mut contract = sample_contract();
    contract.declared_scopes = vec![DeclaredScope {
        label: "path:../src".into(),
        authority: ScopeAuthority::ExplicitPath {
            prefix: "../src".into(),
        },
        source_index: 0,
    }];
    assert_eq!(
        contract
            .validate()
            .expect_err("path prefix must be valid")
            .code(),
        "scope.path-prefix-invalid"
    );
}

fn sample_contract() -> ConformanceContract {
    let value = serde_json::json!({
        "schema_version": CONFORMANCE_SCHEMA_VERSION,
        "binding": {
            "run_id": "run-01",
            "repository_id": "repo-01",
            "canonical_worktree_id": "worktree-01",
            "base_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "head_revision": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "commit_revision": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        },
        "sources": [{"tier":"tier-0","kind":"conventional-commit","reference":"git:commit:bbbb","digest":"sha256:message","evidence_grade":"strong","producer_schema":"conventional-commits.v1"}],
        "declared_scopes": [],
        "claimed_changes": [],
        "acceptance_assertions": [],
        "graph_evidence": [{"run_id":"run-01","path":"src/lib.rs","revision":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","blob":"cccccccccccccccccccccccccccccccccccccccc","schema_version":1,"generation":7}]
    });
    serde_json::from_value(value).expect("sample contract")
}

fn raw_modified_record(path: &str, old_object: char, new_object: char) -> RawGitChangeRecord {
    RawGitChangeRecord {
        status: GitChangeStatus::Modified,
        raw_status: "M".into(),
        rename_score: None,
        old_path: None,
        new_path: Some(path.into()),
        old_mode: "100644".into(),
        new_mode: "100644".into(),
        old_object_type: GitObjectType::Blob,
        new_object_type: GitObjectType::Blob,
        old_object: old_object.to_string().repeat(40),
        new_object: new_object.to_string().repeat(40),
    }
}
