pub mod event_adapter;
pub mod render;

use eddacraft_tui::keyboard::Action;

/// Current watch mode status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchStatus {
    Idle,
    Running,
    Passing,
    Failing,
}

impl WatchStatus {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Idle => "o",
            Self::Running => "~",
            Self::Passing => "*",
            Self::Failing => "x",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Running => "Running",
            Self::Passing => "Passing",
            Self::Failing => "Failing",
        }
    }
}

/// A file change queued for processing.
#[derive(Debug, Clone)]
pub struct QueuedChange {
    pub file: String,
    pub kind: String,
    pub timestamp: String,
}

/// A completed gate run in history.
#[derive(Debug, Clone)]
pub struct RunHistory {
    pub passed: bool,
    pub checks_run: usize,
    pub checks_passed: usize,
    pub duration_ms: u64,
    pub timestamp: String,
}

/// Aggregate watch statistics.
#[derive(Debug, Clone)]
pub struct WatchStats {
    pub total_runs: usize,
    pub pass_rate: f64,
    pub avg_duration_ms: u64,
    pub files_watched: usize,
}

/// All data needed by the watch dashboard.
#[derive(Debug, Clone)]
pub struct WatchData {
    pub status: WatchStatus,
    pub queue: Vec<QueuedChange>,
    pub history: Vec<RunHistory>,
    pub stats: WatchStats,
}

/// Which panel is focused in the 2x2 grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchPanel {
    Status,
    Queue,
    History,
    Stats,
}

impl WatchPanel {
    #[must_use]
    pub fn right(self) -> Self {
        match self {
            Self::Status => Self::Queue,
            Self::Queue => Self::Status,
            Self::History => Self::Stats,
            Self::Stats => Self::History,
        }
    }

    #[must_use]
    pub fn left(self) -> Self {
        self.right() // symmetric in a 2-column grid
    }

    #[must_use]
    pub fn down(self) -> Self {
        match self {
            Self::Status => Self::History,
            Self::Queue => Self::Stats,
            Self::History => Self::Status,
            Self::Stats => Self::Queue,
        }
    }

    #[must_use]
    pub fn up(self) -> Self {
        self.down() // symmetric in a 2-row grid
    }
}

/// State for the watch dashboard surface.
pub struct WatchState {
    pub data: WatchData,
    pub focused_panel: WatchPanel,
    pub selected_item: usize,
    pub should_quit: bool,
    /// Set when state changes and cleared after a render cycle.
    /// Use `mark_dirty()` / `take_dirty()` — field is crate-visible for tests.
    pub(crate) dirty: bool,
}

impl WatchState {
    pub fn surface_name(&self) -> &'static str {
        "w a t c h"
    }

    pub fn help_text(&self) -> &'static str {
        "h/l j/k panels  q quit"
    }

    pub fn new(data: WatchData) -> Self {
        Self {
            data,
            focused_panel: WatchPanel::Status,
            selected_item: 0,
            should_quit: false,
            dirty: true, // render immediately on first frame
        }
    }

    /// Mark state as needing a redraw.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Consume the dirty flag, returning whether a redraw is needed.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    fn max_items_in_panel(&self) -> usize {
        match self.focused_panel {
            WatchPanel::Status | WatchPanel::Stats => 0,
            WatchPanel::Queue => self.data.queue.len(),
            WatchPanel::History => self.data.history.len(),
        }
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.selected_item > 0 {
                    self.selected_item -= 1;
                    self.mark_dirty();
                }
            }
            Action::Down => {
                let max = self.max_items_in_panel().saturating_sub(1);
                if self.selected_item < max {
                    self.selected_item += 1;
                    self.mark_dirty();
                }
            }
            Action::Right => {
                self.focused_panel = self.focused_panel.right();
                self.selected_item = 0;
                self.mark_dirty();
            }
            Action::Left => {
                self.focused_panel = self.focused_panel.left();
                self.selected_item = 0;
                self.mark_dirty();
            }
            Action::PageDown => {
                self.focused_panel = self.focused_panel.down();
                self.selected_item = 0;
                self.mark_dirty();
            }
            Action::PageUp => {
                self.focused_panel = self.focused_panel.up();
                self.selected_item = 0;
                self.mark_dirty();
            }
            Action::Quit => {
                self.should_quit = true;
                self.mark_dirty();
            }
            _ => {}
        }
    }
}

