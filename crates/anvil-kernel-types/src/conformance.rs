//! Canonical intent-conformance contract shared by every intent source.
//!
//! The contract carries normalised intent and the evidence identities needed to
//! evaluate it. Source-specific parsing and conformance policy remain outside
//! this type-only crate.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

/// Current wire version for the canonical intent-conformance contract.
pub const CONFORMANCE_SCHEMA_VERSION: &str = "anvil.intent-conformance.v1";

/// JSON Schema for external intent-conformance producers.
#[must_use]
pub const fn conformance_json_schema() -> &'static str {
    include_str!("../schema/intent-conformance-v1.schema.json")
}

/// Richness tier of the declared intent source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentTier {
    #[serde(rename = "tier-0")]
    Tier0,
    #[serde(rename = "tier-1")]
    Tier1,
    #[serde(rename = "tier-2")]
    Tier2,
}

/// Deterministic producer that supplied an intent record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntentSourceKind {
    ConventionalCommit,
    PullRequest,
    SessionIntent,
    PlanAdapter,
}

/// Strength assigned to one intent source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceGrade {
    Strong,
    Moderate,
    Weak,
}

/// Provenance for a normalised intent source.
///
/// `producer_schema` and `producer_record_id` are the extension seam for a
/// future ILGOV record. The canonical contract references that record instead of
/// copying or forking its schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentSource {
    pub tier: IntentTier,
    pub kind: IntentSourceKind,
    pub reference: String,
    pub digest: String,
    pub evidence_grade: EvidenceGrade,
    pub producer_schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_record_id: Option<String>,
}

/// Stable identity of one evaluation and the Git objects it covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationBinding {
    pub run_id: String,
    pub repository_id: String,
    pub canonical_worktree_id: String,
    pub base_revision: String,
    pub head_revision: String,
    pub commit_revision: String,
}

/// Claim kinds understood by the canonical contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimKind {
    FileClass,
    PathPrefix,
    GraphSemantic,
}

/// One normalised claimed change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceClaim {
    pub kind: ClaimKind,
    pub value: String,
    pub source_index: u32,
}

/// Authority behind a declared scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ScopeAuthority {
    None,
    ExplicitPath {
        prefix: String,
    },
    BaseMapping {
        mapping_key: String,
        prefixes: Vec<String>,
        schema_version: u32,
        source_path: String,
        source_digest: String,
    },
}

/// A scope label and the authority, if any, that gives it path meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredScope {
    pub label: String,
    pub authority: ScopeAuthority,
    pub source_index: u32,
}

/// An acceptance assertion supplied by an intent producer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceAssertion {
    pub id: String,
    pub statement: String,
    pub source_index: u32,
}

/// Revision-bound graph evidence. A matching path alone is never sufficient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEvidenceBinding {
    pub run_id: String,
    pub path: String,
    pub revision: String,
    pub blob: String,
    pub schema_version: u32,
    pub generation: u64,
}

/// Git status represented by one canonical coverage member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GitChangeStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    TypeChanged,
}

/// Object kind represented by a raw Git mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GitObjectType {
    Absent,
    Blob,
    Tree,
    Gitlink,
    Unknown,
}

/// Why an evaluator could or could not account for a changed path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceDisposition {
    GitSufficient,
    Gv2Bound,
    Missing,
    Stale,
    Mismatched,
    Unsupported,
    PolicyChange,
}

/// Independent conformance result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConformanceOutcome {
    Conformant,
    NonConformant,
    NotEvaluated,
}

/// Completeness of the evidence used for a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceStrength {
    Complete,
    Partial,
    Absent,
}

/// One raw Git change record exactly as resolved for evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawGitChangeRecord {
    pub status: GitChangeStatus,
    pub raw_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_score: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_path: Option<String>,
    pub old_mode: String,
    pub new_mode: String,
    pub old_object_type: GitObjectType,
    pub new_object_type: GitObjectType,
    pub old_object: String,
    pub new_object: String,
}

/// One canonical path in the evaluator's closed coverage set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageMember {
    pub path: String,
    pub record_indices: Vec<u32>,
    /// True when this path changes a configuration source that contributed
    /// claim or scope authority. This axis is independent of evidence
    /// availability for other claims.
    pub policy_change: bool,
    pub disposition: EvidenceDisposition,
}

