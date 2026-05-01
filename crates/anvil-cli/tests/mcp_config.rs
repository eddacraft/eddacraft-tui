//! RCLI3-016: integration tests for `anvil mcp-config`.
//!
//! Covers the surface promises in the demo runbook: `--write` lands the
//! config at the right per-target path, `--verify` exits non-zero when the
//! file is missing, and the printed preview is parseable JSON.

use std::fs;
use std::process::Command;

use serde_json::Value;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run(workspace: &std::path::Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui").arg("mcp-config");
    cmd.args(extra);
    cmd.arg("--workspace").arg(workspace);
    cmd.env("ANVIL_DEV", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

fn run_mcp(workspace: &std::path::Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui").arg("mcp");
    cmd.args(extra);
    cmd.arg("--workspace").arg(workspace);
    cmd.env("ANVIL_DEV", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

fn run_mcp_json(workspace: &std::path::Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui").arg("--json").arg("mcp");
    cmd.args(extra);
    cmd.arg("--workspace").arg(workspace);
    cmd.env("ANVIL_DEV", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

#[cfg(unix)]
fn run_mcp_from(cwd: &std::path::Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui").arg("mcp");
    cmd.args(extra);
    cmd.current_dir(cwd);
    cmd.env("ANVIL_DEV", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

fn assert_rust_stdio_entry(parsed: &Value, expected_command: &str) {
    let entry = &parsed["mcpServers"]["anvil"];
    assert_eq!(entry["command"], expected_command);
    assert_eq!(
        entry["args"],
        serde_json::json!(["mcp", "serve", "--stdio"])
    );
}

fn assert_claude_code_rust_stdio_entry(parsed: &Value, expected_command: &str) {
    let entry = &parsed["mcpServers"]["anvil"];
    assert_eq!(entry["type"], "stdio");
    assert_eq!(entry["command"], expected_command);
    assert_eq!(
        entry["args"],
        serde_json::json!(["mcp", "serve", "--stdio"])
    );
}

#[test]
fn write_creates_claude_code_config_at_dot_claude_json() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), &["--target", "claude-code", "--write"]);

    assert!(
        out.status.success(),
        "anvil mcp-config --write exited {:?}\nstdout: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let config_path = dir.path().join(".claude.json");
    assert!(config_path.exists(), "expected config at {config_path:?}");

    let raw = fs::read_to_string(&config_path).unwrap();
    let parsed: Value = serde_json::from_str(&raw).expect("config is valid JSON");
    assert!(
        parsed["mcpServers"]["anvil"].is_object(),
        "expected mcpServers.anvil entry, got {parsed}"
    );
}

#[test]
fn write_creates_cursor_config_at_dot_cursor_mcp_json() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), &["--target", "cursor", "--write"]);
    assert!(out.status.success());
    assert!(dir.path().join(".cursor").join("mcp.json").exists());
}

#[test]
fn write_creates_vscode_config_with_type_field() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), &["--target", "vscode", "--write"]);
    assert!(out.status.success());

    let path = dir.path().join(".vscode").join("settings.json");
    let raw = fs::read_to_string(&path).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["mcp"]["servers"]["anvil"]["type"], "stdio");
}

#[test]
fn verify_exits_non_zero_when_config_missing() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), &["--target", "claude-code", "--verify"]);
    assert!(
        !out.status.success(),
        "expected --verify to exit non-zero on missing config; got {:?}",
        out.status,
    );
}

#[test]
fn verify_succeeds_after_write() {
    let dir = tempfile::tempdir().unwrap();
    let write = run(dir.path(), &["--target", "cursor", "--write"]);
    assert!(write.status.success());

    let verify = run(dir.path(), &["--target", "cursor", "--verify"]);
    assert!(
        verify.status.success(),
        "expected --verify to succeed after --write; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr),
    );
}

#[test]
fn mcp_config_verify_rejects_unrelated_stdio_command() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cursor").join("mcp.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": {
                    "command": "node",
                    "args": ["mcp", "serve", "--stdio"],
                    "env": {}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run(dir.path(), &["--target", "cursor", "--verify"]);

    assert!(
        !verify.status.success(),
        "mcp-config --verify must reject unrelated stdio commands"
    );
}

