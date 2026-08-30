use anvil_checks::conformance::{PrBodyExtractionOutcome, extract_pr_body_claims};
use anvil_kernel_types::{ClaimKind, EvidenceGrade, IntentSourceKind, IntentTier, ScopeAuthority};
use sha2::{Digest, Sha256};

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::from("sha256:"), |mut output, byte| {
            use std::fmt::Write as _;
            write!(output, "{byte:02x}").expect("write digest");
            output
        })
}

#[test]
fn extracts_the_closed_vocabulary_in_canonical_order() {
    let body = r"Human summary that is not retained.

```anvil-claims
claim: refactor-only
scope: path:crates/anvil-checks
claim: documentation-only
claim: test-only
scope: path:docs
claim: no-behaviour-change
claim: documentation-only
```

More prose that is not part of the declaration.
";
    let reference = "github:eddacraft/anvil-001#4242@body-v7";

    let extraction = extract_pr_body_claims(reference, body);

    assert_eq!(extraction.outcome(), PrBodyExtractionOutcome::Extracted);
    assert!(extraction.reasons().is_empty());
    assert_eq!(extraction.source().tier, IntentTier::Tier0);
    assert_eq!(extraction.source().kind, IntentSourceKind::PullRequest);
    assert_eq!(extraction.source().reference, reference);
    assert_eq!(extraction.source().digest, sha256(body.as_bytes()));
    assert_eq!(extraction.source().evidence_grade, EvidenceGrade::Weak);
    assert_eq!(
        extraction.source().producer_schema,
        "github.pull-request-body.anvil-claims.v1"
    );
    assert_eq!(extraction.source().producer_record_id, None);

    let claims: Vec<_> = extraction
        .claims()
        .iter()
        .map(|claim| (claim.kind, claim.value.as_str(), claim.source_index))
        .collect();
    assert_eq!(
        claims,
        vec![
            (ClaimKind::FileClass, "documentation-only", 0),
            (ClaimKind::FileClass, "test-only", 0),
            (ClaimKind::PathPrefix, "crates/anvil-checks", 0),
            (ClaimKind::PathPrefix, "docs", 0),
            (ClaimKind::GraphSemantic, "no-behaviour-change", 0),
            (ClaimKind::GraphSemantic, "refactor-only", 0),
        ]
    );

    let scopes: Vec<_> = extraction
        .declared_scopes()
        .iter()
        .map(|scope| {
            let ScopeAuthority::ExplicitPath { prefix } = &scope.authority else {
                panic!("PR-body scopes must use explicit path authority");
            };
            (scope.label.as_str(), prefix.as_str(), scope.source_index)
        })
        .collect();
    assert_eq!(
        scopes,
        vec![
            ("path:crates/anvil-checks", "crates/anvil-checks", 0),
            ("path:docs", "docs", 0),
        ]
    );

    let rendered = format!("{extraction:?}");
    assert!(!rendered.contains("Human summary"));
    assert!(!rendered.contains("More prose"));
    let admitted = extraction
        .into_contract_parts()
        .expect("complete extraction releases contract parts");
    assert_eq!(admitted.claims.len(), 6);
}

#[test]
fn text_and_other_fences_outside_the_declaration_are_ignored() {
    let body =
        "```text\nclaim: test-only\n```\n\n```anvil-claims\nclaim: documentation-only\n```\n";

    let extraction = extract_pr_body_claims("github:repo#1@body-v1", body);

    assert_eq!(extraction.outcome(), PrBodyExtractionOutcome::Extracted);
    assert_eq!(extraction.claims().len(), 1);
    assert_eq!(extraction.claims()[0].value, "documentation-only");
}

