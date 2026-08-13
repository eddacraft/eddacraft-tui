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

/// Internal switch for the detached account-activity worker process.
///
/// Value must be exactly [`ACCOUNT_ACTIVITY_WORKER_SENTINEL`] (`"1"`), matching
/// the FLEET beacon worker. The feature key travels in
/// [`ACCOUNT_ACTIVITY_FEATURE_ENV`] so `ANVIL_INTERNAL_ACCOUNT_ACTIVITY=check`
/// cannot hijack `anvil check` / `anvil gate`.
pub const ACCOUNT_ACTIVITY_WORKER_ENV: &str = "ANVIL_INTERNAL_ACCOUNT_ACTIVITY";

/// Sentinel that marks this process as the detached activity worker.
pub const ACCOUNT_ACTIVITY_WORKER_SENTINEL: &str = "1";

/// Allowlisted feature key for the detached worker (second env, not the switch).
pub const ACCOUNT_ACTIVITY_FEATURE_ENV: &str = "ANVIL_INTERNAL_ACCOUNT_ACTIVITY_FEATURE";

/// Closed CS feature allowlist (must match API `ACCOUNT_FEATURE_KEYS`).
pub const ACCOUNT_FEATURE_KEYS: &[&str] = &["watch", "start", "check", "auth"];

const HTTP_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

#[must_use]
pub fn is_account_feature_key(key: &str) -> bool {
    ACCOUNT_FEATURE_KEYS.contains(&key)
}

/// Map a canonical CLI command name to an allowlisted account feature key.
///
/// `command_canonical_name` returns `auth-login` / `auth-whoami` /
/// `auth-refresh` (not the bare `"auth"`). Logout is not activity.
#[must_use]
pub fn feature_key_for_command(command: &str) -> Option<&'static str> {
    match command {
        "watch" => Some("watch"),
        "start" => Some("start"),
        "check" => Some("check"),
        "auth" | "auth-login" | "auth-whoami" | "auth-refresh" => Some("auth"),
        _ => None,
    }
}

/// True only when this process is the detached activity worker.
///
/// Requires the `"1"` sentinel **and** an allowlisted feature key. An
/// accidental `ANVIL_INTERNAL_ACCOUNT_ACTIVITY=check` must not skip CLI
/// parsing.
#[must_use]
pub fn is_detached_worker() -> bool {
    should_run_as_activity_worker(
        env::var(ACCOUNT_ACTIVITY_WORKER_ENV).ok().as_deref(),
        env::var(ACCOUNT_ACTIVITY_FEATURE_ENV).ok().as_deref(),
    )
}

#[must_use]
pub fn should_run_as_activity_worker(switch: Option<&str>, feature: Option<&str>) -> bool {
    switch == Some(ACCOUNT_ACTIVITY_WORKER_SENTINEL) && feature.is_some_and(is_account_feature_key)
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
        .env(
            ACCOUNT_ACTIVITY_WORKER_ENV,
            ACCOUNT_ACTIVITY_WORKER_SENTINEL,
        )
        .env(ACCOUNT_ACTIVITY_FEATURE_ENV, feature_key)
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
    if !is_detached_worker() {
        return Ok(());
    }
    let feature_key = env::var(ACCOUNT_ACTIVITY_FEATURE_ENV).unwrap_or_default();
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
        assert_eq!(feature_key_for_command("auth-login"), Some("auth"));
        assert_eq!(feature_key_for_command("auth-whoami"), Some("auth"));
        assert_eq!(feature_key_for_command("auth-refresh"), Some("auth"));
        assert_eq!(feature_key_for_command("auth-logout"), None);
        assert_eq!(feature_key_for_command("login"), None);
        assert_eq!(feature_key_for_command("fleet"), None);
        assert_eq!(feature_key_for_command("admin"), None);
    }

    #[test]
    fn worker_switch_requires_sentinel_and_allowlisted_feature() {
        assert!(should_run_as_activity_worker(Some("1"), Some("check")));
        assert!(should_run_as_activity_worker(Some("1"), Some("auth")));
        // Accidental feature-key-as-switch must not hijack the CLI.
        assert!(!should_run_as_activity_worker(Some("check"), Some("check")));
        assert!(!should_run_as_activity_worker(Some("check"), None));
        assert!(!should_run_as_activity_worker(Some("1"), Some("rm-rf")));
        assert!(!should_run_as_activity_worker(Some("1"), None));
        assert!(!should_run_as_activity_worker(None, Some("check")));
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(!is_account_feature_key("rm-rf"));
        assert!(!is_account_feature_key("anvil.watch"));
        assert!(is_account_feature_key("watch"));
    }
}
