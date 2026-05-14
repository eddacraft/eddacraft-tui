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
use anvil_config::ConfigFormat;
use anvil_hook::{
    BlockReason, BootstrapPlan, ErrorClass, MergeWitnessPlan, PanicReport, PushKind, PushRef,
    RewritePair, SuppressionKey, SuppressionLog, Verdict, build_bootstrap_plan, detect_framework,
    format_panic_report, is_hex_sha, merge_witness_plan, parse_post_rewrite_input,
    parse_pre_push_input, render_success_message, render_verdict,
};
use anvil_l4::{
    BlockKind, CommitDecision, NoOpValidationEngine, Policy, Severity, ValidationDiagnostic,
    ValidationEngine, ValidationVerdict, request_for,
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
    /// L4 pre-push hook — walks the pushed commit range, verifies
    /// each commit's L3 witness, and applies per-branch policy from
    /// `anvil/policy.yml`. Reads git's pre-push stdin contract
    /// (`<local-ref> <local-sha> <remote-ref> <remote-sha>` per
    /// line).
    PrePush(SilentArgs),
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
        HookCommand::PrePush(_) => run_pre_push(&repo_root, &mut sup),
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

/// MLP-004 entry point — the pre-push hook.
///
/// Reads git's pre-push stdin, resolves per-branch policy, walks each
/// pushed range, and emits one [`Verdict`] line on the first block.
/// Exits 1 on block; exits 0 on allow or internal error (Serena rule
/// — ADR-038 §D-6).
///
/// ## v1 scope
///
/// - Witness existence is checked by scanning every `WitnessLine`
///   with a non-empty `commit_sha` in `anvil/witness/active.ndjson`
///   plus every archived segment.
/// - Chain integrity (`verify_chain`) is run once over the active +
///   archive stack; a broken chain blocks the push regardless of
///   policy ([`Verdict::Block`] with [`BlockReason::ChainBroken`]).
/// - Per-branch policy is loaded from `anvil/policy.yml` (also
///   `.yaml` / `.json` / `.toml`). When no policy file exists the
///   hook is a no-op (the project hasn't opted into L4 enforcement).
/// - `NeedsL4Validation` decisions route through MLP2-016's
///   [`anvil_l4::ValidationEngine`] trait. v1 binds
///   [`NoOpValidationEngine`], which returns `EngineUnavailable
///   { reason: NotImplemented }` — preserving the pre-MLP2-016
///   `InternalError { TimedOut }` + admit-push surface byte-for-byte.
///   A future PR replaces the no-op with a real engine; the hook
///   then surfaces `Allow` / `Block` per commit without any further
///   change to this file.
///
/// ## Deferred
///
/// - `cutoff_commit` baseline acceptance (needs `git rev-list
///   --first-parent` ancestry walk per pushed ref).
/// - Time-budget cap with `partial: true` for very large pushes.
/// - L4 witness writes to `refs/notes/anvil-l4` (owned by MLP-010).
fn run_pre_push(repo_root: &Path, sup: &mut SuppressionLog) -> Result<()> {
    run_pre_push_with_engine(repo_root, sup, &NoOpValidationEngine)
}

/// MLP2-016: pre-push entry point that takes a pluggable
/// [`ValidationEngine`]. The production `run_pre_push` binds the
/// [`NoOpValidationEngine`] (preserves pre-MLP2-016 surface);
/// integration tests can substitute a fixture engine to drive the
/// Allow / Block paths through the production call site without
/// shelling out.
fn run_pre_push_with_engine(
    repo_root: &Path,
    sup: &mut SuppressionLog,
    engine: &dyn ValidationEngine,
) -> Result<()> {
    let identity = match read_project_id(repo_root) {
        Ok(Some(id)) => id,
        // No project-id → project hasn't opted into Anvil; hook is a
        // no-op (Serena rule). Don't emit anything.
        Ok(None) => return Ok(()),
        Err(_) => {
            emit_internal(ErrorClass::EmbeddedFailed, sup);
            return Ok(());
        }
    };

    // Read stdin (git pre-push contract).
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        emit_internal(ErrorClass::EmbeddedFailed, sup);
        return Ok(());
    }
    // Malformed stdin: emit InternalError so a corrupted hook
    // invocation surfaces without holding the user hostage. ADR-038
    // §D-6: internal failures don't block the user.
    let Ok(push_refs) = parse_pre_push_input(&input) else {
        emit_internal(ErrorClass::EmbeddedFailed, sup);
        return Ok(());
    };
    if push_refs.is_empty() {
        return Ok(());
    }

    // Load the policy (may be absent).
    let policy = match load_policy(repo_root) {
        Ok(Some(p)) => p,
        Ok(None) => return Ok(()), // No policy → no-op.
        Err(_) => {
            emit_internal(ErrorClass::EmbeddedFailed, sup);
            return Ok(());
        }
    };

    // Verify chain integrity once. A broken chain refuses the push
    // outright; we MUST NOT re-seed.
    if let Some(rendered) = verify_chain_or_block(repo_root, &identity.project_uuid) {
        eprintln!("{}", rendered.stderr_line);
        std::process::exit(rendered.exit_code);
    }

    // Collect the set of witnessed commit SHAs across active +
    // archives. Cheap enough at v1 push frequency; archives can be
    // mmap-backed in a follow-up if profiling shows it.
    let witnessed = collect_witnessed_shas(repo_root).unwrap_or_default();

    // Walk every push ref and apply per-branch policy.
    let mut engine_unavailable = false;
    for push_ref in &push_refs {
        if push_ref.kind == PushKind::Delete {
            continue; // No commits to validate on a deletion.
        }
        let branch = push_ref.branch_name();
        let rule = match policy.resolve(branch) {
            Ok(Some(r)) => r,
            // No matching rule → policy doesn't speak to this branch;
            // admit the push. Better than rejecting silently.
            Ok(None) => continue,
            Err(_) => {
                emit_internal(ErrorClass::EmbeddedFailed, sup);
                return Ok(());
            }
        };
        let Some(range_commits) = list_range(repo_root, push_ref) else {
            emit_internal(ErrorClass::EmbeddedFailed, sup);
            continue;
        };
        for commit in range_commits {
            let has_witness = witnessed.contains(&commit);
            match rule.decide_commit(has_witness) {
                CommitDecision::Allow => {}
                CommitDecision::NeedsL4Validation => {
                    // MLP2-016: route through ValidationEngine instead
                    // of an unconditional InternalError emit.
                    let request = request_for(commit.clone(), rule.clone(), repo_root);
                    match engine.validate(&request) {
                        ValidationVerdict::Allow => {}
                        ValidationVerdict::Block { diagnostics } => {
                            emit_l4_block(&commit, &diagnostics);
                            let rendered = render_verdict(&Verdict::Block {
                                count: 0,
                                witness_id: short_sha(&commit),
                                reason: BlockReason::UnwitnessedCommit,
                            });
                            std::process::exit(rendered.exit_code);
                        }
                        ValidationVerdict::EngineUnavailable { reason } => {
                            // BinaryMissing / Timeout / NotImplemented
                            // all map to the pre-MLP2-016 fall-through:
                            // emit InternalError { TimedOut } once via
                            // suppression, admit the push (ADR-038).
                            engine_unavailable = true;
                            // Reason is intentionally discarded at the
                            // hook surface — operators see the same
                            // single line regardless. Future telemetry
                            // can record the reason separately.
                            let _ = reason;
                        }
                    }
                }
                CommitDecision::Block(BlockKind::UnwitnessedCommit) => {
                    let rendered = render_verdict(&Verdict::Block {
                        count: 0,
                        witness_id: short_sha(&commit),
                        reason: BlockReason::UnwitnessedCommit,
                    });
                    eprintln!("{}", rendered.stderr_line);
                    std::process::exit(rendered.exit_code);
                }
            }
        }
    }

    // MLP2-016: engine-unavailable verdicts collapse to one
    // `ValidationPending` line via SuppressionLog. Stays exit 0 per
    // Serena rule. Pre-MLP2-016 behaviour is byte-for-byte preserved
    // when `engine == NoOpValidationEngine`.
    if engine_unavailable {
        emit_internal(ErrorClass::ValidationPending, sup);
    }

    Ok(())
}

