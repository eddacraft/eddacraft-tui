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
//! handoff, double-fork on Unix, service install on Windows) is owned
//! by the daemon-lifecycle module (DLIFE), gated on ADR-082.

use std::process::ExitCode;

use anvil_intercept::{ForegroundOpts, Shutdown, config, run_foreground, wait_for_shutdown_signal};
use anyhow::{Context, Result};
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
    /// foreground today — backgrounded launch is owned by the
    /// daemon-lifecycle module (DLIFE), gated on ADR-082.
    Start,
}

fn main() -> ExitCode {
    // TRACE-001: install the daemon's tracing subscriber before any
    // request paths spin up. `Err` means a global subscriber is
    // already registered (test harness, parent context, or a
    // misbehaving dependency); the daemon stays up on that subscriber
    // but surfaces the condition so operators can diagnose missing
    // spans rather than silently dropping all daemon observability.
    // Deployment note: stderr must be captured by the supervising
    // process manager (systemd / launchd) — the daemon-lifecycle
    // module (DLIFE) owns the background-launch capture story.
    if let Err(err) =
        anvil_observability::init_tracing(anvil_observability::BinaryKind::InterceptDaemon)
    {
        eprintln!("anvil-intercept: tracing subscriber init skipped: {err}");
    }

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
                        let mut message =
                            String::from("anvil-intercept: shutdown signal handler failed: ");
                        message.push_str(&err.to_string());
                        message.push('\n');
                        let mut stderr = std::io::stderr().lock();
                        let _ = std::io::Write::write_all(&mut stderr, message.as_bytes());
                    }
                    signal_shutdown.trigger();
                });
                // INTD-016 / MLP2-024 / #1671 audit closure: load
                // `.anvil.yaml` from the daemon's launch CWD so the
                // operator-visible `enforcement.dos.*` and
                // `enforcement.session.per_worktree_max` knobs reach
                // the listener and registry. Pre-fix the daemon
                // always ran on `Resolved::default()` and silently
                // ignored every YAML override. See
                // `config::load_for_daemon_cwd` for the full
                // propagation contract — parse / IO failures are
                // fatal per `LoadError::{Parse, Io}`.
                let enforcement_config = config::load_for_daemon_cwd()
                    .context("anvil-intercept: failed to load enforcement config")?;
                run_foreground(
                    ForegroundOpts::default().with_enforcement_config(enforcement_config),
                    token,
                )
                .await
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
