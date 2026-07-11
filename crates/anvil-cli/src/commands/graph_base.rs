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

    /// Repository root to operate on. Defaults to the current directory.
    #[arg(long)]
    pub repo: Option<std::path::PathBuf>,
}

pub fn run(args: &GraphBaseArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    match &args.command {
        GraphBaseCommand::Build(build) => run_build(build),
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
        build_and_persist_base, build_base_graph, resolve_base_commit,
    };
    use anvil_intercept::snapshot_io::base_store::{SystemClaimProcs, default_base_dir};

    let repo_root = match &build.repo {
        Some(path) => path.clone(),
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
        let persisted = build_and_persist_base(&repo_root, &sha, &base_dir, &SystemClaimProcs)
            .map_err(|e| anyhow::anyhow!("could not produce the base: {e}"))?;
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
