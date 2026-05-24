pub mod event_adapter;
pub mod render;

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::surface::Surface;

#[derive(Debug, Clone)]
pub struct PlanDashboardSnapshot {
    pub modules: Vec<PlanModuleRow>,
    pub work_items: Vec<PlanWorkItemRow>,
    pub warnings: Vec<PlanWarningRow>,
    pub branch: Option<String>,
    pub sha: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlanModuleRow {
    pub scope: String,
    pub progress: String,
    pub status: String,
    pub note: String,
    pub has_warning: bool,
}

#[derive(Debug, Clone)]
pub struct PlanWorkItemRow {
    pub id: String,
    pub title: String,
    pub module: String,
    pub status: String,
    pub validation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlanWarningRow {
    pub target: String,
    pub module: Option<String>,
    pub message: String,
}

#[allow(clippy::struct_excessive_bools)]
pub struct PlanDashboardState {
    pub snapshot: PlanDashboardSnapshot,
    pub selected_module: usize,
    pub show_help: bool,
    pub show_detail: bool,
    pub filter_mode: bool,
    pub filter_query: String,
    pub rescan_requested: bool,
    pub should_quit: bool,
}

impl PlanDashboardState {
    pub fn new(snapshot: PlanDashboardSnapshot) -> Self {
        Self {
            snapshot,
            selected_module: 0,
            show_help: false,
            show_detail: false,
            filter_mode: false,
            filter_query: String::new(),
            rescan_requested: false,
            should_quit: false,
        }
    }

    pub fn visible_modules(&self) -> Vec<(usize, &PlanModuleRow)> {
        let query = self.filter_query.to_ascii_lowercase();
        self.snapshot
            .modules
            .iter()
            .enumerate()
            .filter(|(_, module)| {
                query.is_empty()
                    || module.scope.to_ascii_lowercase().contains(&query)
                    || module.status.to_ascii_lowercase().contains(&query)
                    || module.note.to_ascii_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn selected_module_scope(&self) -> Option<&str> {
        self.visible_modules()
            .get(self.selected_module)
            .map(|(_, module)| module.scope.as_str())
    }
}

impl Surface for PlanDashboardState {
    fn surface_name(&self) -> &'static str {
        "Plan Dashboard"
    }

    fn help_text(&self) -> &'static str {
        "j/k navigate  enter details  / filter  r rescan  esc/q back/quit  ? help"
    }

    fn handle_key(&mut self, action: Action) {
        self.handle_dashboard_action(action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        render::render(frame, area, self, theme);
    }
}

#[cfg(test)]
pub(crate) fn sample_state() -> PlanDashboardState {
    PlanDashboardState::new(PlanDashboardSnapshot {
        modules: vec![
            PlanModuleRow {
                scope: "DOCGOV".to_string(),
                progress: "8/10".to_string(),
                status: "In Progress".to_string(),
                note: "metadata backfill remains".to_string(),
                has_warning: true,
            },
            PlanModuleRow {
                scope: "APSCAN".to_string(),
                progress: "1/11".to_string(),
                status: "In Progress".to_string(),
                note: "APS dashboard delivery".to_string(),
                has_warning: false,
            },
        ],
        work_items: vec![PlanWorkItemRow {
            id: "APSCAN-011".to_string(),
            title: "Add APS TUI dashboard".to_string(),
            module: "APSCAN".to_string(),
            status: "Ready".to_string(),
            validation: Some("cargo test -p eddacraft-anvil plan_dashboard".to_string()),
        }],
        warnings: vec![PlanWarningRow {
            target: "DOCGOV".to_string(),
            module: Some("DOCGOV".to_string()),
            message: "needs reconcile".to_string(),
        }],
        branch: Some("feat/apscan-aps-tui-dashboard".to_string()),
        sha: Some("14a40a78".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_requests_exit() {
        let mut state = sample_state();

        state.handle_key(Action::Quit);

        assert!(state.should_quit());
    }

    #[test]
    fn navigation_wraps_module_selection() {
        let mut state = sample_state();

        state.handle_key(Action::Up);
        assert_eq!(state.selected_module, 1);

        state.handle_key(Action::Down);
        assert_eq!(state.selected_module, 0);
    }

    #[test]
    fn filter_limits_visible_modules() {
        let mut state = sample_state();

        state.handle_key(Action::Character('/'));
        state.handle_key(Action::Character('a'));
        state.handle_key(Action::Character('p'));

        assert!(state.filter_mode);
        assert_eq!(state.filter_query, "ap");
        assert_eq!(state.visible_modules().len(), 1);
        assert_eq!(state.selected_module_scope(), Some("APSCAN"));
    }

    #[test]
    fn enter_toggles_detail() {
        let mut state = sample_state();

        state.handle_key(Action::Select);

        assert!(state.show_detail);
    }

    #[test]
    fn r_requests_rescan() {
        let mut state = sample_state();

        state.handle_key(Action::Character('r'));

        assert!(state.rescan_requested);
        assert!(state.should_quit());
    }

    #[test]
    fn filter_mode_allows_q_search_text() {
        let mut state = sample_state();

        state.handle_key(Action::Character('/'));
        state.handle_key(Action::Quit);

        assert_eq!(state.filter_query, "q");
        assert!(!state.should_quit());
    }

    #[test]
    fn selected_detail_uses_exact_module_not_prefix() {
        let mut state = sample_state();
        state.snapshot.modules.insert(
            0,
            PlanModuleRow {
                scope: "APS".to_string(),
                progress: "0/1".to_string(),
                status: "In Progress".to_string(),
                note: "prefix collision".to_string(),
                has_warning: false,
            },
        );
        state.snapshot.work_items.push(PlanWorkItemRow {
            id: "APSCAN-999".to_string(),
            title: "Should not appear for APS".to_string(),
            module: "APSCAN".to_string(),
            status: "Ready".to_string(),
            validation: None,
        });

        state.selected_module = 0;

        let selected = state.selected_module_scope().unwrap();
        let visible: Vec<_> = state
            .snapshot
            .work_items
            .iter()
            .filter(|item| item.module == selected)
            .collect();

        assert!(visible.is_empty());
    }
}
