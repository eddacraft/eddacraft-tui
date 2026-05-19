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

use anvil_intercept::{
    ForegroundOpts, Shutdown, config::Resolved, run_foreground, wait_for_shutdown_signal,
};
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
    // TRACE-001: install the daemon's tracing subscriber before any
    // request paths spin up. `Err` means a global subscriber is
    // already registered (test harness, parent context, or a
    // misbehaving dependency); the daemon stays up on that subscriber
    // but surfaces the condition so operators can diagnose missing
    // spans rather than silently dropping all daemon observability.
    // Deployment note: stderr must be captured by the supervising
    // process manager (systemd / launchd) — INTD-002 owns the
    // background-launch capture story.
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
                // `enforcement.session.per_worktree_max` knobs actually
                // reach the listener and registry. Pre-fix the daemon
                // always ran on `Resolved::default()` and silently
                // ignored every YAML override. Read errors fall back to
                // defaults with operator-visible diagnostics rather
                // than aborting startup — a malformed config should
                // not brick the daemon's recovery path.
                let enforcement_config = load_enforcement_config();
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

/// Load the resolved enforcement config from `<cwd>/.anvil.yaml`,
/// degrading to `Resolved::default()` if the file is missing,
/// unreadable, or malformed. Operator visibility is via stderr —
/// the daemon must not refuse to start because of a typo in YAML.
///
/// `user_config_path = None` until the daemon grows a dedicated
/// user-config search (a follow-on item). A single project-style
/// `.anvil.yaml` in the launch directory is the documented
/// operator surface today.
fn load_enforcement_config() -> Resolved {
    let cwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            eprintln!(
                "anvil-intercept: cannot resolve CWD for config load ({err}); using defaults"
            );
            return Resolved::default();
        }
    };
    match Resolved::load(&cwd, None) {
        Ok(resolved) => resolved,
        Err(err) => {
            eprintln!(
                "anvil-intercept: failed to load .anvil.yaml from {} ({err}); using defaults",
                cwd.display()
            );
            Resolved::default()
        }
    }
}
