//! `anvil hook <kind>` runtime subcommands (MLP-003 / -005 / -008 CLI lane).
//!
//! Hooks fire from git (directly or via Husky / Lefthook / pre-commit
//! framework) and execute the matching `anvil-hook` primitive plus
//! `anvil-witness` append. This module composes the libraries:
//! `anvil-hook` (verdict / suppression / framework / shell / panic
//! / post-hook parsers / bootstrap plan), `anvil-witness` (chain
//! append + verify), and `anvil-baseline` (load recorded findings
//! when the scanner integration follow-up lands).
//!
//! ## What ships here in v1
//!
//! - `anvil hook pre-commit` — appends a `validation_at: "pre-commit"`
//!   witness line. Scanner integration is stubbed; the verdict is
//!   always `Pass` until the rule-engine integration follow-up
//!   threads `anvil-checks` in (the witness-line shape is ready).
//! - `anvil hook post-commit` — appends a `kind: "post-commit"`
//!   bookkeeping line.
//! - `anvil hook post-merge` — builds a DAG-aware witness via
//!   [`anvil_hook::merge_witness_plan`] and appends with the
//!   `parent_commits[]` + `prev_line_hashes[]` arrays populated.
//! - `anvil hook post-rewrite` — reads git's `<old> <new>` stdin via
//!   [`anvil_hook::parse_post_rewrite_input`] and writes one
//!   retroactive witness per rewrite pair, tagged
//!   [`anvil_hook::POST_REWRITE_VALIDATION_AT`].
//! - `anvil hook bootstrap` — executes the
//!   [`anvil_hook::BootstrapPlan`]: regenerate `.husky/_/` shims or
//!   install `.git/hooks/<kind>` files, emit the one-line success
//!   message.
//!
//! ## Deliberately stubbed (follow-up PR)
//!
//! - **Rule-engine integration.** The pre-commit path emits
//!   `Verdict::Pass` with no scan. The next PR threads
//!   `anvil-checks` into this command, filters findings against
//!   `anvil_baseline::Baseline`, and converts the partition into
//!   `Verdict::Warn` / `Verdict::Block`.
//! - **Daemon RPC + embedded fallback.** Currently embedded-only.
//! - **Kindling `action_executed` emission on post-commit.**

// The `unnecessary_wraps` warning fires on `fn -> Result<()>` that
// always returns `Ok(())` today — but every helper here will gain
// real failure paths when the scanner / daemon-RPC follow-up lands.
// Pinning the signature now keeps the dispatch table stable.
#![allow(clippy::unnecessary_wraps)]
// `let...else` is a fine refactor for several matches here but rewrites
// noisy hook code into noisier hook code; the explicit match shape
// makes the Ok(None) vs Err(_) branches obvious in review.
#![allow(clippy::manual_let_else)]
// `items_after_statements` triggers on the test-only `use io::Write`
// inside `append_panic_log`; moving it module-level pulls a trait
// alias into the public surface for one function.
#![allow(clippy::items_after_statements)]

use std::fs;
use std::io::{self, Read};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command;

use anvil_baseline::load as load_baseline;
use anvil_hook::{
    BlockReason, BootstrapPlan, ErrorClass, MergeWitnessPlan, PanicReport, RewritePair,
    SuppressionKey, SuppressionLog, Verdict, build_bootstrap_plan, detect_framework,
    format_panic_report, merge_witness_plan, parse_post_rewrite_input, render_success_message,
    render_verdict,
};
use anvil_witness::{GenesisAnchor, RolloverPolicy, WitnessLine, WitnessWriter, verify_chain};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::GlobalArgs;
use crate::activation::identity::read_project_id;

#[derive(Debug, Args)]
pub struct HookArgs {
    #[command(subcommand)]
    command: HookCommand,
}

#[derive(Debug, Subcommand)]
enum HookCommand {
    /// L3 pre-commit hook — validates the staged diff and appends a
    /// witness line. ADR-038 noise discipline: silent on pass, one
    /// line otherwise.
    PreCommit(SilentArgs),
    /// post-commit hook — records that the commit succeeded.
    PostCommit(SilentArgs),
    /// post-merge hook — appends a DAG-aware witness for merge
    /// joins.
    PostMerge(PostMergeArgs),
    /// post-rewrite hook — regenerates witnesses for amended /
    /// rebased commits. Reads the rewrite pairs from stdin.
    PostRewrite(PostRewriteArgs),
    /// Recover hook-runtime files in a worktree that hasn't been
    /// bootstrapped yet (e.g. fresh clone before `pnpm install`).
    Bootstrap(BootstrapArgs),
}

