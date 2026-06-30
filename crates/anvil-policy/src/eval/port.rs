//! EVAL-001 — `EvalHarnessPort`: the stable adapter contract.
//!
//! The eval-harness integration drives trust-regression suites through an
//! Anvil-owned port so the core domain never couples to a concrete eval
//! framework. Everything in this module binds to the **frozen**
//! `anvil policy eval --json` v1 wire contract
//! ([`docs/specs/policy-eval-output-v1.md`]) — `schema_version`, `findings`,
//! `exit_code` — and to nothing in `anvil-policy-engine`. A later engine
//! refactor that changes its internal `Finding`/`Severity` types cannot break
//! this surface, because these types mirror the wire shape, not the engine.
//!
//! Only `serde`, `std`, and `thiserror` are imported here: the contract is
//! framework-free by construction (EVAL-001 expected outcome).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Severity of a normalised finding, mirroring the frozen `Severity` wire form
/// (`"warning"` / `"error"`). Defaults to [`EvalSeverity::Warning`] per ADR-002,
/// matching the engine default so an omitted field is non-blocking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EvalSeverity {
    /// Advisory. Never blocks (exit 0) unless the suite opts into
    /// fail-on-warnings.
    #[default]
    Warning,
    /// Blocking finding. Contributes a non-zero exit code unless baselined.
    Error,
}

impl std::fmt::Display for EvalSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

/// A single normalised finding from a suite run. The fields are the frozen
/// subset of the v1 `Finding` contract that a trust-regression gate can rely
/// on; diagnostic fields (`coverage`, `trace`, `value`) are deliberately not
/// represented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalFinding {
    #[serde(default)]
    pub severity: EvalSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

impl EvalFinding {
    /// Stable identity used to diff two runs. Prefers the baseline fingerprint
    /// (the engine's own identity for a finding); falls back to the
    /// severity + edge + message tuple when a finding carries no fingerprint.
    pub fn identity(&self) -> String {
        if let Some(fp) = &self.fingerprint {
            return format!("fp:{fp}");
        }
        format!(
            "msg:{}|{}|{}|{}",
            self.severity,
            self.from.as_deref().unwrap_or(""),
            self.to.as_deref().unwrap_or(""),
            self.message,
        )
    }
}

/// A trust-regression suite definition: a policy + optional input document +
/// query, named for reporting. This is the unit the port executes; it is *not*
/// a net-new eval framework (out of scope) — it names an existing
/// `anvil policy eval` invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalSuite {
    /// Stable identifier used in reports and persisted history.
    pub name: String,
    /// The `.rego` policy file evaluated for this suite.
    pub policy: PathBuf,
    /// Optional `PolicyInput` JSON document; defaults to an empty input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<PathBuf>,
    /// The Rego query to evaluate — point at a findings rule for gate
    /// semantics.
    pub query: String,
}

/// The normalised outcome of running one suite, projected from the frozen v1
/// output contract. This is the [`EvalHarnessPort`]'s return shape and the unit
/// of persistence (EVAL-004) and guidance (EVAL-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalRunSummary {
    /// The suite this summary came from.
    pub suite: String,
    /// The contract version the raw document declared (`"1.0.0"` for v1).
    pub schema_version: String,
    /// Display path of the evaluated policy.
    pub policy: String,
    /// The query that ran.
    pub query: String,
    /// Findings in engine order.
    pub findings: Vec<EvalFinding>,
    /// Process exit code: `0` pass, non-zero block (ADR-002).
    pub exit_code: i32,
}

impl EvalRunSummary {
    /// Whether the run passed its gate (mirrors the process exit code).
    pub fn passed(&self) -> bool {
        self.exit_code == 0
    }

    /// Count of blocking (`error`) findings.
    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == EvalSeverity::Error)
            .count()
    }

    /// Count of advisory (`warning`) findings.
    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == EvalSeverity::Warning)
            .count()
    }
}

/// The delta between a baseline run and the current run of the same suite — the
/// trust-regression verdict (EVAL-003 consumes this to emit pass/fail + delta).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalRegressionReport {
    pub suite: String,
    /// Exit code of the baseline run, or `None` on the first ever run.
    pub baseline_exit_code: Option<i32>,
    pub current_exit_code: i32,
    /// Findings present now but not in the baseline.
    pub new_findings: Vec<EvalFinding>,
    /// Findings present in the baseline but gone now.
    pub resolved_findings: Vec<EvalFinding>,
}

