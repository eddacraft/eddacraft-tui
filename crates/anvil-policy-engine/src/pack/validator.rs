//! Policy pack validator (POLVAL-003).
//!
//! Given a parsed [`PackManifest`] and the manifest's base directory, produce a
//! [`ValidationReport`]: an ordered, machine-readable list of issues covering
//! pack structure, metadata completeness, and policy-id uniqueness. The report
//! is remediation-first — every issue carries guidance on how to fix it — so it
//! can back both a human summary and the future CLI `--json` output
//! (POLVAL-005).
//!
//! Scope of this increment: existence and metadata only. This module performs
//! no `regorus` evaluation and no Rego parsing — checking that a `.rego` file
//! *compiles* belongs to a later item. Test *enforcement* (a missing test is a
//! hard failure) is POLVAL-004; here a missing sibling test is reported as a
//! warning, not an error.
//!
//! Validation does not short-circuit: the report aggregates issues across every
//! member so an author sees the whole picture in one pass. Iteration follows the
//! manifest's declared member order, so the report is deterministic.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::pack::manifest::PackManifest;

/// Whether an issue blocks the pack from validating.
///
/// This is deliberately a dedicated two-level enum rather than a reuse of
/// [`crate::pack::PolicySeverity`]. `PolicySeverity` (low/medium/high/critical)
/// ranks the impact of the findings a *policy* emits; an issue's severity
/// answers a different question — does this problem stop the *pack* from
/// loading. Mapping validation issues onto the four-band policy scale would
/// conflate the two axes. Per ADR-002 (warnings over blocks), only
/// [`IssueSeverity::Error`] fails validation; a [`IssueSeverity::Warning`] is
/// advisory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Blocks validation: [`ValidationReport::is_valid`] is `false`.
    Error,
    /// Advisory only; does not block validation.
    Warning,
}

/// Stable, machine-readable classification of a validation issue.
///
/// The wire form is kebab-case (`missing-policy-file`, …) and is part of the
/// report contract consumed by CI and tooling, so variants are added, never
/// renamed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueCode {
    /// A member's `.rego` source is absent under the pack directory.
    MissingPolicyFile,
    /// A member has no sibling `*_test.rego`. Advisory in [`validate_pack`]
    /// (POLVAL-003); escalated to error-class under test enforcement
    /// (POLVAL-004, [`crate::pack::enforce_tests`]).
    MissingTestFile,
    /// Two or more members share a policy id.
    DuplicatePolicyId,
    /// A member's metadata failed completeness validation.
    MetadataIncomplete,
    /// A member's test file exists but declares no `test_*` rules
    /// (POLVAL-004).
    NoTestsDiscovered,
    /// A discovered `test_*` rule failed — evaluated to false, was undefined,
    /// or raised an evaluation error (POLVAL-004).
    PolicyTestFailed,
    /// A test file's package does not follow the `<name>_test` convention
    /// (POLVAL-004). Advisory only; the tests still run.
    TestPackageNaming,
}

/// A single structured validation issue.
///
/// Serialises cleanly for the `--json` report: optional attribution fields are
/// omitted when absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Stable machine-readable code.
    pub code: IssueCode,
    /// Whether this issue blocks validation.
    pub severity: IssueSeverity,
    /// The offending policy id, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    /// The offending path (relative to the pack directory), when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// Human-readable description of the problem.
    pub message: String,
    /// How to fix it — remediation-first guidance.
    pub remediation: String,
}

/// The outcome of validating a pack: an ordered list of issues.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Issues in deterministic order (manifest member order, then cross-cutting
    /// checks).
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// A pack is valid when it has no [`IssueSeverity::Error`] issues. Warnings
    /// (e.g. a missing test file in this increment) do not fail validation.
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }

    /// The error-class issues only.
    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IssueSeverity::Error)
    }
}

