pub mod event_adapter;
pub mod render;

use std::collections::VecDeque;

use animate::{Animate, Lerp, Once};
use eddacraft_tui::keyboard::Action;

type AnimatedF64 = Once<f64, fn(f64) -> f64, fn(&f64, &f64, f64) -> f64>;

const ANIM_DURATION_MS: f64 = 250.0;

fn animated_f64(initial: f64) -> AnimatedF64 {
    Once::new(
        initial,
        ANIM_DURATION_MS,
        animate::easing::quad_out as fn(f64) -> f64,
        <f64 as Lerp>::lerp as fn(&f64, &f64, f64) -> f64,
    )
}

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
    pub queue: VecDeque<QueuedChange>,
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
    pub wants_back: bool,
    pub(crate) anim_pass_rate: AnimatedF64,
    pub(crate) anim_pass_rate_target: f64,
    pub(crate) anim_avg_duration_ms: AnimatedF64,
    pub(crate) anim_avg_duration_target: f64,
    /// Set when state changes; consumed by `take_dirty()` before redraw.
    /// Use `mark_dirty()` / `take_dirty()` — field is crate-visible for tests.
    pub(crate) dirty: bool,
}

impl WatchState {
    pub fn new(data: WatchData) -> Self {
        let pass_rate = data.stats.pass_rate;
        let avg_duration_ms = data.stats.avg_duration_ms as f64;

        Self {
            data,
            focused_panel: WatchPanel::Status,
            selected_item: 0,
            should_quit: false,
            wants_back: false,
            anim_pass_rate: animated_f64(pass_rate),
            anim_pass_rate_target: pass_rate,
            anim_avg_duration_ms: animated_f64(avg_duration_ms),
            anim_avg_duration_target: avg_duration_ms,
            dirty: true, // render immediately on first frame
        }
    }

    /// Mark state as needing a redraw.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Whether a redraw is pending.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Consume the dirty flag, returning whether a redraw is needed.
    /// Clears the flag immediately — call this right before rendering.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    pub fn sync_animations(&mut self) {
        let pass_rate = self.data.stats.pass_rate;
        if (pass_rate - self.anim_pass_rate_target).abs() > f64::EPSILON {
            self.anim_pass_rate.set(pass_rate);
            self.anim_pass_rate_target = pass_rate;
        }
        self.anim_pass_rate.update();

        let avg_duration = self.data.stats.avg_duration_ms as f64;
        if (avg_duration - self.anim_avg_duration_target).abs() > f64::EPSILON {
            self.anim_avg_duration_ms.set(avg_duration);
            self.anim_avg_duration_target = avg_duration;
        }
        self.anim_avg_duration_ms.update();
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
                // Scroll within the panel while there are items above; once
                // at the top (or on item-less panels like Status/Stats),
                // spill over to the panel in the row above so arrow keys
                // navigate the 2×2 grid without needing PgUp/PgDn.
                if self.max_items_in_panel() > 0 && self.selected_item > 0 {
                    self.selected_item -= 1;
                    self.mark_dirty();
                } else {
                    self.focused_panel = self.focused_panel.up();
                    self.selected_item = 0;
                    self.mark_dirty();
                }
            }
            Action::Down => {
                // Scroll within the panel while there are items below; once
                // at the bottom (or on item-less panels), spill over to the
                // panel in the row below.
                let max = self.max_items_in_panel().saturating_sub(1);
                if self.max_items_in_panel() > 0 && self.selected_item < max {
                    self.selected_item += 1;
                    self.mark_dirty();
                } else {
                    self.focused_panel = self.focused_panel.down();
                    self.selected_item = 0;
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
            Action::Back => {
                self.wants_back = true;
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
        "\u{2191}\u{2193}/jk scroll  \u{2190}\u{2192}/hl panel  esc back  q quit"
    }

    fn handle_key(&mut self, action: eddacraft_tui::keyboard::Action) {
        self.handle_key(action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        self.should_quit = false;
        self.wants_back = false;
        self.dirty = true; // ensure first frame renders on re-entry
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
            queue: VecDeque::from([
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
            ]),
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

        // At the bottom of the Queue list — another Down spills over to the
        // panel in the row below (Stats).
        state.handle_key(Action::Down);
        assert_eq!(state.focused_panel, WatchPanel::Stats);
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

        // At the bottom of the History list — another Down spills to the
        // panel in the row below (which wraps back to Status).
        state.handle_key(Action::Down);
        assert_eq!(state.focused_panel, WatchPanel::Status);
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn arrow_up_at_top_of_list_spills_to_row_above() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::History;
        state.selected_item = 0;

        // Up at top of History list should spill up to Status.
        state.handle_key(Action::Up);
        assert_eq!(state.focused_panel, WatchPanel::Status);
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn arrow_down_at_bottom_of_list_spills_to_row_below() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::Queue;
        state.selected_item = state.data.queue.len() - 1;

        // Down at bottom of Queue list should spill to Stats (row below).
        state.handle_key(Action::Down);
        assert_eq!(state.focused_panel, WatchPanel::Stats);
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
    fn edge_scroll_spills_to_adjacent_row_queue() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::Queue;
        state.selected_item = 0;
        state.dirty = false;

        // Up at top of Queue spills to the panel above (Stats — up wraps).
        state.handle_key(Action::Up);
        assert!(state.dirty);
        assert_eq!(state.focused_panel, WatchPanel::Stats);

        // Down at bottom of Queue spills to the panel below (Stats again).
        state.focused_panel = WatchPanel::Queue;
        state.selected_item = state.data.queue.len() - 1;
        state.dirty = false;
        state.handle_key(Action::Down);
        assert!(state.dirty);
        assert_eq!(state.focused_panel, WatchPanel::Stats);
    }

    #[test]
    fn edge_scroll_spills_to_adjacent_row_history() {
        let mut state = WatchState::new(sample_data());
        state.focused_panel = WatchPanel::History;
        state.selected_item = 0;
        state.dirty = false;

        // Up at top of History spills up to Status.
        state.handle_key(Action::Up);
        assert!(state.dirty);
        assert_eq!(state.focused_panel, WatchPanel::Status);

        // Down at bottom of History spills down (wraps to Status).
        state.focused_panel = WatchPanel::History;
        state.selected_item = state.data.history.len() - 1;
        state.dirty = false;
        state.handle_key(Action::Down);
        assert!(state.dirty);
        assert_eq!(state.focused_panel, WatchPanel::Status);
    }

    #[test]
    fn mark_dirty_sets_flag() {
        let mut state = WatchState::new(sample_data());
        state.dirty = false;
        state.mark_dirty();
        assert!(state.dirty);
    }

    #[test]
    fn back_sets_wants_back() {
        let mut state = WatchState::new(sample_data());
        state.handle_key(Action::Back);
        assert!(state.wants_back);
        assert!(state.dirty);
    }

    #[test]
    fn reset_clears_back_and_quit() {
        use crate::surface::Surface;
        let mut state = WatchState::new(sample_data());
        state.should_quit = true;
        state.wants_back = true;
        state.dirty = false;
        state.reset();
        assert!(!state.should_quit);
        assert!(!state.wants_back);
        assert!(state.dirty); // reset restores dirty for re-entry
    }
}
