//! `anvil policy validate` — validate a policy pack before it loads.
//!
//! Runs the pack admission pipeline over a manifest: load, structural and
//! metadata validation, then test execution and enforcement. Emits a
//! remediation-first human report by default, or the machine-readable
//! validation report under `--json`. Exit is non-zero only when an error-class
//! issue is present (warnings never fail) or when the manifest cannot be read.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;

use anvil_policy_engine::pack::{
    IssueCode, IssueSeverity, ValidationReport, enforce_tests, load_manifest, run_pack_tests,
    validate_pack,
};

use crate::GlobalArgs;
use crate::output;

/// Filename resolved when the path argument is a directory rather than a file.
/// Matches the convention used by the pack manifest fixtures.
const MANIFEST_FILENAME: &str = "pack.yaml";

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Pack manifest file, or a directory containing `pack.yaml`.
    path: String,
}

pub fn run(args: &ValidateArgs, global: &GlobalArgs) -> Result<()> {
    let manifest_path = resolve_manifest_path(Path::new(&args.path))?;
    let base_dir = manifest_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let report = assemble_report(&manifest_path, base_dir)?;

    if global.json {
        output::json::print(&report)?;
    } else {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        write_report(&mut out, &report, &manifest_path).context("writing report")?;
    }

    if !report.is_valid() {
        return Err(output::AlreadyReported.into());
    }
    Ok(())
}

/// Resolve the path argument to a concrete manifest file. A directory resolves
/// to its `pack.yaml`. A path that does not exist is an operational failure
/// (distinct from a validation issue).
fn resolve_manifest_path(path: &Path) -> Result<PathBuf> {
    if path.is_dir() {
        let candidate = path.join(MANIFEST_FILENAME);
        if !candidate.is_file() {
            bail!(
                "no `{MANIFEST_FILENAME}` found in directory {}",
                path.display()
            );
        }
        Ok(candidate)
    } else if path.is_file() {
        Ok(path.to_path_buf())
    } else {
        bail!("pack manifest path not found: {}", path.display());
    }
}

/// Run the full pack-admission pipeline and fold every issue into one report.
///
/// The structural validator emits a pre-enforcement warning for a missing test
/// file; test enforcement re-emits that as an error, so the warning is dropped
/// here to avoid double-reporting the same fact at two severities.
fn assemble_report(manifest_path: &Path, base_dir: &Path) -> Result<ValidationReport> {
    let manifest = load_manifest(manifest_path)
        .with_context(|| format!("loading pack manifest {}", manifest_path.display()))?;

    let mut report = validate_pack(&manifest, base_dir);
    report
        .issues
        .retain(|issue| issue.code != IssueCode::MissingTestFile);

    let test_report = run_pack_tests(&manifest, base_dir).context("running policy pack tests")?;
    report.issues.extend(enforce_tests(&test_report));

    Ok(report)
}

/// Write the remediation-first human report to `out`.
fn write_report(
    out: &mut impl Write,
    report: &ValidationReport,
    manifest_path: &Path,
) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "Policy pack validation: {}", manifest_path.display())?;

    if report.issues.is_empty() {
        writeln!(out, "  OK — pack is valid, no issues found.")?;
        return Ok(());
    }

    for issue in &report.issues {
        let tag = match issue.severity {
            IssueSeverity::Error => "ERROR",
            IssueSeverity::Warning => "WARN ",
        };
        let scope = issue
            .policy_id
            .as_deref()
            .map(|id| format!(" ({id})"))
            .unwrap_or_default();
        writeln!(
            out,
            "  [{tag}] {}{scope}: {}",
            code_label(issue.code),
            issue.message
        )?;
        writeln!(out, "         fix: {}", issue.remediation)?;
    }

    let errors = report
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .count();
    let warnings = report.issues.len() - errors;

    writeln!(out)?;
    if report.is_valid() {
        writeln!(out, "  Pack is valid — {warnings} warning(s), no errors.")?;
    } else {
        writeln!(
            out,
            "  Pack is invalid — {errors} error(s), {warnings} warning(s)."
        )?;
    }
    Ok(())
}

