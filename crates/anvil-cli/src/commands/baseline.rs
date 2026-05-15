//! `anvil baseline` command (MLP-007 CLI lane).
//!
//! Wraps the `anvil-baseline` library with the user-facing entry points:
//! `anvil baseline` (create / refresh) and `anvil baseline verify`.
//!
//! ## v1 scope
//!
//! - **`anvil baseline`** creates `anvil/baseline.json` for the
//!   current repo. The orchestrator first calls
//!   [`ensure_project_id`] so adopting Anvil into an existing repo
//!   writes `anvil/project-id` in the same flow (MLP2-032), then runs
//!   the [`anvil_checks`] scanner across the worktree to populate the
//!   findings array (MLP2-034 Phase 1). With no existing
//!   `cutoff_commit`, the on-disk record carries `null`; consumers
//!   that need a pin set it explicitly via `--refresh` after a
//!   subsequent commit.
//! - **`anvil baseline --refresh`** re-creates the file in place,
//!   bumping `created_at`, preserving `cutoff_commit`, and re-running
//!   the scanner so adversarial-refresh detection (MLP2-035) has a
//!   current findings set to compare against.
//! - **`anvil baseline verify`** re-reads `anvil/baseline.json` and
//!   reports findings count + `cutoff_commit`. The diff partition
//!   into the hook lane gate is Phase 2 of MLP2-034.
//!
//! ## Cutoff pinning (MLP2-031 ↔ -032)
//!
//! When a baseline carries a `cutoff_commit`, the orchestrator pins it
//! into `anvil/policy.{yml,yaml,json,toml}` via
//! [`anvil_l4::pin_cutoff_commit`] so the L4 policy lane reads it
//! from the policy file rather than from `baseline.json`. The pin
//! step is best-effort: a missing or unreadable policy file is
//! reported as a hint (warnings over blocks) and does not fail
//! `anvil baseline`. Operators bootstrap the policy file via
//! `anvil init`.
//!
//! ## Deferred (Phase 2 + later)
//!
//! - Diff partition into the hook lane gate (Phase 2 of MLP2-034).
//! - Per-class baseline behaviour (ADR-039 hard-pinned rejection).
//! - Adversarial-refresh detection (MLP2-035).
//! - Async continuation for >100k files (MLP2-036).

use std::path::Path;

use anvil_baseline::{
    Baseline, BaselineFinding, BaselineMetadata, compute_fingerprint, load as load_baseline,
    save as save_baseline,
};
use anvil_checks::antipattern::{AntipatternCheckConfig, run_antipattern_check};
use anvil_config::{DiscoveredConfig, discover};
use anvil_l4::{Policy, PolicyPinError, pin_cutoff_commit};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::GlobalArgs;
use crate::activation::identity::{ensure_project_id, mint_new_identity};
use crate::util::is_ignored_dir_name;

#[derive(Debug, Args)]
pub struct BaselineArgs {
    #[command(subcommand)]
    command: Option<BaselineCommand>,
    /// Refresh an existing baseline at HEAD; updates `created_at`
    /// and preserves `cutoff_commit`. Ignored when a subcommand
    /// (e.g. `verify`) is given.
    #[arg(long)]
    refresh: bool,
    /// MLP2-033: mint a fresh `project_uuid` and record the previous
    /// one as `forked_from`. Use after `git clone`-ing a repo whose
    /// `anvil/project-id` was inherited from the parent and you want
    /// the fork to carry its own identity. Destructive on the prior
    /// `forked_from` field — the chain is single-deep by design.
    #[arg(long = "new-identity")]
    new_identity: bool,
}

#[derive(Debug, Subcommand)]
enum BaselineCommand {
    /// Re-read `anvil/baseline.json` and report contents. With
    /// scanner integration this becomes a real diff against current
    /// findings.
    Verify,
}

pub fn run(args: &BaselineArgs, _global: &GlobalArgs) -> Result<()> {
    let repo_root = std::env::current_dir().context("resolve repo root")?;
    match &args.command {
        Some(BaselineCommand::Verify) => {
            if args.new_identity {
                anyhow::bail!(
                    "`--new-identity` is incompatible with `verify` — verify is read-only"
                );
            }
            run_verify(&repo_root)
        }
        None => run_create_or_refresh(&repo_root, args.refresh, args.new_identity),
    }
}

