//! `anvil-intercept` daemon binary entry point.
//!
//! The shipped CLI is `anvil intercept start ...` (in `anvil-cli`); this
//! binary exists so the daemon crate is independently runnable for
//! triage and for the demo runbook §4.1 fallback. Both paths call into
//! the same library surface (`anvil_intercept::run_foreground`).

use std::process::ExitCode;

use anyhow::Result;
use anvil_intercept::{ForegroundOpts, Shutdown, run_foreground};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "anvil-intercept", about = "Anvil intercept daemon (A1 scaffold)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the intercept daemon in the current process.
    Start {
        /// Stay in the foreground; logs stream to stdout/stderr and
        /// SIGINT/SIGTERM stops the daemon cleanly.
        #[arg(long, default_value_t = true)]
        foreground: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");

    let result: Result<()> = runtime.block_on(async {
        match cli.command {
            Command::Start { foreground: _ } => {
                let (shutdown, token) = Shutdown::new();
                let signal_shutdown = shutdown.clone();
                tokio::spawn(async move {
                    if tokio::signal::ctrl_c().await.is_ok() {
                        signal_shutdown.trigger();
                    }
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
