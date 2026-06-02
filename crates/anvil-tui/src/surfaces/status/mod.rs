pub mod render;

use eddacraft_tui::keyboard::Action;

/// Status of a single hook.
#[derive(Debug, Clone)]
pub struct HookStatus {
    pub name: String,
    pub active: bool,
    pub path: String,
}

/// Configuration profile information.
#[derive(Debug, Clone)]
pub struct ProfileInfo {
    pub name: String,
    pub checks: Vec<String>,
    pub path: String,
}

/// Result of a recent gate run.
#[derive(Debug, Clone)]
pub struct GateRunResult {
    pub timestamp: String,
    pub passed: bool,
    pub score: f64,
    pub checks_run: usize,
    pub checks_passed: usize,
    pub duration_ms: u64,
}

/// All data needed by the status dashboard.
#[derive(Debug, Clone)]
pub struct StatusData {
    pub hooks: Vec<HookStatus>,
    pub profile: ProfileInfo,
    pub recent_runs: Vec<GateRunResult>,
    /// DISTRIB-002: one-line "update available" hint set by anvil-cli
    /// when a newer release is detected and the 24h rate-limit allows
    /// rendering. `None` when no update is available, the probe was
    /// skipped (offline / network failure), or the hint already fired
    /// within the rate-limit window for this version.
    pub update_hint: Option<crate::surfaces::UpdateHint>,
    /// INSIGHTS-004: one-line first-week nudge pointing at `anvil
    /// insights`. Present only for users within 14 days of project
    /// install (per `anvil/project-id` `created_at`) and only once per
    /// 7-day week, and only when the user has not run the default
    /// insights summary in that week. `None` otherwise.
    pub insights_hint: Option<String>,
}

/// Which panel is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusPanel {
    Hooks,
    Profile,
    Results,
}

impl StatusPanel {
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Hooks => Self::Profile,
            Self::Profile => Self::Results,
            Self::Results => Self::Hooks,
        }
    }

    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            Self::Hooks => Self::Results,
            Self::Profile => Self::Hooks,
            Self::Results => Self::Profile,
        }
    }
}

/// State for the status dashboard surface.
pub struct StatusState {
    pub data: StatusData,
    pub focused_panel: StatusPanel,
    pub selected_item: usize,
    /// When `true`, only the focused panel renders, taking the full
    /// area. Same affordance as the watch surface — press `z` to
    /// toggle, `esc` while zoomed exits zoom before navigating back.
    pub zoomed: bool,
    pub should_quit: bool,
    pub wants_back: bool,
}

impl StatusState {
    pub fn surface_name(&self) -> &'static str {
        "s t a t u s"
    }

    pub fn help_text(&self) -> &'static str {
        "h/l panels  j/k navigate  esc back  q quit"
    }

    pub fn new(data: StatusData) -> Self {
        Self {
            data,
            focused_panel: StatusPanel::Hooks,
            selected_item: 0,
            zoomed: false,
            should_quit: false,
            wants_back: false,
        }
    }

    fn max_items_in_panel(&self) -> usize {
        match self.focused_panel {
            StatusPanel::Hooks => self.data.hooks.len(),
            StatusPanel::Profile => self.data.profile.checks.len(),
            StatusPanel::Results => self.data.recent_runs.len(),
        }
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up if self.selected_item > 0 => {
                self.selected_item -= 1;
            }
            Action::Down => {
                let max = self.max_items_in_panel().saturating_sub(1);
                if self.selected_item < max {
                    self.selected_item += 1;
                }
            }
            Action::Right | Action::PageDown => {
                self.focused_panel = self.focused_panel.next();
                self.selected_item = 0;
            }
            Action::Left | Action::PageUp => {
                self.focused_panel = self.focused_panel.prev();
                self.selected_item = 0;
            }
            Action::Character('z') => {
                self.zoomed = !self.zoomed;
            }
            Action::Back if self.zoomed => {
                self.zoomed = false;
            }
            Action::Back => {
                self.wants_back = true;
            }
            Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

impl crate::surface::Surface for StatusState {
    fn surface_name(&self) -> &'static str {
        if self.zoomed {
            "Status [zoom]"
        } else {
            "Status"
        }
    }

    fn help_text(&self) -> &'static str {
        if self.zoomed {
            "j/k navigate  z unzoom  esc unzoom  q quit"
        } else {
            "j/k navigate  h/l switch panel  z zoom  esc back  q quit"
        }
    }

    fn handle_key(&mut self, action: Action) {
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

    fn sample_data() -> StatusData {
        StatusData {
            hooks: vec![
                HookStatus {
                    name: "pre-commit".to_string(),
                    active: true,
                    path: ".husky/pre-commit".to_string(),
                },
                HookStatus {
                    name: "commit-msg".to_string(),
                    active: false,
                    path: ".husky/commit-msg".to_string(),
                },
            ],
            profile: ProfileInfo {
                name: "dev".to_string(),
                checks: vec![
                    "eslint".to_string(),
                    "secret-scan".to_string(),
                    "architecture".to_string(),
                ],
                path: ".anvil/profiles/dev.yaml".to_string(),
            },
            recent_runs: vec![GateRunResult {
                timestamp: "2026-03-16T10:00:00Z".to_string(),
                passed: true,
                score: 0.95,
                checks_run: 5,
                checks_passed: 5,
                duration_ms: 2400,
            }],
            update_hint: None,
            insights_hint: None,
        }
    }

    #[test]
    fn panel_navigation() {
        let mut state = StatusState::new(sample_data());
        assert_eq!(state.focused_panel, StatusPanel::Hooks);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, StatusPanel::Profile);
        assert_eq!(state.selected_item, 0); // reset on switch

        state.handle_key(Action::PageDown);
        assert_eq!(state.focused_panel, StatusPanel::Results);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, StatusPanel::Hooks); // wraps
    }

    #[test]
    fn item_navigation_within_panel() {
        let mut state = StatusState::new(sample_data());
        // Hooks panel has 2 items
        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 1);
        state.handle_key(Action::Down); // at max
        assert_eq!(state.selected_item, 1);
        state.handle_key(Action::Up);
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn panel_switch_resets_selection() {
        let mut state = StatusState::new(sample_data());
        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 1);

        state.handle_key(Action::Right); // switch to Profile
        assert_eq!(state.selected_item, 0);
    }
}
