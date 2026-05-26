//! POLENG-007: `anvil policy eval` integration tests.
//!
//! Pin the CLI contract: evaluate a `.rego` policy against an optional
//! `PolicyInput`, emit a JSON report, and exit-code per ADR-002/003
//! (warnings exit 0 by default, `--fail-on-warnings` blocks).

use std::fs;
use std::path::Path;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

const FINDINGS_POLICY: &str = r#"package arch
import rego.v1

findings contains f if {
    some edge in input.diff.new_edges
    f := {
        "message": sprintf("new edge %s -> %s", [edge.from, edge.to]),
        "from": edge.from,
        "to": edge.to,
    }
}
"#;

fn write(dir: &Path, name: &str, contents: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, contents).expect("write fixture");
    path.display().to_string()
}

/// Run `anvil policy eval` with the given trailing args, returning the parsed
/// JSON stdout and the process exit code.
fn eval(dir: &Path, args: &[&str]) -> (serde_json::Value, i32) {
    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .args(["policy", "eval"])
        .args(args)
        .current_dir(dir)
        .env("HOME", dir)
        // `policy` is licence-gated; ANVIL_DEV=1 is the suite-wide local bypass.
        .env("ANVIL_DEV", "1")
        .output()
        .expect("invoke anvil");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("parse json stdout: {e}\n--- stdout ---\n{stdout}"));
    (value, output.status.code().unwrap_or(-1))
}

#[test]
fn eval_emits_raw_value_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "greet.rego",
        "package t\nimport rego.v1\ngreeting := \"hello world\"\n",
    );

    let (json, code) = eval(dir.path(), &[&policy, "--query", "data.t.greeting"]);
    assert_eq!(code, 0, "non-findings query should exit 0");
    assert_eq!(json["value"], "hello world");
    assert_eq!(json["exit_code"], 0);
    assert_eq!(json["findings"].as_array().unwrap().len(), 0);
}

#[test]
fn eval_emits_structured_debug_tracing() {
    // CIB-017: under debug logging the eval path emits a `policy_eval` span and
    // a "policy eval complete" event with structured fields, so a CI/prod
    // failure is diagnosable beyond an anyhow chain. Quiet by default (the CLI's
    // default filter is `warn`); operators opt in via ANVIL_LOG / RUST_LOG.
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "greet.rego",
        "package t\nimport rego.v1\ngreeting := \"hello world\"\n",
    );

    let output = Command::new(ANVIL_BIN)
        .args(["policy", "eval", &policy, "--query", "data.t.greeting"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("ANVIL_DEV", "1")
        // Hermetic: ignore an ambient RUST_LOG / file sink so the run uses the
        // ANVIL_LOG=debug filter and the default (stream) sink.
        .env_remove("RUST_LOG")
        .env_remove("ANVIL_TRACE_SINK")
        .env("ANVIL_LOG", "debug")
        .output()
        .expect("invoke anvil");
    assert!(output.status.success(), "eval should exit 0");
    // CIB-024: CLI diagnostics go to stderr. Search both streams so this stays
    // correct regardless of the stream the layer happens to use.
    let combined = format!(
        "{}{}",
        String::from_utf8(output.stdout).expect("utf8 stdout"),
        String::from_utf8(output.stderr).expect("utf8 stderr"),
    );

    let event = combined
        .lines()
        .find(|l| l.contains("policy eval complete"))
        .unwrap_or_else(|| panic!("no `policy eval complete` debug event:\n{combined}"));
    // Structured fields + the instrument span (name, policy, query) are present.
    for needle in [
        "policy_bytes",
        "input_bytes",
        "eval_ms",
        "\"findings\"",
        "exit_code",
        "policy_eval",
        "data.t.greeting",
    ] {
        assert!(
            event.contains(needle),
            "debug event missing `{needle}`:\n{event}"
        );
    }
}

#[test]
fn json_stdout_clean_when_warn_fires_at_default_filter() {
    // CIB-024: the motivating case is a `warn!` at the DEFAULT filter (no
    // ANVIL_LOG) — here a `--fail-on-warnings` failure on a non-findings query,
    // which fires the eval-failure `warn!` added in CIB-017. It must land on
    // stderr, not pollute `--json` stdout.
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "greet.rego",
        "package t\nimport rego.v1\ngreeting := \"hi\"\n",
    );

    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .args([
            "policy",
            "eval",
            &policy,
            "--query",
            "data.t.greeting",
            "--fail-on-warnings",
        ])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("ANVIL_DEV", "1")
        // Hermetic: force the DEFAULT `warn` filter and default sink by clearing
        // any ambient overrides — this test is specifically about that path.
        .env_remove("ANVIL_LOG")
        .env_remove("RUST_LOG")
        .env_remove("ANVIL_TRACE_SINK")
        .output()
        .expect("invoke anvil");
    assert!(
        !output.status.success(),
        "non-findings + --fail-on-warnings must fail"
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        !stdout.contains("policy eval failed"),
        "the warn! leaked onto stdout:\n{stdout}"
    );
    assert!(
        stderr.contains("policy eval failed"),
        "the warn! should be on stderr:\n{stderr}"
    );
}

