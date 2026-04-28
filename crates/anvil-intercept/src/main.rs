//! `anvil-intercept` daemon binary entry point.
//!
//! The shipped CLI is `anvil intercept start ...` (in `anvil-cli`); this
//! binary exists so the daemon crate is independently runnable for
//! triage and for the demo runbook §4.1 fallback. Both paths call into
//! the same library surface (`anvil_intercept::run_foreground`) and use
//! the shared `wait_for_shutdown_signal` helper, so signal handling
//! cannot drift between them.
//!
//! INTD-001 scaffold: `anvil-intercept start` runs the daemon in the
//! foreground unconditionally. The backgrounded launch path (PID file
//! handoff, double-fork on Unix, service install on Windows) lands
//! with INTD-002.

use std::process::ExitCode;

use anvil_intercept::{ForegroundOpts, Shutdown, run_foreground, wait_for_shutdown_signal};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "anvil-intercept",
    about = "Anvil intercept daemon (A1 scaffold)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the intercept daemon in the current process. Always
    /// foreground today — backgrounded launch lands with INTD-002.
    Start,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");

    let result: Result<()> = runtime.block_on(async {
        match cli.command {
            Command::Start => {
                let (shutdown, token) = Shutdown::new();
                let signal_shutdown = shutdown.clone();
                tokio::spawn(async move {
                    // Trigger shutdown whether the signal arrived or
                    // the handler install itself failed — silently
                    // swallowing Err would leave the daemon hanging
                    // in restricted environments. The operator-visible
                    // diagnostic plus a triggered shutdown lets the
                    // foreground loop unwind cleanly.
                    if let Err(err) = wait_for_shutdown_signal().await {
                        eprintln!("anvil-intercept: shutdown signal handler failed: {err}");
                    }
                    signal_shutdown.trigger();
                });
                run_foreground(ForegroundOpts::default(), token).await
            }
        }
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("anvil-intercept: {err:#}");
            ExitCode::FAILURE
        }
    }
}