/// Stable kebab-case label for an issue code, matching the `--json` wire form.
fn code_label(code: IssueCode) -> &'static str {
    match code {
        IssueCode::MissingPolicyFile => "missing-policy-file",
        IssueCode::MissingTestFile => "missing-test-file",
        IssueCode::DuplicatePolicyId => "duplicate-policy-id",
        IssueCode::MetadataIncomplete => "metadata-incomplete",
        IssueCode::NoTestsDiscovered => "no-tests-discovered",
        IssueCode::PolicyTestFailed => "policy-test-failed",
        IssueCode::TestPackageNaming => "test-package-naming",
    }
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

    /// A pack whose single member passes its tests, with a manifest at
    /// `pack.yaml`. Returns the temp dir and the manifest path.
    fn valid_pack(test_package: &str) -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "pack.yaml", MANIFEST);
        write(dir.path(), "policies/a.rego", POLICY);
        write(
            dir.path(),
            "policies/a_test.rego",
            &format!(
                "package {test_package}\nimport rego.v1\n\ntest_allow if data.a.allow with input as {{\"x\": 1}}\n"
            ),
        );
        dir
    }

    #[test]
    fn policy_validate_valid_pack_exits_ok() {
        let dir = valid_pack("a_test");
        let manifest = dir.path().join("pack.yaml");
        let report = assemble_report(&manifest, dir.path()).expect("assemble");
        assert!(report.is_valid(), "unexpected issues: {:?}", report.issues);

        let args = ValidateArgs {
            path: manifest.to_string_lossy().into_owned(),
        };
        assert!(run(&args, &GlobalArgs::default()).is_ok());
    }

    #[test]
    fn policy_validate_error_issue_exits_nonzero_and_prints() {
        let dir = TempDir::new().expect("temp dir");
        write(dir.path(), "pack.yaml", MANIFEST);
        write(dir.path(), "policies/a.rego", POLICY);
        // A failing test rule (explicit false) makes the pack invalid.
        write(
            dir.path(),
            "policies/a_test.rego",
            "package a_test\nimport rego.v1\n\ntest_wrong := false\n",
        );
        let manifest = dir.path().join("pack.yaml");

        let report = assemble_report(&manifest, dir.path()).expect("assemble");
        assert!(!report.is_valid());

        // The issue is on the rendered report (what goes to stdout).
        let mut buf = Vec::new();
        write_report(&mut buf, &report, &manifest).expect("write");
        let text = String::from_utf8(buf).expect("utf8");
        assert!(text.contains("ERROR"), "{text}");
        assert!(text.contains("policy-test-failed"), "{text}");
        assert!(text.contains("test_wrong"), "{text}");

        // The command exits non-zero via the AlreadyReported sentinel.
        let args = ValidateArgs {
            path: manifest.to_string_lossy().into_owned(),
        };
        let err = run(&args, &GlobalArgs::default()).expect_err("must fail");
        assert!(err.is::<output::AlreadyReported>(), "got {err:?}");
    }

    #[test]
    fn policy_validate_json_round_trips() {
        let dir = valid_pack("a_test");
        let manifest = dir.path().join("pack.yaml");
        let report = assemble_report(&manifest, dir.path()).expect("assemble");

        let json = serde_json::to_string(&report).expect("serialise");
        let restored: ValidationReport = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored, report);
    }

    #[test]
    fn policy_validate_warnings_only_pack_exits_ok() {
        // A test package that does not end in `_test` yields only a naming
        // warning; the pack stays valid and the command exits 0.
        let dir = valid_pack("a");
        let manifest = dir.path().join("pack.yaml");
        let report = assemble_report(&manifest, dir.path()).expect("assemble");
        assert!(!report.issues.is_empty());
        assert!(
            report
                .issues
                .iter()
                .all(|i| i.severity == IssueSeverity::Warning)
        );
        assert!(report.is_valid());

        let args = ValidateArgs {
            path: manifest.to_string_lossy().into_owned(),
        };
        assert!(run(&args, &GlobalArgs::default()).is_ok());
    }

    #[test]
    fn policy_validate_directory_path_resolves_pack_yaml() {
        let dir = valid_pack("a_test");
        let resolved = resolve_manifest_path(dir.path()).expect("resolve dir");
        assert_eq!(resolved, dir.path().join("pack.yaml"));
    }

    #[test]
    fn policy_validate_missing_path_is_operational_error() {
        let dir = TempDir::new().expect("temp dir");
        let missing = dir.path().join("nope.yaml");
        let args = ValidateArgs {
            path: missing.to_string_lossy().into_owned(),
        };
        let err = run(&args, &GlobalArgs::default()).expect_err("must fail");
        // Operational failure, not a reported validation issue.
        assert!(!err.is::<output::AlreadyReported>());
        assert!(format!("{err:#}").contains("not found"));
    }
}
