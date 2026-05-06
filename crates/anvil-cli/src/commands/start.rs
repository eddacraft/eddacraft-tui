//! `anvil start` — activation entrypoint (LAUNCH-006 / LAUNCH-009).
//!
//! Thin wrapper over `activation::orchestrator`. The orchestration logic
//! lives in the activation module so LAUNCH-009 / LAUNCH-010 / LAUNCH-011
//! can extend the diagnostic probes without touching this command.
//!
//! Behavioural promotion: previously `anvil start` was a clap alias for
//! `anvil welcome` (the menu / tutorial surface). It now drives the
//! activation flow that ends in one literal `ProtectionState`. `anvil
//! welcome` is unchanged and remains the documented menu surface.
//!
//! `anvil start` writes MCP config entries for Cursor and Claude Code to
//! the user's home directory (`~/.cursor/mcp.json`, `~/.claude.json`).
//! Pass `--verify` to skip writes and run a read-only probe instead.

use std::path::Path;

use anyhow::bail;
use clap::Args;

use crate::GlobalArgs;
use crate::activation;
use crate::activation::orchestrator::InstallOutcome;

#[derive(Debug, Args)]
pub struct StartArgs {
    /// Run a non-mutating activation probe — skip init, first-scan, and
    /// the MCP install step. Forwards to the same backend as `anvil
    /// status --verify` (LAUNCH-012).
    #[arg(long)]
    pub verify: bool,
}

pub fn run(args: &StartArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let root = Path::new(".");

    // `--json` implies read-only: init writes its own JSON record to
    // stdout in JSON mode, which would produce two concatenated JSON
    // documents and break parseable consumers. Match `anvil status
    // --verify --json` (LAUNCH-012) — the activation diagnostic is the
    // entire JSON output. Users who want side-effects under JSON
    // should call `anvil init --json` and `anvil start --json`
    // separately.
    let read_only = args.verify || global.json;

    let (diagnostic, install_report) = if read_only {
        (
            activation::verify(root),
            activation::orchestrator::InstallReport::default(),
        )
    } else {
        activation::orchestrator::run(root, global)?
    };

    if global.json {
        let json = serde_json::to_string_pretty(&activation::render_json(&diagnostic))?;
        println!("{json}");
    } else {
        print!(
            "{}",
            activation::render_human_with_install(&diagnostic, &install_report)
        );
    }

    // If any client's install step actually failed, propagate as a
    // non-zero exit so CI hooks (`anvil start && next-step`) don't
    // silently advance past a broken activation. The diagnostic +
    // install report already carry the human/JSON detail; this just
    // wires the exit-code contract to match.
    //
    // Skip in --verify (read-only, install never ran) and --json
    // (programmatic consumers should parse `state` and `last_error`
    // from the JSON document; the `last_error` field carries every
    // failure, aggregated).
    if !read_only
        && let Some(err) = install_report
            .per_client
            .values()
            .find_map(|o| match o {
                InstallOutcome::Failed { error } => Some(error.as_str()),
                _ => None,
            })
    {
        bail!("MCP install failed: {err}");
    }

    Ok(())
}
