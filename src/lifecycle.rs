use std::io;
use std::sync::OnceLock;

use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};

use crate::shared_mode::SharedMode;

/// Live [`TerminalGuard`]s in this process. Raw mode and the alternate screen
/// belong to the process, so only the first guard enters them and only the
/// last one restores them (GH #6557).
static TERMINAL: SharedMode = SharedMode::new();

/// Best-effort terminal restoration for raw-mode / alternate-screen sessions.
///
/// Safe to call when those modes are not active; crossterm treats both restore
/// calls as no-ops on a normal terminal.
pub fn restore_terminal() {
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
}

fn install_panic_hook() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_terminal();
            previous_hook(info);
        }));
    });
}

/// RAII guard for an interactive terminal session.
///
/// [`enter`](Self::enter) enables raw mode and the alternate screen. `Drop`
/// restores both as a best effort, including during unwinding panic. Use
/// [`leave`](Self::leave) when callers want restoration errors surfaced.
///
/// # Nesting
///
/// Raw mode and the alternate screen belong to the process, so guards may
/// overlap: a nested surface, or guards on several threads. Every live guard
/// shares one session. The first guard enters it, and the terminal is
/// restored only when the last live guard leaves or drops; leaving or
/// dropping any other guard changes nothing on the terminal. The panic hook
/// and [`restore_terminal`] still restore the terminal unconditionally.
pub struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    /// Enable raw mode and enter the alternate screen, or join the session
    /// another live guard already entered.
    ///
    /// # Errors
    ///
    /// Returns the error from enabling raw mode or entering the alternate
    /// screen. A failed entry is rolled back and leaves no guard counted.
    pub fn enter() -> io::Result<Self> {
        install_panic_hook();
        TERMINAL.acquire(enter_session)?;
        Ok(Self { active: true })
    }

    /// Disarm the drop guard, restoring the terminal when this is the last
    /// live guard.
    ///
    /// # Errors
    ///
    /// Returns the error from leaving the alternate screen or disabling raw
    /// mode. The guard then stays armed, so its `Drop` retries the restore.
    pub fn leave(mut self) -> io::Result<()> {
        self.disarm_and_restore()
    }

    fn disarm_and_restore(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        TERMINAL.release(leave_session)?;
        self.active = false;
        Ok(())
    }
}

fn enter_session() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    if let Err(error) = execute!(io::stdout(), EnterAlternateScreen) {
        let _ = terminal::disable_raw_mode();
        return Err(error);
    }
    Ok(())
}

fn leave_session() -> io::Result<()> {
    execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.active {
            TERMINAL.release_best_effort(restore_terminal);
            self.active = false;
        }
    }
}
