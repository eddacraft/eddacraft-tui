//! `anvil policy test` — execute policy-pack and free-form Rego tests.
//!
//! Pack directories (with `pack.yaml`) use the policy-engine pack test runner.
//! Loose `*_test.rego` trees are evaluated in-process via the same engine so a
//! CI invocation never exits 0 on a silent stub.

use std::path::{Path, PathBuf};

use anvil_policy_engine::pack::{
    MemberTestResult, PackManifest, PolicyEntry, PolicyMetadata, PolicySeverity, TestOutcome,
    TestRunReport, enforce_tests, load_manifest, run_pack_tests,
};
use anvil_policy_engine::{Engine, EngineConfig, PolicyInput};
use anyhow::{Context, Result};

use crate::GlobalArgs;
use crate::output;

/// Canonical pack manifest filename.
const MANIFEST_FILENAME: &str = "pack.yaml";

/// CLI-facing aggregate of a policy test run.
#[derive(Debug, serde::Serialize)]
pub(super) struct TestResult {
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub tests: Vec<TestCase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
}

#[derive(Debug, serde::Serialize)]
pub(super) struct TestCase {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

/// Run `anvil policy test [path] [--list-files]`.
pub(super) fn run(path: Option<&str>, list_files: bool, global: &GlobalArgs) -> Result<()> {
    let test_path = path.unwrap_or(".anvil/policies");
    let root = Path::new(test_path);

    if list_files {
        return list_discovered_files(test_path, root, global);
    }

    if !root.exists() {
        return emit_missing_path(test_path, global);
    }

    let result = execute_tests(root)
        .with_context(|| format!("running policy tests under {}", root.display()))?;

    emit_result(&result, global)?;

    if result.failed > 0 {
        return Err(output::AlreadyReported.into());
    }
    Ok(())
}

fn list_discovered_files(test_path: &str, root: &Path, global: &GlobalArgs) -> Result<()> {
    if !root.exists() {
        return emit_missing_path(test_path, global);
    }
    let files = collect_policy_test_files(test_path);
    if files.is_empty() {
        return emit_warning_only(
            &format!("No test files found in '{test_path}'"),
            global,
            &TestResult {
                passed: 0,
                failed: 0,
                skipped: 0,
                tests: vec![],
                warning: Some(format!("No test files found in '{test_path}'")),
                files: None,
            },
        );
    }
    let result = TestResult {
        passed: 0,
        failed: 0,
        skipped: 0,
        tests: vec![],
        warning: None,
        files: Some(files.clone()),
    };
    if global.json {
        crate::output::json::print(&result)?;
    } else {
        crate::output::plain::blank();
        crate::output::plain::info(&format!(
            "Found {} policy test file(s) under '{test_path}':",
            files.len()
        ));
        for f in &files {
            crate::output::plain::info(f);
        }
    }
    Ok(())
}

fn emit_missing_path(test_path: &str, global: &GlobalArgs) -> Result<()> {
    emit_warning_only(
        &format!("No policy test directory found at '{test_path}'"),
        global,
        &TestResult {
            passed: 0,
            failed: 0,
            skipped: 0,
            tests: vec![],
            warning: Some(format!("No policy test directory found at '{test_path}'")),
            files: None,
        },
    )
}

fn emit_warning_only(plain_message: &str, global: &GlobalArgs, result: &TestResult) -> Result<()> {
    if global.json {
        crate::output::json::print(result)?;
    } else {
        crate::output::plain::blank();
        crate::output::plain::warn(plain_message);
    }
    Ok(())
}

fn emit_result(result: &TestResult, global: &GlobalArgs) -> Result<()> {
    if global.json {
        crate::output::json::print(result)?;
        return Ok(());
    }

    crate::output::plain::blank();
    if let Some(warning) = &result.warning {
        crate::output::plain::warn(warning);
    }
    for case in &result.tests {
        if case.passed {
            crate::output::plain::info(&format!("PASS  {}", case.name));
        } else {
            crate::output::plain::error(&format!("FAIL  {} — {}", case.name, case.message));
        }
    }
    crate::output::plain::blank();
    crate::output::plain::info(&format!(
        "Policy tests: {} passed, {} failed, {} skipped",
        result.passed, result.failed, result.skipped
    ));
    Ok(())
}

/// Resolve pack and free-form targets under `root` and execute every test.
///
/// When `root` is a policies directory that holds both packs and loose
/// `*_test.rego` files, both are executed. Tests living inside a discovered
/// pack directory are not free-form-evaluated (the pack runner owns them).
fn execute_tests(root: &Path) -> Result<TestResult> {
    let pack_manifests = resolve_pack_manifests(root)?;
    let pack_dirs: Vec<PathBuf> = pack_manifests
        .iter()
        .filter_map(|m| m.parent().map(Path::to_path_buf))
        .collect();

    let mut combined = TestRunReport::default();

    if !pack_manifests.is_empty() {
        let pack_report = run_pack_manifests_report(&pack_manifests)?;
        combined.members.extend(pack_report.members);
    }

    // Free-form: single file path, or loose tests outside pack directories.
    let freeform_report = run_freeform_report(root, &pack_dirs)?;
    combined.members.extend(freeform_report.members);

    if combined.members.is_empty() && pack_manifests.is_empty() {
        return Ok(TestResult {
            passed: 0,
            failed: 0,
            skipped: 0,
            tests: vec![],
            warning: Some(format!(
                "No pack manifests or `*_test.rego` files found under '{}'",
                root.display()
            )),
            files: None,
        });
    }

    Ok(report_to_test_result(&combined))
}

/// Whether `path` names a pack manifest file (any `.yaml` / `.yml`, or the
/// canonical `pack.yaml`).
fn is_manifest_file(path: &Path) -> bool {
    if path.file_name().is_some_and(|n| n == MANIFEST_FILENAME) {
        return true;
    }
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
}

/// Collect pack manifests: the path itself, or immediate child pack dirs.
fn resolve_pack_manifests(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        // Explicit manifest file (`.yaml` or `.yml`, matching `policy validate`).
        if is_manifest_file(root) {
            return Ok(vec![root.to_path_buf()]);
        }
        // A single .rego file is free-form, not a pack.
        return Ok(vec![]);
    }