/// Canonical evidence retained for one evaluated Git commit in a range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommitEvidence {
    pub binding: EvaluationBinding,
    pub parent_revision: String,
    pub git_records: Vec<RawGitChangeRecord>,
    pub coverage: Vec<CoverageMember>,
}

/// Canonical evaluator output. Outcome and evidence strength remain separate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceVerdict {
    pub claim_table_version: u32,
    pub binding: EvaluationBinding,
    pub sources: Vec<IntentSource>,
    pub evaluated_claims: Vec<ConformanceClaim>,
    pub declared_scopes: Vec<DeclaredScope>,
    pub outcome: ConformanceOutcome,
    pub evidence_strength: EvidenceStrength,
    pub reasons: Vec<String>,
    pub git_records: Vec<RawGitChangeRecord>,
    #[serde(default)]
    pub git_commits: Vec<GitCommitEvidence>,
    pub coverage: Vec<CoverageMember>,
    pub uncovered_files: Vec<String>,
}

impl ConformanceVerdict {
    /// Validate deterministic output shape before publishing a verdict.
    pub fn validate_output_shape(&self) -> Result<(), ContractValidationError> {
        Self::validate_git_records(&self.git_records)?;
        Self::validate_coverage_members(&self.git_records, &self.coverage)?;
        self.validate_git_commits()?;
        Ok(())
    }

