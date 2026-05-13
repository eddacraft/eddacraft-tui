//! `anvil baseline` command (MLP-007 CLI lane).
//!
//! Wraps the `anvil-baseline` library with the user-facing entry points:
//! `anvil baseline` (create / refresh) and `anvil baseline verify`.
//!
//! ## v1 scope
//!
//! - **`anvil baseline`** creates `anvil/baseline.json` for the
//!   current repo. Without scanner integration (deferred), the
//!   findings array starts empty — the file is still load-bearing
//!   because it carries `project_uuid` + `created_at` +
//!   `created_by_version` for cross-machine identity, and a future
//!   `--refresh` flag picks it up.
//! - **`anvil baseline --refresh`** re-creates the file in place,
//!   bumping `created_at` and preserving `cutoff_commit`.
//! - **`anvil baseline verify`** re-reads `anvil/baseline.json` and
//!   reports findings count + `cutoff_commit`; with scanner
//!   integration this becomes a real diff against current findings.
//!
//! ## Deferred (scanner-integration follow-up)
//!
//! - Calling `anvil-checks` to populate the findings array.
//! - Per-class baseline behaviour (ADR-039 hard-pinned rejection,
//!   etc.).
//! - Adversarial-refresh detection.
//! - Async continuation for >100k files.

use anvil_baseline::{
    Baseline, BaselineFinding, BaselineMetadata, load as load_baseline, save as save_baseline,
};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::GlobalArgs;
use crate::activation::identity::read_project_id;

#[derive(Debug, Args)]
pub struct BaselineArgs {
    #[command(subcommand)]
    command: Option<BaselineCommand>,
    /// Refresh an existing baseline at HEAD; updates `created_at`
    /// and preserves `cutoff_commit`. Ignored when a subcommand
    /// (e.g. `verify`) is given.
    #[arg(long)]
    refresh: bool,
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
        Some(BaselineCommand::Verify) => run_verify(&repo_root),
        None => run_create_or_refresh(&repo_root, args.refresh),
    }
}

fn run_create_or_refresh(repo_root: &std::path::Path, refresh: bool) -> Result<()> {
    let identity = read_project_id(repo_root)
        .context("read anvil/project-id")?
        .context("anvil/project-id not found — run `anvil start` first")?;

    let existing = load_baseline(repo_root).context("load existing baseline (if any)")?;
    if existing.is_some() {
        if !refresh {
            println!(
                "anvil: baseline already exists at anvil/baseline.json — use --refresh to update"
            );
            return Ok(());
        }
    } else if refresh {
        // --refresh on a missing baseline is the same as creating
        // it; don't refuse the user's intent.
    }

    let cutoff = existing.as_ref().and_then(|b| b.cutoff_commit.clone());

    let metadata = BaselineMetadata {
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        created_by_version: env!("CARGO_PKG_VERSION").to_string(),
        project_uuid: identity.project_uuid,
    };
    // Scanner integration is deferred — start with empty findings.
    // The on-disk schema carries the rest faithfully.
    let findings: Vec<BaselineFinding> = Vec::new();
    let mut baseline = Baseline::new(metadata, findings);
    baseline.cutoff_commit = cutoff;
    save_baseline(repo_root, &baseline).context("write anvil/baseline.json")?;
    println!(
        "anvil: baseline {} ({} findings)",
        if refresh { "refreshed" } else { "created" },
        baseline.findings.len()
    );
    Ok(())
}

fn run_verify(repo_root: &std::path::Path) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_repo_with_identity() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("anvil")).unwrap();
        fs::write(
            tmp.path().join("anvil").join("project-id"),
            "project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456\n",
        )
        .unwrap();
        tmp
    }

    #[test]
    fn create_writes_baseline_file_with_identity() {
        let tmp = make_repo_with_identity();
        run_create_or_refresh(tmp.path(), false).unwrap();
        let loaded = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(
            loaded.metadata.project_uuid,
            "01997e4a-1b2c-7345-8901-abcdef123456"
        );
        assert_eq!(loaded.format_version, anvil_baseline::FORMAT_VERSION);
        assert!(loaded.findings.is_empty());
    }

    #[test]
    fn create_without_refresh_does_not_overwrite_existing() {
        let tmp = make_repo_with_identity();
        run_create_or_refresh(tmp.path(), false).unwrap();
        let first = load_baseline(tmp.path()).unwrap().unwrap();

        // Pretend some time passes; re-run without --refresh.
        std::thread::sleep(std::time::Duration::from_millis(10));
        run_create_or_refresh(tmp.path(), false).unwrap();
        let second = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(first.metadata.created_at, second.metadata.created_at);
    }

    #[test]
    fn refresh_preserves_cutoff_commit_across_runs() {
        let tmp = make_repo_with_identity();
        run_create_or_refresh(tmp.path(), false).unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        run_create_or_refresh(tmp.path(), true).unwrap();
        let refreshed = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(refreshed.cutoff_commit.as_deref(), Some("a3b2ea4e"));
    }

    #[test]
    fn verify_reports_loaded_baseline() {
        let tmp = make_repo_with_identity();
        run_create_or_refresh(tmp.path(), false).unwrap();
        // Should not error.
        run_verify(tmp.path()).unwrap();
    }

    #[test]
    fn verify_returns_error_when_no_baseline() {
        let tmp = make_repo_with_identity();
        let err = run_verify(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("no baseline"));
    }

    #[test]
    fn create_without_project_id_returns_error() {
        let tmp = TempDir::new().unwrap();
        let err = run_create_or_refresh(tmp.path(), false).unwrap_err();
        assert!(err.to_string().contains("project-id"));
    }
}
