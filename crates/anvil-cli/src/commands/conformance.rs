//! Advisory external pull-request declaration conformance check.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anvil_checks::conformance::evaluate::evaluate_pr_declaration_bounded;
use anvil_checks::conformance::{
    ConformanceEvaluation, GitBudgetDiagnostics, GitCommitNonEvaluation, GitExtractor,
    GitFootprintExtractionOutcome, GitNonEvaluation, GitSelection, PR_BODY_MAX_BYTES,
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
const RUN_TIMEOUT: Duration = Duration::from_mins(5);
const REPORT_MAX_BYTES: usize = 128 * 1024 * 1024;

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
    #[serde(skip)]
    selected_commit_count: Option<usize>,
    schema_version: &'static str,
    advisory: bool,
    outcome: ConformanceOutcome,
    declaration_evidence_grade: EvidenceGrade,
    evidence_strength: EvidenceStrength,
    resolved_base: Option<String>,
    resolved_head: Option<String>,
    reasons: Vec<String>,
    not_evaluated_commit_count: Option<usize>,
    git_evaluation_non_evaluation: Option<GitCommitNonEvaluationReport>,
    git_non_evaluations: Vec<GitCommitNonEvaluationReport>,
    verdict: Option<ConformanceVerdict>,
}

impl ConformanceCheckReport {
    fn not_evaluated_with_git_failure(reasons: Vec<String>, git_failure: GitNonEvaluation) -> Self {
        let selected_commit_count = exact_selected_commit_count(&git_failure);
        Self {
            selected_commit_count,
            schema_version: REPORT_SCHEMA,
            advisory: true,
            outcome: ConformanceOutcome::NotEvaluated,
            declaration_evidence_grade: EvidenceGrade::Weak,
            evidence_strength: EvidenceStrength::Absent,
            resolved_base: None,
            resolved_head: None,
            reasons,
            not_evaluated_commit_count: selected_commit_count,
            git_evaluation_non_evaluation: Some(git_failure.into()),
            git_non_evaluations: Vec::new(),
            verdict: None,
        }
    }

