//! End-to-end contract tests for `anvil conformance check`.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

struct GitFixture {
    dir: tempfile::TempDir,
    base: String,
    head: String,
}

impl GitFixture {
    fn docs_only_two_commit_range() -> Self {
        let dir = tempfile::tempdir().expect("temp repository");
        git(dir.path(), &["init", "--quiet"]);
        git(
            dir.path(),
            &["config", "user.email", "conformance@example.invalid"],
        );
        git(dir.path(), &["config", "user.name", "Conformance Fixture"]);

        write(dir.path(), "README.md", "fixture\n");
        commit_all(dir.path(), "chore: establish fixture");
        let base = rev_parse(dir.path(), "HEAD");

        write(dir.path(), "docs/first.md", "first\n");
        commit_all(dir.path(), "docs: add first guide");
        write(dir.path(), "docs/second.md", "second\n");
        commit_all(dir.path(), "docs: add second guide");
        let head = rev_parse(dir.path(), "HEAD");

        Self { dir, base, head }
    }

    fn add_commit(&mut self, path: &str, contents: &str, message: &str) {
        write(self.dir.path(), path, contents);
        commit_all(self.dir.path(), message);
        self.head = rev_parse(self.dir.path(), "HEAD");
    }

    fn body_file(&self, body: &str) -> std::path::PathBuf {
        let path = self.dir.path().join("pr-body.md");
        std::fs::write(&path, body).expect("write PR body");
        path
    }
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture parent");
    }
    std::fs::write(path, contents).expect("write fixture");
}

fn git(root: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "--quiet", "-m", message]);
}

fn rev_parse(root: &Path, revision: &str) -> String {
    String::from_utf8(git(root, &["rev-parse", revision]).stdout)
        .expect("UTF-8 revision")
        .trim()
        .to_owned()
}

fn run_anvil(root: &Path, args: &[&str]) -> Output {
    Command::new(ANVIL_BIN)
        .args(args)
        .current_dir(root)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("run anvil")
}

#[test]
fn docs_only_two_commit_range_is_conformant_with_resolved_binding() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let body = fixture.body_file(
        "Private context that must never be echoed.\n\n```anvil-claims\nclaim: documentation-only\n```\n",
    );
    let output = run_anvil(
        fixture.dir.path(),
        &[
            "--json",
            "conformance",
            "check",
            "--base",
            &fixture.base,
            "--head",
            &fixture.head,
            "--pr-body-file",
            body.to_str().expect("body path"),
            "--source-ref",
            "github:pull-request:42:body:sha256:fixture",
        ],
    );

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).expect("one JSON report");
    assert_eq!(report["schemaVersion"], "anvil.conformance-check.v1");
    assert_eq!(report["advisory"], true);
    assert_eq!(report["outcome"], "conformant");
    assert_eq!(report["declarationEvidenceGrade"], "weak");
    assert_eq!(report["evidenceStrength"], "complete");
    assert_eq!(report["resolvedBase"], fixture.base);
    assert_eq!(report["resolvedHead"], fixture.head);
    assert_eq!(report["verdict"]["binding"]["base_revision"], fixture.base);
    assert_eq!(report["verdict"]["binding"]["head_revision"], fixture.head);
    assert!(!stdout.contains("Private context"));
}

#[test]
fn malformed_declaration_is_not_evaluated_but_retains_resolved_range() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let body = fixture.body_file(
        "Do not echo this malformed body.\n\n```anvil-claims\nclaim: unknown-claim\n```\n",
    );
    let output = run_anvil(
        fixture.dir.path(),
        &[
            "conformance",
            "check",
            "--base",
            &fixture.base,
            "--head",
            &fixture.head,
            "--pr-body-file",
            body.to_str().expect("body path"),
            "--source-ref",
            "github:pull-request:43:body:sha256:fixture",
            "--format",
            "json",
        ],
    );

    assert!(
        output.status.success(),
        "semantic non-evaluation is advisory; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).expect("one JSON report");
    assert_eq!(report["outcome"], "not-evaluated");
    assert_eq!(report["evidenceStrength"], "absent");
    assert_eq!(report["resolvedBase"], fixture.base);
    assert_eq!(report["resolvedHead"], fixture.head);
    assert_eq!(report["verdict"], serde_json::Value::Null);
    assert!(
        report["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|reason| reason == "claim.pr-body.claim-unknown")
    );
    assert!(!stdout.contains("Do not echo"));
}

#[test]
fn out_of_class_range_is_non_conformant_with_uncovered_paths_but_exits_zero() {
    let mut fixture = GitFixture::docs_only_two_commit_range();
    fixture.add_commit(
        "src/lib.rs",
        "pub fn changed() {}\n",
        "docs: misclassify behaviour",
    );
    let body = fixture.body_file("```anvil-claims\nclaim: documentation-only\n```\n");
    let output = run_anvil(
        fixture.dir.path(),
        &[
            "conformance",
            "check",
            "--base",
            &fixture.base,
            "--head",
            &fixture.head,
            "--pr-body-file",
            body.to_str().expect("body path"),
            "--source-ref",
            "github:pull-request:44:body:sha256:fixture",
            "--format",
            "json",
        ],
    );

    assert!(output.status.success(), "non-conformance remains advisory");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "non-conformant");
    assert_eq!(report["evidenceStrength"], "complete");
    assert!(
        report["verdict"]["uncovered_files"]
            .as_array()
            .expect("uncovered files")
            .iter()
            .any(|path| path == "src/lib.rs")
    );
}

