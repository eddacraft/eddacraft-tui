//! Issue #3915: `--json` must put exactly one JSON document on stdout.
//!
//! Three successful surfaces advertised a JSON mode and then printed human
//! prose: `config show`, `gctx egress enable` / `disable`, and
//! `capsule create`. Automation that treats successful stdout as a JSON
//! document broke on all three. Issue #3938 extended the same contract to
//! the two remaining `config` mutation verbs, `config set` and
//! `config convert`.
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

    /// An `anvil` invocation in the sandbox repo with operator state rerooted
    /// under the sandbox `HOME`, so no test touches the developer's real
    /// consent store.
    fn command(&self, args: &[&str]) -> Command {
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
        cmd
    }

    fn anvil(&self, args: &[&str]) -> Output {
        self.command(args)
            .output()
            .expect("failed to invoke anvil binary")
    }

    /// Same, but with the process-scoped `ANVIL_GCTX_EGRESS` override set —
    /// the kill-switch the egress docs promise stays visible in JSON.
    fn anvil_with_egress_env(&self, args: &[&str], value: &str) -> Output {
        self.command(args)
            .env("ANVIL_GCTX_EGRESS", value)
            .output()
            .expect("failed to invoke anvil binary")
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

// ── config set (#3938) ─────────────────────────────────────────────

#[test]
fn config_set_json_emits_only_json_for_both_flag_placements() {
    for args in [
        [
            "config",
            "set",
            "cross-layer-violation",
            "enforce",
            "--json",
        ]
        .as_slice(),
        [
            "--json",
            "config",
            "set",
            "cross-layer-violation",
            "enforce",
        ]
        .as_slice(),
    ] {
        let sandbox = Sandbox::new();
        let out = sandbox.anvil(args);
        let doc = parse_only_json(&out, &format!("anvil {}", args.join(" ")));

        assert_eq!(
            doc.get("rule").and_then(serde_json::Value::as_str),
            Some("cross-layer-violation"),
            "rule must name what was set: {doc}"
        );
        assert_eq!(
            doc.get("mode").and_then(serde_json::Value::as_str),
            Some("enforce"),
            "mode must carry the written value: {doc}"
        );
        let config = doc
            .get("config")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("config must name the written file: {doc}"));
        assert!(
            config.ends_with(".anvil.yaml"),
            "config must be the canonical file the write created: {config}"
        );
        assert!(
            sandbox.repo().join(".anvil.yaml").is_file(),
            "the config write must still happen under --json"
        );
    }
}

#[test]
fn config_set_human_output_is_unchanged() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["config", "set", "cross-layer-violation", "warn"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "config set must exit 0: {stdout}");
    assert_eq!(
        stdout, "set cross-layer-violation=warn\n",
        "human output must stay the single `set rule=mode` line"
    );
}

// ── config convert (#3938) ─────────────────────────────────────────

/// A canonical `snake_case` project config with no legacy keys, so convert
/// tests exercise the happy path without the deprecation side-channel.
fn write_json_config(sandbox: &Sandbox) {
    std::fs::write(
        sandbox.repo().join(".anvil.json"),
        r#"{"enforcement":{"rules":{"cross-layer-violation":{"mode":"enforce"}}}}"#,
    )
    .expect("write .anvil.json");
}

#[test]
fn config_convert_json_emits_only_json_for_both_flag_placements() {
    for args in [
        ["config", "convert", "--to", "yaml", "--json"].as_slice(),
        ["--json", "config", "convert", "--to", "yaml"].as_slice(),
    ] {
        let sandbox = Sandbox::new();
        write_json_config(&sandbox);
        let out = sandbox.anvil(args);
        let doc = parse_only_json(&out, &format!("anvil {}", args.join(" ")));

        let source = doc
            .get("source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("source must be present: {doc}"));
        assert!(
            source.ends_with(".anvil.json"),
            "source must name the discovered config: {source}"
        );
        let destination = doc
            .get("destination")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("destination must be present: {doc}"));
        assert!(
            destination.ends_with(".anvil.yaml"),
            "destination must name the written file: {destination}"
        );
        assert_eq!(
            doc.get("source_removed"),
            Some(&serde_json::Value::Bool(false)),
            "source_removed must be false when the source is kept: {doc}"
        );
        assert!(
            sandbox.repo().join(".anvil.yaml").is_file(),
            "the destination must still be written under --json"
        );
        assert!(
            sandbox.repo().join(".anvil.json").is_file(),
            "the source must still be kept without --remove-old"
        );
    }
}

