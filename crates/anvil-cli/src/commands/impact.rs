//! `anvil impact` — interactive boundary/impact graph of the current
//! repository (IMPV-001).
//!
//! Opens the anvil-tui impact surface over the warm graph-cache snapshot
//! (ADR-069): the crate-level used-import graph with drill-down into a
//! crate's neighbourhood or its internal module graph. Read-only; the
//! snapshot is produced by the daemon, never written here.
//!
//! `--json` prints a stable document of the crate-level graph; a
//! non-interactive stdout (or `--no-tui`) prints a text summary instead of
//! opening the TUI.

use std::io::IsTerminal;

use anvil_tui::surfaces::impact::{ImpactGraph, ImpactState};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;
use crate::tui;
use crate::util;

/// Arguments for `anvil impact`.
#[derive(Debug, Args)]
pub struct ImpactArgs {}

/// Stable `--json` envelope. `graph_present` distinguishes a degraded state
/// (no/unusable snapshot, reason in `unavailable`) from a loaded graph
/// without the top-level shape flipping between object and `null`.
#[derive(Debug, Serialize)]
struct ImpactJson {
    graph_present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    unavailable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graph: Option<ImpactGraphJson>,
}

#[derive(Debug, Serialize)]
struct ImpactGraphJson {
    crates: usize,
    edges: Vec<(String, String)>,
}

pub fn run(_args: &ImpactArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let root = util::workspace_root()?;
    let loaded = ImpactGraph::load(&root);

    if global.json {
        let payload = match &loaded {
            Ok(graph) => ImpactJson {
                graph_present: true,
                unavailable: None,
                graph: Some(ImpactGraphJson {
                    crates: crate_count(&graph.crate_edges),
                    edges: graph.crate_edges.clone(),
                }),
            },
            Err(err) => ImpactJson {
                graph_present: false,
                unavailable: Some(err.to_string()),
                graph: None,
            },
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        match &loaded {
            Ok(graph) => {
                println!(
                    "impact: {} crates, {} used-import edges (open in a terminal for the interactive graph)",
                    crate_count(&graph.crate_edges),
                    graph.crate_edges.len()
                );
                for (from, to) in &graph.crate_edges {
                    println!("  {from} -> {to}");
                }
            }
            Err(err) => println!("impact view unavailable: {err}"),
        }
        return Ok(());
    }

    let state = match loaded {
        Ok(graph) => ImpactState::from_graph(graph),
        // degraded states still open the surface so the reason is on screen
        Err(_) => ImpactState::load(&root),
    };
    // Back and Quit are equivalent for a standalone top-level surface.
    let (_, _exit) = tui::run_surface_with_exit(state)?;
    Ok(())
}

fn crate_count(edges: &[(String, String)]) -> usize {
    let mut set = std::collections::BTreeSet::new();
    for (a, b) in edges {
        set.insert(a);
        set.insert(b);
    }
    set.len()
}
