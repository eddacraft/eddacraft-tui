//! MCPLH-008: daily MCP self-heal and easy pin.
//!
//! Pin freezes daily ensure. Emergency `mcp refresh` still rewrites.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn isolated(home: &Path) -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ANVIL_HOME", home)
        .env("XDG_RUNTIME_DIR", home.join("runtime"))
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("LOCALAPPDATA", home)
        .env("ANVIL_NO_DAEMON", "1")
        .env_remove("ANVIL_MCP_PIN")
        .env_remove("GROK_HOME")
        .env_remove("COPILOT_HOME");
    cmd
}

fn cursor_config(home: &Path) -> PathBuf {
    home.join(".cursor").join("mcp.json")
}

fn write_cursor_cellar(home: &Path) {
    let path = cursor_config(home);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "mcpServers": {
                    "anvil": {
                        "command": "/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil",
                        "args": ["mcp", "serve", "--stdio"],
                        "env": {}
                    }
                }
            }))
            .unwrap()
        ),
    )
    .unwrap();
}

fn cursor_command(home: &Path) -> String {
    let raw = fs::read_to_string(cursor_config(home)).unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    value["mcpServers"]["anvil"]["command"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn mcp_pin_and_unpin_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let pin = isolated(home.path())
        .args(["mcp", "pin", "0.9.2-beta"])
        .output()
        .expect("mcp pin");
    assert_success(&pin);
    let stdout = String::from_utf8_lossy(&pin.stdout);
    assert!(
        stdout.contains("pinned") && stdout.contains("0.9.2-beta"),
        "{stdout}"
    );
    assert!(home.path().join("mcp-heal.pin").is_file());

    let unpin = isolated(home.path())
        .args(["mcp", "unpin"])
        .output()
        .expect("mcp unpin");
    assert_success(&unpin);
    assert!(!home.path().join("mcp-heal.pin").is_file());
}

fn init_activated_repo(root: &Path) {
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .expect("git init");
    assert!(git.success(), "git init");
    fs::write(root.join(".anvil.json"), "{}\n").expect("write config");
}

#[test]
fn bare_ensure_leaves_cellar_when_pinned() {
    let home = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    init_activated_repo(repo.path());
    write_cursor_cellar(home.path());
    let pin = isolated(home.path())
        .args(["mcp", "pin"])
        .output()
        .expect("mcp pin");
    assert_success(&pin);

    let ensure = isolated(home.path())
        .current_dir(repo.path())
        .output()
        .expect("bare anvil");
    assert_success(&ensure);
    assert_eq!(
        cursor_command(home.path()),
        "/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil"
    );
    let stdout = String::from_utf8_lossy(&ensure.stdout);
    assert!(
        stdout.contains("pinned"),
        "ensure should name the pin: {stdout}"
    );
}

#[test]
fn mcp_refresh_rewrites_when_pinned() {
    let home = tempfile::tempdir().unwrap();
    write_cursor_cellar(home.path());
    let pin = isolated(home.path())
        .args(["mcp", "pin"])
        .output()
        .expect("mcp pin");
    assert_success(&pin);

    let refresh = isolated(home.path())
        .args(["mcp", "refresh", "--workspace"])
        .arg(home.path())
        .output()
        .expect("mcp refresh");
    assert_success(&refresh);
    assert_eq!(cursor_command(home.path()), "anvil");
    let stderr = String::from_utf8_lossy(&refresh.stderr);
    assert!(
        stderr.contains("pinned") && stderr.contains("emergency"),
        "refresh must say it is an emergency override: {stderr}"
    );
}
