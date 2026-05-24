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
