//! Advisory external pull-request declaration conformance check.

use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use anvil_checks::conformance::{
    ConformanceEvaluation, GitBudgetDiagnostics, GitCommitNonEvaluation, GitExtractor,
    GitFootprintExtractionOutcome, GitSelection, PR_BODY_MAX_BYTES, evaluate_pr_declaration,
    extract_pr_body_claims, pr_git_footprint_failures, pr_git_footprint_non_evaluations,
};
use anvil_kernel_types::{ConformanceOutcome, ConformanceVerdict, EvidenceGrade, EvidenceStrength};
use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;

use crate::GlobalArgs;
use crate::output::sarif;
use crate::util::workspace_root;

const REPORT_SCHEMA: &str = "anvil.conformance-check.v1";

#[derive(Debug, Args)]
pub struct ConformanceArgs {
    #[command(subcommand)]
    command: ConformanceCommand,
}

#[derive(Debug, Subcommand)]
enum ConformanceCommand {
    /// Check one PR declaration against an exact Git range.
    Check(CheckArgs),
}

#[derive(Debug, Args)]
struct CheckArgs {
    /// Exclusive base revision for the proposed change.
    #[arg(long)]
    base: String,
    /// Inclusive head revision for the proposed change.
    #[arg(long)]
    head: String,
    /// PR body file, or `-` to read from stdin.
    #[arg(long, value_name = "PATH")]
    pr_body_file: PathBuf,
    /// Immutable identity for the exact PR-body version being checked.
    #[arg(long)]
    source_ref: String,
    /// Output format. Auto is plain; --json is the JSON alias.
    #[arg(long, value_enum)]
    format: Option<ConformanceFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ConformanceFormat {
    Auto,
    Plain,
    Json,
    Sarif,
}

impl ConformanceArgs {
    pub(crate) fn wants_structured_output(&self) -> bool {
        match &self.command {
            ConformanceCommand::Check(args) => args.format.is_some_and(|format| {
                matches!(format, ConformanceFormat::Json | ConformanceFormat::Sarif)
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderMode {
    Plain,
    Json,
    Sarif,
}

impl CheckArgs {
    fn render_mode(&self, global: &GlobalArgs) -> RenderMode {
        match self.format {
            Some(ConformanceFormat::Json) => RenderMode::Json,
            Some(ConformanceFormat::Sarif) => RenderMode::Sarif,
            None if global.json => RenderMode::Json,
            Some(ConformanceFormat::Auto | ConformanceFormat::Plain) | None => RenderMode::Plain,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitBudgetDiagnosticsReport {
    configured_limit: usize,
    elapsed_millis: Option<usize>,
    commits: Option<usize>,
    records: Option<usize>,
    rename_sources: Option<usize>,
    rename_targets: Option<usize>,
    raw_bytes: Option<usize>,
    decoded_bytes: Option<usize>,
    raw_output_digest: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitCommitNonEvaluationReport {
    commit_revision: Option<String>,
    reason: &'static str,
    stage: &'static str,
    observed: usize,
    limit: Option<usize>,
    raw_digest: Option<String>,
    budget: Option<GitBudgetDiagnosticsReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConformanceCheckReport {
    schema_version: &'static str,
    advisory: bool,
    outcome: ConformanceOutcome,
    declaration_evidence_grade: EvidenceGrade,
    evidence_strength: EvidenceStrength,
    resolved_base: Option<String>,
    resolved_head: Option<String>,
    reasons: Vec<String>,
    not_evaluated_commit_count: usize,
    git_non_evaluations: Vec<GitCommitNonEvaluationReport>,
    verdict: Option<ConformanceVerdict>,
}

impl ConformanceCheckReport {
    fn not_evaluated(reasons: Vec<String>) -> Self {
        Self::not_evaluated_with_failures(reasons, Vec::new())
    }

    fn not_evaluated_with_failures(
        reasons: Vec<String>,
        git_non_evaluations: Vec<GitCommitNonEvaluation>,
    ) -> Self {
        let not_evaluated_commit_count = git_non_evaluations.len();
        Self {
            schema_version: REPORT_SCHEMA,
            advisory: true,
            outcome: ConformanceOutcome::NotEvaluated,
            declaration_evidence_grade: EvidenceGrade::Weak,
            evidence_strength: EvidenceStrength::Absent,
            resolved_base: None,
            resolved_head: None,
            reasons,
            not_evaluated_commit_count,
            git_non_evaluations: git_non_evaluations
                .into_iter()
                .map(GitCommitNonEvaluationReport::from)
                .collect(),
            verdict: None,
        }
    }

    fn not_evaluated_with_range(
        reasons: Vec<String>,
        resolved_base: String,
        resolved_head: String,
        git_non_evaluations: Vec<GitCommitNonEvaluation>,
    ) -> Self {
        let mut report = Self::not_evaluated_with_failures(reasons, git_non_evaluations);
        report.resolved_base = Some(resolved_base);
        report.resolved_head = Some(resolved_head);
        report
    }

    fn from_evaluation(evaluation: ConformanceEvaluation) -> Self {
        let ConformanceEvaluation {
            verdict,
            git_non_evaluations,
            ..
        } = evaluation;
        let not_evaluated_commit_count = git_non_evaluations.len();
        Self {
            schema_version: REPORT_SCHEMA,
            advisory: true,
            outcome: verdict.outcome,
            declaration_evidence_grade: EvidenceGrade::Weak,
            evidence_strength: verdict.evidence_strength,
            resolved_base: Some(verdict.binding.base_revision.clone()),
            resolved_head: Some(verdict.binding.head_revision.clone()),
            reasons: verdict.reasons.clone(),
            not_evaluated_commit_count,
            git_non_evaluations: git_non_evaluations
                .into_iter()
                .map(GitCommitNonEvaluationReport::from)
                .collect(),
            verdict: Some(verdict),
        }
    }
}

impl From<GitCommitNonEvaluation> for GitCommitNonEvaluationReport {
    fn from(value: GitCommitNonEvaluation) -> Self {
        Self {
            commit_revision: value.commit_revision.map(|revision| revision.to_string()),
            reason: value.reason,
            stage: value.stage,
            observed: value.observed,
            limit: value.limit,
            raw_digest: value.raw_digest.map(|digest| digest.to_string()),
            budget: value.budget.map(|budget| (*budget).into()),
        }
    }
}

impl From<GitBudgetDiagnostics> for GitBudgetDiagnosticsReport {
    fn from(value: GitBudgetDiagnostics) -> Self {
        Self {
            configured_limit: value.configured_limit,
            elapsed_millis: value.elapsed_millis,
            commits: value.commits,
            records: value.records,
            rename_sources: value.rename_sources,
            rename_targets: value.rename_targets,
            raw_bytes: value.raw_bytes,
            decoded_bytes: value.decoded_bytes,
            raw_output_digest: value.raw_output_digest.map(|digest| digest.to_string()),
        }
    }
}

pub fn run(args: &ConformanceArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        ConformanceCommand::Check(check) => run_check(check, global),
    }
}

fn run_check(args: &CheckArgs, global: &GlobalArgs) -> Result<()> {
    let body = read_pr_body(&args.pr_body_file)?;
    let declaration = extract_pr_body_claims(&args.source_ref, &body).into_contract_parts();
    let repository = workspace_root().context("resolve conformance repository")?;
    let extractor = GitExtractor::default();
    let run_id = format!("conformance-{}", uuid::Uuid::new_v4());
    let git_evidence = match extractor.identity_for_repository(&repository, run_id) {
        Err(failure) => Err(failure.reason.to_owned()),
        Ok(identity) => match extractor.extract_footprint(
            &repository,
            GitSelection::Range {
                base: args.base.clone(),
                head: args.head.clone(),
            },
            &identity,
        ) {
            GitFootprintExtractionOutcome::NotEvaluated(failure) => Err(failure.reason.to_owned()),
            GitFootprintExtractionOutcome::Evaluated(extraction) => Ok((identity, extraction)),
        },
    };
    let report = match (declaration, git_evidence) {
        (Ok(declaration), Ok((identity, extraction))) => {
            let evaluation = evaluate_pr_declaration(&declaration, &extraction, &identity);
            ConformanceCheckReport::from_evaluation(evaluation)
        }
        (Ok(_), Err(git_reason)) => ConformanceCheckReport::not_evaluated(vec![git_reason]),
        (Err(non_evaluation), Ok((identity, extraction))) => {
            let git_non_evaluations = pr_git_footprint_non_evaluations(&extraction);
            let mut reasons: Vec<String> = non_evaluation
                .reasons()
                .iter()
                .map(|reason| (*reason).to_owned())
                .collect();
            reasons.extend(pr_git_footprint_failures(&extraction, &identity));
            reasons.sort();
            reasons.dedup();
            ConformanceCheckReport::not_evaluated_with_range(
                reasons,
                extraction.base_revision,
                extraction.head_revision,
                git_non_evaluations,
            )
        }
        (Err(non_evaluation), Err(git_reason)) => {
            let mut reasons: Vec<String> = non_evaluation
                .reasons()
                .iter()
                .map(|reason| (*reason).to_owned())
                .collect();
            reasons.push(git_reason);
            reasons.sort();
            reasons.dedup();
            ConformanceCheckReport::not_evaluated(reasons)
        }
    };

    render_report(&report, args.render_mode(global))
}

fn read_pr_body(path: &PathBuf) -> Result<String> {
    let mut bytes = Vec::with_capacity(PR_BODY_MAX_BYTES.saturating_add(1));
    let limit =
        u64::try_from(PR_BODY_MAX_BYTES.saturating_add(1)).expect("PR body limit fits in u64");
    if path.as_os_str() == "-" {
        io::stdin()
            .lock()
            .take(limit)
            .read_to_end(&mut bytes)
            .context("read PR body from stdin")?;
    } else {
        File::open(path)
            .with_context(|| format!("open PR body {}", path.display()))?
            .take(limit)
            .read_to_end(&mut bytes)
            .with_context(|| format!("read PR body {}", path.display()))?;
    }
    String::from_utf8(bytes).context("PR body is not valid UTF-8")
}

fn render_report(report: &ConformanceCheckReport, mode: RenderMode) -> Result<()> {
    match mode {
        RenderMode::Plain => render_plain(report),
        RenderMode::Json => println!("{}", serde_json::to_string(report)?),
        RenderMode::Sarif => println!("{}", serde_json::to_string(&build_sarif(report))?),
    }
    Ok(())
}

fn render_plain(report: &ConformanceCheckReport) {
    println!("Conformance check: {}", outcome_label(report.outcome));
    println!("Advisory: yes");
    println!("Declaration evidence grade: weak");
    println!(
        "Evidence strength: {}",
        evidence_strength_label(report.evidence_strength)
    );
    if let Some(base) = &report.resolved_base {
        println!("Resolved base: {base}");
    }
    if let Some(head) = &report.resolved_head {
        println!("Resolved head: {head}");
    }
    for reason in &report.reasons {
        println!("Reason: {reason}");
    }
}

fn build_sarif(report: &ConformanceCheckReport) -> sarif::SarifLog {
    let mut rules = Vec::new();
    let mut results = Vec::new();
    if report.outcome != ConformanceOutcome::Conformant {
        for reason in &report.reasons {
            let rule_id = format!("anvil.conformance.{reason}");
            rules.push(
                sarif::ReportingDescriptor::new(rule_id.clone())
                    .short_description("PR declaration conformance"),
            );
            results.push(sarif::SarifResult::new(
                rule_id,
                sarif::Level::Warning,
                format!(
                    "{}: {reason} (declaration grade weak, evidence {})",
                    outcome_label(report.outcome),
                    evidence_strength_label(report.evidence_strength)
                ),
            ));
        }
    }
    sarif::SarifLog::new(sarif::Run::new(rules, results))
}

fn outcome_label(outcome: ConformanceOutcome) -> &'static str {
    match outcome {
        ConformanceOutcome::Conformant => "conformant",
        ConformanceOutcome::NonConformant => "non-conformant",
        ConformanceOutcome::NotEvaluated => "not-evaluated",
    }
}

fn evidence_strength_label(strength: EvidenceStrength) -> &'static str {
    match strength {
        EvidenceStrength::Complete => "complete",
        EvidenceStrength::Partial => "partial",
        EvidenceStrength::Absent => "absent",
    }
}

#[cfg(test)]
mod tests {
    use anvil_checks::conformance::{GitBudgetDiagnostics, GitCommitNonEvaluation};

    use super::ConformanceCheckReport;

    #[test]
    fn json_report_retains_safe_budget_diagnostics_without_raw_detail() {
        let failure = GitCommitNonEvaluation {
            commit_revision: Some("b".repeat(40).into_boxed_str()),
            reason: "budget.records",
            stage: "diff",
            observed: 101,
            limit: Some(100),
            raw_digest: Some("sha256:safe".into()),
            budget: Some(Box::new(GitBudgetDiagnostics {
                configured_limit: 100,
                elapsed_millis: Some(9),
                commits: Some(2),
                records: Some(101),
                rename_sources: Some(3),
                rename_targets: Some(4),
                raw_bytes: Some(4096),
                decoded_bytes: Some(2048),
                raw_output_digest: Some("sha256:safe".into()),
            })),
        };

        let report = ConformanceCheckReport::not_evaluated_with_failures(
            vec!["git.commit-not-evaluated.budget.records".to_owned()],
            vec![failure],
        );
        let json = serde_json::to_value(report).expect("serialise safe report");

        assert_eq!(json["notEvaluatedCommitCount"], 1);
        assert_eq!(json["gitNonEvaluations"][0]["observed"], 101);
        assert_eq!(json["gitNonEvaluations"][0]["limit"], 100);
        assert_eq!(json["gitNonEvaluations"][0]["rawDigest"], "sha256:safe");
        assert_eq!(
            json["gitNonEvaluations"][0]["budget"]["configuredLimit"],
            100
        );
        assert_eq!(json["gitNonEvaluations"][0]["budget"]["records"], 101);
        assert_eq!(json["gitNonEvaluations"][0]["budget"]["rawBytes"], 4096);
        assert_eq!(
            json["gitNonEvaluations"][0]["budget"]["rawOutputDigest"],
            "sha256:safe"
        );
        assert!(!json.to_string().contains("detail"));
    }
}
