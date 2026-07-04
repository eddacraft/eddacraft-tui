//! PATT-003 — `anvil policy attack-regression`: the prompt-attack regression
//! gate.
//!
//! Loads an attack pack (PATT-002), runs it through the deterministic baseline
//! observer, then maps the normalised [`PackRunReport`] to a gate decision via a
//! configurable [`FailPolicy`]: a severity threshold above which a *failing*
//! scenario blocks, and below which it only warns.
//!
//! Exit-code posture follows the repo contract (ADR-002, warnings over blocks):
//! the command is **report-only by default** (exit 0, [`GateDecision::Warn`] on
//! any failure) and only blocks CI when `--fail-above <severity>` opts a
//! threshold in — mirroring `policy eval-regression`'s `--fail-on-regression`
//! and the EVALCI report-only phase. The verdict is always in the output.
//!
//! CI note: this ships the *mechanism* only. Wiring a live defence-under-test
//! observer and promoting this to a **new required, blocking** CI workflow step
//! is a later gated decision (mirroring EVALCI's report-only-then-gate posture);
//! PATT-003 deliberately does not add such a step. The existing report-only
//! surface is enough to make the gate visible without blocking a build.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use anvil_kernel_types::io_risk::RiskSeverity;
use anvil_policy::attack::{ConformanceObserver, PackRunReport, load_pack, run_pack};

use crate::GlobalArgs;
use crate::output;

/// The gate verdict for a pack run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GateDecision {
    /// Every scenario behaved safely.
    Pass,
    /// One or more scenarios failed, but none crossed the blocking threshold —
    /// surfaced, not blocked (the warnings-first default).
    Warn,
    /// A failing scenario's severity crossed the configured blocking threshold.
    Fail,
}

/// A configurable pass/fail threshold by severity.
///
/// A failing scenario blocks (contributes to [`GateDecision::Fail`]) only when
/// its severity ranks **strictly above** [`fail_above`](FailPolicy::fail_above).
/// `None` is warnings-first: failures are reported but never block.
#[derive(Debug, Clone, Copy, Default)]
pub struct FailPolicy {
    /// The highest severity that is still only a warning. A failing scenario
    /// above this band blocks. `None` blocks nothing (report-only).
    fail_above: Option<RiskSeverity>,
}

impl FailPolicy {
    /// A policy that blocks on any failing scenario strictly above `fail_above`.
    /// `None` is the warnings-first, report-only default.
    #[must_use]
    pub fn new(fail_above: Option<RiskSeverity>) -> Self {
        Self { fail_above }
    }

    /// Map a pack run's normalised outcomes to a gate decision.
    ///
    /// Fail-closed on severity: a failing scenario with no declared severity is
    /// treated as [`RiskSeverity::Medium`] (the default operational band), and a
    /// failing scenario with an *unrecognised* severity
    /// ([`RiskSeverity::Unknown`], e.g. from a newer fixture) ranks above every
    /// known band, so it can never slip under a finite threshold unnoticed.
    #[must_use]
    pub fn decide(self, report: &PackRunReport) -> GateDecision {
        let failures = report.failures();
        if failures.is_empty() {
            return GateDecision::Pass;
        }
        let Some(threshold) = self.fail_above else {
            // Warnings-first: report failures, never block.
            return GateDecision::Warn;
        };
        let threshold_rank = severity_rank(threshold);
        if failures
            .iter()
            .any(|f| severity_rank(f.severity.unwrap_or(RiskSeverity::Medium)) > threshold_rank)
        {
            GateDecision::Fail
        } else {
            GateDecision::Warn
        }
    }
}

/// Rank a severity band for threshold comparison. `Unknown` ranks highest
/// (fail-closed): an unrecognised severity on a failing scenario must not sort
/// below a known threshold.
fn severity_rank(severity: RiskSeverity) -> u8 {
    match severity {
        RiskSeverity::Low => 1,
        RiskSeverity::Medium => 2,
        RiskSeverity::High => 3,
        RiskSeverity::Critical => 4,
        RiskSeverity::Unknown => 5,
    }
}

/// Parse a `--fail-above` severity. Only the four ranked bands are accepted as a
/// threshold; `unknown` is not a meaningful threshold and is rejected.
fn parse_threshold(raw: &str) -> Result<RiskSeverity, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "low" => Ok(RiskSeverity::Low),
        "medium" => Ok(RiskSeverity::Medium),
        "high" => Ok(RiskSeverity::High),
        "critical" => Ok(RiskSeverity::Critical),
        other => Err(format!(
            "unknown severity `{other}`; expected one of: low, medium, high, critical"
        )),
    }
}

