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

    fn docs_only_non_conventional_range() -> Self {
        let mut fixture = Self::docs_only_two_commit_range();
        fixture.base = fixture.head.clone();
        fixture.add_commit("docs/plain.md", "plain\n", "Add a guide without a claim");
        fixture
    }

    fn add_commit(&mut self, path: &str, contents: &str, message: &str) {
        write(self.dir.path(), path, contents);
        commit_all(self.dir.path(), message);
        self.head = rev_parse(self.dir.path(), "HEAD");
    }

    #[cfg(unix)]
    fn add_invalid_utf8_path_commit(&mut self, suffix: u8) -> String {
        use std::os::unix::ffi::OsStringExt as _;

        let mut relative = b"docs/invalid-".to_vec();
        relative.push(suffix);
        relative.extend_from_slice(b".md");
        let path = self.dir.path().join(std::ffi::OsString::from_vec(relative));
        std::fs::write(path, "invalid path fixture\n").expect("write invalid UTF-8 path");
        commit_all(self.dir.path(), "Add path Git cannot decode");
        self.head = rev_parse(self.dir.path(), "HEAD");
        self.head.clone()
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

fn run_anvil_with_stdin(root: &Path, args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(ANVIL_BIN)
        .args(args)
        .current_dir(root)
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
        .write_all(input)
        .expect("write PR body");
    child.wait_with_output().expect("wait for anvil")
}

#[test]
fn hostile_git_repository_environment_cannot_redirect_conformance_discovery() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let decoy = tempfile::tempdir().expect("temp decoy repository");
    git(decoy.path(), &["init", "--quiet"]);
    let body = fixture.body_file("```anvil-claims\nclaim: documentation-only\n```\n");
    let output = Command::new(ANVIL_BIN)
        .args([
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
            "pull-request:hostile-environment:body:sha256:fixture",
        ])
        .current_dir(fixture.dir.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("GIT_DIR", decoy.path().join(".git"))
        .env("GIT_WORK_TREE", decoy.path())
        .output()
        .expect("run anvil with hostile Git environment");

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "conformant");
    assert_eq!(report["resolvedBase"], fixture.base);
    assert_eq!(report["resolvedHead"], fixture.head);
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
fn non_conventional_docs_only_range_is_conformant_from_pr_declaration() {
    let fixture = GitFixture::docs_only_non_conventional_range();
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
            "github:pull-request:48:body:sha256:fixture",
        ],
    );

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "conformant");
    assert_eq!(report["evidenceStrength"], "complete");
    assert_eq!(report["resolvedBase"], fixture.base);
    assert_eq!(report["resolvedHead"], fixture.head);
}

#[cfg(unix)]
#[test]
fn invalid_declaration_and_git_footprint_report_both_reason_sets() {
    let mut fixture = GitFixture::docs_only_non_conventional_range();
    fixture.base = fixture.head.clone();
    fixture.add_invalid_utf8_path_commit(0xff);
    fixture.add_invalid_utf8_path_commit(0xfe);
    let body = fixture.body_file("```anvil-claims\nclaim: unknown-claim\n```\n");
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
            "github:pull-request:49:body:sha256:fixture",
        ],
    );

    assert!(output.status.success(), "non-evaluation remains advisory");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "not-evaluated");
    assert_eq!(
        report["reasons"],
        serde_json::json!([
            "claim.pr-body.claim-unknown",
            "git.commit-not-evaluated.git.path-invalid-utf8"
        ])
    );
    assert_eq!(report["notEvaluatedCommitCount"], 2);
    let failures = report["gitNonEvaluations"]
        .as_array()
        .expect("structured per-commit failures");
    assert_eq!(failures.len(), 2);
    assert!(
        failures
            .iter()
            .all(|failure| failure["reason"] == "git.path-invalid-utf8")
    );
    assert!(failures.iter().all(|failure| failure["stage"] == "diff"));
    assert!(
        failures
            .iter()
            .all(|failure| failure["commitRevision"].as_str().is_some())
    );
    let revisions: Vec<_> = failures
        .iter()
        .map(|failure| failure["commitRevision"].as_str().expect("revision"))
        .collect();
    assert!(revisions.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("detail"));
}

