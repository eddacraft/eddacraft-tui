//! `anvil start` — activation entrypoint (LAUNCH-006).
//!
//! Thin wrapper over `activation::orchestrator`. The orchestration logic
//! lives in the activation module so LAUNCH-009 / LAUNCH-010 / LAUNCH-011
//! can extend the diagnostic probes without touching this command.
//!
//! Behavioural promotion: previously `anvil start` was a clap alias for
//! `anvil welcome` (the menu / tutorial surface). It now drives the
//! activation flow that ends in one literal `ProtectionState`. `anvil
//! welcome` is unchanged and remains the documented menu surface.

use std::path::Path;

use clap::Args;

use crate::GlobalArgs;
use crate::activation;

#[derive(Debug, Args)]
pub struct StartArgs {
    /// Run a non-mutating activation probe — skip init and first-scan.
    /// Forwards to the same backend as `anvil status --verify`
    /// (LAUNCH-012).
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

    let diagnostic = if read_only {
        activation::verify(root)
    } else {
        activation::orchestrator::run(root, global)?
    };

    if global.json {
        let json = serde_json::to_string_pretty(&activation::render_json(&diagnostic))?;
        println!("{json}");
    } else {
        print!("{}", activation::render_human(&diagnostic));
    }

    Ok(())
}
