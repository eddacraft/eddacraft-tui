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
    /// (the engine's own identity for a finding); falls back to a
    /// severity + edge + message tuple when a finding carries no fingerprint.
    ///
    /// The fallback is JSON-encoded rather than delimiter-joined: `from`/`to`
    /// are arbitrary engine-supplied strings (file paths, synthesised labels)
    /// that can contain any separator character, so a naive `a|b|c` join would
    /// let two genuinely different findings collide and hide a regression.
    pub fn identity(&self) -> String {
        if let Some(fp) = &self.fingerprint {
            return format!("fp:{fp}");
        }
        // Serialising a tuple of the discriminating fields is collision-free for
        // any field contents. JSON of a fixed-shape tuple of strings cannot
        // fail; the fallback is a `Debug` rendering (also collision-free), never
        // an empty string that would collapse distinct findings together.
        let tuple = (self.severity, &self.from, &self.to, &self.message);
        let body = serde_json::to_string(&tuple).unwrap_or_else(|_| format!("{tuple:?}"));
        format!("msg:{body}")
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
        // Use sets for membership so the diff is O(n) in the finding count, not
        // O(n²) — a findings-heavy suite can emit many entries.
        let baseline_ids: std::collections::HashSet<String> = baseline
            .map(|b| b.findings.iter().map(EvalFinding::identity).collect())
            .unwrap_or_default();
        let current_ids: std::collections::HashSet<String> =
            current.findings.iter().map(EvalFinding::identity).collect();

        let new_findings = current
            .findings
            .iter()
            .filter(|f| !baseline_ids.contains(&f.identity()))
            .cloned()
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

    /// A regression is a *worsening of the gate*, read from `exit_code` — the
    /// authoritative verdict — not from finding severity.
    ///
    /// Severity in the `findings` array is deliberately **not** used: under
    /// ADR-003 an `error`-severity finding can be baselined or non-new-edge and
    /// so not block (`exit_code` stays 0), and the frozen v1 contract this binds
    /// to drops the `baselined`/`is_new_edge` flags — so a severity-based check
    /// would both miss suppressed errors and false-positive on them. `exit_code`
    /// already encodes ADR-002/003, so it is the single source of truth.
    ///
    /// Worsened means the current run blocks (`exit_code != 0`) **and** either
    /// there was no clean baseline to compare against, or the gate's verdict
    /// changed — a clean baseline now failing, or a different non-zero code
    /// (an escalation). A failing gate that is unchanged is not a *new*
    /// regression; a gate that improved (now passing) never is.
    pub fn regressed(&self) -> bool {
        self.current_exit_code != 0
            && self
                .baseline_exit_code
                .is_none_or(|baseline| baseline != self.current_exit_code)
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

    #[test]
    fn eval_harness_port_exit_code_escalation_is_a_regression() {
        // Already failing (exit 1), now exits with a different non-zero code:
        // the gate's verdict changed — a regression even though the finding set
        // is unchanged. This guards the pure `regressed()` predicate defensively;
        // note the production `SubprocessRunner` classifies a *process* exit >=2
        // as an infra error (EVALCI-003) before it ever becomes a summary, so
        // this `exit_code`-field escalation path is only reachable from a
        // producer that reports 2 in JSON while exiting 0 or 1.
        let base = summary("s", 1, vec![finding(EvalSeverity::Error, "x", Some("a"))]);
        let cur = summary("s", 2, vec![finding(EvalSeverity::Error, "x", Some("a"))]);
        let report = EvalRegressionReport::compare(Some(&base), &cur);
        assert!(report.regressed());
    }

    #[test]
    fn eval_harness_port_unchanged_failing_gate_is_not_a_new_regression() {
        let base = summary("s", 1, vec![finding(EvalSeverity::Error, "x", Some("a"))]);
        let cur = summary("s", 1, vec![finding(EvalSeverity::Error, "x", Some("a"))]);
        let report = EvalRegressionReport::compare(Some(&base), &cur);
        assert!(!report.regressed());
    }

    #[test]
    fn eval_harness_port_clean_gate_with_suppressed_error_finding_is_not_a_regression() {
        // ADR-003: an error-severity finding can be present yet baselined /
        // non-new-edge, so `exit_code` stays 0. The frozen contract drops the
        // suppression flags, so the verdict must come from exit_code — a clean
        // gate is never a regression regardless of finding severity.
        let base = summary("s", 0, vec![]);
        let cur = summary(
            "s",
            0,
            vec![finding(EvalSeverity::Error, "suppressed", Some("b"))],
        );
        let report = EvalRegressionReport::compare(Some(&base), &cur);
        assert!(
            !report.regressed(),
            "clean gate (exit 0) is not a regression"
        );
    }

    #[test]
    fn eval_suite_manifest_parses() {
        // EVALCI-005: the committed first-wave suites manifest must parse into
        // the frozen `EvalSuite` shape the `eval-regression` command loads, so a
        // malformed or drifted manifest is caught here rather than only at CI
        // runtime. Bound to the repo-root artefact (two levels up from this
        // crate) so the fixture and its consumer cannot drift apart.
        let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ci/eval/suites.json");
        let raw = std::fs::read_to_string(manifest)
            .unwrap_or_else(|e| panic!("reading committed suites manifest `{manifest}`: {e}"));
        let suites: Vec<EvalSuite> =
            serde_json::from_str(&raw).expect("suites.json parses as an array of eval suites");
        assert!(!suites.is_empty(), "manifest defines at least one suite");

        let arch = suites
            .iter()
            .find(|s| s.name == "arch_boundary")
            .expect("first-wave arch_boundary suite present");
        assert_eq!(
            arch.policy,
            PathBuf::from("policies/eval/arch_boundary.rego")
        );
        assert_eq!(arch.query, "data.anvil.policies.arch_boundary.findings");
        assert_eq!(
            arch.input.as_deref(),
            Some(std::path::Path::new(
                "policies/eval/arch_boundary.input.json"
            )),
            "the suite binds its hermetic input fixture"
        );
    }

    #[test]
    fn eval_harness_port_identity_does_not_collide_on_separator_in_edge() {
        // Two genuinely different edges that a naive `|`-join would collapse:
        // (from=a|b, to=c) vs (from=a, to=b|c).
        let a = EvalFinding {
            severity: EvalSeverity::Error,
            message: "d".into(),
            from: Some("a|b".into()),
            to: Some("c".into()),
            fingerprint: None,
        };
        let b = EvalFinding {
            severity: EvalSeverity::Error,
            message: "d".into(),
            from: Some("a".into()),
            to: Some("b|c".into()),
            fingerprint: None,
        };
        assert_ne!(a.identity(), b.identity());

        // And the regression diff sees `b` as new when the baseline only had `a`.
        let base = summary("s", 1, vec![a]);
        let cur = summary("s", 1, vec![b]);
        let report = EvalRegressionReport::compare(Some(&base), &cur);
        assert_eq!(report.new_findings.len(), 1);
        assert_eq!(report.resolved_findings.len(), 1);
    }
}