fn run_create_or_refresh(repo_root: &Path, refresh: bool, new_identity: bool) -> Result<()> {
    // MLP2-032 / MLP2-033: establish project identity in the same flow
    // as baseline bootstrap. Default path is `ensure_project_id`
    // (idempotent — returns the existing identity, or atomically
    // writes a fresh v7 UUID if absent). `--new-identity` opts into
    // the destructive `mint_new_identity` path: always writes a
    // fresh UUID and records the previous one as `forked_from`. Use
    // after `git clone` when the inherited identity needs to detach.
    let identity = if new_identity {
        mint_new_identity(repo_root, env!("CARGO_PKG_VERSION"))
            .context("mint fresh anvil/project-id (--new-identity)")?
    } else {
        ensure_project_id(repo_root, env!("CARGO_PKG_VERSION"))
            .context("ensure anvil/project-id")?
    };

    let existing = load_baseline(repo_root).context("load existing baseline (if any)")?;
    // `--new-identity` implies a baseline rewrite — the on-disk
    // `baseline.json` carries `project_uuid` in its metadata, and
    // letting it diverge from the freshly-minted identity would
    // recreate the policy/baseline divergence trap MLP2-032 closed
    // for cutoff_commit. Treat the flag as `--refresh` for the
    // already-exists check.
    if existing.is_some() && !refresh && !new_identity {
        println!("anvil: baseline already exists at anvil/baseline.json — use --refresh to update");
        return Ok(());
    }

    // Cutoff resolution: existing baseline.json wins; otherwise fall
    // back to whatever the policy file already pins. The fallback
    // closes a divergence trap on first-create — without it, an
    // operator who hand-set `baseline.cutoff_commit` in policy.yml
    // would end up with a baseline.json carrying `null` and a policy
    // file carrying the SHA, with no operator-visible signal.
    let cutoff = existing
        .as_ref()
        .and_then(|b| b.cutoff_commit.clone())
        .or_else(|| read_policy_cutoff(repo_root));

    let metadata = BaselineMetadata {
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        created_by_version: env!("CARGO_PKG_VERSION").to_string(),
        project_uuid: identity.project_uuid,
    };

    // MLP2-034 Phase 1: populate findings from the antipattern
    // scanner across the worktree. Phase 2 (hook-lane diff partition)
    // and adversarial-refresh detection (MLP2-035) layer on top of
    // this populated record.
    let findings = scan_repo_for_findings(repo_root);

    let mut baseline = Baseline::new(metadata, findings);
    baseline.cutoff_commit.clone_from(&cutoff);
    save_baseline(repo_root, &baseline).context("write anvil/baseline.json")?;

    let action = if refresh { "refreshed" } else { "created" };
    println!(
        "anvil: baseline {action} ({} findings)",
        baseline.findings.len(),
    );

    // MLP2-031 ↔ -032: pin the cutoff into `anvil/policy.{yml,…}` so
    // the L4 policy lane reads it from policy rather than from
    // `baseline.json`. Best-effort — a missing or unreadable policy
    // file emits a hint and does not fail the orchestrator.
    if let Some(sha) = cutoff {
        try_pin_cutoff(repo_root, &sha);
    }

    Ok(())
}

fn run_verify(repo_root: &Path) -> Result<()> {
    let baseline = load_baseline(repo_root)
        .context("load baseline")?
        .context("no baseline at anvil/baseline.json — run `anvil baseline` first")?;
    println!(
        "anvil: baseline ok ({} findings, cutoff={})",
        baseline.findings.len(),
        baseline.cutoff_commit.as_deref().unwrap_or("<none>"),
    );
    Ok(())
}

