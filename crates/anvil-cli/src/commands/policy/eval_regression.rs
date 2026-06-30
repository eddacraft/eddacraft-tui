//! EVAL-003 — `anvil policy eval-regression`: the CI trust-regression command.
//!
//! Runs a set of trust/safety eval suites through the
//! [`EvalHarnessPort`](anvil_policy::eval::EvalHarnessPort), compares each run
//! to the most recent persisted run of the same suite, and emits a pass/fail +
//! delta summary so trust regressions become a standard CI check.
//!
//! Exit-code posture follows the repo contract (ADR-002, warnings over blocks):
//! the command is **report-only by default** (exit 0) and only blocks CI when
//! `--fail-on-regression` is passed — mirroring `policy eval`'s
//! `--fail-on-warnings`. The verdict is always in the output regardless.
//!
//! Suites are defined in a JSON file (`--suites`): an array of
//! [`EvalSuite`](anvil_policy::eval::EvalSuite). History persists under
//! `<ANVIL_HOME>/eval` (override with `--store`) and is appended to only when
//! `--update-baseline` is given, so a dry CI run never mutates the baseline.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use anvil_policy::eval::{
    EvalHarnessPort, EvalRegressionReport, EvalResultStore, EvalRunSummary, EvalSuite,
    GuidedFinding, PolicyEvalAdapter, SubprocessRunner, guidance_for,
};

use crate::GlobalArgs;
use crate::install_root;
use crate::output;

#[derive(Debug, Args)]
pub struct EvalRegressionArgs {
    /// Path to a JSON file defining the suites to run (an array of eval
    /// suites: `{ "name", "policy", "query", "input"? }`).
    #[arg(long, value_name = "PATH")]
    suites: PathBuf,

    /// Directory holding the eval history. Defaults to `<ANVIL_HOME>/eval`,
    /// falling back to `.anvil/eval` when no Anvil home is resolvable.
    #[arg(long, value_name = "DIR")]
    store: Option<PathBuf>,

    /// The `anvil` executable used to run each suite. Defaults to the current
    /// executable so a CI checkout self-hosts without a PATH install.
    #[arg(long, value_name = "PATH")]
    anvil_bin: Option<PathBuf>,

    /// Append each run to the history, updating the baseline future runs
    /// compare against. Off by default so a CI gate run is side-effect free.
    #[arg(long)]
    update_baseline: bool,

    /// Block (exit non-zero) when any suite regressed. Off by default, so the
    /// command reports without failing the build (warnings over blocks).
    #[arg(long)]
    fail_on_regression: bool,
}

/// Per-suite regression outcome (the serialised `--json` shape).
#[derive(Debug, Clone, Serialize)]
struct SuiteOutcome {
    suite: String,
    /// The current run's own gate verdict.
    passed: bool,
    /// Whether the current run regressed against its baseline.
    regressed: bool,
    current_exit_code: i32,
    baseline_exit_code: Option<i32>,
    new_findings: usize,
    resolved_findings: usize,
    /// Remediation guidance, present only for a failing current run.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    guidance: Vec<GuidedFinding>,
}

/// The aggregate regression report for the whole run.
#[derive(Debug, Clone, Serialize)]
struct RegressionOutput {
    suites: Vec<SuiteOutcome>,
    suites_run: usize,
    regressions: usize,
    /// True when any suite regressed — the headline pass/fail.
    regressed: bool,
}

/// Build the aggregate outcome from each suite's current run paired with its
/// optional baseline run. Pure — no IO — so the regression semantics are
/// directly testable.
fn build_outcome(runs: &[(EvalRunSummary, Option<EvalRunSummary>)]) -> RegressionOutput {
    let suites: Vec<SuiteOutcome> = runs
        .iter()
        .map(|(current, baseline)| {
            let report = EvalRegressionReport::compare(baseline.as_ref(), current);
            let guidance = guidance_for(current);
            SuiteOutcome {
                suite: current.suite.clone(),
                passed: current.passed(),
                regressed: report.regressed(),
                current_exit_code: report.current_exit_code,
                baseline_exit_code: report.baseline_exit_code,
                new_findings: report.new_findings.len(),
                resolved_findings: report.resolved_findings.len(),
                guidance,
            }
        })
        .collect();

    let regressions = suites.iter().filter(|s| s.regressed).count();
    RegressionOutput {
        suites_run: suites.len(),
        regressions,
        regressed: regressions > 0,
        suites,
    }
}

/// Index the latest persisted run per suite, reading the history exactly once.
/// Later records overwrite earlier ones, so the map holds the most recent run
/// per suite (history is chronological).
fn latest_per_suite(
    store: &EvalResultStore,
) -> Result<std::collections::HashMap<String, EvalRunSummary>> {
    let mut latest = std::collections::HashMap::new();
    for record in store.all().context("reading eval history")? {
        latest.insert(record.suite.clone(), record.to_summary());
    }
    Ok(latest)
}

