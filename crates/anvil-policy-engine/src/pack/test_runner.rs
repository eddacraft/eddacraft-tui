//! Policy pack test runner and enforcement (POLVAL-004).
//!
//! Two layers sit here:
//!
//! 1. [`run_pack_tests`] loads each member policy together with its sibling
//!    `*_test.rego` into a *fresh* facade [`Engine`] — one engine per member
//!    pair, so a policy that fails to compile or panics cannot poison the tests
//!    of a sibling — discovers `test_*` rules, and evaluates each. OPA
//!    semantics apply: a test rule passes iff it evaluates to `true`; `false`
//!    or `undefined` is a failure. The engine keeps the facade's determinism
//!    fence and evaluation timeout ([`EngineConfig::default`]), so a timeout or
//!    evaluation error becomes a captured failure detail, never a crash.
//! 2. [`enforce_tests`] folds a [`TestRunReport`] into the POLVAL-004
//!    enforcement contract as error-class [`ValidationIssue`]s: a missing test
//!    file (the validator's pre-enforcement warning) escalates to an error, an
//!    existing-but-empty test file is an error, and every failing rule is an
//!    error whose remediation names the rule.
//!
//! ## Test-rule discovery
//!
//! `regorus` 0.10.1 can enumerate modules/rules via its AST, but the facade
//! does not surface that API and traversing the internal AST would couple this
//! crate to `regorus`'s internal shapes. For this slice, discovery is a
//! conservative source scan: a line beginning `test_` at column zero declares a
//! test rule (its leading identifier is taken, names deduplicated). Limitation:
//! a test rule produced by an unusual construct (metadata-generated, or indented
//! under another rule) is not discovered. That is acceptable for slice 1 and no
//! Rego parser is vendored. The test package is read from the file's `package`
//! declaration.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::pack::manifest::{PackManifest, resolve_member_path};
use crate::pack::validator::{IssueCode, IssueSeverity, ValidationIssue, test_sibling};
use crate::{Engine, EngineConfig, PolicyInput};

/// The result of evaluating a single `test_*` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestOutcome {
    /// The rule name, e.g. `test_allow_when_owner`.
    pub rule: String,
    /// Whether the rule passed (evaluated to `true`).
    pub passed: bool,
    /// Why it failed (false / undefined / non-boolean / error), when it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// The test results for one pack member (a policy plus its test file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberTestResult {
    /// The member's policy id (may be empty if metadata is incomplete).
    pub policy_id: String,
    /// The member's `.rego` path, relative to the pack directory.
    pub policy_path: PathBuf,
    /// The sibling test path, or `None` when no `*_test.rego` exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_path: Option<PathBuf>,
    /// The package declared by the test file, when one exists and declares one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_package: Option<String>,
    /// Set when the policy/test could not be loaded or run at all (compile
    /// failure, unreadable file, missing package). Blocks the member's tests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_error: Option<String>,
    /// Per-rule outcomes, in discovery order.
    pub outcomes: Vec<TestOutcome>,
}

/// The outcome of running every member's tests.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRunReport {
    /// One entry per manifest member, in manifest order.
    pub members: Vec<MemberTestResult>,
}

/// A catastrophic failure that prevents the test run from proceeding at all —
/// distinct from a per-rule or per-member failure, which is captured in the
/// report.
#[derive(Debug, Error)]
pub enum TestRunError {
    /// The facade engine could not be constructed.
    #[error("could not construct policy engine: {0}")]
    Engine(String),
}

/// Load and run each member's tests, one fresh engine per member pair.
///
/// `base_dir` is the manifest's own directory; member paths resolve beneath it.
/// A missing test file is not an error here — it is recorded as `test_path:
/// None` and left to [`enforce_tests`] to escalate. Returns [`Err`] only for a
/// setup failure that stops the whole run (see [`TestRunError`]).
pub fn run_pack_tests(
    manifest: &PackManifest,
    base_dir: &Path,
) -> Result<TestRunReport, TestRunError> {
    let mut members = Vec::with_capacity(manifest.policies.len());
    for entry in &manifest.policies {
        members.push(run_member(entry.metadata.id.trim(), &entry.path, base_dir)?);
    }
    Ok(TestRunReport { members })
}

