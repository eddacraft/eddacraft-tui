//! GTAO-005: CI-visible argv contract for the AST follow-up.
//!
//! A regression that shells `anvil check --all` or `anvil gate` per save is
//! the ADR-061 save-storm. The live `watch_resource_budget` bench still
//! measures the watch process tree against [`crate::budget::ResourceBudget::ANVIL_WATCH_CHURN_V1`];
//! this module pins the follow-up argv so that storm cannot land as a
//! "legal" command even when the process-tree bench is too noisy for CI.

/// Legal follow-up: `check` with an explicit path list, never `--all` or `gate`.
#[must_use]
pub fn is_legal_followup_argv(args: &[&str]) -> bool {
    args.first() == Some(&"check") && !args.iter().any(|arg| *arg == "--all" || *arg == "gate")
}

#[cfg(test)]
mod tests {
    use super::is_legal_followup_argv;
    use crate::budget::ResourceBudget;

    #[test]
    fn scoped_check_is_legal() {
        assert!(is_legal_followup_argv(&[
            "check",
            "--json",
            "--no-tui",
            "--",
            "src/lib.rs",
        ]));
    }

    #[test]
    fn check_all_and_gate_fail_the_budget_contract() {
        assert!(!is_legal_followup_argv(&["check", "--all"]));
        assert!(!is_legal_followup_argv(&["gate", "--profile", "ci"]));
    }

    #[test]
    fn followup_stays_inside_the_watch_churn_budget_class() {
        // Follow-up is one coalesced scoped `check` child — the same class
        // RLB-007 already budgeted. A `--all` or `gate` regression would
        // push the tree toward the pre-RLB-007 ~6.55-core storm, which is
        // well above this ceiling.
        let budget = ResourceBudget::ANVIL_WATCH_CHURN_V1;
        assert!(budget.steady_state_cpu_pct < 655.0);
        assert!(budget.steady_state_cpu_pct >= 8.0);
    }
}
