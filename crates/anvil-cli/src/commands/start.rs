//! `anvil start` — activation entrypoint (LAUNCH-006 / LAUNCH-009 / LAUNCH-011).
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
//!
//! ## Watch fallback (LAUNCH-011)
//!
//! When MCP cannot pre-write attach (no client has reached
//! `RestartRequired+`), `anvil start --watch` runs the activation
//! orchestrator and then hands off to the kernel watcher inline,
//! scoped to the current repo. The diagnostic block is rendered first
//! (carrying the explicit "MCP pre-write validation is not attached"
//! note from `activation::render_human`); the watcher then takes over
//! the foreground until Ctrl-C.
//!
//! Honesty contract:
//!
//! - `--watch` does NOT make the orchestrator claim a tier the
//!   diagnostic does not back. The render still emits the literal
//!   `WatchTier::Offered` line up to the moment the kernel watcher
//!   starts running.
//! - When MCP IS pre-write attached, `--watch` is a no-op: pre-write
//!   validation already covers the save path, so spawning the watcher
//!   would only generate redundant fallback noise.
//! - `--watch --verify` is rejected — read-only probes do not spawn
//!   processes, and reporting `state: watching` synthetically would
//!   over-claim.
//! - `--watch --json` is rejected — JSON consumers expect a single
//!   parseable document, but the watcher streams its own event lines.

use std::path::Path;

use anyhow::bail;
use clap::Args;

use crate::GlobalArgs;
use crate::activation;
use crate::activation::orchestrator::InstallOutcome;
use crate::commands::watch as watch_cmd;

#[derive(Debug, Args)]
pub struct StartArgs {
    /// Run a non-mutating activation probe — skip init, first-scan, and
    /// the MCP install step. Forwards to the same backend as `anvil
    /// status --verify` (LAUNCH-012).
    #[arg(long)]
    pub verify: bool,
    /// After activation, run the save-time watch fallback when MCP
    /// cannot pre-write attach. Streams kernel watch events on stdout
    /// until Ctrl-C. Honest fallback only — never claimed equivalent
    /// to MCP pre-write validation. LAUNCH-011.
    #[arg(long)]
    pub watch: bool,
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

    if args.watch {
        // LAUNCH-011: the watch fallback path performs work (spawns
        // the kernel watcher) and streams events on stdout. Both
        // properties are incompatible with the read-only / single-
        // document contracts, so we reject the combination explicitly
        // rather than silently degrading. The user is offered the two
        // honest paths (`--verify` for a read-only probe; `--watch`
        // for the live fallback).
        if args.verify {
            bail!(
                "`--watch` and `--verify` are mutually exclusive — `--verify` is read-only and cannot spawn the watcher. Run `anvil start --watch` to enter watch fallback or `anvil start --verify` to probe state."
            );
        }
        if global.json {
            bail!(
                "`--watch` and `--json` are mutually exclusive — the watcher streams event lines on stdout, breaking the single JSON document contract. Run `anvil start --watch` without `--json`, or `anvil start --json` for a read-only diagnostic."
            );
        }
    }

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
        && let Some(err) = install_report.per_client.values().find_map(|o| match o {
            InstallOutcome::Failed { error } => Some(error.as_str()),
            _ => None,
        })
    {
        bail!("MCP install failed: {err}");
    }

    // LAUNCH-011: hand off to the kernel watcher when `--watch` was
    // requested AND MCP did not reach `LiveValidation`. We deliberately
    // skip the spawn when MCP is already live (pre-write validation
    // covers the save path; a fallback layer is redundant) but still
    // run the spawn for `RestartRequired` because the user has chosen
    // explicitly to layer save-time protection on top of the
    // restart-pending state — that is honest belt-and-braces, not
    // theatre.
    if args.watch {
        if diagnostic.mcp_pre_write_live() {
            // No-op message: MCP pre-write validation is already live.
            // The diagnostic above already rendered `state: protecting`
            // — this trailing line just tells the user we honoured the
            // flag without spawning a redundant watcher.
            println!(
                "  watch: skipped — MCP pre-write validation is live; save-time fallback is redundant."
            );
            return Ok(());
        }

        // Print the explicit watch hand-off marker so subprocess
        // consumers (and humans reading the stream) see the moment
        // activation stops and the kernel watcher takes over. The
        // language is conservative on purpose — never "fully
        // protected", never "MCP attached".
        println!(
            "  watch: starting save-time fallback — MCP pre-write validation is not attached; this layer validates files after they are saved."
        );

        let watch_args = watch_cmd::WatchArgs::fallback_for_repo();
        return watch_cmd::run(&watch_args, global);
    }

    Ok(())
}
