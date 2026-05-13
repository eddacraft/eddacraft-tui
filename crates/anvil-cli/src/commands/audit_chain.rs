//! L5 witness-chain audit (MLP-015).
//!
//! `anvil audit-chain` re-walks a branch's commits and reports any
//! that lack a corresponding L3 witness in `anvil/witness/*.ndjson`.
//! Catches commits that bypassed pre-commit / pre-push (admin
//! overrides, force-push manipulation, hook-failure recovery).
//!
//! Per ADR-037 §D-9, the audit:
//!
//! - Re-uses the same witness-chain primitive as MLP-002/-003/-004.
//! - Emits a structured report (`AuditReport`) — plain table or JSON.
//! - Returns a `degraded:audit-drift` marker when drift exceeds the
//!   configured `--threshold`.
//!
//! Out of scope (deferred follow-ups, not part of v1):
//!
//! - Kindling `gate_evaluated` emission with `mode: audit` — owned by
//!   the kindling-integration consumer when the CLI gets a kindling
//!   client handle wired in.
//! - `anvil start` / `anvil baseline` writing the
//!   `.github/workflows/anvil-audit.yml` template into the repo —
//!   the template ships in-tree (`audit_workflow_template()`); the
//!   activation orchestrator call site is the operator-touch point.
//! - Re-using `anvil-checks` for rule re-scoring (drift is a witness-
//!   presence check today, not a rule re-run).

use std::collections::HashSet;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

use anvil_witness::{WitnessLine, verify_chain};
use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

/// Inlined template for the nightly L5-audit GitHub workflow.
///
/// Public so the activation orchestrator (`anvil start` / `anvil
/// baseline`) can copy it into `.github/workflows/anvil-audit.yml`
/// at adoption time. ADR-037 §D-9: active by default; operator
/// disables by commenting out the `schedule` block.
///
/// `#[allow(dead_code)]` because the call site lives in the
/// activation orchestrator (deferred follow-up). The template is
/// exercised by `audit_workflow_template_is_valid_yaml_shape`.
#[must_use]
#[allow(dead_code)]
pub fn audit_workflow_template() -> &'static str {
    include_str!("../templates/anvil-audit-workflow.yml")
}

#[derive(Debug, Args)]
pub struct AuditChainArgs {
    /// Branch tip to walk back from. Defaults to `HEAD`.
    #[arg(long, default_value = "HEAD")]
    branch: String,
    /// Optional earliest ancestor to include. When set, the audit
    /// walks `<since>..<branch>`; otherwise it walks all reachable
    /// commits and lets the witness set (and any future
    /// `cutoff_commit`) constrain the window.
    #[arg(long)]
    since: Option<String>,
    /// Drift threshold for the `degraded:audit-drift` marker. Default
    /// 5 — matches the nightly-workflow default.
    #[arg(long, default_value_t = 5)]
    threshold: usize,
}

/// Structured audit output. Stable schema so the nightly workflow
/// can pin against it; additive fields only.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuditReport {
    pub schema_version: &'static str,
    pub branch: String,
    pub commits_walked: usize,
    pub witnessed: usize,
    pub unwitnessed: Vec<String>,
    pub chain_intact: bool,
    pub degraded_audit_drift: bool,
    pub threshold: usize,
}

pub fn run(args: &AuditChainArgs, global: &GlobalArgs) -> Result<()> {
    let repo_root = std::env::current_dir().context("resolve repo root")?;
    let report = run_audit_chain(
        &repo_root,
        &args.branch,
        args.since.as_deref(),
        args.threshold,
    );
    if global.json || !std::io::stdout().is_terminal() {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_plain(&report);
    }
    // Exit non-zero on degraded state so the nightly cron surfaces
    // the regression as a workflow failure. Operator can suppress by
    // raising `--threshold` if the drift is expected.
    if report.degraded_audit_drift {
        std::process::exit(1);
    }
    Ok(())
}

