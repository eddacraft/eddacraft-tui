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

/// Recognised project-config filenames, in probe order.
///
/// `.anvilrc` is what `anvil init` writes. The `.anvil.<ext>` names are
/// supported hand-authored / migrated layouts (`anvil config show` and
/// `anvil_config::discover` already recognise them). `.anvil.yml` is
/// included so this list matches discover precedence.
const PROJECT_CONFIG_NAMES: &[&str] = &[
    ".anvilrc",
    ".anvil.yaml",
    ".anvil.yml",
    ".anvil.json",
    ".anvil.toml",
];

/// Check whether an anvil configuration file already exists in the
/// current working directory.
pub fn config_exists() -> bool {
    let Ok(cwd) = std::env::current_dir() else {
        return false;
    };
    config_exists_in(&cwd)
}

/// Check whether a non-empty anvil configuration file exists under `dir`.
///
/// The CLI writes `.anvilrc` regardless of serialisation format; the
/// `.anvil.{yaml,yml,json,toml}` names are retained for tolerance against
/// hand-authored configs or future layout changes.
///
/// Zero-byte files are treated as absent so a stray `touch .anvilrc`
/// does not cause init to silently skip and leave the user without a
/// working configuration.
pub fn config_exists_in(dir: &std::path::Path) -> bool {
    existing_config_name_in(dir).is_some()
}

/// Return the first non-empty project-config filename under `dir`, if any.
///
/// Used by `anvil init` so the refusal message names the file that was
/// actually detected (`.anvil.yaml`, not a phantom `.anvilrc`).
pub fn existing_config_name_in(dir: &std::path::Path) -> Option<&'static str> {
    PROJECT_CONFIG_NAMES.iter().copied().find(|name| {
        std::fs::metadata(dir.join(name)).is_ok_and(|m| m.is_file() && m.len() > 0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_exists_returns_false_in_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!config_exists_in(tmp.path()));
        assert_eq!(existing_config_name_in(tmp.path()), None);
    }

    #[test]
    fn config_exists_detects_anvilrc() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), "{}").unwrap();

        assert!(config_exists_in(tmp.path()));
        assert_eq!(existing_config_name_in(tmp.path()), Some(".anvilrc"));
    }

    #[test]
    fn config_exists_detects_anvil_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), "schemaVersion: 1.0.0\n").unwrap();

        assert!(config_exists_in(tmp.path()));
        assert_eq!(existing_config_name_in(tmp.path()), Some(".anvil.yaml"));
    }

    #[test]
    fn config_exists_ignores_empty_anvilrc() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), b"").unwrap();

        assert!(
            !config_exists_in(tmp.path()),
            "zero-byte .anvilrc must be treated as missing config"
        );
        assert_eq!(existing_config_name_in(tmp.path()), None);
    }
}
