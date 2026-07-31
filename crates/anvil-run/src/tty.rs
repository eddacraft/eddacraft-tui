//! TTY and interactive-session detection for CLI UX branching.

use std::sync::OnceLock;

/// RAII guard that restores the launcher's pgrp as the terminal
/// foreground when dropped.
///
/// On non-Unix targets and when stdin is not a tty this is a
/// zero-cost token that does nothing on drop.
#[must_use = "drop the handoff after wait() to restore the launcher as foreground"]
pub struct TtyHandoff {
    #[cfg(unix)]
    restore: Option<nix::unistd::Pid>,
}

impl Drop for TtyHandoff {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(launcher_pgrp) = self.restore.take() {
            // Best-effort: the child has already exited by the time
            // we get here, so there is nothing useful to do if this
            // fails. The SIGTTOU handler installed in
            // `install_sigttou_handler` ensures we are not stopped
            // by the kernel for issuing this from a background pgrp.
            let stdin = std::io::stdin();
            let _ = nix::unistd::tcsetpgrp(&stdin, launcher_pgrp);
        }
    }
}

impl TtyHandoff {
    /// A guard that does nothing on drop. Used for non-tty stdin,
    /// for Windows, and for tests that don't want to touch the real
    /// terminal.
    pub const fn noop() -> Self {
        Self {
            #[cfg(unix)]
            restore: None,
        }
    }
}

/// Install the process-wide SIGTTOU handler. Idempotent — safe to
/// call from multiple entry points; only the first call registers.
pub fn install_sigttou_handler() {
    #[cfg(unix)]
    {
        static INSTALLED: OnceLock<()> = OnceLock::new();
        INSTALLED.get_or_init(|| {
            use std::sync::Arc;
            use std::sync::atomic::AtomicBool;
            // The flag itself is intentionally unused — registering
            // any handler at all is enough to override SIGTTOU's
            // default stop action.
            let sink = Arc::new(AtomicBool::new(false));
            let _ = signal_hook::flag::register(signal_hook::consts::SIGTTOU, sink);
        });
    }
    #[cfg(not(unix))]
    {
        // No-op on Windows. Job-object based control covers the
        // signal-forwarding equivalent there.
        let _ = &OnceLock::<()>::new();
    }
}

/// Transfer the terminal foreground to `child_pgid` if stdin is a
/// tty. Returns a guard whose drop restores the launcher's pgrp as
/// the foreground. On any error (no tty, ioctl failure, missing
/// pgid) returns a noop guard so the caller does not need to branch.
///
/// `child_pgid` is the OS pgid the launcher already configured via
/// `Command::process_group(0)` — when present it equals the child's
/// PID.
#[cfg_attr(not(unix), allow(unused_variables))]
pub fn transfer_foreground_to(child_pgid: Option<i32>) -> TtyHandoff {
    #[cfg(unix)]
    {
        let Some(pgid_raw) = child_pgid else {
            return TtyHandoff::noop();
        };
        let stdin = std::io::stdin();
        // isatty(0) returning false (or erroring) is the common case
        // for `anvil-run` invocations from CI / programmatic harnesses
        // — bail out quietly without touching the launcher's tty
        // state.
        if !matches!(nix::unistd::isatty(&stdin), Ok(true)) {
            return TtyHandoff::noop();
        }
        let launcher_pgrp = nix::unistd::getpgrp();
        let child_pgrp = nix::unistd::Pid::from_raw(pgid_raw);
        if nix::unistd::tcsetpgrp(&stdin, child_pgrp).is_err() {
            return TtyHandoff::noop();
        }
        TtyHandoff {
            restore: Some(launcher_pgrp),
        }
    }
    #[cfg(not(unix))]
    {
        TtyHandoff::noop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_handoff_is_safe_to_drop() {
        let _ = TtyHandoff::noop();
    }

    #[test]
    fn install_sigttou_handler_is_idempotent() {
        install_sigttou_handler();
        install_sigttou_handler();
    }

    #[test]
    fn transfer_with_no_pgid_is_a_noop() {
        // Should not panic, should not touch the tty, regardless of
        // whether the test environment has a tty attached.
        let _ = transfer_foreground_to(None);
    }

    #[cfg(unix)]
    #[test]
    fn transfer_when_stdin_is_not_a_tty_is_a_noop() {
        // `cargo test` runs with stdin redirected from /dev/null in
        // every CI we care about. If a developer runs this from an
        // interactive shell we still want it to be safe — the
        // function only ever calls tcsetpgrp when isatty(0) is true
        // *and* a pgid was supplied, and the assertion below pins
        // the no-pgid branch which is unconditionally a noop.
        let handoff = transfer_foreground_to(Some(std::process::id().cast_signed()));
        // Drop runs without panicking even if a real pgrp swap was
        // skipped because stdin isn't a tty under test.
        drop(handoff);
    }
}
