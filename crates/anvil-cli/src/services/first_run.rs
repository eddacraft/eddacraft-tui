use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FirstRunMarker {
    created_epoch_secs: String,
    version: String,
}

/// Return the path to the first-run marker file inside `.anvil/`.
pub fn first_run_marker_path() -> anyhow::Result<PathBuf> {
    Ok(crate::util::workspace_root()?
        .join(".anvil")
        .join("first-run"))
}

/// Check whether this is a first run (marker file does not exist).
///
/// Uses `try_exists()` and treats filesystem errors conservatively as
/// "not first run" so unreadable marker paths do not repeatedly trigger
/// first-run behaviour.
pub fn is_first_run(marker_path: &Path) -> bool {
    match marker_path.try_exists() {
        Ok(exists) => !exists,
        Err(err) => {
            eprintln!(
                "[welcome] warning: cannot check first-run marker at {}: {err}",
                marker_path.display()
            );
            false
        }
    }
}

/// Check whether the `ANVIL_SKIP_WELCOME` env var requests skipping.
///
/// Accepts `"1"`, `"true"`, or `"yes"` (case-insensitive).
pub fn should_skip_welcome() -> bool {
    std::env::var("ANVIL_SKIP_WELCOME")
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

/// Remove the first-run marker so the next `welcome` invocation behaves as a
/// fresh install.
pub fn delete_first_run_marker(path: &Path) -> anyhow::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).context("failed to remove first-run marker"),
    }
}

/// Write the first-run marker file to disk atomically.
pub fn create_first_run_marker(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("failed to create .anvil directory")?;
    }

    let marker = FirstRunMarker {
        created_epoch_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let json = serde_json::to_string_pretty(&marker)?;
    crate::util::atomic_write(path, json.as_bytes()).context("failed to write first-run marker")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_marker_path_is_anchored() {
        let path = first_run_marker_path().unwrap();
        assert!(
            path.is_absolute(),
            "marker path should be absolute, got: {path:?}"
        );
        assert!(path.ends_with(".anvil/first-run"));
    }

    #[test]
    fn create_marker_writes_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("first-run");

        create_first_run_marker(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let marker: FirstRunMarker = serde_json::from_str(&content).unwrap();
        assert_eq!(marker.version, env!("CARGO_PKG_VERSION"));
        assert!(!marker.created_epoch_secs.is_empty());
    }

    #[test]
    fn is_first_run_true_when_marker_absent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(is_first_run(&dir.path().join("first-run")));
    }

    #[test]
    fn is_first_run_false_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("first-run");
        create_first_run_marker(&path).unwrap();
        assert!(!is_first_run(&path));
    }

    #[test]
    fn delete_marker_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("first-run");
        create_first_run_marker(&path).unwrap();
        assert!(!is_first_run(&path));

        delete_first_run_marker(&path).unwrap();
        assert!(is_first_run(&path));
    }

    #[test]
    fn delete_marker_noop_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("first-run");
        // Should not error when file doesn't exist.
        delete_first_run_marker(&path).unwrap();
    }

    #[test]
    fn should_skip_welcome_true_when_env_set_to_one() {
        temp_env::with_var("ANVIL_SKIP_WELCOME", Some("1"), || {
            assert!(should_skip_welcome());
        });
    }

    #[test]
    fn should_skip_welcome_false_when_env_unset() {
        temp_env::with_var_unset("ANVIL_SKIP_WELCOME", || {
            assert!(!should_skip_welcome());
        });
    }

    #[test]
    fn should_skip_welcome_true_when_env_set_to_yes() {
        temp_env::with_var("ANVIL_SKIP_WELCOME", Some("yes"), || {
            assert!(should_skip_welcome());
        });
    }

    #[test]
    fn should_skip_welcome_true_when_env_set_to_true() {
        temp_env::with_var("ANVIL_SKIP_WELCOME", Some("true"), || {
            assert!(should_skip_welcome());
        });
    }

    #[test]
    fn should_skip_welcome_true_when_env_set_to_true_uppercase() {
        temp_env::with_var("ANVIL_SKIP_WELCOME", Some("TRUE"), || {
            assert!(should_skip_welcome());
        });
    }

    #[test]
    fn should_skip_welcome_false_when_env_set_to_other() {
        temp_env::with_var("ANVIL_SKIP_WELCOME", Some("nah"), || {
            assert!(!should_skip_welcome());
        });
    }

    #[test]
    fn should_skip_welcome_false_when_env_set_to_zero() {
        temp_env::with_var("ANVIL_SKIP_WELCOME", Some("0"), || {
            assert!(!should_skip_welcome());
        });
    }
}