/// Resolve the eval history directory: explicit `--store`, else
/// `<ANVIL_HOME>/eval`, else `.anvil/eval`.
fn resolve_store_dir(explicit: Option<&PathBuf>) -> PathBuf {
    if let Some(dir) = explicit {
        return dir.clone();
    }
    install_root::install_root()
        .user_dir()
        .map_or_else(|| PathBuf::from(".anvil").join("eval"), |d| d.join("eval"))
}

pub fn run(args: &EvalRegressionArgs, global: &GlobalArgs) -> Result<()> {
    let suites = load_suites(&args.suites)?;

    let program = match &args.anvil_bin {
        Some(path) => path.clone(),
        // Per the operator-config rule, do not silently fall back to a
        // PATH-resolved `anvil` (which could be a different version) — surface
        // the failure so the operator passes `--anvil-bin` explicitly.
        None => std::env::current_exe()
            .context("resolving the current `anvil` executable; pass --anvil-bin to override")?,
    };
    let adapter = PolicyEvalAdapter::new(SubprocessRunner::new(program));

    let store = EvalResultStore::new(resolve_store_dir(args.store.as_ref()));

    // Read the history once and index the latest run per suite, rather than
    // re-reading and re-parsing the whole file once per suite (O(N+H) not
    // O(N*H)).
    let baselines = latest_per_suite(&store)?;

    let mut runs: Vec<(EvalRunSummary, Option<EvalRunSummary>)> = Vec::with_capacity(suites.len());
    for suite in &suites {
        let current = adapter
            .run_suite(suite)
            .with_context(|| format!("running eval suite `{}`", suite.name))?;
        let baseline = baselines.get(&suite.name).cloned();
        runs.push((current, baseline));
    }

    if args.update_baseline {
        persist_runs(&store, &runs)?;
    }

    let outcome = build_outcome(&runs);

    if global.json {
        output::json::print(&outcome)?;
    } else {
        render_plain(&outcome);
    }

    if outcome.regressed && args.fail_on_regression {
        return Err(output::AlreadyReported.into());
    }
    Ok(())
}

/// Append every current run to the history. The `run_id`/timestamp come from
/// the wall clock here; the store itself stays time-agnostic.
fn persist_runs(
    store: &EvalResultStore,
    runs: &[(EvalRunSummary, Option<EvalRunSummary>)],
) -> Result<()> {
    let now = chrono::Utc::now();
    let recorded_at = now.to_rfc3339();
    for (current, _) in runs {
        let run_id = format!("{}-{}", current.suite, now.timestamp_millis());
        let record = anvil_policy::eval::EvalRecord::from_summary(current, run_id, &recorded_at);
        store
            .append(&record)
            .with_context(|| format!("persisting eval run for suite `{}`", current.suite))?;
    }
    Ok(())
}

fn load_suites(path: &PathBuf) -> Result<Vec<EvalSuite>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading suites file `{}`", path.display()))?;
    let suites: Vec<EvalSuite> = serde_json::from_str(&raw)
        .with_context(|| format!("parsing `{}` as an array of eval suites", path.display()))?;
    if suites.is_empty() {
        anyhow::bail!("suites file `{}` defines no suites", path.display());
    }
    Ok(suites)
}

