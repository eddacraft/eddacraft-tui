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
    if args.first() != Some(&"check") {
        return false;
    }
    if args.iter().any(|arg| *arg == "--all" || *arg == "gate") {
        return false;
    }
    let Some(separator) = args.iter().position(|arg| *arg == "--") else {
        return false;
    };
    args.get(separator + 1)
        .is_some_and(|path| !path.is_empty() && !path.starts_with('-'))
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
        assert!(!is_legal_followup_argv(&["check", "--json", "--no-tui"]));
        assert!(!is_legal_followup_argv(&[
            "check", "--json", "--no-tui", "--"
        ]));
        assert!(!is_legal_followup_argv(&["gate", "--profile", "ci"]));
    }

    #[test]
    fn storm_argv_is_outside_the_watch_churn_contract() {
        // GTAO-005 CI-visible proof is the argv shape (scoped `check`, never
        // `--all`/`gate`), not a live process-tree sample. The watch churn
        // ceiling already budgets one scoped child; a `--all`/`gate`
        // regression is what would return to the ADR-061 storm.
        assert!(is_legal_followup_argv(&[
            "check",
            "--json",
            "--no-tui",
            "--",
            "src/lib.rs"
        ]));
        assert!(!is_legal_followup_argv(&["check", "--all"]));
        assert!(!is_legal_followup_argv(&["gate"]));
        let _ = ResourceBudget::ANVIL_WATCH_CHURN_V1;
    }
}
