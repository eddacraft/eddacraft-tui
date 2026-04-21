//! Shared onboarding defaults for `anvil init` and the welcome flow.
//!
//! A single source of truth so the guided-setup wizard in `welcome`, the
//! plain `init` command, and any fallback paths can never drift to
//! different check-name vocabularies. Before this module lived here the
//! welcome flow offered `secret-scan`/`anti-pattern` (names that did not
//! match any other path) while `init` wrote `secret-detection`/
//! `import-boundaries`.
//!
//! Scope: the names defined here control what is written to `.anvilrc`
//! and shown in the guided-init menu, status, and doctor output. They
//! do not drive `anvil gate` / `anvil check` dispatch — that runs on a
//! separate hardcoded vocabulary (`gate.rs::AVAILABLE_CHECKS`). See
//! tracking issue for reconciliation.

use anvil_tui::surfaces::init::AvailableCheck;

/// Check names written to a freshly-generated `.anvilrc`.
pub(crate) const DEFAULT_CHECK_NAMES: &[&str] = &["secret-detection", "import-boundaries"];

/// Returns [`DEFAULT_CHECK_NAMES`] as an owned `Vec<String>`, the shape
/// most callers need when building a config.
pub(crate) fn default_check_names() -> Vec<String> {
    DEFAULT_CHECK_NAMES.iter().map(|s| (*s).to_string()).collect()
}

/// Default menu of checks offered during guided init.
///
/// The first two match [`DEFAULT_CHECK_NAMES`] and are enabled by
/// default; the remainder are available to opt into. These names flow
/// into `.anvilrc` and status/doctor display — they are not the same
/// vocabulary the gate runner dispatches on.
pub(crate) fn default_available_checks() -> Vec<AvailableCheck> {
    vec![
        AvailableCheck {
            name: "secret-detection".to_string(),
            description: "Detect leaked secrets and credentials".to_string(),
            enabled: true,
        },
        AvailableCheck {
            name: "import-boundaries".to_string(),
            description: "Enforce module import boundaries".to_string(),
            enabled: true,
        },
        AvailableCheck {
            name: "antipattern-scan".to_string(),
            description: "Detect common code antipatterns".to_string(),
            enabled: false,
        },
        AvailableCheck {
            name: "architecture".to_string(),
            description: "Validate architecture definitions".to_string(),
            enabled: false,
        },
        AvailableCheck {
            name: "policy".to_string(),
            description: "Evaluate OPA policy rules".to_string(),
            enabled: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_check_names_are_canonical() {
        // If this fails, the menu names drifted from the .anvilrc names
        // and the empty-checks fallback path will register unknown IDs.
        assert_eq!(DEFAULT_CHECK_NAMES, &["secret-detection", "import-boundaries"]);
    }

    #[test]
    fn default_available_checks_enabled_match_default_names() {
        let checks = default_available_checks();
        let enabled_names: Vec<&str> = checks
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(enabled_names, DEFAULT_CHECK_NAMES);
    }

    #[test]
    fn default_available_checks_count() {
        let checks = default_available_checks();
        assert_eq!(checks.len(), 5);
    }
}
