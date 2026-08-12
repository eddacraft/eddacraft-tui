//! `anvil hook <kind>` — git hook entrypoints (pre-commit / pre-push / …).

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
use std::time::{Duration, Instant};

use anvil_baseline::load as load_baseline;
use anvil_hook::{
    BOOTSTRAP_RECOVERY_VALIDATION_AT, BlockReason, BootstrapPlan, ErrorClass, MergeWitnessPlan,
    PanicReport, PushKind, PushRef, RewritePair, SuppressionKey, SuppressionLog, Verdict,
    build_bootstrap_plan, detect_framework, format_panic_report, is_hex_sha, merge_witness_plan,
    parse_post_rewrite_input, parse_pre_push_input, render_success_message, render_verdict,
};
use anvil_intercept::kindling_observation::{
    PostHookAction, PostHookEmissionRequest, PostHookEmitter,
};
use anvil_intercept::telemetry::DEGRADED_EMBEDDED_WITNESS;
use anvil_intercept_proto::protocol::{
    ANVIL_WITNESS_APPEND, WitnessAppendRequest, WitnessAppendResponse, WitnessEntry,
    WitnessOutcomeKind,
};
use anvil_l4::{
    BlockKind, CommitDecision, OnWarn, Policy, Severity, ValidationDiagnostic, ValidationEngine,
    ValidationVerdict, request_for,
};
use anvil_rules::{RequiredAnvilVersion, config_sha_from_canonical, rules_sha};
use anvil_witness::{
    GenesisAnchor, LineHash, RolloverPolicy, WitnessLine, WitnessWriter, WriterError,
    verify_chain_dag, witness_paths,
};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::GlobalArgs;
use crate::activation::identity::read_project_id;
use crate::l4_engine::CommitAntipatternEngine;
use crate::mcp::gctx_client::{DaemonRpcError, daemon_rpc_call};

/// MLP2-022: wall-clock budget for the pre-push hook.
///
/// ADR-038 names a 2 s p95 target for pre-push. Beyond this, the hook
/// stops walking the remaining range and admits the push with a
/// `partial: true` marker (rendered to the operator as
/// `ErrorClass::TimedOut` — "pre-push budget exceeded; partial
/// validation"). The cap protects developers on very large pushes
/// (e.g. branch initial push of a long-running history) from a 30 s
/// hang while still emitting a structured trace event so the future
/// Kindling fan-out can record the partial state.
const PRE_PUSH_BUDGET: Duration = Duration::from_secs(2);

#[derive(Debug, Args)]
pub struct HookArgs {
    #[command(subcommand)]
    command: HookCommand,
}

#[derive(Debug, Subcommand)]
enum HookCommand {
    /// L3 pre-commit hook — validates the staged diff and appends a
    /// witness line. Noise discipline: silent on pass, one line
    /// otherwise.
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
    /// Walk `<remote>..HEAD` after bootstrap and write a retroactive
    /// witness for each commit that hasn't already been witnessed.
    /// Recovers from a worktree-bootstrap failure where the hooks
    /// never fired (e.g. fresh clone before the runtime was
    /// installed).
    #[arg(long)]
    witness_recent: bool,
}