#[test]
fn a_missing_or_empty_block_is_not_evaluated() {
    let missing = extract_pr_body_claims("github:repo#1@body-v1", "No declaration.\n");
    assert_eq!(missing.outcome(), PrBodyExtractionOutcome::NotEvaluated);
    assert_eq!(missing.reasons(), ["claim.pr-body.block-missing"]);

    let empty = extract_pr_body_claims("github:repo#1@body-v2", "```anvil-claims\n\n```\n");
    assert_eq!(empty.outcome(), PrBodyExtractionOutcome::NotEvaluated);
    assert_eq!(empty.reasons(), ["claim.pr-body.block-empty"]);
}

#[test]
fn a_second_block_preserves_known_members_but_is_not_evaluated() {
    let body = "```anvil-claims\nclaim: documentation-only\n```\n\n```anvil-claims\nclaim: test-only\n```\n";

    let extraction = extract_pr_body_claims("github:repo#2@body-v1", body);

    assert_eq!(extraction.outcome(), PrBodyExtractionOutcome::NotEvaluated);
    assert_eq!(extraction.reasons(), ["claim.pr-body.multiple-blocks"]);
    assert_eq!(
        extraction
            .claims()
            .iter()
            .map(|claim| claim.value.as_str())
            .collect::<Vec<_>>(),
        ["documentation-only", "test-only"]
    );
}

#[test]
fn malformed_and_unknown_members_preserve_known_members() {
    let body = "```anvil-claims
claim: documentation-only
claim: invented
scope: path:../outside
not a declaration member
claim: invented
```
";

    let extraction = extract_pr_body_claims("github:repo#3@body-v1", body);

    assert_eq!(extraction.outcome(), PrBodyExtractionOutcome::NotEvaluated);
    assert_eq!(
        extraction.reasons(),
        [
            "claim.pr-body.claim-unknown",
            "claim.pr-body.member-malformed",
            "claim.pr-body.scope-invalid",
        ]
    );
    assert_eq!(extraction.claims().len(), 1);
    assert_eq!(extraction.claims()[0].value, "documentation-only");
    assert!(extraction.declared_scopes().is_empty());
}

#[test]
fn an_unclosed_block_preserves_known_members() {
    let extraction = extract_pr_body_claims(
        "github:repo#4@body-v1",
        "```anvil-claims\r\nclaim: test-only\r\n",
    );

    assert_eq!(extraction.outcome(), PrBodyExtractionOutcome::NotEvaluated);
    assert_eq!(extraction.reasons(), ["claim.pr-body.block-unclosed"]);
    assert_eq!(extraction.claims()[0].value, "test-only");
}

#[test]
fn a_missing_source_reference_is_not_evaluated_without_losing_claims() {
    let body = "```anvil-claims\nclaim: test-only\n```\n";

    let extraction = extract_pr_body_claims("", body);

    assert_eq!(extraction.outcome(), PrBodyExtractionOutcome::NotEvaluated);
    assert_eq!(extraction.reasons(), ["claim.pr-body.reference-missing"]);
    assert_eq!(extraction.claims()[0].value, "test-only");
    assert_eq!(extraction.source().digest, sha256(body.as_bytes()));
}

#[test]
fn the_digest_binds_exact_raw_body_bytes_while_crlf_parses_normally() {
    let lf = "```anvil-claims\nclaim: test-only\n```\n";
    let crlf = "```anvil-claims\r\nclaim: test-only\r\n```\r\n";

    let lf_extraction = extract_pr_body_claims("github:repo#5@body-v1", lf);
    let crlf_extraction = extract_pr_body_claims("github:repo#5@body-v2", crlf);

    assert_eq!(lf_extraction.claims(), crlf_extraction.claims());
    assert_ne!(
        lf_extraction.source().digest,
        crlf_extraction.source().digest
    );
    assert_eq!(crlf_extraction.source().digest, sha256(crlf.as_bytes()));
}

#[test]
fn nested_and_commented_examples_are_not_declarations() {
    for body in [
        "````markdown\n```anvil-claims\nclaim: documentation-only\n```\n````\n",
        "<!--\n```anvil-claims\nclaim: test-only\n```\n-->\n",
    ] {
        let extraction = extract_pr_body_claims("github:repo#6@body-v1", body);

        assert_eq!(extraction.outcome(), PrBodyExtractionOutcome::NotEvaluated);
        assert_eq!(extraction.reasons(), ["claim.pr-body.block-missing"]);
        assert!(extraction.claims().is_empty());
    }
}