#[test]
fn config_convert_json_reports_source_removal() {
    let sandbox = Sandbox::new();
    write_json_config(&sandbox);
    let out = sandbox.anvil(&[
        "config",
        "convert",
        "--to",
        "yaml",
        "--remove-old",
        "--json",
    ]);
    let doc = parse_only_json(&out, "anvil config convert --to yaml --remove-old --json");
    assert_eq!(
        doc.get("source_removed"),
        Some(&serde_json::Value::Bool(true)),
        "source_removed must be true under --remove-old: {doc}"
    );
    assert!(
        !sandbox.repo().join(".anvil.json").exists(),
        "the source must actually be removed"
    );
}

#[test]
fn config_convert_stdout_json_wraps_converted_text_in_an_envelope() {
    // `--stdout` prints the config in the *target* format, so under `--json`
    // the raw text would violate the one-JSON-document contract for every
    // non-JSON target. The converted text moves into a field instead.
    let sandbox = Sandbox::new();
    write_json_config(&sandbox);
    let out = sandbox.anvil(&["config", "convert", "--to", "toml", "--stdout", "--json"]);
    let doc = parse_only_json(&out, "anvil config convert --to toml --stdout --json");

    assert_eq!(
        doc.get("format").and_then(serde_json::Value::as_str),
        Some("toml"),
        "format must name the destination format: {doc}"
    );
    let converted = doc
        .get("converted")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("converted must carry the config text: {doc}"));
    assert!(
        converted.contains("cross-layer-violation"),
        "converted must be the serialised config: {converted}"
    );
    assert!(
        !sandbox.repo().join(".anvil.toml").exists(),
        "--stdout must still not write a destination file"
    );
}

