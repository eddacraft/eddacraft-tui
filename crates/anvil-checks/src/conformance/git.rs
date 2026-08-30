//! Bounded Tier-0 extraction from Git commit objects.

use super::{digest_bytes, digest_hex, valid_path_prefix};
use anvil_config::{DISCOVER_PRECEDENCE, MAX_CONFIG_FILE_BYTES, canonical_json_bytes, parse_str};
use anvil_kernel_types::{
    ClaimKind, ConformanceClaim, DeclaredScope, EvaluationBinding, EvidenceGrade, GitChangeStatus,
    GitObjectType, IntentSource, IntentSourceKind, IntentTier, ScopeAuthority,
};
use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const CAPTURE_DRAIN_TIMEOUT: Duration = Duration::from_millis(250);
static CONVENTIONAL_COMMIT_HEADER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([a-z][a-z0-9-]*)(?:\(([^()\r\n]+)\))?(!)?: ([^\s].*)$")
        .expect("constant Conventional Commit regex")
});

/// Versioned deterministic and operational extraction limits from ADR-134.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitExtractionLimits {
    pub max_commits: usize,
    pub max_records_per_commit: usize,
    pub max_raw_bytes_per_commit: usize,
    pub max_decoded_bytes_per_commit: usize,
    pub max_rename_sources: usize,
    pub max_rename_targets: usize,
    pub git_timeout: Duration,
    pub run_timeout: Duration,
}

impl Default for GitExtractionLimits {
    fn default() -> Self {
        Self {
            max_commits: 10_000,
            max_records_per_commit: 100_000,
            max_raw_bytes_per_commit: 64 * 1024 * 1024,
            max_decoded_bytes_per_commit: 64 * 1024 * 1024,
            max_rename_sources: 1_000,
            max_rename_targets: 1_000,
            git_timeout: Duration::from_secs(30),
            run_timeout: Duration::from_mins(5),
        }
    }
}

/// One commit or an ancestry-checked exclusive/inclusive range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitSelection {
    Commit(String),
    Range { base: String, head: String },
}

/// Stable caller-owned identities for one evaluation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitEvaluationIdentity {
    pub run_id: String,
    pub repository_id: String,
    pub canonical_worktree_id: String,
    /// Monotonic start retained by the caller across all stages of this evaluation run.
    pub run_started: Instant,
}

/// One unclassified raw Git coverage record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCoverageMember {
    pub status: GitChangeStatus,
    pub raw_status: String,
    pub rename_score: Option<u16>,
    pub old_path: Option<String>,
    pub new_path: String,
    pub old_mode: String,
    pub new_mode: String,
    pub old_object_type: GitObjectType,
    pub new_object_type: GitObjectType,
    pub old_object: String,
    pub new_object: String,
}

/// Stable parsed Conventional Commit header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionalCommitHeader {
    pub commit_type: String,
    pub scope: Option<String>,
    pub breaking: bool,
    pub description: String,
}

/// Canonical Tier-0 evidence extracted for one commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionalCommitEvidence {
    pub commit_revision: String,
    pub parent_revision: String,
    pub binding: EvaluationBinding,
    pub header: ConventionalCommitHeader,
    pub source: IntentSource,
    pub declared_scope: Option<DeclaredScope>,
    pub claims: Vec<ConformanceClaim>,
    pub coverage: Vec<GitCoverageMember>,
    pub contributing_base_config_paths: Vec<String>,
}

/// Per-commit extraction result retained inside a selected range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitCommitExtraction {
    Evaluated(Box<ConventionalCommitEvidence>),
    NotEvaluated(GitNonEvaluation),
}

/// Evaluated Git selection and its per-commit evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitExtraction {
    pub base_revision: String,
    pub head_revision: String,
    pub commits: Vec<GitCommitExtraction>,
}

/// Claim-agnostic bounded Git footprint for one commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFootprintCommitEvidence {
    pub commit_revision: String,
    pub parent_revision: String,
    pub binding: EvaluationBinding,
    pub coverage: Vec<GitCoverageMember>,
}

/// Per-commit footprint result retained inside a selected range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitFootprintCommitExtraction {
    Evaluated(Box<GitFootprintCommitEvidence>),
    NotEvaluated(GitNonEvaluation),
}

/// Evaluated Git selection containing claim-agnostic per-commit footprints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFootprintExtraction {
    pub base_revision: String,
    pub head_revision: String,
    pub commits: Vec<GitFootprintCommitExtraction>,
}

/// Structured counters retained when a bounded extraction cannot be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBudgetDiagnostics {
    pub configured_limit: usize,
    pub elapsed_millis: Option<usize>,
    pub commits: Option<usize>,
    pub records: Option<usize>,
    pub rename_sources: Option<usize>,
    pub rename_targets: Option<usize>,
    pub raw_bytes: Option<usize>,
    pub decoded_bytes: Option<usize>,
    pub raw_output_digest: Option<Box<str>>,
}

/// Reason-coded fail-honest extraction result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitNonEvaluation {
    pub commit_revision: Option<Box<str>>,
    pub reason: &'static str,
    pub stage: &'static str,
    pub observed: usize,
    pub limit: Option<usize>,
    pub detail: Box<str>,
    pub raw_digest: Option<Box<str>>,
    pub budget: Option<Box<GitBudgetDiagnostics>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BaseScopeMappings {
    schema_version: u32,
    source_path: String,
    source_digest: String,
    mappings: BTreeMap<String, Vec<String>>,
}

struct ExtractedCoverage {
    members: Vec<GitCoverageMember>,
    decoded_path_bytes: usize,
    raw_digest: String,
    raw_bytes: usize,
    rename_sources: usize,
    rename_targets: usize,
}

struct PreparedCommit {
    revision: String,
    known_parent: Option<String>,
}

struct PreparedExtraction {
    repository: PathBuf,
    empty_config: EmptyGlobalConfig,
    started: Instant,
    base_revision: String,
    head_revision: String,
    commits: Vec<PreparedCommit>,
}

/// Extraction outcome; absence or ambiguity never becomes conformance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitExtractionOutcome {
    Evaluated(GitExtraction),
    NotEvaluated(GitNonEvaluation),
}

/// Claim-agnostic footprint extraction outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitFootprintExtractionOutcome {
    Evaluated(GitFootprintExtraction),
    NotEvaluated(GitNonEvaluation),
}

/// Bounded, replacement-disabled Git extractor.
#[derive(Debug, Clone)]
pub struct GitExtractor {
    limits: GitExtractionLimits,
    git_program: Result<PathBuf, GitNonEvaluation>,
}

impl Default for GitExtractor {
    fn default() -> Self {
        Self {
            limits: GitExtractionLimits::default(),
            git_program: resolve_git_program(),
        }
    }
}

impl GitExtractor {
    /// Construct an extractor with explicit limits.
    #[must_use]
    pub fn with_limits(limits: GitExtractionLimits) -> Self {
        Self {
            limits,
            git_program: resolve_git_program(),
        }
    }