#[test]
fn versioned_budgets_fail_honest_without_discarding_bounded_known_members() {
    let oversized_body = "x".repeat(256 * 1024 + 1);
    let body_result = extract_pr_body_claims("github:repo#7@body-v1", &oversized_body);
    assert_eq!(body_result.outcome(), PrBodyExtractionOutcome::NotEvaluated);
    assert_eq!(body_result.reasons(), ["claim.pr-body.budget.body-bytes"]);
    assert!(body_result.claims().is_empty());

    let long_reference = "r".repeat(4 * 1024 + 1);
    let reference_result =
        extract_pr_body_claims(&long_reference, "```anvil-claims\nclaim: test-only\n```\n");
    assert_eq!(
        reference_result.reasons(),
        ["claim.pr-body.budget.reference-bytes"]
    );
    assert_eq!(
        reference_result.source().reference,
        "unavailable:reference-over-limit"
    );

    let mut too_many_members = String::from("```anvil-claims\n");
    for _ in 0..257 {
        too_many_members.push_str("claim: test-only\n");
    }
    too_many_members.push_str("```\n");
    let member_result = extract_pr_body_claims("github:repo#7@body-v2", &too_many_members);
    assert_eq!(
        member_result.outcome(),
        PrBodyExtractionOutcome::NotEvaluated
    );
    assert_eq!(member_result.reasons(), ["claim.pr-body.budget.members"]);
    assert_eq!(member_result.claims()[0].value, "test-only");

    let long_member = "x".repeat(1025);
    let member_body = format!("```anvil-claims\nclaim: documentation-only\n{long_member}\n```\n");
    let member_bytes_result = extract_pr_body_claims("github:repo#7@body-v3", &member_body);
    assert_eq!(
        member_bytes_result.reasons(),
        ["claim.pr-body.budget.member-bytes"]
    );
    assert_eq!(member_bytes_result.claims()[0].value, "documentation-only");

    let long_scope = "s".repeat(513);
    let scope_body =
        format!("```anvil-claims\nclaim: documentation-only\nscope: path:{long_scope}\n```\n");
    let scope_result = extract_pr_body_claims("github:repo#7@body-v4", &scope_body);
    assert_eq!(
        scope_result.outcome(),
        PrBodyExtractionOutcome::NotEvaluated
    );
    assert_eq!(scope_result.reasons(), ["claim.pr-body.budget.scope-bytes"]);
    assert_eq!(scope_result.claims()[0].value, "documentation-only");

    let mut too_many_scopes = String::from("```anvil-claims\n");
    for index in 0..129 {
        use std::fmt::Write as _;
        writeln!(too_many_scopes, "scope: path:scope-{index:03}").expect("write scope");
    }
    too_many_scopes.push_str("```\n");
    let scope_count_result = extract_pr_body_claims("github:repo#7@body-v5", &too_many_scopes);
    assert_eq!(
        scope_count_result.reasons(),
        ["claim.pr-body.budget.scopes"]
    );
    assert_eq!(scope_count_result.claims().len(), 128);
    assert_eq!(scope_count_result.declared_scopes().len(), 128);
}

#[test]
fn a_not_evaluated_extraction_cannot_release_contract_parts() {
    let extraction = extract_pr_body_claims(
        "github:repo#8@body-v1",
        "```anvil-claims\nclaim: documentation-only\nunknown\n```\n",
    );

    let non_evaluation = extraction
        .into_contract_parts()
        .expect_err("not-evaluated extraction must not release contract parts");

    assert_eq!(non_evaluation.reasons(), ["claim.pr-body.member-malformed"]);
    assert_eq!(
        non_evaluation.understood_claims()[0].value,
        "documentation-only"
    );
}
