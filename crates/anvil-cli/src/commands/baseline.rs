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
    Baseline, BaselineFinding, BaselineMetadata, REFRESH_DEGRADED_REASON, RefreshSuspicion,
    SuspicionThresholds, analyze_refresh, compute_fingerprint, load as load_baseline,
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

/// MLP2-036: default scan budget when `--scan-budget` is unset.
/// Picked so realistic single-language projects (≤ a few tens of
/// thousands of source files) finish in one shot, but a 100k+-file
/// monorepo trips the cap and produces a partial baseline that the
/// operator can resume incrementally.
const DEFAULT_SCAN_BUDGET: usize = 50_000;

#[derive(Debug, Args)]
pub struct BaselineArgs {
    #[command(subcommand)]
    command: Option<BaselineCommand>,
    /// Refresh an existing baseline at HEAD; updates `created_at`
    /// and preserves `cutoff_commit`. Ignored when a subcommand
    /// (e.g. `verify`) is given.
    #[arg(long)]
    refresh: bool,
    /// Mint a fresh project UUID and record the previous one as
    /// `forked_from`. Use after cloning a repo whose
    /// `anvil/project-id` was inherited from the parent and you want
    /// the fork to carry its own identity.
    #[arg(long = "new-identity")]
    new_identity: bool,
    /// Override the adversarial-refresh detector's drop-ratio
    /// threshold (default 0.75). Refresh runs that remove
    /// ≥ratio × old findings AND ≥`--suspicion-min-removed` findings
    /// refuse to save until the operator re-runs with
    /// `--accept-suspicious`. Set above `1.0` (e.g. `1.01`) to
    /// disable the detector entirely.
    #[arg(long = "suspicion-ratio")]
    suspicion_ratio: Option<f64>,
    /// Override the adversarial-refresh detector's minimum-removed
    /// gate (default 10). Prevents the alert from firing on tiny
    /// baselines where a 100% drop is statistically meaningless.
    #[arg(long = "suspicion-min-removed")]
    suspicion_min_removed: Option<usize>,
    /// Explicitly acknowledge that a suspicious-looking refresh
    /// (large finding drop) is intentional. Without this flag,
    /// `anvil baseline --refresh` refuses to save when both the
    /// ratio and minimum-removed thresholds are crossed.
    #[arg(long = "accept-suspicious")]
    accept_suspicious: bool,
    /// Cap the number of files scanned in a single `anvil baseline`
    /// invocation. When the worktree exceeds the budget, the
    /// baseline is written as `partial=true` with a cursor naming
    /// the next file to pick up; a follow-up call resumes from that
    /// cursor. Default 50000. Zero is rejected because it would
    /// produce a never-converging resume loop.
    #[arg(long = "scan-budget", value_parser = parse_scan_budget)]
    scan_budget: Option<usize>,
}

/// MLP2-036: clap value parser for `--scan-budget`. Rejects `0`
/// because the resume loop would otherwise advance zero files per
/// invocation, producing a partial baseline that never converges to
/// complete (Council quick #C-2 MAJOR).
fn parse_scan_budget(raw: &str) -> Result<usize, String> {
    let n: usize = raw
        .parse()
        .map_err(|e| format!("`--scan-budget {raw}` is not a non-negative integer: {e}"))?;
    if n == 0 {
        return Err(
            "`--scan-budget 0` would produce a never-converging resume loop; pass at least 1"
                .to_string(),
        );
    }
    Ok(n)
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
        None => {
            // MLP2-035: assemble the suspicion thresholds from any
            // CLI overrides on top of the library defaults. Pass
            // through unconditionally — the detector only runs on a
            // refresh path with a non-empty prior baseline anyway.
            let mut thresholds = SuspicionThresholds::default();
            if let Some(r) = args.suspicion_ratio {
                thresholds.removed_ratio_threshold = r;
            }
            if let Some(m) = args.suspicion_min_removed {
                thresholds.minimum_removed = m;
            }
            run_create_or_refresh(
                &repo_root,
                args.refresh,
                args.new_identity,
                &thresholds,
                args.accept_suspicious,
                args.scan_budget.unwrap_or(DEFAULT_SCAN_BUDGET),
            )
        }
    }
}

