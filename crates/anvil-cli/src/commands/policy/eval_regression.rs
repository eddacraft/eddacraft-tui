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

use std::io::Write as _;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use anvil_policy::adversarial::is_reserved_suite_name;
use anvil_policy::eval::{
    EvalHarnessPort, EvalRegressionReport, EvalResultStore, EvalRunSummary, EvalSeverity,
    EvalSuite, GuidedFinding, PolicyEvalAdapter, SubprocessRunner, guidance_for,
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
    /// EVALCI-010: whether this suite's output differs from its baseline in
    /// *either* direction. Distinct from `regressed`, which reads `exit_code`
    /// and treats a finding disappearing as an improvement — true for a gate
    /// over changing code, false for a suite over a committed frozen input.
    output_changed: bool,
    /// Remediation guidance, present only for a failing current run.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    guidance: Vec<GuidedFinding>,
}

/// A suite that could not be executed (EVALCI-004). Surfaced in the report so a
/// broken suite is visible, but kept separate from `suites` so a runner error is
/// never conflated with a trust-regression verdict.
#[derive(Debug, Clone, Serialize)]
struct RunnerError {
    suite: String,
    error: String,
}

/// The aggregate regression report for the whole run.
#[derive(Debug, Clone, Serialize)]
struct RegressionOutput {
    suites: Vec<SuiteOutcome>,
    /// Suites that failed to run at all (EVALCI-004 fail-open). Omitted from the
    /// JSON when empty so the common case is unchanged.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    runner_errors: Vec<RunnerError>,
    /// Count of suites that actually produced a verdict (excludes runner errors).
    suites_run: usize,
    regressions: usize,
    /// True when any suite regressed — the headline pass/fail.
    regressed: bool,
    /// EVALCI-010: suites whose fixture output changed in either direction.
    output_changes: usize,
    /// True when any suite's fixture output changed. This is what detects a
    /// policy going silent; `regressed` structurally cannot.
    output_changed: bool,
}

/// A suite that produced a verdict, paired with its optional baseline. Boxed
/// inside [`SuiteRun::Ran`] so the enum's variants stay a similar size.
struct RanSuite {
    current: EvalRunSummary,
    baseline: Option<EvalRunSummary>,
}

/// One suite's execution result. EVALCI-004 fail-open: a suite that errors is
/// recorded as [`SuiteRun::Errored`] and the run continues, rather than a single
/// broken suite aborting the whole command and suppressing regression detection
/// for every other suite.
enum SuiteRun {
    Ran(Box<RanSuite>),
    Errored(RunnerError),
}

/// Run every suite, capturing per-suite runner errors instead of aborting on the
/// first one (EVALCI-004). Each suite yields a [`SuiteRun`]; the returned order
/// mirrors `suites`.
fn collect_runs(
    adapter: &impl EvalHarnessPort,
    suites: &[EvalSuite],
    baselines: &std::collections::HashMap<String, EvalRunSummary>,
) -> Vec<SuiteRun> {
    suites
        .iter()
        .map(|suite| match adapter.run_suite(suite) {
            Ok(current) => SuiteRun::Ran(Box::new(RanSuite {
                baseline: baselines.get(&suite.name).cloned(),
                current,
            })),
            Err(error) => SuiteRun::Errored(RunnerError {
                suite: suite.name.clone(),
                error: error.to_string(),
            }),
        })
        .collect()
}