#[test]
fn config_convert_human_output_is_unchanged() {
    let sandbox = Sandbox::new();
    write_json_config(&sandbox);

    let out = sandbox.anvil(&["config", "convert", "--to", "toml", "--stdout"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "convert --stdout must exit 0: {stdout}"
    );
    assert!(
        stdout.contains("cross-layer-violation") && !stdout.trim_start().starts_with('{'),
        "--stdout without --json must stay raw target-format text: {stdout}"
    );

    let out = sandbox.anvil(&["config", "convert", "--to", "yaml"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "convert must exit 0: {stdout}");
    assert!(
        stdout.starts_with("anvil: converted ")
            && stdout.contains(".anvil.json")
            && stdout.contains(".anvil.yaml")
            && stdout.contains("source kept; pass --remove-old to delete"),
        "human convert output must be unchanged: {stdout}"
    );
}

// ── migrate format (#3946) ─────────────────────────────────────────

#[test]
fn migrate_format_json_emits_only_json_for_both_flag_placements() {
    // Shares the convert writer #3943 fixed for `config convert`, so the
    // document is the same write-mode shape.
    for args in [
        ["migrate", "format", "--format", "yaml", "--json"].as_slice(),
        ["--json", "migrate", "format", "--format", "yaml"].as_slice(),
    ] {
        let sandbox = Sandbox::new();
        write_json_config(&sandbox);
        let out = sandbox.anvil(args);
        let doc = parse_only_json(&out, &format!("anvil {}", args.join(" ")));

        let source = doc
            .get("source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("source must be present: {doc}"));
        assert!(
            source.ends_with(".anvil.json"),
            "source must name the discovered config: {source}"
        );
        let destination = doc
            .get("destination")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("destination must be present: {doc}"));
        assert!(
            destination.ends_with(".anvil.yaml"),
            "destination must name the written file: {destination}"
        );
        assert_eq!(
            doc.get("source_removed"),
            Some(&serde_json::Value::Bool(false)),
            "source_removed must be false when the source is kept: {doc}"
        );
        assert!(
            sandbox.repo().join(".anvil.yaml").is_file(),
            "the destination must still be written under --json"
        );
    }
}

#[test]
fn migrate_format_json_reports_source_removal() {
    let sandbox = Sandbox::new();
    write_json_config(&sandbox);
    let out = sandbox.anvil(&[
        "migrate",
        "format",
        "--format",
        "yaml",
        "--remove-old",
        "--json",
    ]);
    let doc = parse_only_json(
        &out,
        "anvil migrate format --format yaml --remove-old --json",
    );
    assert_eq!(
        doc.get("source_removed"),
        Some(&serde_json::Value::Bool(true)),
        "source_removed must be true under --remove-old: {doc}"
    );
    assert!(
        !sandbox.repo().join(".anvil.json").exists(),
        "the source must actually be removed"
    );
}

#[test]
fn bare_migrate_json_routes_to_format_and_keeps_stdout_json_only() {
    // The deprecated bare `anvil migrate` routes to `format`; its
    // deprecation notice is stderr, so an accepted `--json` still means
    // stdout is exactly one document.
    let sandbox = Sandbox::new();
    write_json_config(&sandbox);
    let out = sandbox.anvil(&["migrate", "--json"]);
    let doc = parse_only_json(&out, "anvil migrate --json (bare back-compat route)");
    assert!(
        doc.get("source").is_some() && doc.get("destination").is_some(),
        "bare migrate must emit the same write-mode document: {doc}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("now has subcommands"),
        "the deprecation notice must stay on stderr: {stderr}"
    );
}

#[test]
fn migrate_format_human_output_is_unchanged() {
    let sandbox = Sandbox::new();
    write_json_config(&sandbox);
    let out = sandbox.anvil(&["migrate", "format", "--format", "yaml"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "migrate format must exit 0: {stdout}");
    assert!(
        stdout.starts_with("anvil: converted ")
            && stdout.contains(".anvil.json")
            && stdout.contains(".anvil.yaml")
            && stdout.contains("source kept; pass --remove-old to delete"),
        "human migrate format output must be unchanged: {stdout}"
    );
}

// ── gctx egress enable / disable ───────────────────────────────────

#[test]
fn gctx_egress_enable_and_disable_json_emit_only_json() {
    // Both verbs at both flag placements: the leading global form and the
    // trailing per-command form are equally documented, so a consumer must
    // not have to know which one this binary happens to honour.
    for leading in [false, true] {
        let sandbox = Sandbox::new();

        let out = sandbox.anvil(&egress_args(
            leading,
            &["enable", "--yes", "--touch-project-state"],
        ));
        let doc = parse_only_json(&out, "anvil gctx egress enable (json)");
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

        // The document `status --json` already emits stays the shape enable
        // and disable report, so one parser handles the whole verb family.
        let status = parse_only_json(
            &sandbox.anvil(&["gctx", "egress", "status", "--json"]),
            "anvil gctx egress status --json",
        );
        assert_eq!(status.get("egress"), doc.get("egress"));
        assert_eq!(status.get("source"), doc.get("source"));

        let out = sandbox.anvil(&egress_args(leading, &["disable", "--touch-project-state"]));
        let doc = parse_only_json(&out, "anvil gctx egress disable (json)");
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
}

/// `anvil --json gctx egress <verb>` when `leading`, else
/// `anvil gctx egress <verb> … --json`.
fn egress_args<'a>(leading: bool, verb: &[&'a str]) -> Vec<&'a str> {
    let mut args = if leading { vec!["--json"] } else { Vec::new() };
    args.extend_from_slice(&["gctx", "egress"]);
    args.extend_from_slice(verb);
    if !leading {
        args.push("--json");
    }
    args
}

#[test]
fn gctx_egress_json_reports_the_effective_state_under_the_kill_switch() {
    // Consent is recorded, but `ANVIL_GCTX_EGRESS=0` still suppresses egress
    // for this process. Both docs promise the document reports the effective
    // state, so a consent audit cannot be misled into reading `enabled`.
    let sandbox = Sandbox::new();
    let out = sandbox.anvil_with_egress_env(&["gctx", "egress", "enable", "--yes", "--json"], "0");
    let doc = parse_only_json(&out, "anvil gctx egress enable --json (kill-switch)");
    assert_eq!(
        doc.get("egress").and_then(serde_json::Value::as_str),
        Some("identity-only"),
        "the kill-switch must win over freshly recorded consent: {doc}"
    );
    assert_eq!(
        doc.get("source").and_then(serde_json::Value::as_str),
        Some("env"),
        "the override must be named as the source: {doc}"
    );
    assert_eq!(
        doc.get("action").and_then(serde_json::Value::as_str),
        Some("enabled"),
        "the consent write still happened and must be reported: {doc}"
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

// ── CLI-wide sweep (#3947) ─────────────────────────────────────────
//
// Operator decision A on #3947: `--json` binds CLI-wide. These tests
// cover the surfaces that used to accept the flag and print prose; the
// clap-tree registry test in `src/json_surface_audit.rs` forces every
// NEW command to classify itself, and these pin the honoured behaviour
// for the repaired ones. All parse the whole of stdout.

#[test]
fn telemetry_on_off_json_emit_only_json_for_both_flag_placements() {
    for args in [
        ["telemetry", "on", "--json"].as_slice(),
        ["--json", "telemetry", "on"].as_slice(),
    ] {
        let sandbox = Sandbox::new();
        let out = sandbox.anvil(args);
        let doc = parse_only_json(&out, &format!("anvil {}", args.join(" ")));
        assert_eq!(
            doc.get("telemetry").and_then(serde_json::Value::as_str),
            Some("on"),
            "telemetry on must report the new state: {doc}"
        );
        assert!(
            doc.get("install_id")
                .and_then(serde_json::Value::as_str)
                .is_some(),
            "telemetry on must carry the minted install id: {doc}"
        );
        assert!(
            doc.get("disclosure")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|text| !text.is_empty()),
            "the disclosure text must ride inside the document: {doc}"
        );

        let out = sandbox.anvil(&["telemetry", "off", "--json"]);
        let doc = parse_only_json(&out, "anvil telemetry off --json");
        assert_eq!(
            doc.get("telemetry").and_then(serde_json::Value::as_str),
            Some("off"),
            "telemetry off must report the new state: {doc}"
        );
    }
}

#[test]
fn telemetry_reset_id_json_emits_only_json() {
    let sandbox = Sandbox::new();
    sandbox.anvil(&["telemetry", "on"]);
    let out = sandbox.anvil(&["telemetry", "reset-id", "--json"]);
    let doc = parse_only_json(&out, "anvil telemetry reset-id --json");
    assert_eq!(
        doc.get("telemetry_id").and_then(serde_json::Value::as_str),
        Some("rotated"),
        "reset-id must report the rotation: {doc}"
    );
}

#[test]
fn telemetry_human_output_is_unchanged() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["telemetry", "off"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "telemetry off must exit 0: {stdout}");
    assert_eq!(
        stdout, "Telemetry is off. No beacon will be sent.\n",
        "human telemetry off line must be unchanged"
    );
}

#[test]
fn migrate_schema_json_emits_one_document_per_outcome() {
    // No `anvil/project-id` in the sandbox, so the origin is unknown —
    // that outcome must be a document, not the historical prose.
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["migrate", "schema", "--json"]);
    let doc = parse_only_json(&out, "anvil migrate schema --json");
    assert_eq!(
        doc.get("outcome").and_then(serde_json::Value::as_str),
        Some("unknown-origin"),
        "schema migrate must name its outcome: {doc}"
    );
}

