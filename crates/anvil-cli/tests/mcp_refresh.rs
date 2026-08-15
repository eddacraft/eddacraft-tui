//! MCPLH-003: `anvil mcp refresh` bulk cascade.
//!
//! Dry-run must not mutate configs or the install-scoped generation file.
//! A real run rewrites Anvil-owned Cellar pins to bare `anvil`, leaves
//! foreign entries untouched, bumps generation, and reports without
//! signalling live MCP children.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn isolated(home: &Path) -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .env("ANVIL_DEV", "1")
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ANVIL_HOME", home)
        .env("XDG_RUNTIME_DIR", home.join("runtime"))
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("LOCALAPPDATA", home)
        .env_remove("GROK_HOME")
        .env_remove("COPILOT_HOME");
    cmd
}

fn run_refresh(home: &Path, extra: &[&str]) -> Output {
    let mut cmd = isolated(home);
    cmd.arg("mcp").arg("refresh");
    cmd.args(extra);
    cmd.arg("--workspace").arg(home);
    cmd.output().expect("invoke anvil mcp refresh")
}

fn run_refresh_json(home: &Path, extra: &[&str]) -> Output {
    let mut cmd = isolated(home);
    cmd.arg("--json").arg("mcp").arg("refresh");
    cmd.args(extra);
    cmd.arg("--workspace").arg(home);
    cmd.output().expect("invoke anvil mcp refresh --json")
}

fn cursor_config(home: &Path) -> PathBuf {
    home.join(".cursor").join("mcp.json")
}

fn generation_path(home: &Path) -> PathBuf {
    home.join("mcp-refresh.generation")
}

fn write_cursor_anvil(home: &Path, command: &str, extra_server: Option<Value>) {
    let path = cursor_config(home);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut servers = serde_json::Map::new();
    servers.insert(
        "anvil".into(),
        json!({
            "command": command,
            "args": ["mcp", "serve", "--stdio"],
            "env": {}
        }),
    );
    if let Some(other) = extra_server {
        servers.insert("other".into(), other);
    }
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({ "mcpServers": servers })).unwrap()
        ),
    )
    .unwrap();
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn mcp_refresh_dry_run_does_not_mutate_drifted_entry_or_generation() {
    let home = tempfile::tempdir().unwrap();
    let cellar = "/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil";
    write_cursor_anvil(home.path(), cellar, None);
    fs::write(generation_path(home.path()), "4\n").unwrap();
    let before = fs::read_to_string(cursor_config(home.path())).unwrap();

    let output = run_refresh(
        home.path(),
        &[
            "--dry-run",
            "--clients",
            "cursor",
            "--daemon",
            "reuse",
            "--processes",
            "none",
        ],
    );
    assert_success(&output);

    assert_eq!(
        fs::read_to_string(cursor_config(home.path())).unwrap(),
        before,
        "dry-run must not rewrite a drifted owned entry"
    );
    assert_eq!(
        fs::read_to_string(generation_path(home.path())).unwrap(),
        "4\n",
        "dry-run must not bump the refresh generation"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("would") || stdout.contains("dry"),
        "dry-run should say what it would do: {stdout}"
    );
    assert!(
        !stdout.to_ascii_lowercase().contains("restart all"),
        "refresh copy must not lead with restarting all agents: {stdout}"
    );
}

#[test]
fn mcp_refresh_rewrites_cellar_command_and_bumps_generation() {
    let home = tempfile::tempdir().unwrap();
    let cellar = "/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil";
    write_cursor_anvil(
        home.path(),
        cellar,
        Some(json!({
            "command": "npx",
            "args": ["-y", "someone-else"]
        })),
    );
    assert!(!generation_path(home.path()).exists());

    let output = run_refresh(
        home.path(),
        &[
            "--clients",
            "cursor",
            "--daemon",
            "reuse",
            "--processes",
            "none",
        ],
    );
    assert_success(&output);

    let raw = fs::read_to_string(cursor_config(home.path())).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["mcpServers"]["anvil"]["command"], "anvil");
    assert_eq!(
        parsed["mcpServers"]["anvil"]["args"],
        json!(["mcp", "serve", "--stdio"])
    );
    assert!(
        !raw.contains("Cellar"),
        "Cellar pin must be rewritten: {raw}"
    );
    assert_eq!(
        parsed["mcpServers"]["other"]["command"], "npx",
        "foreign sidecar servers must be left untouched"
    );

    let generation = fs::read_to_string(generation_path(home.path())).unwrap();
    let value: u64 = generation.trim().parse().expect("generation is a counter");
    assert!(
        value >= 1,
        "real run must bump generation, got {generation:?}"
    );
}

