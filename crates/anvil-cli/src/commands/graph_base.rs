//! `anvil graph-base` — hidden subprocess harness for the shared base-graph
//! producer.
//!
//! Not a user-facing command: it is the subprocess entry point the save-time
//! daemon will detach to when producing a shared base snapshot. For this first
//! slice it resolves the merge-base commit, builds the base graph from the
//! committed tree (never a working tree), and prints a deterministic one-line
//! JSON summary (file / symbol / edge counts) to stdout. Persisting the base to
//! disk arrives in a later slice.
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

#[cfg(unix)]
fn run_build(build: &BuildArgs) -> anyhow::Result<()> {
    use crate::graph_base_producer::{build_base_graph, resolve_base_commit};

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

    let summary = build_base_graph(&repo_root, &sha)
        .map_err(|e| anyhow::anyhow!("could not build the base graph: {e}"))?;

    // One deterministic line of JSON to stdout — the subprocess contract.
    println!("{}", serde_json::to_string(&summary)?);
    Ok(())
}

#[cfg(not(unix))]
fn run_build(_build: &BuildArgs) -> anyhow::Result<()> {
    // The base producer depends on the parser-injection surface, which is
    // unix-only (the inherited platform gap). The daemon serves cold on
    // unsupported platforms, so this is a clean, non-panicking refusal.
    anyhow::bail!("graph-base build is only supported on unix platforms")
}
