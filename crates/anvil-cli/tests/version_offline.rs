//! LAUNCH-013: `anvil version` is install-method-aware and never
//! fails on network errors. These tests run with `--offline` so they
//! work in sandboxed CI without HTTPS access.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
fn version_offline_human_prints_current_and_install_method() {
    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("version")
        .arg("--offline")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil");
    assert!(
        out.status.success(),
        "anvil version --offline failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("anvil "), "missing version line: {stdout}");
    assert!(
        stdout.contains("Installed via:"),
        "missing install method line: {stdout}"
    );
    assert!(
        stdout.contains("unavailable") || stdout.contains("up to date"),
        "missing latest-version line: {stdout}"
    );
}

#[test]
fn version_offline_json_keys_are_stable() {
    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("--json")
        .arg("version")
        .arg("--offline")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse failed: {e}\nstdout: {stdout}"));

    for key in [
        "current_version",
        "latest_version",
        "update_available",
        "install_method",
        "upgrade_command",
    ] {
        assert!(
            parsed.get(key).is_some(),
            "missing key `{key}` in version JSON: {parsed}"
        );
    }
    // Offline mode: latest_version is null, update_available is false.
    assert!(parsed["latest_version"].is_null());
    assert_eq!(parsed["update_available"], false);
    // current_version must match the binary's CARGO_PKG_VERSION exactly
    // (CLAWP-056: a non-empty check let a wrong or garbage version pass;
    // the integration test crate and the `anvil` binary are the same
    // package, so `CARGO_PKG_VERSION` here is the binary's version).
    assert_eq!(
        parsed["current_version"].as_str(),
        Some(env!("CARGO_PKG_VERSION")),
        "current_version must equal the binary's CARGO_PKG_VERSION ({}): {parsed}",
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn version_offline_does_not_require_auth() {
    // LAUNCH-013 acceptance: `anvil version` is informational and
    // must not be blocked by the auth gate. CLAWP-057: prove this by
    // running with NO auth available rather than relying on whatever
    // the parent environment happens to carry — point ANVIL_HOME at an
    // empty temp dir (no stored credentials) and strip the dev/licence
    // bypass vars, so a successful exit can only mean `version`
    // genuinely does not require auth.
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("version")
        .arg("--offline")
        .env("ANVIL_HOME", home.path())
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .env_remove("ANVIL_ADMIN_KEY")
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .expect("failed to invoke anvil");
    assert!(
        out.status.success(),
        "anvil version --offline must run without auth: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