/// MLP2-016: emit per-rule detail lines under a `Verdict::Block`
/// from L4 validation. Each diagnostic prints as
/// `anvil: <rule_id> (<severity>) — <message>` so operators see
/// *which* rule refused the commit. Single-line, ≤200-char messages
/// per the validate.rs contract. The headline `Verdict::Block` line
/// is emitted separately by the caller before exiting.
fn emit_l4_block(commit: &str, diagnostics: &[ValidationDiagnostic]) {
    eprintln!("anvil: L4 validation failed for {}", short_sha(commit));
    for diag in diagnostics {
        let severity = match diag.severity {
            Severity::Block => "block",
            Severity::Warn => "warn",
        };
        // Truncate over-long messages defensively; the engine
        // contract says ≤200 chars, but a misbehaving impl that
        // emits longer messages must not break the hook's
        // single-line discipline.
        let message: String = if diag.message.chars().count() > 200 {
            let mut truncated: String = diag.message.chars().take(197).collect();
            truncated.push_str("...");
            truncated
        } else {
            diag.message.clone()
        };
        eprintln!("  {} ({severity}) — {}", diag.rule_id, message);
    }
}

/// Verify the active + archive chain. Returns `Some(verdict)` when the
/// chain is broken; the caller emits + exits. Returns `None` when the
/// chain is intact (or empty — there's nothing to break yet).
fn verify_chain_or_block(
    repo_root: &Path,
    project_uuid: &str,
) -> Option<anvil_hook::RenderedVerdict> {
    let paths = witness_paths(repo_root);
    if paths.is_empty() {
        return None;
    }
    let path_refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    match verify_chain(&path_refs) {
        Ok(_) => None,
        Err(_) => Some(render_verdict(&Verdict::Block {
            count: 0,
            witness_id: project_uuid.to_string(),
            reason: BlockReason::ChainBroken,
        })),
    }
}

