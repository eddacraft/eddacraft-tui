//! LAUNCH-004 validation: `anvil init --force` lands on a real first
//! signal of value (the sample analysis section) rather than a flat
//! "Run `anvil doctor`" stub.

use std::fs;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
fn init_force_prints_post_init_analysis_section() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("sample.ts"),
        "export const x = 1;\nexport const y = 2;\n",
    )
    .unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("init")
        .arg("--force")
        .current_dir(dir.path())
        // Skip the welcome chain so the run terminates cleanly.
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_DEV", "1")
        .output()
        .expect("failed to invoke anvil binary");

    assert!(
        output.status.success(),
        "anvil init exited with {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("First scan"),
        "expected 'First scan' section in stdout, got:\n{stdout}",
    );
    assert!(
        stdout.contains("Scanned"),
        "expected 'Scanned N file(s)' line in stdout, got:\n{stdout}",
    );
    // Issue #1107: the analysis result also surfaces `anvil auth login`
    // so a new user knows how to authenticate before hitting the gate
    // path's "Session expired" / "Authentication required".
    assert!(
        stdout.contains("anvil auth login"),
        "expected post-analysis hint pointing at `anvil auth login`, got:\n{stdout}",
    );
}

#[test]
fn init_force_post_analysis_shows_empty_tree_hint() {
    // No source files in the temp dir — analysis should land on a
    // discoverable next-step hint ("anvil tutorial" / "anvil watch") so
    // a brand-new project does not look like the tool failed. The init
    // command itself must still succeed.
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("init")
        .arg("--force")
        .current_dir(dir.path())
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_DEV", "1")
        .output()
        .expect("failed to invoke anvil binary");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("First scan"),
        "expected 'First scan' header in empty-tree hint, got:\n{stdout}",
    );
    assert!(
        stdout.contains("No source files yet"),
        "expected empty-tree hint copy in stdout, got:\n{stdout}",
    );
    assert!(
        stdout.contains("anvil tutorial"),
        "expected next-step hint pointing at `anvil tutorial`, got:\n{stdout}",
    );
    // Issue #1107: the empty-tree hint surfaces `anvil auth login` as a
    // next step so a brand-new user sees how to authenticate before
    // they hit "Session expired" / "Authentication required" on a
    // gate-evaluated check.
    assert!(
        stdout.contains("anvil auth login"),
        "expected next-step hint pointing at `anvil auth login`, got:\n{stdout}",
    );
    // The "0 files scanned" noise that LAUNCH-004 originally wanted to
    // suppress must still NOT appear — the hint replaces it, not adds
    // to it.
    assert!(
        !stdout.contains("Scanned 0 file"),
        "empty-tree hint must not show '0 files' counter, got:\n{stdout}",
    );
}

#[test]
fn init_json_mode_skips_post_analysis() {
    // JSON mode preserves the existing config schema — the analysis is
    // a human-facing wow surface, not part of the machine contract.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("sample.ts"), "export const x = 1;\n").unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .arg("init")
        .arg("--force")
        .current_dir(dir.path())
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_DEV", "1")
        .output()
        .expect("failed to invoke anvil binary");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("First scan"),
        "JSON mode should not emit the human analysis section, got:\n{stdout}",
    );
    // Sanity: JSON output is still a valid config document.
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("JSON mode stdout must parse as JSON");
    assert!(parsed.get("schemaVersion").is_some());
}

