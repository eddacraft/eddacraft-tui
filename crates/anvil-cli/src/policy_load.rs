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
pub(crate) fn policy_variants(repo_root: &Path) -> Vec<std::path::PathBuf> {
    let dir = repo_root.join("anvil");
    anvil_config::DISCOVER_PRECEDENCE
        .iter()
        .map(|format| dir.join(format!("policy.{}", format.extension())))
        .filter(|path| path.exists())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML_BODY: &str =
        "branches:\n  - pattern: PATTERN\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n";

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
        let variants = policy_variants(tmp.path());
        let names: Vec<_> = variants
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["policy.yaml", "policy.yml", "policy.toml"]);
    }
}