/// Validate a parsed pack against its on-disk layout.
///
/// `base_dir` is the manifest's own directory; member `.rego` paths are
/// resolved beneath it for existence checks. The manifest is assumed to have
/// come from [`crate::pack::load_manifest`] (so member paths are already known
/// not to escape `base_dir`); this function only *reads* directory entries it is
/// pointed at and never walks beyond a member's declared location.
///
/// Never returns an [`Err`]: an absent file is a reported issue, not a load
/// failure. The report aggregates every issue found.
#[must_use]
pub fn validate_pack(manifest: &PackManifest, base_dir: &Path) -> ValidationReport {
    let mut issues = Vec::new();

    for entry in &manifest.policies {
        let policy_id = trimmed_id(&entry.metadata.id);

        // Metadata completeness. `validate()` reports the first missing field
        // per member; we continue to the next member regardless, so the report
        // is not short-circuited across the pack.
        if let Err(err) = entry.metadata.validate() {
            issues.push(ValidationIssue {
                code: IssueCode::MetadataIncomplete,
                severity: IssueSeverity::Error,
                policy_id: policy_id.clone(),
                path: Some(entry.path.clone()),
                message: err.to_string(),
                remediation: "Complete the required metadata for this policy \
                              (id, title, severity, owner, rationale, scope, tags)."
                    .to_string(),
            });
        }

        // The member's `.rego` source must exist under the pack directory.
        if !base_dir.join(&entry.path).is_file() {
            issues.push(ValidationIssue {
                code: IssueCode::MissingPolicyFile,
                severity: IssueSeverity::Error,
                policy_id: policy_id.clone(),
                path: Some(entry.path.clone()),
                message: format!(
                    "policy source `{}` is referenced by the manifest but does not exist",
                    entry.path.display()
                ),
                remediation: "Add the `.rego` file at the referenced path, or correct \
                              the manifest to point at the real location."
                    .to_string(),
            });
        }

        // A sibling `*_test.rego` is expected but only advised here — test
        // enforcement (a hard failure) is POLVAL-004.
        if let Some(test_path) = test_sibling(&entry.path)
            && !base_dir.join(&test_path).is_file()
        {
            issues.push(ValidationIssue {
                code: IssueCode::MissingTestFile,
                severity: IssueSeverity::Warning,
                policy_id: policy_id.clone(),
                path: Some(test_path.clone()),
                message: format!(
                    "no test found for policy `{}`; expected `{}`",
                    entry.path.display(),
                    test_path.display()
                ),
                remediation: "Add a sibling `*_test.rego` covering this policy so \
                              its behaviour is pinned."
                    .to_string(),
            });
        }
    }

    issues.extend(duplicate_id_issues(manifest));

    ValidationReport { issues }
}

/// Report every member whose (trimmed, non-blank) policy id has already been
/// seen earlier in manifest order. Blank ids are left to metadata completeness.
fn duplicate_id_issues(manifest: &PackManifest) -> Vec<ValidationIssue> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut issues = Vec::new();
    for entry in &manifest.policies {
        let id = entry.metadata.id.trim();
        if id.is_empty() {
            continue;
        }
        if !seen.insert(id) {
            issues.push(ValidationIssue {
                code: IssueCode::DuplicatePolicyId,
                severity: IssueSeverity::Error,
                policy_id: Some(id.to_string()),
                path: Some(entry.path.clone()),
                message: format!("policy id `{id}` is declared by more than one member"),
                remediation: "Give each policy in the pack a unique id.".to_string(),
            });
        }
    }
    issues
}

/// The expected sibling test path for a policy: `foo.rego` → `foo_test.rego`
/// in the same directory. Returns `None` for a path with no usable file stem.
pub(crate) fn test_sibling(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let name = format!("{stem}_test.rego");
    Some(match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(name),
        _ => PathBuf::from(name),
    })
}

