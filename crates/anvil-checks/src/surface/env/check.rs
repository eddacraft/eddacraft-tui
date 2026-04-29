//! Aggregator for the SURFENV rule pack.
//!
//! `run_surfenv_check` is the single entry point higher-level callers
//! (CLI, NAPI binding, future TUI) reach for when they want every
//! SURFENV rule's verdict on a file set. Each rule still has a public
//! function (`scan_env_file`, `check_gitignore_hygiene`,
//! `scan_prod_values`, `check_env_drift`) for tests and one-off use,
//! but the aggregator is the supported integration shape — mirrors
//! the `crate::secret::run_secret_check` contract for the secret
//! surface so callers compose Track 3 surfaces uniformly.
//!
//! Discovery is the caller's job: pass in the list of `.env*` paths
//! you want considered (typically the output of running
//! [`is_env_file`](super::scanner::is_env_file) over a project tree).
//! The aggregator does not walk the filesystem itself — keeping the
//! discovery contract explicit means the same call works against an
//! in-memory snapshot, a git index, or a real working tree.
//!
//! ## Drift pairing
//!
//! [`DriftFinding`](super::drift::DriftFinding) is per-file-pair, not
//! per-file. The aggregator pairs each `.env.example`-style template
//! with its sibling concrete `.env*` file in the same directory and
//! runs `check_env_drift` on each pair. Files without a sibling are
//! skipped — drift is an inherently relational rule and a one-sided
//! result would be noise.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::secret::SecretCheckConfig;
use crate::surface::env::drift::{DriftFinding, check_env_drift};
use crate::surface::env::gitignore::{GitignoreFinding, check_gitignore_hygiene};
use crate::surface::env::prod_value::{ProdValueFinding, scan_prod_values};
use crate::surface::env::scanner::{EnvFinding, scan_env_file};

/// Aggregated result of running every SURFENV rule against a file set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SurfenvCheckResult {
    /// SURFENV-001 — secrets in `.env` values.
    pub secrets: Vec<EnvFinding>,
    /// SURFENV-002 — `.gitignore` hygiene.
    pub gitignore: Vec<GitignoreFinding>,
    /// SURFENV-003 — production-shaped values in non-prod files.
    pub prod_values: Vec<ProdValueFinding>,
    /// SURFENV-004 — drift between templates and concrete env files.
    pub drift: Vec<DriftFinding>,
}

impl SurfenvCheckResult {
    /// Total finding count across every rule, including suppressed
    /// findings. Mirrors how `SecretCheckResult` reports — callers
    /// filter by `suppressed` themselves.
    #[must_use]
    pub fn total_findings(&self) -> usize {
        self.secrets.len() + self.gitignore.len() + self.prod_values.len() + self.drift.len()
    }

    /// Total *unsuppressed* finding count — the number an operator
    /// actually has to action.
    #[must_use]
    pub fn unsuppressed_findings(&self) -> usize {
        self.secrets.iter().filter(|f| !f.suppressed).count()
            + self.gitignore.iter().filter(|f| !f.suppressed).count()
            + self.prod_values.iter().filter(|f| !f.suppressed).count()
            + self.drift.iter().filter(|f| !f.suppressed).count()
    }
}

/// Run every SURFENV rule against a set of `.env*` files.
///
/// `env_files` carries the path and content of each candidate file —
/// the caller has already done discovery and the file read, so the
/// aggregator can stay sync and free of `io::Error` concerns. (When
/// the caller needs a "walk-the-tree" shape, they compose this with
/// [`super::gitignore::check_gitignore_hygiene_for_paths`] for the
/// `.gitignore` half.)
///
/// `gitignore_text` is the repository's root `.gitignore` content, or
/// `None` when there isn't one. SURFENV-002 reports a dedicated
/// finding kind (`MissingGitignore`) in the latter case.
///
/// `secret_config` controls the SURFENV-001 secret-scan pass; the
/// remaining rules are config-free today.
#[must_use]
pub fn run_surfenv_check(
    env_files: &[(PathBuf, String)],
    gitignore_text: Option<&str>,
    secret_config: &SecretCheckConfig,
) -> SurfenvCheckResult {
    let mut result = SurfenvCheckResult::default();

    // SURFENV-001 — secrets in env values.
    for (path, content) in env_files {
        let display = path.to_string_lossy();
        result
            .secrets
            .extend(scan_env_file(&display, content, secret_config));
    }

    // SURFENV-002 — gitignore hygiene. The check builds its own
    // owned (path, content) pairs internally, so we hand it the same
    // slice the caller gave us.
    result.gitignore = check_gitignore_hygiene(env_files, gitignore_text);

    // SURFENV-003 — prod-shaped values in non-prod env files.
    for (path, content) in env_files {
        let display = path.to_string_lossy();
        result
            .prod_values
            .extend(scan_prod_values(&display, content));
    }

    // SURFENV-004 — drift between sibling template/concrete pairs.
    for (template_path, template_content, concrete_path, concrete_content) in
        pair_template_with_concrete(env_files)
    {
        let template_display = template_path.to_string_lossy().into_owned();
        let concrete_display = concrete_path.to_string_lossy().into_owned();
        result.drift.extend(check_env_drift(
            &template_display,
            template_content,
            &concrete_display,
            concrete_content,
        ));
    }

    result
}

