//! Deterministic intent-conformance extraction and evaluation.

pub mod evaluate;
pub mod git;
pub mod pr_body;

pub use evaluate::{
    BoundGraphDelta, CONFORMANCE_CLAIM_TABLE_VERSION, ClaimEvaluation, ConformanceEvaluation,
    evaluate_pr_declaration, evaluate_tier0,
};
pub use git::{
    ConventionalCommitEvidence, ConventionalCommitHeader, GitBudgetDiagnostics,
    GitCommitExtraction, GitCoverageMember, GitEvaluationIdentity, GitExtraction,
    GitExtractionLimits, GitExtractionOutcome, GitExtractor, GitNonEvaluation, GitSelection,
};
pub use pr_body::{
    PR_BODY_EXTRACTION_LIMITS_VERSION, PR_BODY_MAX_BYTES, PR_BODY_MAX_MEMBERS, PR_BODY_MAX_SCOPES,
    PR_BODY_MEMBER_MAX_BYTES, PR_BODY_SCOPE_MAX_BYTES, PR_BODY_SOURCE_REFERENCE_MAX_BYTES,
    PrBodyClaimExtraction, PrBodyContractParts, PrBodyExtractionOutcome, PrBodyNonEvaluation,
    extract_pr_body_claims,
};

fn digest_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::from("sha256:"), |mut output, byte| {
            use std::fmt::Write as _;
            write!(output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        })
}

fn digest_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};

    digest_hex(&Sha256::digest(bytes))
}

fn valid_path_prefix(prefix: &str) -> bool {
    !prefix.is_empty()
        && !prefix.starts_with('/')
        && !prefix.ends_with('/')
        && !prefix.contains('\\')
        && !prefix.contains('\0')
        && !prefix.contains('*')
        && !prefix.contains('?')
        && !prefix.contains('[')
        && !prefix.contains(']')
        && prefix
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}