    fn not_evaluated_with_failures(
        reasons: Vec<String>,
        git_non_evaluations: Vec<GitCommitNonEvaluation>,
    ) -> Self {
        let not_evaluated_commit_count = git_non_evaluations.len();
        Self {
            selected_commit_count: None,
            schema_version: REPORT_SCHEMA,
            advisory: true,
            outcome: ConformanceOutcome::NotEvaluated,
            declaration_evidence_grade: EvidenceGrade::Weak,
            evidence_strength: EvidenceStrength::Absent,
            resolved_base: None,
            resolved_head: None,
            reasons,
            not_evaluated_commit_count: Some(not_evaluated_commit_count),
            git_evaluation_non_evaluation: None,
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
        selected_commit_count: usize,
        git_non_evaluations: Vec<GitCommitNonEvaluation>,
    ) -> Self {
        let mut report = Self::not_evaluated_with_failures(reasons, git_non_evaluations);
        report.resolved_base = Some(resolved_base);
        report.resolved_head = Some(resolved_head);
        report.selected_commit_count = Some(selected_commit_count);
        report
    }

    fn from_evaluation(evaluation: ConformanceEvaluation, selected_commit_count: usize) -> Self {
        let ConformanceEvaluation {
            verdict,
            git_non_evaluations,
            ..
        } = evaluation;
        let not_evaluated_commit_count = git_non_evaluations.len();
        Self {
            selected_commit_count: Some(selected_commit_count),
            schema_version: REPORT_SCHEMA,
            advisory: true,
            outcome: verdict.outcome,
            declaration_evidence_grade: EvidenceGrade::Weak,
            evidence_strength: verdict.evidence_strength,
            resolved_base: Some(verdict.binding.base_revision.clone()),
            resolved_head: Some(verdict.binding.head_revision.clone()),
            reasons: verdict.reasons.clone(),
            not_evaluated_commit_count: Some(not_evaluated_commit_count),
            git_evaluation_non_evaluation: None,
            git_non_evaluations: git_non_evaluations
                .into_iter()
                .map(GitCommitNonEvaluationReport::from)
                .collect(),
            verdict: Some(verdict),
        }
    }
}

fn exact_selected_commit_count(failure: &GitNonEvaluation) -> Option<usize> {
    let has_exact_count = matches!(
        failure.stage,
        "range-aggregation" | "evaluation" | "report-materialisation"
    ) || (failure.stage == "revision-list"
        && failure.reason == "budget.commits");
    has_exact_count
        .then(|| failure.budget.as_deref().and_then(|budget| budget.commits))
        .flatten()
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

impl From<GitNonEvaluation> for GitCommitNonEvaluationReport {
    fn from(value: GitNonEvaluation) -> Self {
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
    let run_started = Instant::now();
    let pr_body = read_pr_body(&args.pr_body_file, run_started, RUN_TIMEOUT)?;
    if let Some(report) = immediate_pr_body_timeout_report(&pr_body, run_started, RUN_TIMEOUT) {
        return render_report(&report, args.render_mode(global), run_started, RUN_TIMEOUT);
    }
    let declaration = match pr_body {
        PrBodyInput::Body(body) => extract_pr_body_claims(&args.source_ref, &body)
            .into_contract_parts()
            .map_err(|non_evaluation| non_evaluation.reasons().to_vec()),
        PrBodyInput::NotEvaluated(reason) => Err(vec![reason]),
    };
    let repository =
        std::env::current_dir().context("resolve conformance repository entry path")?;
    let extractor = GitExtractor::default();
    let run_id = format!("conformance-{}", uuid::Uuid::new_v4());
    let git_evidence = match extractor.identity_for_repository_at(&repository, run_id, run_started)
    {
        Err(failure) => Err(failure),
        Ok(identity) => match extractor.extract_footprint(
            &repository,
            GitSelection::Range {
                base: args.base.clone(),
                head: args.head.clone(),
            },
            &identity,
        ) {
            GitFootprintExtractionOutcome::NotEvaluated(failure) => Err(failure),
            GitFootprintExtractionOutcome::Evaluated(extraction) => Ok((identity, extraction)),
        },
    };
    let report = match (declaration, git_evidence) {
        (Ok(declaration), Ok((identity, extraction))) => {
            match evaluate_pr_declaration_bounded(&declaration, &extraction, &identity) {
                Ok(evaluation) => {
                    ConformanceCheckReport::from_evaluation(evaluation, extraction.commits.len())
                }
                Err(failure) => {
                    let mut report = ConformanceCheckReport::not_evaluated_with_git_failure(
                        vec![failure.reason.to_owned()],
                        failure,
                    );
                    report.resolved_base = Some(extraction.base_revision);
                    report.resolved_head = Some(extraction.head_revision);
                    report
                }
            }
        }
        (Ok(_), Err(git_failure)) => ConformanceCheckReport::not_evaluated_with_git_failure(
            vec![git_failure.reason.to_owned()],
            git_failure,
        ),
        (Err(non_evaluation), Ok((identity, extraction))) => {
            let selected_commit_count = extraction.commits.len();
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
                selected_commit_count,
                git_non_evaluations,
            )
        }
        (Err(non_evaluation), Err(git_failure)) => {
            let mut reasons: Vec<String> = non_evaluation
                .iter()
                .map(|reason| (*reason).to_owned())
                .collect();
            reasons.push(git_failure.reason.to_owned());
            reasons.sort();
            reasons.dedup();
            ConformanceCheckReport::not_evaluated_with_git_failure(reasons, git_failure)
        }
    };

    render_report(&report, args.render_mode(global), run_started, RUN_TIMEOUT)
}

fn immediate_pr_body_timeout_report(
    input: &PrBodyInput,
    run_started: Instant,
    run_timeout: Duration,
) -> Option<ConformanceCheckReport> {
    matches!(input, PrBodyInput::NotEvaluated("budget.run-timeout")).then(|| {
        let failure = report_budget_failure(
            "budget.run-timeout",
            "pr-body-input",
            duration_millis(run_started.elapsed()),
            duration_millis(run_timeout),
            None,
        );
        ConformanceCheckReport::not_evaluated_with_git_failure(
            vec![failure.reason.to_owned()],
            failure,
        )
    })
}

fn read_pr_body(path: &Path, run_started: Instant, run_timeout: Duration) -> Result<PrBodyInput> {
    let path = path.to_path_buf();
    read_pr_body_task_with_deadline(
        move || read_pr_body_without_deadline(&path),
        run_started,
        run_timeout,
    )
}

fn read_pr_body_task_with_deadline<F>(
    read: F,
    run_started: Instant,
    run_timeout: Duration,
) -> Result<PrBodyInput>
where
    F: FnOnce() -> Result<PrBodyInput> + Send + 'static,
{
    let Some(remaining) = run_timeout.checked_sub(run_started.elapsed()) else {
        return Ok(PrBodyInput::NotEvaluated("budget.run-timeout"));
    };
    if remaining.is_zero() {
        return Ok(PrBodyInput::NotEvaluated("budget.run-timeout"));
    }

    // Opening a FIFO and reading a file or stdin can all block. Keep that work
    // off the command thread so the shared run deadline can still materialise
    // one bounded report. A timed-out reader is detached and dies with this
    // short-lived CLI process; it is never joined by the reporting path.
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("anvil-conformance-pr-body".to_owned())
        .spawn(move || {
            let _ = sender.send(read());
        })
        .context("start bounded PR-body reader")?;

    let read_result = receiver.recv_timeout(remaining);
    if run_started.elapsed() >= run_timeout {
        return Ok(PrBodyInput::NotEvaluated("budget.run-timeout"));
    }
    match read_result {
        Ok(input) => input,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            Ok(PrBodyInput::NotEvaluated("budget.run-timeout"))
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            anyhow::bail!("bounded PR-body reader stopped without a result")
        }
    }
}

fn read_pr_body_without_deadline(path: &Path) -> Result<PrBodyInput> {
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

fn render_report(
    report: &ConformanceCheckReport,
    mode: RenderMode,
    run_started: Instant,
    run_timeout: Duration,
) -> Result<()> {
    let bytes =
        materialise_report_with_limit(report, mode, REPORT_MAX_BYTES, run_started, run_timeout)?;
    io::stdout()
        .lock()
        .write_all(&bytes)
        .context("write conformance report")
}

fn materialise_report_with_limit(
    report: &ConformanceCheckReport,
    mode: RenderMode,
    max_bytes: usize,
    run_started: Instant,
    run_timeout: Duration,
) -> Result<Vec<u8>> {
    let failure = if run_started.elapsed() >= run_timeout {
        Some(report_budget_failure(
            "budget.run-timeout",
            "report-materialisation",
            duration_millis(run_started.elapsed()),
            duration_millis(run_timeout),
            report.selected_commit_count,
        ))
    } else {
        match materialise_report_once(report, mode, max_bytes)? {
            Some(bytes) if run_started.elapsed() < run_timeout => return Ok(bytes),
            Some(_) => Some(report_budget_failure(
                "budget.run-timeout",
                "report-materialisation",
                duration_millis(run_started.elapsed()),
                duration_millis(run_timeout),
                report.selected_commit_count,
            )),
            None => Some(report_budget_failure(
                "budget.report-bytes",
                "report-materialisation",
                max_bytes.saturating_add(1),
                max_bytes,
                report.selected_commit_count,
            )),
        }
    };
    let failure = failure.expect("materialisation failure selected");
    let mut fallback = ConformanceCheckReport::not_evaluated_with_git_failure(
        vec![failure.reason.to_owned()],
        failure,
    );
    fallback.resolved_base.clone_from(&report.resolved_base);
    fallback.resolved_head.clone_from(&report.resolved_head);
    materialise_report_once(&fallback, mode, max_bytes)?.ok_or_else(|| {
        anyhow::anyhow!("bounded conformance fallback exceeds its materialisation limit")
    })
}

fn materialise_report_once(
    report: &ConformanceCheckReport,
    mode: RenderMode,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>> {
    let mut output = CappedBuffer::new(max_bytes);
    let result = match mode {
        RenderMode::Plain => render_plain(&mut output, report).map_err(anyhow::Error::from),
        RenderMode::Json => serde_json::to_writer(&mut output, report).map_err(anyhow::Error::from),
        RenderMode::Sarif => {
            serde_json::to_writer(&mut output, &build_sarif(report)).map_err(anyhow::Error::from)
        }
    };
    if output.exceeded {
        return Ok(None);
    }
    result?;
    if let Err(error) = output.write_all(b"\n") {
        if output.exceeded {
            return Ok(None);
        }
        return Err(error.into());
    }
    Ok(Some(output.bytes))
}

struct CappedBuffer {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl CappedBuffer {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1024)),
            limit,
            exceeded: false,
        }
    }
}