#[test]
fn mcp_refresh_leaves_foreign_anvil_entry_untouched() {
    let home = tempfile::tempdir().unwrap();
    let path = cursor_config(home.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let before = format!(
        "{}\n",
        serde_json::to_string_pretty(&json!({
            "mcpServers": {
                "anvil": {
                    "command": "python",
                    "args": ["foreign.py"]
                }
            }
        }))
        .unwrap()
    );
    fs::write(&path, &before).unwrap();

    let output = run_refresh(
        home.path(),
        &[
            "--clients",
            "cursor",
            "--daemon",
            "reuse",
            "--processes",
            "none",
        ],
    );
    assert_success(&output);
    assert_eq!(fs::read_to_string(&path).unwrap(), before);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("foreign")
            || combined.contains("user-owned")
            || combined.contains("skip"),
        "foreign entries should be reported as skipped: {combined}"
    );
}

#[test]
fn mcp_refresh_json_includes_config_daemon_and_processes() {
    let home = tempfile::tempdir().unwrap();
    write_cursor_anvil(home.path(), "anvil", None);

    let output = run_refresh_json(
        home.path(),
        &[
            "--clients",
            "cursor",
            "--daemon",
            "reuse",
            "--processes",
            "report",
        ],
    );
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!("expected JSON report, got {stdout:?}: {err}");
    });
    assert!(parsed.get("config").is_some(), "missing config: {parsed}");
    assert!(parsed.get("daemon").is_some(), "missing daemon: {parsed}");
    assert!(
        parsed.get("processes").is_some(),
        "missing processes: {parsed}"
    );
    assert_eq!(
        parsed["processes"]["signalled"].as_u64().unwrap_or(99),
        0,
        "report mode must never signal: {parsed}"
    );
}

#[test]
fn mcp_refresh_dry_run_orphan_reap_lists_without_signalling() {
    let home = tempfile::tempdir().unwrap();
    write_cursor_anvil(home.path(), "anvil", None);
    fs::write(generation_path(home.path()), "2\n").unwrap();
    let before = fs::read_to_string(cursor_config(home.path())).unwrap();

    let output = run_refresh_json(
        home.path(),
        &[
            "--dry-run",
            "--clients",
            "cursor",
            "--daemon",
            "reuse",
            "--processes",
            "orphan-reap",
        ],
    );
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!("expected JSON report, got {stdout:?}: {err}");
    });
    assert_eq!(
        parsed["processes"]["mode"].as_str().unwrap_or(""),
        "orphan-reap"
    );
    assert_eq!(
        parsed["processes"]["signalled"].as_u64().unwrap_or(99),
        0,
        "dry-run orphan-reap must not signal: {parsed}"
    );
    assert!(
        parsed["processes"].get("orphan").is_some(),
        "dry-run must still list orphan counts: {parsed}"
    );
    assert_eq!(
        fs::read_to_string(cursor_config(home.path())).unwrap(),
        before,
        "dry-run must not rewrite configs"
    );
    assert_eq!(
        fs::read_to_string(generation_path(home.path())).unwrap(),
        "2\n",
        "dry-run must not bump generation"
    );
}

#[test]
fn mcp_refresh_rejects_force_skewed() {
    let home = tempfile::tempdir().unwrap();
    write_cursor_anvil(home.path(), "anvil", None);
    fs::write(generation_path(home.path()), "2\n").unwrap();
    let before = fs::read_to_string(cursor_config(home.path())).unwrap();

    let output = run_refresh(
        home.path(),
        &["--clients", "cursor", "--processes", "force-skewed"],
    );
    assert!(
        !output.status.success(),
        "force-skewed must stay rejected\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("force-skewed")
            && (stderr.contains("not offered") || stderr.contains("forbidden")),
        "force-skewed error must say it is not offered: {stderr}"
    );
    assert_eq!(
        fs::read_to_string(cursor_config(home.path())).unwrap(),
        before
    );
    assert_eq!(
        fs::read_to_string(generation_path(home.path())).unwrap(),
        "2\n",
        "rejected process mode must not bump generation"
    );
}
