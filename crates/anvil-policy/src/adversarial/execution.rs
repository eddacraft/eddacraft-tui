//! ATC-003 — probe execution folded into the eval-harness report shape.
//!
//! This bridges the adversarial probe catalog (ATC-001/ATC-002) into the
//! existing eval-regression machinery so probe runs appear **alongside** policy
//! suites in an eval-regression summary, are persisted in the same history
//! store, and are diffed by the same [`EvalRegressionReport`] logic.
//!
//! ## How the frozen contract stays intact
//!
//! The integration is deliberately *additive by reuse*: a probe run is projected
//! onto the existing, unchanged [`EvalRunSummary`] / [`EvalFinding`] /
//! [`EvalSeverity`] types. **No field is added to, and no field is removed from,
//! any eval type**, so the frozen `anvil policy eval --json` v1 contract and the
//! EVALCI baseline shape are untouched — a probe summary is simply a value of the
//! same shape a policy suite produces. Each probe *category* becomes its own
//! synthetic suite named `probe:<category>` (see [`PROBE_SUITE_PREFIX`]). This
//! keeps every probe's category in the `suite` string — a field the frozen
//! schema already carries — so adversarial trend reporting (ATC-004) can recover
//! per-category history from the store without any schema change.
//!
//! ## Determinism
//!
//! Execution is deterministic: a probe asserts an [`ExpectedOutcome`], a
//! [`ProbeExecutor`] reports the *observed* outcome, and the probe passes iff the
//! two match. Nothing here attacks anything or drives a live model — the executor
//! is the injection point where a real system (or a recorded fixture) supplies
//! the observed outcome. Category suites are emitted in sorted category order.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use anvil_kernel_types::{ExpectedOutcome, Probe, ProbeCategory};

use super::registry::ProbePack;
use crate::eval::{EvalFinding, EvalRunSummary, EvalSeverity};

/// The `suite` prefix marking an [`EvalRunSummary`] as an adversarial probe
/// category suite rather than a policy suite. The category's kebab-case label
/// follows the prefix (e.g. `probe:prompt-injection`).
///
/// This is a **reserved prefix**: a policy eval suite name must never start with
/// it, or its records would be mis-attributed as adversarial trend data by
/// [`category_from_suite`]. Committed suites use colon-free `snake_case`
/// (`arch_boundary`), so the separation holds by convention; [`is_reserved_suite_name`]
/// is the guard that enforces it — `anvil policy eval-regression` calls it when
/// loading a suite manifest, rejecting any suite name that collides with this
/// prefix before it can reach the shared eval store.
pub const PROBE_SUITE_PREFIX: &str = "probe:";

/// The contract version stamped on a projected probe summary — the same `"1.0.0"`
/// the frozen v1 eval output declares, so a probe record is indistinguishable in
/// shape from a policy-suite record.
const PROBE_SCHEMA_VERSION: &str = "1.0.0";

/// The synthetic suite name for a probe `category`.
#[must_use]
pub fn probe_suite_name(category: ProbeCategory) -> String {
    format!("{PROBE_SUITE_PREFIX}{}", category.as_str())
}

/// The category label carried by a probe suite name, or `None` if `suite` is not
/// a probe suite. The returned label is the kebab-case
/// [`ProbeCategory`](anvil_kernel_types::ProbeCategory) wire form.
///
/// `"probe:"` with no category suffix is not itself a valid probe suite, so it
/// also returns `None` rather than leaking an empty category label into trend
/// reporting and grouping keys.
#[must_use]
pub fn category_from_suite(suite: &str) -> Option<&str> {
    match suite.strip_prefix(PROBE_SUITE_PREFIX) {
        Some("") | None => None,
        Some(category) => Some(category),
    }
}

/// Whether `suite` uses the reserved [`PROBE_SUITE_PREFIX`]. A policy eval suite
/// name must not — this is the guard that keeps the probe and policy record
/// namespaces from colliding in the shared eval store.
#[must_use]
pub fn is_reserved_suite_name(suite: &str) -> bool {
    suite.starts_with(PROBE_SUITE_PREFIX)
}

/// Supplies the *observed* safe-behaviour outcome for a probe.
///
/// This is the injection point between the deterministic probe catalog and a
/// real system under test (or a recorded fixture): given a probe, it reports what
/// the system actually did. It does not attack or execute anything itself — it
/// observes and classifies.
pub trait ProbeExecutor {
    /// Observe the outcome the system exhibits for `probe`.
    fn observe(&self, probe: &Probe) -> ExpectedOutcome;
}

/// The outcome of running one probe: what it asserted, what was observed, and
/// whether they matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeOutcome {
    /// The probe's id.
    pub probe_id: String,
    /// The probe's category.
    pub category: ProbeCategory,
    /// The safe behaviour the probe asserted.
    pub expected: ExpectedOutcome,
    /// The safe behaviour actually observed.
    pub observed: ExpectedOutcome,
    /// Whether the observation met the assertion.
    pub passed: bool,
}

