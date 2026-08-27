//! Process-level contract for the default-off `anvil impact` rollout gate.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const EXIT_ERROR: i32 = 1;

fn impact() -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(["impact", "--no-tui"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1")
        .env("ANVIL_LOG", "off")
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_IMPACT")
        .env_remove("ANVIL_ADMIN_KEY");
    cmd
}

#[test]
fn impact_refused_when_gate_closed() {
    let output = impact().output().expect("failed to invoke anvil binary");

    assert_eq!(
        output.status.code(),
        Some(EXIT_ERROR),
        "closed impact gate must use the ordinary error exit, not succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("impact.view") || stderr.contains("ANVIL_IMPACT"),
        "refusal must name the flag or opt-in: {stderr}",
    );
}

#[test]
fn impact_json_refusal_is_structured() {
    let output = impact()
        .arg("--json")
        .output()
        .expect("failed to invoke anvil binary");

    assert_eq!(output.status.code(), Some(EXIT_ERROR));
    assert!(
        output.stderr.is_empty(),
        "structured refusal must not leak prose to stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("refusal stdout must be one JSON document");
    assert_eq!(envelope["error"], "feature_disabled");
    assert_eq!(envelope["flag"], "impact.view");
}

#[test]
fn impact_explicit_opt_in_reaches_the_command() {
    let output = impact()
        .arg("--json")
        .env("ANVIL_IMPACT", "1")
        .output()
        .expect("failed to invoke anvil binary");

    assert!(
        output.status.success(),
        "opted-in impact command must run: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("impact stdout must be one JSON document");
    assert!(
        envelope.get("graph_present").is_some(),
        "open gate must reach the impact command: {stdout}",
    );
    assert_ne!(envelope["error"], "feature_disabled");
}

#[test]
fn impact_dev_session_opts_in() {
    let output = impact()
        .arg("--json")
        .env("ANVIL_DEV", "1")
        .output()
        .expect("failed to invoke anvil binary");

    assert!(
        output.status.success(),
        "developer override must open impact: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("impact stdout must be one JSON document");
    assert!(envelope.get("graph_present").is_some());
}

#[test]
fn impact_force_off_beats_dev_session() {
    let output = impact()
        .arg("--json")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_IMPACT", "0")
        .output()
        .expect("failed to invoke anvil binary");

    assert_eq!(
        output.status.code(),
        Some(EXIT_ERROR),
        "dedicated kill switch must win over ANVIL_DEV",
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("refusal stdout must be one JSON document");
    assert_eq!(envelope["error"], "feature_disabled");
    assert_eq!(envelope["flag"], "impact.view");
}