#[test]
fn json_stdout_is_clean_under_debug_logging() {
    // CIB-024: with CLI logging on, diagnostics go to stderr, so `--json`
    // stdout stays exactly one parseable JSON document — no interleaved log
    // lines that would choke a `jq` / pipeline consumer.
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "greet.rego",
        "package t\nimport rego.v1\ngreeting := \"hello world\"\n",
    );

    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .args(["policy", "eval", &policy, "--query", "data.t.greeting"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("ANVIL_DEV", "1")
        // Hermetic: ignore an ambient RUST_LOG / file sink so logging uses the
        // ANVIL_LOG=debug filter and the default (stream) sink, not a file.
        .env_remove("RUST_LOG")
        .env_remove("ANVIL_TRACE_SINK")
        .env("ANVIL_LOG", "debug")
        .output()
        .expect("invoke anvil");
    assert!(output.status.success(), "eval should exit 0");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    // stdout is exactly the command's JSON — a single document, no log lines.
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "stdout not a single clean JSON doc under debug logging: {e}\n--- stdout ---\n{stdout}"
        )
    });
    assert_eq!(parsed["value"], "hello world");

    // The diagnostics went to stderr instead.
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("policy eval complete"),
        "expected the debug event on stderr:\n{stderr}"
    );
}

#[test]
fn findings_warn_but_do_not_block_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let policy = write(dir.path(), "arch.rego", FINDINGS_POLICY);
    let input = write(
        dir.path(),
        "input.json",
        r#"{ "diff": { "new_edges": [{ "from": "a.rs", "to": "b.rs" }] } }"#,
    );

    let (json, code) = eval(
        dir.path(),
        &[&policy, "--query", "data.arch.findings", "--input", &input],
    );
    assert_eq!(code, 0, "warnings must not block by default (ADR-002)");
    assert_eq!(json["exit_code"], 0);
    let findings = json["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["is_new_edge"], true);
}

#[test]
fn findings_block_under_fail_on_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let policy = write(dir.path(), "arch.rego", FINDINGS_POLICY);
    let input = write(
        dir.path(),
        "input.json",
        r#"{ "diff": { "new_edges": [{ "from": "a.rs", "to": "b.rs" }] } }"#,
    );

    let (json, code) = eval(
        dir.path(),
        &[
            &policy,
            "--query",
            "data.arch.findings",
            "--input",
            &input,
            "--fail-on-warnings",
        ],
    );
    assert_eq!(
        code, 1,
        "--fail-on-warnings must block non-baselined warnings"
    );
    assert_eq!(json["exit_code"], 1);
}

#[test]
fn non_findings_array_query_is_shown_as_raw_value() {
    // A query that legitimately returns an array of non-findings (e.g. a list)
    // must surface as the raw value, not error or misreport as findings.
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "list.rego",
        "package l\nimport rego.v1\nitems := [\"a\", \"b\"]\n",
    );

    let (json, code) = eval(dir.path(), &[&policy, "--query", "data.l.items"]);
    assert_eq!(code, 0);
    assert_eq!(json["value"], serde_json::json!(["a", "b"]));
    assert_eq!(json["findings"].as_array().unwrap().len(), 0);
}

