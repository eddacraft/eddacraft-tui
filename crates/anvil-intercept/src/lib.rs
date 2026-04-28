//! INTD-001: Anvil intercept daemon library surface.
//!
//! This A1 scaffold establishes:
//!
//! - A `run_foreground` entry point with cooperative shutdown via a
//!   tokio cancellation handle. The CLI calls into this from
//!   `anvil intercept start --foreground`; tests drive it through the
//!   same path without sending real signals.
//! - A future `Daemon` lifecycle handle (INTD-002 onwards) that
//!   subsequent tasks (INTD-002 IPC listener, INTD-003 session
//!   registry, INTD-005 enforcement pipeline) attach behind without
//!   touching the CLI surface.
//! - [`wait_for_shutdown_signal`] — the single source of truth for
//!   signal handling shared by the daemon binary and the CLI
//!   subcommand, so SIGINT and (on Unix) SIGTERM cannot drift between
//!   entry points.
//!
//! Intentionally out of scope here:
//!
//! - PID files (deferred until INTD-002 lands the IPC listener that
//!   actually needs a single-instance guard).
//! - Backgrounded / double-fork daemonisation (INTD-002+).
//! - Cross-platform signal handling beyond SIGINT and Unix SIGTERM.
//!   Windows `JobObject` termination arrives with INTD-006.
//!
//! See `plans/modules/intercept-daemon.aps.md` and
//! `plans/decisions/015-intercept-loop-enforcement.md`.

#![forbid(unsafe_code)]

// INTD-005 wires rule evaluation into the enforcement pipeline.
// Until then the rules crate is unused at the call-site level —
// `use ... as _` silences `unused-crate-dependencies`. INTD-002
// consumes `anvil_intercept_proto` directly from `ipc.rs`.
use anvil_intercept_rules as _;

pub mod ipc;

use std::time::Duration;

use anyhow::Result;
use tokio::sync::watch;

/// Options accepted by [`run_foreground`]. Currently empty; future
/// tasks add the socket path, config path, and observe-only flag here.
#[derive(Debug, Default, Clone)]
pub struct ForegroundOpts {}

/// Cooperative shutdown handle. Held by the caller; calling
/// [`Shutdown::trigger`] flips the watch channel and the foreground
/// loop returns at its next await point.
#[derive(Debug, Clone)]
pub struct Shutdown {
    tx: watch::Sender<bool>,
}

impl Shutdown {
    /// Build a fresh shutdown handle plus the receiver the daemon
    /// loop awaits on. Tests construct one of these directly; the
    /// `--foreground` CLI path wires the receiver to
    /// [`wait_for_shutdown_signal`].
    #[must_use]
    pub fn new() -> (Self, ShutdownToken) {
        let (tx, rx) = watch::channel(false);
        (Self { tx }, ShutdownToken { rx })
    }

    /// Mint a fresh [`ShutdownToken`] from this handle. The new token
    /// observes the current shutdown state immediately, so a token
    /// minted after [`Shutdown::trigger`] resolves on the next
    /// [`ShutdownToken::cancelled`] without waiting.
    ///
    /// Use this when a downstream consumer (an INTD-002 IPC handler,
    /// for example) needs its own token but the original receiver
    /// has already been moved into another future.
    #[must_use]
    pub fn token(&self) -> ShutdownToken {
        ShutdownToken {
            rx: self.tx.subscribe(),
        }
    }

    /// Request shutdown. Idempotent — repeated calls are a no-op.
    ///
    /// Uses `send_replace`, which never fails: it overwrites the
    /// watched value regardless of receiver count. Even after every
    /// [`ShutdownToken`] has been dropped (no one to notify), the
    /// trigger is recorded — any token minted later via
    /// [`Shutdown::token`] observes the triggered state on its first
    /// [`ShutdownToken::cancelled`] call.
    pub fn trigger(&self) {
        self.tx.send_replace(true);
    }
}

/// Receiver-side of [`Shutdown`]. Awaiting [`ShutdownToken::cancelled`]
/// resolves once `trigger` has been called.
#[derive(Debug, Clone)]
pub struct ShutdownToken {
    rx: watch::Receiver<bool>,
}

impl ShutdownToken {
    /// Resolve when shutdown has been requested.
    ///
    /// Takes `&mut self` because [`watch::Receiver::changed`] requires
    /// it. Callers that need to await cancellation from multiple
    /// `tokio::select!` arms simultaneously must clone the token —
    /// `ShutdownToken` is `Clone` and cloning a `watch::Receiver` is
    /// cheap. INTD-002 onwards is expected to hold one cloned token
    /// per spawned handler future; the registry-style "share one
    /// token across consumers" idiom needs to clone first.
    pub async fn cancelled(&mut self) {
        // Already triggered before we awaited.
        if *self.rx.borrow_and_update() {
            return;
        }
        // `changed()` resolves when the watched value transitions; if
        // every sender drops we treat that as a cancellation too,
        // because no one can flip the flag any more.
        let _ = self.rx.changed().await;
    }
}