#[test]
fn migrate_architecture_json_reports_the_dry_run_plan() {
    let sandbox = Sandbox::new();
    write_json_config(&sandbox);
    std::fs::create_dir_all(sandbox.repo().join(".anvil")).expect("mkdir .anvil");
    std::fs::write(
        sandbox.repo().join(".anvil/architecture.yaml"),
        "layers: []\n",
    )
    .expect("write architecture.yaml");

    let out = sandbox.anvil(&["migrate", "architecture", "--json"]);
    let doc = parse_only_json(&out, "anvil migrate architecture --json");
    assert_eq!(
        doc.get("outcome").and_then(serde_json::Value::as_str),
        Some("dry-run"),
        "dry-run must be the named outcome: {doc}"
    );
    assert_eq!(
        doc.get("source").and_then(serde_json::Value::as_str),
        Some(".anvil/architecture.yaml"),
        "the planned source line must be a field: {doc}"
    );
}

#[test]
fn migrate_schema_human_output_is_unchanged() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["migrate", "schema"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "migrate schema must exit 0: {stdout}");
    assert!(
        stdout.starts_with("anvil: cannot determine the anvil version"),
        "human unknown-origin prose must be unchanged: {stdout}"
    );
}

#[test]
fn workspace_allow_and_deny_json_emit_only_json_for_both_flag_placements() {
    for args in [
        ["workspace", "allow", "/srv/proj", "--json"].as_slice(),
        ["--json", "workspace", "allow", "/srv/proj"].as_slice(),
    ] {
        let sandbox = Sandbox::new();
        let out = sandbox.anvil(args);
        let doc = parse_only_json(&out, &format!("anvil {}", args.join(" ")));
        assert!(
            doc.get("allowed")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|path| path.ends_with("proj")),
            "allow must name the stored path: {doc}"
        );
        assert_eq!(
            doc.get("kind").and_then(serde_json::Value::as_str),
            Some("exact"),
            "allow must report the match kind: {doc}"
        );

        let out = sandbox.anvil(&["workspace", "deny", "/srv/proj", "--json"]);
        let doc = parse_only_json(&out, "anvil workspace deny --json");
        assert_eq!(
            doc.get("removed"),
            Some(&serde_json::Value::Bool(true)),
            "deny must report the removal: {doc}"
        );
    }
}