fn render_plain(outcome: &RegressionOutput) {
    use crate::output::plain;

    plain::blank();
    plain::section("Eval regression");
    for suite in &outcome.suites {
        let icon = if suite.regressed {
            "\u{2717}" // ✗
        } else if suite.passed {
            "\u{2713}" // ✓
        } else {
            "\u{25cb}" // ○ failing but not a new regression
        };
        println!(
            "  {icon} {name:<24} new:{new} resolved:{resolved} exit:{exit}",
            name = suite.suite,
            new = suite.new_findings,
            resolved = suite.resolved_findings,
            exit = suite.current_exit_code,
        );
        if suite.regressed {
            for guided in &suite.guidance {
                plain::warn(&format!("    {}", guided.guidance.summary));
                for action in &guided.guidance.next_actions {
                    plain::info(&format!("      - {action}"));
                }
            }
        }
    }
    plain::blank();
    if outcome.regressed {
        plain::error(&format!(
            "{} of {} suite(s) regressed",
            outcome.regressions, outcome.suites_run
        ));
    } else {
        plain::success(&format!(
            "no regressions across {} suite(s)",
            outcome.suites_run
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_policy::eval::{EvalFinding, EvalSeverity};
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: EvalRegressionArgs,
    }

    fn finding(sev: EvalSeverity, msg: &str, fp: Option<&str>) -> EvalFinding {
        EvalFinding {
            severity: sev,
            message: msg.into(),
            from: None,
            to: None,
            fingerprint: fp.map(Into::into),
        }
    }

    fn summary(suite: &str, exit: i32, findings: Vec<EvalFinding>) -> EvalRunSummary {
        EvalRunSummary {
            suite: suite.into(),
            schema_version: "1.0.0".into(),
            policy: "p.rego".into(),
            query: "data.anvil.findings".into(),
            findings,
            exit_code: exit,
        }
    }

    #[test]
    fn eval_regression_command_parses_args() {
        let w = Wrapper::try_parse_from([
            "test",
            "--suites",
            "suites.json",
            "--update-baseline",
            "--fail-on-regression",
        ])
        .expect("parse");
        assert_eq!(w.inner.suites, PathBuf::from("suites.json"));
        assert!(w.inner.update_baseline);
        assert!(w.inner.fail_on_regression);
    }

    #[test]
    fn eval_regression_command_requires_suites_flag() {
        // `--suites` is mandatory; parsing without it must fail.
        assert!(Wrapper::try_parse_from(["test"]).is_err());
    }

    #[test]
    fn eval_regression_command_clean_run_has_no_regression() {
        let cur = summary(
            "arch",
            0,
            vec![finding(EvalSeverity::Warning, "advisory", Some("a"))],
        );
        let base = summary(
            "arch",
            0,
            vec![finding(EvalSeverity::Warning, "advisory", Some("a"))],
        );
        let outcome = build_outcome(&[(cur, Some(base))]);
        assert!(!outcome.regressed);
        assert_eq!(outcome.regressions, 0);
        assert_eq!(outcome.suites_run, 1);
        assert!(outcome.suites[0].passed);
        assert!(outcome.suites[0].guidance.is_empty());
    }

    #[test]
    fn eval_regression_command_flags_new_blocking_finding() {
        let base = summary("arch", 0, vec![]);
        let cur = summary(
            "arch",
            1,
            vec![finding(EvalSeverity::Error, "ui->db", Some("b"))],
        );
        let outcome = build_outcome(&[(cur, Some(base))]);

        assert!(outcome.regressed);
        assert_eq!(outcome.regressions, 1);
        let suite = &outcome.suites[0];
        assert!(suite.regressed);
        assert!(!suite.passed);
        assert_eq!(suite.new_findings, 1);
        assert_eq!(suite.baseline_exit_code, Some(0));
        assert_eq!(suite.current_exit_code, 1);
        // EVAL-005 linkage: a regressed suite carries remediation guidance.
        assert!(!suite.guidance.is_empty());
    }

    #[test]
    fn eval_regression_command_first_run_with_failure_regresses() {
        // No baseline at all (first ever run) that fails is a regression.
        let cur = summary(
            "secrets",
            1,
            vec![finding(EvalSeverity::Error, "leak", Some("c"))],
        );
        let outcome = build_outcome(&[(cur, None)]);
        assert!(outcome.regressed);
        assert_eq!(outcome.suites[0].baseline_exit_code, None);
    }

    #[test]
    fn eval_regression_command_improvement_is_not_a_regression() {
        // Gate went 1 -> 0: a fix, not a regression. Delta records the resolved
        // finding.
        let base = summary(
            "arch",
            1,
            vec![finding(EvalSeverity::Error, "old", Some("a"))],
        );
        let cur = summary("arch", 0, vec![]);
        let outcome = build_outcome(&[(cur, Some(base))]);
        assert!(!outcome.regressed);
        assert_eq!(outcome.suites[0].resolved_findings, 1);
        assert!(outcome.suites[0].passed);
    }

    #[test]
    fn eval_regression_command_aggregates_multiple_suites() {
        let clean = (summary("a", 0, vec![]), Some(summary("a", 0, vec![])));
        let regressed = (
            summary("b", 1, vec![finding(EvalSeverity::Error, "x", Some("z"))]),
            Some(summary("b", 0, vec![])),
        );
        let outcome = build_outcome(&[clean, regressed]);
        assert_eq!(outcome.suites_run, 2);
        assert_eq!(outcome.regressions, 1);
        assert!(outcome.regressed);
    }

    #[test]
    fn eval_regression_command_output_serialises_canonically() {
        let cur = summary(
            "arch",
            1,
            vec![finding(EvalSeverity::Error, "x", Some("z"))],
        );
        let outcome = build_outcome(&[(cur, Some(summary("arch", 0, vec![])))]);
        let json: serde_json::Value = serde_json::to_value(&outcome).expect("ser");
        assert_eq!(json["regressed"], true);
        assert_eq!(json["regressions"], 1);
        assert_eq!(json["suites"][0]["new_findings"], 1);
        // Guidance is present on the regressed suite.
        assert!(
            json["suites"][0]["guidance"]
                .as_array()
                .is_some_and(|g| !g.is_empty())
        );
    }

    #[test]
    fn eval_regression_command_store_dir_prefers_explicit() {
        let explicit = PathBuf::from("/tmp/custom-eval");
        assert_eq!(resolve_store_dir(Some(&explicit)), explicit);
    }
}
