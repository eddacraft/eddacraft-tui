//! Resource-budget evaluator for ADOPT-002.
//!
//! Pure primitive: given the pinned [`ResourceBudget`] for
//! `anvil watch` and a [`MeasurementSample`] taken by the bench
//! scenario, decide whether the measurement is under the ceiling.
//! Emits a structured [`BudgetVerdict`] that a CI step can parse
//! and fail on.
//!
//! Live measurement (the bench scenario that drives `anvil watch`
//! on the reference fixture and reports CPU + RSS) and the CI
//! workflow that pipes its output into [`evaluate`] are layered
//! on top of this primitive in follow-up steps of ADOPT-002.

use serde::{Deserialize, Serialize};

/// Pinned steady-state CPU and peak-RSS ceiling for `anvil watch`
/// on the reference benchmark fixture. See
/// `docs/policies/resource-budget.md` for the rationale.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResourceBudget {
    /// Steady-state CPU percentage. Anvil's watcher must idle
    /// below this once the initial scan settles.
    pub steady_state_cpu_pct: f64,
    /// Peak resident-set size in MiB across the measurement window.
    pub peak_rss_mib: f64,
}

impl ResourceBudget {
    /// v1 ceiling. Bumping either field requires an entry in
    /// `plans/decisions/DECISION-LOG.md` and a release note —
    /// senior users notice when the budget slips.
    pub const ANVIL_WATCH_V1: Self = Self {
        steady_state_cpu_pct: 5.0,
        peak_rss_mib: 200.0,
    };

    /// Ceiling for the **default save-time path under sustained churn** — a
    /// single `anvil watch` agent whose debounced saves each spawn a per-save
    /// `anvil check` (the production default since GH #1913). This is the cost
    /// the idle-path [`Self::ANVIL_WATCH_V1`] cannot see (RLB-002).
    ///
    /// Unit: `steady_state_cpu_pct` is scaled so `100.0` == one saturated core.
    /// RLB-007 scoped the per-save check to the changed path and measured a
    /// single churning agent at ~0.08 cores (≈8.0 here), down from ~6.55 cores
    /// (≈655.0) before. The ceiling below carries generous headroom over that
    /// floor while still tripping on a regression back toward whole-repo scans.
    ///
    /// **Calibration:** these numbers are conservative placeholders pending a
    /// quiet-box calibration owned by RLB-008 (see
    /// `docs/policies/resource-budget.md`). Tightening either axis is a budget
    /// bump and requires a `DECISION-LOG.md` entry.
    pub const ANVIL_WATCH_CHURN_V1: Self = Self {
        steady_state_cpu_pct: 50.0,
        peak_rss_mib: 300.0,
    };

    /// Idle steady-state ceiling for the **intercept daemon**
    /// (`anvil intercept start`) with no in-flight requests (RLB-003). A
    /// long-running daemon that idles hot is the class of bug this module
    /// exists to catch — the watcher report (GH #2156) was a not-quite-idle
    /// hot path. The daemon should sit at ~0 CPU and a small resident set
    /// between connections.
    ///
    /// Measured floor (release build, dev box 2026-06-02): 0% CPU, ~23 MiB RSS.
    /// The ceiling keeps generous headroom over that for allocator/hardware
    /// variance while still tripping on a daemon that idles hot or bloats.
    pub const ANVIL_INTERCEPT_IDLE_V1: Self = Self {
        steady_state_cpu_pct: 5.0,
        peak_rss_mib: 96.0,
    };

    /// Burst ceiling for the intercept daemon under sustained concurrent IPC
    /// (RLB-003): many short-lived connections each driving one JSON-RPC
    /// request through the full accept → auth → parse → dispatch → serialise
    /// pipeline. Gates that request handling does not blow CPU or leak RSS
    /// under load.
    ///
    /// Measured floor (release build, dev box 2026-06-02, 4 churn workers):
    /// ~101% CPU (≈1 core), ~23 MiB RSS (flat vs idle — no per-connection
    /// leak). The ceiling carries ~2× CPU and ~5× RSS headroom.
    ///
    /// **Calibration:** like the watch ceilings, these are quiet-box numbers
    /// with headroom; RLB-008 owns final calibration. Tightening is a budget
    /// bump (`DECISION-LOG.md`).
    pub const ANVIL_INTERCEPT_BURST_V1: Self = Self {
        steady_state_cpu_pct: 200.0,
        peak_rss_mib: 128.0,
    };