#[derive(Debug, Args, Default)]
struct SilentArgs {}

#[derive(Debug, Args, Default)]
struct PostMergeArgs {
    /// Merge-commit SHA being witnessed.
    #[arg(long)]
    commit: Option<String>,
}

#[derive(Debug, Args, Default)]
struct PostRewriteArgs {
    /// Hook trigger (`amend` or `rebase`). Informational; the
    /// witness shape is the same either way.
    #[arg(value_name = "TRIGGER", default_value = "amend")]
    _trigger: String,
}

#[derive(Debug, Args, Default)]
struct BootstrapArgs {
    /// Print the plan rather than executing.
    #[arg(long)]
    dry_run: bool,
}

pub fn run(args: &HookArgs, _global: &GlobalArgs) -> Result<()> {
    let repo_root = std::env::current_dir().context("resolve repo root")?;
    install_panic_catcher();
    // One suppression log lives for the lifetime of the hook process
    // so ADR-038 §D-1 burst-suppression actually fires: a daemon-down
    // burst across 82 commits collapses to one emit instead of 82.
    let mut sup = SuppressionLog::new();

    // catch_unwind so a panic deep in the hook body cannot bubble out
    // to git's exit code (101 by default). ADR-038 §D-7: internal
    // errors must not hold the user hostage. The panic hook installed
    // above already wrote the structured log + the one-line stderr
    // message; we just need to swallow the unwind and exit zero.
    let result = catch_unwind(AssertUnwindSafe(|| match &args.command {
        HookCommand::PreCommit(_) => run_pre_commit(&repo_root, &mut sup),
        HookCommand::PostCommit(_) => run_post_commit(&repo_root),
        HookCommand::PostMerge(a) => run_post_merge(&repo_root, a),
        HookCommand::PostRewrite(a) => run_post_rewrite(&repo_root, a, &mut sup),
        HookCommand::Bootstrap(a) => run_bootstrap(&repo_root, a),
    }));
    match result {
        Ok(inner) => inner,
        // Panic: log + stderr already happened in the panic hook;
        // return Ok so the process exits 0, never 101.
        Err(_) => Ok(()),
    }
}

fn run_pre_commit(repo_root: &Path, sup: &mut SuppressionLog) -> Result<()> {
    let identity = match read_project_id(repo_root) {
        Ok(Some(id)) => id,
        // No project-id → Anvil hasn't activated here; the hook is a
        // no-op rather than a noisy failure (Serena rule).
        Ok(None) => return Ok(()),
        Err(_) => {
            emit_internal(ErrorClass::EmbeddedFailed, sup);
            return Ok(());
        }
    };

    // Baseline load — surface failures as a single InternalError so a
    // broken baseline.json doesn't go unnoticed but also doesn't block
    // commits. Scanner integration follow-up will consume the loaded
    // value; today we only need the error visibility.
    if let Err(_e) = load_baseline(repo_root) {
        // TOCTOU/symlink refusal or format errors land here. Suppress
        // repeats within a session so a noisy state doesn't spam the
        // terminal.
        emit_internal(ErrorClass::EmbeddedFailed, sup);
    }

    let result = append_witness(repo_root, &identity.project_uuid, |seq, prev| {
        build_witness_line(&identity.project_uuid, None, "pre-commit", seq, prev)
    });
    match result {
        Ok(()) => {
            let rendered = render_verdict(&Verdict::Pass);
            if !rendered.stderr_line.is_empty() {
                eprintln!("{}", rendered.stderr_line);
            }
            Ok(())
        }
        Err(AppendError::ChainBroken) => {
            // ADR-038: chain integrity break refuses the commit. We
            // do NOT reseed (which would obliterate evidence). The
            // operator runs `anvil hook bootstrap --witness-recent`
            // (future follow-up) to repair.
            let rendered = render_verdict(&Verdict::Block {
                count: 0,
                witness_id: identity.project_uuid.clone(),
                reason: BlockReason::ChainBroken,
            });
            eprintln!("{}", rendered.stderr_line);
            std::process::exit(rendered.exit_code);
        }
        Err(AppendError::WriteFailed) => {
            let rendered = render_verdict(&Verdict::WitnessWriteFailed);
            eprintln!("{}", rendered.stderr_line);
            std::process::exit(rendered.exit_code);
        }
    }
}

