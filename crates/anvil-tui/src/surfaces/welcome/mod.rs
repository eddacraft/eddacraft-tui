pub mod render;

use eddacraft_tui::prelude::*;

/// Quick-start options shown on the welcome screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickStartOption {
    RunAudit,
    RunDoctor,
    RunGate,
    StartWatch,
    RunTutorial,
    ViewDocs,
    RestartOnboarding,
}

impl QuickStartOption {
    pub const ALL: [Self; 7] = [
        Self::RunGate,
        Self::StartWatch,
        Self::RunAudit,
        Self::RunDoctor,
        Self::RunTutorial,
        Self::ViewDocs,
        Self::RestartOnboarding,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::RunGate => "Review gate decision",
            Self::StartWatch => "Watch checks live",
            Self::RunAudit => "Explore project findings",
            Self::RunDoctor => "Check setup health",
            Self::RunTutorial => "Learn the anvil model",
            Self::ViewDocs => "View documentation",
            Self::RestartOnboarding => "Restart onboarding",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::RunGate => "See whether the current findings pass your workflow gate",
            Self::StartWatch => "Run checks continuously and watch findings update on save",
            Self::RunAudit => "Inspect the findings anvil collects across your project",
            Self::RunDoctor => "Verify your environment before relying on checks and gates",
            Self::RunTutorial => "Start with scan -> checks -> findings -> gate, then pick a path",
            Self::ViewDocs => "Open the anvil documentation in your browser",
            Self::RestartOnboarding => "Reset and re-run the first-time setup experience",
        }
    }
}

/// State for the welcome surface.
pub struct WelcomeState {
    pub selected: usize,
    pub should_quit: bool,
    pub chosen: Option<QuickStartOption>,
    /// Transient status message shown below the menu.
    pub status_message: Option<String>,
}

impl WelcomeState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            should_quit: false,
            chosen: None,
            status_message: None,
        }
    }

    pub fn surface_name(&self) -> &'static str {
        "w e l c o m e"
    }

    pub fn help_text(&self) -> &'static str {
        "j/k navigate  enter select  esc/q quit"
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up if self.selected > 0 => {
                self.selected -= 1;
            }
            Action::Down if self.selected < QuickStartOption::ALL.len() - 1 => {
                self.selected += 1;
            }
            Action::Select => {
                self.chosen = Some(QuickStartOption::ALL[self.selected]);
            }
            Action::Back | Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

impl crate::surface::Surface for WelcomeState {
    fn surface_name(&self) -> &'static str {
        "Welcome"
    }

    fn help_text(&self) -> &'static str {
        "j/k navigate  enter select  esc/q quit"
    }

    fn handle_key(&mut self, action: Action) {
        self.handle_key(action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit || self.chosen.is_some()
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

impl Default for WelcomeState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let state = WelcomeState::new();
        assert_eq!(state.selected, 0);
        assert!(!state.should_quit);
        assert!(state.chosen.is_none());
    }

    #[test]
    fn navigate_down_and_up() {
        let mut state = WelcomeState::new();
        state.handle_key(Action::Down);
        assert_eq!(state.selected, 1);
        state.handle_key(Action::Down);
        assert_eq!(state.selected, 2);
        state.handle_key(Action::Up);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn does_not_go_below_zero() {
        let mut state = WelcomeState::new();
        state.handle_key(Action::Up);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn does_not_exceed_max() {
        let mut state = WelcomeState::new();
        for _ in 0..10 {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.selected, QuickStartOption::ALL.len() - 1);
    }

    #[test]
    fn select_sets_chosen() {
        let mut state = WelcomeState::new();
        // Default is RunGate (index 0); Down moves to StartWatch (index 1)
        state.handle_key(Action::Down);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen, Some(QuickStartOption::StartWatch));
    }

    #[test]
    fn quit_sets_flag() {
        let mut state = WelcomeState::new();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn quick_start_copy_uses_model_first_language() {
        assert_eq!(QuickStartOption::RunGate.label(), "Review gate decision");
        assert!(QuickStartOption::RunGate.description().contains("findings"));
        assert!(
            QuickStartOption::RunTutorial
                .description()
                .contains("scan -> checks -> findings -> gate")
        );
    }
}
