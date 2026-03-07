pub mod keyboard;
pub mod theme;
pub mod widgets;

pub mod prelude {
    pub use crate::keyboard::{Action, KeyHandler};
    pub use crate::theme::{EddaCraftTheme, Theme};
    pub use crate::widgets::confirm::{Confirm, ConfirmState};
    pub use crate::widgets::container::{Container, ContainerVariant};
    pub use crate::widgets::divider::{Divider, DividerVariant};
    pub use crate::widgets::header::Header;
    pub use crate::widgets::log_panel::{LogEntry, LogFilter, LogLevel, LogPanel, LogPanelState};
    pub use crate::widgets::parallel_progress::{
        calculate_eta, calculate_overall_progress, format_duration, CheckProgress, CheckStatus,
        ParallelProgress, ParallelProgressState,
    };
    pub use crate::widgets::progress_bar::{ProgressBar, ProgressBarState};
    pub use crate::widgets::quick_wins_panel::{
        BatchGroup, QuickWinType, QuickWinsAnalysis, QuickWinsPanel, QuickWinsPanelState,
    };
    pub use crate::widgets::results_dashboard::{
        HistoricalAnalysis, InitAnalysisResults, ResultsDashboard, ResultsDashboardState,
    };
    pub use crate::widgets::select::{Select, SelectState};
    pub use crate::widgets::spinner::{Spinner, SpinnerState};
    pub use crate::widgets::status_badge::{BadgeStatus, StatusBadge};
    pub use crate::widgets::status_bar::StatusBar;
    pub use crate::widgets::text_input::{TextInput, TextInputState};
}
