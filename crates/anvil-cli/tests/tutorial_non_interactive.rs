use std::process::{Command, Stdio};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
fn tutorial_without_a_terminal_exits_one_with_cause_specific_copy() {
    let workspace = tempfile::tempdir().expect("workspace");
    let install_root = tempfile::tempdir().expect("anvil home");

    let output = Command::new(ANVIL_BIN)
        .arg("tutorial")
        .current_dir(workspace.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_DAEMON", "1")
        .env("ANVIL_NO_MCP", "1")
        .env("ANVIL_HOME", install_root.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run non-interactive tutorial");

    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tutorial requires an interactive terminal."),
        "unexpected refusal copy: {combined}"
    );
    assert!(
        !combined.contains("--no-tui"),
        "absent terminal must not be reported as an explicit flag: {combined}"
    );
}