/// The result of running every probe in a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeRunReport {
    /// The pack the probes came from.
    pub pack_id: String,
    /// Per-probe outcomes, in the pack's declared order.
    pub outcomes: Vec<ProbeOutcome>,
}

impl ProbeRunReport {
    /// Whether every probe in the run passed.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.outcomes.iter().all(|o| o.passed)
    }

    /// Project this run onto the frozen eval-harness summary shape — one
    /// [`EvalRunSummary`] per category, so probe results fold into an
    /// eval-regression report alongside policy suites and persist in the same
    /// store.
    ///
    /// A failing probe becomes an `error`-severity [`EvalFinding`] fingerprinted
    /// by its probe id (so the regression diff tracks it precisely); a category
    /// with any failure exits non-zero, matching the warnings-over-blocks exit
    /// contract. Category suites are returned in sorted category order for
    /// determinism.
    #[must_use]
    pub fn to_eval_summaries(&self) -> Vec<EvalRunSummary> {
        // Group by the category's stable wire label so the ordering is
        // deterministic without requiring `Ord` on the category enum.
        let mut by_category: BTreeMap<&'static str, Vec<&ProbeOutcome>> = BTreeMap::new();
        for outcome in &self.outcomes {
            by_category
                .entry(outcome.category.as_str())
                .or_default()
                .push(outcome);
        }

        by_category
            .into_iter()
            .map(|(category, outcomes)| {
                let findings: Vec<EvalFinding> = outcomes
                    .iter()
                    .filter(|o| !o.passed)
                    .map(|o| EvalFinding {
                        severity: EvalSeverity::Error,
                        message: format!(
                            "probe `{}` expected {:?} but observed {:?}",
                            o.probe_id, o.expected, o.observed
                        ),
                        from: None,
                        to: None,
                        fingerprint: Some(o.probe_id.clone()),
                    })
                    .collect();
                let exit_code = i32::from(!findings.is_empty());
                EvalRunSummary {
                    suite: format!("{PROBE_SUITE_PREFIX}{category}"),
                    schema_version: PROBE_SCHEMA_VERSION.to_string(),
                    policy: self.pack_id.clone(),
                    query: "adversarial.probes".to_string(),
                    findings,
                    exit_code,
                }
            })
            .collect()
    }
}

