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
}

impl WatchState {
    pub fn new(data: WatchData) -> Self {
        Self {
            data,
            focused_panel: WatchPanel::Status,
            selected_item: 0,
            should_quit: false,
        }
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
                }
            }
            Action::Down => {
                let max = self.max_items_in_panel().saturating_sub(1);
                if self.selected_item < max {
                    self.selected_item += 1;
                }
            }
            Action::Right => {
                self.focused_panel = self.focused_panel.right();
                self.selected_item = 0;
            }
            Action::Left => {
                self.focused_panel = self.focused_panel.left();
                self.selected_item = 0;
            }
            Action::PageDown => {
                self.focused_panel = self.focused_panel.down();
                self.selected_item = 0;
            }
            Action::PageUp => {
                self.focused_panel = self.focused_panel.up();
                self.selected_item = 0;
            }
            Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
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
}
