//! Issue #3915: `--json` must put exactly one JSON document on stdout.
//!
//! Three successful surfaces advertised a JSON mode and then printed human
//! prose: `config show`, `gctx egress enable` / `disable`, and
//! `capsule create`. Automation that treats successful stdout as a JSON
//! document broke on all three.
//!
//! Every assertion here parses the **whole** of stdout with
//! `serde_json::from_str`, which is what rejects leading or trailing prose —
//! a trailing `verify with: …` line makes the parse fail, so the contract is
//! enforced by construction rather than by substring matching. Both accepted
//! flag placements are covered (`anvil --json <cmd>` and
//! `anvil <cmd> --json`) because clap propagates the global flag and users
//! reach for either.
//!
//! The human (no `--json`) form is asserted alongside each surface so a
//! future JSON change cannot quietly reshape the operator-facing output.
#![cfg(not(target_os = "windows"))]

use std::path::Path;
use std::process::{Command, Output};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// A git repository with two commits, so `capsule create` has a range to
/// package, plus a rerooted `HOME` for operator-owned state.
struct Sandbox {
    repo: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let repo = tempfile::tempdir().expect("temp repo");
        let home = tempfile::tempdir().expect("temp home");
        let sandbox = Self { repo, home };
        sandbox.git(&["init", "--initial-branch=main", "."]);
        sandbox.git(&["config", "user.email", "test@example.com"]);
        sandbox.git(&["config", "user.name", "Test"]);
        std::fs::write(sandbox.repo().join("a.txt"), "one\n").expect("write a.txt");
        sandbox.git(&["add", "-A"]);
        sandbox.git(&["commit", "-m", "first"]);
        std::fs::write(sandbox.repo().join("b.txt"), "two\n").expect("write b.txt");
        sandbox.git(&["add", "-A"]);
        sandbox.git(&["commit", "-m", "second"]);
        sandbox
    }

    fn repo(&self) -> &Path {
        self.repo.path()
    }

    fn git(&self, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(self.repo())
            .env("HOME", self.home.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("failed to invoke git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Run `anvil` in the sandbox repo with operator state rerooted under the
    /// sandbox `HOME`, so no test touches the developer's real consent store.
    fn anvil(&self, args: &[&str]) -> Output {
        let mut cmd = Command::new(ANVIL_BIN);
        cmd.arg("--no-tui")
            .args(args)
            .current_dir(self.repo())
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("NONINTERACTIVE", "1")
            .env("ANVIL_DEV", "1")
            .env("ANVIL_SKIP_WELCOME", "1")
            .env_remove("ANVIL_GCTX_EGRESS")
            .env_remove("ANVIL_HOME")
            .env_remove("XDG_STATE_HOME")
            .env_remove("XDG_CONFIG_HOME");
        cmd.output().expect("failed to invoke anvil binary")
    }
}

/// Assert the command succeeded and stdout is exactly one JSON document —
/// no leading banner, no trailing hint line.
fn parse_only_json(out: &Output, what: &str) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{what} must exit 0: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!("{what} stdout was not a single JSON document: {err}\n--- stdout ---\n{stdout}")
    })
}

// ── config show ────────────────────────────────────────────────────

#[test]
fn config_show_json_emits_only_json_for_both_flag_placements() {
    let sandbox = Sandbox::new();

    for args in [
        ["config", "show", "--json"].as_slice(),
        ["--json", "config", "show"].as_slice(),
    ] {
        let out = sandbox.anvil(args);
        let doc = parse_only_json(&out, &format!("anvil {}", args.join(" ")));

        assert_eq!(
            doc.get("config").and_then(serde_json::Value::as_str),
            Some("defaults"),
            "config label must carry the source the prose reported: {doc}"
        );
        let modes = doc
            .get("rule_modes")
            .and_then(serde_json::Value::as_object)
            .unwrap_or_else(|| panic!("rule_modes must be an object: {doc}"));
        for rule in [
            "public-api-expansion",
            "new-dependency-introduction",
            "cross-layer-violation",
            "privilege-expansion",
        ] {
            assert_eq!(
                modes.get(rule).and_then(serde_json::Value::as_str),
                Some("warn"),
                "rule_modes.{rule} must be present: {doc}"
            );
        }
        assert!(
            doc.get("note").is_some_and(serde_json::Value::is_null),
            "note must be present and null when no deprecation applies: {doc}"
        );
    }
}

#[test]
fn config_show_json_reports_the_config_path_and_legacy_note() {
    let sandbox = Sandbox::new();
    // A legacy camelCase key is what makes `config show` add its `note:`
    // line, so the JSON document must carry the same warning.
    std::fs::write(
        sandbox.repo().join(".anvil.json"),
        r#"{"enforcement":{"rules":{"public-api-expansion":{"mode":"enforce"}}},"schemaVersion":1}"#,
    )
    .expect("write .anvil.json");

    let out = sandbox.anvil(&["config", "show", "--json"]);
    let doc = parse_only_json(&out, "anvil config show --json (legacy keys)");

    assert_eq!(
        doc.get("config").and_then(serde_json::Value::as_str),
        Some(".anvil.json"),
        "config must name the discovered file: {doc}"
    );
    assert_eq!(
        doc.pointer("/rule_modes/public-api-expansion")
            .and_then(serde_json::Value::as_str),
        Some("enforce"),
        "configured mode must survive into JSON: {doc}"
    );
    let note = doc
        .get("note")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("note must be a string when legacy keys exist: {doc}"));
    assert!(
        note.contains("schemaVersion"),
        "note must name the deprecated key: {note}"
    );
}

