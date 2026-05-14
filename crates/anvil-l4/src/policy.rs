use std::path::Path;

use anvil_config::{ConfigFormat, ParseError, parse_str};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// What an arriving commit must carry to be accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    /// Accept either a verified L3 witness OR a successful L4
    /// re-validation. Default for normal branches.
    L4OrL3,
    /// Only accept L4 re-validation; ignore L3 witnesses entirely.
    /// Useful for bot-only branches (Dependabot/Renovate) where the
    /// signal is "the bot doesn't run Anvil, so check it ourselves."
    L4Only,
    /// Only accept verified L3 witnesses; refuse anything else.
    /// Strict mode for high-assurance branches.
    L3Only,
}

/// Behaviour when a pushed commit has no L3 witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnNoWitness {
    /// Re-run validation server-side; the L4 witness (in
    /// `refs/notes/anvil-l4`) records the outcome.
    ValidateAtL4,
    /// Reject the push outright. Strict mode for branches that
    /// expect every commit to come with witness evidence already.
    Reject,
    /// Allow the push without further checks. Escape hatch for
    /// migration windows; not recommended for protected branches.
    Allow,
}

/// Behaviour when validation produces a block-level finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnBlock {
    /// Refuse the push.
    Reject,
    /// Allow the push; the block-level finding is recorded in the
    /// L4 witness but doesn't gate. Use sparingly.
    Allow,
}

/// Behaviour when validation produces a warn-level finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnWarn {
    /// Allow the push; the warning is recorded in the L4 witness.
    /// Default per CLAUDE.md "warnings over blocks."
    Allow,
    /// Promote warnings to blocks for this branch. Useful for
    /// high-assurance branches.
    Reject,
}

/// One per-branch rule. The first rule whose `pattern` matches the
/// branch name wins (declaration order = priority).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchRule {
    /// Glob pattern, e.g. `"main"`, `"dependabot/*"`, `"*"`.
    pub pattern: String,
    pub require: Requirement,
    pub on_no_witness: OnNoWitness,
    #[serde(default = "default_on_block")]
    pub on_block: OnBlock,
    #[serde(default = "default_on_warn")]
    pub on_warn: OnWarn,
}

fn default_on_block() -> OnBlock {
    OnBlock::Reject
}

fn default_on_warn() -> OnWarn {
    OnWarn::Allow
}

/// Baseline-adjacent metadata that drives `cutoff_commit` acceptance.
/// The `cutoff_commit` itself is populated by `anvil baseline`
/// (MLP-007); this crate only consumes it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineSection {
    /// SHA of the commit at adoption time. Commits at or before
    /// this SHA in the first-parent ancestry are accepted without
    /// witness checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cutoff_commit: Option<String>,
}

/// Parsed `anvil/policy.yml` (or `.json` / `.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// Optional semver floor consumed by
    /// `anvil_rules::RequiredAnvilVersion`. Kept as `Option<String>`
    /// here so this crate doesn't pull in `anvil-rules`; the CLI
    /// caller composes them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_anvil_version: Option<String>,

    /// Adoption-time pin from `anvil baseline`. Optional; absent in
    /// greenfield repos.
    #[serde(default)]
    pub baseline: BaselineSection,

    /// Per-branch rules in priority order.
    pub branches: Vec<BranchRule>,
}