#[test]
fn graph_semantic_claim_stays_not_evaluated() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let body = fixture.body_file("```anvil-claims\nclaim: no-behaviour-change\n```\n");
    let output = run_anvil(
        fixture.dir.path(),
        &[
            "conformance",
            "check",
            "--base",
            &fixture.base,
            "--head",
            &fixture.head,
            "--pr-body-file",
            body.to_str().expect("body path"),
            "--source-ref",
            "github:pull-request:45:body:sha256:fixture",
            "--format",
            "json",
        ],
    );

    assert!(output.status.success(), "not-evaluated remains advisory");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "not-evaluated");
    assert_eq!(report["evidenceStrength"], "absent");
    assert!(
        report["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|reason| reason == "binding.graph-missing")
    );
}

#[test]
fn stdin_and_trailing_global_json_emit_exactly_one_document() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let mut child = Command::new(ANVIL_BIN)
        .args([
            "conformance",
            "check",
            "--base",
            &fixture.base,
            "--head",
            &fixture.head,
            "--pr-body-file",
            "-",
            "--source-ref",
            "github:pull-request:46:body:sha256:fixture",
            "--json",
        ])
        .current_dir(fixture.dir.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn anvil");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(b"```anvil-claims\nclaim: documentation-only\n```\n")
        .expect("write PR body");
    let output = child.wait_with_output().expect("wait for anvil");

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("exactly one JSON document");
    assert_eq!(report["outcome"], "conformant");
}

#[test]
fn explicit_plain_format_wins_over_global_json_alias() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let body = fixture.body_file("```anvil-claims\nclaim: documentation-only\n```\n");
    let output = run_anvil(
        fixture.dir.path(),
        &[
            "--json",
            "conformance",
            "check",
            "--base",
            &fixture.base,
            "--head",
            &fixture.head,
            "--pr-body-file",
            body.to_str().expect("body path"),
            "--source-ref",
            "github:pull-request:47:body:sha256:fixture",
            "--format",
            "plain",
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert!(stdout.starts_with("Conformance check: conformant\n"));
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err());
}

#[test]
fn over_budget_body_is_bounded_and_not_evaluated() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let body = fixture.body_file(&"x".repeat(anvil_checks::conformance::PR_BODY_MAX_BYTES + 100));
    let output = run_anvil(
        fixture.dir.path(),
        &[
            "conformance",
            "check",
            "--base",
            &fixture.base,
            "--head",
            &fixture.head,
            "--pr-body-file",
            body.to_str().expect("body path"),
            "--source-ref",
            "github:pull-request:48:body:sha256:fixture",
            "--format",
            "json",
        ],
    );

    assert!(
        output.status.success(),
        "over-budget is semantic non-evaluation"
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "not-evaluated");
    assert!(
        report["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|reason| reason == "claim.pr-body.budget.body-bytes")
    );
}

#[test]
fn sarif_is_schema_valid_and_warns_for_every_non_conformance_reason() {
    let mut fixture = GitFixture::docs_only_two_commit_range();
    fixture.add_commit(
        "src/lib.rs",
        "pub fn changed() {}\n",
        "docs: misclassify behaviour",
    );
    let body = fixture.body_file("```anvil-claims\nclaim: documentation-only\n```\n");
    let common = [
        "conformance",
        "check",
        "--base",
        fixture.base.as_str(),
        "--head",
        fixture.head.as_str(),
        "--pr-body-file",
        body.to_str().expect("body path"),
        "--source-ref",
        "github:pull-request:49:body:sha256:fixture",
    ];
    let mut json_args = common.to_vec();
    json_args.extend(["--format", "json"]);
    let json_output = run_anvil(fixture.dir.path(), &json_args);
    let report: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("JSON report");
    let reason_count = report["reasons"].as_array().expect("reasons").len();

    let mut sarif_args = common.to_vec();
    sarif_args.extend(["--format", "sarif"]);
    let output = run_anvil(fixture.dir.path(), &sarif_args);
    assert!(output.status.success(), "SARIF non-conformance is advisory");
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one SARIF document");
    let schema: serde_json::Value =
        serde_json::from_str(anvil_sarif::SARIF_SCHEMA_JSON).expect("SARIF schema");
    let validator = jsonschema::validator_for(&schema).expect("compile SARIF schema");
    let errors: Vec<String> = validator
        .iter_errors(&document)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "SARIF schema errors: {errors:?}");
    let results = document["runs"][0]["results"].as_array().expect("results");
    assert_eq!(results.len(), reason_count);
    assert!(!results.is_empty());
    assert!(results.iter().all(|result| result["level"] == "warning"));
}

#[test]
fn conformant_sarif_has_no_results() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let body = fixture.body_file("```anvil-claims\nclaim: documentation-only\n```\n");
    let output = run_anvil(
        fixture.dir.path(),
        &[
            "conformance",
            "check",
            "--base",
            &fixture.base,
            "--head",
            &fixture.head,
            "--pr-body-file",
            body.to_str().expect("body path"),
            "--source-ref",
            "github:pull-request:50:body:sha256:fixture",
            "--format",
            "sarif",
        ],
    );

    assert!(output.status.success());
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one SARIF document");
    assert!(
        document["runs"][0]["results"]
            .as_array()
            .expect("results")
            .is_empty()
    );
}
