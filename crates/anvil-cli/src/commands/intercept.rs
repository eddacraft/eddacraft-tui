//! `anvil intercept` — INTD-001 scaffold.
//!
//! Wires the shipped `anvil` binary's CLI surface to the intercept
//! daemon library. Today only `start --foreground` is implemented;
//! later INTD tasks add `status`, `stop`, and the backgrounded launch
//! path.

use anyhow::Result;
use anvil_intercept::{ForegroundOpts, Shutdown, run_foreground};
use clap::{Args, Subcommand};

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct InterceptArgs {
    #[command(subcommand)]
    command: InterceptCommand,
}

#[derive(Debug, Subcommand)]
enum InterceptCommand {
    /// Start the intercept daemon. Currently only `--foreground` is
    /// supported; backgrounded launch arrives with INTD-002.
    Start(StartArgs),
}

#[derive(Debug, Args)]
struct StartArgs {
    /// Stay in the foreground; logs stream to stdout/stderr and
    /// SIGINT/SIGTERM stops the daemon cleanly. Demo runbook §4.1
    /// fallback path.
    #[arg(long)]
    foreground: bool,
}

pub fn run(args: &InterceptArgs, _global: &GlobalArgs) -> Result<()> {
    match &args.command {
        InterceptCommand::Start(start_args) => run_start(start_args),
    }
}

fn run_start(args: &StartArgs) -> Result<()> {
    if !args.foreground {
        anyhow::bail!(
            "`anvil intercept start` currently requires --foreground; backgrounded launch lands with INTD-002"
        );
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let (shutdown, token) = Shutdown::new();
        let signal_shutdown = shutdown.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                signal_shutdown.trigger();
            }
        });
        run_foreground(ForegroundOpts::default(), token).await
    })
}