#[test]
fn mcp_config_verify_accepts_exact_command_override() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claude.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": {
                    "type": "stdio",
                    "command": "/opt/anvil/bin/anvil",
                    "args": ["mcp", "serve", "--stdio"],
                    "env": {}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run(
        dir.path(),
        &[
            "--target",
            "claude-code",
            "--verify",
            "--command",
            "/opt/anvil/bin/anvil",
        ],
    );

    assert!(
        verify.status.success(),
        "mcp-config --verify should accept exact command override\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr),
    );
}

#[test]
fn preview_without_write_prints_parseable_json() {
    // The preview prints commentary lines plus the JSON. Strip comments and
    // confirm what's left round-trips through serde_json.
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), &["--target", "claude-code"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json_part: String = stdout
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let parsed: Value = serde_json::from_str(&json_part).expect("preview is valid JSON");
    assert_claude_code_rust_stdio_entry(&parsed, "anvil");
}

#[test]
fn write_is_idempotent_and_preserves_unrelated_keys() {
    // Pre-seed an existing cursor config with a foreign server. The anvil
    // install must add its own entry without clobbering the existing one,
    // and a second --write must produce byte-identical output (no
    // re-ordering, no stray whitespace, no growth).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cursor").join("mcp.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let seeded = serde_json::json!({
        "mcpServers": {
            "other": { "command": "other-bin", "args": [] }
        }
    });
    fs::write(&path, serde_json::to_string_pretty(&seeded).unwrap()).unwrap();

    let first = run(dir.path(), &["--target", "cursor", "--write"]);
    assert!(first.status.success());
    let after_first = fs::read_to_string(&path).unwrap();
    let parsed: Value = serde_json::from_str(&after_first).unwrap();
    assert!(parsed["mcpServers"]["other"].is_object());
    assert!(parsed["mcpServers"]["anvil"].is_object());

    let second = run(dir.path(), &["--target", "cursor", "--write"]);
    assert!(second.status.success());
    let after_second = fs::read_to_string(&path).unwrap();
    assert_eq!(
        after_first, after_second,
        "second --write must produce byte-identical output"
    );
}

#[test]
fn write_refuses_when_existing_config_is_invalid_json() {
    // A malformed config (e.g. JSONC with comments, hand-edited typo)
    // must not be silently overwritten — that would clobber the user's
    // other MCP servers and editor settings.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cursor").join("mcp.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{ /* a comment that breaks JSON */ }").unwrap();

    let out = run(dir.path(), &["--target", "cursor", "--write"]);
    assert!(!out.status.success(), "must exit non-zero on invalid JSON");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("refusing to overwrite") || stderr.contains("not valid JSON"),
        "stderr must explain refusal: {stderr}"
    );
    // Original content must be preserved.
    let after = fs::read_to_string(&path).unwrap();
    assert!(after.contains("a comment"), "original file untouched");
}

#[test]
fn mcp_install_cursor_writes_rust_stdio_entry_and_verify_succeeds() {
    let dir = tempfile::tempdir().unwrap();

    let install = run_mcp(dir.path(), &["install", "--client", "cursor"]);
    assert!(
        install.status.success(),
        "install failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr),
    );

    let path = dir.path().join(".cursor").join("mcp.json");
    let raw = fs::read_to_string(&path).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert_rust_stdio_entry(&parsed, "anvil");

    let verify = run_mcp(dir.path(), &["install", "--client", "cursor", "--verify"]);
    assert!(
        verify.status.success(),
        "verify failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr),
    );
}

#[test]
fn mcp_install_claude_code_writes_rust_stdio_entry_and_verify_succeeds() {
    let dir = tempfile::tempdir().unwrap();

    let install = run_mcp(dir.path(), &["install", "--client", "claude-code"]);
    assert!(install.status.success());

    let path = dir.path().join(".claude.json");
    let raw = fs::read_to_string(&path).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert_claude_code_rust_stdio_entry(&parsed, "anvil");

    let verify = run_mcp(
        dir.path(),
        &["install", "--client", "claude-code", "--verify"],
    );
    assert!(verify.status.success());
}

#[test]
#[cfg(unix)]
fn mcp_install_defaults_to_home_client_config_root() {
    let home = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();

    temp_env::with_vars(
        [
            ("HOME", Some(home.path().to_str().unwrap())),
            ("USERPROFILE", Some(home.path().to_str().unwrap())),
        ],
        || {
            let install = run_mcp_from(cwd.path(), &["install", "--client", "cursor"]);
            assert!(
                install.status.success(),
                "install failed\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&install.stdout),
                String::from_utf8_lossy(&install.stderr),
            );

            assert!(home.path().join(".cursor").join("mcp.json").exists());
            assert!(!cwd.path().join(".cursor").join("mcp.json").exists());
        },
    );
}

