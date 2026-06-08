use std::io;
use std::sync::OnceLock;

use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};

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
pub struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    /// Enable raw mode and enter the alternate screen.
    pub fn enter() -> io::Result<Self> {
        install_panic_hook();
        terminal::enable_raw_mode()?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen) {
            let _ = terminal::disable_raw_mode();
            return Err(error);
        }
        Ok(Self { active: true })
    }

    /// Restore the terminal and disarm the drop guard.
    pub fn leave(mut self) -> io::Result<()> {
        self.disarm_and_restore()
    }

    fn disarm_and_restore(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        execute!(io::stdout(), LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        self.active = false;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.active {
            restore_terminal();
            self.active = false;
        }
    }
}