#[derive(Debug, Error)]
pub enum PolicyParseError {
    /// `anvil-config` could not decode the file.
    #[error("config decode error: {0}")]
    Config(#[from] ParseError),
    /// The decoded JSON didn't match the policy schema.
    #[error("policy schema mismatch: {0}")]
    Schema(#[from] serde_json::Error),
    /// At least one [`BranchRule`] is required; an empty `branches`
    /// list would mean no branch can ever match.
    #[error("policy must declare at least one branch rule")]
    NoBranches,
    /// `pattern` was an empty string. Empty patterns can't be
    /// distinguished from missing patterns and would silently match
    /// nothing.
    #[error("branch rule has empty pattern")]
    EmptyPattern,
    /// `required_anvil_version` was present but empty. An empty
    /// string would parse as an unrecognised floor at consumer
    /// fire-time; refuse here so the error surfaces at the policy
    /// boundary instead.
    #[error("required_anvil_version is set but empty; omit the field or supply a value")]
    EmptyRequiredAnvilVersion,
    /// `baseline.cutoff_commit` was present but empty. An empty SHA
    /// can't represent a commit; refuse so a half-written
    /// `anvil baseline` artefact doesn't silently disable cutoff
    /// acceptance.
    #[error("baseline.cutoff_commit is set but empty; omit the field or supply a SHA")]
    EmptyCutoffCommit,
    /// `baseline.cutoff_commit` was present but not hex-shaped (e.g.
    /// a symbolic ref like `HEAD` or a branch name). Such a value
    /// would silently fail to match any SHA in the first-parent
    /// ancestry at fire-time, leaving the cutoff a no-op with no
    /// operator-visible signal. Refuse at the policy boundary so the
    /// typo surfaces before the next push.
    #[error("baseline.cutoff_commit must be a hex-only SHA (4–64 chars); got {raw:?}")]
    InvalidCutoffCommit { raw: String },
}

impl Policy {
    /// Parse a policy from raw text in the given format.
    pub fn parse(raw: &str, format: ConfigFormat, path: &Path) -> Result<Self, PolicyParseError> {
        let value = parse_str(raw, format, path)?;
        let policy: Self = serde_json::from_value(value)?;
        policy.validate()?;
        Ok(policy)
    }

    /// Reject obvious schema violations that serde alone doesn't
    /// catch.
    fn validate(&self) -> Result<(), PolicyParseError> {
        if self.branches.is_empty() {
            return Err(PolicyParseError::NoBranches);
        }
        for rule in &self.branches {
            if rule.pattern.is_empty() {
                return Err(PolicyParseError::EmptyPattern);
            }
        }
        if let Some(v) = &self.required_anvil_version
            && v.is_empty()
        {
            return Err(PolicyParseError::EmptyRequiredAnvilVersion);
        }
        if let Some(c) = &self.baseline.cutoff_commit {
            if c.is_empty() {
                return Err(PolicyParseError::EmptyCutoffCommit);
            }
            if !is_hex_sha_shape(c) {
                return Err(PolicyParseError::InvalidCutoffCommit { raw: c.clone() });
            }
        }
        Ok(())
    }
}

/// True when `raw` looks like a git SHA: 4–64 lowercase or uppercase
/// hex characters, no other content. Mirrors the shape check in
/// `anvil_hook::is_hex_sha` but local to this crate so `anvil-l4`
/// does not gain a dependency on `anvil-hook`.
fn is_hex_sha_shape(raw: &str) -> bool {
    let len = raw.len();
    (4..=64).contains(&len) && raw.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_YAML: &str = r"
required_anvil_version: '>=0.6.0'
baseline:
  cutoff_commit: a3b2ea4e
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
    on_block: reject
  - pattern: dependabot/*
    require: l4_only
    on_no_witness: validate_at_l4
  - pattern: '*'
    require: l4_or_l3
    on_no_witness: validate_at_l4
";

    #[test]
    fn parse_accepts_canonical_yaml_shape() {
        let p = Policy::parse(VALID_YAML, ConfigFormat::Yaml, Path::new("<test>")).unwrap();
        assert_eq!(p.required_anvil_version.as_deref(), Some(">=0.6.0"));
        assert_eq!(p.baseline.cutoff_commit.as_deref(), Some("a3b2ea4e"));
        assert_eq!(p.branches.len(), 3);
        assert_eq!(p.branches[0].pattern, "main");
        assert_eq!(p.branches[0].require, Requirement::L4OrL3);
        assert_eq!(p.branches[0].on_no_witness, OnNoWitness::ValidateAtL4);
        assert_eq!(p.branches[0].on_block, OnBlock::Reject);
        assert_eq!(p.branches[0].on_warn, OnWarn::Allow); // default
        assert_eq!(p.branches[1].require, Requirement::L4Only);
    }

    #[test]
    fn parse_accepts_json_equivalent() {
        let json = r#"{
            "branches": [
                {"pattern": "main", "require": "l4_or_l3", "on_no_witness": "validate_at_l4"}
            ]
        }"#;
        let p = Policy::parse(json, ConfigFormat::Json, Path::new("<test>")).unwrap();
        assert_eq!(p.branches.len(), 1);
        assert!(p.required_anvil_version.is_none());
    }

    #[test]
    fn parse_accepts_toml_equivalent() {
        let toml = r#"
[[branches]]
pattern = "main"
require = "l4_or_l3"
on_no_witness = "validate_at_l4"
"#;
        let p = Policy::parse(toml, ConfigFormat::Toml, Path::new("<test>")).unwrap();
        assert_eq!(p.branches.len(), 1);
        assert_eq!(p.branches[0].pattern, "main");
    }

    #[test]
    fn parse_rejects_empty_branches() {
        let yaml = "branches: []\n";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::NoBranches));
    }

    #[test]
    fn parse_rejects_empty_pattern() {
        let yaml = r"
branches:
  - pattern: ''
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::EmptyPattern));
    }

    #[test]
    fn parse_rejects_unknown_require_value() {
        let yaml = r"
branches:
  - pattern: main
    require: bogus
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::Schema(_)));
    }

    #[test]
    fn parse_rejects_unknown_on_no_witness_value() {
        let yaml = r"
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: maybe
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::Schema(_)));
    }

    #[test]
    fn defaults_for_on_block_and_on_warn_kick_in_when_omitted() {
        let yaml = r"
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let p = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap();
        // ADR-037 §D-5 defaults: blocks reject, warns allow.
        assert_eq!(p.branches[0].on_block, OnBlock::Reject);
        assert_eq!(p.branches[0].on_warn, OnWarn::Allow);
    }

    #[test]
    fn baseline_section_default_is_empty() {
        let yaml = r"
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let p = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap();
        assert!(p.baseline.cutoff_commit.is_none());
    }

    #[test]
    fn parse_propagates_invalid_yaml_as_config_error() {
        let yaml = "branches: [\n";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::Config(_)));
    }

    #[test]
    fn parse_rejects_empty_required_anvil_version() {
        let yaml = r"
required_anvil_version: ''
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::EmptyRequiredAnvilVersion));
    }

    #[test]
    fn parse_rejects_empty_cutoff_commit() {
        let yaml = r"
baseline:
  cutoff_commit: ''
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::EmptyCutoffCommit));
    }

    #[test]
    fn parse_rejects_symbolic_ref_as_cutoff_commit() {
        // MLP2-021 Council follow-up: a symbolic ref like `HEAD`
        // or a branch name would silently fail to match any SHA in
        // the first-parent ancestry at fire-time, leaving the
        // cutoff a no-op with no operator signal. Refuse at the
        // policy boundary so the typo surfaces before push.
        for bad in ["HEAD", "main", "release/0.7", "v0.7.0"] {
            let yaml = format!(
                r"
baseline:
  cutoff_commit: '{bad}'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
"
            );
            let err = Policy::parse(&yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
            match err {
                PolicyParseError::InvalidCutoffCommit { raw } => {
                    assert_eq!(raw, bad);
                }
                other => panic!("expected InvalidCutoffCommit for {bad:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn parse_rejects_three_char_cutoff_commit_as_too_short() {
        // 3-char hex slips past `all is_ascii_hexdigit` but isn't a
        // meaningful prefix for git rev-list lookup.
        let yaml = r"
baseline:
  cutoff_commit: 'abc'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::InvalidCutoffCommit { .. }));
    }

    #[test]
    fn parse_accepts_short_and_full_hex_cutoff_commit() {
        // 7-char abbreviation (git's default --abbrev=7) and full
        // 40-char sha1 / 64-char sha256 all parse.
        for good in ["a3b2ea4", "a3b2ea4e", &"a".repeat(40), &"b".repeat(64)] {
            let yaml = format!(
                r"
baseline:
  cutoff_commit: '{good}'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
"
            );
            let p = Policy::parse(&yaml, ConfigFormat::Yaml, Path::new("<test>"))
                .unwrap_or_else(|e| panic!("{good:?} should parse, got {e}"));
            assert_eq!(p.baseline.cutoff_commit.as_deref(), Some(good));
        }
    }
}
