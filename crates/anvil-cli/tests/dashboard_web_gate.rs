//! DASH-012: binary-level contract for the `anvil dashboard --web`
//! feature-flag gate (`dashboard.web`, default-off).
//!
//! Unit tests in `feature_flags.rs` cover the gate decision; these pin
//! dispatch + exit behaviour:
//! - gate closed → exit 1 (`EXIT_ERROR`), not auth-required
//! - gate open via `ANVIL_DASHBOARD_WEB=1` or `ANVIL_DEV=1` → gate is passed
//!   (the command does not refuse with the feature_disabled envelope)

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const EXIT_ERROR: i32 = 1;
const EXIT_AUTH_REQUIRED: i32 = 3;

fn dashboard_web() -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(["dashboard", "--web", "--no-open", "--port", "0"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1")
        .env("ANVIL_LOG", "off")
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_DASHBOARD_WEB")
        .env_remove("ANVIL_ADMIN_KEY");
    cmd
}

#[test]
fn dashboard_web_refused_when_gate_closed() {
    let output = dashboard_web()
        .output()
        .expect("failed to invoke anvil binary");

    assert_eq!(
        output.status.code(),
        Some(EXIT_ERROR),
        "closed gate must exit EXIT_ERROR (1); stderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_ne!(
        output.status.code(),
        Some(EXIT_AUTH_REQUIRED),
        "feature-flag refusal must not look like auth-required",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("dashboard.web") || stderr.contains("ANVIL_DASHBOARD_WEB"),
        "refusal must name the flag or opt-in: {stderr}",
    );
}

#[test]
fn dashboard_web_json_refusal_envelope_on_stdout() {
    let output = dashboard_web()
        .arg("--json")
        .output()
        .expect("failed to invoke anvil binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(EXIT_ERROR),
        "exit code must stay EXIT_ERROR: stdout=\n{stdout}\nstderr=\n{stderr}",
    );

    let envelope: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!(
            "expected the refusal JSON envelope on stdout ({err}): \
             stdout=\n{stdout}\nstderr=\n{stderr}"
        )
    });
    assert_eq!(
        envelope["error"], "feature_disabled",
        "unexpected envelope shape: stdout=\n{stdout}",
    );
    assert_eq!(
        envelope["flag"], "dashboard.web",
        "envelope must name the flag: stdout=\n{stdout}",
    );
}

// The open-gate check uses spawn + read rather than .output() because the
// server stays alive.
#[test]
fn dashboard_web_startup_envelope_with_opt_in() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::time::Duration;

    let mut child = Command::new(ANVIL_BIN)
        .args(["--json", "dashboard", "--web", "--no-open", "--port", "0"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1")
        .env("ANVIL_LOG", "off")
        .env("ANVIL_DASHBOARD_WEB", "1")
        .env_remove("ANVIL_DEV")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn anvil dashboard --web");

    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    // Bound wait so a hang fails the test rather than CI.
    let start = std::time::Instant::now();
    let mut got = false;
    while start.elapsed() < Duration::from_secs(15) {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                got = true;
                break;
            }
            Err(_) => break,
        }
    }

    let _ = child.kill();
    let _ = child.wait();

    assert!(got, "expected a startup JSON line within 15s");
    let envelope: serde_json::Value =
        serde_json::from_str(line.trim()).expect("startup envelope must be JSON");
    assert_ne!(
        envelope.get("error").and_then(|v| v.as_str()),
        Some("feature_disabled"),
        "open gate must not emit feature_disabled: {line}"
    );
    assert!(
        envelope.get("url").and_then(|v| v.as_str()).is_some(),
        "open gate must emit a url: {line}"
    );
}