    /// Ceiling for the **MCP server** (`anvil mcp serve --stdio`) under
    /// sustained `tools/call` load (RLB-004): a driver hammers
    /// `anvil_validate_write` so the server runs the real embedded scan on
    /// each proposed buffer. MCP stdio is single-threaded and 1:1, so this is
    /// one client driving the server as fast as it answers — the server's
    /// first resource budget.
    ///
    /// Measured floor (release build, dev box 2026-06-02): ~94% CPU (the
    /// server is single-threaded, so ~1 core is its ceiling), ~24 MiB RSS,
    /// ~6.4k tool calls/s. The ceiling carries headroom for slower hardware.
    ///
    /// **Calibration:** quiet-box placeholder with headroom; RLB-008 owns
    /// final numbers. Tightening is a budget bump (`DECISION-LOG.md`).
    pub const ANVIL_MCP_BUSY_V1: Self = Self {
        steady_state_cpu_pct: 150.0,
        peak_rss_mib: 96.0,
    };

    /// Aggregate ceiling for **all three long-running processes running at
    /// once** — watch + intercept daemon + MCP server under concurrent load
    /// (RLB-005). Exposes cross-process rayon oversubscription (each process
    /// caps its own pool at N/2 cores, so three at once can oversubscribe the
    /// box). The CPU axis is generous because it is a whole-box aggregate;
    /// the value of the gate is catching a *regression* in the aggregate, and
    /// the per-process budgets above catch per-process drift.
    ///
    /// **Calibration:** quiet-box placeholder; RLB-008 owns final numbers.
    pub const ANVIL_CONCURRENT_ALL_V1: Self = Self {
        steady_state_cpu_pct: 800.0,
        peak_rss_mib: 700.0,
    };
}

/// One measurement sample produced by the bench scenario.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeasurementSample {
    pub steady_state_cpu_pct: f64,
    pub peak_rss_mib: f64,
}

/// Per-axis status; both axes are reported even on Pass so the
/// CI logs can show headroom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStatus {
    Pass,
    FailCpu,
    FailRss,
    FailBoth,
}

impl BudgetStatus {
    /// CI uses this to fail the build with a single boolean.
    #[must_use]
    pub fn is_fail(self) -> bool {
        !matches!(self, BudgetStatus::Pass)
    }
}

/// Schema version for the [`BudgetVerdict`] JSON shape. Bump
/// when adding or renaming fields — the CI script reads this to
/// decide whether its parser still understands the verdict. See
/// `docs/policies/resource-budget.md` for the schema contract.
pub const BUDGET_VERDICT_SCHEMA_VERSION: u32 = 1;

/// Structured verdict returned by [`evaluate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetVerdict {
    /// Schema version of this JSON document. Always equals
    /// [`BUDGET_VERDICT_SCHEMA_VERSION`] on emission.
    pub schema_version: u32,
    pub status: BudgetStatus,
    pub budget: ResourceBudget,
    pub sample: MeasurementSample,
    /// `sample.steady_state_cpu_pct - budget.steady_state_cpu_pct`.
    /// Negative numbers are headroom.
    pub cpu_over_pct: f64,
    /// `sample.peak_rss_mib - budget.peak_rss_mib`.
    /// Negative numbers are headroom.
    pub rss_over_mib: f64,
}

/// Pure comparison: is `sample` under `budget`?
///
/// Exact equality on either axis is a Pass (the budget is the
/// **ceiling**; touching it is allowed). Strictly greater on
/// either axis is a Fail.
///
/// # Measurement precision
///
/// The boundary semantics rely on the caller producing
/// `MeasurementSample` values at a documented precision floor.
/// The bench-scenario follow-up samples CPU to 0.1% resolution
/// and RSS to 1 MiB resolution from `/proc/stat` /
/// `MemoryGuard`; anything noisier would make the
/// "exactly-at-budget passes" rule moot. The policy doc at
/// `docs/policies/resource-budget.md` pins both numbers.
#[must_use]
pub fn evaluate(budget: ResourceBudget, sample: MeasurementSample) -> BudgetVerdict {
    let cpu_over_pct = sample.steady_state_cpu_pct - budget.steady_state_cpu_pct;
    let rss_over_mib = sample.peak_rss_mib - budget.peak_rss_mib;
    let cpu_fail = cpu_over_pct > 0.0;
    let rss_fail = rss_over_mib > 0.0;
    let status = match (cpu_fail, rss_fail) {
        (false, false) => BudgetStatus::Pass,
        (true, false) => BudgetStatus::FailCpu,
        (false, true) => BudgetStatus::FailRss,
        (true, true) => BudgetStatus::FailBoth,
    };
    BudgetVerdict {
        schema_version: BUDGET_VERDICT_SCHEMA_VERSION,
        status,
        budget,
        sample,
        cpu_over_pct,
        rss_over_mib,
    }
}