/// Build the aggregate outcome from each suite's execution result. Pure — no IO —
/// so the regression semantics are directly testable.
fn build_outcome(runs: &[SuiteRun]) -> RegressionOutput {
    let mut suites = Vec::new();
    let mut runner_errors = Vec::new();
    for run in runs {
        match run {
            SuiteRun::Ran(ran) => {
                let report = EvalRegressionReport::compare(ran.baseline.as_ref(), &ran.current);
                suites.push(SuiteOutcome {
                    suite: ran.current.suite.clone(),
                    passed: ran.current.passed(),
                    regressed: report.regressed(),
                    current_exit_code: report.current_exit_code,
                    baseline_exit_code: report.baseline_exit_code,
                    new_findings: report.new_findings.len(),
                    resolved_findings: report.resolved_findings.len(),
                    output_changed: report.output_changed(),
                    guidance: guidance_for(&ran.current),
                });
            }
            SuiteRun::Errored(error) => runner_errors.push(error.clone()),
        }
    }

    let regressions = suites.iter().filter(|s| s.regressed).count();
    let output_changes = suites.iter().filter(|s| s.output_changed).count();
    RegressionOutput {
        suites_run: suites.len(),
        regressions,
        regressed: regressions > 0,
        output_changes,
        output_changed: output_changes > 0,
        suites,
        runner_errors,
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

    // EVALCI-004: a suite that fails to run is recorded and the run continues,
    // so one broken suite cannot suppress regression detection for the rest.
    let runs = collect_runs(&adapter, &suites, &baselines);

    let outcome = build_outcome(&runs);

    // EVALCI-010: compute the outcome *before* persisting, and never record a
    // run this invocation is about to reject.
    //
    // `persist_runs` applies the per-suite EVALCI-001 ratchet, which reads
    // `regressed()`. That is blind to the fixture verdict: a rule going silent
    // is `regressed() == false` (1->0 reads as an improvement) but
    // `output_changed() == true`. With `--update-baseline --fail-on-regression`
    // the old ordering persisted the silenced run first and only then failed,
    // so the next run compared against an already-silenced baseline and came
    // back clean — baseline poisoning through the door Part 2 opened.
    //
    // A bare `--update-baseline` (no blocking requested) still persists a
    // changed fixture: that is the documented operator refresh, where accepting
    // the new output is the whole point.
    if args.update_baseline && !should_block(&outcome, args.fail_on_regression) {
        persist_runs(&store, &runs)?;
    }

    // GH #3169: never enter a TUI path for this report-only CI command, and
    // flush stdout without taking exclusive ownership of fd 1 (so live pipes
    // such as `… | tee` keep reading until the process exits).
    if global.json {
        output::json::print(&outcome)?;
    } else {
        render_plain(&outcome);
    }
    let _ = std::io::stdout().flush();

    if should_block(&outcome, args.fail_on_regression) {
        return Err(output::AlreadyReported.into());
    }
    Ok(())
}

/// Whether the command should exit non-zero. Report-only by default (ADR-002);
/// under `--fail-on-regression` the gate blocks on a trust regression **or** a
/// runner error. A suite that could not be evaluated is not a passing gate:
/// without this, an all-suites-errored run (missing binary, broken suites file)
/// would exit 0 under `--fail-on-regression` — a green gate that checked
/// nothing, exactly the false-negative EVALCI-004's fail-open must not open.
fn should_block(outcome: &RegressionOutput, fail_on_regression: bool) -> bool {
    fail_on_regression
        && (outcome.regressed || outcome.output_changed || !outcome.runner_errors.is_empty())
}

/// Append eligible runs to the history. The `run_id`/timestamp come from the
/// wall clock here; the store itself stays time-agnostic.
///
/// EVALCI-001 ratchet: a run is persisted only when its gate did **not** regress
/// against its baseline, so a failing/regressed run can never become the
/// accepted baseline that future runs compare against (baseline poisoning). A
/// suite that errored (EVALCI-004) has no verdict and is skipped.
fn persist_runs(store: &EvalResultStore, runs: &[SuiteRun]) -> Result<()> {
    let now = chrono::Utc::now();
    let recorded_at = now.to_rfc3339();
    for run in runs {
        let SuiteRun::Ran(ran) = run else {
            continue;
        };
        // EVALCI-001 ratchet, narrowed by EVALCI-010.
        //
        // Once the runner passes `--fail-on-warnings`, an advisory suite
        // reports `exit_code: 1`, and on a first run `regressed()` is true
        // (`baseline_exit_code.is_none_or(..)` short-circuits for `None`).
        // Under the original blanket skip such a suite could never bootstrap a
        // baseline — it would be skipped forever and silently check nothing.
        //
        // So a first run may establish history, but only when it is
        // *advisory-only*. A first run carrying an `error`-severity finding is
        // genuinely broken and still must not become the accepted baseline —
        // that is EVALCI-001's original case and it stays intact. Once a
        // baseline exists the ratchet is unconditional.
        let report = EvalRegressionReport::compare(ran.baseline.as_ref(), &ran.current);
        let advisory_only = !ran
            .current
            .findings
            .iter()
            .any(|f| f.severity == EvalSeverity::Error);
        let bootstrapping_advisory_suite = ran.baseline.is_none() && advisory_only;

        // EVALCI-010 migration: a store written before `--fail-on-warnings` was
        // wired records `exit_code: 0` for advisory suites. Every unchanged
        // suite therefore compares `0 -> 1` and reads as regressed, and because
        // a baseline exists the ratchet refused it — so the documented
        // `--update-baseline` refresh could not migrate an existing store at
        // all. The committed history here was regenerated by hand; a user's
        // `ANVIL_HOME/eval` or `.anvil/eval` store could not be.
        //
        // Allow exactly that case: the exit code moved while the findings did
        // not. `output_changed()` false means no finding appeared or vanished,
        // so policy behaviour is identical and only the representation changed.
        // A genuine escalation always brings a new finding with it, which makes
        // `output_changed()` true and leaves the ratchet in force.
        let exit_code_only_migration = report.regressed() && !report.output_changed();

        if report.regressed() && !bootstrapping_advisory_suite && !exit_code_only_migration {
            continue;
        }
        let run_id = format!("{}-{}", ran.current.suite, now.timestamp_millis());
        let record =
            anvil_policy::eval::EvalRecord::from_summary(&ran.current, run_id, &recorded_at);
        store
            .append(&record)
            .with_context(|| format!("persisting eval run for suite `{}`", ran.current.suite))?;
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
    // The `probe:` prefix is reserved for adversarial probe categories
    // (ATC-003) so their synthetic suites don't collide with policy suites in
    // the shared eval store — enforce it here, at the boundary where a policy
    // suite name is authored.
    if let Some(suite) = suites.iter().find(|s| is_reserved_suite_name(&s.name)) {
        anyhow::bail!(
            "suite `{}` in `{}` uses the reserved `probe:` prefix, which is only valid for adversarial probe categories",
            suite.name,
            path.display()
        );
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
        } else if suite.output_changed {
            // EVALCI-010: the fixture's output moved even though the gate
            // verdict did not. A rule going silent lands here, so it must not
            // render as a clean ✓.
            "\u{0394}" // Δ
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
        // Show remediation guidance for any failing suite — a persistently
        // failing (○) suite is as actionable as a newly regressed (✗) one. The
        // `guidance` list is non-empty exactly when the current run failed
        // (matches the JSON output).
        if suite.output_changed && !suite.regressed {
            plain::warn(&format!(
                "    fixture output changed on a frozen input ({} new, {} gone) — the policy behaved differently, review before accepting",
                suite.new_findings, suite.resolved_findings
            ));
        }
        for guided in &suite.guidance {
            plain::warn(&format!("    {}", guided.guidance.summary));
            for action in &guided.guidance.next_actions {
                plain::info(&format!("      - {action}"));
            }
        }
    }
    // EVALCI-004: surface suites that failed to run — visible but non-fatal, and
    // deliberately not counted as regressions.
    for runner_error in &outcome.runner_errors {
        // \u{26a0} is ⚠ — a runner error, surfaced but non-fatal.
        println!(
            "  \u{26a0} {name:<24} runner error",
            name = runner_error.suite
        );
        plain::warn(&format!("    {}", runner_error.error));
    }
    plain::blank();
    if outcome.regressed {
        plain::error(&format!(
            "{} of {} suite(s) regressed",
            outcome.regressions, outcome.suites_run
        ));
    } else if outcome.output_changed {
        // EVALCI-010: never render a green headline when a fixture's output
        // moved. The gate verdict is unmoved by design (a resolved finding
        // reads as an improvement), so without this the summary would say
        // "no regressions" for a rule that stopped firing — the exact
        // false-green CPACKS-006 was opened for.
        plain::error(&format!(
            "{} of {} suite(s) changed output on a frozen input",
            outcome.output_changes, outcome.suites_run
        ));
    } else {
        plain::success(&format!(
            "no regressions across {} suite(s)",
            outcome.suites_run
        ));
    }
    if !outcome.runner_errors.is_empty() {
        plain::warn(&format!(
            "{} suite(s) failed to run",
            outcome.runner_errors.len()
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

    /// A successfully-executed suite run for `build_outcome`/`persist_runs`.
    fn ran(current: EvalRunSummary, baseline: Option<EvalRunSummary>) -> SuiteRun {
        SuiteRun::Ran(Box::new(RanSuite { current, baseline }))
    }

    fn suite_def(name: &str) -> EvalSuite {
        EvalSuite {
            name: name.into(),
            policy: "p.rego".into(),
            input: None,
            query: "data.anvil.findings".into(),
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
        let outcome = build_outcome(&[ran(cur, Some(base))]);
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
        let outcome = build_outcome(&[ran(cur, Some(base))]);

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
        let outcome = build_outcome(&[ran(cur, None)]);
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
        let outcome = build_outcome(&[ran(cur, Some(base))]);
        assert!(!outcome.regressed);
        assert_eq!(outcome.suites[0].resolved_findings, 1);
        assert!(outcome.suites[0].passed);
    }

    #[test]
    fn eval_regression_command_aggregates_multiple_suites() {
        let clean = ran(summary("a", 0, vec![]), Some(summary("a", 0, vec![])));
        let regressed = ran(
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
        let outcome = build_outcome(&[ran(cur, Some(summary("arch", 0, vec![])))]);
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

    #[test]
    fn eval_regression_ratchet_baseline() {
        // EVALCI-001: `--update-baseline` must not persist a regressed run, so a
        // failing run can never become the accepted baseline (baseline poisoning).
        let dir = tempfile::TempDir::new().expect("tmp");
        let store = EvalResultStore::new(dir.path().join("eval"));

        // A first-ever failing run regresses — it must not be persisted.
        let failing = ran(
            summary(
                "arch",
                1,
                vec![finding(EvalSeverity::Error, "boom", Some("a"))],
            ),
            None,
        );
        persist_runs(&store, std::slice::from_ref(&failing)).expect("persist failing");
        assert!(
            store.all().expect("all").is_empty(),
            "a regressed run must not poison the baseline"
        );

        // A clean run is persisted and becomes the baseline.
        let clean = ran(summary("arch", 0, vec![]), None);
        persist_runs(&store, std::slice::from_ref(&clean)).expect("persist clean");
        let all = store.all().expect("all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].suite, "arch");

        // An errored suite has no verdict and is skipped entirely.
        let errored = SuiteRun::Errored(RunnerError {
            suite: "arch".into(),
            error: "missing policy".into(),
        });
        persist_runs(&store, std::slice::from_ref(&errored)).expect("persist errored");
        assert_eq!(
            store.all().expect("all").len(),
            1,
            "errored run not persisted"
        );
    }

    #[test]
    fn eval_regression_command_fail_open() {
        use std::collections::HashMap;

        // A port that fails only the `broken` suite, proving one broken suite
        // does not abort the run or suppress the others (EVALCI-004).
        struct PartlyBroken;
        impl EvalHarnessPort for PartlyBroken {
            fn run_suite(
                &self,
                suite: &EvalSuite,
            ) -> Result<EvalRunSummary, anvil_policy::eval::EvalHarnessError> {
                if suite.name == "broken" {
                    Err(anvil_policy::eval::EvalHarnessError::Execution {
                        suite: suite.name.clone(),
                        source: "missing policy".into(),
                    })
                } else {
                    Ok(summary(&suite.name, 0, vec![]))
                }
            }
        }

        let suites = vec![suite_def("broken"), suite_def("healthy")];
        let runs = collect_runs(&PartlyBroken, &suites, &HashMap::new());
        let outcome = build_outcome(&runs);

        // The healthy suite still reported a verdict.
        assert_eq!(outcome.suites_run, 1);
        assert_eq!(outcome.suites[0].suite, "healthy");
        assert!(outcome.suites[0].passed);
        // The broken suite is surfaced as a runner error, not a regression.
        assert_eq!(outcome.runner_errors.len(), 1);
        assert_eq!(outcome.runner_errors[0].suite, "broken");
        assert!(!outcome.regressed);
        assert_eq!(outcome.regressions, 0);

        // Report-only (no `--fail-on-regression`): a runner error is surfaced but
        // does not block. Under `--fail-on-regression` it must block — a suite
        // that could not run is not a passing gate.
        assert!(!should_block(&outcome, false));
        assert!(should_block(&outcome, true));
    }

    #[test]
    fn eval_regression_fail_on_regression_blocks_on_runner_error_only() {
        // An all-suites-errored run has no regression yet must not report a green
        // gate under `--fail-on-regression` (the false-negative guard).
        let errored = SuiteRun::Errored(RunnerError {
            suite: "arch".into(),
            error: "missing policy".into(),
        });
        let outcome = build_outcome(std::slice::from_ref(&errored));
        assert_eq!(outcome.suites_run, 0);
        assert!(!outcome.regressed);
        assert!(should_block(&outcome, true), "errored gate must block");
        assert!(!should_block(&outcome, false), "report-only never blocks");

        // A wholly clean run never blocks, either posture.
        let clean = build_outcome(&[ran(summary("arch", 0, vec![]), None)]);
        assert!(!should_block(&clean, true));
        assert!(!should_block(&clean, false));
    }

    #[test]
    fn eval_regression_catches_a_violation_absorbed_by_an_already_failing_gate() {
        // EVALCI-008's second design question: "an already-failing gate absorbs
        // new violations". With the exit code 1 -> 1, `regressed()` is false
        // (`baseline == current`), so a brand-new violation lands invisibly.
        // EVALCI-010's fixture verdict resolves this as a side effect: the new
        // finding makes `output_changed()` true.
        let base = summary(
            "arch",
            1,
            vec![finding(EvalSeverity::Error, "existing", Some("a"))],
        );
        let cur = summary(
            "arch",
            1,
            vec![
                finding(EvalSeverity::Error, "existing", Some("a")),
                finding(EvalSeverity::Error, "brand new", Some("b")),
            ],
        );
        let outcome = build_outcome(&[ran(cur, Some(base))]);

        assert!(
            !outcome.regressed,
            "1 -> 1 is exactly the absorbed-violation blind spot"
        );
        assert!(
            outcome.output_changed,
            "the new violation must still be caught"
        );
        assert_eq!(outcome.suites[0].new_findings, 1);
        assert!(should_block(&outcome, true));
    }

    #[test]
    fn eval_regression_blocking_mode_does_not_persist_a_silenced_fixture() {
        // EVALCI-010 baseline-poisoning guard. A rule going silent is
        // `regressed() == false` (1->0 reads as an improvement) but
        // `output_changed() == true`. Persisting it under
        // `--update-baseline --fail-on-regression` would make the silence the
        // accepted baseline, and the next run would come back clean.
        let base = summary(
            "s",
            1,
            vec![finding(EvalSeverity::Warning, "fires", Some("a"))],
        );
        let silenced = summary("s", 0, vec![]);
        let outcome = build_outcome(&[ran(silenced.clone(), Some(base.clone()))]);

        assert!(
            !outcome.regressed,
            "gate semantics call this an improvement"
        );
        assert!(outcome.output_changed, "the fixture verdict catches it");
        assert!(
            should_block(&outcome, true),
            "blocking mode must reject the run"
        );

        // The command only persists when the invocation is not rejecting the
        // run, so a rejected run never reaches the store.
        let dir = tempfile::TempDir::new().expect("tmp");
        let store = EvalResultStore::new(dir.path());
        if !should_block(&outcome, true) {
            persist_runs(&store, &[ran(silenced.clone(), Some(base.clone()))]).expect("persist");
        }
        assert!(
            store.all().expect("all").is_empty(),
            "a silenced fixture must not become the baseline"
        );

        // Report-only is the documented operator refresh: accepting the new
        // output is the point, so it still persists.
        assert!(!should_block(&outcome, false));
        persist_runs(&store, &[ran(silenced, Some(base))]).expect("persist");
        assert_eq!(store.all().expect("all").len(), 1);
    }

    #[test]
    fn eval_regression_migrates_an_exit_code_only_change() {
        // EVALCI-010: a store written before `--fail-on-warnings` records
        // exit_code 0 for advisory suites. After the flag, an unchanged suite
        // compares 0 -> 1 and reads as regressed, and the ratchet would refuse
        // it because a baseline exists — leaving such a store unmigratable.
        // Findings identical means policy behaviour is identical.
        let f = finding(EvalSeverity::Warning, "steady", Some("a"));
        let old_baseline = summary("s", 0, vec![f.clone()]);
        let after_flag = summary("s", 1, vec![f]);
        let report = EvalRegressionReport::compare(Some(&old_baseline), &after_flag);
        assert!(report.regressed(), "0 -> 1 reads as regressed");
        assert!(
            !report.output_changed(),
            "but no finding appeared or vanished"
        );

        let dir = tempfile::TempDir::new().expect("tmp");
        let store = EvalResultStore::new(dir.path());
        persist_runs(&store, &[ran(after_flag, Some(old_baseline))]).expect("persist");
        let stored = store.latest("s").expect("read").expect("migrated record");
        assert_eq!(stored.exit_code, 1, "the migration must be recorded");

        // A genuine escalation still brings a new finding, so it stays refused.
        let base2 = summary("t", 0, vec![]);
        let escalated = summary(
            "t",
            1,
            vec![finding(EvalSeverity::Error, "boom", Some("b"))],
        );
        persist_runs(&store, &[ran(escalated, Some(base2))]).expect("persist");
        assert!(
            store.latest("t").expect("read").is_none(),
            "a real escalation must still be refused"
        );
    }

    #[test]
    fn eval_regression_first_run_bootstraps_a_warning_suite_baseline() {
        // EVALCI-010 guard: with `--fail-on-warnings` wired, a warning-emitting
        // suite exits 1, so `regressed()` is true on a first run. The ratchet
        // must still let that first run establish a baseline, or the suite can
        // never gain one and silently checks nothing forever.
        let dir = tempfile::TempDir::new().expect("tmp");
        let store = EvalResultStore::new(dir.path());
        let current = summary(
            "warns",
            1,
            vec![finding(EvalSeverity::Warning, "fires", Some("a"))],
        );
        let report = EvalRegressionReport::compare(None, &current);
        assert!(
            report.regressed(),
            "a first run with a non-zero exit still reports as regressed"
        );

        persist_runs(&store, &[ran(current.clone(), None)]).expect("persist");
        let stored = store
            .latest("warns")
            .expect("read")
            .expect("the first run must establish the baseline despite regressed()");
        assert_eq!(stored.exit_code, 1);

        // The ratchet still holds once a baseline exists: a worse run against
        // that baseline must not replace it.
        let worse = summary("warns", 2, vec![]);
        persist_runs(&store, &[ran(worse, Some(current))]).expect("persist");
        let after = store.latest("warns").expect("read").expect("record");
        assert_eq!(after.exit_code, 1, "a regressed run must not poison it");
    }

    #[test]
    fn eval_regression_blocks_when_a_rule_goes_silent() {
        // EVALCI-010 / the CPACKS-006 defect, end to end at the CLI seam.
        // A warning-tier finding disappears against a committed baseline on a
        // frozen input. `regressed()` calls that an improvement (gate 1->0), so
        // before this fix the run reported a clean gate and could not block.
        let base = summary(
            "anvil_baseline_sensitive_paths",
            0,
            vec![finding(EvalSeverity::Warning, "fires", Some("a"))],
        );
        let cur = summary("anvil_baseline_sensitive_paths", 0, vec![]);
        let outcome = build_outcome(&[ran(cur, Some(base))]);

        assert!(
            !outcome.regressed,
            "gate semantics still read this as an improvement — that is why \
             output_changed exists rather than replacing regressed()"
        );
        assert!(outcome.output_changed, "the silent rule must be detected");
        assert_eq!(outcome.output_changes, 1);
        assert_eq!(outcome.suites[0].resolved_findings, 1);
        assert!(
            should_block(&outcome, true),
            "under --fail-on-regression a rule going silent must block"
        );
        assert!(
            !should_block(&outcome, false),
            "report-only posture never blocks (ADR-002)"
        );
    }

    #[test]
    fn eval_regression_stable_fixture_is_not_an_output_change() {
        // The false-positive guard: an unchanged fixture must stay green, or
        // the new verdict would fire on every run and be ignored.
        let f = finding(EvalSeverity::Warning, "steady", Some("a"));
        let base = summary("s", 1, vec![f.clone()]);
        let cur = summary("s", 1, vec![f]);
        let outcome = build_outcome(&[ran(cur, Some(base))]);
        assert!(!outcome.output_changed);
        assert!(!outcome.regressed);
        assert!(!should_block(&outcome, true));
    }

    #[test]
    fn load_suites_rejects_reserved_probe_prefix() {
        let dir = tempfile::TempDir::new().expect("tmp");
        let path = dir.path().join("suites.json");
        std::fs::write(
            &path,
            serde_json::to_string(&[suite_def("probe:prompt-injection")]).expect("serialise"),
        )
        .expect("write");

        let err = load_suites(&path).expect_err("reserved prefix must be rejected");
        assert!(
            err.to_string().contains("reserved `probe:` prefix"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_suites_accepts_non_reserved_names() {
        let dir = tempfile::TempDir::new().expect("tmp");
        let path = dir.path().join("suites.json");
        std::fs::write(
            &path,
            serde_json::to_string(&[suite_def("arch_boundary")]).expect("serialise"),
        )
        .expect("write");

        let suites = load_suites(&path).expect("suite loads");
        assert_eq!(suites.len(), 1);
        assert_eq!(suites[0].name, "arch_boundary");
    }
}