    let direct = root.join(MANIFEST_FILENAME);
    if direct.is_file() {
        return Ok(vec![direct]);
    }

    // Policies root: one level of child packs (`.anvil/policies/<pack>/pack.yaml`).
    let mut manifests = Vec::new();
    let entries = std::fs::read_dir(root)
        .with_context(|| format!("reading policy test directory {}", root.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading entry under {}", root.display()))?;
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join(MANIFEST_FILENAME);
            if candidate.is_file() {
                manifests.push(candidate);
            }
        }
    }
    manifests.sort();
    Ok(manifests)
}

fn run_pack_manifests_report(manifests: &[PathBuf]) -> Result<TestRunReport> {
    let mut combined = TestRunReport::default();
    for manifest_path in manifests {
        let base_dir = manifest_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let manifest = load_manifest(manifest_path)
            .with_context(|| format!("loading pack manifest {}", manifest_path.display()))?;
        let report = run_pack_tests(&manifest, base_dir)
            .with_context(|| format!("running tests for pack {}", manifest_path.display()))?;
        combined.members.extend(report.members);
    }
    Ok(combined)
}

/// True when `path` is inside (or equal to) any of `pack_dirs`.
fn under_any_pack(path: &Path, pack_dirs: &[PathBuf]) -> bool {
    pack_dirs.iter().any(|pack| path.starts_with(pack))
}

fn run_freeform_report(root: &Path, pack_dirs: &[PathBuf]) -> Result<TestRunReport> {
    let mut test_files = discover_test_rego_files(root);
    // Skip tests owned by a pack the pack runner already executes.
    if !pack_dirs.is_empty() {
        test_files.retain(|p| !under_any_pack(p, pack_dirs));
    }
    if test_files.is_empty() {
        return Ok(TestRunReport::default());
    }

    // Prefer the pack runner when we can form policy/test pairs under a
    // synthetic manifest (keeps semantics aligned with `policy validate`).
    let mut members = Vec::new();
    let mut paired_policies: Vec<PolicyEntry> = Vec::new();
    let mut orphans: Vec<PathBuf> = Vec::new();

    // When root is a single test file, base is its parent.
    let base_dir = if root.is_file() {
        root.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    } else {
        root.to_path_buf()
    };

    for test_abs in &test_files {
        let Some(policy_abs) = policy_sibling(test_abs) else {
            orphans.push(test_abs.clone());
            continue;
        };
        if policy_abs.is_file() {
            let rel = pathdiff_rel(&base_dir, &policy_abs).unwrap_or_else(|| policy_abs.clone());
            let id = policy_abs
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("policy")
                .to_string();
            paired_policies.push(PolicyEntry {
                path: rel,
                metadata: synthetic_metadata(&id),
            });
        } else {
            orphans.push(test_abs.clone());
        }
    }

    if !paired_policies.is_empty() {
        // Deduplicate by path (stable order).
        paired_policies.sort_by(|a, b| a.path.cmp(&b.path));
        paired_policies.dedup_by(|a, b| a.path == b.path);
        let manifest = PackManifest {
            id: "freeform".into(),
            name: "Free-form policy tests".into(),
            version: "0.0.0".into(),
            description: "Ad-hoc policy test run without pack.yaml".into(),
            owner: "local".into(),
            policies: paired_policies,
        };
        // Synthetic manifests are in-memory; metadata is complete via
        // synthetic_metadata. run_pack_tests does not call validate().
        let report = run_pack_tests(&manifest, &base_dir)
            .context("running free-form paired policy tests")?;
        members.extend(report.members);
    }

    for orphan in orphans {
        members.push(run_orphan_test_file(&orphan)?);
    }

    Ok(TestRunReport { members })
}