/// Wait for the operating system to ask the daemon to stop, on every
/// platform the daemon supports.
///
/// - Unix: races SIGINT (via [`tokio::signal::ctrl_c`]) and SIGTERM
///   (via [`tokio::signal::unix`]). Either wakes the future. SIGTERM
///   is the signal `kill <pid>`, `systemd stop`, Docker, and
///   Kubernetes use; SIGINT is the controlling-terminal Ctrl+C.
/// - Windows: only Ctrl+C is wired today. Process-manager
///   termination on Windows uses `JobObject` semantics, which
///   INTD-006 owns.
///
/// Both intercept entrypoints (`anvil intercept start --foreground`
/// in the CLI, the standalone `anvil-intercept` binary) call this
/// helper. Keeping the signal logic in one place stops the two
/// entrypoints drifting — a shutdown signal that cleanly stops one
/// must cleanly stop the other.
///
/// Returns when any supported signal arrives; errors only if the
/// signal infrastructure itself fails to install (rare, generally
/// fatal).
pub async fn wait_for_shutdown_signal() -> Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate())
            .map_err(|err| anyhow::anyhow!("failed to install SIGTERM handler: {err}"))?;

        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                result.map_err(|err| anyhow::anyhow!("ctrl_c handler failed: {err}"))?;
            }
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .map_err(|err| anyhow::anyhow!("ctrl_c handler failed: {err}"))?;
    }

    Ok(())
}

/// Run the intercept daemon in the current process. Blocks until
/// `shutdown` is triggered (by SIGINT/SIGTERM in production, or by
/// the caller in tests).
///
/// The body of the loop is intentionally a single sleep tick: the
/// scaffold demonstrates clean shutdown without asserting any
/// particular IPC or watcher integration. INTD-002 grafts the IPC
/// listener onto this loop; INTD-003 the session registry; etc.
pub async fn run_foreground(_opts: ForegroundOpts, mut token: ShutdownToken) -> Result<()> {
    // TODO(INTD-002): once IPC handler tasks are spawned here, track
    // them in a `tokio::task::JoinSet` so shutdown can drain with a
    // bounded deadline (then hard-abort). Today the loop only ticks,
    // so a slow handler problem cannot exist yet — but the registry
    // shape that prevents one needs to be in place before the IPC
    // listener lands. Do not paste handler-spawn code into the select
    // arm without adding the JoinSet first.
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    loop {
        tokio::select! {
            biased;
            () = token.cancelled() => return Ok(()),
            _ = tick.tick() => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;

    /// `Shutdown::trigger` before `run_foreground` is awaited still
    /// stops the loop on the first poll — the cancellation flag is
    /// observed via `borrow_and_update`, not just via `changed()`.
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_returns_when_shutdown_already_triggered() {
        let (shutdown, token) = Shutdown::new();
        shutdown.trigger();

        let result = timeout(
            Duration::from_secs(1),
            run_foreground(ForegroundOpts::default(), token),
        )
        .await
        .expect("foreground loop did not return after pre-triggered shutdown");
        result.expect("foreground loop reported error");
    }

    /// Triggering shutdown after the loop has started still resolves
    /// promptly — well inside the 250 ms tick interval is fine because
    /// `cancelled` resolves on the watch transition, not on the tick.
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_returns_when_shutdown_triggered_concurrently() {
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(run_foreground(ForegroundOpts::default(), token));

        // Yield once so the spawned task enters its select.
        tokio::task::yield_now().await;
        shutdown.trigger();

        let result = timeout(Duration::from_secs(1), handle)
            .await
            .expect("foreground loop did not return after shutdown trigger")
            .expect("join failure");
        result.expect("foreground loop reported error");
    }

    /// Multiple `trigger` calls are idempotent and do not panic.
    #[tokio::test(flavor = "current_thread")]
    async fn shutdown_trigger_is_idempotent() {
        let (shutdown, token) = Shutdown::new();
        shutdown.trigger();
        shutdown.trigger();
        shutdown.trigger();

        let result = timeout(
            Duration::from_secs(1),
            run_foreground(ForegroundOpts::default(), token),
        )
        .await
        .expect("foreground loop did not return after repeated triggers");
        result.expect("foreground loop reported error");
    }

    /// Trigger applied after every receiver dropped still records the
    /// state, and a fresh token minted via [`Shutdown::token`]
    /// observes it without further work. This is the property
    /// `send_replace` (used by `trigger`) gives us over `send`, which
    /// would silently no-op when no receivers exist.
    #[tokio::test(flavor = "current_thread")]
    async fn shutdown_trigger_survives_all_tokens_dropped() {
        let (shutdown, token) = Shutdown::new();
        drop(token);
        shutdown.trigger();

        // Mint a brand-new token from the handle and verify it
        // observes the triggered state. Without this assertion the
        // test would pass even if `trigger` became a no-op.
        let mut late_token = shutdown.token();
        let result = timeout(Duration::from_secs(1), late_token.cancelled()).await;
        assert!(
            result.is_ok(),
            "fresh token did not observe pre-triggered shutdown",
        );
    }
}