/// The trimmed id, or `None` when blank — so an unattributed issue omits the
/// field rather than carrying an empty string.
fn trimmed_id(id: &str) -> Option<String> {
    let id = id.trim();
    (!id.is_empty()).then(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A manifest with two well-formed members referencing `policies/a.rego`
    /// and `policies/b.rego`.
    const TWO_MEMBER_MANIFEST: &str = r"
id: baseline-pack
name: Baseline Pack
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
      rationale: A exists to guard A.
      scope: src/**
      tags: [security]
  - path: policies/b.rego
    metadata:
      id: policy-b
      title: Policy B
      severity: medium
      owner: dx
      rationale: B exists to guard B.
      scope: crates/**
      tags: [quality]
";

    fn parse(manifest: &str) -> PackManifest {
        serde_yaml::from_str(manifest).expect("parse manifest fixture")
    }

    fn touch(base: &Path, rel: &str) {
        let path = base.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create dirs");
        }
        std::fs::write(path, "package p\nimport rego.v1\n").expect("write file");
    }

    #[test]
    fn policy_pack_validator_clean_pack_is_valid() {
        let dir = TempDir::new().expect("temp dir");
        for name in ["a", "b"] {
            touch(dir.path(), &format!("policies/{name}.rego"));
            touch(dir.path(), &format!("policies/{name}_test.rego"));
        }
        let report = validate_pack(&parse(TWO_MEMBER_MANIFEST), dir.path());
        assert!(report.is_valid(), "unexpected issues: {:?}", report.issues);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn policy_pack_validator_missing_policy_file_is_error() {
        let dir = TempDir::new().expect("temp dir");
        // Only `a` exists; `b.rego` is absent.
        touch(dir.path(), "policies/a.rego");
        touch(dir.path(), "policies/a_test.rego");
        touch(dir.path(), "policies/b_test.rego");
        let report = validate_pack(&parse(TWO_MEMBER_MANIFEST), dir.path());
        assert!(!report.is_valid());
        let missing: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.code == IssueCode::MissingPolicyFile)
            .collect();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].policy_id.as_deref(), Some("policy-b"));
        assert_eq!(missing[0].severity, IssueSeverity::Error);
    }

    #[test]
    fn policy_pack_validator_missing_test_file_is_warning_not_error() {
        let dir = TempDir::new().expect("temp dir");
        // Both policies present, neither has a test sibling.
        touch(dir.path(), "policies/a.rego");
        touch(dir.path(), "policies/b.rego");
        let report = validate_pack(&parse(TWO_MEMBER_MANIFEST), dir.path());
        let warnings: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.code == IssueCode::MissingTestFile)
            .collect();
        assert_eq!(warnings.len(), 2);
        assert!(
            warnings
                .iter()
                .all(|w| w.severity == IssueSeverity::Warning)
        );
        // Warnings alone must not fail validation (ADR-002).
        assert!(report.is_valid());
        assert_eq!(
            warnings[0].path.as_deref(),
            Some(Path::new("policies/a_test.rego"))
        );
    }

    #[test]
    fn policy_pack_validator_duplicate_ids_reported() {
        let dir = TempDir::new().expect("temp dir");
        for name in ["a", "b"] {
            touch(dir.path(), &format!("policies/{name}.rego"));
            touch(dir.path(), &format!("policies/{name}_test.rego"));
        }
        let manifest = parse(&TWO_MEMBER_MANIFEST.replace("id: policy-b", "id: policy-a"));
        let report = validate_pack(&manifest, dir.path());
        let dups: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.code == IssueCode::DuplicatePolicyId)
            .collect();
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].policy_id.as_deref(), Some("policy-a"));
        assert!(!report.is_valid());
    }

    #[test]
    fn policy_pack_validator_incomplete_metadata_reported() {
        let dir = TempDir::new().expect("temp dir");
        for name in ["a", "b"] {
            touch(dir.path(), &format!("policies/{name}.rego"));
            touch(dir.path(), &format!("policies/{name}_test.rego"));
        }
        // Blank out policy-a's rationale — parses (serde default), fails validate.
        let manifest = parse(
            &TWO_MEMBER_MANIFEST.replace("rationale: A exists to guard A.", "rationale: \"\""),
        );
        let report = validate_pack(&manifest, dir.path());
        let incomplete: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.code == IssueCode::MetadataIncomplete)
            .collect();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].policy_id.as_deref(), Some("policy-a"));
        assert_eq!(incomplete[0].severity, IssueSeverity::Error);
        assert!(!report.is_valid());
    }

    #[test]
    fn policy_pack_validator_aggregates_all_issues_without_short_circuit() {
        let dir = TempDir::new().expect("temp dir");
        // Nothing on disk: both members miss their `.rego` and their test.
        let report = validate_pack(&parse(TWO_MEMBER_MANIFEST), dir.path());
        // Two missing-policy-file errors + two missing-test-file warnings.
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|i| i.code == IssueCode::MissingPolicyFile)
                .count(),
            2,
            "both members must be reported, not just the first: {:?}",
            report.issues
        );
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|i| i.code == IssueCode::MissingTestFile)
                .count(),
            2
        );
        assert_eq!(report.errors().count(), 2);
        assert!(!report.is_valid());
    }

    #[test]
    fn policy_pack_validator_report_serialises_to_json() {
        let dir = TempDir::new().expect("temp dir");
        let report = validate_pack(&parse(TWO_MEMBER_MANIFEST), dir.path());
        let json = serde_json::to_string(&report).expect("serialise report");
        assert!(
            json.contains("missing-policy-file"),
            "kebab-case code must survive serialisation: {json}"
        );
        // Round-trips back to an equal report.
        let restored: ValidationReport = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored, report);
    }
}
