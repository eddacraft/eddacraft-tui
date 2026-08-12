//! BACT-005: fire-and-forget authenticated account feature-touch emission.
//!
//! Distinct from FLEET (`telemetry.rs`): requires a stored licence session and
//! posts only allowlisted feature keys to `/api/v1/account/activity`. Failures
//! never affect the user command path.

use std::env;
use std::io;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use crate::auth::credentials;

/// Internal marker for the detached account-activity worker process.
pub const ACCOUNT_ACTIVITY_WORKER_ENV: &str = "ANVIL_INTERNAL_ACCOUNT_ACTIVITY";

/// Closed CS feature allowlist (must match API `ACCOUNT_FEATURE_KEYS`).
pub const ACCOUNT_FEATURE_KEYS: &[&str] = &["watch", "start", "check", "auth"];

const HTTP_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

#[must_use]
pub fn is_account_feature_key(key: &str) -> bool {
    ACCOUNT_FEATURE_KEYS.contains(&key)
}

/// Map a canonical CLI command name to an allowlisted account feature key.
#[must_use]
pub fn feature_key_for_command(command: &str) -> Option<&'static str> {
    match command {
        "watch" => Some("watch"),
        "start" => Some("start"),
        "check" => Some("check"),
        "auth" => Some("auth"),
        _ => None,
    }
}

/// Spawn a detached worker that records one feature touch. Returns immediately.
/// No-ops when logged out, unknown key, or spawn fails.
pub fn spawn_feature_touch(feature_key: &str) {
    if !is_account_feature_key(feature_key) {
        return;
    }
    let Ok(Some(_)) = credentials::load() else {
        return;
    };
    let Ok(executable) = env::current_exe() else {
        return;
    };
    let _ = spawn_worker(&executable, feature_key);
}

fn spawn_worker(executable: &Path, feature_key: &str) -> io::Result<()> {
    std::process::Command::new(executable)
        .env(ACCOUNT_ACTIVITY_WORKER_ENV, feature_key)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
}

/// Run the detached worker. All failures are silent.
pub fn run_worker() {
    let _ = try_run_worker();
}

fn try_run_worker() -> anyhow::Result<()> {
    let feature_key = env::var(ACCOUNT_ACTIVITY_WORKER_ENV).unwrap_or_default();
    if !is_account_feature_key(&feature_key) {
        return Ok(());
    }
    let Some(creds) = credentials::load()? else {
        return Ok(());
    };
    if credentials::is_expired(&creds) {
        return Ok(());
    }

    let endpoint = format!("{}/api/v1/account/activity", crate::auth::api_url()?);
    let body = serde_json::json!({ "features": [feature_key] });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let client = reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()?;
    let _ = runtime.block_on(async {
        client
            .post(endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", creds.license),
            )
            .json(&body)
            .send()
            .await
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_matches_oq1_defaults() {
        assert_eq!(ACCOUNT_FEATURE_KEYS, &["watch", "start", "check", "auth"]);
    }

    #[test]
    fn command_mapping_covers_core_surfaces() {
        assert_eq!(feature_key_for_command("watch"), Some("watch"));
        assert_eq!(feature_key_for_command("start"), Some("start"));
        assert_eq!(feature_key_for_command("check"), Some("check"));
        assert_eq!(feature_key_for_command("auth"), Some("auth"));
        assert_eq!(feature_key_for_command("fleet"), None);
        assert_eq!(feature_key_for_command("admin"), None);
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(!is_account_feature_key("rm-rf"));
        assert!(!is_account_feature_key("anvil.watch"));
        assert!(is_account_feature_key("watch"));
    }
}