impl Write for CappedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "conformance report materialisation limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn report_budget_failure(
    reason: &'static str,
    stage: &'static str,
    observed: usize,
    limit: usize,
    commits: Option<usize>,
) -> GitNonEvaluation {
    GitNonEvaluation {
        commit_revision: None,
        reason,
        stage,
        observed,
        limit: Some(limit),
        detail: "bounded report materialisation failed".into(),
        raw_digest: None,
        budget: Some(Box::new(GitBudgetDiagnostics {
            configured_limit: limit,
            elapsed_millis: (reason == "budget.run-timeout").then_some(observed),
            commits,
            records: None,
            rename_sources: None,
            rename_targets: None,
            raw_bytes: None,
            decoded_bytes: None,
            raw_output_digest: None,
        })),
    }
}

fn duration_millis(duration: Duration) -> usize {
    usize::try_from(duration.as_millis()).unwrap_or(usize::MAX)
}

fn render_plain(output: &mut impl Write, report: &ConformanceCheckReport) -> io::Result<()> {
    writeln!(
        output,
        "Conformance check: {}",
        outcome_label(report.outcome)
    )?;
    writeln!(output, "Advisory: yes")?;
    writeln!(output, "Declaration evidence grade: weak")?;
    writeln!(
        output,
        "Evidence strength: {}",
        evidence_strength_label(report.evidence_strength)
    )?;
    if let Some(base) = &report.resolved_base {
        writeln!(output, "Resolved base: {base}")?;
    }
    if let Some(head) = &report.resolved_head {
        writeln!(output, "Resolved head: {head}")?;
    }
    writeln!(
        output,
        "Not-evaluated commits: {}",
        report
            .not_evaluated_commit_count
            .map_or_else(|| "unknown".to_owned(), |count| count.to_string())
    )?;
    if let Some(failure) = &report.git_evaluation_non_evaluation {
        writeln!(
            output,
            "Git evaluation non-evaluation: {}",
            render_git_non_evaluation(failure)
        )?;
    }
    for failure in &report.git_non_evaluations {
        writeln!(
            output,
            "Git non-evaluation: {}",
            render_git_non_evaluation(failure)
        )?;
    }
    for reason in &report.reasons {
        writeln!(output, "Reason: {reason}")?;
    }
    Ok(())
}