/// Render a verdict as pretty JSON for CI to upload as an
/// artifact. Stable shape; the CI assertion reads
/// `.status == "pass"`.
#[must_use]
pub fn evaluate_json(budget: ResourceBudget, sample: MeasurementSample) -> String {
    let verdict = evaluate(budget, sample);
    serde_json::to_string_pretty(&verdict)
        .expect("BudgetVerdict serializes infallibly via #[derive(Serialize)]")
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // exact-constant comparisons; values are bit-stable f64 literals
mod tests {
    use super::*;

    const V1: ResourceBudget = ResourceBudget::ANVIL_WATCH_V1;

    fn sample(cpu: f64, rss: f64) -> MeasurementSample {
        MeasurementSample {
            steady_state_cpu_pct: cpu,
            peak_rss_mib: rss,
        }
    }

    #[test]
    fn anvil_watch_v1_ceiling_is_pinned() {
        // Drift here is meaningful — surface it on a diff.
        assert_eq!(V1.steady_state_cpu_pct, 5.0);
        assert_eq!(V1.peak_rss_mib, 200.0);
    }

    #[test]
    fn sample_well_under_budget_passes() {
        let v = evaluate(V1, sample(1.0, 80.0));
        assert_eq!(v.status, BudgetStatus::Pass);
        assert!(!v.status.is_fail());
        assert!(v.cpu_over_pct < 0.0);
        assert!(v.rss_over_mib < 0.0);
    }

    #[test]
    fn sample_exactly_at_budget_passes() {
        let v = evaluate(V1, sample(V1.steady_state_cpu_pct, V1.peak_rss_mib));
        assert_eq!(v.status, BudgetStatus::Pass);
        assert_eq!(v.cpu_over_pct, 0.0);
        assert_eq!(v.rss_over_mib, 0.0);
    }

    #[test]
    fn cpu_just_over_budget_fails_cpu_only() {
        let v = evaluate(V1, sample(V1.steady_state_cpu_pct + 0.1, 50.0));
        assert_eq!(v.status, BudgetStatus::FailCpu);
        assert!(v.status.is_fail());
        assert!(v.cpu_over_pct > 0.0);
        assert!(v.rss_over_mib < 0.0);
    }

    #[test]
    fn rss_just_over_budget_fails_rss_only() {
        let v = evaluate(V1, sample(1.0, V1.peak_rss_mib + 1.0));
        assert_eq!(v.status, BudgetStatus::FailRss);
        assert!(v.cpu_over_pct < 0.0);
        assert!(v.rss_over_mib > 0.0);
    }

    #[test]
    fn both_axes_over_budget_fails_both() {
        let v = evaluate(
            V1,
            sample(V1.steady_state_cpu_pct + 5.0, V1.peak_rss_mib + 50.0),
        );
        assert_eq!(v.status, BudgetStatus::FailBoth);
        assert!(v.cpu_over_pct > 0.0);
        assert!(v.rss_over_mib > 0.0);
    }

    #[test]
    fn evaluate_preserves_budget_and_sample_in_verdict() {
        let s = sample(2.0, 120.0);
        let v = evaluate(V1, s);
        assert_eq!(v.budget, V1);
        assert_eq!(v.sample, s);
    }

    #[test]
    fn budget_status_is_fail_matches_pass_inverse() {
        for status in [
            BudgetStatus::Pass,
            BudgetStatus::FailCpu,
            BudgetStatus::FailRss,
            BudgetStatus::FailBoth,
        ] {
            let is_fail = status.is_fail();
            let is_pass = matches!(status, BudgetStatus::Pass);
            assert_ne!(is_fail, is_pass, "status {status:?} contradicts itself");
        }
    }

    #[test]
    fn evaluate_json_passes_renders_status_pass() {
        let json = evaluate_json(V1, sample(1.0, 80.0));
        assert!(json.contains("\"status\": \"pass\""));
        assert!(json.contains("\"steady_state_cpu_pct\": 5.0"));
        assert!(json.contains("\"peak_rss_mib\": 200.0"));
    }

    #[test]
    fn evaluate_json_failure_renders_status_fail_axis() {
        let json = evaluate_json(V1, sample(10.0, 80.0));
        assert!(json.contains("\"status\": \"fail_cpu\""));
        let json = evaluate_json(V1, sample(1.0, 300.0));
        assert!(json.contains("\"status\": \"fail_rss\""));
        let json = evaluate_json(V1, sample(10.0, 300.0));
        assert!(json.contains("\"status\": \"fail_both\""));
    }

    #[test]
    fn evaluate_emits_pinned_schema_version() {
        let v = evaluate(V1, sample(1.0, 80.0));
        assert_eq!(v.schema_version, BUDGET_VERDICT_SCHEMA_VERSION);
        assert_eq!(v.schema_version, 1);
        let json = evaluate_json(V1, sample(1.0, 80.0));
        assert!(json.contains("\"schema_version\": 1"));
    }

    #[test]
    fn evaluate_json_is_round_trippable() {
        let json = evaluate_json(V1, sample(2.5, 150.0));
        let parsed: BudgetVerdict = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, BudgetStatus::Pass);
        assert_eq!(parsed.budget, V1);
        assert_eq!(parsed.sample.steady_state_cpu_pct, 2.5);
        assert_eq!(parsed.sample.peak_rss_mib, 150.0);
    }
}