/// Walk the worktree and run the antipattern scanner; convert each
/// warning into a [`BaselineFinding`] with a move-resistant
/// fingerprint.
///
/// On any per-file failure (read error, empty snippet at the warning
/// line, etc.) the affected finding is silently skipped — adoption
/// must not be blocked by a transient I/O race or an exotic encoding.
/// Returning a partial set is consistent with the "warnings over
/// blocks" CLAUDE.md principle.
///
/// **TOCTOU caveat.** The fingerprint is computed from a *second*
/// read of the file (the scanner already read it once). On a busy
/// tree where the file changes between reads, the snippet at
/// `warning.location.line` may differ from what the scanner saw —
/// in that case the resulting fingerprint will not match any future
/// scan, leaving the finding permanently stale. The window is small
/// during interactive `anvil baseline` runs but is documented here
/// because the silent-skip recovery hides it; future work could
/// either return the source content alongside warnings from
/// `run_antipattern_check` or require a quiescent worktree at
/// adoption time.
fn scan_repo_for_findings(repo_root: &Path) -> Vec<BaselineFinding> {
    let config = AntipatternCheckConfig::default();
    let files = collect_scannable_files(repo_root, &config.extensions);
    if files.is_empty() {
        return Vec::new();
    }
    let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let workspace_root = repo_root.to_string_lossy();
    let result = run_antipattern_check(&file_refs, &config, Some(workspace_root.as_ref()));

    let mut findings = Vec::with_capacity(result.warnings.warnings.len());
    for warning in &result.warnings.warnings {
        // Suppressed warnings are explicit author intent; they are
        // not baseline material because the author already
        // acknowledged them.
        if warning.suppressed.is_some() {
            continue;
        }
        // Re-read the source line for fingerprinting. The file path
        // on the warning is relative to `workspace_root`, so we join
        // it back onto `repo_root` to read.
        let abs = repo_root.join(&warning.location.file);
        let Ok(content) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let line_idx = warning.location.line.saturating_sub(1);
        let Some(snippet) = content.lines().nth(line_idx) else {
            continue;
        };
        let Ok(fingerprint) = compute_fingerprint(&warning.id, snippet) else {
            continue;
        };
        findings.push(BaselineFinding {
            file_path: warning.location.file.clone(),
            fingerprint,
            rule_id: warning.id.clone(),
        });
    }
    findings
}

/// Walk `repo_root` with `ignore::WalkBuilder`, mirroring
/// `anvil check --all`'s file discovery (SCAN-001 shape) but rooted at
/// the explicit baseline target rather than `git rev-parse --show-toplevel`.
fn collect_scannable_files(repo_root: &Path, extensions: &[String]) -> Vec<String> {
    let walker = ignore::WalkBuilder::new(repo_root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            if e.file_type().is_some_and(|ft| ft.is_dir()) {
                let name = e.file_name().to_string_lossy();
                !is_ignored_dir_name(&name)
            } else {
                true
            }
        })
        .build();

    let mut files: Vec<String> = walker
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .filter_map(|e| {
            let path_str = e.path().to_string_lossy().to_string();
            extensions
                .iter()
                .any(|ext| path_str.ends_with(ext.as_str()))
                .then_some(path_str)
        })
        .collect();
    files.sort();
    files
}

/// Pin `cutoff_commit` into the first `anvil/policy.*` file present.
///
/// Best-effort: any failure (no policy file present, parse error,
/// symlink, malformed cutoff) is reported as a hint and does NOT
/// fail the orchestrator. Adoption must not break because the
/// operator hasn't yet bootstrapped a policy file.
fn try_pin_cutoff(repo_root: &Path, cutoff: &str) {
    let Some(DiscoveredConfig {
        path: policy_path, ..
    }) = find_policy_file(repo_root)
    else {
        println!(
            "anvil: cutoff_commit recorded in baseline.json but no anvil/policy.{{yaml,yml,json,toml}} found — run `anvil init` to materialise a policy file before pinning"
        );
        return;
    };
    match pin_cutoff_commit(&policy_path, cutoff) {
        Ok(()) => println!(
            "anvil: cutoff_commit {cutoff} pinned into {}",
            policy_path
                .strip_prefix(repo_root)
                .unwrap_or(&policy_path)
                .display(),
        ),
        Err(err) => {
            let label = match &err {
                PolicyPinError::Io(_) => "io",
                PolicyPinError::Parse(_) => "policy parse",
                PolicyPinError::NotAnObject | PolicyPinError::BaselineNotAMap => "policy shape",
                PolicyPinError::InvalidCutoffCommit { .. } => "invalid cutoff",
                PolicyPinError::Serialise { .. } => "serialise",
                PolicyPinError::SymlinkRefusal { .. } => "symlink refusal",
            };
            println!(
                "anvil: cutoff_commit recorded in baseline.json but pin into {} skipped ({label}: {err})",
                policy_path
                    .strip_prefix(repo_root)
                    .unwrap_or(&policy_path)
                    .display(),
            );
        }
    }
}

