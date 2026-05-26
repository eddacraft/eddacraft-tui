//! Read-only dashboard picker surface and per-domain dashboard surfaces.
//!
//! Lists the native Anvil dashboards a user can open. The CLI owns the
//! catalogue of dashboards (which are wired, which are planned); this surface
//! renders whatever entries it is given and records the user's choice. It is
//! the shared scaffold the per-domain dashboards (architecture, drift,
//! suppressions) plug into as they land.

pub mod architecture;
pub mod drift;
pub mod render;
pub mod suppressions;

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::surface::Surface;

/// A single dashboard the picker can list.
///
/// Rendering-only metadata — the CLI builds these from its catalogue. Kept free
/// of serde on purpose: serialization of the catalogue lives in the CLI, this
/// crate only renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardEntry {
    /// Machine name used on the command line (e.g. `architecture`).
    pub name: String,
    /// Human-facing title shown in the picker.
    pub title: String,
    /// One-line summary of what the dashboard shows.
    pub description: String,
    /// Whether the dashboard surface is implemented yet. Planned-but-unbuilt
    /// dashboards still list, marked as coming soon.
    pub available: bool,
}

impl DashboardEntry {
    /// Build a dashboard entry. `available` is `false` for planned dashboards
    /// whose surface has not landed yet.
    pub fn new(
        name: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        available: bool,
    ) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            description: description.into(),
            available,
        }
    }
}

/// Read-only picker listing the available dashboards.
pub struct DashboardPickerState {
    /// Dashboards to list, in display order.
    pub entries: Vec<DashboardEntry>,
    /// Index of the highlighted entry.
    pub selected: usize,
    /// Set when the surface wants to exit.
    pub should_quit: bool,
    /// Machine name of the dashboard the user chose with Enter, if any. The CLI
    /// inspects this after the surface exits to launch the chosen dashboard.
    pub chosen: Option<String>,
}

impl DashboardPickerState {
    /// Create a picker over the given entries.
    #[must_use]
    pub fn new(entries: Vec<DashboardEntry>) -> Self {
        Self {
            entries,
            selected: 0,
            should_quit: false,
            chosen: None,
        }
    }

    /// The currently highlighted entry, if any.
    #[must_use]
    pub fn selected_entry(&self) -> Option<&DashboardEntry> {
        self.entries.get(self.selected)
    }

    // Navigation clamps at the ends rather than wrapping: the picker is a short
    // fixed list, so stopping at the boundary reads more predictably than a
    // jump from last back to first.
    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    fn choose_selected(&mut self) {
        if let Some(entry) = self.selected_entry() {
            // Only available dashboards can be opened; selecting a coming-soon
            // entry is a no-op so the picker stays put.
            if entry.available {
                self.chosen = Some(entry.name.clone());
                self.should_quit = true;
            }
        }
    }

    fn handle_picker_action(&mut self, action: Action) {
        match action {
            Action::Up => self.move_up(),
            Action::Down => self.move_down(),
            Action::Select => self.choose_selected(),
            Action::Back | Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

impl Surface for DashboardPickerState {
    fn surface_name(&self) -> &'static str {
        "Dashboards"
    }

    fn help_text(&self) -> &'static str {
        "j/k navigate  enter open  esc/q quit"
    }

    fn handle_key(&mut self, action: Action) {
        self.handle_picker_action(action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        render::render(frame, area, self, theme);
    }
}

#[cfg(test)]
pub(crate) fn sample_state() -> DashboardPickerState {
    DashboardPickerState::new(vec![
        DashboardEntry::new(
            "architecture",
            "Architecture Health",
            "Layer boundaries, violations, and rule compliance",
            false,
        ),
        DashboardEntry::new(
            "drift",
            "Drift Snapshots",
            "Snapshot history and new-edge deltas vs baseline",
            false,
        ),
        DashboardEntry::new(
            "suppressions",
            "Suppressions",
            "Active suppressions with scope, justification, approver",
            false,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_clamps_at_bounds() {
        let mut state = sample_state();
        assert_eq!(state.selected, 0);
        state.handle_key(Action::Up); // already at top
        assert_eq!(state.selected, 0);
        state.handle_key(Action::Down);
        assert_eq!(state.selected, 1);
        state.handle_key(Action::Down);
        state.handle_key(Action::Down); // past the end
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn selecting_unavailable_dashboard_is_a_noop() {
        let mut state = sample_state(); // all entries available: false
        state.handle_key(Action::Select);
        assert!(state.chosen.is_none());
        assert!(!state.should_quit);
    }

    #[test]
    fn selecting_available_dashboard_records_choice_and_quits() {
        let mut state = DashboardPickerState::new(vec![DashboardEntry::new(
            "architecture",
            "Architecture Health",
            "desc",
            true,
        )]);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen.as_deref(), Some("architecture"));
        assert!(state.should_quit);
    }

    #[test]
    fn quit_sets_should_quit_without_choice() {
        let mut state = sample_state();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
        assert!(state.chosen.is_none());
    }
}
