//! Advisory external pull-request declaration conformance check.

use std::collections::{BTreeMap, BTreeSet};
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

const REPORT_SCHEMA: &str = "anvil.conformance-check.v1";
const PR_BODY_OVER_BUDGET_REASON: &str = "claim.pr-body.budget.body-bytes";
const PR_BODY_INVALID_UTF8_REASON: &str = "claim.pr-body.encoding.invalid-utf8";

#[derive(Debug)]
enum PrBodyInput {
    Body(String),
    NotEvaluated(&'static str),
}

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
    let declaration = match read_pr_body(&args.pr_body_file)? {
        PrBodyInput::Body(body) => extract_pr_body_claims(&args.source_ref, &body)
            .into_contract_parts()
            .map_err(|non_evaluation| non_evaluation.reasons().to_vec()),
        PrBodyInput::NotEvaluated(reason) => Err(vec![reason]),
    };
    let repository =
        std::env::current_dir().context("resolve conformance repository entry path")?;
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
            let git_non_evaluations = pr_git_footprint_non_evaluations(&extraction, &identity);
            let mut reasons: Vec<String> = non_evaluation
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

fn read_pr_body(path: &PathBuf) -> Result<PrBodyInput> {
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
    if bytes.len() > PR_BODY_MAX_BYTES {
        return Ok(PrBodyInput::NotEvaluated(PR_BODY_OVER_BUDGET_REASON));
    }
    Ok(String::from_utf8(bytes).map_or(
        PrBodyInput::NotEvaluated(PR_BODY_INVALID_UTF8_REASON),
        PrBodyInput::Body,
    ))
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
    println!(
        "Not-evaluated commits: {}",
        report.not_evaluated_commit_count
    );
    for failure in &report.git_non_evaluations {
        println!("Git non-evaluation: {}", render_git_non_evaluation(failure));
    }
    for reason in &report.reasons {
        println!("Reason: {reason}");
    }
}

fn build_sarif(report: &ConformanceCheckReport) -> sarif::SarifLog {
    let mut rules = BTreeMap::new();
    let mut results = Vec::new();
    if report.outcome != ConformanceOutcome::Conformant {
        let represented_reasons: BTreeSet<_> = report
            .git_non_evaluations
            .iter()
            .map(git_non_evaluation_summary_reason)
            .collect();
        for reason in &report.reasons {
            if represented_reasons.contains(reason) {
                continue;
            }
            let rule_id = format!("anvil.conformance.{reason}");
            rules.insert(
                rule_id.clone(),
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
        for failure in &report.git_non_evaluations {
            let reason = git_non_evaluation_summary_reason(failure);
            let rule_id = format!("anvil.conformance.{reason}");
            rules.entry(rule_id.clone()).or_insert_with(|| {
                sarif::ReportingDescriptor::new(rule_id.clone())
                    .short_description("Git commit conformance evidence unavailable")
            });
            let commit_revision = failure.commit_revision.as_deref().unwrap_or("unknown");
            let fingerprint = sarif::stable_fingerprint(
                &rule_id,
                commit_revision,
                None,
                &format!("{}:{}", failure.reason, failure.stage),
            );
            results.push(
                sarif::SarifResult::new(
                    rule_id,
                    sarif::Level::Warning,
                    format!(
                        "{}: {}",
                        outcome_label(report.outcome),
                        render_git_non_evaluation(failure)
                    ),
                )
                .fingerprint("anvilConformanceCommit/v1", fingerprint),
            );
        }
    }
    sarif::SarifLog::new(sarif::Run::new(rules.into_values().collect(), results))
}

fn git_non_evaluation_summary_reason(failure: &GitCommitNonEvaluationReport) -> String {
    if failure.stage == "binding" {
        failure.reason.to_owned()
    } else {
        format!("git.commit-not-evaluated.{}", failure.reason)
    }
}

fn render_git_non_evaluation(failure: &GitCommitNonEvaluationReport) -> String {
    let mut rendered = format!(
        "commit={} reason={} stage={} observed={} limit={} raw-digest={}",
        failure.commit_revision.as_deref().unwrap_or("unknown"),
        failure.reason,
        failure.stage,
        failure.observed,
        optional_usize(failure.limit),
        failure.raw_digest.as_deref().unwrap_or("none")
    );
    if let Some(budget) = &failure.budget {
        use std::fmt::Write as _;
        write!(
            rendered,
            " budget.configured-limit={} budget.elapsed-millis={} budget.commits={} budget.records={} budget.rename-sources={} budget.rename-targets={} budget.raw-bytes={} budget.decoded-bytes={} budget.raw-output-digest={}",
            budget.configured_limit,
            optional_usize(budget.elapsed_millis),
            optional_usize(budget.commits),
            optional_usize(budget.records),
            optional_usize(budget.rename_sources),
            optional_usize(budget.rename_targets),
            optional_usize(budget.raw_bytes),
            optional_usize(budget.decoded_bytes),
            budget.raw_output_digest.as_deref().unwrap_or("none")
        )
        .expect("writing to a String cannot fail");
    }
    rendered
}

fn optional_usize(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
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

    use super::{ConformanceCheckReport, build_sarif};

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

    #[test]
    fn sarif_replaces_one_binding_summary_with_each_affected_commit() {
        let failures = ["b", "c"].map(|seed| GitCommitNonEvaluation {
            commit_revision: Some(seed.repeat(40).into_boxed_str()),
            reason: "binding.run-id-mismatch",
            stage: "binding",
            observed: 0,
            limit: None,
            raw_digest: None,
            budget: None,
        });
        let report = ConformanceCheckReport::not_evaluated_with_failures(
            vec!["binding.run-id-mismatch".to_owned()],
            failures.into(),
        );

        let json = serde_json::to_value(build_sarif(&report)).expect("serialise SARIF");
        let rules = json["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules");
        let results = json["runs"][0]["results"].as_array().expect("results");

        assert_eq!(rules.len(), 1);
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .all(|result| result["ruleId"] == "anvil.conformance.binding.run-id-mismatch")
        );
        let fingerprints: std::collections::BTreeSet<_> = results
            .iter()
            .map(|result| {
                result["partialFingerprints"]["anvilConformanceCommit/v1"]
                    .as_str()
                    .expect("fingerprint")
            })
            .collect();
        assert_eq!(fingerprints.len(), 2);
    }
}
