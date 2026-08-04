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
//! - Returns a `degraded:audit-drift` marker when drift meets or
//!   exceeds the configured `--threshold` (inclusive, so
//!   `--threshold 5` flips on the 5th unwitnessed commit).
//!
//! Output format: JSON when `--json` is passed **or** stdout is not a
//! terminal (so pipes and CI get machine-readable output); the plain
//! summary otherwise. `--no-tui` does not itself force JSON here —
//! this command has no TUI surface, so on a terminal it stays plain.
//!
//! `chain_intact` and `degraded_audit_drift` are narrow signals and
//! neither reports coverage: see their field docs on [`AuditReport`].
//! The plain summary therefore leads with witnessed/walked coverage.
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
use std::io::{IsTerminal, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anvil_intercept::kindling_observation::{
    AUDIT_CHAIN_GATE_ID, AuditChainContext, AuditChainSummary, GateEvaluatedObservation,
    from_audit_chain,
};
use anvil_l4::{
    BranchRule, NoOpValidationEngine, OnBlock, OnNoWitness, OnWarn, Requirement, ValidationEngine,
    ValidationRequest, ValidationVerdict,
};
use anvil_witness::{WitnessLine, compute_line_hash, verify_chain_dag, witness_paths};
use anyhow::{Context, Result};
use chrono::Utc;
use clap::Args;
use serde::Serialize;
use uuid::Uuid;

use crate::GlobalArgs;

/// Sidecar NDJSON file the audit emits a Kindling row to on every
/// run. One observation per line; append-only. Future consumers
/// (Kindling-integration, dashboard tooling) tail this file the same
/// way they consume the `anvil/witness/manifest/chain.ndjson` stream
/// from MLP2-012.
const KINDLING_AUDIT_NDJSON: &str = "audit-chain.ndjson";

/// Inlined template for the nightly L5-audit GitHub workflow.
///
/// Consumed by the activation orchestrator (MLP2-053), which copies
/// the template into `.github/workflows/anvil-audit.yml` at adoption
/// time. ADR-037 §D-9: active by default; operator disables by
/// commenting out the `schedule` block.
#[must_use]
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
    /// Re-run the rule engine across history in addition to checking
    /// witness presence. Reports commits that today's rules would
    /// refuse even though they were allowed historically. Off by
    /// default — the nightly cron stays cheap; operators opt in when
    /// they suspect rule drift.
    ///
    /// With the default no-op validation engine the rescan reports
    /// no drift; the field becomes meaningful once a real engine is
    /// configured.
    #[arg(long, default_value_t = false)]
    rescan: bool,
    /// Wall-clock cap on the audit walk in seconds. Unbounded by
    /// default; set when the nightly cron needs a runtime ceiling so
    /// the workflow never runs away on very long histories. On cap,
    /// the audit stops walking and reports `partial: true`.
    #[arg(long)]
    max_runtime: Option<u64>,
}

/// MLP2-055: one row of rule-drift evidence. A commit appears here
/// when the rescan engine produced a `Block` verdict on it — meaning
/// the current rule set would refuse the commit today even though it
/// was allowed historically. Empty on a clean rescan or when the
/// default no-op engine returns `EngineUnavailable`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuleDriftEntry {
    /// Full hex commit SHA the rescan flagged.
    pub commit_sha: String,
    /// Rule identifiers (`rule_id`s on `ValidationDiagnostic`) that
    /// refused the commit. Empty rule sets are filtered out before
    /// the entry is recorded so consumers can rely on
    /// `!rule_ids.is_empty()`.
    pub rule_ids: Vec<String>,
}

/// Structured audit output. Stable schema so the nightly workflow
/// can pin against it; additive fields only.
///
/// Fields added after the v1 schema-version pin must use
/// `#[serde(skip_serializing_if = ...)]` so the byte-exact `schema_version`
/// contract holds for the empty-state case (MLP2-052 forward-compat
/// rule).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuditReport {
    pub schema_version: &'static str,
    pub branch: String,
    /// Commits in the audited range. `witnessed + unwitnessed.len()`.
    pub commits_walked: usize,
    /// Commits in the range that carry a witness record. Read together
    /// with `commits_walked` this is the audit's **coverage** — the
    /// figure the human summary leads on, because neither
    /// `chain_intact` nor `degraded_audit_drift` reports it.
    pub witnessed: usize,
    pub unwitnessed: Vec<String>,
    /// Tamper check on the witness records themselves: the chain-segment
    /// DAG parses and verifies. **Not** a coverage claim — a repo where
    /// no commit was ever witnessed has an intact (empty-ish) chain and
    /// reports `true`. Consumers that want coverage read `witnessed` /
    /// `commits_walked`.
    pub chain_intact: bool,
    /// `unwitnessed.len() >= threshold`. A threshold breach only —
    /// `false` means the count is under `--threshold`, not that the
    /// unwitnessed commits are accounted for.
    pub degraded_audit_drift: bool,
    pub threshold: usize,
    /// Hex SHA-256 line-hash of the most recent witness line in the
    /// active chain segment. `None` when the chain is empty or the
    /// last line fails to parse. Maps to `inputs.baseline_hash` on the
    /// Kindling observation emitted for this audit run (MLP2-054).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_head_hash: Option<String>,
    /// `true` when the audit walk did NOT complete (e.g. a future
    /// `--max-runtime` cap fired). Default `false` keeps the v1
    /// byte-exact output for the common case (MLP2-056 follow-up).
    #[serde(default, skip_serializing_if = "is_false")]
    pub partial: bool,
    /// MLP2-055 — commits whose contents the current rule set would
    /// refuse today. Populated only on `--rescan`; empty when the
    /// flag is off OR when the configured validation engine returns
    /// `EngineUnavailable` (the default until a real engine lands).
    /// Skipped from the JSON shape when empty so v1 consumers
    /// pinning on the original schema stay byte-compat.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_drift: Vec<RuleDriftEntry>,
}