#[test]
fn mcp_install_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cursor").join("mcp.json");

    let first = run_mcp(dir.path(), &["install", "--client", "cursor"]);
    assert!(first.status.success());
    let after_first = fs::read_to_string(&path).unwrap();

    let second = run_mcp(dir.path(), &["install", "--client", "cursor"]);
    assert!(second.status.success());
    let after_second = fs::read_to_string(&path).unwrap();

    assert_eq!(
        after_first, after_second,
        "second install must leave config byte-identical"
    );
}

#[test]
fn mcp_install_warns_when_rewriting_drifted_entry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cursor").join("mcp.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": { "command": "node", "args": ["legacy-mcp.js"] }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let install = run_mcp(dir.path(), &["install", "--client", "cursor"]);
    assert!(install.status.success());
    let stdout = String::from_utf8_lossy(&install.stdout);
    assert!(
        stdout.contains("drifted") || stdout.contains("rewrote"),
        "stdout should warn about drifted entry: {stdout}"
    );

    let raw = fs::read_to_string(&path).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert_rust_stdio_entry(&parsed, "anvil");
}

#[test]
fn mcp_install_claude_code_command_override_writes_user_config_and_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let command = "/tmp/fake/anvil";

    let install = run_mcp(
        dir.path(),
        &["install", "--client", "claude-code", "--command", command],
    );
    assert!(
        install.status.success(),
        "install failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr),
    );

    let path = dir.path().join(".claude.json");
    assert!(
        path.exists(),
        "expected Claude Code user config at {path:?}"
    );
    let raw = fs::read_to_string(&path).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert_claude_code_rust_stdio_entry(&parsed, command);

    let verify = run_mcp(
        dir.path(),
        &[
            "install",
            "--client",
            "claude-code",
            "--verify",
            "--command",
            command,
        ],
    );
    assert!(
        verify.status.success(),
        "verify failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr),
    );
}

#[test]
fn mcp_install_verify_accepts_non_default_command() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claude.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": {
                    "type": "stdio",
                    "command": "/opt/anvil/bin/anvil",
                    "args": ["mcp", "serve", "--stdio"],
                    "env": {}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run_mcp(
        dir.path(),
        &[
            "install",
            "--client",
            "claude-code",
            "--verify",
            "--command",
            "/opt/anvil/bin/anvil",
        ],
    );

    assert!(
        verify.status.success(),
        "non-default command should verify\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr),
    );
}

#[test]
fn mcp_install_verify_rejects_non_default_command_without_override() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claude.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": {
                    "type": "stdio",
                    "command": "/opt/anvil/bin/anvil",
                    "args": ["mcp", "serve", "--stdio"],
                    "env": {}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run_mcp(
        dir.path(),
        &["install", "--client", "claude-code", "--verify"],
    );

    assert!(
        !verify.status.success(),
        "non-default command should require explicit --command verification"
    );
}

#[test]
fn mcp_install_rejects_blank_command_override() {
    let dir = tempfile::tempdir().unwrap();

    let install = run_mcp(
        dir.path(),
        &["install", "--client", "claude-code", "--command", "  "],
    );

    assert!(
        !install.status.success(),
        "blank command override must fail"
    );
    assert!(
        !dir.path().join(".claude.json").exists(),
        "blank command must not write a config"
    );
}

#[test]
fn mcp_install_trims_command_override() {
    let dir = tempfile::tempdir().unwrap();

    let install = run_mcp(
        dir.path(),
        &[
            "install",
            "--client",
            "claude-code",
            "--command",
            "  /tmp/fake/anvil  ",
        ],
    );

    assert!(install.status.success());
    let raw = fs::read_to_string(dir.path().join(".claude.json")).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert_claude_code_rust_stdio_entry(&parsed, "/tmp/fake/anvil");
}

#[test]
fn mcp_install_verify_rejects_unrelated_command_even_with_valid_args() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cursor").join("mcp.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": {
                    "command": "node",
                    "args": ["mcp", "serve", "--stdio"],
                    "env": {}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run_mcp(dir.path(), &["install", "--client", "cursor", "--verify"]);

    assert!(
        !verify.status.success(),
        "unrelated command must fail verification"
    );
}