#[cfg(unix)]
#[test]
fn plain_output_counts_and_enumerates_every_failed_commit() {
    let mut fixture = GitFixture::docs_only_non_conventional_range();
    fixture.base = fixture.head.clone();
    let first = fixture.add_invalid_utf8_path_commit(0xff);
    let second = fixture.add_invalid_utf8_path_commit(0xfe);
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
            "plain",
        ],
    );

    assert!(output.status.success(), "non-evaluation remains advisory");
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 plain report");
    assert!(stdout.contains("Not-evaluated commits: 2"));
    assert_eq!(stdout.matches("Git non-evaluation:").count(), 2);
    assert!(stdout.contains("reason=git.path-invalid-utf8"));
    assert!(stdout.contains("stage=diff"));
    assert!(stdout.contains("observed=0"));
    assert!(stdout.contains("limit=none"));
    let mut revisions = [first, second];
    revisions.sort();
    let failure_lines: Vec<_> = stdout
        .lines()
        .filter(|line| line.starts_with("Git non-evaluation:"))
        .collect();
    assert!(failure_lines[0].contains(&revisions[0]));
    assert!(failure_lines[1].contains(&revisions[1]));
    assert!(!stdout.contains("detail"));
}

#[cfg(unix)]
#[test]
fn sarif_keeps_declaration_reason_and_one_fingerprinted_result_per_failed_commit() {
    let mut fixture = GitFixture::docs_only_non_conventional_range();
    fixture.base = fixture.head.clone();
    let first = fixture.add_invalid_utf8_path_commit(0xff);
    let second = fixture.add_invalid_utf8_path_commit(0xfe);
    let body = fixture.body_file("```anvil-claims\nclaim: unknown-claim\n```\n");
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
            "github:pull-request:51:body:sha256:fixture",
            "--format",
            "sarif",
        ],
    );

    assert!(output.status.success(), "non-evaluation remains advisory");
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
    assert_eq!(results.len(), 3);
    assert_eq!(
        results
            .iter()
            .filter(|result| {
                result["ruleId"] == "anvil.conformance.claim.pr-body.claim-unknown"
            })
            .count(),
        1
    );
    let git_results: Vec<_> = results
        .iter()
        .filter(|result| {
            result["ruleId"] == "anvil.conformance.git.commit-not-evaluated.git.path-invalid-utf8"
        })
        .collect();
    assert_eq!(git_results.len(), 2);
    assert!(git_results.iter().all(|result| {
        result["partialFingerprints"]["anvilConformanceCommit/v1"]
            .as_str()
            .is_some()
    }));
    let fingerprints: std::collections::BTreeSet<_> = git_results
        .iter()
        .map(|result| {
            result["partialFingerprints"]["anvilConformanceCommit/v1"]
                .as_str()
                .expect("fingerprint")
        })
        .collect();
    assert_eq!(fingerprints.len(), 2);
    for revision in [first, second] {
        assert!(git_results.iter().any(|result| {
            result["message"]["text"]
                .as_str()
                .is_some_and(|message| message.contains(&revision))
        }));
    }
    assert!(!String::from_utf8_lossy(&output.stdout).contains("detail"));
}

#[test]
fn selection_failure_retains_structure_and_unknown_cardinality_in_json() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let body = fixture.body_file("```anvil-claims\nclaim: documentation-only\n```\n");
    let output = run_anvil(
        fixture.dir.path(),
        &[
            "conformance",
            "check",
            "--base",
            "refs/heads/definitely-missing",
            "--head",
            &fixture.head,
            "--pr-body-file",
            body.to_str().expect("body path"),
            "--source-ref",
            "pull-request:selection-failure:body:sha256:fixture",
            "--format",
            "json",
        ],
    );

    assert!(
        output.status.success(),
        "Git non-evaluation remains advisory"
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "not-evaluated");
    assert!(report["notEvaluatedCommitCount"].is_null());
    assert_eq!(
        report["gitEvaluationNonEvaluation"]["reason"],
        "revision.invalid"
    );
    assert_eq!(report["gitEvaluationNonEvaluation"]["stage"], "revision");
    assert_eq!(report["gitEvaluationNonEvaluation"]["observed"], 0);
    assert!(report["gitEvaluationNonEvaluation"]["limit"].is_null());
    assert!(report["gitEvaluationNonEvaluation"]["rawDigest"].is_null());
    assert!(report["gitEvaluationNonEvaluation"]["budget"].is_null());
    assert!(
        report["gitNonEvaluations"]
            .as_array()
            .expect("per-commit failures")
            .is_empty()
    );
}