    fn validate_git_records(records: &[RawGitChangeRecord]) -> Result<(), ContractValidationError> {
        for record in records {
            if record.raw_status.is_empty() {
                return Err(ContractValidationError {
                    code: "git-record.raw-status-missing",
                });
            }
            if record
                .rename_score
                .is_some_and(|rename_score| rename_score > 100)
            {
                return Err(ContractValidationError {
                    code: "git-record.rename-score-invalid",
                });
            }
            for (value, code) in [
                (&record.old_mode, "git-record.old-mode-missing"),
                (&record.new_mode, "git-record.new-mode-missing"),
                (&record.old_object, "git-record.old-object-missing"),
                (&record.new_object, "git-record.new-object-missing"),
            ] {
                if value.is_empty() {
                    return Err(ContractValidationError { code });
                }
            }

            match record.status {
                GitChangeStatus::Added => validate_optional_path(
                    record.new_path.as_deref(),
                    "git-record.new-path-missing",
                )?,
                GitChangeStatus::Deleted => validate_optional_path(
                    record.old_path.as_deref(),
                    "git-record.old-path-missing",
                )?,
                GitChangeStatus::Modified | GitChangeStatus::TypeChanged => {
                    validate_optional_path(
                        record.new_path.as_deref(),
                        "git-record.new-path-missing",
                    )?;
                }
                GitChangeStatus::Renamed => {
                    validate_optional_path(
                        record.old_path.as_deref(),
                        "git-record.old-path-missing",
                    )?;
                    validate_optional_path(
                        record.new_path.as_deref(),
                        "git-record.new-path-missing",
                    )?;
                    if record.rename_score.is_none() {
                        return Err(ContractValidationError {
                            code: "git-record.rename-score-missing",
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_coverage_members(
        records: &[RawGitChangeRecord],
        coverage: &[CoverageMember],
    ) -> Result<(), ContractValidationError> {
        let mut expected = BTreeMap::<String, Vec<u32>>::new();
        for (index, record) in records.iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| ContractValidationError {
                code: "coverage.record-index-out-of-bounds",
            })?;
            let paths = match record.status {
                GitChangeStatus::Added
                | GitChangeStatus::Modified
                | GitChangeStatus::TypeChanged => [record.new_path.as_deref(), None],
                GitChangeStatus::Deleted => [record.old_path.as_deref(), None],
                GitChangeStatus::Renamed => {
                    [record.old_path.as_deref(), record.new_path.as_deref()]
                }
            };
            for path in paths.into_iter().flatten() {
                let indices = expected.entry(path.to_owned()).or_default();
                if indices.last() != Some(&index) {
                    indices.push(index);
                }
            }
        }

        if coverage
            .windows(2)
            .any(|pair| pair[0].path.as_bytes() >= pair[1].path.as_bytes())
        {
            return Err(ContractValidationError {
                code: "coverage.paths-not-canonical",
            });
        }

        for member in coverage {
            if !valid_git_path(&member.path) {
                return Err(ContractValidationError {
                    code: "coverage.path-invalid",
                });
            }
            if member.record_indices.is_empty() {
                return Err(ContractValidationError {
                    code: "coverage.record-indices-missing",
                });
            }
            if member
                .record_indices
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            {
                return Err(ContractValidationError {
                    code: "coverage.record-indices-not-canonical",
                });
            }
            let Some(expected_indices) = expected.remove(&member.path) else {
                return Err(ContractValidationError {
                    code: "coverage.path-not-backed-by-record",
                });
            };
            if member.record_indices != expected_indices {
                return Err(ContractValidationError {
                    code: "coverage.record-indices-incomplete",
                });
            }
        }
        if !expected.is_empty() {
            return Err(ContractValidationError {
                code: "coverage.record-endpoint-missing",
            });
        }

        Ok(())
    }

    fn validate_git_commits(&self) -> Result<(), ContractValidationError> {
        if self.git_commits.windows(2).any(|pair| {
            pair[0].binding.commit_revision.as_bytes() >= pair[1].binding.commit_revision.as_bytes()
        }) {
            return Err(ContractValidationError {
                code: "git-commit-evidence.not-canonical",
            });
        }

        for commit in &self.git_commits {
            if commit.parent_revision.is_empty() {
                return Err(ContractValidationError {
                    code: "git-commit-evidence.parent-revision-missing",
                });
            }
            if commit.binding.commit_revision.is_empty() {
                return Err(ContractValidationError {
                    code: "git-commit-evidence.commit-revision-missing",
                });
            }
            for (actual, expected, code) in [
                (
                    &commit.binding.run_id,
                    &self.binding.run_id,
                    "git-commit-evidence.run-id-mismatch",
                ),
                (
                    &commit.binding.repository_id,
                    &self.binding.repository_id,
                    "git-commit-evidence.repository-mismatch",
                ),
                (
                    &commit.binding.canonical_worktree_id,
                    &self.binding.canonical_worktree_id,
                    "git-commit-evidence.worktree-mismatch",
                ),
                (
                    &commit.binding.base_revision,
                    &self.binding.base_revision,
                    "git-commit-evidence.base-revision-mismatch",
                ),
                (
                    &commit.binding.head_revision,
                    &self.binding.head_revision,
                    "git-commit-evidence.head-revision-mismatch",
                ),
            ] {
                if actual != expected {
                    return Err(ContractValidationError { code });
                }
            }
            Self::validate_git_records(&commit.git_records)?;
            Self::validate_coverage_members(&commit.git_records, &commit.coverage)?;
        }

        Ok(())
    }
}

/// Structural defect in a canonical conformance contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractValidationError {
    code: &'static str,
}

impl ContractValidationError {
    /// Stable machine-readable reason.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ContractValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for ContractValidationError {}

/// Canonical normalised input consumed by conformance evaluators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceContract {
    pub schema_version: String,
    pub binding: EvaluationBinding,
    pub sources: Vec<IntentSource>,
    pub declared_scopes: Vec<DeclaredScope>,
    pub claimed_changes: Vec<ConformanceClaim>,
    pub acceptance_assertions: Vec<AcceptanceAssertion>,
    pub graph_evidence: Vec<GraphEvidenceBinding>,
}

impl ConformanceContract {
    /// Validate cross-record bindings before an evaluator trusts the contract.
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.schema_version != CONFORMANCE_SCHEMA_VERSION {
            return Err(ContractValidationError {
                code: "schema.unsupported",
            });
        }
        self.validate_binding()?;
        self.validate_sources()?;
        self.validate_scopes()?;
        self.validate_source_indices()?;
        self.validate_graph_evidence()?;
        Ok(())
    }

    fn validate_binding(&self) -> Result<(), ContractValidationError> {
        for (value, code) in [
            (&self.binding.run_id, "binding.run-id-missing"),
            (&self.binding.repository_id, "binding.repository-id-missing"),
            (
                &self.binding.canonical_worktree_id,
                "binding.canonical-worktree-id-missing",
            ),
            (&self.binding.base_revision, "binding.base-revision-missing"),
            (&self.binding.head_revision, "binding.head-revision-missing"),
            (
                &self.binding.commit_revision,
                "binding.commit-revision-missing",
            ),
        ] {
            if value.is_empty() {
                return Err(ContractValidationError { code });
            }
        }
        Ok(())
    }

    fn validate_sources(&self) -> Result<(), ContractValidationError> {
        if self.sources.is_empty() {
            return Err(ContractValidationError {
                code: "source.none",
            });
        }
        for source in &self.sources {
            for (value, code) in [
                (&source.reference, "source.reference-missing"),
                (&source.digest, "source.digest-missing"),
                (&source.producer_schema, "source.producer-schema-missing"),
            ] {
                if value.is_empty() {
                    return Err(ContractValidationError { code });
                }
            }
        }
        Ok(())
    }

    fn validate_scopes(&self) -> Result<(), ContractValidationError> {
        for scope in &self.declared_scopes {
            if scope.label.is_empty() {
                return Err(ContractValidationError {
                    code: "scope.label-missing",
                });
            }
            match &scope.authority {
                ScopeAuthority::None => {}
                ScopeAuthority::ExplicitPath { prefix } => {
                    if !valid_path_prefix(prefix) {
                        return Err(ContractValidationError {
                            code: "scope.path-prefix-invalid",
                        });
                    }
                }
                ScopeAuthority::BaseMapping {
                    mapping_key,
                    prefixes,
                    schema_version,
                    source_path,
                    source_digest,
                } => {
                    if mapping_key.is_empty()
                        || mapping_key != &scope.label
                        || *schema_version == 0
                        || source_path.is_empty()
                        || source_digest.is_empty()
                    {
                        return Err(ContractValidationError {
                            code: "scope.mapping-authority-invalid",
                        });
                    }
                    if prefixes.is_empty()
                        || prefixes.iter().any(|prefix| !valid_path_prefix(prefix))
                        || prefixes
                            .windows(2)
                            .any(|pair| pair[0].as_bytes() >= pair[1].as_bytes())
                    {
                        return Err(ContractValidationError {
                            code: "scope.mapping-prefixes-not-canonical",
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_source_indices(&self) -> Result<(), ContractValidationError> {
        let source_count = self.sources.len();
        let source_index_valid =
            |index: u32| usize::try_from(index).is_ok_and(|index| index < source_count);
        if self
            .declared_scopes
            .iter()
            .any(|scope| !source_index_valid(scope.source_index))
            || self
                .claimed_changes
                .iter()
                .any(|claim| !source_index_valid(claim.source_index))
            || self
                .acceptance_assertions
                .iter()
                .any(|assertion| !source_index_valid(assertion.source_index))
        {
            return Err(ContractValidationError {
                code: "source.index-out-of-bounds",
            });
        }
        Ok(())
    }

    fn validate_graph_evidence(&self) -> Result<(), ContractValidationError> {
        for evidence in &self.graph_evidence {
            if evidence.path.is_empty() {
                return Err(ContractValidationError {
                    code: "binding.graph-path-missing",
                });
            }
            if evidence.blob.is_empty() {
                return Err(ContractValidationError {
                    code: "binding.graph-blob-missing",
                });
            }
            if evidence.schema_version == 0 {
                return Err(ContractValidationError {
                    code: "binding.graph-schema-missing",
                });
            }
            if evidence.generation == 0 {
                return Err(ContractValidationError {
                    code: "binding.graph-generation-missing",
                });
            }
            if evidence.run_id != self.binding.run_id {
                return Err(ContractValidationError {
                    code: "binding.graph-run-mismatch",
                });
            }
            if evidence.revision != self.binding.commit_revision {
                return Err(ContractValidationError {
                    code: "binding.graph-revision-mismatch",
                });
            }
        }

        Ok(())
    }
}

fn valid_path_prefix(prefix: &str) -> bool {
    !prefix.is_empty()
        && !prefix.starts_with('/')
        && !prefix.ends_with('/')
        && !prefix.contains(['\\', '\0', '*', '?', '[', ']'])
        && prefix
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn valid_git_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.ends_with('/')
        && !path.contains('\0')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn validate_optional_path(
    path: Option<&str>,
    missing_code: &'static str,
) -> Result<(), ContractValidationError> {
    match path {
        Some(path) if valid_git_path(path) => Ok(()),
        Some(_) => Err(ContractValidationError {
            code: "git-record.path-invalid",
        }),
        None => Err(ContractValidationError { code: missing_code }),
    }
}