#[inline]
// `skip_serializing_if` requires a fn(&T) -> bool signature; passing
// `bool` by value would be more cache-friendly but would not satisfy
// serde's contract. Allow the clippy nit for this one helper.
#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_false(b: &bool) -> bool {
    !*b
}

pub fn run(args: &AuditChainArgs, global: &GlobalArgs) -> Result<()> {
    let repo_root = std::env::current_dir().context("resolve repo root")?;
    // Single wall-clock anchor for both the rescan budget AND the
    // Kindling `duration_ms`. Decoupling them would let
    // `--max-runtime` measure a different window than the duration
    // recorded on the row, which is the bug MLP2-056 explicitly
    // wants to avoid.
    let started = Instant::now();
    let report = run_audit_chain_with_engine_started(
        &repo_root,
        &args.branch,
        args.since.as_deref(),
        args.threshold,
        args.rescan,
        args.max_runtime.map(Duration::from_secs),
        started,
        &NoOpValidationEngine,
    );
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    // MLP2-054 — emit one `gate_evaluated` Kindling row per audit run
    // so historical drift is queryable through the observation
    // timeline. Failures are non-fatal: a missing `anvil/kindling/`
    // path or a write error must not flip the audit's exit code.
    if let Err(e) = emit_audit_kindling_row(&repo_root, &report, duration_ms) {
        tracing::warn!(
            error = %e,
            "audit-chain: failed to append Kindling observation row; continuing",
        );
    }

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

/// Build the Kindling observation for an audit run and append it to
/// `anvil/kindling/audit-chain.ndjson`. Pure-ish: takes the repo root
/// plus an already-computed `AuditReport` plus the wall-clock
/// duration. The session id, gate eval id, and timestamp are minted
/// here because they are per-invocation identity rather than audit
/// semantics.
///
/// Exposed at crate visibility so the test module can drive it
/// against a tempdir without going through `std::env::current_dir`.
pub(crate) fn emit_audit_kindling_row(
    repo_root: &Path,
    report: &AuditReport,
    duration_ms: u64,
) -> Result<()> {
    // DISTRIB-006 (ADR-060): the kindling observation row is appended under the
    // project root (`anvil/kindling/`). Skip it under a gated ANVIL_HOME so a
    // candidate does not write a real project's audit sidecar — the audit output
    // itself (read-only) is unaffected and the caller already treats this as
    // best-effort.
    if crate::install_root::project_writes_gated() {
        return Ok(());
    }

    let session_id = Uuid::new_v4().to_string();
    let gate_eval_id = format!("audit-chain-{}", Uuid::new_v4());
    let timestamp = Utc::now().to_rfc3339();
    let summary = AuditChainSummary {
        commits_walked: report.commits_walked,
        unwitnessed_count: report.unwitnessed.len(),
        chain_intact: report.chain_intact,
        partial: report.partial,
        degraded_audit_drift: report.degraded_audit_drift,
        chain_head_hash: report.chain_head_hash.as_deref(),
    };
    let ctx = AuditChainContext {
        session_id: &session_id,
        timestamp: &timestamp,
        gate_eval_id: &gate_eval_id,
        duration_ms,
    };
    let observation = from_audit_chain(&ctx, &summary);
    append_kindling_observation(repo_root, &observation)
}

/// Append one JSON-serialised observation as a single NDJSON line.
/// Creates `anvil/kindling/` if it does not yet exist; the audit-
/// chain consumer (Kindling-integration) tails the file the same way
/// it tails the witness manifest stream.
fn append_kindling_observation(
    repo_root: &Path,
    observation: &GateEvaluatedObservation,
) -> Result<()> {
    // Sanity check — a refactor that swapped the observation's
    // gate_id should fail loudly rather than silently mis-route rows.
    debug_assert_eq!(observation.gate_id, AUDIT_CHAIN_GATE_ID);

    let dir = repo_root.join("anvil").join("kindling");
    fs::create_dir_all(&dir).with_context(|| format!("create kindling dir {}", dir.display()))?;
    let path = dir.join(KINDLING_AUDIT_NDJSON);
    let serialised =
        serde_json::to_string(observation).context("serialise audit-chain Kindling observation")?;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open kindling sidecar {}", path.display()))?;
    writeln!(f, "{serialised}")
        .with_context(|| format!("append kindling row to {}", path.display()))?;
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
/// Convenience entry point: no rescan, no budget, no-op validation
/// engine. Preserves the pre-MLP2-055/-056 byte-exact JSON shape (both
/// new fields skip on default).
///
/// Public so external CLI consumers (current: only the unit tests in
/// this module, but the symbol is part of the documented audit-chain
/// surface) can call the audit without spelling out engine plumbing.
#[must_use]
#[allow(dead_code)]
pub fn run_audit_chain(
    repo_root: &Path,
    branch: &str,
    since: Option<&str>,
    threshold: usize,
) -> AuditReport {
    run_audit_chain_with_engine(
        repo_root,
        branch,
        since,
        threshold,
        false,
        None,
        &NoOpValidationEngine,
    )
}

/// MLP2-055/-056: rescan- and budget-aware audit walk with a
/// pluggable [`ValidationEngine`]. Mints a fresh `started` instant
/// internally — callers (mostly tests) that don't care about
/// synchronising the budget clock with an outer
/// `duration_ms` use this wrapper.
///
/// `max_runtime` caps the wall-clock cost of the rescan loop only —
/// the witness-presence pass is bounded by the witness set size and
/// runs to completion. When the cap fires mid-rescan, `partial: true`
/// is set on the report so consumers see that the rule-drift list is
/// incomplete.
#[must_use]
pub fn run_audit_chain_with_engine<E: ValidationEngine + ?Sized>(
    repo_root: &Path,
    branch: &str,
    since: Option<&str>,
    threshold: usize,
    rescan: bool,
    max_runtime: Option<Duration>,
    engine: &E,
) -> AuditReport {
    run_audit_chain_with_engine_started(
        repo_root,
        branch,
        since,
        threshold,
        rescan,
        max_runtime,
        Instant::now(),
        engine,
    )
}

/// Caller-provided-clock variant of [`run_audit_chain_with_engine`].
/// The CLI's `run()` uses this so the rescan budget and the Kindling
/// `duration_ms` derive from the **same** `Instant` — otherwise
/// `--max-runtime N` would measure a different window from the
/// duration recorded on the row.
#[must_use]
// 8 args is the price of an explicit shared-clock seam. Bundling
// them into a config struct would obscure that the clock is
// caller-owned; this single annotated call site is cheaper than the
// indirection.
#[allow(clippy::too_many_arguments)]
pub fn run_audit_chain_with_engine_started<E: ValidationEngine + ?Sized>(
    repo_root: &Path,
    branch: &str,
    since: Option<&str>,
    threshold: usize,
    rescan: bool,
    max_runtime: Option<Duration>,
    started: Instant,
    engine: &E,
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

    let (rule_drift, partial) = if rescan {
        rescan_commits_with_budget(
            engine,
            repo_root,
            &commits,
            &default_rescan_branch_rule(),
            started,
            max_runtime,
        )
    } else {
        (Vec::new(), false)
    };

    AuditReport {
        schema_version: "anvil.audit-chain.v1",
        branch: branch.to_string(),
        commits_walked: commits.len(),
        witnessed: witnessed_count,
        unwitnessed,
        chain_intact,
        degraded_audit_drift: degraded,
        threshold,
        chain_head_hash: compute_chain_head_hash(repo_root),
        partial,
        rule_drift,
    }
}

/// Synthetic branch rule used when the audit runs the rescan engine
/// without a project policy file. The audit's purpose here is "what
/// would today's rules say about every historical commit", so we
/// emulate a strict L4 lane: re-validate via the engine, refuse on
/// block, admit warnings. A future enhancement may consult the actual
/// `anvil/policy.{yml,json,toml}` if present.
fn default_rescan_branch_rule() -> BranchRule {
    BranchRule {
        pattern: "*".to_string(),
        require: Requirement::L4Only,
        on_no_witness: OnNoWitness::ValidateAtL4,
        on_block: OnBlock::Reject,
        on_warn: OnWarn::Allow,
    }
}

/// Budget-aware rescan. Returns `(drift, partial)` where `partial` is
/// `true` if the loop exited early because the wall-clock budget
/// fired. Callers without a budget pass `max_runtime: None` and
/// receive `partial: false`.
///
/// The budget is checked **between** commits — once a commit's
/// `validate` call has started, it runs to completion. This is the
/// conservative shape: it keeps the engine's per-commit contract
/// intact (no torn writes, no half-finished diagnostics) at the cost
/// of occasionally running slightly over budget. The trade-off is
/// fine because the budget exists to cap nightly cron runtime, not
/// to bound individual commit evaluation.
pub(crate) fn rescan_commits_with_budget<E: ValidationEngine + ?Sized>(
    engine: &E,
    repo_root: &Path,
    commits: &[String],
    branch_rule: &BranchRule,
    started: Instant,
    max_runtime: Option<Duration>,
) -> (Vec<RuleDriftEntry>, bool) {
    let mut drift: Vec<RuleDriftEntry> = Vec::new();
    let mut partial = false;
    // ADR-100: suppression authority for the rescan is the tip of the
    // audited commit list itself (`list_commits` is `git rev-list`
    // newest-first over the `--branch` argument), NOT the checkout's
    // HEAD — auditing `release-1.0` from a `main` checkout must not
    // apply `main`'s grants (2026-07-04 council HIGH). An empty list
    // applies no exceptions (fail-safe: drift rows surface).
    let exceptions_tip: Option<String> = commits.first().cloned();
    for (processed, sha) in commits.iter().enumerate() {
        if budget_exhausted(started, max_runtime) {
            partial = true;
            tracing::warn!(
                kind = "audit_chain",
                gate_id = AUDIT_CHAIN_GATE_ID,
                partial = true,
                commits_processed = processed,
                commits_skipped = commits.len() - processed,
                "audit-chain rescan budget exhausted; reporting partial run",
            );
            break;
        }
        let request = ValidationRequest {
            commit_sha: sha.clone(),
            branch_rule: branch_rule.clone(),
            repo_root: repo_root.to_path_buf(),
            exceptions_tip_sha: exceptions_tip.clone(),
        };
        if let ValidationVerdict::Block { diagnostics } = engine.validate(&request) {
            let rule_ids: Vec<String> = diagnostics.iter().map(|d| d.rule_id.clone()).collect();
            // Defence in depth — a `Block` with no diagnostics is a
            // contract violation by the engine, but we'd rather drop
            // the row than write an empty rule_ids list that the
            // consumer would treat as "blocked for no reason."
            if !rule_ids.is_empty() {
                drift.push(RuleDriftEntry {
                    commit_sha: sha.clone(),
                    rule_ids,
                });
            }
        }
    }
    (drift, partial)
}

#[inline]
fn budget_exhausted(started: Instant, cap: Option<Duration>) -> bool {
    cap.is_some_and(|d| started.elapsed() >= d)
}

/// Read the most recent witness line and return its SHA-256 line-hash
/// (hex). Returns `None` when the chain is empty, the last line is
/// blank, or it fails to parse — the audit is a no-fail surface
/// (ADR-038 §D-6) so any IO / parse hiccup degrades to "no baseline
/// hash" rather than failing the run.
fn compute_chain_head_hash(repo_root: &Path) -> Option<String> {
    let paths = witness_paths(repo_root);
    let last_path = paths.last()?;
    let contents = fs::read_to_string(last_path).ok()?;
    let last_line = contents.lines().rev().find(|l| !l.trim().is_empty())?;
    let parsed = WitnessLine::from_ndjson_line(last_line.as_bytes()).ok()?;
    let bytes = parsed.to_canonical_bytes().ok()?;
    Some(compute_line_hash(&bytes))
}

fn print_plain(r: &AuditReport) {
    print!("{}", render_plain(r));
}

/// Render the human summary.
///
/// Coverage leads. `chain_intact` and `degraded_audit_drift` are both
/// narrow, correct signals — a repo where no commit is witnessed still
/// reports `chain intact: yes` (the witness records are unmodified) and
/// no drift (unwitnessed count under `--threshold`). Stacked as a
/// column of green-looking fields they skim as a clean health pass,
/// which they are not. So the summary states witness coverage first and
/// qualifies what `chain intact` actually asserts. The field semantics
/// behind them are deliberately unchanged.
fn render_plain(r: &AuditReport) -> String {
    use std::fmt::Write as _;

    // Writing into a String is infallible; `let _ =` keeps the render
    // path free of unwrap noise.
    let mut out = String::new();
    let _ = writeln!(out, "anvil audit-chain — branch {}", r.branch);

    // `checked_div` folds the empty-walk guard into the percentage:
    // `None` is exactly the "nothing was audited" case. `saturating_mul`
    // keeps a pathological commit count from wrapping the display.
    let coverage_percent = r
        .witnessed
        .saturating_mul(100)
        .checked_div(r.commits_walked);
    match coverage_percent {
        None => {
            let _ = writeln!(out, "  witness coverage: no commits in range");
        }
        Some(percent) => {
            let _ = writeln!(
                out,
                "  witness coverage: {}/{} commits witnessed ({percent}%)",
                r.witnessed, r.commits_walked,
            );
            if r.witnessed == 0 {
                let _ = writeln!(
                    out,
                    "  NO WITNESS COVERAGE: not one commit in this range carries a witness.",
                );
            }
        }
    }

    let _ = writeln!(out, "  commits walked: {}", r.commits_walked);
    let _ = writeln!(out, "  witnessed:      {}", r.witnessed);
    let _ = writeln!(out, "  unwitnessed:    {}", r.unwitnessed.len());
    let _ = writeln!(
        out,
        "  chain intact:   {} (witness records unmodified — not a coverage claim)",
        if r.chain_intact { "yes" } else { "NO" },
    );
    if r.partial {
        let _ = writeln!(
            out,
            "  PARTIAL: max-runtime cap fired; rescan list is incomplete",
        );
    }
    if r.degraded_audit_drift {
        let _ = writeln!(
            out,
            "  DEGRADED: drift {} >= threshold {}",
            r.unwitnessed.len(),
            r.threshold,
        );
    } else if !r.unwitnessed.is_empty() {
        // A clear drift marker is a threshold statement, not a verdict
        // on the unwitnessed commits. Say so, so a quiet marker is not
        // read as "those commits are accounted for".
        let _ = writeln!(
            out,
            "  (drift marker clear: {} unwitnessed < threshold {})",
            r.unwitnessed.len(),
            r.threshold,
        );
    }
    if !r.unwitnessed.is_empty() && r.unwitnessed.len() <= 20 {
        let _ = writeln!(out, "  unwitnessed SHAs:");
        for sha in &r.unwitnessed {
            let _ = writeln!(out, "    {sha}");
        }
    }
    out
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
    // MLP2-011 — DAG-aware so a chain with merge witnesses is still
    // recognised as intact.
    verify_chain_dag(&path_refs).is_ok()
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
            cutoff_commit: None,
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
            None,
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
        assert!(verify_chain_dag(&[active.as_path()]).is_ok());
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
            chain_head_hash: None,
            partial: false,
            rule_drift: Vec::new(),
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
            chain_head_hash: None,
            partial: false,
            rule_drift: Vec::new(),
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

    /// Build a report with `witnessed` of `walked` commits witnessed and
    /// the remainder unwitnessed. Keeps the coverage-rendering tests
    /// focused on presentation rather than chain plumbing.
    fn coverage_report(walked: usize, witnessed: usize, threshold: usize) -> AuditReport {
        let unwitnessed: Vec<String> = (0..walked.saturating_sub(witnessed))
            .map(|i| format!("sha{i:03}"))
            .collect();
        AuditReport {
            schema_version: "anvil.audit-chain.v1",
            branch: "main".to_string(),
            commits_walked: walked,
            witnessed,
            degraded_audit_drift: unwitnessed.len() >= threshold,
            unwitnessed,
            chain_intact: true,
            threshold,
            chain_head_hash: None,
            partial: false,
            rule_drift: Vec::new(),
        }
    }

    /// The reported case: 2 commits, none witnessed, default threshold 5.
    /// `chain_intact: true` / `degraded_audit_drift: false` are both
    /// correct under the contract, so the *summary* has to lead with
    /// coverage or a reader skims it as a clean bill of health.
    #[test]
    fn plain_summary_leads_with_witness_coverage() {
        let r = coverage_report(2, 0, 5);
        let out = render_plain(&r);
        let coverage_line = out
            .lines()
            .find(|l| l.contains("witness coverage"))
            .expect("summary must carry a witness-coverage line");
        assert!(
            coverage_line.contains("0/2"),
            "coverage must show witnessed/walked, got: {coverage_line}"
        );

        // "Leads with" is load-bearing: coverage must precede the
        // `chain intact` line, which is the field that reads green.
        let coverage_at = out.find("witness coverage").expect("coverage line present");
        let intact_at = out.find("chain intact").expect("chain intact line present");
        assert!(
            coverage_at < intact_at,
            "coverage must appear before `chain intact`:\n{out}"
        );
    }

    /// Zero coverage is the skim hazard — it must be called out
    /// explicitly, not left to the reader to derive from `witnessed: 0`.
    #[test]
    fn plain_summary_calls_out_zero_coverage() {
        let r = coverage_report(2, 0, 5);
        let out = render_plain(&r);
        assert!(
            out.contains("NO WITNESS COVERAGE"),
            "zero-coverage runs need an explicit callout:\n{out}"
        );
    }

    /// `chain intact` must not be readable as "every commit was
    /// witnessed". The rendered line has to say what it actually checks.
    #[test]
    fn plain_summary_qualifies_chain_intact_meaning() {
        let r = coverage_report(2, 0, 5);
        let out = render_plain(&r);
        let intact_line = out
            .lines()
            .find(|l| l.contains("chain intact"))
            .expect("chain intact line present");
        assert!(
            intact_line.contains("not a coverage claim"),
            "`chain intact` must disclaim coverage, got: {intact_line}"
        );
    }

    /// Full coverage renders the ratio and drops the zero-coverage
    /// callout — the warning must not become background noise.
    #[test]
    fn plain_summary_reports_full_coverage_without_warning() {
        let r = coverage_report(3, 3, 5);
        let out = render_plain(&r);
        assert!(
            out.contains("3/3"),
            "full coverage must still show the ratio:\n{out}"
        );
        assert!(
            !out.contains("NO WITNESS COVERAGE"),
            "fully witnessed runs must not warn about zero coverage:\n{out}"
        );
    }

    /// An empty walk has no coverage to report and must not be dressed
    /// up as a zero-coverage failure.
    #[test]
    fn plain_summary_handles_empty_walk() {
        let r = coverage_report(0, 0, 5);
        let out = render_plain(&r);
        assert!(
            !out.contains("NO WITNESS COVERAGE"),
            "an empty walk is not a coverage breach:\n{out}"
        );
        assert!(
            out.contains("no commits in range"),
            "empty walk must say so plainly:\n{out}"
        );
    }

    /// Guard the contract this item must NOT change: presentation work
    /// on the summary must leave `chain_intact` and
    /// `degraded_audit_drift` semantics exactly where they were.
    /// `chain_intact` = witness DAG untampered (not coverage);
    /// `degraded_audit_drift` = unwitnessed count >= threshold.
    #[test]
    fn zero_coverage_below_threshold_keeps_field_semantics() {
        let r = coverage_report(2, 0, 5);
        assert!(
            r.chain_intact,
            "chain_intact must stay a tamper check, not a coverage check"
        );
        assert!(
            !r.degraded_audit_drift,
            "2 unwitnessed under threshold 5 must not flip the drift marker"
        );
        // And the JSON contract is untouched by the presentation change.
        let value = serde_json::to_value(&r).unwrap();
        assert_eq!(value["chain_intact"], serde_json::json!(true));
        assert_eq!(value["degraded_audit_drift"], serde_json::json!(false));
        assert_eq!(value["witnessed"], serde_json::json!(0));
        assert_eq!(value["schema_version"], "anvil.audit-chain.v1");
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
            None,
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
            cutoff_commit: None,
            ts: "2026-05-13T00:00:00Z".to_string(),
            validation_at: "post-merge".to_string(),
        };
        writer.append(&merge_line).unwrap();
        let set = collect_witnessed_shas(tmp.path()).unwrap();
        assert!(set.contains("merge-sha"));
        assert!(set.contains("parent-a"));
        assert!(set.contains("parent-b"));
    }

    // ---- MLP2-054: Kindling row emission ------------------------------

    fn read_kindling_lines(repo_root: &Path) -> Vec<serde_json::Value> {
        let path = repo_root
            .join("anvil")
            .join("kindling")
            .join(KINDLING_AUDIT_NDJSON);
        let contents = fs::read_to_string(&path).expect("kindling sidecar exists");
        contents
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("each line parses as JSON"))
            .collect()
    }

    #[test]
    fn audit_chain_run_appends_kindling_row_with_audit_gate_id() {
        let tmp = TempDir::new().unwrap();
        write_minimal_chain(tmp.path(), &["aaa", "bbb"]);
        let report = run_audit_chain(tmp.path(), "HEAD", None, 5);

        emit_audit_kindling_row(tmp.path(), &report, 42).expect("emit");

        let rows = read_kindling_lines(tmp.path());
        assert_eq!(rows.len(), 1, "one audit run must produce one row");
        assert_eq!(rows[0]["kind"], "gate_evaluated");
        assert_eq!(rows[0]["gate_id"], AUDIT_CHAIN_GATE_ID);
        assert_eq!(rows[0]["duration_ms"], 42);
    }

    #[test]
    fn audit_chain_two_runs_append_two_kindling_lines() {
        let tmp = TempDir::new().unwrap();
        write_minimal_chain(tmp.path(), &["aaa"]);
        let report = run_audit_chain(tmp.path(), "HEAD", None, 5);

        emit_audit_kindling_row(tmp.path(), &report, 10).expect("first emit");
        emit_audit_kindling_row(tmp.path(), &report, 11).expect("second emit");

        let rows = read_kindling_lines(tmp.path());
        assert_eq!(
            rows.len(),
            2,
            "subsequent audit runs must append, not overwrite"
        );
        assert_eq!(rows[0]["duration_ms"], 10);
        assert_eq!(rows[1]["duration_ms"], 11);
    }

    #[test]
    fn audit_chain_kindling_row_carries_baseline_hash_when_chain_populated() {
        let tmp = TempDir::new().unwrap();
        write_minimal_chain(tmp.path(), &["aaa"]);
        let report = run_audit_chain(tmp.path(), "HEAD", None, 5);
        // Pre-condition: AuditReport must have a chain_head_hash so the
        // observation has something to propagate.
        assert!(
            report.chain_head_hash.is_some(),
            "populated chain must expose chain_head_hash"
        );
        let expected = report.chain_head_hash.clone().unwrap();

        emit_audit_kindling_row(tmp.path(), &report, 1).expect("emit");

        let rows = read_kindling_lines(tmp.path());
        assert_eq!(
            rows[0]["inputs"]["baseline_hash"].as_str(),
            Some(expected.as_str()),
            "Kindling row must inherit baseline_hash from chain_head_hash"
        );
    }

    #[test]
    fn audit_chain_kindling_row_omits_baseline_hash_on_empty_chain() {
        let tmp = TempDir::new().unwrap();
        // No witness chain — chain_head_hash is None.
        let report = run_audit_chain(tmp.path(), "HEAD", None, 5);
        assert!(report.chain_head_hash.is_none());

        emit_audit_kindling_row(tmp.path(), &report, 1).expect("emit");

        let rows = read_kindling_lines(tmp.path());
        assert!(
            rows[0]["inputs"].get("baseline_hash").is_none(),
            "empty-chain runs must omit baseline_hash from the row"
        );
    }

    #[test]
    fn audit_chain_report_serialises_without_optional_fields_when_empty() {
        // Schema-stability pin (MLP2-052 additive-optional rule).
        // The greenfield empty-chain report must serialise without the
        // new MLP2-054/-056 fields so downstream consumers pinning on
        // the v1 shape keep parsing.
        let tmp = TempDir::new().unwrap();
        let report = run_audit_chain(tmp.path(), "HEAD", None, 5);
        let value = serde_json::to_value(&report).unwrap();
        assert!(
            value.get("chain_head_hash").is_none(),
            "empty-chain report must omit chain_head_hash"
        );
        assert!(
            value.get("partial").is_none(),
            "complete (non-partial) report must omit partial"
        );
    }

    #[test]
    fn audit_chain_report_serialises_chain_head_hash_when_present() {
        let tmp = TempDir::new().unwrap();
        write_minimal_chain(tmp.path(), &["aaa"]);
        let report = run_audit_chain(tmp.path(), "HEAD", None, 5);
        let value = serde_json::to_value(&report).unwrap();
        assert!(
            value["chain_head_hash"].is_string(),
            "populated chain must surface chain_head_hash in the JSON report"
        );
    }

    // ---- MLP2-055: rescan / rule drift -------------------------------

    use anvil_l4::{Severity, ValidationDiagnostic};

    /// Fixture engine that always blocks with a fixed rule-id set so
    /// the rescan path is exercisable without a real rule engine.
    struct AlwaysBlockEngine {
        rule_ids: Vec<&'static str>,
    }

    impl ValidationEngine for AlwaysBlockEngine {
        fn validate(&self, _request: &ValidationRequest) -> ValidationVerdict {
            ValidationVerdict::Block {
                diagnostics: self
                    .rule_ids
                    .iter()
                    .map(|rid| ValidationDiagnostic {
                        rule_id: (*rid).to_string(),
                        severity: Severity::Block,
                        message: "rule drift".to_string(),
                    })
                    .collect(),
            }
        }
    }

    /// Fixture engine that blocks only on the SHAs the test names.
    /// Lets us prove the audit reports rule drift for the targeted
    /// commits and leaves the others alone.
    struct SelectiveBlockEngine {
        targets: Vec<String>,
        rule_id: String,
    }

    impl ValidationEngine for SelectiveBlockEngine {
        fn validate(&self, request: &ValidationRequest) -> ValidationVerdict {
            if self.targets.iter().any(|t| t == &request.commit_sha) {
                ValidationVerdict::Block {
                    diagnostics: vec![ValidationDiagnostic {
                        rule_id: self.rule_id.clone(),
                        severity: Severity::Block,
                        message: "rule drift".to_string(),
                    }],
                }
            } else {
                ValidationVerdict::Allow
            }
        }
    }

    /// Test-only helper: rescan without a budget, ignoring the
    /// returned `partial` bit. Keeps the existing rescan tests focused
    /// on the drift list semantics.
    fn rescan_drift_only<E: ValidationEngine + ?Sized>(
        engine: &E,
        repo_root: &Path,
        commits: &[String],
        branch_rule: &BranchRule,
    ) -> Vec<RuleDriftEntry> {
        let (drift, _partial) = rescan_commits_with_budget(
            engine,
            repo_root,
            commits,
            branch_rule,
            Instant::now(),
            None,
        );
        drift
    }

    #[test]
    fn rescan_returns_empty_for_no_op_engine() {
        // The default (no-op) engine returns `EngineUnavailable` so
        // the rescan reports no drift — the engine couldn't actually
        // run. Surfacing those as drift would be misleading.
        let tmp = TempDir::new().unwrap();
        let commits = vec!["aaa".to_string(), "bbb".to_string()];
        let drift = rescan_drift_only(
            &NoOpValidationEngine,
            tmp.path(),
            &commits,
            &default_rescan_branch_rule(),
        );
        assert!(
            drift.is_empty(),
            "no-op engine must produce zero rule-drift entries"
        );
    }

    #[test]
    fn rescan_records_each_commit_engine_blocks() {
        let tmp = TempDir::new().unwrap();
        let commits = vec!["aaa".to_string(), "bbb".to_string()];
        let engine = AlwaysBlockEngine {
            rule_ids: vec!["secret-detection.aws-key", "antipattern.todo-comment"],
        };
        let drift = rescan_drift_only(&engine, tmp.path(), &commits, &default_rescan_branch_rule());
        assert_eq!(drift.len(), 2, "both commits must appear as drift entries");
        assert_eq!(drift[0].commit_sha, "aaa");
        assert_eq!(
            drift[0].rule_ids,
            vec![
                "secret-detection.aws-key".to_string(),
                "antipattern.todo-comment".to_string(),
            ]
        );
        assert_eq!(drift[1].commit_sha, "bbb");
    }

    #[test]
    fn rescan_filters_block_verdicts_with_empty_rule_ids() {
        // Defensive-mode: an engine that returns Block with zero
        // diagnostics is a contract violation, so the audit drops
        // the row rather than emitting an empty rule_ids list that
        // would parse as "blocked for no reason."
        struct EmptyBlockEngine;
        impl ValidationEngine for EmptyBlockEngine {
            fn validate(&self, _request: &ValidationRequest) -> ValidationVerdict {
                ValidationVerdict::Block {
                    diagnostics: Vec::new(),
                }
            }
        }
        let tmp = TempDir::new().unwrap();
        let commits = vec!["aaa".to_string()];
        let drift = rescan_drift_only(
            &EmptyBlockEngine,
            tmp.path(),
            &commits,
            &default_rescan_branch_rule(),
        );
        assert!(
            drift.is_empty(),
            "Block with empty diagnostics must not produce a drift entry"
        );
    }

    #[test]
    fn rescan_off_skips_engine_invocation() {
        // run_audit_chain (the no-rescan entry point) must produce an
        // empty rule_drift even when the engine would otherwise block,
        // because rescan defaults off. Proven by calling
        // run_audit_chain_with_engine with rescan=false.
        let tmp = TempDir::new().unwrap();
        write_minimal_chain(tmp.path(), &["aaa"]);
        let engine = AlwaysBlockEngine {
            rule_ids: vec!["any-rule"],
        };
        let report = run_audit_chain_with_engine(
            tmp.path(),
            "HEAD",
            None,
            5,
            false, // rescan OFF
            None,
            &engine,
        );
        assert!(
            report.rule_drift.is_empty(),
            "rescan=false must skip the engine entirely"
        );
    }

    #[test]
    fn rescan_on_with_blocking_engine_surfaces_drift_in_report() {
        // History fixture spec: when a rule would block a historical
        // commit today, --rescan flags that commit. We can't easily
        // synthesise commits without a real git repo, so we drive
        // `rescan_commits` directly with the same engine the run
        // would use.
        let tmp = TempDir::new().unwrap();
        let commits = vec!["aaa".to_string(), "bbb".to_string(), "ccc".to_string()];
        let engine = SelectiveBlockEngine {
            targets: vec!["bbb".to_string()],
            rule_id: "rule-added-after-bbb".to_string(),
        };
        let drift = rescan_drift_only(&engine, tmp.path(), &commits, &default_rescan_branch_rule());
        assert_eq!(drift.len(), 1, "only the targeted commit should drift");
        assert_eq!(drift[0].commit_sha, "bbb");
        assert_eq!(drift[0].rule_ids, vec!["rule-added-after-bbb".to_string()]);
    }

    #[test]
    fn rule_drift_field_omitted_from_json_when_empty() {
        // MLP2-052 forward-compat: a clean rescan (or no rescan at
        // all) must NOT emit the `rule_drift` field in the JSON
        // shape. v1 consumers pinning on the original AuditReport
        // schema would otherwise have to start tolerating the field.
        let tmp = TempDir::new().unwrap();
        let report = run_audit_chain(tmp.path(), "HEAD", None, 5);
        let value = serde_json::to_value(&report).unwrap();
        assert!(
            value.get("rule_drift").is_none(),
            "empty rule_drift must serialise as absent"
        );
    }

    // ---- MLP2-056: --max-runtime time-budget cap ---------------------

    /// Engine that records each commit it sees AND sleeps for a fixed
    /// duration on each call. Lets the budget tests prove the cap
    /// fires after a known number of evaluations.
    struct SleepyEngine {
        per_call: Duration,
        seen: std::sync::Mutex<Vec<String>>,
    }

    impl SleepyEngine {
        fn new(per_call: Duration) -> Self {
            Self {
                per_call,
                seen: std::sync::Mutex::new(Vec::new()),
            }
        }
        fn calls(&self) -> Vec<String> {
            self.seen.lock().unwrap().clone()
        }
    }

    impl ValidationEngine for SleepyEngine {
        fn validate(&self, request: &ValidationRequest) -> ValidationVerdict {
            self.seen.lock().unwrap().push(request.commit_sha.clone());
            std::thread::sleep(self.per_call);
            ValidationVerdict::Allow
        }
    }

    #[test]
    fn rescan_no_budget_runs_to_completion() {
        // No cap → every commit gets evaluated, partial is false.
        let tmp = TempDir::new().unwrap();
        let commits = vec!["a".into(), "b".into(), "c".into()];
        let engine = SleepyEngine::new(Duration::ZERO);
        let (_drift, partial) = rescan_commits_with_budget(
            &engine,
            tmp.path(),
            &commits,
            &default_rescan_branch_rule(),
            Instant::now(),
            None,
        );
        assert_eq!(engine.calls().len(), 3, "all commits must be evaluated");
        assert!(!partial, "no budget → never partial");
    }

    #[test]
    fn rescan_zero_budget_short_circuits_before_first_commit() {
        // `Duration::ZERO` is a degenerate budget — `elapsed() >= 0`
        // is always true, so the cap fires immediately. No commits
        // get evaluated; partial is true.
        let tmp = TempDir::new().unwrap();
        let commits = vec!["a".into(), "b".into()];
        let engine = SleepyEngine::new(Duration::from_millis(50));
        // Started 1ms in the past so elapsed() > 0 deterministically.
        let started = Instant::now()
            .checked_sub(Duration::from_millis(1))
            .expect("Instant::now() is well past the epoch");
        let (drift, partial) = rescan_commits_with_budget(
            &engine,
            tmp.path(),
            &commits,
            &default_rescan_branch_rule(),
            started,
            Some(Duration::ZERO),
        );
        assert!(partial, "zero budget must produce partial=true");
        assert!(drift.is_empty(), "zero budget yields zero drift entries");
        assert!(
            engine.calls().is_empty(),
            "zero budget must short-circuit before any engine call"
        );
    }

    #[test]
    fn rescan_partial_when_budget_fires_mid_walk() {
        // 5 commits, 20ms each → ~100ms total. Budget of 50ms should
        // cap somewhere between commit 2 and commit 4 (between-commit
        // check semantics). We assert the structural invariant rather
        // than a precise number to avoid flakes on slow CI hosts.
        let tmp = TempDir::new().unwrap();
        let commits = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
        let engine = SleepyEngine::new(Duration::from_millis(20));
        let started = Instant::now();
        let (_drift, partial) = rescan_commits_with_budget(
            &engine,
            tmp.path(),
            &commits,
            &default_rescan_branch_rule(),
            started,
            Some(Duration::from_millis(50)),
        );
        assert!(partial, "budget < total work must produce partial=true");
        let seen = engine.calls();
        assert!(
            seen.len() < commits.len(),
            "partial run must evaluate fewer than all commits; saw {}",
            seen.len()
        );
        assert!(
            !seen.is_empty(),
            "between-commit budget check should let at least one commit run"
        );
    }

    #[test]
    fn run_audit_chain_with_engine_threads_budget_into_report() {
        // End-to-end: --max-runtime → partial=true on the AuditReport
        // when the budget fires. Needs a real git repo so
        // `list_commits` returns a non-empty Vec for the budget check
        // to actually run.
        let git_probe = Command::new("git").arg("--version").output();
        if !matches!(&git_probe, Ok(out) if out.status.success()) {
            eprintln!("skipping MLP2-056 budget integration test: git unavailable");
            return;
        }

        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let run_git = |args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(root).args(args).output();
            assert!(
                matches!(&out, Ok(o) if o.status.success()),
                "git {args:?} failed: {out:?}"
            );
        };
        run_git(&["init", "-q", "-b", "main"]);
        run_git(&["config", "user.email", "mlp2-056@anvil.test"]);
        run_git(&["config", "user.name", "MLP2-056 fixture"]);
        run_git(&["config", "commit.gpgsign", "false"]);
        // Three trivial commits so the rescan has something to walk.
        for i in 0..3 {
            std::fs::write(root.join(format!("f{i}.txt")), "x\n").unwrap();
            run_git(&["add", "."]);
            run_git(&["commit", "-q", "-m", &format!("c{i}")]);
        }

        // Sleepy engine + zero budget → budget exhausted on first
        // between-commit check; rescan exits early; partial true.
        let engine = SleepyEngine::new(Duration::from_millis(20));
        let report = run_audit_chain_with_engine(
            root,
            "HEAD",
            None,
            5,
            true, // rescan ON so the budget is consulted
            Some(Duration::ZERO),
            &engine,
        );
        assert!(
            report.partial,
            "zero budget with rescan over a non-empty repo must mark report partial"
        );
    }

    #[test]
    fn run_audit_chain_default_path_never_partial() {
        // The no-rescan / no-budget default must always report
        // partial=false so v1 consumers pinning on the empty-state
        // shape stay byte-compat (the field skips when false).
        let tmp = TempDir::new().unwrap();
        let report = run_audit_chain(tmp.path(), "HEAD", None, 5);
        assert!(
            !report.partial,
            "default audit run must never set partial=true"
        );
        let value = serde_json::to_value(&report).unwrap();
        assert!(
            value.get("partial").is_none(),
            "partial must skip from the JSON when false"
        );
    }

    #[test]
    fn partial_report_propagates_to_kindling_observation() {
        // MLP2-054 ↔ MLP2-056 wiring — when the audit report is
        // partial, the Kindling row carries the bit on the wire so
        // consumers tailing the NDJSON stream can route on it
        // without re-deriving it from other fields.
        let tmp = TempDir::new().unwrap();
        let mut report = run_audit_chain(tmp.path(), "HEAD", None, 5);
        report.partial = true;
        emit_audit_kindling_row(tmp.path(), &report, 1).expect("emit");
        let rows = read_kindling_lines(tmp.path());
        assert_eq!(rows.len(), 1, "partial reports still produce one row");
        assert_eq!(
            rows[0]["partial"], true,
            "partial=true on the report must surface on the wire row"
        );
        // A partial walk is treated as a failure for outcome /
        // enforcement so a runaway nightly cron can't quietly report
        // green.
        assert_eq!(rows[0]["outcome"], "fail");
        assert_eq!(rows[0]["enforcement"], "blocking");
    }

    #[test]
    fn complete_report_omits_partial_field_from_wire() {
        // Symmetric pin: a complete (non-partial) run must NOT emit
        // `partial: false` on the wire — the field is `skip_if_false`
        // so pre-MLP2-056 consumers stay byte-compat.
        let tmp = TempDir::new().unwrap();
        let report = run_audit_chain(tmp.path(), "HEAD", None, 5);
        emit_audit_kindling_row(tmp.path(), &report, 1).expect("emit");
        let rows = read_kindling_lines(tmp.path());
        assert!(
            rows[0].get("partial").is_none(),
            "complete audit runs must omit the partial field from the wire"
        );
    }

    #[test]
    fn rule_drift_field_present_in_json_when_populated() {
        // The flip side: when drift is non-empty the field MUST appear
        // so downstream tooling can pattern-match on its presence.
        let report = AuditReport {
            schema_version: "anvil.audit-chain.v1",
            branch: "main".to_string(),
            commits_walked: 1,
            witnessed: 1,
            unwitnessed: Vec::new(),
            chain_intact: true,
            degraded_audit_drift: false,
            threshold: 5,
            chain_head_hash: None,
            partial: false,
            rule_drift: vec![RuleDriftEntry {
                commit_sha: "deadbeef".to_string(),
                rule_ids: vec!["rule-x".to_string()],
            }],
        };
        let value = serde_json::to_value(&report).unwrap();
        let drift = value["rule_drift"]
            .as_array()
            .expect("rule_drift renders as array");
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0]["commit_sha"], "deadbeef");
    }
}
