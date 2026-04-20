pub mod complete;
pub mod hooks;
mod hooks_render;
pub mod init_complete;
pub mod welcome;
mod welcome_render;

pub use complete::{CompletionState, OnboardingSummary};
pub use hooks::HooksState;
pub use init_complete::{InitCompleteState, InitCompleteSummary};
pub use welcome::{OnboardingChoice, OnboardingWelcomeState};

/// Check whether an Anvil configuration file already exists in the
/// current working directory.
pub fn config_exists() -> bool {
    let Ok(cwd) = std::env::current_dir() else {
        return false;
    };
    config_exists_in(&cwd)
}

/// Check whether a non-empty Anvil configuration file exists under `dir`.
///
/// The CLI writes `.anvilrc` regardless of serialisation format; the
/// `.anvil.{yaml,json,toml}` names are retained for tolerance against
/// hand-authored configs or future layout changes.
///
/// Zero-byte files are treated as absent so a stray `touch .anvilrc`
/// does not cause init to silently skip and leave the user without a
/// working configuration.
pub fn config_exists_in(dir: &std::path::Path) -> bool {
    [".anvilrc", ".anvil.yaml", ".anvil.json", ".anvil.toml"]
        .iter()
        .any(|name| {
            std::fs::metadata(dir.join(name))
                .map(|m| m.is_file() && m.len() > 0)
                .unwrap_or(false)
        })
}

/// Default set of checks offered during guided init.
pub fn default_available_checks() -> Vec<super::init::AvailableCheck> {
    use super::init::AvailableCheck;
    vec![
        AvailableCheck {
            name: "secret-scan".to_string(),
            description: "Detect leaked secrets and API keys".to_string(),
            enabled: true,
        },
        AvailableCheck {
            name: "anti-pattern".to_string(),
            description: "Flag common code anti-patterns".to_string(),
            enabled: true,
        },
        AvailableCheck {
            name: "architecture".to_string(),
            description: "Enforce module boundary rules".to_string(),
            enabled: false,
        },
        AvailableCheck {
            name: "dependency".to_string(),
            description: "Check dependency health and licences".to_string(),
            enabled: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_exists_returns_false_in_empty_dir() {
        let tmp =
            std::env::temp_dir().join(format!("anvil_test_config_exists_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        assert!(!config_exists_in(&tmp));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn config_exists_detects_anvilrc() {
        let tmp = std::env::temp_dir().join(format!("anvil_test_anvilrc_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join(".anvilrc"), "{}").unwrap();

        assert!(config_exists_in(&tmp));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn config_exists_ignores_empty_anvilrc() {
        let tmp =
            std::env::temp_dir().join(format!("anvil_test_empty_anvilrc_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join(".anvilrc"), b"").unwrap();

        assert!(
            !config_exists_in(&tmp),
            "zero-byte .anvilrc must be treated as missing config"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn default_checks_has_four_entries() {
        let checks = default_available_checks();
        assert_eq!(checks.len(), 4);
        assert_eq!(checks[0].name, "secret-scan");
        assert_eq!(checks[1].name, "anti-pattern");
        assert_eq!(checks[2].name, "architecture");
        assert_eq!(checks[3].name, "dependency");
    }

    #[test]
    fn default_checks_first_two_enabled() {
        let checks = default_available_checks();
        assert!(checks[0].enabled, "secret-scan should be enabled");
        assert!(checks[1].enabled, "anti-pattern should be enabled");
        assert!(!checks[2].enabled, "architecture should be disabled");
        assert!(!checks[3].enabled, "dependency should be disabled");
    }
}