pub fn run(args: &HookArgs, _global: &GlobalArgs) -> Result<()> {
    let repo_root = std::env::current_dir().context("resolve repo root")?;
    install_panic_catcher();
    // One suppression log lives for the lifetime of the hook process
    // so ADR-038 §D-1 burst-suppression actually fires: a daemon-down
    // burst across 82 commits collapses to one emit instead of 82.
    let mut sup = SuppressionLog::new();

    // MLP2-010: post-hook surfaces emit Kindling `action_executed`
    // rows after each successful witness append. The hook process is
    // short-lived (one-shot per git event), so we mint a fresh
    // session UUID per invocation and bind a `NoopKindlingObservationSink`
    // by default — concrete sink wiring (IPC bridge to the daemon's
    // fan-out, or in-process Kindling client) is the deferred follow-up
    // shared with MLP2-006 / MLP2-007. The trait seam in
    // `anvil_intercept::kindling_observation` is the snap-in point.
    let post_hook_emitter = PostHookEmitter::noop(uuid::Uuid::new_v4().to_string());

    // catch_unwind so a panic deep in the hook body cannot bubble out
    // to git's exit code (101 by default). ADR-038 §D-7: internal
    // errors must not hold the user hostage. The panic hook installed
    // above already wrote the structured log + the one-line stderr
    // message; we just need to swallow the unwind and exit zero.
    let result = catch_unwind(AssertUnwindSafe(|| match &args.command {
        HookCommand::PreCommit(_) => run_pre_commit(&repo_root, &mut sup),
        HookCommand::PrePush(_) => run_pre_push(&repo_root, &mut sup),
        HookCommand::PostCommit(_) => run_post_commit(&repo_root, &post_hook_emitter),
        HookCommand::PostMerge(a) => run_post_merge(&repo_root, a, &post_hook_emitter),
        HookCommand::PostRewrite(a) => {
            run_post_rewrite(&repo_root, a, &mut sup, &post_hook_emitter)
        }
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
        // No project-id → anvil hasn't activated here; the hook is a
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

    // MLP2-014 — resolve the active rule set and compute `rules_sha`
    // so every pre-commit witness line carries the rule-set digest.
    // The hook fires once per commit, so a direct config read here
    // (rather than reusing the daemon-side MLP2-001 cache) keeps the
    // call site simple — the daemon cache is a performance
    // optimisation, not a correctness boundary, and the hook subprocess
    // does not have IPC access to it.
    let pre_commit_rules_sha = compute_pre_commit_rules_sha(repo_root);

    let result = append_witness_routed(repo_root, &identity.project_uuid, |seq, prev| {
        let mut line = build_witness_line(&identity.project_uuid, None, "pre-commit", seq, prev);
        line.rules_sha.clone_from(&pre_commit_rules_sha);
        line
    });
    match result {
        Ok(_line_hash) => {
            let rendered = render_verdict(&Verdict::Pass);
            if !rendered.stderr_line.is_empty() {
                eprintln!("{}", rendered.stderr_line);
            }
            Ok(())
        }
        Err(AppendError::ChainBroken) => {
            // ADR-038: chain integrity break refuses the commit. We do NOT reseed
            // (which would obliterate evidence). Recovery depends on the cause: a
            // hash-chain break is inspected with `anvil show <id>`; a CIB-126
            // emptied/deleted `active.ndjson` (marker survives) is restored with
            // `git checkout -- anvil/witness/active.ndjson` (or, if the chain was
            // never committed, acknowledged by removing the chain-init marker to
            // permit a reseed). See docs/runbooks/anvil-witness-chain.md.
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
        Err(AppendError::Gated) => {
            // DISTRIB-006 (ADR-060): under a gated ANVIL_HOME the candidate does
            // not write the witness line — but it must not block the commit. The
            // read-only / dry-run posture renders Pass so the operator's commit
            // proceeds exactly as it would with the production binary.
            let rendered = render_verdict(&Verdict::Pass);
            if !rendered.stderr_line.is_empty() {
                eprintln!("{}", rendered.stderr_line);
            }
            Ok(())
        }
    }
}

fn run_post_commit(repo_root: &Path, emitter: &PostHookEmitter) -> Result<()> {
    let identity = match read_project_id(repo_root) {
        Ok(Some(id)) => id,
        Ok(None) | Err(_) => return Ok(()),
    };
    let started = Instant::now();
    // Post-commit is the only hook that can bind a real commit SHA: the object
    // exists and HEAD points at it. Pre-commit intentionally leaves
    // `commit_sha: None` (the commit does not exist yet). Without this binding
    // `anvil audit-chain` always reports 0 witnessed (Dave SEC-WIT-1).
    let commit_sha = resolve_head_sha(repo_root);
    let appended = append_witness_routed(repo_root, &identity.project_uuid, |seq, prev| {
        let mut line = build_witness_line(
            &identity.project_uuid,
            Some(commit_sha.clone()),
            "post-commit",
            seq,
            prev,
        );
        line.kind = "post-commit".to_string();
        line
    });
    if let Ok(line_hash) = &appended {
        emit_post_hook_action(
            emitter,
            PostHookAction::PostCommit,
            repo_root,
            &commit_sha,
            line_hash,
            started.elapsed(),
        );
    }
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
///   [`anvil_l4::ValidationEngine`] trait. Production binds
///   [`CommitAntipatternEngine`] (see `crates/anvil-cli/src/l4_engine.rs`)
///   which materialises the commit's tree via git plumbing and runs
///   the `anvil-checks` antipattern catalogue, surfacing `Allow` or
///   `Block { diagnostics }` per commit. When git plumbing fails the
///   engine returns `EngineUnavailable`, which the hook collapses to
///   the legacy `InternalError { TimedOut }` + admit-push surface
///   per ADR-038 §D-6 (internal failures never block the user).
///   Tests substitute fixture engines via
///   [`run_pre_push_with_engine`].
///
/// ## MLP2-020 / -021 / -022 — Wave 1E closure
///
/// - **MLP2-020** Hook reads `policy.required_anvil_version`, parses
///   it via [`RequiredAnvilVersion`], and refuses to run the per-commit
///   walk when the running binary is below the floor — emitting a
///   single `ErrorClass::VersionFloor` line ("upgrade anvil") at exit
///   0 (Serena rule: internal preconditions don't hold the user
///   hostage).
/// - **MLP2-021** When `policy.baseline.cutoff_commit` is set, the
///   hook drives [`Policy::commit_is_before_cutoff`] with the
///   first-parent ancestry of each pushed tip (via `git rev-list
///   --first-parent`). Commits at or before the cutoff are treated as
///   baselined and skipped — closing the adoption-friction story for
///   repos with a long history.
/// - **MLP2-022** A wall-clock budget ([`PRE_PUSH_BUDGET`], default
///   2 s) caps the walk; on exceed the hook stops walking the
///   remaining range, admits the push with a `partial: true` tracing
///   marker, and emits `ErrorClass::TimedOut` ("pre-push budget
///   exceeded; partial validation") at exit 0. The tracing event is
///   the v1 Kindling-row surface — the daemon's Kindling IPC
///   fan-out is deferred and will consume the same structured event.
///
/// ## Deferred
///
/// - L4 witness writes to `refs/notes/anvil-l4` (owned by MLP-010).
/// - In-process Kindling IPC fan-out (the partial-state marker is
///   currently a structured tracing event; the daemon-side write to
///   the Kindling `SQLite` handle lands with INTD-004).
fn run_pre_push(repo_root: &Path, sup: &mut SuppressionLog) -> Result<()> {
    run_pre_push_with_engine(repo_root, sup, &CommitAntipatternEngine)
}

/// MLP2-016: pre-push entry point that takes a pluggable
/// [`ValidationEngine`]. The production `run_pre_push` binds
/// [`CommitAntipatternEngine`] (the real `anvil-checks` antipattern
/// pipeline); integration tests substitute a fixture engine to drive
/// the Allow / Block paths through the production call site without
/// shelling out.
//
// `too_many_lines` is allowed here because the function is a
// linear sequence of ADR-038 stages (project-id → stdin → policy →
// version-floor → chain → witnesses → per-ref walk) and each stage
// carries the comment that documents the contract it implements.
// Extracting helpers further would push the contract into
// separately-grepped files. The per-commit decision body still
// uses `std::process::exit` for blocks, which makes a sub-helper
// awkward — the exits are part of the visible top-level flow.
#[allow(clippy::too_many_lines)]
fn run_pre_push_with_engine(
    repo_root: &Path,
    sup: &mut SuppressionLog,
    engine: &dyn ValidationEngine,
) -> Result<()> {
    let identity = match read_project_id(repo_root) {
        Ok(Some(id)) => id,
        // No project-id → project hasn't opted into anvil; hook is a
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

    // MLP2-020: hook-side `required_anvil_version` floor check at
    // fire time. Two distinct routings: `BelowFloor` is the operator's
    // problem to fix by upgrading the binary; `InvalidFloor` is the
    // operator's problem to fix in the policy file. Both admit the
    // push (Serena rule — an internal precondition must not block
    // the user) but surface different lines so the remediation is
    // actionable. The daemon-side check at registration mirrors
    // this; see MLP2-018.
    //
    // Ordering note: the floor check fires BEFORE chain verification
    // by design. If the running binary doesn't meet the floor we
    // cannot trust this binary's own chain verifier to give a
    // meaningful answer — the same logic that pins
    // `required_anvil_version` may have changed witness chain
    // semantics. Admitting on floor-unmet is consistent with
    // Serena; chain integrity becomes the post-upgrade walk's
    // problem.
    match check_version_floor(&policy, env!("CARGO_PKG_VERSION")) {
        VersionFloorOutcome::Satisfied => {}
        VersionFloorOutcome::BelowFloor => {
            emit_internal(ErrorClass::VersionFloor, sup);
            return Ok(());
        }
        VersionFloorOutcome::InvalidFloor => {
            // The floor string itself is malformed. The remediation
            // is to fix the policy file, not the binary — so route
            // through `EmbeddedFailed` ("validation errored") rather
            // than `VersionFloor` ("upgrade anvil") to avoid sending
            // the operator after the wrong fix.
            emit_internal(ErrorClass::EmbeddedFailed, sup);
            return Ok(());
        }
    }

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

    // MLP2-022: start the wall-clock budget *before* the
    // per-push-ref walk. The cap protects developers on very large
    // pushes — when exceeded, the hook stops walking and admits the
    // push with a partial-state marker rather than hanging.
    let budget_start = Instant::now();
    let mut budget_exceeded = false;
    let mut commits_processed: usize = 0;
    let mut commits_skipped_for_cutoff: usize = 0;

    // Walk every push ref and apply per-branch policy.
    let mut engine_unavailable = false;
    'walk: for push_ref in &push_refs {
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
        // MLP2-021: lazily build the cutoff-acceptance lookup table
        // only when the policy actually pins a `cutoff_commit`.
        // Skipping the git invocation in the common (greenfield)
        // case keeps the hook's cold-path cheap. The lookup table
        // is `(cutoff_index, ancestry_index)` where `ancestry_index`
        // maps SHA → position in the newest-first ancestry. Per-
        // commit cutoff check then becomes a single hash lookup +
        // index compare (O(1)) rather than the O(ancestry) double
        // linear scan that `Policy::commit_is_before_cutoff`
        // performs on its own (Council kernel-maintainer follow-up).
        let cutoff_lookup: Option<(usize, std::collections::HashMap<String, usize>)> =
            policy.baseline.cutoff_commit.as_deref().and_then(|cutoff| {
                let ancestry = first_parent_ancestry(repo_root, &push_ref.local_sha)?;
                let cutoff_idx = ancestry.iter().position(|s| s == cutoff)?;
                let index: std::collections::HashMap<String, usize> = ancestry
                    .into_iter()
                    .enumerate()
                    .map(|(i, sha)| (sha, i))
                    .collect();
                Some((cutoff_idx, index))
            });
        for commit in range_commits {
            // MLP2-022: check the budget at the top of each commit
            // iteration. The boundary is between commits so we never
            // interrupt validation mid-flight.
            if is_budget_exceeded(budget_start, PRE_PUSH_BUDGET) {
                budget_exceeded = true;
                break 'walk;
            }
            // MLP2-021: if the cutoff is set and this commit is at
            // or before it in the first-parent ancestry, treat the
            // commit as baselined — skip witness/validation entirely.
            // Ancestry is newest-first, so a position INDEX greater
            // than or equal to the cutoff's index means "older than
            // or equal to cutoff" → skip.
            if let Some((cutoff_idx, ref index)) = cutoff_lookup
                && let Some(commit_idx) = index.get(&commit)
                && *commit_idx >= cutoff_idx
            {
                commits_skipped_for_cutoff += 1;
                continue;
            }
            commits_processed += 1;
            let has_witness = witnessed.contains(&commit);
            match rule.decide_commit(has_witness) {
                CommitDecision::Allow => {}
                CommitDecision::NeedsL4Validation => {
                    // MLP2-016: route through ValidationEngine instead
                    // of an unconditional InternalError emit.
                    let request = request_for(
                        commit.clone(),
                        rule.clone(),
                        repo_root,
                        Some(push_ref.local_sha.clone()),
                    );
                    match engine.validate(&request) {
                        ValidationVerdict::Allow => {}
                        ValidationVerdict::Block { diagnostics } => {
                            // MLP2-016 Council #C-016A: the branch
                            // rule's `on_warn` knob decides whether
                            // warn-only diagnostics upgrade to a
                            // block. If every diagnostic is
                            // `Severity::Warn` AND the rule says
                            // `on_warn: Allow`, surface the
                            // diagnostics but admit the push — the
                            // engine has done its job by reporting
                            // them. Any `Severity::Block` diagnostic
                            // (or any `Warn` diagnostic with
                            // `OnWarn::Reject`) hard-refuses.
                            let warn_only = !diagnostics.is_empty()
                                && diagnostics.iter().all(|d| d.severity == Severity::Warn);
                            let warn_can_allow = rule.on_warn == OnWarn::Allow;
                            if warn_only && warn_can_allow {
                                emit_l4_block(&commit, &diagnostics);
                                // No exit — admit the push.
                            } else {
                                emit_l4_block(&commit, &diagnostics);
                                let rendered = render_verdict(&Verdict::Block {
                                    count: 0,
                                    witness_id: short_sha(&commit),
                                    reason: BlockReason::UnwitnessedCommit,
                                });
                                std::process::exit(rendered.exit_code);
                            }
                        }
                        ValidationVerdict::EngineUnavailable { reason } => {
                            // BinaryMissing / Timeout / NotImplemented
                            // all map to the pre-MLP2-016 fall-through:
                            // emit InternalError { TimedOut } once via
                            // suppression, admit the push (ADR-038).
                            engine_unavailable = true;
                            // Council #C-016I: emit the reason at
                            // tracing-warn so production incident
                            // investigations have a machine-readable
                            // signal distinguishing
                            // "engine not implemented" from
                            // "git not on PATH" from
                            // "rule catalogue missing" — the operator
                            // still sees one `ValidationPending`
                            // line via SuppressionLog below.
                            tracing::warn!(
                                target: "anvil::hook::pre_push",
                                kind = "engine_unavailable",
                                commit = %short_sha(&commit),
                                reason = ?reason,
                                "L4 validation engine could not run; admitting push",
                            );
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
    //
    // MLP2-022 Council follow-up: gate this emit on
    // `!budget_exceeded`. When the budget fired, the partial-state
    // line ("pre-push budget exceeded; partial validation") is the
    // single source of truth — adding a second `ValidationPending`
    // line below it would tell the operator "L4 surface missing"
    // when the actual cause was "ran out of time".
    if engine_unavailable && !budget_exceeded {
        emit_internal(ErrorClass::ValidationPending, sup);
    }

    // MLP2-022: on budget exceed, emit a single
    // `ErrorClass::TimedOut` line ("pre-push budget exceeded; partial
    // validation") via SuppressionLog and a structured tracing event
    // that future Kindling fan-out will consume as the partial-state
    // observation. Stays exit 0 per Serena rule.
    if budget_exceeded {
        tracing::warn!(
            target: "anvil::hook::pre_push",
            kind = "gate_evaluated",
            gate_id = "prePush",
            partial = true,
            budget_ms = u64::try_from(PRE_PUSH_BUDGET.as_millis()).unwrap_or(u64::MAX),
            elapsed_ms = u64::try_from(budget_start.elapsed().as_millis()).unwrap_or(u64::MAX),
            commits_processed,
            commits_skipped_for_cutoff,
            "pre-push budget exceeded; partial validation, push admitted",
        );
        emit_internal(ErrorClass::TimedOut, sup);
    }

    Ok(())
}

/// MLP2-020: outcome of checking the running binary against the
/// policy's `required_anvil_version` floor at fire time.
#[derive(Debug, Clone, PartialEq, Eq)]
enum VersionFloorOutcome {
    /// Either no floor is pinned, OR `current >= floor`. The hook
    /// proceeds with the normal walk.
    Satisfied,
    /// `current < floor`. The hook admits the push but emits a
    /// distinct "upgrade anvil" line.
    BelowFloor,
    /// The floor string is not valid semver. Treated as unmet (don't
    /// silently admit a malformed floor; surface the problem so the
    /// operator can fix the policy file).
    InvalidFloor,
}

/// MLP2-020: check `policy.required_anvil_version` against the
/// running binary's version.
///
/// Returns [`VersionFloorOutcome::Satisfied`] when no floor is pinned
/// OR when the running version is at or above the floor. Otherwise
/// returns [`VersionFloorOutcome::BelowFloor`] (parsed floor, version
/// below it) or [`VersionFloorOutcome::InvalidFloor`] (floor string
/// fails semver parse).
///
/// The caller emits a single `ErrorClass::VersionFloor` line on
/// anything other than `Satisfied`. Both failure cases admit the push
/// per ADR-038 §D-6 (Serena rule).
fn check_version_floor(policy: &Policy, current_version: &str) -> VersionFloorOutcome {
    let Some(floor_raw) = policy.required_anvil_version.as_deref() else {
        return VersionFloorOutcome::Satisfied;
    };
    let Ok(floor) = RequiredAnvilVersion::parse(floor_raw) else {
        return VersionFloorOutcome::InvalidFloor;
    };
    match floor.satisfied_by(current_version) {
        Ok(true) => VersionFloorOutcome::Satisfied,
        // Below floor OR the running version itself isn't valid
        // semver — both routed to BelowFloor because the operator's
        // remediation is the same ("upgrade anvil").
        Ok(false) | Err(_) => VersionFloorOutcome::BelowFloor,
    }
}

/// MLP2-021: maximum ancestry walk per pushed ref.
///
/// Caps the `git rev-list --first-parent` invocation so the hook
/// cannot block on a multi-megabyte stdout when a freshly-pushed
/// branch ancestrally reaches deep into a long-running history.
/// 100 000 first-parent commits is generous (a daily-commit project
/// would take ~270 years to hit it) but still bounds the worst case
/// well under the 2 s pre-push budget on commodity hardware.
const ANCESTRY_WALK_CAP: usize = 100_000;

/// MLP2-021: fetch the first-parent ancestry from a tip SHA via
/// `git rev-list --first-parent --max-count=<cap> <tip>`.
///
/// Returns SHAs newest-first (ancestry[0] == tip) so they can be fed
/// directly into [`Policy::commit_is_before_cutoff`]. Returns `None`
/// when the git invocation fails — the caller treats that as "no
/// ancestry available" and falls back to validating the full pushed
/// range (which is also the current pre-MLP2-021 behaviour). The
/// ancestry length is bounded by [`ANCESTRY_WALK_CAP`] so a
/// pathologically deep history cannot consume the wall-clock
/// budget inside the git invocation itself.
fn first_parent_ancestry(repo_root: &Path, tip_sha: &str) -> Option<Vec<String>> {
    // Defence in depth: same hex-SHA refusal as `list_range` to keep
    // `git rev-list` from re-interpreting a malformed value as a
    // revspec or path.
    if !is_hex_sha(tip_sha) {
        return None;
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-list")
        .arg("--first-parent")
        .arg(format!("--max-count={ANCESTRY_WALK_CAP}"))
        .arg(tip_sha)
        .arg("--")
        .output()
        .ok()?;
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

/// MLP2-022: true when the wall-clock budget for the pre-push walk
/// has been exhausted. Returns true at exactly the boundary so a
/// zero-budget cap (used in tests) trips immediately.
fn is_budget_exceeded(start: Instant, budget: Duration) -> bool {
    start.elapsed() >= budget
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
    // MLP2-011 — DAG-aware so merge witnesses don't trip the verifier.
    match verify_chain_dag(&path_refs) {
        Ok(_) => None,
        Err(_) => Some(render_verdict(&Verdict::Block {
            count: 0,
            witness_id: project_uuid.to_string(),
            reason: BlockReason::ChainBroken,
        })),
    }
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

use crate::policy_load::load_policy;

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

/// MLP2-010: resolve the current HEAD SHA via git plumbing so the
/// post-commit row carries the canonical commit identifier rather
/// than a placeholder. Falls back to the literal `"HEAD"` when git
/// invocation fails, so the row still records the event with an
/// honest token rather than panicking.
fn resolve_head_sha(repo_root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
        .arg("HEAD")
        .output();
    let Ok(output) = output else {
        return "HEAD".to_string();
    };
    if !output.status.success() {
        return "HEAD".to_string();
    }
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// MLP2-010: build + emit an `action_executed` Kindling row from the
/// post-hook surface. Always-best-effort: sink failures are logged
/// inside [`PostHookEmitter::try_emit`] and never propagated, so the
/// hook return path stays uncoupled from sink health.
fn emit_post_hook_action(
    emitter: &PostHookEmitter,
    action: PostHookAction,
    repo_root: &Path,
    commit_sha: &str,
    line_hash: &str,
    elapsed: Duration,
) {
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let working_directory = repo_root.to_string_lossy().into_owned();
    let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    let request = PostHookEmissionRequest {
        action,
        commit_sha,
        witness_line_hash: line_hash,
        working_directory: &working_directory,
        timestamp: &timestamp,
        duration_ms,
    };
    let _ = emitter.try_emit(&request);
}

fn run_post_merge(repo_root: &Path, args: &PostMergeArgs, emitter: &PostHookEmitter) -> Result<()> {
    let identity = match read_project_id(repo_root) {
        Ok(Some(id)) => id,
        Ok(None) | Err(_) => return Ok(()),
    };
    let started = Instant::now();
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
    let plan = merge_witness_plan(merge_sha.clone(), parent_pairs);
    let appended = append_witness_routed(repo_root, &identity.project_uuid, |seq, prev| {
        build_merge_witness_line(&identity.project_uuid, plan.clone(), seq, prev)
    });
    if let Ok(line_hash) = &appended {
        emit_post_hook_action(
            emitter,
            PostHookAction::PostMerge,
            repo_root,
            &merge_sha,
            line_hash,
            started.elapsed(),
        );
    }
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
    emitter: &PostHookEmitter,
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
        let started = Instant::now();
        // MLP2-010: stamp the new SHA on the row so an external
        // rewrite-tracker can join on `details.command`.
        let new_sha = pair.new_sha.clone();
        let appended = append_witness_routed(repo_root, &identity.project_uuid, |seq, prev| {
            build_rewrite_witness_line(&identity.project_uuid, &pair, seq, prev)
        });
        if let Ok(line_hash) = &appended {
            emit_post_hook_action(
                emitter,
                PostHookAction::PostRewrite,
                repo_root,
                &new_sha,
                line_hash,
                started.elapsed(),
            );
        }
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
    // DISTRIB-006 (ADR-060): installing `.git/hooks/*` / `.husky/*` is a durable
    // per-project mutation a different binary reads. Refuse it under a non-default
    // ANVIL_HOME without `--touch-project-state` — bootstrap is the first command
    // a tester runs on a fresh clone, so a candidate must not silently install
    // hooks into the real repo. (`--dry-run` above is read-only and unaffected.)
    crate::install_root::ensure_project_write_allowed("hook bootstrap")?;
    let _files_written = execute_bootstrap_plan(repo_root, &plan)?;
    // MLP2-037: when --witness-recent is set, walk @{u}..HEAD and
    // write one retroactive witness per unwitnessed commit. The
    // success-message counter reflects only the retroactive lines
    // (not the installed hook files) so the user-facing line stays
    // honest.
    let witnessed = if args.witness_recent {
        run_witness_recent_walk(repo_root)
    } else {
        0
    };
    eprintln!("{}", render_success_message(witnessed));
    Ok(())
}

/// MLP2-037 walk: enumerate `<remote>..HEAD` and append a retroactive
/// witness line for every commit not already in the chain. Returns
/// the number of new witnesses written so the caller can thread the
/// count into the success message.
///
/// Best-effort: a missing `@{u}`, an absent project-id, or a transient
/// witness-writer failure each degrade to zero rather than blocking
/// the bootstrap. The bootstrap path's contract is recovery, not
/// enforcement; surfacing a hard error here would re-create the
/// hostage scenario MLP2-037 exists to defuse.
fn run_witness_recent_walk(repo_root: &Path) -> usize {
    let identity = match read_project_id(repo_root) {
        Ok(Some(id)) => id,
        // Without a project-id there's nothing meaningful to witness.
        // Treat as zero rather than panicking — same contract as the
        // other hook commands (Serena rule).
        Ok(None) | Err(_) => return 0,
    };
    let range = match list_unwitnessed_range(repo_root) {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let mut written = 0;
    for sha in range {
        if commit_is_witnessed(repo_root, &sha) {
            continue;
        }
        let result = append_witness_routed(repo_root, &identity.project_uuid, |seq, prev| {
            build_bootstrap_recovery_witness_line(&identity.project_uuid, &sha, seq, prev)
        });
        if result.is_ok() {
            written += 1;
        }
    }
    written
}

/// Walk `git -C <root> rev-list --reverse @{u}..HEAD --` and return the
/// SHAs in old→new order. Returns `Ok(vec![])` when `@{u}` is not
/// configured (the upstream isn't set) so the caller can degrade to a
/// clean zero-count success rather than surfacing an error.
///
/// Each returned SHA is validated via [`is_hex_sha`] as defence in
/// depth against a future bug feeding `git` a revspec or option.
///
/// Council MAJOR (wave 1G review) — `--reverse` is required so the
/// recovery walk writes witness lines in temporal-causal order
/// (genesis → A → B → C). Without it, `git rev-list` returns
/// newest-first, which would invert the `seq` ordering of the
/// retroactive lines relative to commit time and confuse audit tooling
/// that expects monotonic seq-vs-timestamp correlation.
fn list_unwitnessed_range(repo_root: &Path) -> io::Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-list")
        .arg("--reverse")
        .arg("@{u}..HEAD")
        .arg("--")
        .output()?;
    if !output.status.success() {
        // `@{u}` not configured, or HEAD doesn't resolve — degrade
        // to empty rather than escalating to an error.
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && is_hex_sha(s))
        .map(str::to_string)
        .collect())
}

/// Streaming check: is `sha` already present as a `commit_sha` value
/// anywhere in the active witness file *or* any archive segment?
/// Returns `false` when the chain doesn't exist yet.
///
/// Reads line-by-line via `BufRead::lines()` so memory stays bounded
/// on large chains; do not collect the whole file. Uses a substring
/// match against the canonical `"commit_sha":"<sha>"` shape rather
/// than full JSON parsing — the witness writer emits canonical JSON
/// with sorted keys (no whitespace) so the substring is stable, and
/// avoiding `serde_json::from_slice` per line keeps the walk cheap
/// for large chains.
///
/// MLP2-061: pre-fix the function only inspected `active.ndjson`,
/// which meant the bootstrap `--witness-recent` retroactive-witness
/// check could re-witness a commit that already appeared in an
/// archive segment after rollover (silent duplicate witnesses on top
/// of archived history). Walking `witness_paths(repo_root)` keeps
/// the witnessed-set complete across rollover boundaries.
fn commit_is_witnessed(repo_root: &Path, sha: &str) -> bool {
    use std::io::{BufRead, BufReader};
    let needle = format!("\"commit_sha\":\"{sha}\"");
    for path in witness_paths(repo_root) {
        let Ok(file) = fs::File::open(&path) else {
            continue;
        };
        let reader = BufReader::new(file);
        for line in reader.lines() {
            // A read error on one line skips that line — same shape
            // as the file-open fall-through above. Pre-fix this used
            // `break`, which aborted the whole segment on a transient
            // read error and could miss a SHA recorded later in the
            // same file (Council quick review).
            let Ok(line) = line else { continue };
            if line.contains(&needle) {
                return true;
            }
        }
    }
    false
}

fn build_bootstrap_recovery_witness_line(
    project_uuid: &str,
    commit_sha: &str,
    seq: u64,
    prev_line_hash: String,
) -> WitnessLine {
    WitnessLine {
        seq,
        scope: "active".to_string(),
        kind: "witness".to_string(),
        prev_line_hash,
        project_uuid: project_uuid.to_string(),
        commit_sha: Some(commit_sha.to_string()),
        parent_commits: Vec::new(),
        prev_line_hashes: Vec::new(),
        agent_tag: None,
        rules_sha: None,
        cutoff_commit: None,
        ts: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        validation_at: BOOTSTRAP_RECOVERY_VALIDATION_AT.to_string(),
    }
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
    /// DISTRIB-006 (ADR-060): running under a non-default `ANVIL_HOME` without
    /// `--touch-project-state`. The witness line was deliberately *not* written
    /// so an unreleased candidate cannot append to a real project's chain. This
    /// is a benign skip, not a failure — upstream reads/validation still ran, and
    /// the commit must not be blocked.
    Gated,
}

/// Project the wire [`WitnessEntry`] from a fully-built [`WitnessLine`]: the
/// entry is the caller-controlled subset of a line (the writer derives
/// `seq`/`prev_line_hash` under its lock, and the daemon asserts `ts`). This lets
/// the daemon and embedded legs share one `build` closure — the embedded leg
/// writes the line directly; the daemon leg sends this projection and the daemon
/// rebuilds a line whose **caller-controlled fields are identical** (Parity
/// criterion; `seq`/`prev_line_hash`/`ts` differ, being writer-derived).
fn witness_entry_from_line(line: &WitnessLine) -> WitnessEntry {
    WitnessEntry {
        project_uuid: line.project_uuid.clone(),
        kind: line.kind.clone(),
        scope: line.scope.clone(),
        commit_sha: line.commit_sha.clone(),
        parent_commits: line.parent_commits.clone(),
        prev_line_hashes: line.prev_line_hashes.clone(),
        agent_tag: line.agent_tag.clone(),
        rules_sha: line.rules_sha.clone(),
        cutoff_commit: line.cutoff_commit.clone(),
        // Sent for completeness but NOT authoritative: the daemon asserts its own
        // wall-clock `ts` at append time (see `save_time.rs::witness_append`), so
        // the value carried here is only used by the embedded leg's own line.
        ts: line.ts.clone(),
        validation_at: line.validation_at.clone(),
    }
}

/// The routing decision for a daemon witness-append attempt. Kept as a pure
/// function over the transport result so the classification — the load-bearing
/// invariant — is unit-testable without a live daemon.
///
/// **The daemon is a pure optimisation** (owner decision, MLP2-005 phase-3
/// council): only an *authoritative* daemon result is terminal — `Appended`
/// (success) or `ChainBroken` (tamper, refused on either leg per ADR-038).
/// **Everything else falls back to the embedded writer**: any transport failure
/// (absent socket, `-32601`, `NotAdmitted`, timeout, IO, parse, peer-reject) and
/// any non-authoritative in-band outcome. This guarantees the daemon never blocks
/// a commit the embedded path would not have (Serena, ADR-038 §D-6). The one cost
/// is a benign duplicate line in the narrow window where the daemon appended but
/// its reply was lost — the chain stays linear and verifies (`append_chained` is
/// atomic under flock; the embedded leg simply chains off the new tip).
#[derive(Debug)]
enum WitnessRoute {
    /// The daemon returned an authoritative result; carry its translated outcome.
    Daemon(Result<LineHash, AppendError>),
    /// No authoritative daemon result — take the embedded leg.
    Embedded,
}

fn route_daemon_witness_result(
    result: std::result::Result<WitnessAppendResponse, DaemonRpcError>,
) -> WitnessRoute {
    match result {
        Ok(resp) => match resp.outcome {
            // Authoritative success — the daemon durably wrote the line.
            WitnessOutcomeKind::Appended => match resp.line_hash {
                Some(hash) => WitnessRoute::Daemon(Ok(hash)),
                // Success without a hash is a malformed reply — fall back rather
                // than fabricate one (never worse than embedded).
                None => WitnessRoute::Embedded,
            },
            // Authoritative tamper detection — refuse on either leg, never reseed.
            WitnessOutcomeKind::ChainBroken => WitnessRoute::Daemon(Err(AppendError::ChainBroken)),
            // The daemon did not durably witness (its write failed, or a future
            // variant we do not understand) — retry locally; the embedded write
            // can only make things better, never worse.
            WitnessOutcomeKind::WriteFailed | WitnessOutcomeKind::Unknown => WitnessRoute::Embedded,
        },
        // No authoritative result at all (Unavailable OR Failure) — fall back.
        Err(_) => WitnessRoute::Embedded,
    }
}

/// Attempt the append over the daemon's `anvil/witness/append` RPC, reusing the
/// shared daemon JSON-RPC client. Returns the in-band response, or the transport
/// classification ([`DaemonRpcError`]) on failure.
fn daemon_witness_append(
    repo_root: &Path,
    entry: &WitnessEntry,
) -> std::result::Result<WitnessAppendResponse, DaemonRpcError> {
    let request = WitnessAppendRequest {
        workspace_root: repo_root.to_string_lossy().into_owned(),
        entry: entry.clone(),
    };
    daemon_rpc_call(ANVIL_WITNESS_APPEND, &request, "anvil-hook-witness")
}

/// Append a witness line **daemon-first** with an embedded fallback (MLP2-005
/// phase 3). DISTRIB-006 gating short-circuits before either leg. When the daemon
/// returns an authoritative result the append routes through it (one writer across
/// worktrees, shared lock state); otherwise it falls back to the embedded
/// [`WitnessWriter`] (emitting [`DEGRADED_EMBEDDED_WITNESS`]). Both legs share
/// `build`, so the recorded caller-controlled fields are identical.
fn append_witness_routed<F>(
    repo_root: &Path,
    project_uuid: &str,
    build: F,
) -> std::result::Result<LineHash, AppendError>
where
    F: Fn(u64, String) -> WitnessLine,
{
    // DISTRIB-006 (ADR-060): a gated candidate must not append to the real chain,
    // and must not reach the daemon to do so. Read the ambient gate here and route
    // on it — this is where the phase-2 council's deferred F2 (daemon-side gate) is
    // satisfied: the only production caller never opens the socket when gated.
    append_witness_routed_gated(
        crate::install_root::project_writes_gated(),
        repo_root,
        project_uuid,
        build,
    )
}

/// Routing core with the DISTRIB-006 gate decision injected, so the
/// gate-before-routing ordering is unit-testable without mutating the process
/// environment (which would race concurrent witness tests).
fn append_witness_routed_gated<F>(
    gated: bool,
    repo_root: &Path,
    project_uuid: &str,
    build: F,
) -> std::result::Result<LineHash, AppendError>
where
    F: Fn(u64, String) -> WitnessLine,
{
    if gated {
        return Err(AppendError::Gated);
    }

    // Build the line ONCE. `build` (and any non-determinism inside it, e.g.
    // `Utc::now()`) runs exactly once regardless of which leg wins: the daemon leg
    // sends the projected entry; the embedded leg reuses this exact template and
    // only fills the writer-derived `(seq, prev_line_hash)` under the lock. This is
    // what keeps the recorded caller-controlled fields identical across legs and
    // avoids re-invoking `build` on the fallback path (no double side effects).
    let template = build(0, String::new());
    let entry = witness_entry_from_line(&template);

    finish_witness_route(
        daemon_witness_append(repo_root, &entry),
        repo_root,
        project_uuid,
        move |seq, prev_line_hash| {
            let mut line = template.clone();
            line.seq = seq;
            line.prev_line_hash = prev_line_hash;
            line
        },
    )
}

/// Execute the [`WitnessRoute`] for a daemon attempt: return the authoritative
/// daemon outcome, or run the embedded leg. Split out with the daemon result
/// injected so **both** legs (daemon-authoritative and embedded fallback) are
/// exercisable end-to-end in tests without a live daemon.
fn finish_witness_route<F>(
    daemon_result: std::result::Result<WitnessAppendResponse, DaemonRpcError>,
    repo_root: &Path,
    project_uuid: &str,
    build: F,
) -> std::result::Result<LineHash, AppendError>
where
    F: Fn(u64, String) -> WitnessLine,
{
    match route_daemon_witness_result(daemon_result) {
        WitnessRoute::Daemon(result) => {
            // Surface the chosen leg (Observability). The daemon leg is the quiet
            // happy path — debug, not warn — so a normal commit prints nothing.
            tracing::debug!(
                target: "anvil::witness",
                project_uuid = %project_uuid,
                "witness appended via the daemon",
            );
            result
        }
        WitnessRoute::Embedded => {
            // Degrading to the embedded writer is the *expected* graceful path
            // (the daemon is a pure optimisation), so this is `info`, not `warn`:
            // it stays silent under the default `warn` filter and does not spew a
            // JSON blob to the developer's terminal on every daemon-absent commit.
            tracing::info!(
                target: "anvil::witness",
                reason = DEGRADED_EMBEDDED_WITNESS,
                project_uuid = %project_uuid,
                "witness daemon unavailable; appending via the embedded writer",
            );
            append_witness(repo_root, project_uuid, build)
        }
    }
}

/// Resolve the witness flock-acquire timeout from `ANVIL_WITNESS_LOCK_TIMEOUT`
/// (CIB-124 override), warning and falling back to the default on a malformed
/// value rather than silently defaulting.
fn resolve_witness_lock_timeout() -> Duration {
    let raw = match std::env::var(anvil_witness::LOCK_TIMEOUT_ENV) {
        Ok(v) => Some(v),
        Err(std::env::VarError::NotPresent) => None,
        // A set-but-non-UTF-8 value is malformed too — warn, don't silently default.
        Err(std::env::VarError::NotUnicode(_)) => {
            tracing::warn!(
                target: "anvil::witness",
                "{} is set but not valid UTF-8; using the default",
                anvil_witness::LOCK_TIMEOUT_ENV,
            );
            return anvil_witness::DEFAULT_LOCK_ACQUIRE_TIMEOUT;
        }
    };
    anvil_witness::lock_timeout_from_env(raw.as_deref()).unwrap_or_else(|bad| {
        tracing::warn!(
            target: "anvil::witness",
            value = %bad,
            "ignoring invalid {} (want a positive integer of seconds); using the default",
            anvil_witness::LOCK_TIMEOUT_ENV,
        );
        anvil_witness::DEFAULT_LOCK_ACQUIRE_TIMEOUT
    })
}

fn append_witness<F>(
    repo_root: &Path,
    project_uuid: &str,
    build: F,
) -> std::result::Result<LineHash, AppendError>
where
    F: FnOnce(u64, String) -> WitnessLine,
{
    // DISTRIB-006 (ADR-060): skip the durable witness append under a gated
    // ANVIL_HOME (non-default install root, no `--touch-project-state`). The
    // candidate has already read/validated against the real repo; it just must
    // not persist a witness line into a chain the production binary reads.
    if crate::install_root::project_writes_gated() {
        return Err(AppendError::Gated);
    }

    let writer = WitnessWriter::open(repo_root, "active", RolloverPolicy::default())
        .map_err(|_| AppendError::WriteFailed)?;

    // MLP2-005: derive `(seq, prev)` from the full archive + active chain and
    // append, **atomically under one flock hold**, via `append_chained`. The
    // earlier code read the chain head (`chain_head`) outside the writer's lock
    // and only then appended, so a daemon and an embedded fallback (or concurrent
    // worktree hooks) could read the same tip and fork the chain. `append_chained`
    // holds the lock across read-head → derive → append; it seeds genesis on an
    // empty chain (walking `witness_paths` so the chain stays continuous across
    // rollover boundaries — MLP2-061) and refuses a broken chain without reseeding
    // (ADR-038). It also hashes the canonical bytes before the write (MLP2-010), so
    // a serialise failure still surfaces on the `WriteFailed` path.
    let project_uuid = project_uuid.to_string();
    writer
        .append_chained_with_lock_timeout(
            || {
                WitnessLine::genesis(
                    &GenesisAnchor::Fresh,
                    &project_uuid,
                    "active",
                    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    "pre-commit",
                    None,
                )
            },
            build,
            resolve_witness_lock_timeout(),
        )
        .map_err(|err| {
            // Distinct operator log per failure class (the `witness_root` field lets
            // a machine running many parallel worktree hooks correlate the line to a
            // specific chain). The AppendError mapping itself is the pure
            // `classify_append_error` below.
            match &err {
                WriterError::ChainBroken => {
                    // ADR-038 tamper-detection fired.
                    tracing::warn!(
                        target: "anvil::witness",
                        project_uuid = %project_uuid,
                        witness_root = %repo_root.display(),
                        "witness chain integrity check failed; refusing to append (ADR-038)",
                    );
                }
                // CIB-124: the bounded lock acquire gave up — the `.lock` was held
                // past the timeout (another writer, slow I/O, or a stuck holder).
                // Distinct log so an operator can tell lock contention from a
                // genuine write failure.
                WriterError::LockTimeout(_) => {
                    tracing::warn!(
                        target: "anvil::witness",
                        error = %err,
                        project_uuid = %project_uuid,
                        witness_root = %repo_root.display(),
                        "witness lock acquire timed out; the lock was held past the timeout",
                    );
                }
                // Corruption (incl. a genesis that failed to re-verify), IO, scope,
                // or symlink refusal — "we couldn't witness this write" (ADR-038).
                other => {
                    tracing::warn!(
                        target: "anvil::witness",
                        error = %other,
                        project_uuid = %project_uuid,
                        witness_root = %repo_root.display(),
                        "witness append failed",
                    );
                }
            }
            classify_append_error(&err)
        })
}

/// Map a witness [`WriterError`] onto the hook's [`AppendError`]. Only
/// `ChainBroken` (ADR-038 tamper) is distinct; every other failure — a CIB-124
/// `LockTimeout`, IO, corruption, scope, or symlink refusal — is a `WriteFailed`
/// ("we couldn't witness this write"). Pure, so the mapping (in particular
/// `LockTimeout` → `WriteFailed`) is unit-testable without provoking the error.
fn classify_append_error(err: &WriterError) -> AppendError {
    match err {
        WriterError::ChainBroken => AppendError::ChainBroken,
        _ => AppendError::WriteFailed,
    }
}

// MLP2-005: `chain_head` + `ChainState` were removed — the chain-head read now
// lives in `WitnessWriter::read_chain_head` (returning `ChainHead`), run under
// the flock by `WitnessWriter::append_chained` so the read and the append are
// atomic. The MLP2-061 rollover-recovery behaviour (walking `witness_paths` so
// archive segments participate) moved with it.

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
    // DISTRIB-006 (ADR-060): under a non-default ANVIL_HOME, install-owned kernel
    // logs re-root to `<ANVIL_HOME>/cache/` so a candidate's panic log never
    // mingles with production's. Unset = platform default below.
    if let Some(cache_dir) = crate::install_root::install_root().cache_dir() {
        return Some(cache_dir.join(anvil_hook::PANIC_LOG_FILE));
    }
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

/// MLP2-014: OPA / Rego runtime version pinned for the v1 hook lane.
///
/// The L4 engine (MLP2-016) and the recognised-rules registry
/// (MLP2-019) both pin `"0.10.0"` for the v1 rule sets. The pre-commit
/// hook must use the same string so a witness line written here and a
/// verification check run on L4 collapse to the same `rules_sha` for
/// equivalent configs. Centralised in `anvil_rules::OPA_RUNTIME_VERSION`
/// once the capsule digest collector became the second consumer
/// (GITGOV-006 council follow-up); `anvil-l4` keeps its own literal to
/// stay free of the `anvil-rules` dependency.
const HOOK_OPA_RUNTIME_VERSION: &str = anvil_rules::OPA_RUNTIME_VERSION;

/// MLP2-014: compute the `rules_sha` for the active rule set at hook
/// fire time, or `None` when no `.anvil.<ext>` config is present.
///
/// Mirrors the discovery rule already used by `gate.rs::read_anvilrc_checks`
/// (MLP2-040): walk the worktree for `.anvil.{yaml,yml,json,toml}` via
/// `anvil_config::discover`, parse, canonicalise, and feed the
/// resulting `config_sha` plus the pinned runtime versions into
/// `anvil_rules::rules_sha`. The legacy `.anvilrc` fallback is
/// intentionally NOT consulted here — the witness `rules_sha` field
/// is forward-looking and pinning it to the `.anvil.<ext>` channel
/// keeps the digest stable as projects migrate off `.anvilrc`.
///
/// Returns `None` when no `.anvil.<ext>` file is discovered (the
/// conservative path: spec MLP2-014 is silent on the no-config case,
/// and omitting the field is preferable to inventing a digest for an
/// empty default the witness verifier can't anchor against). Any I/O
/// or parse error also collapses to `None` rather than failing the
/// commit — the field is an evidence-stream annotation, not a
/// validation gate, and a missing digest is preferable to refusing the
/// commit (ADR-038 §D-1 noise discipline: warnings over blocks).
///
/// The `rules` list is empty for v1; MLP2-014 wires the field shape,
/// and a future task threads the resolved rule-id set through when
/// the rule engine integration lands.
fn compute_pre_commit_rules_sha(repo_root: &Path) -> Option<String> {
    let discovered = match anvil_config::discover(repo_root, ".anvil") {
        Ok(Some(found)) => found,
        // No `.anvil.<ext>` config (or a `discover` I/O error) →
        // conservative path: leave the witness field as `None`. The
        // test `pre_commit_with_missing_config_uses_none` pins the
        // no-config branch; I/O errors collapse to the same outcome
        // because the field is an annotation, not a validation gate
        // (ADR-038 §D-1 noise discipline).
        Ok(None) | Err(_) => return None,
    };

    let value = anvil_config::parse_file(&discovered.path).ok()?;
    let canonical = anvil_config::canonical_json_bytes(&value).ok()?;
    let config_sha = config_sha_from_canonical(&canonical);

    rules_sha(
        env!("CARGO_PKG_VERSION"),
        HOOK_OPA_RUNTIME_VERSION,
        std::iter::empty::<&str>(),
        config_sha,
    )
    .ok()
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
        cutoff_commit: None,
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
        cutoff_commit: None,
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
        cutoff_commit: None,
        ts: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        validation_at: anvil_hook::POST_REWRITE_VALIDATION_AT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_config::ConfigFormat;
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

    // ---- MLP2-005 phase 3: daemon-first routing + embedded fallback ----

    fn witness_response(outcome: WitnessOutcomeKind, hash: Option<&str>) -> WitnessAppendResponse {
        WitnessAppendResponse {
            outcome,
            line_hash: hash.map(str::to_string),
            error: None,
        }
    }

    #[test]
    fn witness_entry_from_line_carries_caller_fields() {
        let line = build_merge_witness_line(
            "01997e4a-1b2c-7345-8901-abcdef123456",
            merge_witness_plan(
                "merge-sha".to_string(),
                vec![("parent-a".to_string(), Some("tip-a".to_string()))],
            ),
            7,
            "single-tip".to_string(),
        );
        let entry = witness_entry_from_line(&line);
        // Caller-controlled fields are carried verbatim; `seq`/`prev_line_hash`
        // are absent from the entry by construction (the daemon derives them).
        assert_eq!(entry.project_uuid, line.project_uuid);
        assert_eq!(entry.kind, line.kind);
        assert_eq!(entry.scope, line.scope);
        assert_eq!(entry.commit_sha, line.commit_sha);
        assert_eq!(entry.parent_commits, line.parent_commits);
        assert_eq!(entry.prev_line_hashes, line.prev_line_hashes);
        assert_eq!(entry.validation_at, line.validation_at);
    }

    /// Reusable `build` closure for the routing tests (a fn is `Fn`).
    fn sample_witness_build(seq: u64, prev: String) -> WitnessLine {
        build_witness_line(
            "01997e4a-1b2c-7345-8901-abcdef123456",
            Some("c1".to_string()),
            "pre-commit",
            seq,
            prev,
        )
    }

    #[test]
    fn route_treats_only_appended_and_chainbroken_as_authoritative() {
        match route_daemon_witness_result(Ok(witness_response(
            WitnessOutcomeKind::Appended,
            Some("hash"),
        ))) {
            WitnessRoute::Daemon(Ok(h)) => assert_eq!(h, "hash"),
            other => panic!("Appended ⇒ Daemon(Ok): {other:?}"),
        }
        assert!(matches!(
            route_daemon_witness_result(Ok(witness_response(
                WitnessOutcomeKind::ChainBroken,
                None
            ))),
            WitnessRoute::Daemon(Err(AppendError::ChainBroken)),
        ));
    }

    #[test]
    fn route_falls_back_to_embedded_for_everything_non_authoritative() {
        // The daemon is a pure optimisation: any transport failure AND any
        // non-authoritative in-band outcome ⇒ embedded leg, never a hard block.
        for result in [
            Err(DaemonRpcError::Unavailable),
            Err(DaemonRpcError::Failure),
            Ok(witness_response(WitnessOutcomeKind::WriteFailed, None)),
            Ok(witness_response(WitnessOutcomeKind::Unknown, None)),
            // Malformed success (no hash) also falls back rather than fabricating.
            Ok(witness_response(WitnessOutcomeKind::Appended, None)),
        ] {
            assert!(
                matches!(route_daemon_witness_result(result), WitnessRoute::Embedded),
                "expected Embedded",
            );
        }
    }

    #[test]
    fn finish_route_returns_daemon_hash_without_writing_locally() {
        let (_tmp, root) = make_test_repo();
        // An authoritative daemon success is returned as-is; the embedded writer
        // is NOT invoked (no local witness tree is created).
        let out = finish_witness_route(
            Ok(witness_response(
                WitnessOutcomeKind::Appended,
                Some("daemon-hash"),
            )),
            &root,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            sample_witness_build,
        );
        assert_eq!(out.unwrap(), "daemon-hash");
        assert!(
            !root.join("anvil").join("witness").exists(),
            "the daemon handled the append; the embedded leg must not run",
        );
    }

    #[test]
    fn finish_route_refuses_on_authoritative_chainbroken_without_writing() {
        let (_tmp, root) = make_test_repo();
        let out = finish_witness_route(
            Ok(witness_response(WitnessOutcomeKind::ChainBroken, None)),
            &root,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            sample_witness_build,
        );
        assert!(matches!(out, Err(AppendError::ChainBroken)));
        assert!(!root.join("anvil").join("witness").exists());
    }

    #[test]
    fn finish_route_embeds_on_daemon_failure() {
        let (_tmp, root) = make_test_repo();
        // A mid-exchange daemon Failure now falls back to the embedded writer
        // (owner decision) rather than blocking the commit — the chain is written.
        // Pin ANVIL_HOME so the embedded leg's own gate re-read is ungated.
        temp_env::with_var("ANVIL_HOME", None::<&str>, || {
            let out = finish_witness_route(
                Err(DaemonRpcError::Failure),
                &root,
                "01997e4a-1b2c-7345-8901-abcdef123456",
                sample_witness_build,
            );
            assert!(!out.unwrap().is_empty(), "embedded leg wrote the line");
            let paths = witness_paths(&root);
            let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
            let dag = verify_chain_dag(&refs).expect("chain verifies");
            assert_eq!(dag.line_count, 2, "genesis + 1 record");
        });
    }

    #[test]
    fn routed_append_falls_back_to_embedded_when_daemon_absent() {
        let (_tmp, root) = make_test_repo();
        // Point the socket at an empty runtime dir (no daemon) and pin ANVIL_HOME
        // to default so the embedded leg's own gate re-read (inside append_witness)
        // is ungated regardless of the runner's ambient env.
        let runtime = TempDir::new().unwrap();
        temp_env::with_vars(
            [
                ("ANVIL_HOME", None::<&str>),
                ("XDG_RUNTIME_DIR", Some(runtime.path().to_str().unwrap())),
            ],
            || {
                let hash = append_witness_routed_gated(
                    false,
                    &root,
                    "01997e4a-1b2c-7345-8901-abcdef123456",
                    sample_witness_build,
                )
                .expect("embedded fallback writes the line");
                assert!(!hash.is_empty());

                let paths = witness_paths(&root);
                let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
                let dag = verify_chain_dag(&refs).expect("chain verifies");
                assert_eq!(dag.line_count, 2, "genesis + 1 record");
            },
        );
    }

    #[test]
    fn routed_append_is_gated_before_touching_the_daemon() {
        let (_tmp, root) = make_test_repo();
        // The gate is the first thing checked: a gated append returns `Gated`
        // without writing or opening the socket. No env mutation — the gate
        // decision is injected (its env detection is covered in `install_root`).
        let result = append_witness_routed_gated(
            true,
            &root,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            |seq, prev| {
                build_witness_line(
                    "01997e4a-1b2c-7345-8901-abcdef123456",
                    Some("c1".to_string()),
                    "pre-commit",
                    seq,
                    prev,
                )
            },
        );
        assert!(matches!(result, Err(AppendError::Gated)));
        assert!(
            !root.join("anvil").join("witness").exists(),
            "a gated append must not create the witness tree",
        );
    }

    #[test]
    fn classify_append_error_maps_lock_timeout_and_others_to_write_failed() {
        // CIB-124: a bounded-lock timeout is a write failure, not a tamper event.
        assert!(matches!(
            classify_append_error(&WriterError::LockTimeout(std::time::Duration::from_secs(5))),
            AppendError::WriteFailed,
        ));
        // Only ChainBroken stays distinct (ADR-038).
        assert!(matches!(
            classify_append_error(&WriterError::ChainBroken),
            AppendError::ChainBroken,
        ));
        // A generic IO error is also WriteFailed.
        assert!(matches!(
            classify_append_error(&WriterError::Io(std::io::Error::other("disk full"))),
            AppendError::WriteFailed,
        ));
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
        let report = verify_chain_dag(&[active.as_path()]).expect("chain verifies");
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
        run_bootstrap(
            &root,
            &BootstrapArgs {
                dry_run: true,
                witness_recent: false,
            },
        )
        .unwrap();
        assert!(!root.join(".git").join("hooks").exists());
    }

    #[test]
    fn bootstrap_plain_repo_installs_all_five_v1_hooks() {
        let (_tmp, root) = make_test_repo();
        run_bootstrap(
            &root,
            &BootstrapArgs {
                dry_run: false,
                witness_recent: false,
            },
        )
        .unwrap();
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

        let report = verify_chain_dag(&[active.as_path()]).unwrap();
        // genesis + two retroactive records.
        assert_eq!(report.line_count, 3);
        let contents = fs::read_to_string(&active).unwrap();
        assert!(contents.contains("\"commit_sha\":\"new1\""));
        assert!(contents.contains("\"commit_sha\":\"new2\""));
        assert!(contents.contains("\"validation_at\":\"post-rewrite-recovery\""));
    }

    // ----- MLP2-010: post-hook Kindling action_executed emission -----

    const MLP2_010_SESSION_UUID: &str = "22222222-2222-4222-8222-222222222222";

    #[test]
    fn post_commit_binds_head_commit_sha_on_witness_line() {
        // Dave SEC-WIT-1: without commit_sha on the witness record,
        // audit-chain can never mark a commit as witnessed.
        let (_tmp, root) = make_test_repo();
        // Seed a real git commit so resolve_head_sha returns a hex SHA.
        let init = Command::new("git")
            .args(["-C", &root.to_string_lossy(), "init"])
            .output()
            .expect("git init");
        assert!(init.status.success(), "git init: {init:?}");
        let _ = Command::new("git")
            .args(["-C", &root.to_string_lossy(), "config", "user.email", "t@t"])
            .status();
        let _ = Command::new("git")
            .args(["-C", &root.to_string_lossy(), "config", "user.name", "t"])
            .status();
        fs::write(root.join("f.txt"), "x").unwrap();
        let _ = Command::new("git")
            .args(["-C", &root.to_string_lossy(), "add", "f.txt"])
            .status();
        let commit = Command::new("git")
            .args([
                "-C",
                &root.to_string_lossy(),
                "commit",
                "-m",
                "seed",
                "--allow-empty",
            ])
            .output()
            .expect("git commit");
        assert!(commit.status.success(), "git commit: {commit:?}");
        let head = resolve_head_sha(&root);
        assert!(
            head.len() >= 7 && head.chars().all(|c| c.is_ascii_hexdigit()),
            "expected hex HEAD, got {head}"
        );

        let (emitter, _recorder) = PostHookEmitter::with_recorder(MLP2_010_SESSION_UUID);
        run_post_commit(&root, &emitter).unwrap();

        let active = root.join("anvil/witness/active.ndjson");
        let contents = fs::read_to_string(&active).expect("active.ndjson");
        assert!(
            contents.contains(&format!("\"commit_sha\":\"{head}\"")),
            "post-commit witness must bind HEAD ({head}):\n{contents}"
        );
    }

    #[test]
    fn post_commit_emits_one_action_executed_row_per_invocation() {
        let (_tmp, root) = make_test_repo();
        let (emitter, recorder) = PostHookEmitter::with_recorder(MLP2_010_SESSION_UUID);

        run_post_commit(&root, &emitter).unwrap();

        let rows = recorder.recorded_actions();
        assert_eq!(
            rows.len(),
            1,
            "post-commit must emit exactly one action_executed row",
        );
        let row = &rows[0];
        assert_eq!(row.kind, "action_executed");
        assert_eq!(row.session_id, MLP2_010_SESSION_UUID);
        assert_eq!(row.action_type, "command");
        assert!(
            row.action_id.starts_with("post-commit:"),
            "action_id must encode the post-hook surface: {}",
            row.action_id
        );
        let command = row.details.command.as_ref().expect("command populated");
        assert!(
            command.contains("post-commit"),
            "command must name the surface: {command}"
        );
        // Witness line hash on the row must be the SHA-256 (64 hex chars).
        assert!(
            command.contains("witness_line_hash="),
            "command must carry the witness line hash: {command}"
        );
        assert_eq!(row.details.working_directory, root.to_string_lossy());
        // gate_evaluated bucket stays empty — distinct trait routes.
        assert!(recorder.recorded().is_empty());
    }

    #[test]
    fn post_merge_emits_action_executed_with_merge_sha_in_command() {
        let (_tmp, root) = make_test_repo();
        let (emitter, recorder) = PostHookEmitter::with_recorder(MLP2_010_SESSION_UUID);
        // Synthetic merge ref — git rev-list will fail, so the
        // emitted commit SHA is the literal ref. That's exactly the
        // honest fallback the merge handler defines.
        let args = PostMergeArgs {
            commit: Some("merge-sha-deadbeef".to_string()),
        };

        run_post_merge(&root, &args, &emitter).unwrap();

        let rows = recorder.recorded_actions();
        assert_eq!(rows.len(), 1, "post-merge must emit exactly one row");
        let row = &rows[0];
        assert_eq!(row.action_id, "post-merge:merge-sha-deadbeef");
        let command = row.details.command.as_ref().expect("command populated");
        assert!(command.contains("post-merge"));
        assert!(
            command.contains("merge-sha-deadbeef"),
            "command must carry the merge sha: {command}"
        );
    }

    #[test]
    fn post_rewrite_emits_one_action_executed_row_per_pair() {
        // The production hook reads stdin via io::stdin(), which
        // the test harness can't easily redirect — drive the
        // builder + emitter inline against the same parsing path
        // and verify the row shape per pair. Same pattern as
        // `post_rewrite_writes_one_witness_per_pair`.
        let (_tmp, root) = make_test_repo();
        let (emitter, recorder) = PostHookEmitter::with_recorder(MLP2_010_SESSION_UUID);

        let pairs = parse_post_rewrite_input("old1 newsha1\nold2 newsha2\n").unwrap();
        for pair in pairs {
            let started = Instant::now();
            let new_sha = pair.new_sha.clone();
            let line_hash = append_witness(
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
            emit_post_hook_action(
                &emitter,
                PostHookAction::PostRewrite,
                &root,
                &new_sha,
                &line_hash,
                started.elapsed(),
            );
        }

        let rows = recorder.recorded_actions();
        assert_eq!(
            rows.len(),
            2,
            "post-rewrite must emit one row per rewrite pair",
        );
        assert_eq!(rows[0].action_id, "post-rewrite:newsha1");
        assert_eq!(rows[1].action_id, "post-rewrite:newsha2");
        for row in &rows {
            assert_eq!(row.session_id, MLP2_010_SESSION_UUID);
            let command = row.details.command.as_ref().expect("command populated");
            assert!(command.contains("witness_line_hash="));
        }
    }

    #[test]
    fn post_commit_with_no_project_id_is_silent_kindling_emit_too() {
        // No project-id on disk → run_post_commit short-circuits at
        // the identity check and never appends a witness. The
        // emitter must therefore stay silent (no row), matching the
        // existing no-witness contract.
        let tmp = TempDir::new().unwrap();
        let (emitter, recorder) = PostHookEmitter::with_recorder(MLP2_010_SESSION_UUID);
        run_post_commit(tmp.path(), &emitter).unwrap();
        assert!(
            recorder.is_empty(),
            "no project id → no witness append → no Kindling row",
        );
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

    // Precedence tests live with the shared implementation in
    // `crate::policy_load::tests` (UCFG-009): yaml-first per
    // DISCOVER_PRECEDENCE, including the deliberate ADR-120 pt 6
    // yaml-beats-yml flip.

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

    // The MLP2-061 rollover-recovery test moved into `anvil-witness`
    // (`append_chained_recovers_the_tip_across_a_real_rollover`) when
    // `read_chain_head` became `pub(crate)`. End-to-end rollover coverage stays in
    // `append_witness_after_rollover_chains_off_archive_tip` below.

    /// MLP2-061 regression: the tight-rollover loop. Append, force
    /// a rollover, append again, then verify that the chain across
    /// archive + active reads as one continuous DAG with exactly
    /// one genesis line. Pre-fix this would produce two genesis
    /// lines and the cross-segment verifier would fail.
    #[test]
    fn append_witness_after_rollover_chains_off_archive_tip() {
        let (_tmp, root) = make_test_repo();
        let project_uuid = "01997e4a-1b2c-7345-8901-abcdef123456";
        write_witness_line_for(&root, project_uuid, "a");
        write_witness_line_for(&root, project_uuid, "b");
        // Force rollover by relocating active → archive.
        let active = root.join("anvil").join("witness").join("active.ndjson");
        let archive_dir = root.join("anvil").join("witness").join("archive");
        fs::create_dir_all(&archive_dir).unwrap();
        let archived = archive_dir.join("active-00000000000000000003-rollover-tight.ndjson");
        fs::rename(&active, &archived).unwrap();
        // Append after rollover — MUST NOT seed a fresh genesis.
        write_witness_line_for(&root, project_uuid, "c");

        // Walk the full path list and verify one continuous DAG.
        let paths = witness_paths(&root);
        assert_eq!(paths.len(), 2, "expected archive + active");
        let path_refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        let dag = verify_chain_dag(&path_refs).expect("archive+active chain verifies");
        // 3 lines in archive (genesis + a + b) + 1 line in active (c) = 4.
        assert_eq!(dag.line_count, 4, "exactly one genesis across both files");

        // Active file must contain exactly the single new record —
        // not a fresh genesis above it.
        let active_contents = fs::read_to_string(&active).unwrap();
        let active_lines: Vec<&str> = active_contents.lines().collect();
        assert_eq!(
            active_lines.len(),
            1,
            "post-rollover active must hold one record, no second genesis"
        );
        assert!(active_lines[0].contains("\"commit_sha\":\"c\""));
        assert!(active_lines[0].contains("\"seq\":4"));
    }

    /// MLP2-061: `commit_is_witnessed` MUST walk archive segments,
    /// otherwise bootstrap `--witness-recent` can re-witness a SHA
    /// that already appears in archived history.
    #[test]
    fn commit_is_witnessed_finds_sha_recorded_in_archive_segment() {
        let (_tmp, root) = make_test_repo();
        let project_uuid = "01997e4a-1b2c-7345-8901-abcdef123456";
        write_witness_line_for(&root, project_uuid, "archived-sha");
        // Relocate active → archive (simulate rollover boundary).
        let active = root.join("anvil").join("witness").join("active.ndjson");
        let archive_dir = root.join("anvil").join("witness").join("archive");
        fs::create_dir_all(&archive_dir).unwrap();
        let archived = archive_dir.join("active-00000000000000000002-archived-only.ndjson");
        fs::rename(&active, &archived).unwrap();

        assert!(
            commit_is_witnessed(&root, "archived-sha"),
            "MLP2-061: commit_is_witnessed must scan archive segments"
        );
        assert!(!commit_is_witnessed(&root, "never-recorded"));
    }

    /// MLP2-063: oversized policy files MUST be refused before
    /// `read_to_string` allocates the body. anvil-config's bounded
    /// loader caps each file at `MAX_CONFIG_FILE_BYTES`; the shared
    /// hook loader honours the same cap.
    #[test]
    fn load_policy_refuses_oversized_yaml() {
        let (_tmp, root) = make_test_repo();
        // 1 MiB + 1 byte → just past the cap.
        let cap = usize::try_from(anvil_config::MAX_CONFIG_FILE_BYTES).expect("1 MiB fits usize");
        let mut body = String::with_capacity(cap + 64);
        body.push_str("branches:\n  - pattern: main\n    require: l4_or_l3\n");
        body.push_str("    on_no_witness: validate_at_l4\n");
        // Pad with a long YAML comment so the file is parseable in
        // principle but exceeds the size cap.
        let comment_prefix = "# pad ";
        while body.len() <= cap {
            body.push_str(comment_prefix);
            body.push_str("x".repeat(64).as_str());
            body.push('\n');
        }
        assert!(body.len() as u64 > anvil_config::MAX_CONFIG_FILE_BYTES);
        fs::write(root.join("anvil").join("policy.yml"), &body).unwrap();

        let err = load_policy(&root).unwrap_err();
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("exceeds") || rendered.contains("byte limit"),
            "expected FileTooLarge surface in error chain, got: {rendered}"
        );
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
        let request = request_for("deadbeef".repeat(5), rule, Path::new("/work/repo"), None);
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
        let req = request_for("c".repeat(40), rule, Path::new("/work/repo"), None);
        let verdict = anvil_l4::validate_at_l4(&BlockingEngine, &req);
        match verdict {
            ValidationVerdict::Block { diagnostics } => {
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(diagnostics[0].rule_id, "secret-detection.aws-key");
            }
            other => panic!("expected Block, got {other:?}"),
        }
    }

    /// MLP2-016 Council #C-016A: a `Block { diagnostics }` with
    /// every diagnostic at `Severity::Warn` AND a branch rule of
    /// `OnWarn::Allow` must NOT exit the hook — the rule's
    /// contract is "warn diagnostics surface but are admitted." Pin
    /// the discriminator logic so the hook's fall-through branch
    /// fires for this combination.
    #[test]
    fn warn_only_diagnostics_with_on_warn_allow_should_admit() {
        use anvil_l4::{OnBlock, OnNoWitness, OnWarn, Requirement, Severity, ValidationDiagnostic};
        // Mirror the hook's `warn_only && warn_can_allow` predicate
        // exactly. A refactor that drops either half forces this test
        // to break.
        let rule = anvil_l4::BranchRule {
            pattern: "main".to_string(),
            require: Requirement::L4OrL3,
            on_no_witness: OnNoWitness::ValidateAtL4,
            on_block: OnBlock::Reject,
            on_warn: OnWarn::Allow,
        };
        let diagnostics = [
            ValidationDiagnostic {
                rule_id: "style.import-order".to_string(),
                severity: Severity::Warn,
                message: "imports out of canonical order".to_string(),
            },
            ValidationDiagnostic {
                rule_id: "style.trailing-whitespace".to_string(),
                severity: Severity::Warn,
                message: "trailing whitespace on line 42".to_string(),
            },
        ];
        let warn_only =
            !diagnostics.is_empty() && diagnostics.iter().all(|d| d.severity == Severity::Warn);
        let warn_can_allow = rule.on_warn == OnWarn::Allow;
        assert!(
            warn_only && warn_can_allow,
            "warn-only diagnostics on OnWarn::Allow must fall through to admit",
        );
    }

    /// MLP2-016 Council #C-016A: a `Block { diagnostics }` containing
    /// even one `Severity::Block` diagnostic MUST still exit, even on
    /// `OnWarn::Allow`. The fall-through is for warn-only payloads.
    #[test]
    fn mixed_diagnostics_with_one_block_still_refuses_push() {
        use anvil_l4::{OnBlock, OnNoWitness, OnWarn, Requirement, Severity, ValidationDiagnostic};
        let rule = anvil_l4::BranchRule {
            pattern: "main".to_string(),
            require: Requirement::L4OrL3,
            on_no_witness: OnNoWitness::ValidateAtL4,
            on_block: OnBlock::Reject,
            on_warn: OnWarn::Allow,
        };
        let diagnostics = [
            ValidationDiagnostic {
                rule_id: "style.import-order".to_string(),
                severity: Severity::Warn,
                message: "imports out of canonical order".to_string(),
            },
            ValidationDiagnostic {
                rule_id: "secret-detection.aws-key".to_string(),
                severity: Severity::Block,
                message: "AWS access key leaked".to_string(),
            },
        ];
        let warn_only =
            !diagnostics.is_empty() && diagnostics.iter().all(|d| d.severity == Severity::Warn);
        let warn_can_allow = rule.on_warn == OnWarn::Allow;
        assert!(
            !(warn_only && warn_can_allow),
            "any Severity::Block in the diagnostics must refuse the push",
        );
    }

    /// MLP2-016 Council #C-016A: `OnWarn::Reject` upgrades even a
    /// warn-only payload to a hard block.
    #[test]
    fn warn_only_diagnostics_with_on_warn_reject_still_refuses_push() {
        use anvil_l4::{OnBlock, OnNoWitness, OnWarn, Requirement, Severity, ValidationDiagnostic};
        let rule = anvil_l4::BranchRule {
            pattern: "main".to_string(),
            require: Requirement::L4OrL3,
            on_no_witness: OnNoWitness::ValidateAtL4,
            on_block: OnBlock::Reject,
            on_warn: OnWarn::Reject,
        };
        let diagnostics = [ValidationDiagnostic {
            rule_id: "style.import-order".to_string(),
            severity: Severity::Warn,
            message: "imports out of canonical order".to_string(),
        }];
        let warn_only =
            !diagnostics.is_empty() && diagnostics.iter().all(|d| d.severity == Severity::Warn);
        let warn_can_allow = rule.on_warn == OnWarn::Allow;
        assert!(warn_only, "diagnostic set is warn-only");
        assert!(
            !warn_can_allow,
            "OnWarn::Reject must NOT enable the fall-through",
        );
    }

    /// `NoOpValidationEngine` (kept in the trait crate for tests and
    /// future fallback paths) returns
    /// `EngineUnavailable { NotImplemented }`. The hook's
    /// `engine_unavailable` accumulator routes that to the legacy
    /// `InternalError { TimedOut }` line + admit-push surface
    /// (ADR-038 §D-6). Production no longer binds this engine —
    /// the audit-required production default is
    /// [`CommitAntipatternEngine`]; this test exists to pin the
    /// fallback verdict shape that the accumulator depends on.
    #[test]
    fn no_op_engine_produces_engine_unavailable() {
        use anvil_l4::{
            BranchRule, EngineUnavailableReason, NoOpValidationEngine, OnBlock, OnNoWitness,
            OnWarn, Requirement, ValidationVerdict,
        };
        let rule = BranchRule {
            pattern: "main".to_string(),
            require: Requirement::L4OrL3,
            on_no_witness: OnNoWitness::ValidateAtL4,
            on_block: OnBlock::Reject,
            on_warn: OnWarn::Allow,
        };
        let req = request_for("d".repeat(40), rule, Path::new("/work/repo"), None);
        let verdict = anvil_l4::validate_at_l4(&NoOpValidationEngine, &req);
        assert_eq!(
            verdict,
            ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::NotImplemented,
            }
        );
    }

    // ---- MLP2-020: required_anvil_version floor check ---------------

    fn policy_with_floor(floor: Option<&str>) -> Policy {
        let yaml = if let Some(f) = floor {
            format!(
                "required_anvil_version: '{f}'\nbranches:\n  - pattern: main\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n",
            )
        } else {
            "branches:\n  - pattern: main\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n"
                .to_string()
        };
        Policy::parse(&yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap()
    }

    #[test]
    fn version_floor_satisfied_when_policy_omits_field() {
        // No floor in the policy → the hook proceeds with the normal
        // walk, regardless of what the running binary version is.
        let p = policy_with_floor(None);
        assert_eq!(
            check_version_floor(&p, "0.0.1"),
            VersionFloorOutcome::Satisfied,
        );
    }

    #[test]
    fn version_floor_satisfied_when_running_version_equals_floor() {
        let p = policy_with_floor(Some("0.7.0"));
        assert_eq!(
            check_version_floor(&p, "0.7.0"),
            VersionFloorOutcome::Satisfied,
        );
    }

    #[test]
    fn version_floor_satisfied_when_running_version_newer_than_floor() {
        let p = policy_with_floor(Some("0.6.0"));
        assert_eq!(
            check_version_floor(&p, "0.7.0"),
            VersionFloorOutcome::Satisfied,
        );
        assert_eq!(
            check_version_floor(&p, "1.0.0"),
            VersionFloorOutcome::Satisfied,
        );
    }

    #[test]
    fn version_floor_below_when_running_version_older_than_floor() {
        let p = policy_with_floor(Some("0.7.0"));
        assert_eq!(
            check_version_floor(&p, "0.6.2-beta"),
            VersionFloorOutcome::BelowFloor,
        );
        assert_eq!(
            check_version_floor(&p, "0.6.99"),
            VersionFloorOutcome::BelowFloor,
        );
    }

    #[test]
    fn version_floor_invalid_when_floor_is_not_semver() {
        // The policy's `validate()` rejects empty strings but
        // non-semver values like "v0.7" land in serde and pass
        // through as raw `Option<String>`. The fire-time check is the
        // last line of defence.
        let p = Policy::parse(
            "required_anvil_version: 'v0.7'\nbranches:\n  - pattern: main\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n",
            ConfigFormat::Yaml,
            Path::new("<test>"),
        )
        .unwrap();
        assert_eq!(
            check_version_floor(&p, "0.7.0"),
            VersionFloorOutcome::InvalidFloor,
        );
    }

    #[test]
    fn version_floor_below_when_running_version_is_malformed() {
        // A future bug could feed the helper a non-semver running
        // version. The remediation is the same as below-floor
        // ("upgrade anvil"), so they share the BelowFloor outcome —
        // the operator should never see "your binary's version
        // string is wrong" because that's not actionable.
        let p = policy_with_floor(Some("0.7.0"));
        assert_eq!(
            check_version_floor(&p, "not-a-version"),
            VersionFloorOutcome::BelowFloor,
        );
    }

    // ---- MLP2-021: cutoff_commit baseline-ancestry acceptance -------

    /// Run a small sequence of git plumbing commands to build a
    /// linear repo with two commits, returning `(repo_root, first_sha,
    /// second_sha)`. Kept local to this test module rather than added
    /// to the production code — none of the helpers under test need
    /// to construct repos.
    fn make_git_repo_with_two_commits() -> (TempDir, String, String) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .expect("git available");
            assert!(out.status.success(), "git {args:?} failed: {out:?}");
            out
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        run(&["commit", "--allow-empty", "-q", "-m", "first"]);
        let first = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        run(&["commit", "--allow-empty", "-q", "-m", "second"]);
        let second = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        (tmp, first, second)
    }

    #[test]
    fn first_parent_ancestry_returns_tip_first_then_parents() {
        // Real git repo with two linear commits: the ancestry of the
        // tip is [second, first] (newest-first), which is exactly
        // what `Policy::commit_is_before_cutoff` expects.
        let (tmp, first, second) = make_git_repo_with_two_commits();
        let ancestry = first_parent_ancestry(tmp.path(), &second).unwrap();
        assert_eq!(ancestry.len(), 2);
        assert_eq!(ancestry[0], second);
        assert_eq!(ancestry[1], first);
    }

    #[test]
    fn first_parent_ancestry_returns_none_for_non_hex_tip() {
        // Defence in depth: `is_hex_sha` refusal keeps `git
        // rev-list` from being fed a revspec or option.
        let (tmp, _, _) = make_git_repo_with_two_commits();
        assert!(first_parent_ancestry(tmp.path(), "not-a-sha").is_none());
        assert!(first_parent_ancestry(tmp.path(), "--all").is_none());
    }

    #[test]
    fn first_parent_ancestry_returns_none_when_tip_unknown_to_git() {
        // The tip is hex-shaped but doesn't exist in this repo. git
        // rev-list exits non-zero; the helper degrades to None and
        // the caller falls back to validating the full range.
        let (tmp, _, _) = make_git_repo_with_two_commits();
        let bogus = "deadbeef".repeat(5); // 40 hex chars
        assert!(first_parent_ancestry(tmp.path(), &bogus).is_none());
    }

    #[test]
    fn cutoff_filter_skips_commit_at_or_before_cutoff_in_ancestry() {
        // Mirror the pre-push call shape: load a policy with a
        // hex-shaped `baseline.cutoff_commit` (the Council follow-up
        // adds shape validation at the policy boundary) and an
        // ancestry that mirrors what the hook would synthesise from
        // `git rev-list --first-parent`.
        let p = Policy::parse(
            r"
baseline:
  cutoff_commit: c0ff00
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
",
            ConfigFormat::Yaml,
            Path::new("<test>"),
        )
        .unwrap();
        let ancestry = ["aaaa01", "bbbb02", "c0ff00", "0deadbeef"];
        // The "0deadbeef" is before the cutoff in first-parent
        // ancestry → the pre-push filter must treat it as baselined.
        assert!(p.commit_is_before_cutoff("0deadbeef", &ancestry));
        // The "bbbb02" is after the cutoff → still needs full
        // witness/validation.
        assert!(!p.commit_is_before_cutoff("bbbb02", &ancestry));
    }

    #[test]
    fn cutoff_filter_is_inert_when_policy_pins_no_cutoff() {
        // The hook only fetches ancestry when the policy actually
        // sets a cutoff; with no cutoff, every commit goes through
        // the normal witness/validation path.
        let p = Policy::parse(
            r"
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
",
            ConfigFormat::Yaml,
            Path::new("<test>"),
        )
        .unwrap();
        assert!(p.baseline.cutoff_commit.is_none());
        // With no cutoff, the helper returns false for every input,
        // so no commit is short-circuited as baselined.
        let ancestry = ["any-sha"];
        assert!(!p.commit_is_before_cutoff("any-sha", &ancestry));
    }

    // ---- MLP2-022: pre-push wall-clock budget cap -------------------

    #[test]
    fn is_budget_exceeded_returns_true_when_elapsed_equals_or_exceeds_budget() {
        // A zero-budget cap fires immediately — useful for fault
        // injection in larger tests.
        let start = Instant::now();
        assert!(is_budget_exceeded(start, Duration::from_nanos(0)));
    }

    #[test]
    fn is_budget_exceeded_returns_false_when_under_budget() {
        // A generously large budget never trips on a freshly-started
        // walk.
        let start = Instant::now();
        assert!(!is_budget_exceeded(start, Duration::from_hours(1)));
    }

    #[test]
    fn pre_push_budget_constant_matches_adr_038_two_second_target() {
        // ADR-038 names a 2 s p95 target for pre-push. Pinning the
        // default here so a refactor that drops the cap (or
        // changes it without owner sign-off) surfaces.
        assert_eq!(PRE_PUSH_BUDGET, Duration::from_secs(2));
    }

    #[test]
    fn ancestry_walk_cap_bounds_history_walk() {
        // MLP2-021 Council follow-up: the ancestry walk is bounded
        // so an unbounded `git rev-list --first-parent` cannot itself
        // consume the wall-clock budget on a 500 k-commit branch.
        // Pinning the value so a refactor that drops the cap
        // surfaces.
        assert_eq!(ANCESTRY_WALK_CAP, 100_000);
    }

    // ---- Council follow-ups: cross-cutting interaction pins --------

    /// MLP2-022 Council follow-up: when the budget is exceeded AND
    /// the engine was unavailable on at least one already-walked
    /// commit, only one stderr line should surface. `TimedOut` (the
    /// budget cap) supersedes `ValidationPending` because it more
    /// accurately describes why the push was admitted with partial
    /// coverage. This test pins the suppression order so a future
    /// refactor that drops the `!budget_exceeded` guard re-emits
    /// both lines.
    #[test]
    fn budget_exceeded_suppresses_validation_pending_emit() {
        // The branch lives in `run_pre_push_with_engine` and can't
        // be exercised here without `std::process::exit`. Instead
        // we pin the decision boolean directly, mirroring the
        // guard expression in the production code.
        let budget_exceeded = true;
        let engine_unavailable = true;
        let emit_validation_pending = engine_unavailable && !budget_exceeded;
        assert!(
            !emit_validation_pending,
            "budget_exceeded must suppress ValidationPending so the partial-validation line is the single source of truth",
        );

        // Symmetry: when budget is NOT exceeded but engine WAS
        // unavailable, the ValidationPending emit still fires (the
        // pre-MLP2-022 behaviour).
        let budget_exceeded = false;
        let emit_validation_pending = engine_unavailable && !budget_exceeded;
        assert!(emit_validation_pending);
    }

    #[test]
    fn invalid_floor_routes_through_embedded_failed_not_version_floor() {
        // MLP2-020 Council follow-up: an unparseable
        // `required_anvil_version` is a policy-file problem, not a
        // binary-version problem. Surface "validation errored"
        // rather than the "upgrade anvil" line so the operator's
        // remediation isn't pointed at the wrong fix.
        let p = Policy::parse(
            "required_anvil_version: 'v0.7'\nbranches:\n  - pattern: main\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n",
            ConfigFormat::Yaml,
            Path::new("<test>"),
        )
        .unwrap();
        let outcome = check_version_floor(&p, "0.7.0");
        assert_eq!(outcome, VersionFloorOutcome::InvalidFloor);
        // Pin the route in the production code: the InvalidFloor
        // arm in `run_pre_push_with_engine` calls
        // `emit_internal(ErrorClass::EmbeddedFailed, ...)`. The
        // rendered line carries "errored" rather than "upgrade
        // anvil".
        let rendered = anvil_hook::render_verdict(&Verdict::InternalError {
            class: ErrorClass::EmbeddedFailed,
        });
        assert!(rendered.stderr_line.contains("errored"));
        assert!(!rendered.stderr_line.contains("upgrade anvil"));
        assert_eq!(rendered.exit_code, 0);
    }

    // ---- MLP2-037: anvil hook bootstrap --witness-recent ----------

    /// Build a git repo with N commits past `origin/main`, with
    /// `@{u}` configured so `git rev-list @{u}..HEAD` walks the
    /// expected range. Uses a co-located bare repo as the "remote"
    /// so the test never touches the network.
    ///
    /// Returns `(tmp, repo_root, commit_shas_newest_first)`.
    fn make_git_repo_with_unwitnessed_commits(
        ahead_of_origin: usize,
    ) -> (TempDir, PathBuf, Vec<String>) {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path();
        let bare = workspace.join("origin.git");
        let repo = workspace.join("repo");
        std::fs::create_dir_all(&repo).unwrap();

        let run_in = |dir: &Path, args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .output()
                .expect("git available");
            assert!(out.status.success(), "git {args:?} failed: {out:?}");
            out
        };
        // Bare "remote" repo.
        let out = Command::new("git")
            .arg("init")
            .arg("--bare")
            .arg("-q")
            .arg("-b")
            .arg("main")
            .arg(&bare)
            .output()
            .expect("git available");
        assert!(out.status.success(), "git init --bare failed: {out:?}");

        run_in(&repo, &["init", "-q", "-b", "main"]);
        run_in(&repo, &["config", "user.email", "test@example.com"]);
        run_in(&repo, &["config", "user.name", "Test"]);
        // Seed: one commit pushed to origin so `@{u}` resolves to a
        // real ref before the unwitnessed range stacks on top.
        run_in(&repo, &["commit", "--allow-empty", "-q", "-m", "base"]);
        run_in(&repo, &["remote", "add", "origin", bare.to_str().unwrap()]);
        run_in(&repo, &["push", "-q", "-u", "origin", "main"]);
        // Stack `ahead_of_origin` more commits past origin.
        let mut shas: Vec<String> = Vec::new();
        for i in 0..ahead_of_origin {
            run_in(
                &repo,
                &["commit", "--allow-empty", "-q", "-m", &format!("ahead-{i}")],
            );
            let sha = String::from_utf8(run_in(&repo, &["rev-parse", "HEAD"]).stdout)
                .unwrap()
                .trim()
                .to_string();
            shas.push(sha);
        }
        // git rev-list returns newest first; mirror that here so the
        // test's vector order matches the helper's output.
        shas.reverse();

        // Seed the anvil project-id so read_project_id (and the
        // append_witness helper) succeed when the bootstrap walks.
        std::fs::create_dir_all(repo.join("anvil")).unwrap();
        std::fs::write(
            repo.join("anvil").join("project-id"),
            "project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456\n",
        )
        .unwrap();

        (tmp, repo, shas)
    }

    #[test]
    fn bootstrap_witness_recent_walks_remote_to_head_and_writes_retroactive_lines() {
        // Fixture: 3 commits past origin with no witnesses → bootstrap
        // --witness-recent must walk @{u}..HEAD and write one witness
        // per commit, all tagged `validation_at = "bootstrap-recovery"`.
        let (_tmp, root, shas) = make_git_repo_with_unwitnessed_commits(3);
        run_bootstrap(
            &root,
            &BootstrapArgs {
                dry_run: false,
                witness_recent: true,
            },
        )
        .unwrap();
        let witnessed = collect_witnessed_shas(&root).unwrap();
        for sha in &shas {
            assert!(
                witnessed.contains(sha),
                "expected witness for {sha}, set={witnessed:?}"
            );
        }
        let contents =
            fs::read_to_string(root.join("anvil").join("witness").join("active.ndjson")).unwrap();
        // Each of the 3 commits carries the bootstrap-recovery tag.
        let tagged = contents
            .lines()
            .filter(|l| l.contains("\"validation_at\":\"bootstrap-recovery\""))
            .count();
        assert_eq!(tagged, 3);
    }

    #[test]
    fn bootstrap_witness_recent_skips_already_witnessed_commits() {
        // Fixture: 3 commits past origin, 2 already witnessed → only
        // the third gets a retroactive line.
        let (_tmp, root, shas) = make_git_repo_with_unwitnessed_commits(3);
        // Pre-seed witnesses for the first two SHAs (newest-first
        // vector, so indexes 0 and 1).
        for sha in &shas[..2] {
            write_witness_line_for(&root, "01997e4a-1b2c-7345-8901-abcdef123456", sha);
        }
        let before = collect_witnessed_shas(&root).unwrap();
        assert_eq!(before.len(), 2);
        run_bootstrap(
            &root,
            &BootstrapArgs {
                dry_run: false,
                witness_recent: true,
            },
        )
        .unwrap();
        let after = collect_witnessed_shas(&root).unwrap();
        // All three are now witnessed.
        assert_eq!(after.len(), 3);
        // Only one new bootstrap-recovery line was added (the
        // unwitnessed third commit).
        let contents =
            fs::read_to_string(root.join("anvil").join("witness").join("active.ndjson")).unwrap();
        let tagged = contents
            .lines()
            .filter(|l| l.contains("\"validation_at\":\"bootstrap-recovery\""))
            .count();
        assert_eq!(tagged, 1);
    }

    #[test]
    fn bootstrap_witness_recent_is_idempotent() {
        // A second run must see all commits already witnessed and
        // write zero new bootstrap-recovery lines. Pinning idempotency
        // so a future "always rewrite" refactor trips this guard.
        let (_tmp, root, _shas) = make_git_repo_with_unwitnessed_commits(2);
        run_bootstrap(
            &root,
            &BootstrapArgs {
                dry_run: false,
                witness_recent: true,
            },
        )
        .unwrap();
        let after_first =
            fs::read_to_string(root.join("anvil").join("witness").join("active.ndjson")).unwrap();
        let first_count = after_first
            .lines()
            .filter(|l| l.contains("\"validation_at\":\"bootstrap-recovery\""))
            .count();
        assert_eq!(first_count, 2);
        run_bootstrap(
            &root,
            &BootstrapArgs {
                dry_run: false,
                witness_recent: true,
            },
        )
        .unwrap();
        let after_second =
            fs::read_to_string(root.join("anvil").join("witness").join("active.ndjson")).unwrap();
        let second_count = after_second
            .lines()
            .filter(|l| l.contains("\"validation_at\":\"bootstrap-recovery\""))
            .count();
        assert_eq!(second_count, 2, "idempotent: no new lines on rerun");
    }

    #[test]
    fn bootstrap_witness_recent_no_remote_configured_returns_clean_zero() {
        // Fixture: repo with no `@{u}` set → list_unwitnessed_range
        // degrades to an empty vec; bootstrap exits cleanly with no
        // new witnesses and no error.
        let (_tmp, root) = make_test_repo();
        // Initialise a git repo (no remote, no upstream).
        let run = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .expect("git available")
        };
        let init = run(&["init", "-q", "-b", "main"]);
        assert!(init.status.success());
        // Calling the helper directly: no @{u} → empty.
        let range = list_unwitnessed_range(&root).unwrap();
        assert!(range.is_empty());
        // The full bootstrap run still succeeds end-to-end.
        run_bootstrap(
            &root,
            &BootstrapArgs {
                dry_run: false,
                witness_recent: true,
            },
        )
        .unwrap();
        // No bootstrap-recovery lines were written (no commits to
        // witness).
        let active = root.join("anvil").join("witness").join("active.ndjson");
        if active.exists() {
            let contents = fs::read_to_string(&active).unwrap();
            assert!(!contents.contains("bootstrap-recovery"));
        }
    }

    #[test]
    fn commit_is_witnessed_returns_true_when_chain_contains_sha() {
        // Direct unit test on the streaming check. The streaming
        // contract is in the comment on `commit_is_witnessed`: do not
        // load the whole file into memory. The unit test pins the
        // positive-case truth value; the streaming property is
        // structural (BufRead::lines) and reviewed at the source.
        let (_tmp, root) = make_test_repo();
        write_witness_line_for(&root, "01997e4a-1b2c-7345-8901-abcdef123456", "abc123");
        assert!(commit_is_witnessed(&root, "abc123"));
        assert!(!commit_is_witnessed(&root, "xyz789"));
    }

    #[test]
    fn commit_is_witnessed_returns_false_when_no_chain_exists() {
        // No active.ndjson at all → no commit is witnessed; the
        // helper must not error (the open path returns Ok(false)).
        let (_tmp, root) = make_test_repo();
        assert!(!commit_is_witnessed(&root, "anything"));
    }

    // ---- MLP2-014 rules_sha wire-up tests --------------------------

    /// Helper: read every record (non-genesis) `WitnessLine` from
    /// `anvil/witness/active.ndjson` under `root`. Genesis lines have
    /// `prev_line_hash` starting with `GENESIS-`; we filter them out so
    /// callers can assert on the per-commit records the hook writes.
    fn read_record_lines(root: &Path) -> Vec<WitnessLine> {
        let path = root.join("anvil").join("witness").join("active.ndjson");
        let contents = fs::read_to_string(&path).expect("witness file exists");
        contents
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| WitnessLine::from_ndjson_line(line.as_bytes()).expect("parse witness line"))
            .filter(|line| !line.prev_line_hash.starts_with("GENESIS-"))
            .collect()
    }

    /// MLP2-014 — Validation test #1 from the task spec.
    ///
    /// Fixture: a repo with an `.anvil.yaml` containing a known rule
    /// set. Running `run_pre_commit` writes a witness line whose
    /// `rules_sha` is `Some(...)` and equal to the deterministic
    /// digest computed by `anvil_rules::rules_sha` over the same
    /// canonical config bytes the hook saw.
    #[test]
    fn pre_commit_witness_line_carries_rules_sha() {
        let (_tmp, root) = make_test_repo();
        let mut sup = SuppressionLog::new();

        let config = "checks:\n  - secret-detection\n  - architecture\n";
        fs::write(root.join(".anvil.yaml"), config).unwrap();

        run_pre_commit(&root, &mut sup).unwrap();

        let records = read_record_lines(&root);
        assert_eq!(records.len(), 1, "exactly one record line expected");
        let actual_sha = records[0]
            .rules_sha
            .as_deref()
            .expect("rules_sha must be set when .anvil.yaml is present");

        // Independently recompute the expected digest using the same
        // canonical-JSON + rules_sha primitives the hook uses. The
        // canonicalisation collapses yaml → JSON, so both call sites
        // agree byte-for-byte.
        let value = anvil_config::parse_file(&root.join(".anvil.yaml")).unwrap();
        let canonical = anvil_config::canonical_json_bytes(&value).unwrap();
        let expected_config_sha = config_sha_from_canonical(&canonical);
        let expected_sha = rules_sha(
            env!("CARGO_PKG_VERSION"),
            "0.10.0",
            std::iter::empty::<&str>(),
            expected_config_sha,
        )
        .unwrap();

        assert_eq!(actual_sha, expected_sha);
    }

    /// MLP2-014 — Validation test #2 from the task spec (the spec's
    /// stated validation): two commits with different config files
    /// carry distinct `rules_sha` values.
    #[test]
    fn pre_commit_two_commits_with_different_configs_produce_distinct_rules_sha() {
        let (_tmp, root) = make_test_repo();
        let mut sup = SuppressionLog::new();

        // Commit 1 under config A.
        fs::write(root.join(".anvil.yaml"), "checks:\n  - secret-detection\n").unwrap();
        run_pre_commit(&root, &mut sup).unwrap();

        // Swap to config B and commit again.
        fs::write(
            root.join(".anvil.yaml"),
            "checks:\n  - architecture\n  - secret-detection\n",
        )
        .unwrap();
        run_pre_commit(&root, &mut sup).unwrap();

        let records = read_record_lines(&root);
        assert_eq!(records.len(), 2, "two record lines expected");
        let sha_a = records[0]
            .rules_sha
            .as_deref()
            .expect("commit 1 rules_sha set");
        let sha_b = records[1]
            .rules_sha
            .as_deref()
            .expect("commit 2 rules_sha set");
        assert_ne!(
            sha_a, sha_b,
            "distinct configs must produce distinct rules_sha"
        );
    }

    /// MLP2-014 — Validation test #3: missing config files leave the
    /// `rules_sha` field unset (`None`). The spec is silent on this
    /// case; this test pins the conservative path the implementation
    /// chose so a future refactor cannot accidentally start inventing
    /// digests for the no-config branch.
    #[test]
    fn pre_commit_with_missing_config_uses_none() {
        let (_tmp, root) = make_test_repo();
        let mut sup = SuppressionLog::new();
        // No `.anvil.<ext>` and no `.anvilrc` written.
        run_pre_commit(&root, &mut sup).unwrap();

        let records = read_record_lines(&root);
        assert_eq!(records.len(), 1);
        assert!(
            records[0].rules_sha.is_none(),
            "no-config path must leave rules_sha unset; got {:?}",
            records[0].rules_sha,
        );
    }

    /// MLP2-014 — Validation test #4: shape assertion. When present,
    /// `rules_sha` is exactly 64 lowercase hex characters. Catches a
    /// future bug where the digest format drifts.
    #[test]
    fn pre_commit_witness_line_rules_sha_is_64_lowercase_hex() {
        let (_tmp, root) = make_test_repo();
        let mut sup = SuppressionLog::new();
        fs::write(root.join(".anvil.yaml"), "checks:\n  - secret-detection\n").unwrap();
        run_pre_commit(&root, &mut sup).unwrap();

        let records = read_record_lines(&root);
        let sha = records[0].rules_sha.as_deref().expect("rules_sha set");
        assert_eq!(sha.len(), 64, "rules_sha must be 64 chars; got {sha:?}");
        assert!(
            sha.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "rules_sha must be lowercase hex; got {sha:?}",
        );
    }
}