#[test]
fn config_show_human_output_is_unchanged() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["config", "show"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "config show must exit 0: {stdout}");
    assert!(
        stdout.starts_with("config: defaults\n"),
        "human output must keep the `config:` line: {stdout}"
    );
    assert!(
        stdout.contains("rule modes: public-api-expansion=warn"),
        "human output must keep the `rule modes:` line: {stdout}"
    );
}

// ── gctx egress enable / disable ───────────────────────────────────

#[test]
fn gctx_egress_enable_and_disable_json_emit_only_json() {
    let sandbox = Sandbox::new();

    let out = sandbox.anvil(&[
        "gctx",
        "egress",
        "enable",
        "--yes",
        "--touch-project-state",
        "--json",
    ]);
    let doc = parse_only_json(&out, "anvil gctx egress enable --json");
    assert_eq!(
        doc.get("egress").and_then(serde_json::Value::as_str),
        Some("enabled"),
        "enable must report the resulting state: {doc}"
    );
    assert_eq!(
        doc.get("source").and_then(serde_json::Value::as_str),
        Some("config"),
        "enable must report where the state now comes from: {doc}"
    );
    assert_eq!(
        doc.get("action").and_then(serde_json::Value::as_str),
        Some("enabled"),
        "enable must report the action it performed: {doc}"
    );

    // The document `status --json` already emits stays the shape enable and
    // disable report, so one parser handles the whole verb family.
    let status = parse_only_json(
        &sandbox.anvil(&["gctx", "egress", "status", "--json"]),
        "anvil gctx egress status --json",
    );
    assert_eq!(status.get("egress"), doc.get("egress"));
    assert_eq!(status.get("source"), doc.get("source"));

    let out = sandbox.anvil(&[
        "--json",
        "gctx",
        "egress",
        "disable",
        "--touch-project-state",
    ]);
    let doc = parse_only_json(&out, "anvil --json gctx egress disable");
    assert_eq!(
        doc.get("egress").and_then(serde_json::Value::as_str),
        Some("identity-only"),
        "disable must report the reverted state: {doc}"
    );
    assert_eq!(
        doc.get("action").and_then(serde_json::Value::as_str),
        Some("disabled"),
        "disable must report the action it performed: {doc}"
    );
}

#[test]
fn gctx_egress_human_output_is_unchanged() {
    let sandbox = Sandbox::new();

    let out = sandbox.anvil(&["gctx", "egress", "enable", "--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "enable must exit 0: {stdout}");
    assert!(
        stdout.contains("Snippet egress enabled for this workspace."),
        "human enable output must be unchanged: {stdout}"
    );
    assert!(
        stdout.contains("anvil gctx egress disable"),
        "human enable output must keep the revoke hint: {stdout}"
    );

    let out = sandbox.anvil(&["gctx", "egress", "disable"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "disable must exit 0: {stdout}");
    assert!(
        stdout.contains("Snippet egress disabled for this workspace (identity-only)."),
        "human disable output must be unchanged: {stdout}"
    );
}

// ── capsule create ─────────────────────────────────────────────────

#[test]
fn capsule_create_json_emits_only_json_for_both_flag_placements() {
    let sandbox = Sandbox::new();

    for (index, placement) in [["--json"].as_slice(), [].as_slice()].iter().enumerate() {
        let out_dir = sandbox.home.path().join(format!("capsule-{index}"));
        let out_path = out_dir.to_string_lossy().into_owned();
        let mut args: Vec<&str> = placement.to_vec();
        args.extend_from_slice(&["capsule", "create", "--range", "HEAD~1..HEAD", "--out"]);
        args.push(&out_path);
        if placement.is_empty() {
            args.push("--json");
        }

        let out = sandbox.anvil(&args);
        let doc = parse_only_json(&out, &format!("anvil {}", args.join(" ")));

        assert_eq!(
            doc.get("schema").and_then(serde_json::Value::as_str),
            Some("anvil.capsule-create.v1"),
            "capsule documents are schema-stamped: {doc}"
        );
        assert_eq!(
            doc.get("out").and_then(serde_json::Value::as_str),
            Some(out_path.as_str()),
            "out must name the written capsule directory: {doc}"
        );
        assert_eq!(
            doc.get("commit_count").and_then(serde_json::Value::as_u64),
            Some(1),
            "commit_count must carry what the prose reported: {doc}"
        );
        assert!(
            doc.get("file_count")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|files| files > 1),
            "file_count must carry what the prose reported: {doc}"
        );
        let base = doc
            .pointer("/range/base")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("range.base must be present: {doc}"));
        let head = doc
            .pointer("/range/head")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("range.head must be present: {doc}"));
        assert_eq!(base.len(), 40, "range.base must be a full sha: {base}");
        assert_eq!(head.len(), 40, "range.head must be a full sha: {head}");
        assert_eq!(
            doc.get("verify_command")
                .and_then(serde_json::Value::as_str),
            Some(format!("anvil capsule verify {out_path}").as_str()),
            "the verify hint the prose printed must survive as a field: {doc}"
        );
        assert!(
            out_dir.join("manifest.json").is_file(),
            "the capsule must still be written"
        );
    }
}

#[test]
fn capsule_create_human_output_is_unchanged() {
    let sandbox = Sandbox::new();
    let out_dir = sandbox.home.path().join("capsule-human");
    let out_path = out_dir.to_string_lossy().into_owned();
    let out = sandbox.anvil(&[
        "capsule",
        "create",
        "--range",
        "HEAD~1..HEAD",
        "--out",
        &out_path,
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "capsule create must exit 0: {stdout}");
    assert!(
        stdout.starts_with(&format!("capsule written: {out_path} (1 commit ")),
        "human output must keep the `capsule written:` line: {stdout}"
    );
    assert!(
        stdout.contains(&format!("verify with: anvil capsule verify {out_path}")),
        "human output must keep the verify hint: {stdout}"
    );
}
