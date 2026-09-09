//! Shared L4 policy discovery and bounded load (UCFG-009, ADR-120 pt 6).
//!
//! One discovery implementation: [`anvil_config::discover`] over
//! `anvil/policy.*` replaces the hand-rolled candidate lists that
//! `commands/hook.rs` and `commands/l4_validate.rs` each carried.
//!
//! **Deliberate behaviour change** (ADR-120 pt 6): `DISCOVER_PRECEDENCE`
//! is yaml-first, so a repo holding both `anvil/policy.yml` and
//! `anvil/policy.yaml` now resolves to `policy.yaml` — the hand-rolled
//! lists were yml-first. `anvil doctor` warns on multi-variant repos,
//! naming the winner. Policy **authority** semantics (ADR-100:
//! committed-to-count) are untouched; this changes only how the file is
//! found.

use std::path::Path;

use anvil_l4::Policy;
use anyhow::{Context, Result};

/// ADR-037 D-5 default branch posture for a fresh project.
///
/// Omits `required_anvil_version` (optional floor) and
/// `baseline.cutoff_commit` (`anvil baseline` pins that on adoption).
/// `on_warn` is left unset so the schema default (`allow`) stands.
pub(crate) const DEFAULT_ACCEPTANCE_POLICY_YML: &str = "\
# L4 acceptance policy (ADR-037). First matching branch wins.
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
    on_block: reject
  - pattern: dependabot/*
    require: l4_only
    on_no_witness: validate_at_l4
  - pattern: \"*\"
    require: l4_or_l3
    on_no_witness: validate_at_l4
";

/// Load `anvil/policy.{yaml,yml,json,toml}` if present, yaml-first per
/// [`anvil_config::DISCOVER_PRECEDENCE`].
///
/// Returns `Ok(None)` when no policy file exists — callers treat that as
/// "this project hasn't opted into L4 enforcement" and skip the checks
/// entirely. Errors are propagated so callers can degrade per their own
/// contracts (the pre-push hook maps them to `InternalError`).
///
/// MLP2-063: refuses oversized policy files before allocating the body —
/// the shared bounded loader caps each file at
/// [`anvil_config::MAX_CONFIG_FILE_BYTES`] (1 MiB), matching the bound
/// `.anvil.*` parsing already enforces.
pub(crate) fn load_policy(repo_root: &Path) -> Result<Option<Policy>> {
    let Some(found) = anvil_config::discover(&repo_root.join("anvil"), "policy")
        .with_context(|| format!("probe anvil/policy.* under {}", repo_root.display()))?
    else {
        return Ok(None);
    };
    let raw = anvil_config::read_to_string_bounded(&found.path)
        .with_context(|| format!("read {}", found.path.display()))?;
    let policy = Policy::parse(&raw, found.format, &found.path)
        .with_context(|| format!("parse {}", found.path.display()))?;
    Ok(Some(policy))
}

/// The `anvil/policy.<ext>` variants present under `repo_root`, in
/// [`anvil_config::DISCOVER_PRECEDENCE`] order — index 0 is the winner
/// [`load_policy`] would pick. Used by `anvil doctor` to warn on
/// ambiguous multi-variant repos.
///
/// Propagates stat errors (`try_exists`) instead of treating them as
/// absent, matching `discover`'s loud posture — a permission-denied
/// `anvil/` must not report as "no policy file".
pub(crate) fn policy_variants(repo_root: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let dir = repo_root.join("anvil");
    let mut present = Vec::new();
    for format in &anvil_config::DISCOVER_PRECEDENCE {
        let path = dir.join(format!("policy.{}", format.extension()));
        if path.try_exists()? {
            present.push(path);
        }
    }
    Ok(present)
}

/// True when `anvil/policy.*` exists and parses as an L4 acceptance policy.
///
/// Absent, unreadable, or unparseable files are all `false` — status uses
/// this so L4 cannot claim `On` when both L4 entry points would no-op.
pub(crate) fn acceptance_policy_is_parseable(repo_root: &Path) -> bool {
    matches!(load_policy(repo_root), Ok(Some(_)))
}

#[cfg(test)]
pub(crate) fn seed_default_acceptance_policy(repo_root: &Path) -> Result<bool> {
    let result = crate::scaffold::reconcile_acceptance_policy(repo_root)?;
    Ok(result.outcome == crate::scaffold::MutationOutcome::Created)
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML_BODY: &str = "branches:\n  - pattern: PATTERN\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n";

    fn repo_with(files: &[(&str, &str)]) -> tempfile::TempDir {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("anvil")).unwrap();
        for (name, body) in files {
            std::fs::write(tmp.path().join("anvil").join(name), body).unwrap();
        }
        tmp
    }

    fn yaml(pattern: &str) -> String {
        YAML_BODY.replace("PATTERN", pattern)
    }

    #[test]
    fn none_when_no_policy_file() {
        let tmp = repo_with(&[]);
        assert!(load_policy(tmp.path()).unwrap().is_none());
    }

    /// Parity matrix (UCFG-009 validation): each extension present alone
    /// resolves to that file — identical to the old hand-rolled lists.
    #[test]
    fn single_variant_parity_across_all_extensions() {
        let cases: [(&str, &str); 4] = [
            ("policy.yaml", &yaml("main")),
            ("policy.yml", &yaml("main")),
            (
                "policy.json",
                r#"{"branches":[{"pattern":"main","require":"l4_or_l3","on_no_witness":"validate_at_l4"}]}"#,
            ),
            (
                "policy.toml",
                "[[branches]]\npattern = \"main\"\nrequire = \"l4_or_l3\"\non_no_witness = \"validate_at_l4\"\n",
            ),
        ];
        for (name, body) in cases {
            let tmp = repo_with(&[(name, body)]);
            let p = load_policy(tmp.path())
                .unwrap()
                .unwrap_or_else(|| panic!("{name} should load"));
            assert_eq!(p.branches[0].pattern, "main", "{name}");
        }
    }

    /// ADR-120 pt 6 deliberate flip: yaml beats yml (the hand-rolled
    /// lists were yml-first). This test pins the NEW winner.
    #[test]
    fn dual_variant_yaml_beats_yml() {
        let tmp = repo_with(&[
            ("policy.yml", yaml("yml-loses").as_str()),
            ("policy.yaml", yaml("yaml-wins").as_str()),
        ]);
        let p = load_policy(tmp.path()).unwrap().unwrap();
        assert_eq!(p.branches[0].pattern, "yaml-wins");
    }

    #[test]
    fn yml_still_beats_json_and_toml() {
        let tmp = repo_with(&[
            ("policy.yml", yaml("yml-wins").as_str()),
            (
                "policy.json",
                r#"{"branches":[{"pattern":"json-loses","require":"l4_or_l3","on_no_witness":"validate_at_l4"}]}"#,
            ),
        ]);
        let p = load_policy(tmp.path()).unwrap().unwrap();
        assert_eq!(p.branches[0].pattern, "yml-wins");
    }

    #[test]
    fn oversized_policy_refused_before_parse() {
        let big = format!(
            "# {}\n{}",
            "x".repeat(usize::try_from(anvil_config::MAX_CONFIG_FILE_BYTES).unwrap()),
            yaml("main")
        );
        let tmp = repo_with(&[("policy.yaml", big.as_str())]);
        let err = load_policy(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("read"), "got: {err:#}");
    }

    #[test]
    fn variants_listed_in_precedence_order() {
        let tmp = repo_with(&[
            ("policy.toml", "x = 1\n"),
            ("policy.yaml", "a: 1\n"),
            ("policy.yml", "b: 2\n"),
        ]);
        let variants = policy_variants(tmp.path()).unwrap();
        let names: Vec<_> = variants
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["policy.yaml", "policy.yml", "policy.toml"]);
    }

    #[test]
    fn default_acceptance_policy_parses_as_adr_037_posture() {
        let p = Policy::parse(
            DEFAULT_ACCEPTANCE_POLICY_YML,
            anvil_config::ConfigFormat::Yml,
            std::path::Path::new("anvil/policy.yml"),
        )
        .expect("default policy must parse");
        assert_eq!(p.branches.len(), 3);
        assert_eq!(p.branches[0].pattern, "main");
        assert_eq!(p.branches[0].require, anvil_l4::Requirement::L4OrL3);
        assert_eq!(
            p.branches[0].on_no_witness,
            anvil_l4::OnNoWitness::ValidateAtL4
        );
        assert_eq!(p.branches[0].on_block, anvil_l4::OnBlock::Reject);
        assert_eq!(p.branches[0].on_warn, anvil_l4::OnWarn::Allow);
        assert_eq!(p.branches[1].pattern, "dependabot/*");
        assert_eq!(p.branches[1].require, anvil_l4::Requirement::L4Only);
        assert_eq!(p.branches[2].pattern, "*");
        assert_eq!(p.branches[2].require, anvil_l4::Requirement::L4OrL3);
        assert!(p.required_anvil_version.is_none());
        assert!(p.baseline.cutoff_commit.is_none());
    }

    #[test]
    fn unparseable_policy_is_not_parseable() {
        let tmp = repo_with(&[("policy.yml", "this is not a policy\n")]);
        assert!(!acceptance_policy_is_parseable(tmp.path()));
    }
}