#[test]
fn mcp_install_verify_command_override_requires_exact_match() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claude.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": {
                    "type": "stdio",
                    "command": "/tmp/actual/anvil",
                    "args": ["mcp", "serve", "--stdio"],
                    "env": {}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run_mcp(
        dir.path(),
        &[
            "install",
            "--client",
            "claude-code",
            "--verify",
            "--command",
            "/tmp/expected/anvil",
        ],
    );

    assert!(
        !verify.status.success(),
        "explicit --command should verify the exact configured command"
    );
}

#[test]
fn mcp_install_verify_claude_code_requires_stdio_type() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claude.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": {
                    "command": "/tmp/fake/anvil",
                    "args": ["mcp", "serve", "--stdio"],
                    "env": {}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run_mcp(
        dir.path(),
        &["install", "--client", "claude-code", "--verify"],
    );

    assert!(
        !verify.status.success(),
        "Claude Code verify must require type=stdio"
    );
}

#[test]
fn mcp_install_refuses_non_object_mcp_servers_container() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cursor").join("mcp.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": []
        }))
        .unwrap(),
    )
    .unwrap();

    let install = run_mcp(dir.path(), &["install", "--client", "cursor"]);

    assert!(
        !install.status.success(),
        "non-object mcpServers must fail instead of silently skipping insert"
    );
}

#[test]
fn mcp_install_refuses_non_object_config_root() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cursor").join("mcp.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!([])).unwrap(),
    )
    .unwrap();

    let install = run_mcp(dir.path(), &["install", "--client", "cursor"]);

    assert!(
        !install.status.success(),
        "non-object config root must fail instead of being replaced"
    );
}

#[test]
fn mcp_config_write_refuses_non_object_vscode_servers_container() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".vscode").join("settings.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcp": { "servers": [] }
        }))
        .unwrap(),
    )
    .unwrap();

    let write = run(dir.path(), &["--target", "vscode", "--write"]);

    assert!(
        !write.status.success(),
        "non-object mcp.servers must fail instead of silently skipping insert"
    );
}

#[test]
fn mcp_install_verify_fails_when_rust_stdio_entry_is_malformed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cursor").join("mcp.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": { "command": "anvil", "args": ["mcp", "serve"] }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run_mcp(dir.path(), &["install", "--client", "cursor", "--verify"]);
    assert!(
        !verify.status.success(),
        "malformed entry must fail verification"
    );
    let stderr = String::from_utf8_lossy(&verify.stderr);
    assert!(
        stderr.contains("malformed") || stderr.contains("expected command"),
        "stderr should explain malformed entry: {stderr}"
    );
}

#[test]
fn mcp_install_verify_json_uses_machine_readable_expected_type() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claude.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": {
                    "type": "pipe",
                    "command": "anvil",
                    "args": ["mcp", "serve", "--stdio"],
                    "env": {}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run_mcp_json(
        dir.path(),
        &["install", "--client", "claude-code", "--verify"],
    );

    assert!(!verify.status.success());
    let stderr = String::from_utf8_lossy(&verify.stderr);
    let parsed: Value = serde_json::from_str(stderr.trim()).expect("stderr is JSON");
    assert_eq!(parsed["expected"]["type"], "stdio");
    assert_eq!(parsed["expected"]["typeRequired"], true);
}

#[test]
fn mcp_install_verify_fails_when_command_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claude.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": { "args": ["mcp", "serve", "--stdio"] }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run_mcp(
        dir.path(),
        &["install", "--client", "claude-code", "--verify"],
    );

    assert!(
        !verify.status.success(),
        "missing command must fail verification"
    );
    let stderr = String::from_utf8_lossy(&verify.stderr);
    assert!(
        stderr.contains("malformed") || stderr.contains("expected command"),
        "stderr should explain missing command: {stderr}"
    );
}

#[test]
fn mcp_install_verify_rejects_malformed_args_with_non_default_command() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".claude.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "anvil": {
                    "type": "stdio",
                    "command": "/opt/anvil/bin/anvil",
                    "args": ["mcp", "serve"],
                    "env": {}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let verify = run_mcp(
        dir.path(),
        &[
            "install",
            "--client",
            "claude-code",
            "--verify",
            "--command",
            "/opt/anvil/bin/anvil",
        ],
    );

    assert!(
        !verify.status.success(),
        "malformed args must fail verification regardless of command"
    );
}