#[test]
fn why_focuses_a_finding_and_includes_trace() {
    let dir = tempfile::tempdir().unwrap();
    let policy = write(dir.path(), "arch.rego", FINDINGS_POLICY);
    let input = write(
        dir.path(),
        "input.json",
        r#"{ "diff": { "new_edges": [{ "from": "a.rs", "to": "b.rs" }] } }"#,
    );

    let (json, code) = eval(
        dir.path(),
        &[
            &policy,
            "--query",
            "data.arch.findings",
            "--input",
            &input,
            "--why",
            "0",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["why"], 0);
    assert!(
        json["trace"].is_object(),
        "expected trace in output: {json}"
    );
}

#[test]
fn why_out_of_range_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "greet.rego",
        "package t\nimport rego.v1\ngreeting := \"hi\"\n",
    );
    // No findings from this query, so index 0 is out of range.
    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .args([
            "policy",
            "eval",
            &policy,
            "--query",
            "data.t.greeting",
            "--why",
            "0",
        ])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("ANVIL_DEV", "1")
        .output()
        .expect("invoke anvil");
    assert!(!output.status.success(), "out-of-range --why must fail");
}

#[test]
fn non_array_findings_query_errors_under_fail_on_warnings() {
    let dir = tempfile::tempdir().unwrap();
    // `findings := true` is a scalar, not a findings set — a policy bug.
    let policy = write(
        dir.path(),
        "bad.rego",
        "package arch\nimport rego.v1\nfindings := true\n",
    );
    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .args([
            "policy",
            "eval",
            &policy,
            "--query",
            "data.arch.findings",
            "--fail-on-warnings",
        ])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("ANVIL_DEV", "1")
        .output()
        .expect("invoke anvil");
    assert!(
        !output.status.success(),
        "a non-array result must not silently pass a gate"
    );
}

#[test]
fn explain_includes_coverage() {
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "greet.rego",
        "package t\nimport rego.v1\ngreeting := \"hi\"\n",
    );

    let (json, code) = eval(
        dir.path(),
        &[&policy, "--query", "data.t.greeting", "--explain"],
    );
    assert_eq!(code, 0);
    assert!(
        json["coverage"]["files"].is_array(),
        "expected coverage in output: {json}"
    );
}

/// Helper: invoke `policy eval` raw (no JSON parse) and return the exit code +
/// combined stderr, for cases expected to fail before producing JSON.
fn eval_status(dir: &Path, args: &[&str]) -> (i32, String) {
    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .args(["policy", "eval"])
        .args(args)
        .current_dir(dir)
        .env("HOME", dir)
        .env("ANVIL_DEV", "1")
        .output()
        .expect("invoke anvil");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn oversized_input_is_rejected() {
    // POLENG-009 resource bound: an input file over the cap is refused before
    // being read into memory, rather than risking an OOM.
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "greet.rego",
        "package t\nimport rego.v1\ngreeting := \"hi\"\n",
    );
    let big = dir.path().join("huge.json");
    fs::write(&big, vec![b'a'; (8 << 20) + 1024]).expect("write huge input");

    let (code, _err) = eval_status(
        dir.path(),
        &[
            &policy,
            "--query",
            "data.t.greeting",
            "--input",
            &big.display().to_string(),
        ],
    );
    assert!(code != 0, "oversized input must be rejected");
}

#[test]
fn malformed_findings_array_is_a_hard_error() {
    // POLENG-009 findings-parse: an array of objects missing the required
    // `message` is a broken policy, not a non-findings value — it must error,
    // never silently pass as exit 0.
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "bad.rego",
        "package arch\nimport rego.v1\nfindings contains f if { some _ in [1]; f := {\"sev\": \"warning\"} }\n",
    );
    let (code, _err) = eval_status(dir.path(), &[&policy, "--query", "data.arch.findings"]);
    assert!(
        code != 0,
        "a malformed findings array must be a hard error, not a silent pass"
    );
}

#[test]
fn malformed_findings_with_smuggled_non_object_still_errors() {
    // POLENG-009 council C-1: a malformed findings policy must not dodge the
    // gate by mixing a non-object element into the array. An array containing
    // any object is findings-shaped, so this hard-errors rather than falling
    // through to a silent raw-value exit 0.
    let dir = tempfile::tempdir().unwrap();
    let policy = write(
        dir.path(),
        "smuggle.rego",
        "package arch\nimport rego.v1\nfindings := [{\"sev\": \"error\"}, null]\n",
    );
    let (code, _err) = eval_status(dir.path(), &[&policy, "--query", "data.arch.findings"]);
    assert!(
        code != 0,
        "a mixed object/non-object findings array must hard-error, not silently pass"
    );
}
