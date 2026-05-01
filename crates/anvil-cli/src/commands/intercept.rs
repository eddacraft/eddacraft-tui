//! `anvil intercept` — INTD-001 scaffold.
//!
//! Wires the shipped `anvil` binary's CLI surface to the intercept
//! daemon library. Today only `start --foreground` is implemented;
//! later INTD tasks add `status`, `stop`, and the backgrounded launch
//! path.

use anvil_intercept::{ForegroundOpts, Shutdown, run_foreground, wait_for_shutdown_signal};
use anyhow::Result;
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
    /// Stay in the foreground; logs stream to stdout/stderr.
    /// Ctrl+C (and SIGTERM on Unix) stops the daemon cleanly. Demo
    /// runbook §4.1 fallback path.
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
            // Trigger shutdown on either a signal arriving or a
            // signal-handler installation failure. Swallowing the Err
            // would leave the daemon hanging in restricted /
            // containerised environments where signal install fails;
            // the operator-visible diagnostic plus a triggered
            // shutdown lets the foreground loop unwind cleanly.
            if let Err(err) = wait_for_shutdown_signal().await {
                #[allow(clippy::uninlined_format_args)]
                {
                    eprintln!("anvil intercept: shutdown signal handler failed: {}", err);
                }
            }
            signal_shutdown.trigger();
        });
        run_foreground(ForegroundOpts::default(), token).await
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the contract: `anvil intercept start` without `--foreground`
    /// exits with a clear error directing the user at the missing
    /// flag, not a silent no-op or a confusing panic. Future flag
    /// changes that drop the bail (e.g. flipping the default to
    /// foreground) must update this test.
    #[test]
    fn run_start_without_foreground_bails_with_actionable_message() {
        let args = StartArgs { foreground: false };
        let err = run_start(&args).expect_err("expected bail when --foreground omitted");
        let msg = format!("{err}");
        assert!(
            msg.contains("--foreground"),
            "bail message must mention --foreground, got: {msg}",
        );
        assert!(
            msg.contains("INTD-002"),
            "bail message must point at the future backgrounded path (INTD-002), got: {msg}",
        );
    }
}
