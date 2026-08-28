//! Deterministic intent-conformance extraction and evaluation.

pub mod evaluate;
pub mod git;

pub use evaluate::{
    BoundGraphDelta, CONFORMANCE_CLAIM_TABLE_VERSION, ClaimEvaluation, ConformanceEvaluation,
    evaluate_tier0,
};
pub use git::{
    ConventionalCommitEvidence, ConventionalCommitHeader, GitBudgetDiagnostics,
    GitCommitExtraction, GitCoverageMember, GitEvaluationIdentity, GitExtraction,
    GitExtractionLimits, GitExtractionOutcome, GitExtractor, GitNonEvaluation, GitSelection,
};