#[derive(Debug, Args)]
pub struct AttackRegressionArgs {
    /// Path to an attack pack YAML file (a PATT-002 pack of scenario fixtures).
    #[arg(long, value_name = "PATH")]
    pack: PathBuf,

    /// Block (exit non-zero) when a failing scenario's severity is strictly
    /// above this band (`low`, `medium`, `high`, `critical`). Omitted =
    /// report-only: failures warn but never block (warnings over blocks).
    #[arg(long, value_name = "SEVERITY", value_parser = parse_threshold)]
    fail_above: Option<RiskSeverity>,
}

/// The serialised (`--json`) gate output.
#[derive(Debug, Clone, Serialize)]
struct GateOutput {
    pack_id: String,
    version: String,
    total: usize,
    passed: usize,
    failed: usize,
    decision: GateDecision,
    report: PackRunReport,
}

/// Build the gate output from a report and a policy. Pure — no IO — so the gate
/// semantics are directly testable.
fn build_output(report: PackRunReport, policy: FailPolicy) -> GateOutput {
    let decision = policy.decide(&report);
    let passed = report.passed_count();
    let total = report.outcomes.len();
    GateOutput {
        pack_id: report.pack_id.clone(),
        version: report.version.clone(),
        total,
        passed,
        failed: total - passed,
        decision,
        report,
    }
}

pub fn run(args: &AttackRegressionArgs, global: &GlobalArgs) -> Result<()> {
    let pack = load_pack(&args.pack)
        .with_context(|| format!("loading attack pack `{}`", args.pack.display()))?;
    // Deterministic baseline observer until a live defence-under-test is wired
    // (see the module note): validates the fixtures load and drives the gate
    // pipeline end-to-end.
    let report = run_pack(&pack, &ConformanceObserver);
    let policy = FailPolicy::new(args.fail_above);
    let output = build_output(report, policy);

    if global.json {
        output::json::print(&output)?;
    } else {
        render_plain(&output);
    }

    if output.decision == GateDecision::Fail {
        return Err(output::AlreadyReported.into());
    }
    Ok(())
}