/// True when `path`'s filename is a template — `.env.sample`,
/// `.env.template`, or any `.env*.example` form (covers the bare
/// `.env.example` and Next.js' `.env.local.example`,
/// `.env.production.example`, etc).
fn is_template_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if matches!(name, ".env.sample" | ".env.template") {
        return true;
    }
    name.starts_with(".env") && name.ends_with(".example")
}

/// True when `path`'s filename is a key/value `.env*` file the drift
/// rule can sensibly compare against a template — i.e. anything
/// matched by `is_env_file` *except* `.envrc` (a direnv shell script,
/// not a `KEY=value` file) and template forms (handled by
/// [`is_template_filename`]).
fn is_drift_concrete_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if name == ".envrc" {
        return false;
    }
    !is_template_filename(path)
}

/// Pair each template file with sibling concrete `.env*` files in the
/// same directory. A template may pair with multiple concretes (a
/// repo could have `.env.local` and `.env.production` both alongside
/// one `.env.example`); we yield one pairing per concrete sibling.
/// Returns borrowed slices into `env_files` so we don't duplicate
/// content.
fn pair_template_with_concrete<'a>(
    env_files: &'a [(PathBuf, String)],
) -> Vec<(&'a PathBuf, &'a String, &'a PathBuf, &'a String)> {
    // Bucket files by parent directory so the lookup is O(1) per
    // template instead of O(n) — n env files per repo is small today
    // but the bucket also lets us deterministically order pairings.
    let mut by_dir: BTreeMap<PathBuf, Vec<&'a (PathBuf, String)>> = BTreeMap::new();
    for entry in env_files {
        let parent = entry.0.parent().map(Path::to_path_buf).unwrap_or_default();
        by_dir.entry(parent).or_default().push(entry);
    }

    let mut pairings = Vec::new();
    for siblings in by_dir.values() {
        let templates: Vec<&&(PathBuf, String)> = siblings
            .iter()
            .filter(|e| is_template_filename(&e.0))
            .collect();
        // Concrete drift candidates exclude `.envrc` — a direnv
        // shell script that isn't a `KEY=value` file and so would
        // produce noise when compared to an env template. Copilot
        // review flagged this; the pair-with-everything default
        // would have surfaced spurious drift findings on any repo
        // that ships a `.envrc` alongside an `.env.example`.
        let concretes: Vec<&&(PathBuf, String)> = siblings
            .iter()
            .filter(|e| is_drift_concrete_filename(&e.0))
            .collect();
        for template in &templates {
            for concrete in &concretes {
                pairings.push((&template.0, &template.1, &concrete.0, &concrete.1));
            }
        }
    }
    pairings
}

#[cfg(test)]
mod tests {
    use super::{SurfenvCheckResult, run_surfenv_check};
    use crate::secret::SecretCheckConfig;
    use std::path::PathBuf;

    fn config_no_entropy() -> SecretCheckConfig {
        SecretCheckConfig {
            enable_entropy: false,
            ..SecretCheckConfig::default()
        }
    }

    #[test]
    fn empty_input_yields_default_result() {
        let result = run_surfenv_check(&[], None, &config_no_entropy());
        assert_eq!(result.total_findings(), 0);
        assert_eq!(result.unsuppressed_findings(), 0);
    }

    #[test]
    fn aggregator_runs_every_rule_against_sample_repo() {
        // .env.example template documenting two keys.
        let example = "DATABASE_URL=\nAPI_KEY=\n".to_string();
        // .env.local with a prod-shaped DB value, an extra key (drift),
        // and a SURFENV-002 trigger (not covered by the gitignore).
        let local = "DATABASE_URL=postgres://prod-db.acme.io/app\n\
                     API_KEY=local-dev-key\n\
                     NEW_FLAG=enabled\n"
            .to_string();
        let env_files = vec![
            (PathBuf::from("apps/web/.env.example"), example),
            (PathBuf::from("apps/web/.env.local"), local),
        ];
        let gitignore = "node_modules/\n";

        let result = run_surfenv_check(&env_files, Some(gitignore), &config_no_entropy());

        // .env.local is unprotected → SURFENV-002 fires (.env.example
        // is intentionally committed and skipped).
        assert!(
            result.gitignore.iter().any(|f| !f.suppressed),
            "expected SURFENV-002 finding, got {:?}",
            result.gitignore
        );

        // prod-shaped DB url → SURFENV-003 fires.
        assert!(
            result.prod_values.iter().any(|f| f.key == "DATABASE_URL"),
            "expected SURFENV-003 finding on DATABASE_URL"
        );

        // NEW_FLAG appears in concrete but not example → SURFENV-004.
        assert!(
            result.drift.iter().any(|f| f.key == "NEW_FLAG"),
            "expected SURFENV-004 drift finding"
        );

        // No raw secret pattern in the values → SURFENV-001 silent.
        assert!(
            result.secrets.is_empty(),
            "no AWS-shaped value present, got {:?}",
            result.secrets
        );

        // Aggregate accounting holds.
        let manual_total = result.secrets.len()
            + result.gitignore.len()
            + result.prod_values.len()
            + result.drift.len();
        assert_eq!(result.total_findings(), manual_total);
    }

