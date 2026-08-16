//! `anvil graph-base` — hidden subprocess harness for the shared base-graph
//! producer.
//!
//! Not a user-facing command: it is the subprocess entry point the save-time
//! daemon detaches to when producing a shared base snapshot. It resolves the
//! merge-base commit, then (GBASE-002) single-flight **claims**, builds the base
//! graph from the committed tree (never a working tree), serialises it to an
//! `ANVILGB1` content-addressed artefact, **write-once publishes** it under the
//! graph-cache `base/` dir, and releases the claim — printing a deterministic,
//! **path-free** one-line JSON summary to stdout. A claim contention is a clean,
//! non-fatal exit with a distinct `"claimed-elsewhere"` outcome (ADR-105 §6).
//!
//! Hidden from `--help` via `hide = true` on the top-level variant.

use clap::{Args, Subcommand};

use crate::GlobalArgs;

/// Top-level args for the hidden `graph-base` command group.
#[derive(Debug, Args)]
pub struct GraphBaseArgs {
    #[command(subcommand)]
    pub command: GraphBaseCommand,
}

#[derive(Debug, Subcommand)]
pub enum GraphBaseCommand {
    /// Build the base graph of a merge-base commit's committed tree and print a
    /// deterministic JSON summary to stdout.
    Build(BuildArgs),

    /// Run one on-demand shared-base GC pass, or (with `--purge-all`) empty the
    /// base store. The operator disk-pressure remediation for
    /// `<graph-cache>/base`.
    Gc(GcArgs),
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Explicit merge-base commit sha to build from. When omitted, the base is
    /// resolved from the repository's default branch.
    #[arg(long)]
    pub merge_base: Option<String>,

    /// Fallback default-branch ref used only when the repository has no
    /// `origin/HEAD` symref (e.g. a local-only repo).
    #[arg(long)]
    pub default_branch: Option<String>,

    /// Worktree root or git directory (`<root>/.git`) to operate on.
    /// Defaults to the current directory.
    #[arg(long)]
    pub repo: Option<std::path::PathBuf>,
}

#[derive(Debug, Args)]
pub struct GcArgs {
    /// Empty the base store entirely — the disk-pressure remediation. Unlinks
    /// **every** base artefact regardless of references (a base under an active
    /// production claim is skipped and reported — the safe, non-blocking
    /// semantic). Without this flag, only unreferenced bases are reclaimed
    /// (a keep-set pass over the durably-registered worktrees).
    #[arg(long)]
    pub purge_all: bool,
}

pub fn run(args: &GraphBaseArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    match &args.command {
        GraphBaseCommand::Build(build) => run_build(build),
        GraphBaseCommand::Gc(gc) => run_gc(gc),
    }
}

/// The `build` subprocess's one-line JSON contract. Deliberately **path-free**
/// (ADR-105 §2): it carries the merge-base sha, the deterministic counts (only
/// when this run built the graph), and the persistence `outcome` — never an
/// absolute store path.
#[cfg(unix)]
#[derive(serde::Serialize)]
struct BuildOutput {
    merge_base: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edge_count: Option<usize>,
    /// Whether a base artefact is present in the store after this run.
    persisted: bool,
    /// `written` | `already-present` | `claimed-elsewhere` | `unpersisted`.
    outcome: String,
}

#[cfg(unix)]
impl BuildOutput {
    fn from_summary(
        summary: &crate::graph_base_producer::BaseGraphSummary,
        persisted: bool,
        outcome: &str,
    ) -> Self {
        Self {
            merge_base: summary.merge_base.clone(),
            file_count: Some(summary.file_count),
            symbol_count: Some(summary.symbol_count),
            edge_count: Some(summary.edge_count),
            persisted,
            outcome: outcome.to_string(),
        }
    }

    fn sha_only(sha: &str, persisted: bool, outcome: &str) -> Self {
        Self {
            merge_base: sha.to_string(),
            file_count: None,
            symbol_count: None,
            edge_count: None,
            persisted,
            outcome: outcome.to_string(),
        }
    }
}