fn render_plain(output: &GateOutput) {
    use crate::output::plain;

    plain::blank();
    plain::section("Attack regression");
    for outcome in &output.report.outcomes {
        let icon = if outcome.passed {
            "\u{2713}" // ✓
        } else {
            "\u{2717}" // ✗
        };
        println!(
            "  {icon} {id:<28} expected:{expected:?} observed:{observed:?}",
            id = outcome.scenario_id,
            expected = outcome.expected,
            observed = outcome.observed,
        );
    }
    plain::blank();
    let summary = format!(
        "{}/{} scenario(s) safe in pack `{}`",
        output.passed, output.total, output.pack_id
    );
    match output.decision {
        GateDecision::Pass => plain::success(&summary),
        GateDecision::Warn => plain::warn(&format!("{summary} (report-only)")),
        GateDecision::Fail => plain::error(&summary),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::attack_scenario::{AttackCategory, SafeBehaviour};
    use anvil_kernel_types::io_risk::Confidence;
    use anvil_policy::attack::ScenarioOutcome;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: AttackRegressionArgs,
    }

    fn outcome(passed: bool, severity: Option<RiskSeverity>) -> ScenarioOutcome {
        ScenarioOutcome {
            scenario_id: "s".into(),
            category: AttackCategory::InstructionOverride,
            expected: SafeBehaviour::Refused,
            observed: if passed {
                SafeBehaviour::Refused
            } else {
                SafeBehaviour::Warned
            },
            passed,
            confidence: Confidence::High,
            severity,
        }
    }

    fn report(outcomes: Vec<ScenarioOutcome>) -> PackRunReport {
        PackRunReport {
            pack_id: "p".into(),
            version: "1.0.0".into(),
            outcomes,
        }
    }

    #[test]
    fn attack_regression_gate_all_passed_is_pass() {
        let r = report(vec![outcome(true, Some(RiskSeverity::Critical))]);
        // Even the strictest threshold passes when nothing failed.
        assert_eq!(
            FailPolicy::new(Some(RiskSeverity::Low)).decide(&r),
            GateDecision::Pass
        );
        assert_eq!(FailPolicy::default().decide(&r), GateDecision::Pass);
    }

    #[test]
    fn attack_regression_gate_default_is_warnings_first() {
        // No threshold: a failing critical scenario warns, never blocks.
        let r = report(vec![outcome(false, Some(RiskSeverity::Critical))]);
        assert_eq!(FailPolicy::default().decide(&r), GateDecision::Warn);
    }

    #[test]
    fn attack_regression_gate_blocks_above_threshold() {
        // fail-above high: a failing critical scenario is strictly above => Fail.
        let r = report(vec![outcome(false, Some(RiskSeverity::Critical))]);
        assert_eq!(
            FailPolicy::new(Some(RiskSeverity::High)).decide(&r),
            GateDecision::Fail
        );
    }

    #[test]
    fn attack_regression_gate_warns_at_or_below_threshold() {
        // fail-above high: a failing high scenario is NOT strictly above => Warn.
        let r = report(vec![outcome(false, Some(RiskSeverity::High))]);
        assert_eq!(
            FailPolicy::new(Some(RiskSeverity::High)).decide(&r),
            GateDecision::Warn
        );
        // A failing low scenario likewise only warns.
        let low = report(vec![outcome(false, Some(RiskSeverity::Low))]);
        assert_eq!(
            FailPolicy::new(Some(RiskSeverity::High)).decide(&low),
            GateDecision::Warn
        );
    }

    #[test]
    fn attack_regression_gate_missing_severity_defaults_to_medium() {
        let r = report(vec![outcome(false, None)]);
        // Default medium: above `low` threshold => Fail.
        assert_eq!(
            FailPolicy::new(Some(RiskSeverity::Low)).decide(&r),
            GateDecision::Fail
        );
        // Not above `medium` threshold => Warn.
        assert_eq!(
            FailPolicy::new(Some(RiskSeverity::Medium)).decide(&r),
            GateDecision::Warn
        );
    }

    #[test]
    fn attack_regression_gate_unknown_severity_is_fail_closed() {
        // A failing scenario with an unrecognised severity must block even at the
        // strictest known threshold — it can never slip under the gate.
        let r = report(vec![outcome(false, Some(RiskSeverity::Unknown))]);
        assert_eq!(
            FailPolicy::new(Some(RiskSeverity::Critical)).decide(&r),
            GateDecision::Fail
        );
        // But warnings-first still overrides when no threshold is set.
        assert_eq!(FailPolicy::default().decide(&r), GateDecision::Warn);
    }

    #[test]
    fn attack_regression_gate_mixed_outcomes_take_the_worst() {
        // One passing, one low failing, one critical failing.
        let r = report(vec![
            outcome(true, Some(RiskSeverity::Critical)),
            outcome(false, Some(RiskSeverity::Low)),
            outcome(false, Some(RiskSeverity::Critical)),
        ]);
        assert_eq!(
            FailPolicy::new(Some(RiskSeverity::High)).decide(&r),
            GateDecision::Fail
        );
        let out = build_output(r, FailPolicy::new(Some(RiskSeverity::High)));
        assert_eq!(out.total, 3);
        assert_eq!(out.passed, 1);
        assert_eq!(out.failed, 2);
    }

    #[test]
    fn attack_regression_gate_output_serialises_canonically() {
        let r = report(vec![outcome(false, Some(RiskSeverity::Critical))]);
        let out = build_output(r, FailPolicy::new(Some(RiskSeverity::High)));
        let json: serde_json::Value = serde_json::to_value(&out).expect("ser");
        assert_eq!(json["decision"], "fail");
        assert_eq!(json["failed"], 1);
        assert_eq!(json["report"]["outcomes"][0]["passed"], false);
    }

    #[test]
    fn attack_regression_gate_parses_args() {
        let w = Wrapper::try_parse_from(["test", "--pack", "pack.yaml", "--fail-above", "high"])
            .expect("parse");
        assert_eq!(w.inner.pack, PathBuf::from("pack.yaml"));
        assert_eq!(w.inner.fail_above, Some(RiskSeverity::High));
    }

    #[test]
    fn attack_regression_gate_requires_pack_flag() {
        assert!(Wrapper::try_parse_from(["test"]).is_err());
    }

    #[test]
    fn attack_regression_gate_rejects_unknown_severity_threshold() {
        assert!(
            Wrapper::try_parse_from(["test", "--pack", "p.yaml", "--fail-above", "apocalyptic"])
                .is_err()
        );
        // `unknown` is not a valid threshold either.
        assert!(
            Wrapper::try_parse_from(["test", "--pack", "p.yaml", "--fail-above", "unknown"])
                .is_err()
        );
    }

    #[test]
    fn attack_regression_gate_default_report_only_never_fails() {
        // The whole point of the default: no threshold => the command can only
        // return Pass or Warn, never a blocking Fail.
        let failing = report(vec![outcome(false, Some(RiskSeverity::Critical))]);
        let out = build_output(failing, FailPolicy::default());
        assert_ne!(out.decision, GateDecision::Fail);
    }
}