/// Locate the policy file using `anvil-config`'s canonical
/// discovery precedence (`yaml > yml > json > toml`). Returning the
/// `DiscoveredConfig` keeps the caller honest about which format
/// was selected — every downstream path (`pin_cutoff_commit`,
/// `Policy::parse`) needs the [`ConfigFormat`] to decode the file.
fn find_policy_file(repo_root: &Path) -> Option<DiscoveredConfig> {
    discover(&repo_root.join("anvil"), "policy").ok().flatten()
}

/// Read the `baseline.cutoff_commit` field from the discovered
/// policy file, if any. Best-effort: any failure (no file, parse
/// error, missing field) returns `None` — the caller treats absence
/// the same as "operator hasn't pinned a cutoff yet".
fn read_policy_cutoff(repo_root: &Path) -> Option<String> {
    let DiscoveredConfig { path, format } = find_policy_file(repo_root)?;
    let raw = std::fs::read_to_string(&path).ok()?;
    let policy = Policy::parse(&raw, format, &path).ok()?;
    policy.baseline.cutoff_commit
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Minimal valid `anvil/policy.{yaml,yml}` body: one branch rule
    /// with the required fields (`pattern`, `require`,
    /// `on_no_witness`). Anything less fails `Policy::validate` —
    /// the orchestrator's pin step would still run via
    /// `pin_cutoff_commit` (which works at the JSON-shape layer
    /// without typed parsing) but `read_policy_cutoff` would not be
    /// able to decode it on the read-back path.
    const MIN_VALID_POLICY: &str =
        "branches:\n  - pattern: main\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n";

    fn write_policy_yml(root: &Path) {
        fs::create_dir_all(root.join("anvil")).unwrap();
        fs::write(root.join("anvil/policy.yml"), MIN_VALID_POLICY).unwrap();
    }

    #[test]
    fn create_mints_identity_when_absent() {
        let tmp = TempDir::new().unwrap();
        // No anvil/project-id pre-seeded — the orchestrator must
        // mint one (MLP2-032).
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let identity_path = tmp.path().join("anvil/project-id");
        assert!(identity_path.exists(), "anvil/project-id should be minted");
        let baseline = load_baseline(tmp.path()).unwrap().unwrap();
        let identity_text = fs::read_to_string(&identity_path).unwrap();
        assert!(identity_text.contains(&baseline.metadata.project_uuid));
    }

    #[test]
    fn create_is_idempotent_on_identity_when_present() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("anvil")).unwrap();
        fs::write(
            tmp.path().join("anvil/project-id"),
            "project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456\n",
        )
        .unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let baseline = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(
            baseline.metadata.project_uuid, "01997e4a-1b2c-7345-8901-abcdef123456",
            "existing identity must be preserved across baseline runs"
        );
    }

    #[test]
    fn create_without_refresh_does_not_overwrite_existing() {
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let first = load_baseline(tmp.path()).unwrap().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let second = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(first.metadata.created_at, second.metadata.created_at);
    }

    #[test]
    fn refresh_preserves_cutoff_commit_across_runs() {
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        run_create_or_refresh(tmp.path(), true, false).unwrap();
        let refreshed = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(refreshed.cutoff_commit.as_deref(), Some("a3b2ea4e"));
    }

    #[test]
    fn refresh_pins_cutoff_into_policy_when_present() {
        // MLP2-031 ↔ -032: when a baseline carries a cutoff_commit
        // and `anvil/policy.yml` exists, the orchestrator pins it.
        let tmp = TempDir::new().unwrap();
        write_policy_yml(tmp.path());

        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        run_create_or_refresh(tmp.path(), true, false).unwrap();

        let policy_text = fs::read_to_string(tmp.path().join("anvil/policy.yml")).unwrap();
        assert!(
            policy_text.contains("a3b2ea4e"),
            "expected cutoff_commit pinned into policy.yml; got:\n{policy_text}"
        );
    }

    #[test]
    fn pin_targets_yaml_over_yml_when_both_present() {
        // Council #C-1 (quick): when both policy.yaml and policy.yml
        // exist, the pin must follow `anvil-config`'s canonical
        // discovery precedence (yaml > yml). Regression guard against
        // a hand-rolled candidate list silently disagreeing with
        // `anvil_config::discover`.
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("anvil")).unwrap();
        fs::write(tmp.path().join("anvil/policy.yaml"), MIN_VALID_POLICY).unwrap();
        fs::write(tmp.path().join("anvil/policy.yml"), MIN_VALID_POLICY).unwrap();

        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        run_create_or_refresh(tmp.path(), true, false).unwrap();

        let high_precedence = fs::read_to_string(tmp.path().join("anvil/policy.yaml")).unwrap();
        let low_precedence = fs::read_to_string(tmp.path().join("anvil/policy.yml")).unwrap();
        assert!(
            high_precedence.contains("a3b2ea4e"),
            "policy.yaml (higher precedence) should receive the pin; got:\n{high_precedence}"
        );
        assert!(
            !low_precedence.contains("a3b2ea4e"),
            "policy.yml (lower precedence) must remain untouched; got:\n{low_precedence}"
        );
    }

    #[test]
    fn create_picks_up_cutoff_from_policy_when_baseline_absent() {
        // Council #C-2 (quick): on first `anvil baseline`, an
        // existing `policy.yaml` carrying `baseline.cutoff_commit`
        // must seed the freshly written baseline.json so the two
        // files cannot silently diverge.
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("anvil")).unwrap();
        fs::write(
            tmp.path().join("anvil/policy.yaml"),
            "baseline:\n  cutoff_commit: a3b2ea4e\nbranches:\n  - pattern: main\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n",
        )
        .unwrap();

        run_create_or_refresh(tmp.path(), false, false).unwrap();

        let baseline = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(
            baseline.cutoff_commit.as_deref(),
            Some("a3b2ea4e"),
            "first-create must seed cutoff from policy when baseline.json is being bootstrapped"
        );
    }

    #[test]
    fn refresh_does_not_fail_when_no_policy_file_to_pin() {
        // Warnings over blocks: a missing policy file is a hint, not
        // a failure of `anvil baseline --refresh`.
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();
        // No anvil/policy.* file present.
        run_create_or_refresh(tmp.path(), true, false).unwrap();
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(after.cutoff_commit.as_deref(), Some("a3b2ea4e"));
    }

    #[test]
    fn create_populates_findings_from_scanner() {
        // MLP2-034 Phase 1: a worktree containing a known
        // antipattern (`AP-003: any-type-annotation`) must produce
        // a populated `BaselineFinding` with rule_id, file_path, and
        // a non-empty fingerprint.
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(
            tmp.path().join("src/app.ts"),
            "const value: any = input;\nconsole.log(value);\n",
        )
        .unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let baseline = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(
            !baseline.findings.is_empty(),
            "scanner should populate at least one finding for `any`-type annotation"
        );
        let ap003 = baseline
            .findings
            .iter()
            .find(|f| f.rule_id == "AP-003")
            .expect("AP-003 (any-type) should be flagged on src/app.ts");
        assert_eq!(ap003.file_path, "src/app.ts");
        assert_eq!(ap003.fingerprint.len(), 16, "16-hex fingerprint");
        assert!(ap003.fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn refresh_repopulates_findings_after_new_violation() {
        // After --refresh, a newly added violation must surface in
        // the rewritten baseline (Phase 1: the on-disk record is the
        // reflection of the current scan).
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/clean.ts"), "export const x = 1;\n").unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let first = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(first.findings.is_empty(), "no antipatterns yet");

        // Introduce a violation and refresh.
        fs::write(tmp.path().join("src/app.ts"), "const v: any = bad;\n").unwrap();
        run_create_or_refresh(tmp.path(), true, false).unwrap();
        let refreshed = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(
            refreshed.findings.iter().any(|f| f.rule_id == "AP-003"),
            "AP-003 should appear after --refresh"
        );
    }

    #[test]
    fn verify_reports_loaded_baseline() {
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        run_verify(tmp.path()).unwrap();
    }

    #[test]
    fn verify_returns_error_when_no_baseline() {
        let tmp = TempDir::new().unwrap();
        let err = run_verify(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("no baseline"));
    }

    // ---- MLP2-033: --new-identity ------------------------------

    #[test]
    fn new_identity_remints_uuid_and_records_forked_from() {
        // Validation fixture from MLP2-033: parent uuid A → grandchild
        // uuid B with `forked_from = A` after `anvil baseline
        // --new-identity`. Both `anvil/project-id` AND
        // `anvil/baseline.json`'s `metadata.project_uuid` must reflect
        // the new identity — letting them diverge would recreate the
        // policy/baseline divergence trap MLP2-032 closed for cutoff.
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let parent_uuid = load_baseline(tmp.path())
            .unwrap()
            .unwrap()
            .metadata
            .project_uuid;

        run_create_or_refresh(tmp.path(), false, true).unwrap();

        let child_baseline = load_baseline(tmp.path()).unwrap().unwrap();
        assert_ne!(
            child_baseline.metadata.project_uuid, parent_uuid,
            "baseline.json metadata must carry the freshly minted UUID"
        );

        let project_id_text = fs::read_to_string(tmp.path().join("anvil/project-id")).unwrap();
        assert!(
            project_id_text.contains(&child_baseline.metadata.project_uuid),
            "project-id must record the new UUID; got:\n{project_id_text}"
        );
        assert!(
            project_id_text.contains(&format!("forked_from: {parent_uuid}")),
            "project-id must record forked_from = parent UUID; got:\n{project_id_text}"
        );
    }

    #[test]
    fn new_identity_bypasses_already_exists_short_circuit() {
        // Without --new-identity, a second `anvil baseline` against an
        // existing baseline is a no-op (operator must opt into refresh).
        // With --new-identity, the rewrite is mandatory — otherwise
        // baseline.json's metadata would silently keep the parent UUID.
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let first = load_baseline(tmp.path()).unwrap().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // No --refresh, but --new-identity → must rewrite.
        run_create_or_refresh(tmp.path(), false, true).unwrap();
        let second = load_baseline(tmp.path()).unwrap().unwrap();
        assert_ne!(
            first.metadata.project_uuid, second.metadata.project_uuid,
            "--new-identity must rewrite baseline metadata even without --refresh"
        );
    }

    #[test]
    fn new_identity_preserves_existing_cutoff_commit() {
        // Council quick #C-4 (MINOR) regression guard: rewriting the
        // baseline under `--new-identity` must carry the existing
        // `cutoff_commit` forward. Otherwise the operator who pinned
        // a cutoff would silently lose it the moment they detached
        // the project identity.
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(tmp.path(), false, false).unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        run_create_or_refresh(tmp.path(), false, true).unwrap();

        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(
            after.cutoff_commit.as_deref(),
            Some("a3b2ea4e"),
            "--new-identity must preserve cutoff_commit across the rewrite"
        );
    }

    #[test]
    fn new_identity_on_empty_repo_mints_with_no_parent() {
        // Same as `mint_new_identity_on_empty_repo_acts_like_fresh`
        // but exercises the orchestrator entry point. No parent UUID
        // → forked_from absent.
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(tmp.path(), false, true).unwrap();
        let project_id_text = fs::read_to_string(tmp.path().join("anvil/project-id")).unwrap();
        assert!(project_id_text.contains("project_uuid:"));
        assert!(
            !project_id_text.contains("forked_from:"),
            "no parent → no forked_from; got:\n{project_id_text}"
        );
    }
}