impl EvalRegressionReport {
    /// Compute the regression report for `current` against an optional
    /// `baseline`. With no baseline (first run), everything in `current` is new.
    pub fn compare(baseline: Option<&EvalRunSummary>, current: &EvalRunSummary) -> Self {
        let baseline_ids: Vec<String> = baseline
            .map(|b| b.findings.iter().map(EvalFinding::identity).collect())
            .unwrap_or_default();
        let current_ids: Vec<String> = current.findings.iter().map(EvalFinding::identity).collect();

        let new_findings = current
            .findings
            .iter()
            .zip(&current_ids)
            .filter(|(_, id)| !baseline_ids.contains(id))
            .map(|(f, _)| f.clone())
            .collect();

        let resolved_findings = baseline
            .map(|b| {
                b.findings
                    .iter()
                    .filter(|f| !current_ids.contains(&f.identity()))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        Self {
            suite: current.suite.clone(),
            baseline_exit_code: baseline.map(|b| b.exit_code),
            current_exit_code: current.exit_code,
            new_findings,
            resolved_findings,
        }
    }

    /// A regression is a *worsening*: the gate now blocks where it did not
    /// before, or a new blocking finding appeared. A first run with a failing
    /// gate is a regression against the implicit clean baseline.
    pub fn regressed(&self) -> bool {
        let gate_worsened =
            self.current_exit_code != 0 && self.baseline_exit_code.is_none_or(|b| b == 0);
        let new_blocking = self
            .new_findings
            .iter()
            .any(|f| f.severity == EvalSeverity::Error);
        gate_worsened || new_blocking
    }
}

/// Errors a harness port can surface. Variants are framework-neutral so the
/// concrete adapter (EVAL-002) maps its own failures onto them.
#[derive(Debug, Error)]
pub enum EvalHarnessError {
    /// The suite could not be executed (binary missing, IO failure, …).
    #[error("failed to execute suite `{suite}`: {source}")]
    Execution {
        suite: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The raw output did not match the frozen v1 contract.
    #[error(
        "suite `{suite}` produced output that does not match the eval-output v1 contract: {detail}"
    )]
    Contract { suite: String, detail: String },
    /// The output declared a `schema_version` this build cannot consume.
    #[error(
        "suite `{suite}` declared unsupported schema_version `{version}` (this build speaks v1)"
    )]
    UnsupportedSchema { suite: String, version: String },
}

/// The stable adapter contract for executing a harness suite and retrieving its
/// normalised result. The core domain depends on this trait only — never on a
/// concrete framework (EVAL-001 expected outcome).
pub trait EvalHarnessPort {
    /// Run one suite and return its normalised summary.
    fn run_suite(&self, suite: &EvalSuite) -> Result<EvalRunSummary, EvalHarnessError>;

    /// Run a set of suites in order, collecting every summary. The default
    /// implementation short-circuits on the first execution error.
    fn run_suites(&self, suites: &[EvalSuite]) -> Result<Vec<EvalRunSummary>, EvalHarnessError> {
        suites.iter().map(|s| self.run_suite(s)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A fake port proves the contract is implementable with no framework.
    struct FakePort(EvalRunSummary);
    impl EvalHarnessPort for FakePort {
        fn run_suite(&self, _suite: &EvalSuite) -> Result<EvalRunSummary, EvalHarnessError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn eval_harness_port_is_implementable_and_object_safe() {
        let port = FakePort(summary("s", 0, vec![]));
        let suite = EvalSuite {
            name: "s".into(),
            policy: "p.rego".into(),
            input: None,
            query: "data".into(),
        };
        // Used through a trait object — proves object safety for dynamic wiring.
        let dyn_port: &dyn EvalHarnessPort = &port;
        let out = dyn_port.run_suite(&suite).expect("run");
        assert!(out.passed());
    }

    #[test]
    fn eval_harness_port_summary_counts_and_pass() {
        let s = summary(
            "s",
            1,
            vec![
                finding(EvalSeverity::Error, "boom", Some("f1")),
                finding(EvalSeverity::Warning, "meh", None),
            ],
        );
        assert!(!s.passed());
        assert_eq!(s.error_count(), 1);
        assert_eq!(s.warning_count(), 1);
    }

    #[test]
    fn eval_harness_port_severity_defaults_to_warning_on_wire() {
        let f: EvalFinding = serde_json::from_str(r#"{"message":"x"}"#).expect("parse");
        assert_eq!(f.severity, EvalSeverity::Warning);
    }

    #[test]
    fn eval_harness_port_optional_fields_omitted_not_null() {
        let f = finding(EvalSeverity::Warning, "x", None);
        let json = serde_json::to_string(&f).expect("ser");
        assert!(
            !json.contains("from"),
            "absent edge must be omitted: {json}"
        );
        assert!(!json.contains("fingerprint"));
    }

    #[test]
    fn eval_harness_port_regression_flags_new_blocking_finding() {
        let base = summary(
            "s",
            0,
            vec![finding(EvalSeverity::Warning, "old", Some("a"))],
        );
        let cur = summary(
            "s",
            1,
            vec![
                finding(EvalSeverity::Warning, "old", Some("a")),
                finding(EvalSeverity::Error, "new", Some("b")),
            ],
        );
        let report = EvalRegressionReport::compare(Some(&base), &cur);
        assert!(report.regressed());
        assert_eq!(report.new_findings.len(), 1);
        assert_eq!(report.new_findings[0].message, "new");
        assert!(report.resolved_findings.is_empty());
        assert_eq!(report.baseline_exit_code, Some(0));
    }

    #[test]
    fn eval_harness_port_regression_tracks_resolved_findings() {
        let base = summary(
            "s",
            1,
            vec![
                finding(EvalSeverity::Error, "fixed", Some("a")),
                finding(EvalSeverity::Warning, "kept", Some("b")),
            ],
        );
        let cur = summary(
            "s",
            0,
            vec![finding(EvalSeverity::Warning, "kept", Some("b"))],
        );
        let report = EvalRegressionReport::compare(Some(&base), &cur);
        assert!(!report.regressed(), "gate improved 1->0, not a regression");
        assert!(report.new_findings.is_empty());
        assert_eq!(report.resolved_findings.len(), 1);
        assert_eq!(report.resolved_findings[0].message, "fixed");
    }

    #[test]
    fn eval_harness_port_first_failing_run_is_a_regression() {
        let cur = summary(
            "s",
            1,
            vec![finding(EvalSeverity::Error, "boom", Some("a"))],
        );
        let report = EvalRegressionReport::compare(None, &cur);
        assert!(report.regressed());
        assert_eq!(report.baseline_exit_code, None);
        assert_eq!(report.new_findings.len(), 1);
    }
}