fn run_post_commit(repo_root: &Path) -> Result<()> {
    let identity = match read_project_id(repo_root) {
        Ok(Some(id)) => id,
        Ok(None) | Err(_) => return Ok(()),
    };
    let _ = append_witness(repo_root, &identity.project_uuid, |seq, prev| {
        let mut line = build_witness_line(&identity.project_uuid, None, "post-commit", seq, prev);
        line.kind = "post-commit".to_string();
        line
    });
    Ok(())
}

fn run_post_merge(repo_root: &Path, args: &PostMergeArgs) -> Result<()> {
    let identity = match read_project_id(repo_root) {
        Ok(Some(id)) => id,
        Ok(None) | Err(_) => return Ok(()),
    };
    let merge_ref = args.commit.clone().unwrap_or_else(|| "HEAD".to_string());
    let (merge_sha, parents) = resolve_merge_parents(repo_root, &merge_ref);
    // Per-parent chain-head lookup is the next follow-up — it
    // requires walking each parent's witness chain back to its tip,
    // which lives in the cross-branch verification path. For now we
    // emit `None` for every parent's `prev_line_hash` so the witness
    // is honest about the missing edges rather than fabricating a
    // single empty array.
    let parent_pairs: Vec<(String, Option<String>)> =
        parents.into_iter().map(|p| (p, None)).collect();
    let plan = merge_witness_plan(merge_sha, parent_pairs);
    let _ = append_witness(repo_root, &identity.project_uuid, |seq, prev| {
        build_merge_witness_line(&identity.project_uuid, plan.clone(), seq, prev)
    });
    Ok(())
}

/// Resolve the merge commit and its parents via git plumbing.
///
/// Returns `(canonical_sha, parents)`. When git invocation fails or
/// the ref doesn't resolve, falls back to `(merge_ref, vec![])` so
/// the witness still records the event with an honest empty parent
/// list rather than panicking.
fn resolve_merge_parents(repo_root: &Path, merge_ref: &str) -> (String, Vec<String>) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-list")
        .arg("--parents")
        .arg("-n")
        .arg("1")
        .arg(merge_ref)
        .output();
    let Ok(output) = output else {
        return (merge_ref.to_string(), Vec::new());
    };
    if !output.status.success() {
        return (merge_ref.to_string(), Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().unwrap_or("").trim();
    let mut tokens = line.split_whitespace();
    let merge_sha = match tokens.next() {
        Some(s) => s.to_string(),
        None => return (merge_ref.to_string(), Vec::new()),
    };
    let parents: Vec<String> = tokens.map(String::from).collect();
    (merge_sha, parents)
}

fn run_post_rewrite(
    repo_root: &Path,
    _args: &PostRewriteArgs,
    sup: &mut SuppressionLog,
) -> Result<()> {
    let identity = match read_project_id(repo_root) {
        Ok(Some(id)) => id,
        Ok(None) | Err(_) => return Ok(()),
    };
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("read post-rewrite stdin")?;
    let pairs = if let Ok(p) = parse_post_rewrite_input(&input) {
        p
    } else {
        emit_internal(ErrorClass::EmbeddedFailed, sup);
        return Ok(());
    };
    for pair in pairs {
        let _ = append_witness(repo_root, &identity.project_uuid, |seq, prev| {
            build_rewrite_witness_line(&identity.project_uuid, &pair, seq, prev)
        });
    }
    Ok(())
}

fn run_bootstrap(repo_root: &Path, args: &BootstrapArgs) -> Result<()> {
    let framework = detect_framework(repo_root);
    let plan = build_bootstrap_plan(framework);
    if args.dry_run {
        println!(
            "anvil hook bootstrap (dry-run): framework={}",
            framework.id()
        );
        match &plan {
            BootstrapPlan::HuskyRegenerate { files } => {
                for f in files {
                    println!("  would write {}", f.relative_path);
                }
            }
            BootstrapPlan::InstallPlain { files } => {
                for f in files {
                    println!("  would install .git/hooks/{}", f.filename);
                }
            }
            BootstrapPlan::NothingToDo { framework } => {
                println!("  nothing to do for framework={}", framework.id());
            }
        }
        return Ok(());
    }
    let _files_written = execute_bootstrap_plan(repo_root, &plan)?;
    // The success-message counter is for retroactive witnesses
    // generated by `--witness-recent` (deferred follow-up), not for
    // installed hook files. Until that ships, report zero so the
    // user-facing line stays honest: `anvil: bootstrapped`.
    eprintln!("{}", render_success_message(0));
    Ok(())
}

fn execute_bootstrap_plan(repo_root: &Path, plan: &BootstrapPlan) -> Result<usize> {
    match plan {
        BootstrapPlan::HuskyRegenerate { files } => {
            let mut written = 0;
            for f in files {
                let target = repo_root.join(&f.relative_path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).context("create .husky/_/")?;
                }
                fs::write(&target, &f.contents)
                    .with_context(|| format!("write {}", target.display()))?;
                #[cfg(unix)]
                if f.executable {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&target)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&target, perms)?;
                }
                written += 1;
            }
            Ok(written)
        }
        BootstrapPlan::InstallPlain { files } => {
            let hooks_dir = repo_root.join(".git").join("hooks");
            fs::create_dir_all(&hooks_dir).context("create .git/hooks")?;
            let mut written = 0;
            for f in files {
                let target = hooks_dir.join(&f.filename);
                fs::write(&target, &f.contents)
                    .with_context(|| format!("write {}", target.display()))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&target)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&target, perms)?;
                }
                written += 1;
            }
            Ok(written)
        }
        BootstrapPlan::NothingToDo { .. } => Ok(0),
    }
}