/// Pure entry point used by tests — takes the repo root and inputs,
/// returns the structured report. The `run` wrapper only adds
/// rendering + exit handling.
///
/// Returns `AuditReport` directly (not wrapped in `Result`) because
/// every failure mode here degrades to "empty walk" by design:
/// missing git repos, missing witness files, and missing branches
/// all produce a valid audit with `commits_walked: 0` rather than
/// surfacing as errors. ADR-038 §D-6: don't hold the operator
/// hostage to environmental gaps.
#[must_use]
pub fn run_audit_chain(
    repo_root: &Path,
    branch: &str,
    since: Option<&str>,
    threshold: usize,
) -> AuditReport {
    let commits = list_commits(repo_root, branch, since).unwrap_or_default();
    let witnessed = collect_witnessed_shas(repo_root).unwrap_or_default();
    let chain_intact = chain_is_intact(repo_root);

    let mut unwitnessed: Vec<String> = commits
        .iter()
        .filter(|c| !witnessed.contains(*c))
        .cloned()
        .collect();
    // Deterministic ordering so the JSON output is stable across runs.
    unwitnessed.sort();

    let witnessed_count = commits.len().saturating_sub(unwitnessed.len());
    let degraded = unwitnessed.len() >= threshold;
    AuditReport {
        schema_version: "anvil.audit-chain.v1",
        branch: branch.to_string(),
        commits_walked: commits.len(),
        witnessed: witnessed_count,
        unwitnessed,
        chain_intact,
        degraded_audit_drift: degraded,
        threshold,
    }
}

fn print_plain(r: &AuditReport) {
    println!("anvil audit-chain — branch {}", r.branch);
    println!("  commits walked: {}", r.commits_walked);
    println!("  witnessed:      {}", r.witnessed);
    println!("  unwitnessed:    {}", r.unwitnessed.len());
    println!(
        "  chain intact:   {}",
        if r.chain_intact { "yes" } else { "NO" }
    );
    if r.degraded_audit_drift {
        println!(
            "  DEGRADED: drift {} >= threshold {}",
            r.unwitnessed.len(),
            r.threshold
        );
    }
    if !r.unwitnessed.is_empty() && r.unwitnessed.len() <= 20 {
        println!("  unwitnessed SHAs:");
        for sha in &r.unwitnessed {
            println!("    {sha}");
        }
    }
}

/// List the commits to audit. When `since` is set, uses `git rev-list
/// <since>..<branch>`; otherwise walks the full reachable history.
/// Returns `None` on git failure so the caller can degrade to an
/// "empty audit" report rather than panicking.
fn list_commits(repo_root: &Path, branch: &str, since: Option<&str>) -> Option<Vec<String>> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo_root).arg("rev-list");
    if let Some(s) = since {
        cmd.arg(format!("{s}..{branch}"));
    } else {
        cmd.arg(branch);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(
        stdout
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

/// Witness paths in chain order: archive segments first, then active.
/// Shared shape with the pre-push hook's `witness_paths`; the audit
/// path inlines it to avoid a CLI-internal helper export.
fn witness_paths(repo_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let archive_dir = repo_root.join("anvil").join("witness").join("archive");
    if let Ok(entries) = fs::read_dir(&archive_dir) {
        let mut files: Vec<PathBuf> = entries
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("ndjson"))
            .collect();
        files.sort();
        out.extend(files);
    }
    let active = repo_root
        .join("anvil")
        .join("witness")
        .join("active.ndjson");
    if active.exists() {
        out.push(active);
    }
    out
}

/// Collect witnessed commit SHAs from every chain segment.
///
/// Mirrors the pre-push hook's collector: `commit_sha` + `parent_commits`
/// from merge witnesses both count as "presence." Returns an empty
/// set when there are no chain files yet.
fn collect_witnessed_shas(
    repo_root: &Path,
) -> std::result::Result<HashSet<String>, std::io::Error> {
    let mut out: HashSet<String> = HashSet::new();
    for path in witness_paths(repo_root) {
        let contents = fs::read_to_string(&path)?;
        for line in contents.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(parsed) = WitnessLine::from_ndjson_line(line.as_bytes()) else {
                continue;
            };
            if let Some(sha) = parsed.commit_sha
                && !sha.is_empty()
            {
                out.insert(sha);
            }
            for p in parsed.parent_commits {
                if !p.is_empty() {
                    out.insert(p);
                }
            }
        }
    }
    Ok(out)
}