// Threads identity, suspicion, partial-scan + cutoff pin in one
// orchestrator pass; splitting out micro-helpers obscures the
// lifecycle.
#[allow(clippy::too_many_lines)]
fn run_create_or_refresh(
    repo_root: &Path,
    refresh: bool,
    new_identity: bool,
    suspicion_thresholds: &SuspicionThresholds,
    accept_suspicious: bool,
    scan_budget: usize,
) -> Result<()> {
    // DISTRIB-006 (ADR-060): `anvil baseline` create/refresh is a durable
    // per-project mutation command. Refuse the WHOLE flow under a non-default
    // ANVIL_HOME without `--touch-project-state` — this must precede the identity
    // step below, because `--new-identity` (`mint_new_identity`) overwrites
    // `anvil/project-id` and `ensure_project_id` may create it; both are durable
    // state the production binary reads. Refusing here keeps the candidate from
    // touching project identity, the baseline, or the cutoff pin.
    crate::install_root::ensure_project_write_allowed("baseline write")?;

    let mut refresh = refresh;
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
    // MLP2-036: a partial baseline is incomplete by design — the
    // user's intent on a follow-up `anvil baseline` invocation is
    // to make progress, not to be told "use --refresh". Auto-treat
    // partial as a resume. `--new-identity` (Council #C-1 MAJOR)
    // forces a fresh accumulator — carrying findings scanned under
    // the prior identity into the new one would violate the reset
    // semantics. `--refresh` is the explicit re-scan path.
    let mut is_partial_resume = existing.as_ref().is_some_and(|b| b.partial) && !new_identity;
    // MLP2-065: detect tree drift before silently skipping pre-cursor
    // files. If the saved partial fingerprint disagrees with the
    // current tree's pre-cursor hash, a file was added, renamed, or
    // removed before the cursor between runs — restart the scan so
    // the new file is included before the baseline is marked
    // complete. The same restart fires when an older partial without
    // a fingerprint is loaded ("drift-detection unavailable —
    // restart to be safe").
    if is_partial_resume {
        let saved_fp = existing
            .as_ref()
            .and_then(|b| b.pre_cursor_fingerprint.clone());
        let saved_cursor = existing.as_ref().and_then(|b| b.continuation.clone());
        let drift_reason: Option<&str> = match (saved_fp.as_deref(), saved_cursor.as_deref()) {
            (None, Some(_)) => Some("pre-MLP2-065 baseline carries no drift fingerprint"),
            (Some(saved), Some(cursor)) => match peek_pre_cursor_fingerprint(repo_root, cursor) {
                Some(current) if current == saved => None,
                Some(_) => Some("pre-cursor file list changed since the previous partial pass"),
                None => {
                    // Council quick #4 minor: empty / unscannable
                    // tree at resume time. Restart is safer than
                    // assuming a now-empty pre-cursor list matches
                    // the saved one — emit a distinct reason so the
                    // operator isn't told the file list "changed".
                    Some("no scannable files visible at resume time")
                }
            },
            _ => None,
        };
        if let Some(reason) = drift_reason {
            println!(
                "anvil: baseline restart triggered ({reason}). Discarding the partial cursor and rescanning from the start so new files before the cursor are not silently skipped."
            );
            is_partial_resume = false;
            // Treat the drift restart as an implicit refresh so the
            // existing-baseline guard below doesn't short-circuit
            // the rerun.
            refresh = true;
        }
    }
    if existing.is_some() && !refresh && !new_identity && !is_partial_resume {
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

    // MLP2-034 Phase 1 + MLP2-036: populate findings from the
    // antipattern scanner. When resuming a partial baseline, skip
    // files lexicographically before the saved continuation cursor;
    // the scan returns its own next cursor when the budget is
    // exhausted before all remaining files are processed.
    let resume_cursor: Option<String> = if is_partial_resume {
        existing.as_ref().and_then(|b| b.continuation.clone())
    } else {
        None
    };
    let BudgetedScan {
        findings: new_findings,
        continuation: new_continuation,
        pre_cursor_fingerprint: new_pre_cursor_fingerprint,
    } = scan_repo_for_findings_with_budget_v2(repo_root, scan_budget, resume_cursor.as_deref());

    // Snapshot the prior state for the MLP2-035 suspicion analysis
    // *before* the resume branch consumes `existing`. We only need
    // the findings list and a "was this baseline complete?" bit;
    // the suspicion comparison is meaningless when either side is
    // partial, so a partial prior is treated as "no prior" for the
    // detector's purposes.
    let prior_findings_for_suspicion: Option<Vec<BaselineFinding>> = existing
        .as_ref()
        .filter(|b| !b.partial)
        .map(|b| b.findings.clone());

    let mut baseline = if is_partial_resume {
        // Resume path: take the prior partial baseline as the
        // accumulator and merge this scan's findings into it.
        // Metadata is refreshed so the operator can see when the
        // last partial pass ran; project_uuid is preserved (the
        // identity check above already ensured we have it).
        let mut acc = existing.expect("is_partial_resume implies existing.is_some()");
        acc.metadata = metadata;
        acc.merge_partial_findings(new_findings);
        acc
    } else {
        Baseline::new(metadata, new_findings)
    };
    baseline.cutoff_commit.clone_from(&cutoff);
    baseline.partial = new_continuation.is_some();
    baseline.continuation = new_continuation;
    // MLP2-065: carry the pre-cursor fingerprint alongside the
    // cursor so the next resume can detect tree drift. Cleared on a
    // complete baseline so the on-disk shape stays byte-identical
    // for the common case.
    baseline.pre_cursor_fingerprint = if baseline.partial {
        new_pre_cursor_fingerprint
    } else {
        None
    };

    // Council quick #C-3 (MAJOR): an operator running
    // `--refresh --scan-budget=<small>` against a *complete* prior
    // baseline can drop the on-disk findings to the budgeted
    // prefix's view, marked partial=true, with no detection. Treat
    // that complete → partial transition as an explicit suspicious
    // intent: refuse without `--accept-suspicious`. The freshly-
    // resumed (partial → still-partial) and freshly-created
    // (no prior → partial) paths are unaffected — they're the
    // legitimate huge-monorepo adoption shape.
    if baseline.partial
        && prior_findings_for_suspicion.is_some()
        && !is_partial_resume
        && !accept_suspicious
    {
        println!(
            "anvil: {REFRESH_DEGRADED_REASON} — `--refresh` with `--scan-budget {scan_budget}` would replace the complete prior baseline with a partial snapshot covering only the budgeted prefix. \
Refusing to overwrite anvil/baseline.json without explicit acknowledgement. Re-run with `--accept-suspicious` if you're deliberately re-adopting this monorepo incrementally, or raise `--scan-budget` to cover the full tree."
        );
        return Ok(());
    }

    // MLP2-035: analyse BEFORE saving so a suspicious refresh
    // requires explicit operator acknowledgement. Without this gate
    // the rewrite would commit silently and an adversarial
    // whitewash would only surface as a printed line *after* the
    // damage was already on disk. Skipped when either side is
    // partial (drop ratio is meaningless mid-scan) or when the
    // operator already passed `--accept-suspicious`.
    if !baseline.partial
        && let Some(prior_findings) = prior_findings_for_suspicion.as_ref()
        && let RefreshSuspicion::Suspicious {
            removed_count,
            old_total,
            removed_ratio,
            threshold,
        } = analyze_refresh(prior_findings, &baseline.findings, suspicion_thresholds)
    {
        if accept_suspicious {
            println!(
                "anvil: {REFRESH_DEGRADED_REASON} acknowledged — refresh removed {removed_count} of {old_total} prior findings ({pct:.0}% drop ≥ {thr_pct:.0}% threshold). Proceeding with rewrite per `--accept-suspicious`.",
                pct = removed_ratio * 100.0,
                thr_pct = threshold.removed_ratio_threshold * 100.0,
            );
        } else {
            println!(
                "anvil: {REFRESH_DEGRADED_REASON} — refresh would remove {removed_count} of {old_total} prior findings ({pct:.0}% drop ≥ {thr_pct:.0}% threshold). \
Refusing to overwrite anvil/baseline.json without explicit acknowledgement. Re-run with `--accept-suspicious` if this is a legitimate large refactor, or with `--suspicion-ratio <value>` to permanently adjust the threshold for this project.",
                pct = removed_ratio * 100.0,
                thr_pct = threshold.removed_ratio_threshold * 100.0,
            );
            return Ok(());
        }
    }

    save_baseline(repo_root, &baseline).context("write anvil/baseline.json")?;

    // MLP2-036: distinguish complete from partial in the operator
    // signal. Resume runs say "resumed (partial)" so the operator
    // doesn't think a single 50k-file budget exhausted a 200k-file
    // monorepo means the scan is finished. The cursor is included
    // so the operator knows where they are in the tree.
    let action = match (is_partial_resume, refresh, baseline.partial) {
        (true, _, true) => "resumed (still partial)",
        (true, _, false) => "resumed (now complete)",
        (false, true, true) => "refreshed (partial)",
        (false, true, false) => "refreshed",
        (false, false, true) => "created (partial)",
        (false, false, false) => "created",
    };
    if let Some(cursor) = baseline.continuation.as_deref() {
        println!(
            "anvil: baseline {action} (current posture — {} findings, baselined as-is; resume from `{cursor}` with another `anvil baseline`)",  // CIB-016 pairs with "new regressions — M findings since baseline" in subsequent check/gate/activation scans (see activation/render.rs)
            baseline.findings.len(),
        );
    } else {
        println!(
            "anvil: baseline {action} (current posture — {} findings, baselined as-is)",
            baseline.findings.len(),
        );
    }

    // MLP2-031 ↔ -032: pin the cutoff into `anvil/policy.{yml,…}` so
    // the L4 policy lane reads it from policy rather than from
    // `baseline.json`. Best-effort — a missing or unreadable policy
    // file emits a hint and does not fail the orchestrator. Skipped
    // while the baseline is partial — pinning a cutoff against an
    // incomplete record would lock in a half-state.
    if let Some(sha) = cutoff
        && !baseline.partial
    {
        try_pin_cutoff(repo_root, &sha);
    }

    Ok(())
}

fn run_verify(repo_root: &Path) -> Result<()> {
    let baseline = load_baseline(repo_root)
        .context("load baseline")?
        .context("no baseline at anvil/baseline.json — run `anvil baseline` first")?;
    println!(
        "anvil: baseline ok (current posture — {} findings, baselined as-is; cutoff={})",
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
/// MLP2-036: scan with an explicit file-count budget and an
/// optional resume cursor. Returns the scan's findings plus the
/// next-file cursor when the budget was exhausted before all
/// remaining files were processed (`None` means the scan reached
/// the end of the file list — the baseline is now complete).
///
/// Files are sorted lexicographically by their *repo-relative*
/// path so the cursor is portable across machines and across worktree
/// re-locations. The scanner itself still receives absolute paths;
/// the relative form is bookkeeping for the resume contract.
/// MLP2-065: outcome of one budgeted scan pass. Carries the new
/// findings, the next continuation cursor (if any), and the
/// fingerprint of the pre-cursor file list at scan time so the
/// resume path can detect drift before silently skipping new files.
pub(crate) struct BudgetedScan {
    pub findings: Vec<BaselineFinding>,
    pub continuation: Option<String>,
    /// `Some(hex_sha256)` when the scan produced a partial result
    /// (cursor present). Hash spans the sorted relative paths of
    /// every file lexicographically `< continuation` at scan time.
    /// `None` when the baseline became complete in this pass.
    pub pre_cursor_fingerprint: Option<String>,
}

/// MLP2-065: hash the canonical relative paths that fall before
/// `cursor` in `sorted_pairs`. Used to detect tree drift across
/// resume runs — a new file inserted before the cursor between
/// passes changes this hash, and the resume path forces a restart
/// rather than silently skipping the new file.
fn compute_pre_cursor_fingerprint(sorted_pairs: &[(String, String)], cursor: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for (rel, _) in sorted_pairs {
        if rel.as_str() >= cursor {
            break;
        }
        hasher.update(rel.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

/// MLP2-065: budgeted scan that additionally returns the pre-cursor
/// fingerprint so the resume path can detect tree drift before
/// marking the baseline complete.
pub(crate) fn scan_repo_for_findings_with_budget_v2(
    repo_root: &Path,
    budget: usize,
    resume_cursor: Option<&str>,
) -> BudgetedScan {
    // Council quick #C-2 (MAJOR) defence-in-depth: a zero budget
    // would advance zero files per resume, looping forever. The CLI
    // surface rejects this via `parse_scan_budget`; the assert
    // catches programmatic callers that bypass the CLI parser.
    assert!(
        budget > 0,
        "scan budget must be at least 1; zero would produce an infinite resume loop"
    );

    let config = AntipatternCheckConfig::default();
    let absolute_files = collect_scannable_files(repo_root, &config.extensions);
    if absolute_files.is_empty() {
        return BudgetedScan {
            findings: Vec::new(),
            continuation: None,
            pre_cursor_fingerprint: None,
        };
    }

    // Build (relative, absolute) pairs sorted by the relative path so
    // the resume cursor (stored as a relative path on disk) compares
    // against the same key the scanner walks. Paths use forward
    // slashes — Windows-side relative paths get normalised here so
    // baselines round-trip across OSes without re-sorting. Council
    // quick #C-4: paths whose relative form isn't valid UTF-8 are
    // dropped from the budgeted scan (rather than coerced via
    // `to_string_lossy`'s U+FFFD substitution), so a cursor written
    // on one OS resumes correctly on another. Such files are vanishingly
    // rare on real source trees and are surfaced by the rest of the
    // toolchain, not silently scanned.
    let mut pairs: Vec<(String, String)> = absolute_files
        .iter()
        .filter_map(|abs| {
            let rel = Path::new(abs).strip_prefix(repo_root).ok()?;
            let rel_str = rel.to_str()?.replace('\\', "/");
            Some((rel_str, abs.clone()))
        })
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    // Resume: skip files lexicographically before the cursor. The
    // cursor names the *next file to scan*, so equality is included
    // (we resume AT it, not after it).
    let start_idx = match resume_cursor {
        Some(cursor) => pairs
            .binary_search_by(|(rel, _)| rel.as_str().cmp(cursor))
            .unwrap_or_else(|i| i),
        None => 0,
    };

    let remaining = &pairs[start_idx..];
    let scan_count = remaining.len().min(budget);
    let to_scan = &remaining[..scan_count];
    // The continuation cursor is the first file we did NOT scan in
    // this pass. None when the budget covered the entire remaining
    // tail — the baseline is now complete.
    let continuation = remaining.get(budget).map(|(rel, _)| rel.clone());

    // MLP2-065: compute the pre-cursor fingerprint when this pass
    // produces a partial result. The hash spans the relative paths
    // strictly less than `continuation`; the resume path recomputes
    // the same hash against the next tree state to detect files
    // added or renamed before the cursor between runs.
    let pre_cursor_fingerprint = continuation
        .as_deref()
        .map(|cursor| compute_pre_cursor_fingerprint(&pairs, cursor));

    if to_scan.is_empty() {
        return BudgetedScan {
            findings: Vec::new(),
            continuation,
            pre_cursor_fingerprint,
        };
    }

    let file_refs: Vec<&str> = to_scan.iter().map(|(_, abs)| abs.as_str()).collect();
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
    BudgetedScan {
        findings,
        continuation,
        pre_cursor_fingerprint,
    }
}

/// MLP2-065: peek at the current tree and return the fingerprint of
/// the pre-`cursor` file list. Called by `run` before resuming a
/// partial baseline so a drift between save-time and resume-time
/// state forces a full restart rather than silently skipping new
/// files. Mirrors the file-list construction in
/// [`scan_repo_for_findings_with_budget_v2`] so the two hashes
/// compare like-for-like.
///
/// **Known limitation** (Council quick #3 minor): the fingerprint is
/// computed over the Rust-sorted relative path bytes. On a case-
/// insensitive filesystem (macOS HFS+/APFS default, Windows NTFS),
/// a rename that only changes case (`Foo.ts` → `foo.ts`) leaves the
/// canonical relative-path string unchanged and is not flagged as
/// drift. Renames of *other* casings that touch a different filename
/// still trigger restart. The threat model for MLP2-065 is
/// "operator added or renamed a code file before the cursor between
/// resume passes"; case-only renames on a case-insensitive FS are
/// out of scope and accepted as a known limitation.
pub(crate) fn peek_pre_cursor_fingerprint(repo_root: &Path, cursor: &str) -> Option<String> {
    let config = AntipatternCheckConfig::default();
    let absolute_files = collect_scannable_files(repo_root, &config.extensions);
    if absolute_files.is_empty() {
        return None;
    }
    let mut pairs: Vec<(String, String)> = absolute_files
        .iter()
        .filter_map(|abs| {
            let rel = Path::new(abs).strip_prefix(repo_root).ok()?;
            let rel_str = rel.to_str()?.replace('\\', "/");
            Some((rel_str, abs.clone()))
        })
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    Some(compute_pre_cursor_fingerprint(&pairs, cursor))
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
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
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
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let baseline = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(
            baseline.metadata.project_uuid, "01997e4a-1b2c-7345-8901-abcdef123456",
            "existing identity must be preserved across baseline runs"
        );
    }

    #[test]
    fn create_without_refresh_does_not_overwrite_existing() {
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let first = load_baseline(tmp.path()).unwrap().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let second = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(first.metadata.created_at, second.metadata.created_at);
    }

    #[test]
    fn refresh_preserves_cutoff_commit_across_runs() {
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let refreshed = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(refreshed.cutoff_commit.as_deref(), Some("a3b2ea4e"));
    }

    #[test]
    fn refresh_pins_cutoff_into_policy_when_present() {
        // MLP2-031 ↔ -032: when a baseline carries a cutoff_commit
        // and `anvil/policy.yml` exists, the orchestrator pins it.
        let tmp = TempDir::new().unwrap();
        write_policy_yml(tmp.path());

        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();

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

        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();

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

        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();

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
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();
        // No anvil/policy.* file present.
        run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
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
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
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
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let first = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(first.findings.is_empty(), "no antipatterns yet");

        // Introduce a violation and refresh.
        fs::write(tmp.path().join("src/app.ts"), "const v: any = bad;\n").unwrap();
        run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let refreshed = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(
            refreshed.findings.iter().any(|f| f.rule_id == "AP-003"),
            "AP-003 should appear after --refresh"
        );
    }

    #[test]
    fn verify_reports_loaded_baseline() {
        let tmp = TempDir::new().unwrap();
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
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
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let parent_uuid = load_baseline(tmp.path())
            .unwrap()
            .unwrap()
            .metadata
            .project_uuid;

        run_create_or_refresh(
            tmp.path(),
            false,
            true,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();

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
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let first = load_baseline(tmp.path()).unwrap().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // No --refresh, but --new-identity → must rewrite.
        run_create_or_refresh(
            tmp.path(),
            false,
            true,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
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
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.cutoff_commit = Some("a3b2ea4e".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        run_create_or_refresh(
            tmp.path(),
            false,
            true,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();

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
        run_create_or_refresh(
            tmp.path(),
            false,
            true,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let project_id_text = fs::read_to_string(tmp.path().join("anvil/project-id")).unwrap();
        assert!(project_id_text.contains("project_uuid:"));
        assert!(
            !project_id_text.contains("forked_from:"),
            "no parent → no forked_from; got:\n{project_id_text}"
        );
    }

    // ── MLP2-035: adversarial-refresh detection (orchestrator path) ──

    /// Helper: seed an "adversarial" baseline by hand-writing a
    /// large finding set into `anvil/baseline.json`. Bypasses the
    /// scanner so the test is deterministic and doesn't depend on
    /// which antipatterns happen to fire on synthesised content.
    fn seed_baseline_with_n_findings(root: &Path, n: usize) {
        run_create_or_refresh(
            root,
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let mut baseline = load_baseline(root).unwrap().unwrap();
        baseline.findings = (0..n)
            .map(|i| BaselineFinding {
                rule_id: "AP-999".to_string(),
                file_path: format!("src/synth_{i}.ts"),
                fingerprint: format!("{i:04x}{:0>12}", "0"),
            })
            .collect();
        save_baseline(root, &baseline).unwrap();
    }

    #[test]
    fn refresh_refuses_to_save_when_suspicious_without_ack() {
        // Council quick #C-2 (MAJOR): the spec's "explicit
        // acknowledgement" requirement means a suspicious refresh
        // must NOT silently overwrite — the operator must see the
        // detector's output and re-run with --accept-suspicious.
        // The orchestrator returns Ok (warnings-over-blocks: not a
        // hard failure) but does not write.
        let tmp = TempDir::new().unwrap();
        seed_baseline_with_n_findings(tmp.path(), 50);
        let prior_findings = load_baseline(tmp.path()).unwrap().unwrap().findings.clone();

        let result = run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &SuspicionThresholds::default(),
            false, // no acknowledgement
            DEFAULT_SCAN_BUDGET,
        );
        assert!(result.is_ok(), "suspicious refresh must not error");
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(
            after.findings, prior_findings,
            "baseline must NOT be rewritten when refresh is suspicious and unacknowledged"
        );
    }

    #[test]
    fn refresh_proceeds_when_suspicious_with_ack_flag() {
        // Mirror of the test above: with --accept-suspicious set,
        // the rewrite proceeds (and the warning still prints,
        // confirming the operator's choice).
        let tmp = TempDir::new().unwrap();
        seed_baseline_with_n_findings(tmp.path(), 50);
        run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &SuspicionThresholds::default(),
            true, // explicit ack
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(
            after.findings.is_empty(),
            "ack flag must let the rewrite proceed even when suspicious"
        );
    }

    #[test]
    fn refresh_threshold_override_above_one_disables_detection() {
        // Operator escape hatch: the `--suspicion-ratio 1.01` flag
        // (above 1.0) effectively disables the detector.
        // analyze_refresh returns Clean → no ack required.
        let tmp = TempDir::new().unwrap();
        seed_baseline_with_n_findings(tmp.path(), 50);
        let lenient = SuspicionThresholds {
            removed_ratio_threshold: 1.01,
            minimum_removed: 10,
        };
        run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &lenient,
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(after.findings.is_empty());
    }

    #[test]
    fn refresh_at_exactly_1_0_threshold_still_fires() {
        // Council quick #C-1 + #C-5 boundary pin: the help text
        // says "above 1.0 disables" because the comparison is
        // `ratio >= threshold` (strict-less for Clean). At exactly
        // 1.0 the detector still fires on a 100% drop, so the
        // rewrite is refused without --accept-suspicious.
        let tmp = TempDir::new().unwrap();
        seed_baseline_with_n_findings(tmp.path(), 50);
        let prior_findings = load_baseline(tmp.path()).unwrap().unwrap().findings.clone();
        let edge = SuspicionThresholds {
            removed_ratio_threshold: 1.0,
            minimum_removed: 10,
        };
        run_create_or_refresh(tmp.path(), true, false, &edge, false, DEFAULT_SCAN_BUDGET).unwrap();
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(
            after.findings, prior_findings,
            "ratio=1.0 with a 100% drop must still be refused (boundary inclusive)"
        );
    }

    #[test]
    fn refresh_under_minimum_removed_is_clean() {
        // Tiny baseline of 8 → 0 findings would hit ratio 1.0 but
        // is below the default minimum_removed=10 gate, so the
        // detector returns Clean and the rewrite proceeds without
        // ack.
        let tmp = TempDir::new().unwrap();
        seed_baseline_with_n_findings(tmp.path(), 8);
        run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(after.findings.is_empty());
    }

    // ── MLP2-036: partial baseline + resume continuation ──────────

    /// Seed a worktree with `n` source files each containing the
    /// AP-003 antipattern. Files are named `src/file_NNN.ts` so
    /// their lexicographic order is deterministic — important for
    /// the resume cursor's stability across machines.
    fn seed_worktree_with_n_violation_files(root: &Path, n: usize) {
        std::fs::create_dir_all(root.join("src")).unwrap();
        for i in 0..n {
            std::fs::write(
                root.join(format!("src/file_{i:03}.ts")),
                "const v: any = 1;\n",
            )
            .unwrap();
        }
    }

    #[test]
    fn small_worktree_produces_complete_baseline() {
        // Sanity: when the file count is well under the budget the
        // baseline is complete (partial=false, no continuation).
        // Confirms the new code path is byte-compatible with the
        // pre-MLP2-036 default behaviour.
        let tmp = TempDir::new().unwrap();
        seed_worktree_with_n_violation_files(tmp.path(), 5);
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let baseline = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(
            !baseline.partial,
            "small worktree must produce a complete baseline"
        );
        assert!(baseline.continuation.is_none());
        assert!(
            baseline.findings.len() >= 5,
            "expected ≥5 findings (one per file), got {}",
            baseline.findings.len()
        );
    }

    #[test]
    fn budget_smaller_than_filecount_writes_partial_baseline() {
        // MLP2-036 core contract: a budget below the file count
        // produces a partial baseline carrying a continuation
        // cursor naming the next file to scan. Only the budgeted
        // prefix's findings appear in this snapshot.
        let tmp = TempDir::new().unwrap();
        seed_worktree_with_n_violation_files(tmp.path(), 10);
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            3, // budget
        )
        .unwrap();
        let baseline = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(
            baseline.partial,
            "budget < file_count must yield partial=true"
        );
        let cursor = baseline
            .continuation
            .as_deref()
            .expect("partial requires continuation");
        // Files are sorted lexicographically; with 10 files
        // (file_000.ts..file_009.ts) and budget 3, the cursor is
        // file_003.ts (the first NOT scanned).
        assert_eq!(cursor, "src/file_003.ts");
        assert_eq!(
            baseline.findings.len(),
            3,
            "partial scan should carry exactly the budgeted prefix's findings"
        );
    }

    #[test]
    fn resume_continues_from_cursor_and_can_complete() {
        // MLP2-036 validation: a sequence of budget-limited runs
        // produces the same final baseline as a single one-shot
        // scan. Three rounds of budget=4 over 10 files: 4 + 4 +
        // 2, ending with partial=false.
        let tmp = TempDir::new().unwrap();
        seed_worktree_with_n_violation_files(tmp.path(), 10);

        // Round 1: scan first 4, write partial.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            4,
        )
        .unwrap();
        let after_r1 = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(after_r1.partial);
        assert_eq!(after_r1.continuation.as_deref(), Some("src/file_004.ts"));
        assert_eq!(after_r1.findings.len(), 4);

        // Round 2: auto-resume on plain `anvil baseline` (partial
        // detected → resume path bypasses the "use --refresh"
        // short-circuit). Scan files 4..7, write partial.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            3,
        )
        .unwrap();
        let after_r2 = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(after_r2.partial);
        assert_eq!(after_r2.continuation.as_deref(), Some("src/file_007.ts"));
        assert_eq!(after_r2.findings.len(), 7);

        // Round 3: scan files 7..10, finish.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            5,
        )
        .unwrap();
        let after_r3 = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(!after_r3.partial, "final round must clear partial flag");
        assert!(after_r3.continuation.is_none());
        assert_eq!(after_r3.findings.len(), 10);
    }

    /// MLP2-065 regression: a new violating file inserted
    /// lexicographically BEFORE the saved continuation cursor between
    /// resume passes MUST trigger a restart rather than being
    /// silently skipped. Pre-fix the partial baseline was marked
    /// complete without ever scanning the inserted file.
    #[test]
    fn resume_restarts_when_pre_cursor_file_inserted_between_passes() {
        let tmp = TempDir::new().unwrap();
        seed_worktree_with_n_violation_files(tmp.path(), 10);

        // Round 1: partial scan of first 4 files; cursor stops at
        // src/file_004.ts. The pre-cursor fingerprint hashes
        // src/file_000.ts .. src/file_003.ts.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            4,
        )
        .unwrap();
        let after_r1 = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(after_r1.partial);
        assert_eq!(after_r1.continuation.as_deref(), Some("src/file_004.ts"));
        assert!(
            after_r1.pre_cursor_fingerprint.is_some(),
            "partial baseline must record drift fingerprint"
        );

        // Adversary: drop a new violating file BEFORE the cursor
        // (`000-injected.ts` sorts before `src/file_000.ts`).
        std::fs::write(
            tmp.path().join("000-injected.ts"),
            "const v: any = 'leaked';\n",
        )
        .unwrap();

        // Round 2: resume with a budget that would normally complete
        // the scan. With the drift guard the run MUST restart and
        // include `000-injected.ts` instead of silently skipping it.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            20, // big enough to finish the full tree
        )
        .unwrap();
        let after_r2 = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(!after_r2.partial, "post-restart scan must reach completion");
        let paths: std::collections::HashSet<&str> = after_r2
            .findings
            .iter()
            .map(|f| f.file_path.as_str())
            .collect();
        assert!(
            paths.contains("000-injected.ts"),
            "the pre-cursor inserted file must be scanned after restart, got paths: {paths:?}",
        );
    }

    /// MLP2-065: a baseline produced by a pre-MLP2-065 anvil carries
    /// no `pre_cursor_fingerprint`. The resume path treats that as
    /// "drift-detection unavailable — restart to be safe" so older
    /// partial baselines never silently skip files on the next
    /// resume.
    #[test]
    fn resume_restarts_when_partial_baseline_lacks_drift_fingerprint() {
        let tmp = TempDir::new().unwrap();
        seed_worktree_with_n_violation_files(tmp.path(), 6);

        // Build a partial baseline the normal way, then hand-strip
        // the fingerprint to simulate an older on-disk shape.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            2,
        )
        .unwrap();
        let mut legacy = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(legacy.partial);
        assert!(legacy.pre_cursor_fingerprint.is_some());
        legacy.pre_cursor_fingerprint = None;
        save_baseline(tmp.path(), &legacy).unwrap();

        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            20,
        )
        .unwrap();
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(!after.partial, "restart must complete the scan");
        assert_eq!(
            after.findings.len(),
            6,
            "post-restart scan must cover all 6 seeded files"
        );
    }

    #[test]
    fn resume_path_is_idempotent_against_re_scan_of_cursor_file() {
        // If an operator re-runs `anvil baseline` after a partial
        // run without making progress (budget reset to 0 somehow,
        // or rapid double-invocation), the merge_partial_findings
        // dedup must keep the on-disk baseline byte-stable.
        let tmp = TempDir::new().unwrap();
        seed_worktree_with_n_violation_files(tmp.path(), 5);

        // Round 1: scan first 2.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            2,
        )
        .unwrap();
        let after_r1 = load_baseline(tmp.path()).unwrap().unwrap();
        let r1_findings_count = after_r1.findings.len();

        // Round 2: scan from the cursor with another budget=2 →
        // covers files 2 and 3. Should add new findings for those
        // without duplicating the first two.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            2,
        )
        .unwrap();
        let after_r2 = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(
            after_r2.findings.len(),
            r1_findings_count + 2,
            "merge must add 2 new findings (not duplicate the first 2)"
        );

        // Confirm canonical order is preserved (alphabetical by
        // file_path within the same rule_id).
        let files: Vec<_> = after_r2
            .findings
            .iter()
            .map(|f| f.file_path.as_str())
            .collect();
        let mut sorted = files.clone();
        sorted.sort_unstable();
        assert_eq!(files, sorted, "merge must preserve canonical ordering");
    }

    #[test]
    fn one_shot_scan_matches_resumed_scan_byte_for_byte() {
        // MLP2-036 validation fixture per the spec ("full + resumed
        // flow produces same final baseline"): same fixture, two
        // tmp roots, one scanned in one shot and one in chunks of
        // 3. After both reach completion, the canonical bytes must
        // match.
        let one_shot = TempDir::new().unwrap();
        let chunked = TempDir::new().unwrap();
        seed_worktree_with_n_violation_files(one_shot.path(), 7);
        seed_worktree_with_n_violation_files(chunked.path(), 7);

        run_create_or_refresh(
            one_shot.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        // Chunked: 3 + 3 + 1.
        for _ in 0..3 {
            run_create_or_refresh(
                chunked.path(),
                false,
                false,
                &SuspicionThresholds::default(),
                false,
                3,
            )
            .unwrap();
        }

        let one_shot_baseline = load_baseline(one_shot.path()).unwrap().unwrap();
        let chunked_baseline = load_baseline(chunked.path()).unwrap().unwrap();

        // Metadata fields differ (timestamp, project_uuid) so we
        // compare findings + partial + continuation directly.
        assert!(!one_shot_baseline.partial);
        assert!(!chunked_baseline.partial);
        assert_eq!(
            one_shot_baseline.continuation,
            chunked_baseline.continuation
        );
        assert_eq!(
            one_shot_baseline.findings, chunked_baseline.findings,
            "one-shot vs chunked findings must match"
        );
    }

    #[test]
    fn new_identity_with_partial_existing_does_not_resume() {
        // Council quick #C-1 (MAJOR) regression guard: when the
        // operator runs `--new-identity` against a partial
        // baseline, the resume accumulator must NOT be reused —
        // partial findings scanned under the old identity would
        // leak into the new identity's baseline, violating reset
        // semantics.
        let tmp = TempDir::new().unwrap();
        seed_baseline_with_n_findings(tmp.path(), 50);
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.partial = true;
        baseline.continuation = Some("src/zzz_synthetic.ts".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();
        let parent_uuid = baseline.metadata.project_uuid.clone();

        // --new-identity must produce a fresh accumulator + fresh
        // UUID, dropping the prior partial findings.
        run_create_or_refresh(
            tmp.path(),
            false,
            true, // new_identity
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert_ne!(
            after.metadata.project_uuid, parent_uuid,
            "new identity must mint a fresh UUID"
        );
        assert!(
            after.findings.is_empty(),
            "fresh identity must NOT carry forward the prior partial findings; got {:?}",
            after.findings
        );
        assert!(
            !after.partial,
            "fresh-identity scan covers the empty tree → complete"
        );
    }

    #[test]
    #[should_panic(expected = "scan budget must be at least 1")]
    fn zero_budget_panics_at_library_boundary() {
        // Council quick #C-2 (MAJOR) defence-in-depth: the CLI
        // surface rejects `--scan-budget 0` via clap's value parser,
        // but the library function asserts as well so a programmatic
        // caller bypassing the CLI cannot enter the infinite resume
        // loop.
        let tmp = TempDir::new().unwrap();
        let _ = scan_repo_for_findings_with_budget_v2(tmp.path(), 0, None);
    }

    #[test]
    fn parse_scan_budget_rejects_zero() {
        // Council quick #C-2 (MAJOR) CLI-side regression guard.
        let err = parse_scan_budget("0").unwrap_err();
        assert!(
            err.contains("never-converging"),
            "expected hint about resume loop; got {err}"
        );
        assert!(parse_scan_budget("1").is_ok());
        assert!(parse_scan_budget("50000").is_ok());
        assert!(
            parse_scan_budget("-1").is_err(),
            "negative parses as non-usize"
        );
    }

    #[test]
    fn refresh_to_partial_against_complete_prior_refuses_without_ack() {
        // Council quick #C-3 (MAJOR) regression guard: refreshing a
        // complete baseline with a budget below the file count would
        // silently overwrite the on-disk record with a partial
        // prefix-only view — a whitewash vector. Refuse without
        // `--accept-suspicious`.
        let tmp = TempDir::new().unwrap();
        seed_worktree_with_n_violation_files(tmp.path(), 10);
        // Round 1 (no budget cap): complete baseline.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        let prior = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(!prior.partial);
        let prior_findings = prior.findings.clone();

        // Round 2: --refresh --scan-budget=2 attempts to replace
        // the complete baseline with a 2-file prefix. Refused
        // without ack.
        run_create_or_refresh(
            tmp.path(),
            true, // refresh
            false,
            &SuspicionThresholds::default(),
            false, // no ack
            2,
        )
        .unwrap();
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(
            after.findings, prior_findings,
            "complete prior must not be replaced by partial without --accept-suspicious"
        );
        assert!(!after.partial, "the on-disk baseline must remain complete");
    }

    #[test]
    fn refresh_to_partial_against_complete_prior_proceeds_with_ack() {
        // Mirror of the test above: with `--accept-suspicious` the
        // operator has explicitly opted into incremental
        // re-adoption; the rewrite proceeds.
        let tmp = TempDir::new().unwrap();
        seed_worktree_with_n_violation_files(tmp.path(), 10);
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();

        run_create_or_refresh(
            tmp.path(),
            true,
            false,
            &SuspicionThresholds::default(),
            true, // ack
            2,
        )
        .unwrap();
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(after.partial, "ack lets the partial rewrite proceed");
        assert_eq!(after.findings.len(), 2);
    }

    #[test]
    fn suspicion_detection_skipped_while_baseline_is_partial() {
        // A partial baseline by definition has incomplete findings
        // — running analyze_refresh against it would generate
        // false-positive `degraded:baseline-suspicious` warnings on
        // every resume. The orchestrator must skip the detector
        // when either side is partial.
        let tmp = TempDir::new().unwrap();
        seed_baseline_with_n_findings(tmp.path(), 50);
        // Force the existing baseline into a partial state on disk.
        let mut baseline = load_baseline(tmp.path()).unwrap().unwrap();
        baseline.partial = true;
        baseline.continuation = Some("src/zzz_synthetic.ts".to_string());
        save_baseline(tmp.path(), &baseline).unwrap();

        // Now run with the suspicion detector that would fire on a
        // 50-to-0 drop. With partial=true on both sides (the
        // empty resume scan also has nothing to add) the detector
        // must NOT block. accept_suspicious=false confirms the
        // skip.
        run_create_or_refresh(
            tmp.path(),
            false,
            false,
            &SuspicionThresholds::default(),
            false,
            DEFAULT_SCAN_BUDGET,
        )
        .unwrap();
        // The baseline is now complete (no remaining files past
        // the synthetic cursor) and the prior synthetic findings
        // were merged forward. partial=false, no continuation.
        let after = load_baseline(tmp.path()).unwrap().unwrap();
        assert!(!after.partial);
        assert!(after.continuation.is_none());
    }
}