#[test]
fn selection_failure_is_distinct_in_plain_and_sarif() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let body = fixture.body_file("```anvil-claims\nclaim: documentation-only\n```\n");
    let common = [
        "conformance",
        "check",
        "--base",
        "refs/heads/definitely-missing",
        "--head",
        fixture.head.as_str(),
        "--pr-body-file",
        body.to_str().expect("body path"),
        "--source-ref",
        "pull-request:selection-failure:body:sha256:fixture",
    ];

    let mut plain_args = common.to_vec();
    plain_args.extend(["--format", "plain"]);
    let plain = run_anvil(fixture.dir.path(), &plain_args);
    assert!(
        plain.status.success(),
        "Git non-evaluation remains advisory"
    );
    let plain = String::from_utf8(plain.stdout).expect("UTF-8 plain report");
    assert!(plain.contains("Not-evaluated commits: unknown"));
    assert!(plain.contains(
        "Git evaluation non-evaluation: commit=unknown reason=revision.invalid stage=revision observed=0 limit=none raw-digest=none"
    ));
    assert!(!plain.contains("Git non-evaluation:"));

    let mut sarif_args = common.to_vec();
    sarif_args.extend(["--format", "sarif"]);
    let sarif = run_anvil(fixture.dir.path(), &sarif_args);
    assert!(
        sarif.status.success(),
        "Git non-evaluation remains advisory"
    );
    let document: serde_json::Value =
        serde_json::from_slice(&sarif.stdout).expect("one SARIF document");
    let results = document["runs"][0]["results"].as_array().expect("results");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0]["ruleId"],
        "anvil.conformance.git.evaluation-not-evaluated.revision.invalid"
    );
    assert!(
        results[0]["message"]["text"]
            .as_str()
            .expect("message")
            .contains(
                "reason=revision.invalid stage=revision observed=0 limit=none raw-digest=none"
            )
    );
    assert!(
        results[0]["partialFingerprints"]["anvilConformanceEvaluation/v1"]
            .as_str()
            .is_some()
    );
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
fn invalid_utf8_body_is_advisory_for_file_and_stdin_in_every_format() {
    const REASON: &str = "claim.pr-body.encoding.invalid-utf8";

    let fixture = GitFixture::docs_only_two_commit_range();
    let path = fixture.dir.path().join("pr-body-invalid-utf8.md");
    std::fs::write(&path, b"\xff").expect("write invalid UTF-8 PR body");

    for format in ["plain", "json", "sarif"] {
        for from_stdin in [false, true] {
            let body_argument = if from_stdin {
                "-"
            } else {
                path.to_str().expect("body path")
            };
            let args = [
                "conformance",
                "check",
                "--base",
                fixture.base.as_str(),
                "--head",
                fixture.head.as_str(),
                "--pr-body-file",
                body_argument,
                "--source-ref",
                "pull-request:invalid-utf8:body:sha256:fixture",
                "--format",
                format,
            ];
            let output = if from_stdin {
                run_anvil_with_stdin(fixture.dir.path(), &args, b"\xff")
            } else {
                run_anvil(fixture.dir.path(), &args)
            };

            assert!(
                output.status.success(),
                "{format} from_stdin={from_stdin} must be advisory; stderr:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            match format {
                "plain" => {
                    let stdout = String::from_utf8(output.stdout).expect("UTF-8 plain report");
                    assert!(stdout.contains("Conformance check: not-evaluated"));
                    assert!(stdout.contains(&format!("Reason: {REASON}")));
                }
                "json" => {
                    let report: serde_json::Value =
                        serde_json::from_slice(&output.stdout).expect("one JSON report");
                    assert_eq!(report["outcome"], "not-evaluated");
                    assert!(
                        report["reasons"]
                            .as_array()
                            .expect("reasons")
                            .iter()
                            .any(|reason| reason == REASON)
                    );
                }
                "sarif" => {
                    let document: serde_json::Value =
                        serde_json::from_slice(&output.stdout).expect("one SARIF document");
                    assert!(
                        document["runs"][0]["results"]
                            .as_array()
                            .expect("results")
                            .iter()
                            .any(|result| {
                                result["ruleId"] == format!("anvil.conformance.{REASON}")
                            })
                    );
                }
                _ => unreachable!("fixed test formats"),
            }
        }
    }
}

#[test]
fn over_budget_body_with_a_split_multibyte_character_is_not_evaluated() {
    let fixture = GitFixture::docs_only_two_commit_range();
    let path = fixture.dir.path().join("pr-body-split-multibyte.md");
    let mut body = vec![b'x'; anvil_checks::conformance::PR_BODY_MAX_BYTES];
    body.extend_from_slice("é".as_bytes());
    std::fs::write(&path, body).expect("write over-budget PR body");
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
            path.to_str().expect("body path"),
            "--source-ref",
            "pull-request:over-budget:body:sha256:fixture",
            "--format",
            "json",
        ],
    );

    assert!(
        output.status.success(),
        "over-budget is semantic non-evaluation; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
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