#[test]
fn json_mode_auth_failure_emits_only_json_error() {
    let dir = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .arg("status")
        .current_dir(dir.path())
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_LOG", "off")
        .env("ANVIL_NO_PROMPT", "1")
        .env("XDG_CONFIG_HOME", config_home.path())
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .output()
        .expect("failed to invoke anvil binary");

    // CIB-169: `status` is the read-only state surface — auth-required is
    // the expected informational answer, so it stays exit 0 with the
    // informational envelope (action commands now exit 3).
    // CIB-049: the envelope is structured data, so per the `--json`
    // stream policy (`docs/guides/cli-output-streams.md`) it lands on
    // **stdout** — a JSON consumer piping stdout must receive it.
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!("stdout must be one JSON object, got stdout={stdout:?} stderr={stderr:?}: {err}")
    });
    assert_eq!(
        parsed.get("state").and_then(|value| value.as_str()),
        Some("authRequired"),
        "action-command auth-required envelope must use the informational shape: {stdout}",
    );
    assert_eq!(
        parsed.get("next").and_then(|value| value.as_str()),
        Some("anvil auth login"),
    );
    assert!(
        parsed.get("error").is_none(),
        "informational envelope must not carry an `error` key (reserved for the probe shape): {stdout}",
    );
    assert_eq!(
        stdout.trim().lines().count(),
        1,
        "stdout should contain only the JSON auth envelope: {stdout}",
    );
    assert!(
        stderr.trim().is_empty(),
        "no human chatter or duplicate envelope on stderr in JSON mode: {stderr}",
    );
    // No need to grep for human-language strings: a single-line stdout
    // that parses cleanly as one JSON object proves nothing else leaked.
    // The phrase "Authentication required" / "Run `anvil auth login`"
    // now lives *inside* the envelope's `message` / `next` fields by
    // design, so a substring check would self-trip.
}

/// CIB-049 / CIB-169: `anvil start` is the activation surface scripts
/// drive with `--json`. Unauthenticated it must exit 3 (CIB-169: action
/// commands carry the auth signal so `anvil start && deploy` stops) AND
/// deliver the `authRequired` envelope on **stdout** — before CIB-049 the
/// envelope went to stderr, so a JSON consumer reading stdout got nothing.
#[test]
fn start_json_auth_failure_emits_envelope_on_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .arg("start")
        .current_dir(dir.path())
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_LOG", "off")
        .env("ANVIL_NO_PROMPT", "1")
        .env("XDG_CONFIG_HOME", config_home.path())
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .output()
        .expect("failed to invoke anvil binary");

    assert_eq!(
        output.status.code(),
        Some(3),
        "auth-required on an action command must exit 3 (CIB-169); stderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!("stdout must be one JSON object, got stdout={stdout:?} stderr={stderr:?}: {err}")
    });
    assert_eq!(
        parsed.get("state").and_then(|value| value.as_str()),
        Some("authRequired"),
        "unauthenticated `start --json` must emit the informational envelope on stdout: {stdout}",
    );
    assert!(
        stderr.trim().is_empty(),
        "no human chatter or duplicate envelope on stderr in JSON mode: {stderr}",
    );
}

#[test]
fn json_verbose_edict_auth_failure_emits_only_json_error() {
    let dir = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();
    // CIB-221: only an explicit `is_edict: true` marker forces the edict
    // re-verify path. `ANVIL_LICENSE` alone is an ordinary session and must
    // not be treated as an edict via the `anvil_beta_*` prefix.
    let anvil_dir = config_home.path().join("anvil");
    std::fs::create_dir_all(&anvil_dir).unwrap();
    std::fs::write(
        anvil_dir.join("credentials.json"),
        r#"{"license":"anvil_beta_bad","isEdict":true}"#,
    )
    .unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .arg("--verbose")
        .arg("status")
        .current_dir(dir.path())
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_LOG", "off")
        .env("ANVIL_NO_PROMPT", "1")
        .env("ANVIL_API_URL", "http://127.0.0.1:9")
        .env("XDG_CONFIG_HOME", config_home.path())
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .output()
        .expect("failed to invoke anvil binary");

    // CIB-169: edict re-verify failure on the read-only `status` surface
    // treats auth-required as an expected state and exits 0 with the
    // informational envelope (same shape as missing creds).
    // CIB-049: the envelope is structured data → stdout (stream policy).
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!("stdout must be one JSON object, got stdout={stdout:?} stderr={stderr:?}: {err}")
    });
    assert_eq!(
        parsed.get("state").and_then(|value| value.as_str()),
        Some("authRequired"),
        "action-command auth-required envelope must use the informational shape: {stdout}",
    );
    assert!(
        parsed.get("error").is_none(),
        "informational envelope must not carry an `error` key: {stdout}",
    );
    assert_eq!(
        stdout.trim().lines().count(),
        1,
        "stdout should contain only the JSON auth envelope: {stdout}",
    );
    // `[auth]` is a verbose-only human diagnostic and must never leak
    // under --json — stderr must stay clean even with --verbose.
    assert!(
        stderr.trim().is_empty(),
        "no human chatter (incl. verbose `[auth]` diagnostics) on stderr under --json: {stderr}",
    );
}
