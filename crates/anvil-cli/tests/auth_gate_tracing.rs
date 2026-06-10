//! CIB-061: the pre-dispatch auth gate fires for unauthenticated gated
//! commands as an *expected state* (issue #1822: exit 0, human stderr
//! message from `check_auth`). The tracing event that records the state
//! must therefore not pass the CLI's default `warn` filter (CIB-024) —
//! it leaked a raw JSON line onto stderr directly under the human
//! message on the beta golden path.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Build an unauthenticated `anvil status` invocation: empty HOME/XDG so
/// no credentials resolve, no dev bypass, no log-filter overrides.
fn unauthenticated_status(home: &std::path::Path, cwd: &std::path::Path) -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("status")
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home)
        // Point XDG at an empty dir so no credentials resolve.
        .env("XDG_CONFIG_HOME", home.join("xdg"))
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .env_remove("ANVIL_LOG")
        .env_remove("RUST_LOG")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1");
    cmd
}

#[test]
fn auth_gate_emits_no_tracing_json_at_default_filter() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let output = unauthenticated_status(home.path(), dir.path())
        .output()
        .expect("failed to invoke anvil binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("anvil auth login"),
        "expected the human auth message on stderr: stderr=\n{stderr}"
    );
    // Format-agnostic: no tracing JSON of any level may reach stderr at
    // the default filter — not just the WARN line this regressed as.
    assert!(
        !stderr
            .lines()
            .any(|line| line.trim_start().starts_with('{')),
        "no raw JSON line may leak onto stderr at the default filter \
         (CIB-061): stderr=\n{stderr}"
    );
    assert!(
        output.status.success(),
        "auth-required is an expected state (issue #1822) and exits 0"
    );
}

#[test]
fn auth_gate_event_still_visible_to_operators_at_info_filter() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut cmd = unauthenticated_status(home.path(), dir.path());
    cmd.env("ANVIL_LOG", "info");
    let output = cmd.output().expect("failed to invoke anvil binary");

    // The subscriber emits JSON lines to stderr (CIB-024); at `info`
    // there are other info events too (e.g. the subscriber-install and
    // command-parsed lines), so match the event's JSON `message` field
    // rather than scanning for bare text.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("\"message\":\"cli command authentication required\""),
        "operators opting in via ANVIL_LOG=info must still see the \
         auth-gate event as a tracing JSON line: stderr=\n{stderr}"
    );
}
