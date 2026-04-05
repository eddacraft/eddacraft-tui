use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FirstRunMarker {
    created_at: String,
    version: String,
}

/// Return the path to the first-run marker file inside `.anvil/`.
pub fn first_run_marker_path() -> anyhow::Result<PathBuf> {
    Ok(crate::util::workspace_root()?
        .join(".anvil")
        .join("first-run"))
}

/// Check whether this is a first run (marker file does not exist).
pub fn is_first_run(marker_path: &Path) -> bool {
    !marker_path.exists()
}

/// Check whether the `ANVIL_SKIP_WELCOME` env var is set to `"1"`.
pub fn should_skip_welcome() -> bool {
    std::env::var("ANVIL_SKIP_WELCOME")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Write the first-run marker file to disk atomically.
pub fn create_first_run_marker(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("failed to create .anvil directory")?;
    }

    let marker = FirstRunMarker {
        created_at: format!(
            "{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ),
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
        assert!(!marker.created_at.is_empty());
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
    fn should_skip_welcome_false_when_env_set_to_other() {
        temp_env::with_var("ANVIL_SKIP_WELCOME", Some("yes"), || {
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
