//! RAII session for mouse capture and optional terminal lifecycle (TUIN-019).

use super::MouseCaptureGuard;

/// Holds mouse capture for the interactive session and restores it on drop.
///
/// # Stability
///
/// **experimental** (TUIN-019).
pub struct FlowSession {
    _mouse: MouseCaptureGuard,
}

impl FlowSession {
    /// Enable mouse reporting. Safe on terminals that do not support it.
    #[must_use]
    pub fn enter() -> Self {
        Self {
            _mouse: MouseCaptureGuard::enable(),
        }
    }

    /// Enter a TUI session with [`crate::lifecycle::TerminalGuard`] composed.
    ///
    /// Mouse capture is enabled after the terminal is in raw/alt-screen mode.
    ///
    /// # Errors
    ///
    /// Propagates lifecycle enter failures.
    #[cfg(feature = "lifecycle")]
    #[cfg_attr(docsrs, doc(cfg(feature = "lifecycle")))]
    pub fn enter_with_terminal() -> std::io::Result<(Self, crate::lifecycle::TerminalGuard)> {
        let terminal = crate::lifecycle::TerminalGuard::enter()?;
        Ok((Self::enter(), terminal))
    }
}

#[cfg(all(test, feature = "lifecycle"))]
const _: fn() -> std::io::Result<(FlowSession, crate::lifecycle::TerminalGuard)> =
    FlowSession::enter_with_terminal;
