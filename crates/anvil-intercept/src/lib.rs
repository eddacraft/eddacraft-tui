//! INTD-001: Anvil intercept daemon library surface.
//!
//! This A1 scaffold establishes:
//!
//! - A `run_foreground` entry point with cooperative shutdown via a
//!   tokio cancellation handle. The CLI calls into this from
//!   `anvil intercept start --foreground`; tests drive it through the
//!   same path without sending real signals.
//! - A `Daemon` lifecycle handle that future tasks (INTD-002 IPC
//!   listener, INTD-003 session registry, INTD-005 enforcement
//!   pipeline) attach behind without touching the CLI surface.
//!
//! Intentionally out of scope here:
//!
//! - PID files (deferred until INTD-002 lands the IPC listener that
//!   actually needs a single-instance guard).
//! - Backgrounded / double-fork daemonisation (INTD-002+).
//! - Cross-platform signal handling beyond SIGINT / SIGTERM /
//!   `ctrl_c`. Windows `JobObject` termination arrives with INTD-006.
//!
//! See `plans/modules/intercept-daemon.aps.md` and
//! `plans/decisions/015-intercept-loop-enforcement.md`.

#![forbid(unsafe_code)]

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
    /// `--foreground` CLI path wires the receiver to `tokio::signal`.
    #[must_use]
    pub fn new() -> (Self, ShutdownToken) {
        let (tx, rx) = watch::channel(false);
        (Self { tx }, ShutdownToken { rx })
    }

    /// Request shutdown. Idempotent — repeated calls are a no-op.
    pub fn trigger(&self) {
        // `send_replace` always succeeds because we hold a clone of
        // the sender ourselves, so there is at least one receiver.
        let _ = self.tx.send(true);
    }
}

/// Receiver-side of [`Shutdown`]. Awaiting [`ShutdownToken::cancelled`]
/// resolves once `trigger` has been called.
#[derive(Debug, Clone)]
pub struct ShutdownToken {
    rx: watch::Receiver<bool>,
}

impl ShutdownToken {
    /// Resolve when shutdown has been requested. Cheap to call from
    /// multiple awaiters — every clone of the token sees the same
    /// transition.
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

/// Run the intercept daemon in the current process. Blocks until
/// `shutdown` is triggered (by SIGINT/SIGTERM in production, or by
/// the caller in tests).
///
/// The body of the loop is intentionally a single sleep tick: the
/// scaffold demonstrates clean shutdown without asserting any
/// particular IPC or watcher integration. INTD-002 grafts the IPC
/// listener onto this loop; INTD-003 the session registry; etc.
pub async fn run_foreground(_opts: ForegroundOpts, mut token: ShutdownToken) -> Result<()> {
    // A real heartbeat tick will replace this once the daemon loop
    // does work. For now it just keeps the future polling so a
    // shutdown signal is observed.
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
}