fn build_sarif(report: &ConformanceCheckReport) -> sarif::SarifLog {
    let mut rules = BTreeMap::new();
    let mut results = Vec::new();
    if report.outcome != ConformanceOutcome::Conformant {
        let mut represented_reasons: BTreeSet<_> = report
            .git_non_evaluations
            .iter()
            .map(git_non_evaluation_summary_reason)
            .collect();
        if let Some(failure) = &report.git_evaluation_non_evaluation {
            represented_reasons.insert(failure.reason.to_owned());
        }
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
        if let Some(failure) = &report.git_evaluation_non_evaluation {
            let rule_id = format!(
                "anvil.conformance.git.evaluation-not-evaluated.{}",
                failure.reason
            );
            rules.insert(
                rule_id.clone(),
                sarif::ReportingDescriptor::new(rule_id.clone())
                    .short_description("Git evaluation evidence unavailable"),
            );
            let fingerprint = sarif::stable_fingerprint(
                &rule_id,
                "git-evaluation",
                None,
                &format!("{}:{}", failure.reason, failure.stage),
            );
            results.push(
                sarif::SarifResult::new(
                    rule_id,
                    sarif::Level::Warning,
                    format!(
                        "{}: Git evaluation non-evaluation: {}",
                        outcome_label(report.outcome),
                        render_git_non_evaluation(failure)
                    ),
                )
                .fingerprint("anvilConformanceEvaluation/v1", fingerprint),
            );
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
    let run = sarif::Run::new(rules.into_values().collect(), results)
        .properties(sarif_run_properties(report));
    sarif::SarifLog::new(run)
}

fn sarif_run_properties(report: &ConformanceCheckReport) -> BTreeMap<String, sarif::PropertyValue> {
    let mut properties = BTreeMap::new();
    properties.insert("schemaVersion".to_owned(), REPORT_SCHEMA.into());
    properties.insert("advisory".to_owned(), report.advisory.into());
    properties.insert("outcome".to_owned(), outcome_label(report.outcome).into());
    properties.insert(
        "declarationEvidenceGrade".to_owned(),
        evidence_grade_label(report.declaration_evidence_grade).into(),
    );
    properties.insert(
        "evidenceStrength".to_owned(),
        evidence_strength_label(report.evidence_strength).into(),
    );
    properties.insert(
        "resolvedBase".to_owned(),
        optional_string_property(report.resolved_base.as_deref()),
    );
    properties.insert(
        "resolvedHead".to_owned(),
        optional_string_property(report.resolved_head.as_deref()),
    );
    properties.insert(
        "notEvaluatedCommitCount".to_owned(),
        optional_usize_property(report.not_evaluated_commit_count),
    );
    properties.insert(
        "gitEvaluationNonEvaluation".to_owned(),
        report
            .git_evaluation_non_evaluation
            .as_ref()
            .map_or(sarif::PropertyValue::Null, git_non_evaluation_property),
    );
    properties
}

fn git_non_evaluation_property(failure: &GitCommitNonEvaluationReport) -> sarif::PropertyValue {
    let mut properties = BTreeMap::new();
    properties.insert(
        "commitRevision".to_owned(),
        optional_string_property(failure.commit_revision.as_deref()),
    );
    properties.insert("reason".to_owned(), failure.reason.into());
    properties.insert("stage".to_owned(), failure.stage.into());
    properties.insert("observed".to_owned(), usize_property(failure.observed));
    properties.insert("limit".to_owned(), optional_usize_property(failure.limit));
    properties.insert(
        "rawDigest".to_owned(),
        optional_string_property(failure.raw_digest.as_deref()),
    );
    properties.insert(
        "budget".to_owned(),
        failure
            .budget
            .as_ref()
            .map_or(sarif::PropertyValue::Null, git_budget_property),
    );
    sarif::PropertyValue::Object(properties)
}

fn git_budget_property(budget: &GitBudgetDiagnosticsReport) -> sarif::PropertyValue {
    let mut properties = BTreeMap::new();
    properties.insert(
        "configuredLimit".to_owned(),
        usize_property(budget.configured_limit),
    );
    properties.insert(
        "elapsedMillis".to_owned(),
        optional_usize_property(budget.elapsed_millis),
    );
    properties.insert(
        "commits".to_owned(),
        optional_usize_property(budget.commits),
    );
    properties.insert(
        "records".to_owned(),
        optional_usize_property(budget.records),
    );
    properties.insert(
        "renameSources".to_owned(),
        optional_usize_property(budget.rename_sources),
    );
    properties.insert(
        "renameTargets".to_owned(),
        optional_usize_property(budget.rename_targets),
    );
    properties.insert(
        "rawBytes".to_owned(),
        optional_usize_property(budget.raw_bytes),
    );
    properties.insert(
        "decodedBytes".to_owned(),
        optional_usize_property(budget.decoded_bytes),
    );
    properties.insert(
        "rawOutputDigest".to_owned(),
        optional_string_property(budget.raw_output_digest.as_deref()),
    );
    sarif::PropertyValue::Object(properties)
}

fn optional_string_property(value: Option<&str>) -> sarif::PropertyValue {
    value.map_or(sarif::PropertyValue::Null, Into::into)
}

fn optional_usize_property(value: Option<usize>) -> sarif::PropertyValue {
    value.map_or(sarif::PropertyValue::Null, usize_property)
}

fn usize_property(value: usize) -> sarif::PropertyValue {
    sarif::PropertyValue::Integer(
        u64::try_from(value).expect("usize conformance counters fit SARIF unsigned integers"),
    )
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

fn evidence_grade_label(grade: EvidenceGrade) -> &'static str {
    match grade {
        EvidenceGrade::Strong => "strong",
        EvidenceGrade::Moderate => "moderate",
        EvidenceGrade::Weak => "weak",
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
    use std::time::{Duration, Instant};

    use anvil_checks::conformance::{
        GitBudgetDiagnostics, GitCommitNonEvaluation, GitNonEvaluation,
    };

    use super::{
        ConformanceCheckReport, PrBodyInput, RenderMode, build_sarif,
        immediate_pr_body_timeout_report, materialise_report_with_limit,
        read_pr_body_task_with_deadline,
    };

    #[test]
    fn timed_out_pr_body_input_selects_the_immediate_report_boundary() {
        let report = immediate_pr_body_timeout_report(
            &PrBodyInput::NotEvaluated("budget.run-timeout"),
            Instant::now(),
            Duration::from_mins(5),
        )
        .expect("timeout must bypass repository and Git work");

        assert_eq!(report.reasons, ["budget.run-timeout"]);
        let failure = report
            .git_evaluation_non_evaluation
            .expect("structured timeout diagnostics");
        assert_eq!(failure.reason, "budget.run-timeout");
        assert_eq!(failure.stage, "pr-body-input");
        assert!(
            immediate_pr_body_timeout_report(
                &PrBodyInput::NotEvaluated("claim.pr-body.encoding.invalid-utf8"),
                Instant::now(),
                Duration::from_mins(5),
            )
            .is_none()
        );
    }

    #[test]
    fn blocked_pr_body_reader_returns_a_timeout_at_the_shared_deadline() {
        let (reader_started_sender, reader_started_receiver) = std::sync::mpsc::sync_channel(1);
        let (release_sender, release_receiver) = std::sync::mpsc::sync_channel(1);
        let run_started = Instant::now();
        let timeout = Duration::from_millis(50);

        let input = read_pr_body_task_with_deadline(
            move || {
                reader_started_sender.send(()).expect("record reader start");
                release_receiver.recv().expect("release blocked reader");
                Ok(PrBodyInput::Body("too late".to_owned()))
            },
            run_started,
            timeout,
        )
        .expect("deadline is a semantic non-evaluation");

        reader_started_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("reader task started");
        assert!(run_started.elapsed() >= timeout);
        assert!(run_started.elapsed() < Duration::from_secs(1));
        assert!(matches!(
            input,
            PrBodyInput::NotEvaluated("budget.run-timeout")
        ));
        release_sender.send(()).expect("release reader task");
    }

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
    fn top_level_git_budget_failure_keeps_safe_structured_diagnostics() {
        let failure = GitNonEvaluation {
            commit_revision: None,
            reason: "budget.commits",
            stage: "revision-list",
            observed: 10_001,
            limit: Some(10_000),
            detail: "must not be reported".into(),
            raw_digest: Some("sha256:selection".into()),
            budget: Some(Box::new(GitBudgetDiagnostics {
                configured_limit: 10_000,
                elapsed_millis: Some(17),
                commits: Some(10_001),
                records: None,
                rename_sources: None,
                rename_targets: None,
                raw_bytes: Some(410_000),
                decoded_bytes: None,
                raw_output_digest: Some("sha256:selection".into()),
            })),
        };
        let report = ConformanceCheckReport::not_evaluated_with_git_failure(
            vec!["budget.commits".to_owned()],
            failure,
        );

        let json = serde_json::to_value(&report).expect("serialise safe report");
        assert_eq!(json["notEvaluatedCommitCount"], 10_001);
        assert_eq!(json["gitEvaluationNonEvaluation"]["observed"], 10_001);
        assert_eq!(json["gitEvaluationNonEvaluation"]["limit"], 10_000);
        assert_eq!(
            json["gitEvaluationNonEvaluation"]["rawDigest"],
            "sha256:selection"
        );
        assert_eq!(
            json["gitEvaluationNonEvaluation"]["budget"]["commits"],
            10_001
        );
        assert!(
            json["gitNonEvaluations"]
                .as_array()
                .expect("per-commit failures")
                .is_empty()
        );
        assert!(!json.to_string().contains("must not be reported"));

        let sarif = serde_json::to_value(build_sarif(&report)).expect("serialise SARIF");
        let result = &sarif["runs"][0]["results"][0];
        assert_eq!(
            result["ruleId"],
            "anvil.conformance.git.evaluation-not-evaluated.budget.commits"
        );
        let message = result["message"]["text"].as_str().expect("message");
        assert!(message.contains("observed=10001 limit=10000"));
        assert!(message.contains("budget.commits=10001"));
        assert!(message.contains("budget.raw-bytes=410000"));
        assert!(!message.contains("must not be reported"));
    }

    #[test]
    fn truncated_revision_list_keeps_selected_cardinality_unknown() {
        let failure = GitNonEvaluation {
            commit_revision: None,
            reason: "budget.commits",
            stage: "revision-list",
            observed: 10_001,
            limit: Some(10_000),
            detail: "must not be reported".into(),
            raw_digest: Some("sha256:truncated-selection".into()),
            budget: Some(Box::new(GitBudgetDiagnostics {
                configured_limit: 10_000,
                elapsed_millis: Some(17),
                commits: None,
                records: None,
                rename_sources: None,
                rename_targets: None,
                raw_bytes: Some(1_419_129),
                decoded_bytes: None,
                raw_output_digest: Some("sha256:truncated-selection".into()),
            })),
        };
        let report = ConformanceCheckReport::not_evaluated_with_git_failure(
            vec!["budget.commits".to_owned()],
            failure,
        );

        let json = serde_json::to_value(report).expect("serialise safe report");
        assert!(json["notEvaluatedCommitCount"].is_null());
        assert!(json["gitEvaluationNonEvaluation"]["budget"]["commits"].is_null());
        assert!(!json.to_string().contains("must not be reported"));
    }

    #[test]
    fn resolved_range_budget_failure_keeps_exact_selected_cardinality() {
        let failure = GitNonEvaluation {
            commit_revision: Some("c".repeat(40).into_boxed_str()),
            reason: "budget.evaluation-records",
            stage: "range-aggregation",
            observed: 100_001,
            limit: Some(100_000),
            detail: "must not be reported".into(),
            raw_digest: None,
            budget: Some(Box::new(GitBudgetDiagnostics {
                configured_limit: 100_000,
                elapsed_millis: None,
                commits: Some(17),
                records: Some(100_001),
                rename_sources: None,
                rename_targets: None,
                raw_bytes: None,
                decoded_bytes: None,
                raw_output_digest: None,
            })),
        };
        let report = ConformanceCheckReport::not_evaluated_with_git_failure(
            vec!["budget.evaluation-records".to_owned()],
            failure,
        );

        let json = serde_json::to_value(report).expect("serialise range budget report");
        assert_eq!(json["notEvaluatedCommitCount"], 17);
        assert_eq!(
            json["gitEvaluationNonEvaluation"]["stage"],
            "range-aggregation"
        );
        assert_eq!(
            json["gitEvaluationNonEvaluation"]["budget"]["records"],
            100_001
        );
        assert!(!json.to_string().contains("must not be reported"));
    }

    #[test]
    fn all_renderers_replace_oversized_or_timed_out_reports_atomically() {
        let failures = (0..64)
            .map(|index| GitCommitNonEvaluation {
                commit_revision: Some(format!("{index:040x}").into_boxed_str()),
                reason: "budget.records",
                stage: "diff",
                observed: 101,
                limit: Some(100),
                raw_digest: None,
                budget: None,
            })
            .collect();
        let mut report = ConformanceCheckReport::not_evaluated_with_failures(
            vec!["x".repeat(8 * 1024)],
            failures,
        );
        report.selected_commit_count = Some(64);

        for mode in [RenderMode::Plain, RenderMode::Json, RenderMode::Sarif] {
            let bytes = materialise_report_with_limit(
                &report,
                mode,
                8 * 1024,
                Instant::now(),
                Duration::from_secs(1),
            )
            .expect("small fallback report");
            let rendered = String::from_utf8(bytes).expect("UTF-8 output");
            assert!(rendered.contains("budget.report-bytes"));
            assert!(!rendered.contains(&"x".repeat(8 * 1024)));
            if mode == RenderMode::Json {
                let json: serde_json::Value = serde_json::from_str(&rendered).expect("one JSON");
                assert_eq!(json["notEvaluatedCommitCount"], 64);
                assert_eq!(
                    json["gitEvaluationNonEvaluation"]["reason"],
                    "budget.report-bytes"
                );
                assert_eq!(json["gitNonEvaluations"], serde_json::json!([]));
            }
        }

        let timeout = materialise_report_with_limit(
            &report,
            RenderMode::Json,
            128 * 1024 * 1024,
            Instant::now(),
            Duration::ZERO,
        )
        .expect("small timeout fallback");
        let json: serde_json::Value = serde_json::from_slice(&timeout).expect("one JSON fallback");
        assert_eq!(
            json["gitEvaluationNonEvaluation"]["reason"],
            "budget.run-timeout"
        );
        assert_eq!(json["notEvaluatedCommitCount"], 64);
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