impl crate::surface::Surface for WatchState {
    fn surface_name(&self) -> &'static str {
        "Watch"
    }

    fn help_text(&self) -> &'static str {
        "j/k navigate  h/l switch panel  PgUp/PgDn row  q quit"
    }

    fn handle_key(&mut self, action: eddacraft_tui::keyboard::Action) {
        self.handle_key(action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn render(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        theme: &eddacraft_tui::theme::EddaCraftTheme,
    ) {
        render::render(frame, area, self, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> WatchData {
        WatchData {
            status: WatchStatus::Passing,
            queue: vec![
                QueuedChange {
                    file: "src/main.rs".to_string(),
                    kind: "modified".to_string(),
                    timestamp: "10:30:01".to_string(),
                },
                QueuedChange {
                    file: "src/lib.rs".to_string(),
                    kind: "created".to_string(),
                    timestamp: "10:30:02".to_string(),
                },
            ],
            history: vec![
                RunHistory {
                    passed: true,
                    checks_run: 5,
                    checks_passed: 5,
                    duration_ms: 1200,
                    timestamp: "10:29:50".to_string(),
                },
                RunHistory {
                    passed: false,
                    checks_run: 5,
                    checks_passed: 3,
                    duration_ms: 980,
                    timestamp: "10:28:30".to_string(),
                },
                RunHistory {
                    passed: true,
                    checks_run: 4,
                    checks_passed: 4,
                    duration_ms: 750,
                    timestamp: "10:27:00".to_string(),
                },
            ],
            stats: WatchStats {
                total_runs: 42,
                pass_rate: 0.88,
                avg_duration_ms: 1050,
                files_watched: 128,
            },
        }
    }

    #[test]
    fn panel_navigation_left_right() {
        let mut state = WatchState::new(sample_data());
        assert_eq!(state.focused_panel, WatchPanel::Status);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, WatchPanel::Queue);
        assert_eq!(state.selected_item, 0);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, WatchPanel::Status); // wraps

        state.handle_key(Action::Left);
        assert_eq!(state.focused_panel, WatchPanel::Queue);
    }

    #[test]
    fn panel_navigation_up_down() {
        let mut state = WatchState::new(sample_data());
        assert_eq!(state.focused_panel, WatchPanel::Status);

        state.handle_key(Action::PageDown);
        assert_eq!(state.focused_panel, WatchPanel::History);

        state.handle_key(Action::PageDown);
        assert_eq!(state.focused_panel, WatchPanel::Status); // wraps

        state.handle_key(Action::PageUp);
        assert_eq!(state.focused_panel, WatchPanel::History);
    }

    #[test]
    fn panel_navigation_grid_traversal() {
        let mut state = WatchState::new(sample_data());

        // Status -> Right -> Queue
        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, WatchPanel::Queue);

        // Queue -> PageDown -> Stats
        state.handle_key(Action::PageDown);
        assert_eq!(state.focused_panel, WatchPanel::Stats);

        // Stats -> Left -> History
        state.handle_key(Action::Left);
        assert_eq!(state.focused_panel, WatchPanel::History);

        // History -> PageUp -> Status
        state.handle_key(Action::PageUp);
        assert_eq!(state.focused_panel, WatchPanel::Status);
    }

    #[test]
    fn item_navigation_within_queue() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::Queue;

        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 1);

        state.handle_key(Action::Down); // at max
        assert_eq!(state.selected_item, 1);

        state.handle_key(Action::Up);
        assert_eq!(state.selected_item, 0);

        state.handle_key(Action::Up); // at min
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn item_navigation_within_history() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::History;

        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 1);
        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 2);
        state.handle_key(Action::Down); // at max
        assert_eq!(state.selected_item, 2);
    }

    #[test]
    fn panel_switch_resets_selection() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::Queue;
        state.selected_item = 1;

        state.handle_key(Action::Right);
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn status_no_item_navigation() {
        let mut state = WatchState::new(sample_data());
        assert_eq!(state.focused_panel, WatchPanel::Status);

        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 0); // no items
    }

    #[test]
    fn status_display_variants() {
        assert_eq!(WatchStatus::Idle.icon(), "o");
        assert_eq!(WatchStatus::Running.icon(), "~");
        assert_eq!(WatchStatus::Passing.icon(), "*");
        assert_eq!(WatchStatus::Failing.icon(), "x");

        assert_eq!(WatchStatus::Idle.label(), "Idle");
        assert_eq!(WatchStatus::Running.label(), "Running");
        assert_eq!(WatchStatus::Passing.label(), "Passing");
        assert_eq!(WatchStatus::Failing.label(), "Failing");
    }

    #[test]
    fn quit_sets_flag() {
        let mut state = WatchState::new(sample_data());
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn quit_sets_dirty_for_final_frame() {
        let mut state = WatchState::new(sample_data());
        state.dirty = false;
        state.handle_key(Action::Quit);
        assert!(state.dirty);
    }

    #[test]
    fn new_state_is_dirty() {
        let state = WatchState::new(sample_data());
        assert!(state.dirty);
    }

    #[test]
    fn take_dirty_clears_flag() {
        let mut state = WatchState::new(sample_data());
        assert!(state.take_dirty());
        assert!(!state.dirty);
        assert!(!state.take_dirty());
    }

    #[test]
    fn key_navigation_sets_dirty() {
        let mut state = WatchState::new(sample_data());
        state.dirty = false;

        state.handle_key(Action::Right);
        assert!(state.dirty);

        state.dirty = false;
        state.handle_key(Action::Left);
        assert!(state.dirty);

        state.dirty = false;
        state.handle_key(Action::PageDown);
        assert!(state.dirty);

        state.dirty = false;
        state.handle_key(Action::PageUp);
        assert!(state.dirty);
    }

    #[test]
    fn item_scroll_sets_dirty() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::Queue;
        state.dirty = false;

        state.handle_key(Action::Down);
        assert!(state.dirty);

        state.dirty = false;
        state.handle_key(Action::Up);
        assert!(state.dirty);
    }

    #[test]
    fn noop_scroll_stays_clean_queue() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::Queue;
        state.selected_item = 0;
        state.dirty = false;

        // Up at top — no change
        state.handle_key(Action::Up);
        assert!(!state.dirty);

        // Down to max
        state.selected_item = state.data.queue.len() - 1;
        state.dirty = false;
        state.handle_key(Action::Down);
        assert!(!state.dirty);
    }

    #[test]
    fn noop_scroll_stays_clean_history() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::History;
        state.selected_item = 0;
        state.dirty = false;

        // Up at top — no change
        state.handle_key(Action::Up);
        assert!(!state.dirty);

        // Down to max
        state.selected_item = state.data.history.len() - 1;
        state.dirty = false;
        state.handle_key(Action::Down);
        assert!(!state.dirty);
    }

    #[test]
    fn mark_dirty_sets_flag() {
        let mut state = WatchState::new(sample_data());
        state.dirty = false;
        state.mark_dirty();
        assert!(state.dirty);
    }
}