    /// Derive opaque repository and worktree identities under the closed Git environment.
    pub fn identity_for_repository(
        &self,
        repository: &Path,
        run_id: impl Into<String>,
    ) -> Result<GitEvaluationIdentity, GitNonEvaluation> {
        let started = Instant::now();
        let repository = repository
            .canonicalize()
            .map_err(|error| not_evaluated("repository.invalid", "identity", error.to_string()))?;
        let empty_config = EmptyGlobalConfig::create()?;
        let repository = self.resolve_canonical_worktree(&repository, &empty_config, started)?;
        self.reject_replacement_state(&repository, &empty_config, started)?;
        self.derive_repository_identity(&repository, &empty_config, run_id.into(), started)
    }

    /// Extract one commit or range from a canonical repository/worktree path.
    #[must_use]
    pub fn extract(
        &self,
        repository: &Path,
        selection: GitSelection,
        identity: &GitEvaluationIdentity,
    ) -> GitExtractionOutcome {
        match self.extract_inner(repository, selection, identity) {
            Ok(extraction) => GitExtractionOutcome::Evaluated(extraction),
            Err(non_evaluation) => GitExtractionOutcome::NotEvaluated(non_evaluation),
        }
    }

    /// Extract a claim-agnostic bounded footprint for one commit or range.
    #[must_use]
    pub fn extract_footprint(
        &self,
        repository: &Path,
        selection: GitSelection,
        identity: &GitEvaluationIdentity,
    ) -> GitFootprintExtractionOutcome {
        match self.extract_footprint_inner(repository, selection, identity) {
            Ok(extraction) => GitFootprintExtractionOutcome::Evaluated(extraction),
            Err(non_evaluation) => GitFootprintExtractionOutcome::NotEvaluated(non_evaluation),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn extract_inner(
        &self,
        repository: &Path,
        selection: GitSelection,
        identity: &GitEvaluationIdentity,
    ) -> Result<GitExtraction, GitNonEvaluation> {
        let prepared = self.prepare_extraction(repository, selection, identity)?;
        let scope_mappings = self.load_base_scope_mappings(
            &prepared.repository,
            &prepared.empty_config,
            &prepared.base_revision,
            prepared.started,
        );
        let mut commits = Vec::with_capacity(prepared.commits.len());
        for prepared_commit in prepared.commits {
            let commit = prepared_commit.revision;
            let result = match &scope_mappings {
                Ok(scope_mappings) => prepared_commit
                    .known_parent
                    .map_or_else(
                        || {
                            self.first_parent_or_empty_tree(
                                &prepared.repository,
                                &prepared.empty_config,
                                &commit,
                                prepared.started,
                            )
                        },
                        Ok,
                    )
                    .and_then(|parent| {
                        self.extract_commit(
                            &prepared.repository,
                            &prepared.empty_config,
                            &commit,
                            &parent,
                            &prepared.base_revision,
                            &prepared.head_revision,
                            identity,
                            scope_mappings.as_ref(),
                            prepared.started,
                        )
                    }),
                Err(failure) => Err(failure.clone()),
            };
            commits.push(retain_commit_result(result, &commit));
        }
        Ok(GitExtraction {
            base_revision: prepared.base_revision,
            head_revision: prepared.head_revision,
            commits,
        })
    }

    fn extract_footprint_inner(
        &self,
        repository: &Path,
        selection: GitSelection,
        identity: &GitEvaluationIdentity,
    ) -> Result<GitFootprintExtraction, GitNonEvaluation> {
        let prepared = self.prepare_extraction(repository, selection, identity)?;
        let mut commits = Vec::with_capacity(prepared.commits.len());
        for prepared_commit in prepared.commits {
            let commit = prepared_commit.revision;
            let result = prepared_commit
                .known_parent
                .map_or_else(
                    || {
                        self.first_parent_or_empty_tree(
                            &prepared.repository,
                            &prepared.empty_config,
                            &commit,
                            prepared.started,
                        )
                    },
                    Ok,
                )
                .and_then(|parent| {
                    self.extract_footprint_commit(
                        &prepared.repository,
                        &prepared.empty_config,
                        &commit,
                        &parent,
                        &prepared.base_revision,
                        &prepared.head_revision,
                        identity,
                        prepared.started,
                    )
                });
            commits.push(retain_footprint_result(result, &commit));
        }
        Ok(GitFootprintExtraction {
            base_revision: prepared.base_revision,
            head_revision: prepared.head_revision,
            commits,
        })
    }

    #[allow(clippy::too_many_lines)]
    fn prepare_extraction(
        &self,
        repository: &Path,
        selection: GitSelection,
        identity: &GitEvaluationIdentity,
    ) -> Result<PreparedExtraction, GitNonEvaluation> {
        let started = identity.run_started;
        let repository = repository.canonicalize().map_err(|error| {
            not_evaluated("repository.invalid", "repository", error.to_string())
        })?;
        let empty_config = EmptyGlobalConfig::create()?;
        let repository = self.resolve_canonical_worktree(&repository, &empty_config, started)?;
        self.reject_replacement_state(&repository, &empty_config, started)?;
        let verified_identity = self.derive_repository_identity(
            &repository,
            &empty_config,
            identity.run_id.clone(),
            started,
        )?;
        if verified_identity.repository_id != identity.repository_id {
            return Err(not_evaluated(
                "identity.repository-mismatch",
                "identity",
                "identity was derived for a different canonical repository".to_owned(),
            ));
        }
        if verified_identity.canonical_worktree_id != identity.canonical_worktree_id {
            return Err(not_evaluated(
                "identity.worktree-mismatch",
                "identity",
                "identity was derived for a different canonical worktree".to_owned(),
            ));
        }

        match selection {
            GitSelection::Commit(input) => {
                let commit = self.resolve_commit(&repository, &empty_config, &input, started)?;
                let parent =
                    self.first_parent_or_empty_tree(&repository, &empty_config, &commit, started)?;
                Ok(PreparedExtraction {
                    repository,
                    empty_config,
                    started,
                    base_revision: parent.clone(),
                    head_revision: commit.clone(),
                    commits: vec![PreparedCommit {
                        revision: commit,
                        known_parent: Some(parent),
                    }],
                })
            }
            GitSelection::Range { base, head } => {
                let base = self.resolve_commit(&repository, &empty_config, &base, started)?;
                let head = self.resolve_commit(&repository, &empty_config, &head, started)?;
                self.run_git(
                    &repository,
                    &empty_config,
                    &["merge-base", "--is-ancestor", &base, &head],
                    started,
                    "ancestry",
                    1024,
                    "git.output-bytes",
                )
                .map_err(|mut failure| {
                    if !failure.reason.starts_with("budget.") {
                        failure.reason = "revision.not-ancestor";
                        failure.stage = "selection";
                    }
                    failure
                })?;
                let range = format!("{base}..{head}");
                let revision_limit = self
                    .limits
                    .max_commits
                    .saturating_add(1)
                    .saturating_mul(129);
                let output = self.run_git(
                    &repository,
                    &empty_config,
                    &["rev-list", &range],
                    started,
                    "revision-list",
                    revision_limit,
                    "budget.commits",
                )?;
                let revision_digest = digest_bytes(&output);
                let revision_raw_bytes = output.len();
                let text = utf8_trimmed(output, "revision.list-invalid")?;
                let mut revisions: Vec<String> = text
                    .lines()
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect();
                if revisions.len() > self.limits.max_commits {
                    let mut failure = over_budget(
                        "budget.commits",
                        "revision-list",
                        revisions.len(),
                        self.limits.max_commits,
                    );
                    failure.raw_digest = Some(revision_digest.into_boxed_str());
                    failure.budget.as_mut().expect("budget diagnostics").commits =
                        Some(revisions.len());
                    failure
                        .budget
                        .as_mut()
                        .expect("budget diagnostics")
                        .raw_output_digest
                        .clone_from(&failure.raw_digest);
                    failure
                        .budget
                        .as_mut()
                        .expect("budget diagnostics")
                        .raw_bytes = Some(revision_raw_bytes);
                    return Err(failure);
                }
                revisions.sort();
                Ok(PreparedExtraction {
                    repository,
                    empty_config,
                    started,
                    base_revision: base,
                    head_revision: head,
                    commits: revisions
                        .into_iter()
                        .map(|revision| PreparedCommit {
                            revision,
                            known_parent: None,
                        })
                        .collect(),
                })
            }
        }
    }

    fn reject_replacement_state(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        started: Instant,
    ) -> Result<(), GitNonEvaluation> {
        let replacements = self.run_git(
            repository,
            empty_config,
            &["for-each-ref", "--format=%(refname)", "refs/replace/"],
            started,
            "replacement-check",
            1024 * 1024,
            "git.output-bytes",
        )?;
        if !replacements.is_empty() {
            return Err(not_evaluated(
                "repository.replace-ref",
                "repository",
                String::from_utf8_lossy(&replacements).trim().to_owned(),
            ));
        }

        let common_dir = self.run_git(
            repository,
            empty_config,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            started,
            "common-directory",
            64 * 1024,
            "git.output-bytes",
        )?;
        let common_dir = utf8_trimmed(common_dir, "repository.git-common-dir")?;
        if Path::new(&common_dir).join("info/grafts").exists() {
            return Err(not_evaluated(
                "repository.graft",
                "repository",
                "legacy info/grafts is present".to_owned(),
            ));
        }
        let shallow = self.run_git(
            repository,
            empty_config,
            &["rev-parse", "--is-shallow-repository"],
            started,
            "shallow-check",
            16,
            "git.output-bytes",
        )?;
        if utf8_trimmed(shallow, "repository.shallow-check-invalid")? == "true" {
            return Err(not_evaluated(
                "repository.shallow",
                "repository",
                "shallow repositories cannot prove complete ancestry".to_owned(),
            ));
        }
        Ok(())
    }

    fn resolve_canonical_worktree(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        started: Instant,
    ) -> Result<PathBuf, GitNonEvaluation> {
        let bare = self.run_git(
            repository,
            empty_config,
            &["rev-parse", "--is-bare-repository"],
            started,
            "identity",
            16,
            "git.output-bytes",
        )?;
        if utf8_trimmed(bare, "identity.bare-check-invalid")? == "true" {
            return Err(not_evaluated(
                "repository.bare",
                "identity",
                "bare repositories have no canonical worktree".to_owned(),
            ));
        }
        let worktree = self.run_git(
            repository,
            empty_config,
            &["rev-parse", "--path-format=absolute", "--show-toplevel"],
            started,
            "identity-worktree",
            64 * 1024,
            "git.output-bytes",
        )?;
        let worktree = utf8_trimmed(worktree, "identity.worktree-invalid")?;
        Path::new(&worktree).canonicalize().map_err(|error| {
            not_evaluated("identity.worktree-invalid", "identity", error.to_string())
        })
    }

    fn derive_repository_identity(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        run_id: String,
        started: Instant,
    ) -> Result<GitEvaluationIdentity, GitNonEvaluation> {
        let common_dir = self.run_git(
            repository,
            empty_config,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            started,
            "identity-common-directory",
            64 * 1024,
            "git.output-bytes",
        )?;
        let common_dir = utf8_trimmed(common_dir, "identity.git-common-dir")?;
        let common_dir = Path::new(&common_dir).canonicalize().map_err(|error| {
            not_evaluated("identity.common-dir-invalid", "identity", error.to_string())
        })?;
        Ok(GitEvaluationIdentity {
            run_id,
            repository_id: opaque_path_identity(b"repository", &common_dir),
            canonical_worktree_id: opaque_path_identity(b"worktree", repository),
            run_started: started,
        })
    }

    fn resolve_commit(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        input: &str,
        started: Instant,
    ) -> Result<String, GitNonEvaluation> {
        let revision = format!("{input}^{{commit}}");
        let output = self
            .run_git(
                repository,
                empty_config,
                &["rev-parse", "--verify", "--end-of-options", &revision],
                started,
                "revision",
                512,
                "git.output-bytes",
            )
            .map_err(|mut failure| {
                if !failure.reason.starts_with("budget.") {
                    failure.reason = "revision.invalid";
                    failure.stage = "revision";
                }
                failure
            })?;
        utf8_trimmed(output, "revision.invalid")
    }

    fn first_parent_or_empty_tree(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        commit: &str,
        started: Instant,
    ) -> Result<String, GitNonEvaluation> {
        let line = self.run_git(
            repository,
            empty_config,
            &["rev-list", "--parents", "-n", "1", commit],
            started,
            "parents",
            512,
            "git.output-bytes",
        )?;
        let line = utf8_trimmed(line, "revision.parents-invalid")?;
        if let Some(parent) = line.split_ascii_whitespace().nth(1) {
            return Ok(parent.to_owned());
        }
        let empty_tree = self.run_git(
            repository,
            empty_config,
            &["hash-object", "-t", "tree", "--stdin"],
            started,
            "empty-tree",
            128,
            "git.output-bytes",
        )?;
        utf8_trimmed(empty_tree, "revision.empty-tree-invalid")
    }

    fn load_base_scope_mappings(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        base: &str,
        started: Instant,
    ) -> Result<Option<BaseScopeMappings>, GitNonEvaluation> {
        let candidates = self.run_git(
            repository,
            empty_config,
            &[
                "ls-tree",
                "-z",
                "--name-only",
                base,
                "--",
                ".anvil.yaml",
                ".anvil.yml",
                ".anvil.json",
                ".anvil.toml",
            ],
            started,
            "base-config-discovery",
            256,
            "git.output-bytes",
        )?;
        let candidate_text = std::str::from_utf8(&candidates).map_err(|_| {
            mapping_invalid("base-tree config discovery returned non-UTF-8".to_owned())
        })?;
        let available: Vec<&str> = candidate_text
            .split('\0')
            .filter(|path| !path.is_empty())
            .collect();
        let selected = DISCOVER_PRECEDENCE.iter().find_map(|format| {
            let path = format!(".anvil.{}", format.extension());
            available
                .iter()
                .any(|candidate| *candidate == path)
                .then_some((*format, path))
        });
        let Some((format, source_path)) = selected else {
            return Ok(None);
        };

        let object_spec = format!("{base}:{source_path}");
        let bytes = self.run_git(
            repository,
            empty_config,
            &["cat-file", "blob", &object_spec],
            started,
            "base-config-read",
            usize::try_from(MAX_CONFIG_FILE_BYTES).expect("config byte limit fits usize"),
            "budget.config-bytes",
        )?;
        let contents = std::str::from_utf8(&bytes)
            .map_err(|_| mapping_invalid("base-tree config is not UTF-8".to_owned()))?;
        let value = parse_str(contents, format, Path::new(&source_path))
            .map_err(|error| mapping_invalid(error.to_string()))?;
        let canonical =
            canonical_json_bytes(&value).map_err(|error| mapping_invalid(error.to_string()))?;
        parse_scope_mappings(&value, source_path, digest_bytes(&canonical))
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn extract_commit(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        commit: &str,
        parent: &str,
        base: &str,
        head: &str,
        identity: &GitEvaluationIdentity,
        scope_mappings: Option<&BaseScopeMappings>,
        started: Instant,
    ) -> Result<ConventionalCommitEvidence, GitNonEvaluation> {
        let ExtractedCoverage {
            members: coverage,
            decoded_path_bytes: path_bytes,
            raw_digest: coverage_digest,
            raw_bytes: coverage_raw_bytes,
            rename_sources,
            rename_targets,
        } = self.extract_coverage(repository, empty_config, parent, commit, started)?;
        let object = self
            .run_git(
                repository,
                empty_config,
                &["cat-file", "commit", commit],
                started,
                "commit-message",
                self.limits
                    .max_decoded_bytes_per_commit
                    .saturating_add(1024 * 1024),
                "budget.decoded-bytes",
            )
            .map_err(|mut failure| {
                attach_coverage_counts(
                    &mut failure,
                    coverage.len(),
                    coverage_raw_bytes,
                    &coverage_digest,
                    rename_sources,
                    rename_targets,
                );
                failure
            })?;
        let message = object
            .windows(2)
            .position(|pair| pair == b"\n\n")
            .map_or(&[][..], |index| &object[index + 2..]);
        let decoded_bytes = message.len().saturating_add(path_bytes);
        if decoded_bytes > self.limits.max_decoded_bytes_per_commit {
            let mut failure = over_budget(
                "budget.decoded-bytes",
                "commit",
                decoded_bytes,
                self.limits.max_decoded_bytes_per_commit,
            );
            failure.raw_digest = Some(coverage_digest.into_boxed_str());
            let diagnostics = failure.budget.as_mut().expect("budget diagnostics");
            diagnostics.decoded_bytes = Some(decoded_bytes);
            diagnostics.records = Some(coverage.len());
            diagnostics.raw_bytes = Some(coverage_raw_bytes);
            diagnostics
                .raw_output_digest
                .clone_from(&failure.raw_digest);
            diagnostics.rename_sources = Some(rename_sources);
            diagnostics.rename_targets = Some(rename_targets);
            return Err(failure);
        }
        let message_text = std::str::from_utf8(message).map_err(|_| {
            not_evaluated(
                "claim.invalid-utf8",
                "commit-message",
                "commit message is not UTF-8".to_owned(),
            )
        })?;
        let first_line = message_text
            .split('\n')
            .next()
            .unwrap_or_default()
            .strip_suffix('\r')
            .unwrap_or_else(|| message_text.split('\n').next().unwrap_or_default());
        let (header, declared_scope, claims) = parse_header(first_line, scope_mappings)?;

        let mut hasher = Sha256::new();
        hasher.update(message);
        let digest = hasher
            .finalize()
            .iter()
            .fold(String::from("sha256:"), |mut output, byte| {
                use std::fmt::Write as _;
                write!(output, "{byte:02x}").expect("write to String");
                output
            });
        let source = IntentSource {
            tier: IntentTier::Tier0,
            kind: IntentSourceKind::ConventionalCommit,
            reference: commit.to_owned(),
            digest,
            evidence_grade: EvidenceGrade::Weak,
            producer_schema: "git.conventional-commit.v1".to_owned(),
            producer_record_id: None,
        };

        Ok(ConventionalCommitEvidence {
            commit_revision: commit.to_owned(),
            parent_revision: parent.to_owned(),
            binding: EvaluationBinding {
                run_id: identity.run_id.clone(),
                repository_id: identity.repository_id.clone(),
                canonical_worktree_id: identity.canonical_worktree_id.clone(),
                base_revision: base.to_owned(),
                head_revision: head.to_owned(),
                commit_revision: commit.to_owned(),
            },
            header,
            source,
            declared_scope,
            claims,
            coverage,
            contributing_base_config_paths: scope_mappings
                .map_or_else(Vec::new, |authority| vec![authority.source_path.clone()]),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_footprint_commit(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        commit: &str,
        parent: &str,
        base: &str,
        head: &str,
        identity: &GitEvaluationIdentity,
        started: Instant,
    ) -> Result<GitFootprintCommitEvidence, GitNonEvaluation> {
        let ExtractedCoverage {
            members: coverage,
            decoded_path_bytes,
            raw_digest,
            raw_bytes,
            rename_sources,
            rename_targets,
        } = self.extract_coverage(repository, empty_config, parent, commit, started)?;
        if decoded_path_bytes > self.limits.max_decoded_bytes_per_commit {
            let mut failure = over_budget(
                "budget.decoded-bytes",
                "diff",
                decoded_path_bytes,
                self.limits.max_decoded_bytes_per_commit,
            );
            failure.raw_digest = Some(raw_digest.into_boxed_str());
            let diagnostics = failure.budget.as_mut().expect("budget diagnostics");
            diagnostics.decoded_bytes = Some(decoded_path_bytes);
            diagnostics.records = Some(coverage.len());
            diagnostics.raw_bytes = Some(raw_bytes);
            diagnostics.rename_sources = Some(rename_sources);
            diagnostics.rename_targets = Some(rename_targets);
            diagnostics
                .raw_output_digest
                .clone_from(&failure.raw_digest);
            return Err(failure);
        }
        Ok(GitFootprintCommitEvidence {
            commit_revision: commit.to_owned(),
            parent_revision: parent.to_owned(),
            binding: EvaluationBinding {
                run_id: identity.run_id.clone(),
                repository_id: identity.repository_id.clone(),
                canonical_worktree_id: identity.canonical_worktree_id.clone(),
                base_revision: base.to_owned(),
                head_revision: head.to_owned(),
                commit_revision: commit.to_owned(),
            },
            coverage,
        })
    }

    #[allow(clippy::too_many_lines)]
    fn extract_coverage(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        parent: &str,
        commit: &str,
        started: Instant,
    ) -> Result<ExtractedCoverage, GitNonEvaluation> {
        if started.elapsed() >= self.limits.run_timeout {
            return Err(over_budget(
                "budget.run-timeout",
                "diff-preflight",
                duration_millis(started.elapsed()),
                duration_millis(self.limits.run_timeout),
            ));
        }
        let preflight = self.run_git(
            repository,
            empty_config,
            &[
                "diff-tree",
                "--no-commit-id",
                "--raw",
                "-z",
                "-r",
                "--no-ext-diff",
                "--no-textconv",
                "--no-renames",
                "--no-abbrev",
                parent,
                commit,
            ],
            started,
            "diff-preflight",
            self.limits.max_raw_bytes_per_commit,
            "budget.raw-bytes",
        )?;
        if preflight.len() > self.limits.max_raw_bytes_per_commit {
            return Err(over_budget(
                "budget.raw-bytes",
                "diff-preflight",
                preflight.len(),
                self.limits.max_raw_bytes_per_commit,
            ));
        }
        let preflight_digest = digest_bytes(&preflight);
        let (preflight_records, preflight_path_bytes) =
            parse_raw_records(&preflight, self.limits.max_records_per_commit).map_err(
                |mut failure| {
                    attach_raw_output(&mut failure, preflight.len(), &preflight_digest);
                    failure
                },
            )?;
        let rename_sources = preflight_records
            .iter()
            .filter(|record| {
                matches!(
                    record.status,
                    GitChangeStatus::Deleted | GitChangeStatus::TypeChanged
                )
            })
            .count();
        let rename_targets = preflight_records
            .iter()
            .filter(|record| {
                matches!(
                    record.status,
                    GitChangeStatus::Added | GitChangeStatus::TypeChanged
                )
            })
            .count();
        if rename_sources > self.limits.max_rename_sources
            || rename_targets > self.limits.max_rename_targets
        {
            let observed = rename_sources.max(rename_targets);
            let limit = self
                .limits
                .max_rename_sources
                .max(self.limits.max_rename_targets);
            let mut failure = over_budget(
                "budget.rename-candidates",
                "diff-preflight",
                observed,
                limit,
            );
            failure.raw_digest = Some(preflight_digest.into_boxed_str());
            let diagnostics = failure.budget.as_mut().expect("budget diagnostics");
            diagnostics.records = Some(preflight_records.len());
            diagnostics.rename_sources = Some(rename_sources);
            diagnostics.rename_targets = Some(rename_targets);
            diagnostics.raw_bytes = Some(preflight.len());
            diagnostics.decoded_bytes = Some(preflight_path_bytes);
            diagnostics
                .raw_output_digest
                .clone_from(&failure.raw_digest);
            return Err(failure);
        }

        let raw = self
            .run_git(
                repository,
                empty_config,
                &[
                    "diff-tree",
                    "--no-commit-id",
                    "--raw",
                    "-z",
                    "-r",
                    "--no-ext-diff",
                    "--no-textconv",
                    "-M50%",
                    "-l1000",
                    "--no-abbrev",
                    parent,
                    commit,
                ],
                started,
                "diff",
                self.limits.max_raw_bytes_per_commit,
                "budget.raw-bytes",
            )
            .map_err(|mut failure| {
                if let Some(diagnostics) = &mut failure.budget {
                    diagnostics.rename_sources = Some(rename_sources);
                    diagnostics.rename_targets = Some(rename_targets);
                }
                failure
            })?;
        if raw.len() > self.limits.max_raw_bytes_per_commit {
            return Err(over_budget(
                "budget.raw-bytes",
                "diff",
                raw.len(),
                self.limits.max_raw_bytes_per_commit,
            ));
        }
        let raw_digest = digest_bytes(&raw);
        parse_raw_records(&raw, self.limits.max_records_per_commit)
            .and_then(|(members, decoded_path_bytes)| {
                if coverage_endpoints(&preflight_records) != coverage_endpoints(&members) {
                    let mut failure = not_evaluated(
                        "git.endpoint-mismatch",
                        "diff",
                        "preflight and final diff endpoint sets differ".to_owned(),
                    );
                    failure.raw_digest = Some(raw_digest.clone().into_boxed_str());
                    return Err(failure);
                }
                Ok(ExtractedCoverage {
                    members,
                    decoded_path_bytes,
                    raw_digest: raw_digest.clone(),
                    raw_bytes: raw.len(),
                    rename_sources,
                    rename_targets,
                })
            })
            .map_err(|mut failure| {
                attach_raw_output(&mut failure, raw.len(), &raw_digest);
                if let Some(diagnostics) = &mut failure.budget {
                    diagnostics.rename_sources = Some(rename_sources);
                    diagnostics.rename_targets = Some(rename_targets);
                }
                failure
            })
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn run_git(
        &self,
        repository: &Path,
        empty_config: &EmptyGlobalConfig,
        args: &[&str],
        started: Instant,
        stage: &'static str,
        output_limit: usize,
        overflow_reason: &'static str,
    ) -> Result<Vec<u8>, GitNonEvaluation> {
        if started.elapsed() >= self.limits.run_timeout {
            return Err(over_budget(
                "budget.run-timeout",
                stage,
                duration_millis(started.elapsed()),
                duration_millis(self.limits.run_timeout),
            ));
        }
        let git_program = self.git_program.as_ref().map_err(Clone::clone)?;
        let mut command = Command::new(git_program);
        command
            .env_clear()
            .env("LC_ALL", "C")
            .env("TZ", "UTC")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &empty_config.path)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .arg("--no-replace-objects")
            .arg("-c")
            .arg("diff.renames=true")
            .arg("-c")
            .arg("diff.renameLimit=1000")
            .arg("-C")
            .arg(repository)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_process_isolation(&mut command);

        let command_started = Instant::now();
        let mut child = command
            .spawn()
            .map_err(|error| not_evaluated("git.spawn", stage, error.to_string()))?;
        let stdout = child.stdout.take().expect("piped Git stdout");
        let stderr = child.stderr.take().expect("piped Git stderr");
        let (overflow_sender, overflow_receiver) = mpsc::sync_channel(1);
        let (stdout_sender, stdout_receiver) = mpsc::sync_channel(1);
        let (stderr_sender, stderr_receiver) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let _ = stdout_sender.send(capture_reader(stdout, output_limit, Some(overflow_sender)));
        });
        thread::spawn(move || {
            let _ = stderr_sender.send(capture_reader(stderr, 64 * 1024, None));
        });

        let mut status = None;
        let mut termination: Option<(&'static str, usize, usize)> = None;
        loop {
            if started.elapsed() >= self.limits.run_timeout {
                termination = Some((
                    "budget.run-timeout",
                    duration_millis(started.elapsed()),
                    duration_millis(self.limits.run_timeout),
                ));
                break;
            }
            if command_started.elapsed() >= self.limits.git_timeout {
                termination = Some((
                    "budget.git-timeout",
                    duration_millis(command_started.elapsed()),
                    duration_millis(self.limits.git_timeout),
                ));
                break;
            }
            if let Ok(observed) = overflow_receiver.try_recv() {
                termination = Some((overflow_reason, observed, output_limit));
                break;
            }
            match child.try_wait() {
                Ok(Some(exit)) => {
                    status = Some(exit);
                    break;
                }
                Ok(None) => thread::sleep(Duration::from_millis(1)),
                Err(error) => {
                    terminate_process_tree(&mut child);
                    return Err(not_evaluated("git.wait-failed", stage, error.to_string()));
                }
            }
        }

        if termination.is_some() {
            terminate_process_tree(&mut child);
        }
        let mut stdout = stdout_receiver.recv_timeout(CAPTURE_DRAIN_TIMEOUT).ok();
        let mut stderr = stderr_receiver.recv_timeout(CAPTURE_DRAIN_TIMEOUT).ok();
        if stdout.is_none() || stderr.is_none() {
            terminate_process_tree(&mut child);
            stdout = stdout.or_else(|| stdout_receiver.recv_timeout(CAPTURE_DRAIN_TIMEOUT).ok());
            stderr = stderr.or_else(|| stderr_receiver.recv_timeout(CAPTURE_DRAIN_TIMEOUT).ok());
        }
        let stdout = stdout.ok_or_else(|| {
            not_evaluated(
                "git.capture-timeout",
                stage,
                "stdout reader did not close after process-tree termination".to_owned(),
            )
        })?;
        let stderr = stderr.ok_or_else(|| {
            not_evaluated(
                "git.capture-timeout",
                stage,
                "stderr reader did not close after process-tree termination".to_owned(),
            )
        })?;
        if let Some(error) = &stdout.error {
            return Err(not_evaluated("git.capture-failed", stage, error.clone()));
        }
        if let Some(error) = &stderr.error {
            return Err(not_evaluated("git.capture-failed", stage, error.clone()));
        }

        if termination.is_none() && stdout.total > output_limit {
            termination = Some((overflow_reason, stdout.total, output_limit));
        }
        if let Some((reason, observed, limit)) = termination {
            let (observed, limit) = if reason == "budget.commits" {
                (
                    complete_revision_count(&stdout.bytes),
                    self.limits.max_commits,
                )
            } else {
                (observed, limit)
            };
            let mut failure = over_budget(reason, stage, observed, limit);
            failure.raw_digest = Some(stdout.digest.into_boxed_str());
            let diagnostics = failure.budget.as_mut().expect("budget diagnostics");
            if matches!(reason, "budget.git-timeout" | "budget.run-timeout") {
                diagnostics.elapsed_millis = Some(observed);
            }
            if matches!(stage, "diff-preflight" | "diff") {
                let (records, decoded_bytes) = complete_raw_record_counts(&stdout.bytes);
                diagnostics.records = Some(records);
                diagnostics.decoded_bytes = Some(decoded_bytes);
            }
            if reason == "budget.commits" {
                diagnostics.commits = Some(observed);
            }
            diagnostics.raw_bytes = Some(stdout.total);
            diagnostics
                .raw_output_digest
                .clone_from(&failure.raw_digest);
            return Err(failure);
        }

        let status = status.expect("completed Git command has exit status");
        if !status.success() {
            return Err(not_evaluated(
                "git.command-failed",
                stage,
                String::from_utf8_lossy(&stderr.bytes).trim().to_owned(),
            ));
        }
        Ok(stdout.bytes)
    }
}

struct CapturedOutput {
    bytes: Vec<u8>,
    total: usize,
    digest: String,
    error: Option<String>,
}

#[allow(clippy::needless_pass_by_value)]
fn capture_reader<R: Read>(
    mut reader: R,
    limit: usize,
    overflow_sender: Option<mpsc::SyncSender<usize>>,
) -> CapturedOutput {
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    let mut total = 0usize;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    let mut notified = false;
    let mut error = None;
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => count,
            Err(read_error) => {
                error = Some(read_error.to_string());
                break;
            }
        };
        total = total.saturating_add(count);
        hasher.update(&buffer[..count]);
        let remaining = limit.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..count.min(remaining)]);
        if total > limit && !notified {
            if let Some(sender) = &overflow_sender {
                let _ = sender.try_send(total);
            }
            notified = true;
        }
    }
    let digest = hasher.finalize();
    CapturedOutput {
        bytes,
        total,
        digest: digest_hex(&digest),
        error,
    }
}

fn complete_raw_record_counts(raw: &[u8]) -> (usize, usize) {
    let mut cursor = 0usize;
    let mut records = 0usize;
    let mut decoded_bytes = 0usize;
    while let Some(header_end) = raw[cursor..].iter().position(|byte| *byte == 0) {
        let header_end = cursor + header_end;
        let Ok(header) = std::str::from_utf8(&raw[cursor..header_end]) else {
            break;
        };
        let Some(status) = header.split_ascii_whitespace().nth(4) else {
            break;
        };
        let path_count = match status.as_bytes().first() {
            Some(b'R') => 2,
            Some(b'A' | b'D' | b'M' | b'T') if status.len() == 1 => 1,
            _ => break,
        };
        cursor = header_end + 1;

        let mut record_decoded_bytes = 0usize;
        let mut complete = true;
        for _ in 0..path_count {
            let Some(path_end) = raw[cursor..].iter().position(|byte| *byte == 0) else {
                complete = false;
                break;
            };
            let path_end = cursor + path_end;
            if std::str::from_utf8(&raw[cursor..path_end]).is_err() {
                complete = false;
                break;
            }
            record_decoded_bytes =
                record_decoded_bytes.saturating_add(path_end.saturating_sub(cursor));
            cursor = path_end + 1;
        }
        if !complete {
            break;
        }
        records = records.saturating_add(1);
        decoded_bytes = decoded_bytes.saturating_add(record_decoded_bytes);
    }
    (records, decoded_bytes)
}

fn complete_revision_count(raw: &[u8]) -> usize {
    raw.split_inclusive(|byte| *byte == b'\n')
        .filter(|line| {
            let Some(oid) = line.strip_suffix(b"\n") else {
                return false;
            };
            matches!(oid.len(), 40 | 64) && oid.iter().all(u8::is_ascii_hexdigit)
        })
        .count()
}

fn coverage_endpoints(members: &[GitCoverageMember]) -> BTreeSet<&str> {
    members
        .iter()
        .flat_map(|member| {
            member
                .old_path
                .as_deref()
                .into_iter()
                .chain(std::iter::once(member.new_path.as_str()))
        })
        .collect()
}

fn retain_commit_result(
    result: Result<ConventionalCommitEvidence, GitNonEvaluation>,
    commit: &str,
) -> GitCommitExtraction {
    match result {
        Ok(evidence) => GitCommitExtraction::Evaluated(Box::new(evidence)),
        Err(mut failure) => {
            failure.commit_revision = Some(commit.into());
            GitCommitExtraction::NotEvaluated(failure)
        }
    }
}

fn retain_footprint_result(
    result: Result<GitFootprintCommitEvidence, GitNonEvaluation>,
    commit: &str,
) -> GitFootprintCommitExtraction {
    match result {
        Ok(evidence) => GitFootprintCommitExtraction::Evaluated(Box::new(evidence)),
        Err(mut failure) => {
            failure.commit_revision = Some(commit.into());
            GitFootprintCommitExtraction::NotEvaluated(failure)
        }
    }
}

#[allow(clippy::too_many_lines)]
fn parse_raw_records(
    raw: &[u8],
    max_records: usize,
) -> Result<(Vec<GitCoverageMember>, usize), GitNonEvaluation> {
    if raw.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let chunks: Vec<&[u8]> = raw.split(|byte| *byte == 0).collect();
    if chunks.last().is_none_or(|chunk| !chunk.is_empty()) {
        return Err(not_evaluated(
            "git.raw-malformed",
            "diff",
            "raw -z output has no terminal NUL".to_owned(),
        ));
    }

    let mut records = Vec::new();
    let mut decoded_bytes = 0usize;
    let mut index = 0usize;
    while index + 1 < chunks.len() {
        if records.len() >= max_records {
            let mut failure = over_budget("budget.records", "diff", records.len() + 1, max_records);
            failure.budget.as_mut().expect("budget diagnostics").records = Some(records.len());
            failure
                .budget
                .as_mut()
                .expect("budget diagnostics")
                .decoded_bytes = Some(decoded_bytes);
            return Err(failure);
        }
        let header = std::str::from_utf8(chunks[index]).map_err(|_| {
            not_evaluated(
                "git.raw-malformed",
                "diff",
                "raw record header is not ASCII".to_owned(),
            )
        })?;
        index += 1;
        let mut fields = header.split_ascii_whitespace();
        let old_mode = fields
            .next()
            .and_then(|mode| mode.strip_prefix(':'))
            .ok_or_else(raw_malformed)?;
        let new_mode = fields.next().ok_or_else(raw_malformed)?;
        let old_object = fields.next().ok_or_else(raw_malformed)?;
        let new_object = fields.next().ok_or_else(raw_malformed)?;
        let raw_status = fields.next().ok_or_else(raw_malformed)?;
        if fields.next().is_some()
            || !valid_mode(old_mode)
            || !valid_mode(new_mode)
            || !valid_object_id(old_object)
            || !valid_object_id(new_object)
        {
            return Err(raw_malformed());
        }

        let (status, rename_score) = match raw_status.as_bytes().first() {
            Some(b'A') if raw_status == "A" => (GitChangeStatus::Added, None),
            Some(b'D') if raw_status == "D" => (GitChangeStatus::Deleted, None),
            Some(b'M') if raw_status == "M" => (GitChangeStatus::Modified, None),
            Some(b'T') if raw_status == "T" => (GitChangeStatus::TypeChanged, None),
            Some(b'R') => {
                let score = raw_status[1..]
                    .parse::<u16>()
                    .ok()
                    .filter(|score| *score <= 100)
                    .ok_or_else(raw_malformed)?;
                (GitChangeStatus::Renamed, Some(score))
            }
            _ => {
                return Err(not_evaluated(
                    "git.status-unsupported",
                    "diff",
                    format!("unsupported raw status {raw_status:?}"),
                ));
            }
        };

        let first_path = chunks.get(index).copied().ok_or_else(raw_malformed)?;
        index += 1;
        let (old_path, new_path_bytes) = if status == GitChangeStatus::Renamed {
            let new_path = chunks.get(index).copied().ok_or_else(raw_malformed)?;
            index += 1;
            (Some(decode_repository_path(first_path)?), new_path)
        } else {
            (None, first_path)
        };
        let new_path = decode_repository_path(new_path_bytes)?;
        let old_path_bytes = old_path.as_ref().map_or(0, String::len);
        decoded_bytes = decoded_bytes
            .checked_add(new_path_bytes.len())
            .and_then(|total| total.checked_add(old_path_bytes))
            .ok_or_else(|| {
                let mut failure =
                    over_budget("budget.decoded-bytes", "diff", usize::MAX, usize::MAX);
                let diagnostics = failure.budget.as_mut().expect("budget diagnostics");
                diagnostics.decoded_bytes = Some(usize::MAX);
                diagnostics.records = Some(records.len());
                failure
            })?;

        records.push(GitCoverageMember {
            status,
            raw_status: raw_status.to_owned(),
            rename_score,
            old_path,
            new_path,
            old_mode: old_mode.to_owned(),
            new_mode: new_mode.to_owned(),
            old_object_type: object_type(old_mode),
            new_object_type: object_type(new_mode),
            old_object: old_object.to_owned(),
            new_object: new_object.to_owned(),
        });
    }

    records.sort_by(|left, right| {
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
    });
    Ok((records, decoded_bytes))
}

fn raw_malformed() -> GitNonEvaluation {
    not_evaluated(
        "git.raw-malformed",
        "diff",
        "malformed raw -z record".to_owned(),
    )
}

fn valid_mode(mode: &str) -> bool {
    mode.len() == 6 && mode.bytes().all(|byte| matches!(byte, b'0'..=b'7'))
}

fn valid_object_id(object: &str) -> bool {
    matches!(object.len(), 40 | 64) && object.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn object_type(mode: &str) -> GitObjectType {
    match mode {
        "000000" => GitObjectType::Absent,
        "040000" => GitObjectType::Tree,
        "160000" => GitObjectType::Gitlink,
        mode if mode.starts_with("100") || mode == "120000" => GitObjectType::Blob,
        _ => GitObjectType::Unknown,
    }
}

fn decode_repository_path(path: &[u8]) -> Result<String, GitNonEvaluation> {
    let path = std::str::from_utf8(path).map_err(|_| {
        not_evaluated(
            "git.path-invalid-utf8",
            "diff",
            "raw path is not UTF-8".to_owned(),
        )
    })?;
    if path.is_empty()
        || path.starts_with('/')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(not_evaluated(
            "git.path-invalid",
            "diff",
            "raw path is not repository-relative".to_owned(),
        ));
    }
    Ok(path.to_owned())
}

fn duration_millis(duration: Duration) -> usize {
    usize::try_from(duration.as_millis()).unwrap_or(usize::MAX)
}

fn parse_scope_mappings(
    value: &Value,
    source_path: String,
    source_digest: String,
) -> Result<Option<BaseScopeMappings>, GitNonEvaluation> {
    let root = value
        .as_object()
        .ok_or_else(|| mapping_invalid("canonical config root is not an object".to_owned()))?;
    let Some(intent_conformance) = root.get("intent_conformance") else {
        return Ok(None);
    };
    let intent_conformance = intent_conformance
        .as_object()
        .ok_or_else(|| mapping_invalid("intent_conformance is not an object".to_owned()))?;
    let Some(scope_mappings) = intent_conformance.get("scope_mappings") else {
        return Ok(None);
    };
    let scope_mappings = scope_mappings
        .as_object()
        .ok_or_else(|| mapping_invalid("scope_mappings is not an object".to_owned()))?;
    if scope_mappings
        .keys()
        .any(|key| key != "schema_version" && key != "mappings")
    {
        return Err(mapping_invalid(
            "scope_mappings contains an unknown key".to_owned(),
        ));
    }
    let schema_version = scope_mappings
        .get("schema_version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .filter(|version| *version == 1)
        .ok_or_else(|| mapping_invalid("scope_mappings schema_version must be 1".to_owned()))?;
    let raw_mappings = scope_mappings
        .get("mappings")
        .and_then(Value::as_object)
        .ok_or_else(|| mapping_invalid("scope_mappings.mappings is not an object".to_owned()))?;
    let mut mappings = BTreeMap::new();
    for (mapping_key, raw_prefixes) in raw_mappings {
        if mapping_key.is_empty() {
            return Err(mapping_invalid("mapping key is empty".to_owned()));
        }
        let raw_prefixes = raw_prefixes
            .as_array()
            .ok_or_else(|| mapping_invalid(format!("mapping {mapping_key:?} is not an array")))?;
        let prefixes: Vec<String> = raw_prefixes
            .iter()
            .map(|prefix| {
                prefix.as_str().map(str::to_owned).ok_or_else(|| {
                    mapping_invalid(format!("mapping {mapping_key:?} has a non-string prefix"))
                })
            })
            .collect::<Result<_, _>>()?;
        if prefixes.is_empty()
            || prefixes.iter().any(|prefix| !valid_path_prefix(prefix))
            || prefixes
                .windows(2)
                .any(|pair| pair[0].as_bytes() >= pair[1].as_bytes())
        {
            return Err(mapping_invalid(format!(
                "mapping {mapping_key:?} prefixes are not sorted, unique valid paths"
            )));
        }
        mappings.insert(mapping_key.clone(), prefixes);
    }
    Ok(Some(BaseScopeMappings {
        schema_version,
        source_path,
        source_digest,
        mappings,
    }))
}

#[allow(clippy::too_many_lines)]
fn parse_header(
    first_line: &str,
    scope_mappings: Option<&BaseScopeMappings>,
) -> Result<
    (
        ConventionalCommitHeader,
        Option<DeclaredScope>,
        Vec<ConformanceClaim>,
    ),
    GitNonEvaluation,
> {
    let captures = CONVENTIONAL_COMMIT_HEADER_PATTERN
        .captures(first_line)
        .ok_or_else(|| {
            not_evaluated(
                "claim.malformed",
                "claim",
                "header does not match Conventional Commit syntax".to_owned(),
            )
        })?;

    let commit_type = captures[1].to_owned();
    let scope = captures.get(2).map(|value| value.as_str().to_owned());
    let breaking = captures.get(3).is_some();
    let description = captures[4].to_owned();
    let header = ConventionalCommitHeader {
        commit_type: commit_type.clone(),
        scope: scope.clone(),
        breaking,
        description,
    };

    let mut claims = Vec::new();
    match commit_type.as_str() {
        "docs" => claims.push(ConformanceClaim {
            kind: ClaimKind::FileClass,
            value: "documentation-only".to_owned(),
            source_index: 0,
        }),
        "test" => claims.push(ConformanceClaim {
            kind: ClaimKind::FileClass,
            value: "test-only".to_owned(),
            source_index: 0,
        }),
        _ => {}
    }

    let declared_scope = match scope {
        Some(label) if label.starts_with("path:") => {
            let prefix = label["path:".len()..].to_owned();
            if !valid_path_prefix(&prefix) {
                return Err(not_evaluated(
                    "claim.scope-invalid",
                    "claim",
                    "explicit path scope is invalid".to_owned(),
                ));
            }
            claims.push(ConformanceClaim {
                kind: ClaimKind::PathPrefix,
                value: prefix.clone(),
                source_index: 0,
            });
            Some(DeclaredScope {
                label,
                authority: ScopeAuthority::ExplicitPath { prefix },
                source_index: 0,
            })
        }
        Some(label) => {
            if let Some((authority, prefixes)) = scope_mappings.and_then(|authority| {
                authority
                    .mappings
                    .get(&label)
                    .map(|prefixes| (authority, prefixes))
            }) {
                claims.extend(prefixes.iter().cloned().map(|value| ConformanceClaim {
                    kind: ClaimKind::PathPrefix,
                    value,
                    source_index: 0,
                }));
                Some(DeclaredScope {
                    label: label.clone(),
                    authority: ScopeAuthority::BaseMapping {
                        mapping_key: label,
                        prefixes: prefixes.clone(),
                        schema_version: authority.schema_version,
                        source_path: authority.source_path.clone(),
                        source_digest: authority.source_digest.clone(),
                    },
                    source_index: 0,
                })
            } else {
                if claims.is_empty() {
                    return Err(not_evaluated(
                        "claim.unknown",
                        "claim",
                        "free-form scope has no base-tree mapping".to_owned(),
                    ));
                }
                Some(DeclaredScope {
                    label,
                    authority: ScopeAuthority::None,
                    source_index: 0,
                })
            }
        }
        None => None,
    };

    if claims.is_empty() {
        return Err(not_evaluated(
            "claim.unknown",
            "claim",
            "header has no closed Tier-0 claim".to_owned(),
        ));
    }

    Ok((header, declared_scope, claims))
}

fn mapping_invalid(detail: String) -> GitNonEvaluation {
    not_evaluated("claim.scope-mapping-invalid", "base-config", detail)
}

fn utf8_trimmed(bytes: Vec<u8>, reason: &'static str) -> Result<String, GitNonEvaluation> {
    let value = String::from_utf8(bytes)
        .map_err(|_| not_evaluated(reason, "git-output", "Git output is not UTF-8".to_owned()))?;
    Ok(value.trim().to_owned())
}

fn not_evaluated(reason: &'static str, stage: &'static str, detail: String) -> GitNonEvaluation {
    GitNonEvaluation {
        commit_revision: None,
        reason,
        stage,
        observed: 0,
        limit: None,
        detail: detail.into_boxed_str(),
        raw_digest: None,
        budget: None,
    }
}

fn over_budget(
    reason: &'static str,
    stage: &'static str,
    observed: usize,
    limit: usize,
) -> GitNonEvaluation {
    GitNonEvaluation {
        commit_revision: None,
        reason,
        stage,
        observed,
        limit: Some(limit),
        detail: format!("observed {observed}, limit {limit}").into_boxed_str(),
        raw_digest: None,
        budget: Some(Box::new(GitBudgetDiagnostics {
            configured_limit: limit,
            elapsed_millis: matches!(reason, "budget.git-timeout" | "budget.run-timeout")
                .then_some(observed),
            commits: None,
            records: None,
            rename_sources: None,
            rename_targets: None,
            raw_bytes: None,
            decoded_bytes: None,
            raw_output_digest: None,
        })),
    }
}

fn attach_raw_output(failure: &mut GitNonEvaluation, raw_bytes: usize, digest: &str) {
    failure.raw_digest = Some(digest.into());
    if let Some(diagnostics) = &mut failure.budget {
        diagnostics.raw_bytes = Some(raw_bytes);
        diagnostics.raw_output_digest = Some(digest.into());
    }
}

fn attach_coverage_counts(
    failure: &mut GitNonEvaluation,
    records: usize,
    raw_bytes: usize,
    digest: &str,
    rename_sources: usize,
    rename_targets: usize,
) {
    attach_raw_output(failure, raw_bytes, digest);
    if let Some(diagnostics) = &mut failure.budget {
        diagnostics.records = Some(records);
        diagnostics.rename_sources = Some(rename_sources);
        diagnostics.rename_targets = Some(rename_targets);
    }
}

fn opaque_path_identity(domain: &[u8], path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"anvil.git.identity.v1\0");
    hasher.update(domain);
    hasher.update(b"\0");
    hasher.update(path.as_os_str().as_encoded_bytes());
    digest_hex(&hasher.finalize())
}

fn resolve_git_program() -> Result<PathBuf, GitNonEvaluation> {
    let executable = format!("git{}", std::env::consts::EXE_SUFFIX);
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(&executable))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| candidate.canonicalize().ok())
        .filter(|candidate| candidate.is_absolute() && candidate.is_file())
        .ok_or_else(|| {
            not_evaluated(
                "git.executable-not-found",
                "git",
                "PATH contains no canonical absolute Git executable".to_owned(),
            )
        })
}

fn configure_process_isolation(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
}

fn terminate_process_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, killpg};
        use nix::unistd::Pid;
        if let Ok(process_group) = i32::try_from(child.id()) {
            let _ = killpg(Pid::from_raw(process_group), Signal::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            let taskkill = PathBuf::from(system_root).join("System32/taskkill.exe");
            if taskkill.is_absolute() && taskkill.is_file() {
                let _ = Command::new(taskkill)
                    .env_clear()
                    .args(["/F", "/T", "/PID", &child.id().to_string()])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[derive(Debug)]
struct EmptyGlobalConfig {
    path: PathBuf,
}

impl EmptyGlobalConfig {
    fn create() -> Result<Self, GitNonEvaluation> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("anvil-conf-gitconfig-{}-{id}", std::process::id()));
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| not_evaluated("git.environment", "git", error.to_string()))?;
        Ok(Self { path })
    }
}

impl Drop for EmptyGlobalConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