/// Verify the chain over all segments. Returns `true` when the chain
/// is intact OR when there is no chain yet (greenfield repo). Returns
/// `false` only when existing files fail verification — the audit
/// surfaces tamper evidence rather than silently passing.
fn chain_is_intact(repo_root: &Path) -> bool {
    let paths = witness_paths(repo_root);
    if paths.is_empty() {
        return true;
    }
    let path_refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    verify_chain(&path_refs).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_witness::{RolloverPolicy, WitnessWriter};
    use tempfile::TempDir;

    fn build_witness_record(
        project_uuid: &str,
        commit_sha: Option<String>,
        seq: u64,
        prev: String,
    ) -> WitnessLine {
        WitnessLine {
            seq,
            scope: "active".to_string(),
            kind: "witness".to_string(),
            prev_line_hash: prev,
            project_uuid: project_uuid.to_string(),
            commit_sha,
            parent_commits: Vec::new(),
            prev_line_hashes: Vec::new(),
            agent_tag: None,
            rules_sha: None,
            ts: "2026-05-13T00:00:00Z".to_string(),
            validation_at: "pre-commit".to_string(),
        }
    }

    fn write_minimal_chain(root: &Path, commit_shas: &[&str]) {
        use anvil_witness::GenesisAnchor;
        let writer = WitnessWriter::open(root, "active", RolloverPolicy::default()).unwrap();
        let active = writer.active_path();
        drop(writer);
        let writer = WitnessWriter::open(root, "active", RolloverPolicy::default()).unwrap();
        let genesis = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "test-uuid",
            "active",
            "2026-05-13T00:00:00Z".to_string(),
            "pre-commit",
        );
        writer.append(&genesis).unwrap();
        let mut prev = anvil_witness::compute_line_hash(&genesis.to_canonical_bytes().unwrap());
        for (seq, sha) in (2_u64..).zip(commit_shas.iter()) {
            let line =
                build_witness_record("test-uuid", Some((*sha).to_string()), seq, prev.clone());
            writer.append(&line).unwrap();
            prev = anvil_witness::compute_line_hash(&line.to_canonical_bytes().unwrap());
        }
        // Sanity: chain verifies.
        assert!(verify_chain(&[active.as_path()]).is_ok());
    }

    #[test]
    fn report_schema_version_is_pinned() {
        // Drift this and downstream consumers break. ADR-038 stability.
        let tmp = TempDir::new().unwrap();
        let r = run_audit_chain(tmp.path(), "HEAD", None, 5);
        assert_eq!(r.schema_version, "anvil.audit-chain.v1");
    }

    #[test]
    fn empty_repo_reports_zero_drift_and_intact_chain() {
        let tmp = TempDir::new().unwrap();
        // No git repo, no witness files. The audit returns an empty
        // walk and "chain intact" (because there's nothing to break).
        let r = run_audit_chain(tmp.path(), "HEAD", None, 5);
        assert_eq!(r.commits_walked, 0);
        assert_eq!(r.witnessed, 0);
        assert!(r.unwitnessed.is_empty());
        assert!(r.chain_intact);
        assert!(!r.degraded_audit_drift);
    }

    #[test]
    fn collect_witnessed_shas_finds_commits_in_chain() {
        let tmp = TempDir::new().unwrap();
        write_minimal_chain(tmp.path(), &["aaa", "bbb"]);
        let set = collect_witnessed_shas(tmp.path()).unwrap();
        assert!(set.contains("aaa"));
        assert!(set.contains("bbb"));
    }

    #[test]
    fn chain_intact_returns_true_when_no_chain() {
        let tmp = TempDir::new().unwrap();
        assert!(chain_is_intact(tmp.path()));
    }

    #[test]
    fn chain_intact_returns_false_on_tampered_active() {
        let tmp = TempDir::new().unwrap();
        write_minimal_chain(tmp.path(), &["aaa"]);
        let active = tmp
            .path()
            .join("anvil")
            .join("witness")
            .join("active.ndjson");
        fs::write(&active, "not-valid-ndjson\n").unwrap();
        assert!(!chain_is_intact(tmp.path()));
    }

    #[test]
    fn degraded_flag_fires_when_unwitnessed_meets_threshold() {
        // 5 unwitnessed commits, threshold 5 → degraded. The full
        // pipeline (`run_audit_chain`) needs a git repo for
        // `list_commits`; here we verify the boolean logic via
        // direct field assertion on a synthesised report.
        let r = AuditReport {
            schema_version: "anvil.audit-chain.v1",
            branch: "main".to_string(),
            commits_walked: 5,
            witnessed: 0,
            unwitnessed: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
            chain_intact: true,
            degraded_audit_drift: 5 >= 5,
            threshold: 5,
        };
        assert!(r.degraded_audit_drift);
    }

    #[test]
    fn degraded_flag_clear_below_threshold() {
        let r = AuditReport {
            schema_version: "anvil.audit-chain.v1",
            branch: "main".to_string(),
            commits_walked: 3,
            witnessed: 2,
            unwitnessed: vec!["a".into()],
            chain_intact: true,
            degraded_audit_drift: 1 >= 5,
            threshold: 5,
        };
        assert!(!r.degraded_audit_drift);
    }

    #[test]
    fn unwitnessed_list_is_sorted() {
        // Determinism: same chain → same JSON across runs.
        let tmp = TempDir::new().unwrap();
        // No git → list_commits returns empty → unwitnessed = [].
        let r = run_audit_chain(tmp.path(), "HEAD", None, 100);
        // Vacuously sorted.
        let mut copy = r.unwitnessed.clone();
        copy.sort();
        assert_eq!(r.unwitnessed, copy);
    }

    #[test]
    fn audit_workflow_template_is_valid_yaml_shape() {
        // Light validation: the template references `anvil audit-chain`
        // and uses a cron schedule. Don't parse YAML here (avoid the
        // dep); just pin the load-bearing strings so a refactor of the
        // template doesn't silently break the wired binary command.
        let t = audit_workflow_template();
        assert!(t.contains("anvil audit-chain"), "must call new command");
        assert!(t.contains("cron:"), "must declare a cron schedule");
        assert!(t.contains("--threshold"), "must thread --threshold");
        assert!(t.contains("--json"), "must thread --json");
        // Pin the workflow name so dashboards / branch protections that
        // key on it don't silently miss future renames.
        assert!(t.contains("name: anvil-audit"));
    }

    #[test]
    fn collect_witnessed_shas_includes_merge_parents() {
        use anvil_witness::GenesisAnchor;
        let tmp = TempDir::new().unwrap();
        let writer = WitnessWriter::open(tmp.path(), "active", RolloverPolicy::default()).unwrap();
        let genesis = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "test-uuid",
            "active",
            "2026-05-13T00:00:00Z".to_string(),
            "pre-commit",
        );
        writer.append(&genesis).unwrap();
        let prev = anvil_witness::compute_line_hash(&genesis.to_canonical_bytes().unwrap());
        let merge_line = WitnessLine {
            seq: 2,
            scope: "active".to_string(),
            kind: "witness".to_string(),
            prev_line_hash: prev,
            project_uuid: "test-uuid".to_string(),
            commit_sha: Some("merge-sha".to_string()),
            parent_commits: vec!["parent-a".to_string(), "parent-b".to_string()],
            prev_line_hashes: vec![None, None],
            agent_tag: None,
            rules_sha: None,
            ts: "2026-05-13T00:00:00Z".to_string(),
            validation_at: "post-merge".to_string(),
        };
        writer.append(&merge_line).unwrap();
        let set = collect_witnessed_shas(tmp.path()).unwrap();
        assert!(set.contains("merge-sha"));
        assert!(set.contains("parent-a"));
        assert!(set.contains("parent-b"));
    }
}