/// Run one member's tests. All member-local failures are captured in the
/// returned [`MemberTestResult`]; only engine construction can bubble up.
fn run_member(
    policy_id: &str,
    policy_path: &Path,
    base_dir: &Path,
) -> Result<MemberTestResult, TestRunError> {
    let mut result = MemberTestResult {
        policy_id: policy_id.to_string(),
        policy_path: policy_path.to_path_buf(),
        test_path: None,
        test_package: None,
        load_error: None,
        outcomes: Vec::new(),
    };

    // A sibling `*_test.rego` must exist for tests to run at all. Resolve it
    // through the containment guard: an escaping symlink or an absent file both
    // mean "no valid in-pack test", so external content is never opened.
    let Some(test_rel) = test_sibling(policy_path) else {
        return Ok(result);
    };
    let test_abs = match resolve_member_path(base_dir, &test_rel) {
        Ok(path) if path.is_file() => path,
        _ => return Ok(result),
    };
    result.test_path = Some(test_rel.clone());

    // Resolve the policy source through the same guard before reading it, so a
    // symlink pointing outside the pack is refused rather than evaluated.
    let Ok(policy_abs) = resolve_member_path(base_dir, policy_path) else {
        result.load_error = Some(format!(
            "policy source `{}` escapes the pack directory and was not evaluated",
            policy_path.display()
        ));
        return Ok(result);
    };

    // Read both sources. An existing-but-unreadable file, or a missing policy
    // source, is a load error (the member's tests cannot run).
    let policy_source = match std::fs::read_to_string(&policy_abs) {
        Ok(src) => src,
        Err(e) => {
            result.load_error = Some(format!(
                "could not read policy source `{}`: {e}",
                policy_path.display()
            ));
            return Ok(result);
        }
    };
    let test_source = match std::fs::read_to_string(&test_abs) {
        Ok(src) => src,
        Err(e) => {
            result.load_error = Some(format!(
                "could not read test file `{}`: {e}",
                test_rel.display()
            ));
            return Ok(result);
        }
    };

    let Some(package) = parse_test_package(&test_source) else {
        result.load_error = Some(format!(
            "test file `{}` has no `package` declaration",
            test_rel.display()
        ));
        return Ok(result);
    };
    result.test_package = Some(package.clone());

    // Discover rules before the source is consumed by `add_policy`.
    let rules = discover_test_rules_from_source(&test_source);

    // Fresh engine per member pair (determinism fence + eval timeout via the
    // default config), so a broken member cannot poison a sibling.
    let mut engine =
        Engine::new(EngineConfig::default()).map_err(|e| TestRunError::Engine(e.to_string()))?;

    if let Err(e) = engine.add_policy(path_string(policy_path), policy_source) {
        result.load_error = Some(format!("policy failed to load: {e}"));
        return Ok(result);
    }
    if let Err(e) = engine.add_policy(path_string(&test_rel), test_source) {
        result.load_error = Some(format!("test file failed to load: {e}"));
        return Ok(result);
    }

    let input = PolicyInput::default();
    for rule in rules {
        let query = format!("data.{package}.{rule}");
        result
            .outcomes
            .push(evaluate_rule(&mut engine, &input, &query, rule));
    }

    Ok(result)
}

/// Evaluate one rule query and classify the outcome under OPA test semantics.
fn evaluate_rule(
    engine: &mut Engine,
    input: &PolicyInput,
    query: &str,
    rule: String,
) -> TestOutcome {
    match engine.eval(input, query) {
        Ok(result) => match result.value {
            Some(serde_json::Value::Bool(true)) => TestOutcome {
                rule,
                passed: true,
                detail: None,
            },
            Some(serde_json::Value::Bool(false)) => TestOutcome {
                rule,
                passed: false,
                detail: Some("rule evaluated to false".to_string()),
            },
            None => TestOutcome {
                rule,
                passed: false,
                detail: Some("rule was undefined".to_string()),
            },
            Some(other) => TestOutcome {
                rule,
                passed: false,
                detail: Some(format!("rule evaluated to a non-boolean value: {other}")),
            },
        },
        Err(e) => TestOutcome {
            rule,
            passed: false,
            detail: Some(format!("evaluation error: {e}")),
        },
    }
}