fn synthetic_metadata(id: &str) -> PolicyMetadata {
    PolicyMetadata {
        id: id.to_string(),
        title: id.to_string(),
        severity: Some(PolicySeverity::Medium),
        owner: "local".into(),
        rationale: "free-form policy test".into(),
        scope: "*".into(),
        tags: vec!["test".into()],
    }
}

/// Evaluate a self-contained `*_test.rego` with no sibling policy source.
fn run_orphan_test_file(test_abs: &Path) -> Result<MemberTestResult> {
    let mut result = MemberTestResult {
        policy_id: test_abs
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("orphan")
            .to_string(),
        policy_path: test_abs.to_path_buf(),
        test_path: Some(test_abs.to_path_buf()),
        test_package: None,
        load_error: None,
        outcomes: Vec::new(),
    };

    let test_source = match std::fs::read_to_string(test_abs) {
        Ok(s) => s,
        Err(e) => {
            result.load_error = Some(format!(
                "could not read test file `{}`: {e}",
                test_abs.display()
            ));
            return Ok(result);
        }
    };

    let Some(package) = parse_test_package(&test_source) else {
        result.load_error = Some(format!(
            "test file `{}` has no `package` declaration",
            test_abs.display()
        ));
        return Ok(result);
    };
    result.test_package = Some(package.clone());

    let rules = discover_test_rules_from_source(&test_source);

    let mut engine = Engine::new(EngineConfig::default())
        .map_err(|e| anyhow::anyhow!("could not construct policy engine: {e}"))?;

    let label = test_abs.to_string_lossy().into_owned();
    if let Err(e) = engine.add_policy(label, test_source) {
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

fn evaluate_rule(
    engine: &mut Engine,
    input: &PolicyInput,
    query: &str,
    rule: String,
) -> TestOutcome {
    match engine.eval(input, query) {
        Ok(eval) => match eval.value {
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

fn report_to_test_result(report: &TestRunReport) -> TestResult {
    // Surface enforcement issues (missing tests, load errors, no rules) as
    // failures even when no per-rule outcomes were produced.
    let issues = enforce_tests(report);

    let mut tests = Vec::new();
    let mut passed = 0u32;
    let mut failed = 0u32;

    for member in &report.members {
        let prefix = if member.policy_id.is_empty() {
            member.test_path.as_ref().map_or_else(
                || member.policy_path.display().to_string(),
                |p| p.display().to_string(),
            )
        } else {
            member.policy_id.clone()
        };

        if let Some(load_error) = &member.load_error {
            failed = failed.saturating_add(1);
            tests.push(TestCase {
                name: prefix.clone(),
                passed: false,
                message: load_error.clone(),
            });
            continue;
        }

        if member.outcomes.is_empty() {
            // enforce_tests will flag this; record a synthetic failure so the
            // aggregate fails closed rather than looking empty-green.
            failed = failed.saturating_add(1);
            tests.push(TestCase {
                name: prefix.clone(),
                passed: false,
                message: "no test_* rules discovered".into(),
            });
            continue;
        }

        for outcome in &member.outcomes {
            let name = format!("{prefix}::{}", outcome.rule);
            if outcome.passed {
                passed = passed.saturating_add(1);
                tests.push(TestCase {
                    name,
                    passed: true,
                    message: String::new(),
                });
            } else {
                failed = failed.saturating_add(1);
                tests.push(TestCase {
                    name,
                    passed: false,
                    message: outcome
                        .detail
                        .clone()
                        .unwrap_or_else(|| "test failed".into()),
                });
            }
        }
    }

    // Count enforcement-only issues that were not already folded into outcomes
    // (e.g. MissingTestFile on a pack member with no sibling test).
    for issue in &issues {
        // Skip issues already reflected as outcome/load failures above.
        let already = tests.iter().any(|t| {
            !t.passed
                && issue
                    .policy_id
                    .as_ref()
                    .is_some_and(|id| t.name.starts_with(id.as_str()))
        });
        if already {
            continue;
        }
        // Missing-test-file / no-tests on members with no outcomes already
        // produced a synthetic failure above when outcomes were empty. Only
        // add when the member had outcomes but enforce still raised something
        // (package naming is a warning — skip).
        if issue.severity == anvil_policy_engine::pack::IssueSeverity::Warning {
            continue;
        }
        // Missing test file members never entered the outcomes loop with a
        // load_error; they have test_path None and empty outcomes — already
        // counted. Double-check via policy id presence in tests.
        if tests.iter().any(|t| {
            issue
                .policy_id
                .as_ref()
                .is_some_and(|id| t.name == *id || t.name.starts_with(&format!("{id}::")))
        }) {
            continue;
        }
        failed = failed.saturating_add(1);
        tests.push(TestCase {
            name: issue.policy_id.clone().unwrap_or_else(|| "policy".into()),
            passed: false,
            message: issue.message.clone(),
        });
    }

    TestResult {
        passed,
        failed,
        skipped: 0,
        tests,
        warning: None,
        files: None,
    }
}

/// Discover `*_test.rego` files under `root` (or the file itself).
fn discover_test_rego_files(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        if is_test_rego(root) {
            return vec![root.to_path_buf()];
        }
        return Vec::new();
    }

    let mut files: Vec<PathBuf> = ignore::WalkBuilder::new(root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .build()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .map(|e| e.path().to_path_buf())
        .filter(|p| is_test_rego(p))
        .collect();
    files.sort();
    files
}

fn is_test_rego(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with("_test.rego"))
}

/// Sibling policy for `foo_test.rego` → `foo.rego`.
fn policy_sibling(test_path: &Path) -> Option<PathBuf> {
    let name = test_path.file_name()?.to_str()?;
    let stem = name.strip_suffix("_test.rego")?;
    Some(test_path.with_file_name(format!("{stem}.rego")))
}

/// Lexical relative path of `abs` under `base`, or `None` when `abs` is not
/// nested under `base`. Prefer `strip_prefix` on the raw paths first, then on
/// canonical forms when both resolve.
fn pathdiff_rel(base: &Path, abs: &Path) -> Option<PathBuf> {
    if let Ok(rel) = abs.strip_prefix(base) {
        return Some(rel.to_path_buf());
    }
    let base_c = base.canonicalize().ok()?;
    let abs_c = abs.canonicalize().ok()?;
    abs_c.strip_prefix(base_c).ok().map(Path::to_path_buf)
}

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

fn discover_test_rules_from_source(source: &str) -> Vec<String> {
    let stripped = strip_raw_strings(source);
    let mut names = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
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

// Shared with mod.rs walker tests / list-files behaviour: all files, not only
// `*_test.rego`, matching the historical discovery surface for `--list-files`.
pub(super) fn policy_test_file_walker(test_path: &str) -> ignore::Walk {
    ignore::WalkBuilder::new(test_path)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .build()
}

pub(super) fn collect_policy_test_files(test_path: &str) -> Vec<String> {
    let mut files: Vec<String> = policy_test_file_walker(test_path)
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const MANIFEST: &str = "
id: pack
name: Pack
version: 1.0.0
description: Guardrails.
owner: platform-security
policies:
  - path: policies/a.rego
    metadata:
      id: policy-a
      title: Policy A
      severity: high
      owner: platform-security
      rationale: A guards A.
      scope: src/**
      tags: [security]
";

    const POLICY: &str = "package a\nimport rego.v1\n\nallow if input.x == 1\n";

    fn write(base: &Path, rel: &str, body: &str) {
        let path = base.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create dirs");
        }
        std::fs::write(path, body).expect("write file");
    }

    fn failing_pack() -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "pack.yaml", MANIFEST);
        write(dir.path(), "policies/a.rego", POLICY);
        write(
            dir.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ntest_wrong := false\n",
        );
        dir
    }

    fn passing_pack() -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "pack.yaml", MANIFEST);
        write(dir.path(), "policies/a.rego", POLICY);
        write(
            dir.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ntest_allow if data.a.allow with input as {\"x\": 1}\n",
        );
        dir
    }

    #[test]
    fn policy_test_failing_pack_returns_already_reported() {
        let dir = failing_pack();
        let err = run(
            Some(dir.path().to_str().expect("utf8 path")),
            false,
            &GlobalArgs::default(),
        )
        .expect_err("failing pack must exit non-zero");
        assert!(
            err.is::<output::AlreadyReported>(),
            "expected AlreadyReported, got {err:?}"
        );
    }

    #[test]
    fn policy_test_passing_pack_exits_ok() {
        let dir = passing_pack();
        run(
            Some(dir.path().to_str().expect("utf8 path")),
            false,
            &GlobalArgs::default(),
        )
        .expect("passing pack must exit 0");
    }

    #[test]
    fn policy_test_freeform_failing_test_returns_already_reported() {
        // Reproduction: directory with a failing `*_test.rego` and no pack.yaml.
        let dir = TempDir::new().expect("temp dir");
        write(
            dir.path(),
            "lonely_test.rego",
            "package lonely_test\nimport rego.v1\n\ntest_always_fails := false\n",
        );
        let err = run(
            Some(dir.path().to_str().expect("utf8 path")),
            false,
            &GlobalArgs::default(),
        )
        .expect_err("failing free-form test must exit non-zero");
        assert!(
            err.is::<output::AlreadyReported>(),
            "expected AlreadyReported, got {err:?}"
        );
    }

    #[test]
    fn policy_test_freeform_passing_exits_ok() {
        let dir = TempDir::new().expect("temp dir");
        write(
            dir.path(),
            "ok_test.rego",
            "package ok_test\nimport rego.v1\n\ntest_ok if true\n",
        );
        run(
            Some(dir.path().to_str().expect("utf8 path")),
            false,
            &GlobalArgs::default(),
        )
        .expect("passing free-form test must exit 0");
    }

    #[test]
    fn policy_test_execute_populates_failures() {
        let dir = failing_pack();
        let result = execute_tests(dir.path()).expect("execute");
        assert!(result.failed > 0, "expected failures: {result:?}");
        assert!(
            result.tests.iter().any(|t| !t.passed),
            "expected a failing case: {result:?}"
        );
    }

    #[test]
    fn policy_test_list_files_does_not_execute() {
        let dir = failing_pack();
        // --list-files must not fail on a failing pack; it only lists.
        run(
            Some(dir.path().to_str().expect("utf8 path")),
            true,
            &GlobalArgs::default(),
        )
        .expect("list-files is discovery-only");
    }

    #[test]
    fn policy_test_policies_root_runs_packs_and_loose_tests() {
        // Policies root with a child pack plus a loose failing free-form test.
        let root = TempDir::new().expect("temp dir");
        let pack_dir = root.path().join("anvil-baseline");
        write(&pack_dir, "pack.yaml", MANIFEST);
        write(&pack_dir, "policies/a.rego", POLICY);
        write(
            &pack_dir,
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ntest_allow if data.a.allow with input as {\"x\": 1}\n",
        );
        write(
            root.path(),
            "loose_test.rego",
            "package loose_test\nimport rego.v1\n\ntest_loose_fails := false\n",
        );

        let err = run(
            Some(root.path().to_str().expect("utf8 path")),
            false,
            &GlobalArgs::default(),
        )
        .expect_err("loose failing test must fail the root run");
        assert!(err.is::<output::AlreadyReported>(), "got {err:?}");

        let result = execute_tests(root.path()).expect("execute");
        assert!(
            result
                .tests
                .iter()
                .any(|t| t.passed && t.name.contains("test_allow")),
            "pack tests must still run: {result:?}"
        );
        assert!(
            result
                .tests
                .iter()
                .any(|t| !t.passed && t.name.contains("test_loose_fails")),
            "loose free-form test must run: {result:?}"
        );
    }

    #[test]
    fn policy_test_yml_manifest_path_is_accepted() {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "pack.yml", MANIFEST);
        write(dir.path(), "policies/a.rego", POLICY);
        write(
            dir.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ntest_allow if data.a.allow with input as {\"x\": 1}\n",
        );
        let manifest = dir.path().join("pack.yml");
        run(
            Some(manifest.to_str().expect("utf8 path")),
            false,
            &GlobalArgs::default(),
        )
        .expect(".yml pack manifest must execute");
    }
}