    #[test]
    fn drift_pairing_treats_next_js_local_example_as_template() {
        // Real-world Next.js layout: `.env.local.example` documents
        // the shape of `.env.local` for new contributors. Copilot
        // review caught the prior allowlist missing this — drift
        // pairing must still recognise it as a template.
        let example = "FOO=\nAPI_KEY=\n".to_string();
        let local = "FOO=val\nAPI_KEY=val\nNEW=set\n".to_string();
        let env_files = vec![
            (PathBuf::from("apps/web/.env.local.example"), example),
            (PathBuf::from("apps/web/.env.local"), local),
        ];
        let result = run_surfenv_check(&env_files, Some(""), &config_no_entropy());
        assert!(
            result.drift.iter().any(|f| f.key == "NEW"),
            "expected drift finding for NEW; got {:?}",
            result.drift
        );
    }

    #[test]
    fn drift_pairing_excludes_envrc_from_concrete_set() {
        // `.envrc` is a direnv shell script, not a key/value env
        // file. Pairing it with an `.env.example` template would
        // produce noisy "every example key is missing" findings
        // since `.envrc` parses as zero entries. Copilot review
        // flagged this.
        let example = "FOO=\n".to_string();
        let envrc = "export FOO=val\n".to_string();
        let env_files = vec![
            (PathBuf::from(".env.example"), example),
            (PathBuf::from(".envrc"), envrc),
        ];
        let result = run_surfenv_check(&env_files, Some(""), &config_no_entropy());
        assert!(
            result.drift.is_empty(),
            ".envrc must not pair with .env.example; got {:?}",
            result.drift
        );
    }

    #[test]
    fn drift_pairs_only_within_same_directory() {
        // Two parallel apps each with their own template + concrete.
        // Drift findings must respect directory boundaries — a
        // template in apps/a must NOT pair with a concrete in apps/b.
        let example_a = "FOO=\n".to_string();
        let example_b = "BAR=\n".to_string();
        let concrete_a = "FOO=val\nNEW_A=set\n".to_string();
        let concrete_b = "BAR=val\nNEW_B=set\n".to_string();
        let env_files = vec![
            (PathBuf::from("apps/a/.env.example"), example_a),
            (PathBuf::from("apps/a/.env.local"), concrete_a),
            (PathBuf::from("apps/b/.env.example"), example_b),
            (PathBuf::from("apps/b/.env.local"), concrete_b),
        ];

        let result = run_surfenv_check(&env_files, Some(""), &config_no_entropy());

        // Each app should produce exactly one drift finding (NEW_A in
        // app a, NEW_B in app b). No cross-app pairings.
        let drift_keys: Vec<&str> = result.drift.iter().map(|f| f.key.as_str()).collect();
        assert!(drift_keys.contains(&"NEW_A"));
        assert!(drift_keys.contains(&"NEW_B"));
        // Cross-pairing would surface FOO and BAR as
        // "MissingFromConcrete" / "MissingFromExample" too — assert
        // none appear.
        assert!(
            !drift_keys.contains(&"FOO") && !drift_keys.contains(&"BAR"),
            "directory boundary leaked, got {drift_keys:?}"
        );
    }

    #[test]
    fn result_serializes_round_trip_via_json() {
        // Operations review flagged the missing serde derives — this
        // test pins the contract that the aggregate result and every
        // contained finding type can round-trip through JSON.
        let example = "FOO=\n".to_string();
        let local = "FOO=val\nNEW=set\n".to_string();
        let env_files = vec![
            (PathBuf::from("app/.env.example"), example),
            (PathBuf::from("app/.env.local"), local),
        ];
        let result = run_surfenv_check(&env_files, Some(""), &config_no_entropy());

        let json = serde_json::to_string(&result).expect("serialize");
        let round: SurfenvCheckResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round.total_findings(), result.total_findings());
        assert_eq!(round.drift.len(), result.drift.len());
    }
}