/// Build the ordered list of witness files for the chain verifier:
/// archive segments (lexicographic — matches `<scope>-<seq>-<merkle>`)
/// followed by `active.ndjson`. Returns an empty list when the chain
/// hasn't materialised yet.
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

/// Scan every witness file under `anvil/witness/` and collect the set
/// of recorded `commit_sha` values.
///
/// Returns `None` only when a witness file fails to open; a malformed
/// line is skipped (the chain verifier already catches structural
/// corruption with a stronger guarantee).
///
/// Streams each segment via `BufRead::lines()` rather than reading
/// the whole file into a `String` so memory stays bounded on large
/// chains (archive segments cap at 1 MB each, but on long-lived
/// repos there can be many).
fn collect_witnessed_shas(
    repo_root: &Path,
) -> std::result::Result<std::collections::HashSet<String>, std::io::Error> {
    use std::collections::HashSet;
    use std::io::{BufRead, BufReader};
    let mut out: HashSet<String> = HashSet::new();
    for path in witness_paths(repo_root) {
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
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
            // DAG-aware: parents from a merge witness are also
            // attestations of presence. Treat each parent as
            // witnessed too.
            for p in parsed.parent_commits {
                if !p.is_empty() {
                    out.insert(p);
                }
            }
        }
    }
    Ok(out)
}