/// Result of [`chain_head`] — either a usable `(next_seq, prev)`
/// pair or a typed failure that the caller turns into the matching
/// [`Verdict`].
enum ChainState {
    /// The chain is empty: no active file. Caller should seed a
    /// genesis line before its first record.
    Empty,
    /// The chain is healthy. Append at this seq with this prev hash.
    Healthy { seq: u64, prev: String },
    /// The active file exists but verification failed. Per ADR-038
    /// the next commit is refused; we MUST NOT reseed (that would
    /// erase evidence of tampering).
    Broken,
}

/// Errors returned by [`append_witness`]. Mapped to [`Verdict`]
/// variants by the caller.
#[derive(Debug)]
enum AppendError {
    /// Hash chain integrity is broken on the existing active file.
    /// ADR-038: refuse the commit; do not reseed.
    ChainBroken,
    /// Writer surface failed (permissions, disk full, symlink
    /// refusal). ADR-038: "we don't claim what we can't witness."
    WriteFailed,
}

fn append_witness<F>(
    repo_root: &Path,
    project_uuid: &str,
    build: F,
) -> std::result::Result<(), AppendError>
where
    F: FnOnce(u64, String) -> WitnessLine,
{
    let writer = WitnessWriter::open(repo_root, "active", RolloverPolicy::default())
        .map_err(|_| AppendError::WriteFailed)?;
    let active = writer.active_path();
    match chain_head(&active) {
        ChainState::Broken => Err(AppendError::ChainBroken),
        ChainState::Empty => {
            // Fresh chain — seed genesis then chain off it.
            let genesis = WitnessLine::genesis(
                &GenesisAnchor::Fresh,
                project_uuid,
                "active",
                chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                "pre-commit",
            );
            writer
                .append(&genesis)
                .map_err(|_| AppendError::WriteFailed)?;
            let (seq, prev) = match chain_head(&active) {
                ChainState::Healthy { seq, prev } => (seq, prev),
                // If the chain we just wrote can't be re-verified,
                // something is badly wrong with the on-disk state.
                _ => return Err(AppendError::WriteFailed),
            };
            let line = build(seq, prev);
            writer
                .append(&line)
                .map(|_| ())
                .map_err(|_| AppendError::WriteFailed)
        }
        ChainState::Healthy { seq, prev } => {
            let line = build(seq, prev);
            writer
                .append(&line)
                .map(|_| ())
                .map_err(|_| AppendError::WriteFailed)
        }
    }
}

