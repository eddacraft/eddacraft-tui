//! Shared onboarding defaults for `anvil init` and the welcome flow.

use anvil_tui::surfaces::init::AvailableCheck;

use crate::commands::check_catalog::{default_init_available_checks, default_init_check_names};

/// Returns the default init checks as an owned `Vec<String>`, the shape
/// most callers need when building a config.
pub(crate) fn default_check_names() -> Vec<String> {
    default_init_check_names()
}

/// Default menu of checks offered during guided init.
///
/// These names flow into `.anvilrc` and the guided-init UI.
pub(crate) fn default_available_checks() -> Vec<AvailableCheck> {
    default_init_available_checks()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::check_catalog::DEFAULT_INIT_CHECKS;

    #[test]
    fn default_check_names_are_canonical() {
        assert_eq!(
            DEFAULT_INIT_CHECKS,
            &["secret-detection", "import-boundaries", "antipattern-scan"]
        );
    }

    #[test]
    fn default_available_checks_enabled_match_default_names() {
        let checks = default_available_checks();
        let enabled_names: Vec<&str> = checks
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(enabled_names, DEFAULT_INIT_CHECKS);
    }

    #[test]
    fn default_available_checks_count() {
        let checks = default_available_checks();
        assert_eq!(checks.len(), 4);
    }
}