#[cfg(unix)]
fn run_build(build: &BuildArgs) -> anyhow::Result<()> {
    use crate::graph_base_producer::{
        BaseGraphError, build_and_persist_base, build_base_graph, resolve_base_commit,
    };
    use anvil_intercept::graph_base_trigger::BASE_PRODUCER_CLAIM_FAILURE_EXIT_CODE;
    use anvil_intercept::snapshot_io::base_store::{SystemClaimProcs, default_base_dir};

    let repo_root = match &build.repo {
        Some(path) => anvil_intercept::graph_base_trigger::normalise_repo_path(path),
        None => std::env::current_dir()?,
    };

    let sha = resolve_base_commit(
        &repo_root,
        build.merge_base.as_deref(),
        build.default_branch.as_deref(),
    )
    .map_err(|e| anyhow::anyhow!("could not resolve a merge-base: {e}"))?;

    // Persist to the shared-base store when one resolves. All failure is
    // non-fatal to the daemon (ADR-105 §6) — the CLI surfaces a producer error as
    // a non-zero exit, and the daemon that spawned it degrades to serving cold.
    let output = if let Some(base_dir) = default_base_dir() {
        let persisted = match build_and_persist_base(&repo_root, &sha, &base_dir, &SystemClaimProcs)
        {
            Ok(persisted) => persisted,
            // GBASE-011: a claim-path I/O failure (`base_store::claim` returned
            // `Err`, mapped to `Store { op: "claim", .. }`) exits with a DISTINCT
            // code so the daemon's reaper raises the ADR-090 "base claim could not
            // make progress" health envelope — distinct from a general production
            // failure (any other error → the generic non-zero `EXIT_ERROR`). A
            // normal live-peer contention is a clean `ClaimedElsewhere` exit and
            // never reaches here. No summary to print; the exit code carries the
            // signal (the daemon discards this child's stdout anyway).
            Err(err) if matches!(&err, BaseGraphError::Store { op, .. } if op == "claim") => {
                eprintln!("could not claim the base: {err}");
                std::process::exit(BASE_PRODUCER_CLAIM_FAILURE_EXIT_CODE);
            }
            Err(err) => return Err(anyhow::anyhow!("could not produce the base: {err}")),
        };
        let outcome = persisted.outcome.as_str();
        let persisted_flag = persisted.outcome.persisted();
        match &persisted.summary {
            Some(summary) => BuildOutput::from_summary(summary, persisted_flag, outcome),
            None => BuildOutput::sha_only(&persisted.sha, persisted_flag, outcome),
        }
    } else {
        // No resolvable store dir (no ANVIL_HOME / XDG_STATE_HOME / HOME): build a
        // summary but do not persist.
        let summary = build_base_graph(&repo_root, &sha)
            .map_err(|e| anyhow::anyhow!("could not build the base graph: {e}"))?;
        BuildOutput::from_summary(&summary, false, "unpersisted")
    };

    // One deterministic line of JSON to stdout — the subprocess contract.
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

#[cfg(not(unix))]
fn run_build(_build: &BuildArgs) -> anyhow::Result<()> {
    // The base producer depends on the parser-injection surface, which is
    // unix-only (the inherited platform gap). The daemon serves cold on
    // unsupported platforms, so this is a clean, non-panicking refusal.
    anyhow::bail!("graph-base build is only supported on unix platforms")
}

/// The `gc` subcommand's one-line JSON contract. Deliberately **path-free**:
/// counts + shas only, never an absolute store path.
#[cfg(unix)]
#[derive(serde::Serialize)]
struct GcOutput {
    /// `keep-set` | `purge-all` | `no-store` | `persistence-disabled`.
    mode: &'static str,
    reclaimed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    kept: Option<usize>,
    skipped_claimed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    aborted_uncertain: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<usize>,
}

/// The disk-pressure remediation + on-demand GC surface (GBASE-010 council).
///
/// - `--purge-all` empties the store (claim-respecting, non-blocking), and is
///   **not** gated on `ANVIL_PERSIST_GRAPH` — an operator freeing disk may have
///   opted persistence out yet still hold leftover bases.
/// - The plain pass reclaims only unreferenced bases against the keep-set of
///   durably-registered worktrees, gated on the persistence flag (the exact
///   daemon-side gate), so it never fights an intentionally-off deployment.
#[cfg(unix)]
fn run_gc(gc: &GcArgs) -> anyhow::Result<()> {
    use anvil_intercept::snapshot_io::base_gc;
    use anvil_intercept::snapshot_io::base_store::{SystemClaimProcs, default_base_dir};

    let Some(base_dir) = default_base_dir() else {
        // No resolvable store dir (no ANVIL_HOME / XDG_STATE_HOME / HOME).
        let output = GcOutput {
            mode: "no-store",
            reclaimed: 0,
            kept: None,
            skipped_claimed: 0,
            aborted_uncertain: None,
            errors: None,
        };
        println!("{}", serde_json::to_string(&output)?);
        return Ok(());
    };

    let output = if gc.purge_all {
        let purge = base_gc::purge_all_bases(&base_dir, &SystemClaimProcs);
        GcOutput {
            mode: "purge-all",
            reclaimed: purge.purged.len(),
            kept: None,
            skipped_claimed: purge.skipped_claimed.len(),
            aborted_uncertain: None,
            errors: Some(purge.errors),
        }
    } else {
        let persist_env = std::env::var("ANVIL_PERSIST_GRAPH").ok();
        let worktrees = registered_keep_set();
        match base_gc::run_daemon_gc_pass(persist_env.as_deref(), &worktrees, None) {
            Some(gc_outcome) => GcOutput {
                mode: "keep-set",
                reclaimed: gc_outcome.reclaimed,
                kept: Some(gc_outcome.kept),
                skipped_claimed: gc_outcome.skipped_claimed,
                aborted_uncertain: Some(gc_outcome.aborted_uncertain),
                errors: None,
            },
            None => GcOutput {
                mode: "persistence-disabled",
                reclaimed: 0,
                kept: None,
                skipped_claimed: 0,
                aborted_uncertain: None,
                errors: None,
            },
        }
    };

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

/// The keep-set for an on-demand GC pass: the durably-registered worktrees
/// (ADR-094 = the daemon's GC keep-set), loaded from the same on-disk store the
/// daemon uses. An absent/unreadable store yields an empty keep-set (the pass
/// then treats every base as unreferenced) — safe: an over-eager reclaim is
/// re-produced on the next ref-change trigger (ADR-105 §6).
#[cfg(unix)]
fn registered_keep_set() -> Vec<std::path::PathBuf> {
    use anvil_intercept::fence::default_fence_state_path;
    use anvil_intercept::registration_store::RegistrationStore;

    let Ok(fence_path) = default_fence_state_path() else {
        return Vec::new();
    };
    let store = RegistrationStore::at_path(fence_path.with_file_name("registered-worktrees.json"));
    store
        .load()
        .unwrap_or_default()
        .into_iter()
        .map(|record| record.worktree)
        .collect()
}

#[cfg(not(unix))]
fn run_gc(_gc: &GcArgs) -> anyhow::Result<()> {
    anyhow::bail!("graph-base gc is only supported on unix platforms")
}
