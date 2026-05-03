//! # eddacraft-tui
//!
//! Shared Ratatui component library for the `eddacraft` product family.
//!
//! Provides a consistent set of terminal UI building blocks — themed widgets,
//! keyboard handling, shell chrome, and a surface abstraction — so that every
//! `eddacraft` TUI application shares the same look and feel.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use eddacraft_tui::prelude::*;
//!
//! let theme = EddaCraftTheme;
//! // Use any widget with the theme:
//! // let spinner = Spinner::new(&theme).eddacraft().label("Loading...");
//! // render_shell(frame, area, ShellBranding::Anvil, "anvil", "Watch", "[q] quit", &theme, env!("CARGO_PKG_VERSION"));
//! ```
//!
//! ## Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`widgets`] | A growing suite of reusable Ratatui widgets (select, text input, progress, data table, tree, modal, toast, overlay, etc.) |
//! | [`animation`] | Lightweight shims over the internal animation runtime |
//! | [`pretext`] | Two-phase prepare/layout text engine — measure once, lay out cheaply |
//! | [`theme`] | `Theme` trait + `EddaCraftTheme` implementation |
//! | [`keyboard`] | `KeyHandler` mapping crossterm events to semantic `Action`s |
//! | [`surface`] | `Surface` trait for multi-screen TUI applications |
//! | [`shell`] | Branded header/footer chrome renderer |
//! | [`compat`] | Terminal detection and minimum-size validation |
//! | [`test_utils`] | Snapshot testing helpers for style-aware buffer serialisation |

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod animation;
pub mod compat;
pub mod keyboard;
pub mod pretext;
pub mod shell;
pub mod surface;
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
pub mod theme;
pub mod widgets;

pub mod prelude {
    // Animation shim is owned by this crate so the underlying engine can be
    // swapped without breaking the prelude API.
    pub use crate::animation::{animate_tick, is_animating};
    pub use crate::compat::{TerminalInfo, detect_terminal, validate_minimum_size};
    pub use crate::keyboard::{Action, Binding, KeyHandler};
    pub use crate::pretext::{ExclusionZone, LayoutResult, PositionedWord, PreparedText};
    pub use crate::shell::{ShellBranding, render_shell};
    pub use crate::surface::Surface;
    pub use crate::theme::{EddaCraftTheme, Role, Theme};
    pub use crate::widgets::confirm::{Confirm, ConfirmState};
    pub use crate::widgets::container::{Container, ContainerVariant};
    pub use crate::widgets::data_table::{DataTable, DataTableState, SortDirection, SortIndicator};
    pub use crate::widgets::divider::{Divider, DividerVariant};
    pub use crate::widgets::editor::{Editor, EditorState};
    pub use crate::widgets::header::Header;
    pub use crate::widgets::help_bar::HelpBar;
    pub use crate::widgets::log_panel::{LogEntry, LogFilter, LogLevel, LogPanel, LogPanelState};
    pub use crate::widgets::modal::Modal;
    pub use crate::widgets::overlay::{Layer, OverlayStack, Placement};
    pub use crate::widgets::parallel_progress::{
        CheckProgress, CheckStatus, ParallelProgress, ParallelProgressState,
    };
    pub use crate::widgets::pretext::{PretextState, PretextWidget};
    pub use crate::widgets::progress_bar::{ProgressBar, ProgressBarState};
    pub use crate::widgets::select::{Select, SelectItem, SelectState};
    pub use crate::widgets::spinner::{Spinner, SpinnerPreset, SpinnerState, anvil, eddacraft};
    pub use crate::widgets::status_badge::{BadgeStatus, StatusBadge};
    pub use crate::widgets::status_bar::StatusBar;
    pub use crate::widgets::text_input::{TextInput, TextInputState};
    pub use crate::widgets::toast::{Toast, ToastPlacement, ToastStack};
    pub use crate::widgets::tree::{Tree, TreeNode, TreeState};
    pub use crate::widgets::wrappers::{Disableable, Hideable, Padded};

    #[cfg(feature = "big-text")]
    #[cfg_attr(docsrs, doc(cfg(feature = "big-text")))]
    pub use crate::widgets::big_banner::BigBanner;
    #[cfg(feature = "image")]
    #[cfg_attr(docsrs, doc(cfg(feature = "image")))]
    pub use crate::widgets::image_pane::ImagePane;
}