/// Classify the on-disk chain state for the next append.
///
/// Distinguishes the three cases the writer cares about:
/// `Empty` (no file → seed genesis), `Healthy` (verifiable tip →
/// chain off it), and `Broken` (file exists but fails
/// `verify_chain` → refuse, do NOT reseed). The earlier version
/// collapsed `Broken` into `Empty` and would obliterate evidence
/// by appending a fresh genesis on top of a tampered chain; the
/// distinction here is the ADR-038 contract.
fn chain_head(active_path: &Path) -> ChainState {
    if !active_path.exists() {
        return ChainState::Empty;
    }
    match verify_chain(&[active_path]) {
        Ok(report) => {
            if report.line_count == 0 {
                ChainState::Empty
            } else {
                let seq = report.line_count.saturating_add(1);
                let prev = report
                    .tip_hash
                    .unwrap_or_else(|| GenesisAnchor::Fresh.anchor_string().to_string());
                ChainState::Healthy { seq, prev }
            }
        }
        Err(_) => ChainState::Broken,
    }
}

fn install_panic_catcher() {
    // Resolve the state dir lazily so a failure during dir resolution
    // doesn't itself panic inside the hook. ADR-038 §D-7: log lives
    // out-of-tree (`~/.local/state/anvil/intercept-panic.log` on
    // Linux, `%LOCALAPPDATA%\anvil` on Windows) to avoid polluting
    // the repo and to keep ops tooling consistent across worktrees.
    std::panic::set_hook(Box::new(|info| {
        let report = format_panic_report(info);
        let _ = append_panic_log(&report);
        eprintln!("anvil: hook errored (anvil doctor for details)");
    }));
}

fn panic_log_path() -> Option<PathBuf> {
    // ADR-038 §D-7 pins this to a state-dir path, not a config dir.
    // `dirs::state_dir` returns `None` on platforms without an
    // analogous concept (e.g. some Windows shells); fall back to the
    // local data dir so the log always has a home.
    let base = dirs::state_dir().or_else(dirs::data_local_dir)?;
    Some(base.join("anvil").join(anvil_hook::PANIC_LOG_FILE))
}

fn append_panic_log(report: &PanicReport) -> Result<()> {
    let Some(log_path) = panic_log_path() else {
        return Ok(()); // No usable state dir; silently drop.
    };
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let body = report.log_text.replace("{ts}", &ts);
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_path)?;
    f.write_all(body.as_bytes())?;
    Ok(())
}

fn emit_internal(class: ErrorClass, sup: &mut SuppressionLog) {
    let key = SuppressionKey::from_class(class);
    if sup.should_emit(&key) {
        let rendered = render_verdict(&Verdict::InternalError { class });
        if !rendered.stderr_line.is_empty() {
            eprintln!("{}", rendered.stderr_line);
        }
    }
}

fn build_witness_line(
    project_uuid: &str,
    commit_sha: Option<String>,
    validation_at: &str,
    seq: u64,
    prev_line_hash: String,
) -> WitnessLine {
    WitnessLine {
        seq,
        scope: "active".to_string(),
        kind: "witness".to_string(),
        prev_line_hash,
        project_uuid: project_uuid.to_string(),
        commit_sha,
        parent_commits: Vec::new(),
        prev_line_hashes: Vec::new(),
        agent_tag: None,
        rules_sha: None,
        ts: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        validation_at: validation_at.to_string(),
    }
}

fn build_merge_witness_line(
    project_uuid: &str,
    plan: MergeWitnessPlan,
    seq: u64,
    prev_line_hash: String,
) -> WitnessLine {
    WitnessLine {
        seq,
        scope: "active".to_string(),
        kind: "witness".to_string(),
        prev_line_hash,
        project_uuid: project_uuid.to_string(),
        commit_sha: Some(plan.merge_commit_sha),
        parent_commits: plan.parent_commits,
        prev_line_hashes: plan.prev_line_hashes,
        agent_tag: None,
        rules_sha: None,
        ts: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        validation_at: "post-merge".to_string(),
    }
}