#[test]
fn workspace_mode_json_emits_only_json() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["workspace", "mode", "allowlist", "--json"]);
    let doc = parse_only_json(&out, "anvil workspace mode allowlist --json");
    assert_eq!(
        doc.get("admission_mode")
            .and_then(serde_json::Value::as_str),
        Some("allowlist"),
        "mode must report the new admission mode: {doc}"
    );
}

#[test]
fn workspace_allow_human_output_is_unchanged() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["workspace", "allow", "/srv/proj"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "workspace allow must exit 0: {stdout}"
    );
    assert!(
        stdout.starts_with("Allowed ") && stdout.contains("(exact)"),
        "human allow output must be unchanged: {stdout}"
    );
}

#[test]
fn licenses_json_wraps_the_text_in_one_document() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["licenses", "--json"]);
    let doc = parse_only_json(&out, "anvil licenses --json");
    assert_eq!(
        doc.get("format").and_then(serde_json::Value::as_str),
        Some("plain"),
        "licenses must name the rendered format: {doc}"
    );
    assert!(
        doc.get("text")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|text| text.contains("ACKNOWLEDGEMENTS")
                || text.contains("licence")
                || text.contains("License")),
        "the licence text must travel inside the document: {doc}"
    );
}

#[test]
fn gate_list_profiles_json_emits_only_json_for_both_flag_placements() {
    for args in [
        ["gate", "--list-profiles", "--json"].as_slice(),
        ["--json", "gate", "--list-profiles"].as_slice(),
    ] {
        let sandbox = Sandbox::new();
        let out = sandbox.anvil(args);
        let doc = parse_only_json(&out, &format!("anvil {}", args.join(" ")));
        let profiles = doc
            .get("profiles")
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("profiles must be an array: {doc}"));
        assert!(
            profiles
                .iter()
                .any(|p| p.get("name").and_then(serde_json::Value::as_str) == Some("ai")),
            "the ai profile must be listed: {doc}"
        );
    }
}

#[test]
fn new_scaffold_json_emits_only_json() {
    let sandbox = Sandbox::new();
    let listing = parse_only_json(
        &sandbox.anvil(&["new", "--list", "--json"]),
        "anvil new --list --json",
    );
    let template_id = listing
        .get("templates")
        .and_then(serde_json::Value::as_array)
        .and_then(|templates| templates.first())
        .and_then(|t| t.get("id"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("template listing must expose ids: {listing}"))
        .to_owned();

    let out = sandbox.anvil(&["new", &template_id, "--json"]);
    let doc = parse_only_json(&out, &format!("anvil new {template_id} --json"));
    assert_eq!(
        doc.get("template").and_then(serde_json::Value::as_str),
        Some(template_id.as_str()),
        "the scaffold document must name the template: {doc}"
    );
    let output = doc
        .get("output")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("output must name the written file: {doc}"));
    assert!(
        sandbox.repo().join(output).is_file() || Path::new(output).is_file(),
        "the scaffold must still be written under --json: {output}"
    );
}

#[test]
fn tutorial_reset_json_emits_only_json() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["tutorial", "--reset", "--json"]);
    let doc = parse_only_json(&out, "anvil tutorial --reset --json");
    assert_eq!(
        doc.get("reset"),
        Some(&serde_json::Value::Bool(true)),
        "reset must be reported: {doc}"
    );
}