/// Fold a [`TestRunReport`] into error-class validation issues per the
/// POLVAL-004 enforcement contract. Issues are emitted in member order.
#[must_use]
pub fn enforce_tests(report: &TestRunReport) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for member in &report.members {
        let policy_id = (!member.policy_id.is_empty()).then(|| member.policy_id.clone());

        // Missing test file — escalated from the validator's warning to error.
        if member.test_path.is_none() {
            issues.push(ValidationIssue {
                code: IssueCode::MissingTestFile,
                severity: IssueSeverity::Error,
                policy_id: policy_id.clone(),
                path: Some(member.policy_path.clone()),
                message: format!(
                    "policy `{}` has no sibling test file; tests are required",
                    member.policy_path.display()
                ),
                remediation: "Add a sibling `*_test.rego` with `test_*` rules covering \
                              this policy."
                    .to_string(),
            });
            continue;
        }

        // A load failure means the tests could not run — block.
        if let Some(load_error) = &member.load_error {
            issues.push(ValidationIssue {
                code: IssueCode::PolicyTestFailed,
                severity: IssueSeverity::Error,
                policy_id: policy_id.clone(),
                path: member.test_path.clone(),
                message: format!("could not run tests: {load_error}"),
                remediation: "Ensure the policy and its test file compile and declare a \
                              package before validating."
                    .to_string(),
            });
            continue;
        }

        // A non-`_test` package is advisory: still run, but flag the naming.
        if let Some(package) = &member.test_package
            && !package.ends_with("_test")
        {
            issues.push(ValidationIssue {
                code: IssueCode::TestPackageNaming,
                severity: IssueSeverity::Warning,
                policy_id: policy_id.clone(),
                path: member.test_path.clone(),
                message: format!(
                    "test package `{package}` does not follow the `<name>_test` convention"
                ),
                remediation: "Rename the test package to end in `_test`.".to_string(),
            });
        }

        // An existing test file with no discovered rules blocks.
        if member.outcomes.is_empty() {
            issues.push(ValidationIssue {
                code: IssueCode::NoTestsDiscovered,
                severity: IssueSeverity::Error,
                policy_id: policy_id.clone(),
                path: member.test_path.clone(),
                message: format!(
                    "test file for policy `{}` declares no `test_*` rules",
                    member.policy_path.display()
                ),
                remediation: "Add at least one `test_*` rule to the test file.".to_string(),
            });
            continue;
        }

        // Each failing rule is an error whose remediation names the rule.
        for outcome in &member.outcomes {
            if !outcome.passed {
                let detail = outcome.detail.as_deref().unwrap_or("test failed");
                issues.push(ValidationIssue {
                    code: IssueCode::PolicyTestFailed,
                    severity: IssueSeverity::Error,
                    policy_id: policy_id.clone(),
                    path: member.test_path.clone(),
                    message: format!("test rule `{}` failed: {detail}", outcome.rule),
                    remediation: format!(
                        "Fix the policy or the test so `{}` passes.",
                        outcome.rule
                    ),
                });
            }
        }
    }
    issues
}

/// Read the package from a test file's `package` declaration, or `None`.
fn parse_test_package(source: &str) -> Option<String> {
    for line in source.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("package ") {
            let package = rest.trim();
            if !package.is_empty() {
                return Some(package.to_string());
            }
        }
    }
    None
}

/// Discover `test_*` rule names by a conservative source scan: a line starting
/// at column zero with `test_` followed by at least one more identifier char.
/// Names are deduplicated, preserving first-seen order. See the module docs for
/// the limitation.
///
/// Backtick raw-string content is blanked out first so a `test_*` line *inside*
/// a Rego raw string is not mistaken for a rule (which would evaluate undefined
/// and red a healthy pack).
fn discover_test_rules_from_source(source: &str) -> Vec<String> {
    let stripped = strip_raw_strings(source);
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for line in stripped.lines() {
        if !line.starts_with("test_") {
            continue;
        }
        let ident: String = line
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if ident.len() <= "test_".len() {
            continue;
        }
        if seen.insert(ident.clone()) {
            names.push(ident);
        }
    }
    names
}