fn build_rewrite_witness_line(
    project_uuid: &str,
    pair: &RewritePair,
    seq: u64,
    prev_line_hash: String,
) -> WitnessLine {
    WitnessLine {
        seq,
        scope: "active".to_string(),
        kind: "witness".to_string(),
        prev_line_hash,
        project_uuid: project_uuid.to_string(),
        commit_sha: Some(pair.new_sha.clone()),
        parent_commits: Vec::new(),
        prev_line_hashes: Vec::new(),
        agent_tag: None,
        rules_sha: None,
        ts: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        validation_at: anvil_hook::POST_REWRITE_VALIDATION_AT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_test_repo() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        // Write a minimal anvil/project-id so read_project_id succeeds.
        fs::create_dir_all(root.join("anvil")).unwrap();
        fs::write(
            root.join("anvil").join("project-id"),
            "project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456\n",
        )
        .unwrap();
        (tmp, root)
    }

    #[test]
    fn append_witness_creates_chain_with_genesis_and_record() {
        let (_tmp, root) = make_test_repo();
        let writer = WitnessWriter::open(&root, "active", RolloverPolicy::default()).unwrap();
        let active = writer.active_path();
        drop(writer);

        append_witness(
            &root,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            |seq, prev| {
                build_witness_line(
                    "01997e4a-1b2c-7345-8901-abcdef123456",
                    Some("commit-sha-1".to_string()),
                    "pre-commit",
                    seq,
                    prev,
                )
            },
        )
        .unwrap();

        let contents = fs::read_to_string(&active).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "expected genesis + record");
        // Genesis is seq=1; record is seq=2.
        assert!(lines[0].contains("\"seq\":1"));
        assert!(lines[1].contains("\"seq\":2"));
        assert!(lines[1].contains("\"commit_sha\":\"commit-sha-1\""));
    }

    #[test]
    fn append_witness_chains_subsequent_records_onto_tip() {
        let (_tmp, root) = make_test_repo();
        let writer = WitnessWriter::open(&root, "active", RolloverPolicy::default()).unwrap();
        let active = writer.active_path();
        drop(writer);

        for i in 0..3 {
            append_witness(
                &root,
                "01997e4a-1b2c-7345-8901-abcdef123456",
                |seq, prev| {
                    build_witness_line(
                        "01997e4a-1b2c-7345-8901-abcdef123456",
                        Some(format!("commit-{i}")),
                        "pre-commit",
                        seq,
                        prev,
                    )
                },
            )
            .unwrap();
        }

        // Verify the resulting chain is intact.
        let report = verify_chain(&[active.as_path()]).expect("chain verifies");
        // 1 genesis + 3 records = 4 lines.
        assert_eq!(report.line_count, 4);
    }

    #[test]
    fn post_merge_witness_carries_dag_arrays_when_parents_provided() {
        let (_tmp, root) = make_test_repo();
        let writer = WitnessWriter::open(&root, "active", RolloverPolicy::default()).unwrap();
        let active = writer.active_path();
        drop(writer);

        let plan = merge_witness_plan(
            "merge-sha".to_string(),
            [
                ("parent-a".to_string(), Some("hash-a".to_string())),
                ("parent-b".to_string(), Some("hash-b".to_string())),
            ],
        );
        append_witness(
            &root,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            |seq, prev| {
                build_merge_witness_line(
                    "01997e4a-1b2c-7345-8901-abcdef123456",
                    plan.clone(),
                    seq,
                    prev,
                )
            },
        )
        .unwrap();

        let contents = fs::read_to_string(&active).unwrap();
        let merge_line = contents.lines().last().unwrap();
        assert!(merge_line.contains("\"parent_commits\":[\"parent-a\",\"parent-b\"]"));
        assert!(merge_line.contains("\"prev_line_hashes\":[\"hash-a\",\"hash-b\"]"));
        assert!(merge_line.contains("\"validation_at\":\"post-merge\""));
    }

    #[test]
    fn bootstrap_dry_run_plain_repo_does_not_write_files() {
        let (_tmp, root) = make_test_repo();
        run_bootstrap(&root, &BootstrapArgs { dry_run: true }).unwrap();
        assert!(!root.join(".git").join("hooks").exists());
    }

    #[test]
    fn bootstrap_plain_repo_installs_all_five_v1_hooks() {
        let (_tmp, root) = make_test_repo();
        run_bootstrap(&root, &BootstrapArgs { dry_run: false }).unwrap();
        for kind in [
            "pre-commit",
            "post-commit",
            "pre-push",
            "post-merge",
            "post-rewrite",
        ] {
            let path = root.join(".git").join("hooks").join(kind);
            assert!(path.is_file(), "missing {kind}");
            let contents = fs::read_to_string(&path).unwrap();
            assert!(contents.starts_with("#!/bin/sh"));
            assert!(contents.contains("exec anvil hook"));
        }
    }

    #[test]
    fn pre_commit_with_no_project_id_is_a_no_op() {
        let tmp = TempDir::new().unwrap();
        let mut sup = SuppressionLog::new();
        // No anvil/project-id written.
        run_pre_commit(tmp.path(), &mut sup).unwrap();
        // No witness file should exist.
        assert!(
            !tmp.path()
                .join("anvil")
                .join("witness")
                .join("active.ndjson")
                .exists()
        );
    }

    #[test]
    fn post_rewrite_writes_one_witness_per_pair() {
        let (_tmp, root) = make_test_repo();
        let writer = WitnessWriter::open(&root, "active", RolloverPolicy::default()).unwrap();
        let active = writer.active_path();
        drop(writer);

        // Drive run_post_rewrite by hand via the parser + builder
        // pipeline (the real command reads stdin, which the test
        // harness isn't well set up to feed).
        let pairs = parse_post_rewrite_input("old1 new1\nold2 new2\n").unwrap();
        for pair in pairs {
            append_witness(
                &root,
                "01997e4a-1b2c-7345-8901-abcdef123456",
                |seq, prev| {
                    build_rewrite_witness_line(
                        "01997e4a-1b2c-7345-8901-abcdef123456",
                        &pair,
                        seq,
                        prev,
                    )
                },
            )
            .unwrap();
        }

        let report = verify_chain(&[active.as_path()]).unwrap();
        // genesis + two retroactive records.
        assert_eq!(report.line_count, 3);
        let contents = fs::read_to_string(&active).unwrap();
        assert!(contents.contains("\"commit_sha\":\"new1\""));
        assert!(contents.contains("\"commit_sha\":\"new2\""));
        assert!(contents.contains("\"validation_at\":\"post-rewrite-recovery\""));
    }

    #[test]
    fn append_witness_refuses_corrupted_chain_rather_than_reseeding() {
        // ADR-038: a tampered chain must surface as ChainBroken
        // refusal; the hook MUST NOT obliterate evidence by appending
        // a fresh genesis on top.
        let (_tmp, root) = make_test_repo();
        // Seed a normal chain first.
        append_witness(
            &root,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            |seq, prev| {
                build_witness_line(
                    "01997e4a-1b2c-7345-8901-abcdef123456",
                    Some("commit-a".to_string()),
                    "pre-commit",
                    seq,
                    prev,
                )
            },
        )
        .unwrap();
        // Tamper: overwrite the active file with byte-noise that no
        // longer parses as the chain.
        let active = root.join("anvil").join("witness").join("active.ndjson");
        fs::write(&active, "not-valid-ndjson\n").unwrap();
        // Now an append should refuse, NOT reseed.
        let err = append_witness(
            &root,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            |seq, prev| {
                build_witness_line(
                    "01997e4a-1b2c-7345-8901-abcdef123456",
                    Some("commit-b".to_string()),
                    "pre-commit",
                    seq,
                    prev,
                )
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppendError::ChainBroken));
        // The tampered file must still be the tampered byte-noise —
        // not silently reseeded.
        let contents = fs::read_to_string(&active).unwrap();
        assert_eq!(contents, "not-valid-ndjson\n");
    }

    #[test]
    fn emit_internal_suppresses_repeats_within_session() {
        // Burst-suppression contract: same class fires once per
        // SuppressionLog lifetime.
        let mut sup = SuppressionLog::new();
        let key = SuppressionKey::from_class(ErrorClass::DaemonUnreachable);
        assert!(sup.should_emit(&key));
        assert!(!sup.should_emit(&key));
        // emit_internal itself uses should_emit; calling it 82 times
        // with the same sup must not blow up — that's the regression
        // we're guarding against (the old code created a fresh
        // SuppressionLog per call, so it'd emit every time).
        for _ in 0..82 {
            emit_internal(ErrorClass::DaemonUnreachable, &mut sup);
        }
        // Sanity: the burst didn't reset the log.
        assert!(!sup.should_emit(&key));
    }
}