#[test]
fn tutorial_json_refuses_with_empty_stdout() {
    // Interactive-only surface: under `--json` the refusal is a
    // structured error (stderr envelope from main), never a TUI or
    // prose on stdout.
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["tutorial", "--json"]);
    assert!(!out.status.success(), "tutorial --json must refuse");
    assert!(
        out.stdout.is_empty(),
        "refusal must leave stdout empty: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn welcome_json_emits_only_json_with_and_without_skip() {
    let sandbox = Sandbox::new();
    // The sandbox exports ANVIL_SKIP_WELCOME=1, so the bypass branch runs.
    let doc = parse_only_json(
        &sandbox.anvil(&["welcome", "--json"]),
        "anvil welcome --json (skip env)",
    );
    assert_eq!(
        doc.get("skipped"),
        Some(&serde_json::Value::Bool(true)),
        "the env bypass must be reported: {doc}"
    );

    let out = sandbox
        .command(&["welcome", "--json"])
        .env_remove("ANVIL_SKIP_WELCOME")
        .output()
        .expect("invoke anvil welcome");
    let doc = parse_only_json(&out, "anvil welcome --json");
    assert_eq!(
        doc.get("skipped"),
        Some(&serde_json::Value::Bool(false)),
        "the full welcome flow must report one document: {doc}"
    );
    assert!(
        doc.get("next_step")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|step| step.contains("anvil start")),
        "the next-step line must survive as a field: {doc}"
    );
}

#[test]
fn baseline_and_verify_json_emit_only_json() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["baseline", "--json"]);
    let doc = parse_only_json(&out, "anvil baseline --json");
    assert_eq!(
        doc.get("outcome").and_then(serde_json::Value::as_str),
        Some("created"),
        "baseline create must name its outcome: {doc}"
    );
    assert!(
        doc.get("findings")
            .and_then(serde_json::Value::as_u64)
            .is_some(),
        "the findings count must be a field: {doc}"
    );

    let out = sandbox.anvil(&["baseline", "verify", "--json"]);
    let doc = parse_only_json(&out, "anvil baseline verify --json");
    assert_eq!(
        doc.get("outcome").and_then(serde_json::Value::as_str),
        Some("ok"),
        "verify must report ok: {doc}"
    );
}

#[test]
fn capsule_prune_json_emits_one_document_when_empty() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["capsule", "prune", "--keep-last", "1", "--json"]);
    let doc = parse_only_json(&out, "anvil capsule prune --json (empty)");
    assert_eq!(
        doc.get("dry_run"),
        Some(&serde_json::Value::Bool(true)),
        "prune without --apply must report a dry run: {doc}"
    );
    assert!(
        doc.get("capsules")
            .and_then(serde_json::Value::as_array)
            .is_some_and(std::vec::Vec::is_empty),
        "an empty staging root must yield an empty capsule list: {doc}"
    );
}

#[test]
fn hook_bootstrap_dry_run_json_emits_only_json() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["hook", "bootstrap", "--dry-run", "--json"]);
    let doc = parse_only_json(&out, "anvil hook bootstrap --dry-run --json");
    assert_eq!(
        doc.get("dry_run"),
        Some(&serde_json::Value::Bool(true)),
        "the dry run must be reported: {doc}"
    );
    assert!(
        doc.get("plan")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "the plan kind must be a field: {doc}"
    );
}

// NOTE: `intercept stop --json` has no process test here: the per-user
// daemon PID file is discovered outside the sandbox `HOME` reroot, so a
// test invocation signals the developer's real intercept daemon. The
// JSON projection of `run_stop` shares `registration_fields`-style
// mapping and is covered by review; do not add a process test without
// a daemon-path override.

#[test]
fn edda_list_json_is_identical_for_both_flag_placements() {
    // The missing-storage envelope exits 1 by design; the contract here
    // is placement parity — the leading global flag must produce the
    // same stdout document as the trailing one (the clap propagation
    // gap this sweep fixed).
    let sandbox = Sandbox::new();
    let leading = sandbox.anvil(&["--json", "edda", "list"]);
    let trailing = sandbox.anvil(&["edda", "list", "--json"]);
    assert_eq!(
        String::from_utf8_lossy(&leading.stdout),
        String::from_utf8_lossy(&trailing.stdout),
        "both placements must produce the same stdout"
    );
    let stdout = String::from_utf8_lossy(&leading.stdout);
    serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or_else(|err| {
        panic!("edda list --json stdout must be one JSON document: {err}\n{stdout}")
    });
}

#[test]
fn mcp_config_toml_preview_json_wraps_config_in_one_document() {
    let sandbox = Sandbox::new();
    let out = sandbox.anvil(&["mcp-config", "--target", "codex", "--json"]);
    let doc = parse_only_json(&out, "anvil mcp-config --target codex --json");
    assert_eq!(
        doc.get("format").and_then(serde_json::Value::as_str),
        Some("toml"),
        "the TOML preview must name its format: {doc}"
    );
    assert!(
        doc.get("config")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|config| config.contains("anvil")),
        "the rendered config must travel inside the document: {doc}"
    );
}
