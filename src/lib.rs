//! # eddacraft-tui
//!
//! Shared Ratatui component library for the EddaCraft product family.
//!
//! Provides a consistent set of terminal UI building blocks — themed widgets,
//! keyboard handling, shell chrome, and a surface abstraction — so that every
//! EddaCraft TUI application shares the same look and feel.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use eddacraft_tui::prelude::*;
//!
//! let theme = EddaCraftTheme;
//! // Use any widget with the theme:
//! // let spinner = Spinner::new(&theme).label("Loading…");
//! ```
//!
//! ## Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`widgets`] | 12 reusable Ratatui widgets (select, text input, progress, etc.) |
//! | [`theme`] | `Theme` trait + `EddaCraftTheme` implementation |
//! | [`keyboard`] | `KeyHandler` mapping crossterm events to semantic `Action`s |
//! | [`surface`] | `Surface` trait for multi-screen TUI applications |
//! | [`shell`] | Branded header/footer chrome renderer |
//! | [`compat`] | Terminal detection and minimum-size validation |
//! | [`test_utils`] | Snapshot testing helpers for style-aware buffer serialisation |

pub mod compat;
pub mod keyboard;
pub mod shell;
pub mod surface;
pub mod test_utils;
pub mod theme;
pub mod widgets;

pub mod prelude {
    pub use crate::compat::{TerminalInfo, detect_terminal, validate_minimum_size};
    pub use crate::keyboard::{Action, KeyHandler};
    pub use crate::shell::render_shell;
    pub use crate::surface::Surface;
    pub use crate::theme::{EddaCraftTheme, Theme};
    pub use crate::widgets::confirm::{Confirm, ConfirmState};
    pub use crate::widgets::container::{Container, ContainerVariant};
    pub use crate::widgets::divider::{Divider, DividerVariant};
    pub use crate::widgets::header::Header;
    pub use crate::widgets::log_panel::{LogEntry, LogFilter, LogLevel, LogPanel, LogPanelState};
    pub use crate::widgets::parallel_progress::{
        CheckProgress, CheckStatus, ParallelProgress, ParallelProgressState, calculate_eta,
        calculate_overall_progress, format_duration,
    };
    pub use crate::widgets::progress_bar::{ProgressBar, ProgressBarState};
    pub use crate::widgets::select::{Select, SelectItem, SelectState};
    pub use crate::widgets::spinner::{Spinner, SpinnerState};
    pub use crate::widgets::status_badge::{BadgeStatus, StatusBadge};
    pub use crate::widgets::status_bar::StatusBar;
    pub use crate::widgets::text_input::{TextInput, TextInputState};
}