/// Replace the content of Rego backtick raw strings with spaces, preserving
/// newlines so line structure (and thus column-zero scanning) survives. Rego
/// raw strings have no escape sequences, so a single toggle on the backtick
/// character is sufficient.
fn strip_raw_strings(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_raw = false;
    for ch in source.chars() {
        match ch {
            '`' => {
                in_raw = !in_raw;
                out.push(' ');
            }
            '\n' => out.push('\n'),
            _ if in_raw => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

/// Render a path as a stable string for `add_policy` (regorus uses it only as a
/// source label). Non-UTF-8 paths fall back to a lossy label.
fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(base: &Path, rel: &str, body: &str) {
        let path = base.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create dirs");
        }
        std::fs::write(path, body).expect("write file");
    }

    fn one_member_manifest(policy_id: &str) -> PackManifest {
        let yaml = format!(
            "
id: pack
name: Pack
version: 1.0.0
description: d
owner: o
policies:
  - path: policies/a.rego
    metadata:
      id: {policy_id}
      title: t
      severity: high
      owner: o
      rationale: r
      scope: s
      tags: [x]
"
        );
        serde_yaml::from_str(&yaml).expect("parse manifest")
    }

    const POLICY: &str = "package a\nimport rego.v1\n\nallow if input.x == 1\n";

    #[test]
    fn policy_test_runner_discovers_rules_and_package() {
        let src = "package a_test\nimport rego.v1\n\ntest_one if true\ntest_two := false\nhelper := 1\ntest_one if false\n";
        assert_eq!(parse_test_package(src).as_deref(), Some("a_test"));
        assert_eq!(
            discover_test_rules_from_source(src),
            vec!["test_one".to_string(), "test_two".to_string()]
        );
    }

    #[test]
    fn policy_test_runner_backtick_raw_string_not_discovered() {
        // Reviewer repro (MAJOR): a `test_*` line inside a backtick raw string
        // must not be discovered as a rule. Exactly one real rule survives.
        let src = "package a_test\nimport rego.v1\n\ndoc := `\ntest_example_usage if { input.x == 1 }\n`\n\ntest_real if true\n";
        assert_eq!(
            discover_test_rules_from_source(src),
            vec!["test_real".to_string()],
            "a test_ line inside a raw string must be ignored"
        );
    }

    #[test]
    fn policy_test_runner_backtick_raw_string_pack_stays_green() {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "policies/a.rego", POLICY);
        write(
            dir.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ndoc := `\ntest_example_usage if { true }\n`\n\ntest_real if data.a.allow with input as {\"x\": 1}\n",
        );
        let report = run_pack_tests(&one_member_manifest("policy-a"), dir.path()).expect("run");
        let member = &report.members[0];
        assert_eq!(member.outcomes.len(), 1, "{member:?}");
        assert!(member.outcomes[0].passed);
        assert!(enforce_tests(&report).is_empty(), "pack must stay green");
    }

    // Reviewer repro (CRITICAL): a symlink at the policy member path pointing
    // outside the pack must not be read or evaluated; it surfaces as a load
    // error that enforcement turns into an error-class issue. Unix-only.
    #[cfg(unix)]
    #[test]
    fn policy_test_runner_symlink_policy_escape_not_evaluated() {
        let outside = TempDir::new().expect("outside dir");
        let secret = outside.path().join("secret.rego");
        // External content that, if evaluated, would define `allow`.
        std::fs::write(&secret, "package a\nimport rego.v1\n\nallow if true\n").expect("write");

        let pack = TempDir::new().expect("pack dir");
        std::fs::create_dir_all(pack.path().join("policies")).expect("mkdir");
        std::os::unix::fs::symlink(&secret, pack.path().join("policies/a.rego")).expect("symlink");
        write(
            pack.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ntest_real if data.a.allow with input as {\"x\": 1}\n",
        );

        let report = run_pack_tests(&one_member_manifest("policy-a"), pack.path()).expect("run");
        let member = &report.members[0];
        assert!(
            member.load_error.is_some(),
            "escaping policy symlink must not be evaluated: {member:?}"
        );
        assert!(member.outcomes.is_empty());

        let issues = enforce_tests(&report);
        assert!(
            issues.iter().any(
                |i| i.code == IssueCode::PolicyTestFailed && i.severity == IssueSeverity::Error
            ),
            "escape must be an error-class outcome: {issues:?}"
        );
    }

    #[test]
    fn policy_test_runner_passing_pack_is_all_green() {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "policies/a.rego", POLICY);
        write(
            dir.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ntest_allow if data.a.allow with input as {\"x\": 1}\ntest_deny if not data.a.allow with input as {\"x\": 2}\n",
        );
        let report = run_pack_tests(&one_member_manifest("policy-a"), dir.path()).expect("run");
        let member = &report.members[0];
        assert_eq!(member.test_package.as_deref(), Some("a_test"));
        assert_eq!(member.outcomes.len(), 2);
        assert!(member.outcomes.iter().all(|o| o.passed), "{member:?}");
        assert!(
            enforce_tests(&report).is_empty(),
            "a green pack raises no issues"
        );
    }

    #[test]
    fn policy_test_runner_failing_rule_is_reported_and_enforced() {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "policies/a.rego", POLICY);
        // test_false evaluates to an explicit false → fail.
        write(
            dir.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ntest_ok if data.a.allow with input as {\"x\": 1}\ntest_false := false\n",
        );
        let report = run_pack_tests(&one_member_manifest("policy-a"), dir.path()).expect("run");
        let member = &report.members[0];
        let failed: Vec<_> = member.outcomes.iter().filter(|o| !o.passed).collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].rule, "test_false");
        assert!(failed[0].detail.as_deref().unwrap().contains("false"));

        let issues = enforce_tests(&report);
        let test_failures: Vec<_> = issues
            .iter()
            .filter(|i| i.code == IssueCode::PolicyTestFailed)
            .collect();
        assert_eq!(test_failures.len(), 1);
        assert_eq!(test_failures[0].severity, IssueSeverity::Error);
        assert!(test_failures[0].remediation.contains("test_false"));
    }

    #[test]
    fn policy_test_runner_undefined_rule_is_a_failure() {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "policies/a.rego", POLICY);
        // test_undef asserts allow is true when x=2, but allow is undefined then.
        write(
            dir.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ntest_undef if data.a.allow with input as {\"x\": 2}\n",
        );
        let report = run_pack_tests(&one_member_manifest("policy-a"), dir.path()).expect("run");
        let outcome = &report.members[0].outcomes[0];
        assert!(!outcome.passed);
        assert!(outcome.detail.as_deref().unwrap().contains("undefined"));
    }

    #[test]
    fn policy_test_runner_missing_test_file_is_enforced_as_error() {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "policies/a.rego", POLICY);
        // No sibling *_test.rego.
        let report = run_pack_tests(&one_member_manifest("policy-a"), dir.path()).expect("run");
        assert!(report.members[0].test_path.is_none());
        let issues = enforce_tests(&report);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::MissingTestFile);
        assert_eq!(issues[0].severity, IssueSeverity::Error);
    }

    #[test]
    fn policy_test_runner_empty_test_file_is_enforced_as_error() {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "policies/a.rego", POLICY);
        write(
            dir.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\nhelper := 1\n",
        );
        let report = run_pack_tests(&one_member_manifest("policy-a"), dir.path()).expect("run");
        assert!(report.members[0].outcomes.is_empty());
        let issues = enforce_tests(&report);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::NoTestsDiscovered);
        assert_eq!(issues[0].severity, IssueSeverity::Error);
    }

    #[test]
    fn policy_test_runner_non_test_package_is_a_naming_warning() {
        // Fold-in logic unit test: a member whose test package is not `_test`
        // and whose rules pass yields exactly one Warning-class naming issue.
        let report = TestRunReport {
            members: vec![MemberTestResult {
                policy_id: "policy-a".into(),
                policy_path: PathBuf::from("policies/a.rego"),
                test_path: Some(PathBuf::from("policies/a_test.rego")),
                test_package: Some("a".into()),
                load_error: None,
                outcomes: vec![TestOutcome {
                    rule: "test_ok".into(),
                    passed: true,
                    detail: None,
                }],
            }],
        };
        let issues = enforce_tests(&report);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::TestPackageNaming);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn policy_test_runner_load_error_is_enforced_as_error() {
        // Fold-in logic unit test standing in for a compile/timeout failure.
        let report = TestRunReport {
            members: vec![MemberTestResult {
                policy_id: "policy-a".into(),
                policy_path: PathBuf::from("policies/a.rego"),
                test_path: Some(PathBuf::from("policies/a_test.rego")),
                test_package: None,
                load_error: Some("policy failed to load: syntax error".into()),
                outcomes: vec![],
            }],
        };
        let issues = enforce_tests(&report);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::PolicyTestFailed);
        assert_eq!(issues[0].severity, IssueSeverity::Error);
        assert!(issues[0].message.contains("syntax error"));
    }
}