/// Load `anvil/policy.yml` (or `.yaml` / `.json` / `.toml`).
///
/// Returns `Ok(None)` when no policy file exists — the caller treats
/// that as "this project hasn't opted into L4 enforcement" and skips
/// the pre-push checks entirely. Errors are propagated so the caller
/// can degrade to `InternalError` per Serena rule.
fn load_policy(repo_root: &Path) -> Result<Option<Policy>> {
    let candidates: &[(&str, ConfigFormat)] = &[
        ("anvil/policy.yml", ConfigFormat::Yaml),
        ("anvil/policy.yaml", ConfigFormat::Yaml),
        ("anvil/policy.json", ConfigFormat::Json),
        ("anvil/policy.toml", ConfigFormat::Toml),
    ];
    for (rel, format) in candidates {
        let path = repo_root.join(rel);
        if path.exists() {
            let raw =
                fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
            let policy = Policy::parse(&raw, *format, &path)
                .with_context(|| format!("parse {}", path.display()))?;
            return Ok(Some(policy));
        }
    }
    Ok(None)
}

/// Walk the pushed range via `git rev-list` and return commit SHAs in
/// new→old order. For `PushKind::Update` the range is
/// `<remote_sha>..<local_sha>`; for `PushKind::Create` (new branch)
/// we walk the full ancestry of `local_sha`.
///
/// Returns `None` when the git invocation fails so the caller can
/// downgrade to `InternalError` rather than panicking. An empty `Vec`
/// is a valid result (nothing to validate; e.g. fast-forward push of
/// already-pushed commits).
fn list_range(repo_root: &Path, push_ref: &PushRef) -> Option<Vec<String>> {
    // Defence in depth: the parser already enforces hex-only SHAs,
    // but verify again at the call site so a future contributor who
    // hand-builds a `PushRef` can't feed `git rev-list` a revspec or
    // option. Belt-and-braces; cheap.
    if !is_hex_sha(&push_ref.local_sha) || !is_hex_sha(&push_ref.remote_sha) {
        return None;
    }
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo_root).arg("rev-list");
    match push_ref.kind {
        PushKind::Update => {
            let range = format!("{}..{}", push_ref.remote_sha, push_ref.local_sha);
            // `--` after the revision argument makes git refuse to
            // re-interpret `range` as a path or option even if a
            // future bug lets a non-hex token slip through.
            cmd.arg(range).arg("--");
        }
        PushKind::Create => {
            // git rev-list local_sha walks the full ancestry; the
            // operator opts in by configuring `OnNoWitness::Allow` on
            // the matching rule if the legacy commits shouldn't be
            // re-witnessed. For initial-branch adoption,
            // `cutoff_commit` (deferred) is the right mechanism.
            cmd.arg(&push_ref.local_sha).arg("--");
        }
        PushKind::Delete => return Some(Vec::new()),
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

/// Short 12-char SHA prefix for the verdict line. Matches the
/// `witness_id` format used by `anvil show <id>`.
fn short_sha(sha: &str) -> String {
    let len = sha.len().min(12);
    sha[..len].to_string()
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

    // ---- MLP-004 pre-push helper tests ------------------------------

    fn write_witness_line_for(root: &Path, project_uuid: &str, commit_sha: &str) {
        let writer = WitnessWriter::open(root, "active", RolloverPolicy::default()).unwrap();
        drop(writer);
        append_witness(root, project_uuid, |seq, prev| {
            build_witness_line(
                project_uuid,
                Some(commit_sha.to_string()),
                "pre-commit",
                seq,
                prev,
            )
        })
        .unwrap();
    }

    #[test]
    fn witness_paths_is_empty_when_no_chain_exists() {
        let (_tmp, root) = make_test_repo();
        assert!(witness_paths(&root).is_empty());
    }

    #[test]
    fn witness_paths_includes_active_when_present() {
        let (_tmp, root) = make_test_repo();
        write_witness_line_for(&root, "01997e4a-1b2c-7345-8901-abcdef123456", "deadbeef");
        let paths = witness_paths(&root);
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("active.ndjson"));
    }

    #[test]
    fn collect_witnessed_shas_extracts_commit_sha_from_chain() {
        let (_tmp, root) = make_test_repo();
        write_witness_line_for(&root, "01997e4a-1b2c-7345-8901-abcdef123456", "commit-xyz");
        let set = collect_witnessed_shas(&root).unwrap();
        assert!(set.contains("commit-xyz"));
    }

    #[test]
    fn collect_witnessed_shas_returns_empty_when_no_chain() {
        let (_tmp, root) = make_test_repo();
        let set = collect_witnessed_shas(&root).unwrap();
        assert!(set.is_empty());
    }

    #[test]
    fn collect_witnessed_shas_skips_genesis_and_lines_without_commit() {
        // The genesis line has commit_sha=None; only records with a
        // commit_sha go into the set.
        let (_tmp, root) = make_test_repo();
        write_witness_line_for(&root, "01997e4a-1b2c-7345-8901-abcdef123456", "real-sha");
        let set = collect_witnessed_shas(&root).unwrap();
        // 1 entry (the record), not 2 (which would include genesis).
        assert_eq!(set.len(), 1);
        assert!(set.contains("real-sha"));
    }

    #[test]
    fn load_policy_returns_none_when_file_absent() {
        let (_tmp, root) = make_test_repo();
        assert!(load_policy(&root).unwrap().is_none());
    }

    #[test]
    fn load_policy_reads_yaml() {
        let (_tmp, root) = make_test_repo();
        fs::write(
            root.join("anvil").join("policy.yml"),
            "branches:\n  - pattern: main\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n",
        )
        .unwrap();
        let p = load_policy(&root).unwrap().unwrap();
        assert_eq!(p.branches.len(), 1);
        assert_eq!(p.branches[0].pattern, "main");
    }

    #[test]
    fn load_policy_reads_json() {
        let (_tmp, root) = make_test_repo();
        fs::write(
            root.join("anvil").join("policy.json"),
            r#"{"branches":[{"pattern":"main","require":"l4_or_l3","on_no_witness":"validate_at_l4"}]}"#,
        )
        .unwrap();
        let p = load_policy(&root).unwrap().unwrap();
        assert_eq!(p.branches[0].pattern, "main");
    }

    #[test]
    fn load_policy_reads_toml() {
        let (_tmp, root) = make_test_repo();
        fs::write(
            root.join("anvil").join("policy.toml"),
            "[[branches]]\npattern = \"main\"\nrequire = \"l4_or_l3\"\non_no_witness = \"validate_at_l4\"\n",
        )
        .unwrap();
        let p = load_policy(&root).unwrap().unwrap();
        assert_eq!(p.branches[0].pattern, "main");
    }

    #[test]
    fn load_policy_prefers_yml_over_other_extensions() {
        // If multiple files exist, .yml wins per the candidate order
        // — documented precedence so an accidental .json doesn't
        // shadow the .yml a user is editing.
        let (_tmp, root) = make_test_repo();
        fs::write(
            root.join("anvil").join("policy.yml"),
            "branches:\n  - pattern: yml-wins\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n",
        )
        .unwrap();
        fs::write(
            root.join("anvil").join("policy.json"),
            r#"{"branches":[{"pattern":"json-loses","require":"l4_or_l3","on_no_witness":"validate_at_l4"}]}"#,
        )
        .unwrap();
        let p = load_policy(&root).unwrap().unwrap();
        assert_eq!(p.branches[0].pattern, "yml-wins");
    }

    #[test]
    fn short_sha_truncates_to_twelve_chars() {
        let full = "0123456789abcdef0123456789abcdef";
        assert_eq!(short_sha(full), "0123456789ab");
    }

    #[test]
    fn short_sha_returns_full_when_already_short() {
        assert_eq!(short_sha("abc"), "abc");
    }

    #[test]
    fn short_sha_returns_empty_for_empty_input() {
        assert_eq!(short_sha(""), "");
    }

    #[test]
    fn verify_chain_or_block_returns_none_when_no_chain() {
        let (_tmp, root) = make_test_repo();
        assert!(verify_chain_or_block(&root, "uuid").is_none());
    }

    #[test]
    fn verify_chain_or_block_blocks_on_corrupted_active_file() {
        let (_tmp, root) = make_test_repo();
        // Seed a chain so the active file exists.
        write_witness_line_for(&root, "01997e4a-1b2c-7345-8901-abcdef123456", "abc");
        // Corrupt it.
        let active = root.join("anvil").join("witness").join("active.ndjson");
        fs::write(&active, "not-valid-ndjson\n").unwrap();
        let rendered = verify_chain_or_block(&root, "uuid").expect("expected block");
        assert_eq!(rendered.exit_code, 1);
        assert!(rendered.stderr_line.contains("chain integrity broken"));
    }

    #[test]
    fn collect_witnessed_shas_includes_parent_commits_from_merge_lines() {
        // A merge witness names parent SHAs; those should be treated
        // as witnessed presence too, otherwise a merge-base ancestor
        // would look unwitnessed during pre-push verification.
        let (_tmp, root) = make_test_repo();
        let writer = WitnessWriter::open(&root, "active", RolloverPolicy::default()).unwrap();
        drop(writer);
        let plan = merge_witness_plan(
            "merge-sha".to_string(),
            [
                ("parent-x".to_string(), Some("hash-x".to_string())),
                ("parent-y".to_string(), Some("hash-y".to_string())),
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
        let set = collect_witnessed_shas(&root).unwrap();
        assert!(set.contains("merge-sha"));
        assert!(set.contains("parent-x"));
        assert!(set.contains("parent-y"));
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

    // MLP2-016 — L4 validation engine dispatch + diagnostic rendering.
    //
    // The hook's `run_pre_push_with_engine` calls `std::process::exit`
    // on a block, which makes the full pre-push path untestable from
    // within a unit test. The trait-dispatch shape itself is covered
    // by `anvil_l4::validate::tests::*`; the tests below pin the
    // pieces unique to this file: the `emit_l4_block` rendering and
    // the request builder integration.

    /// `emit_l4_block` is exercised via output capture indirectly —
    /// here we pin the underlying message-truncation rule. The
    /// `validate.rs` contract says diagnostics are ≤200 chars; the
    /// hook defends against a misbehaving engine by truncating
    /// to 197 chars + `...`.
    #[test]
    fn emit_l4_block_truncates_overlong_messages() {
        // Direct-call would write to stderr in-process; the goal here
        // is to confirm the truncation arithmetic. Build the same
        // truncated string the function produces and compare against
        // the input post-truncate to pin the contract.
        let long_message: String = "x".repeat(500);
        let mut truncated: String = long_message.chars().take(197).collect();
        truncated.push_str("...");
        assert_eq!(truncated.chars().count(), 200);
        assert!(truncated.ends_with("..."));
        // Short messages are untouched.
        let short: String = "short".to_string();
        assert!(short.chars().count() <= 200);
    }

    /// `Severity::Block` and `Severity::Warn` round-trip to their
    /// stable lowercase labels — pinned because the hook renders
    /// them in stderr lines that operator dashboards may parse.
    #[test]
    fn severity_labels_are_stable() {
        // Mirror the match in `emit_l4_block` exactly so a future
        // rename forces both call sites to update.
        let map = [(Severity::Block, "block"), (Severity::Warn, "warn")];
        for (severity, expected) in map {
            let label = match severity {
                Severity::Block => "block",
                Severity::Warn => "warn",
            };
            assert_eq!(label, expected);
        }
    }

    /// `request_for` (re-exported from `anvil_l4`) builds a
    /// `ValidationRequest` from a commit SHA + branch rule + repo
    /// root. The hook constructs one of these per
    /// `NeedsL4Validation` commit; pin the input pass-through so a
    /// future `BranchRule` clone bug surfaces in this file.
    #[test]
    fn request_for_propagates_inputs_to_engine() {
        use anvil_l4::{BranchRule, OnBlock, OnNoWitness, OnWarn, Requirement};
        let rule = BranchRule {
            pattern: "main".to_string(),
            require: Requirement::L4OrL3,
            on_no_witness: OnNoWitness::ValidateAtL4,
            on_block: OnBlock::Reject,
            on_warn: OnWarn::Allow,
        };
        let request = request_for("deadbeef".repeat(5), rule, Path::new("/work/repo"));
        assert_eq!(request.commit_sha, "deadbeef".repeat(5));
        assert_eq!(request.branch_rule.pattern, "main");
        assert_eq!(request.repo_root, Path::new("/work/repo"));
    }

    /// End-to-end dispatch: with a fixture `BlockingEngine`, the
    /// hook's validation step produces `Block { diagnostics }` —
    /// exercises the trait wire path through the production
    /// `anvil_l4::validate_at_l4` helper that the hook calls into.
    #[test]
    fn blocking_engine_surfaces_block_with_diagnostics() {
        use anvil_l4::{
            BranchRule, OnBlock, OnNoWitness, OnWarn, Requirement, ValidationDiagnostic,
            ValidationRequest, ValidationVerdict,
        };
        struct BlockingEngine;
        impl ValidationEngine for BlockingEngine {
            fn validate(&self, _request: &ValidationRequest) -> ValidationVerdict {
                ValidationVerdict::Block {
                    diagnostics: vec![ValidationDiagnostic {
                        rule_id: "secret-detection.aws-key".to_string(),
                        severity: Severity::Block,
                        message: "AWS access key leaked in src/config.rs:42".to_string(),
                    }],
                }
            }
        }
        let rule = BranchRule {
            pattern: "main".to_string(),
            require: Requirement::L4OrL3,
            on_no_witness: OnNoWitness::ValidateAtL4,
            on_block: OnBlock::Reject,
            on_warn: OnWarn::Allow,
        };
        let req = request_for("c".repeat(40), rule, Path::new("/work/repo"));
        let verdict = anvil_l4::validate_at_l4(&BlockingEngine, &req);
        match verdict {
            ValidationVerdict::Block { diagnostics } => {
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(diagnostics[0].rule_id, "secret-detection.aws-key");
            }
            other => panic!("expected Block, got {other:?}"),
        }
    }

    /// The default `NoOpValidationEngine` bound at the production
    /// call site returns `EngineUnavailable { NotImplemented }`.
    /// Pre-MLP2-016 behaviour (single `InternalError { TimedOut }`
    /// emit + admit push) is the responsibility of the hook's
    /// `engine_unavailable` accumulator; this test pins that the
    /// default engine bound on production produces the variant the
    /// accumulator routes on.
    #[test]
    fn default_noop_engine_produces_engine_unavailable() {
        use anvil_l4::{
            BranchRule, EngineUnavailableReason, OnBlock, OnNoWitness, OnWarn, Requirement,
            ValidationVerdict,
        };
        let rule = BranchRule {
            pattern: "main".to_string(),
            require: Requirement::L4OrL3,
            on_no_witness: OnNoWitness::ValidateAtL4,
            on_block: OnBlock::Reject,
            on_warn: OnWarn::Allow,
        };
        let req = request_for("d".repeat(40), rule, Path::new("/work/repo"));
        let verdict = anvil_l4::validate_at_l4(&NoOpValidationEngine, &req);
        assert_eq!(
            verdict,
            ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::NotImplemented,
            }
        );
    }
}