/// Run every probe in `pack` through `executor`, returning a [`ProbeRunReport`].
///
/// A probe passes iff the executor's observed outcome equals the probe's
/// asserted [`ExpectedOutcome`]. Outcomes preserve the pack's declared order.
#[must_use]
pub fn run_probe_pack(executor: &impl ProbeExecutor, pack: &ProbePack) -> ProbeRunReport {
    let outcomes = pack
        .probes
        .iter()
        .map(|probe| {
            let observed = executor.observe(probe);
            ProbeOutcome {
                probe_id: probe.id.clone(),
                category: probe.category,
                expected: probe.expected_outcome,
                observed,
                passed: observed == probe.expected_outcome,
            }
        })
        .collect();
    ProbeRunReport {
        pack_id: pack.id.clone(),
        outcomes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::{EvalRecord, EvalRegressionReport};
    use anvil_kernel_types::PayloadClass;

    fn probe(id: &str, category: ProbeCategory, expected: ExpectedOutcome) -> Probe {
        Probe::new(
            id,
            category,
            PayloadClass::DirectInstruction,
            expected,
            "1.0.0",
            "test probe",
        )
    }

    fn pack(id: &str, probes: Vec<Probe>) -> ProbePack {
        ProbePack {
            id: id.into(),
            name: "n".into(),
            version: "1.0.0".into(),
            description: "d".into(),
            owner: "o".into(),
            probes,
        }
    }

    /// An executor that returns a fixed observed outcome per probe id.
    struct Fixture(std::collections::HashMap<String, ExpectedOutcome>);
    impl ProbeExecutor for Fixture {
        fn observe(&self, probe: &Probe) -> ExpectedOutcome {
            *self.0.get(&probe.id).unwrap_or(&ExpectedOutcome::Unknown)
        }
    }

    /// An executor that always reports the probe's own asserted outcome — every
    /// probe passes.
    struct AlwaysSafe;
    impl ProbeExecutor for AlwaysSafe {
        fn observe(&self, probe: &Probe) -> ExpectedOutcome {
            probe.expected_outcome
        }
    }

    #[test]
    fn adversarial_eval_integration_suite_name_round_trips() {
        let name = probe_suite_name(ProbeCategory::PromptInjection);
        assert_eq!(name, "probe:prompt-injection");
        assert_eq!(category_from_suite(&name), Some("prompt-injection"));
        // A non-probe suite is not misread as a probe suite.
        assert_eq!(category_from_suite("arch_boundary"), None);
        // The bare reserved prefix has no category suffix — not a valid probe
        // suite either, so it must not yield an empty category label.
        assert_eq!(category_from_suite(PROBE_SUITE_PREFIX), None);
    }

    #[test]
    fn adversarial_eval_integration_all_pass_run_is_clean() {
        let p = pack(
            "baseline",
            vec![probe(
                "pi-1",
                ProbeCategory::PromptInjection,
                ExpectedOutcome::Refused,
            )],
        );
        let report = run_probe_pack(&AlwaysSafe, &p);
        assert!(report.all_passed());

        let summaries = report.to_eval_summaries();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].suite, "probe:prompt-injection");
        assert!(summaries[0].passed());
        assert_eq!(summaries[0].exit_code, 0);
        assert!(summaries[0].findings.is_empty());
    }

    #[test]
    fn adversarial_eval_integration_failing_probe_becomes_error_finding() {
        let p = pack(
            "baseline",
            vec![probe(
                "pi-1",
                ProbeCategory::PromptInjection,
                ExpectedOutcome::Refused,
            )],
        );
        // The system did NOT refuse — it warned; the probe fails.
        let mut observed = std::collections::HashMap::new();
        observed.insert("pi-1".to_string(), ExpectedOutcome::Warned);
        let report = run_probe_pack(&Fixture(observed), &p);
        assert!(!report.all_passed());
        assert!(!report.outcomes[0].passed);

        let summaries = report.to_eval_summaries();
        assert_eq!(summaries.len(), 1);
        let summary = &summaries[0];
        assert!(!summary.passed());
        assert_eq!(summary.exit_code, 1);
        assert_eq!(summary.error_count(), 1);
        assert_eq!(summary.findings[0].severity, EvalSeverity::Error);
        assert_eq!(summary.findings[0].fingerprint.as_deref(), Some("pi-1"));
    }

    #[test]
    fn adversarial_eval_integration_emits_one_suite_per_category_sorted() {
        // Two categories, authored out of sorted order; summaries come back
        // sorted by category label for deterministic reporting.
        let p = pack(
            "baseline",
            vec![
                probe(
                    "pi-1",
                    ProbeCategory::PromptInjection,
                    ExpectedOutcome::Refused,
                ),
                probe(
                    "ex-1",
                    ProbeCategory::DataExfiltration,
                    ExpectedOutcome::Blocked,
                ),
            ],
        );
        let summaries = run_probe_pack(&AlwaysSafe, &p).to_eval_summaries();
        let suites: Vec<&str> = summaries.iter().map(|s| s.suite.as_str()).collect();
        assert_eq!(
            suites,
            ["probe:data-exfiltration", "probe:prompt-injection"]
        );
    }

    #[test]
    fn adversarial_eval_integration_reserved_prefix_is_recognised() {
        assert!(is_reserved_suite_name("probe:prompt-injection"));
        assert!(!is_reserved_suite_name("arch_boundary"));
        assert_eq!(
            category_from_suite("probe:tool-misuse"),
            Some("tool-misuse")
        );
        assert_eq!(category_from_suite("arch_boundary"), None);
    }

    #[test]
    fn adversarial_eval_integration_flows_through_regression_report() {
        // A probe summary is an ordinary EvalRunSummary, so the existing
        // regression diff treats a newly-failing probe as a regression — proving
        // probes fold into the eval harness with zero contract change.
        let p = pack(
            "baseline",
            vec![probe(
                "pi-1",
                ProbeCategory::PromptInjection,
                ExpectedOutcome::Refused,
            )],
        );
        let clean = run_probe_pack(&AlwaysSafe, &p).to_eval_summaries();

        let mut observed = std::collections::HashMap::new();
        observed.insert("pi-1".to_string(), ExpectedOutcome::Warned);
        let regressed = run_probe_pack(&Fixture(observed), &p).to_eval_summaries();

        let report = EvalRegressionReport::compare(Some(&clean[0]), &regressed[0]);
        assert!(report.regressed(), "a newly-failing probe is a regression");
        assert_eq!(report.new_findings.len(), 1);
    }

    #[test]
    fn adversarial_eval_integration_summary_persists_in_frozen_store_schema() {
        // The projected summary round-trips through the frozen EvalRecord store
        // schema unchanged — the integration adds no field to the eval contract.
        let p = pack(
            "baseline",
            vec![probe(
                "pi-1",
                ProbeCategory::PromptInjection,
                ExpectedOutcome::Refused,
            )],
        );
        let summary = &run_probe_pack(&AlwaysSafe, &p).to_eval_summaries()[0];
        let record = EvalRecord::from_summary(summary, "run-1", "2026-07-04T00:00:00Z");
        assert_eq!(record.suite, "probe:prompt-injection");
        assert_eq!(record.schema_version, "1.0.0");
        assert!(record.passed);
        // The reconstructed summary equals the original — no lossy projection.
        assert_eq!(&record.to_summary(), summary);
    }
}
