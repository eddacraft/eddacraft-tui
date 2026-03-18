pub mod render;

use eddacraft_tui::prelude::*;

/// Quick-start options shown on the welcome screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickStartOption {
    RunTutorial,
    RunAudit,
    RunDoctor,
    ViewDocs,
    // Not yet implemented — uncomment as each command ships:
    // RunGate,
    // StartWatch,
}

impl QuickStartOption {
    pub const ALL: [Self; 4] = [
        Self::RunTutorial,
        Self::RunAudit,
        Self::RunDoctor,
        Self::ViewDocs,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::RunTutorial => "Run interactive tutorial",
            Self::RunAudit => "Run project audit",
            Self::RunDoctor => "Run diagnostics",
            Self::ViewDocs => "View documentation",
            // Self::RunGate => "Run gate checks",
            // Self::StartWatch => "Start watch mode",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::RunTutorial => "Learn Anvil with a guided walkthrough",
            Self::RunAudit => "Scan your project for security issues and anti-patterns",
            Self::RunDoctor => "Check your environment and fix common issues",
            Self::ViewDocs => "Open the Anvil documentation in your browser",
            // Self::RunGate => "Check your project against configured quality gates",
            // Self::StartWatch => "Monitor files and run checks on every save",
        }
    }
}

/// State for the welcome surface.
pub struct WelcomeState {
    pub selected: usize,
    pub should_quit: bool,
    pub chosen: Option<QuickStartOption>,
}

impl WelcomeState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            should_quit: false,
            chosen: None,
        }
    }

    pub fn surface_name(&self) -> &'static str {
        "w e l c o m e"
    }

    pub fn help_text(&self) -> &'static str {
        "j/k navigate  enter select  q quit"
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            Action::Down => {
                if self.selected < QuickStartOption::ALL.len() - 1 {
                    self.selected += 1;
                }
            }
            Action::Select => {
                self.chosen = Some(QuickStartOption::ALL[self.selected]);
            }
            Action::Quit => {
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
        "j/k navigate  enter select  q quit"
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
        state.handle_key(Action::Down); // RunAudit
        state.handle_key(Action::Select);
        assert_eq!(state.chosen, Some(QuickStartOption::RunAudit));
    }

    #[test]
    fn quit_sets_flag() {
        let mut state = WelcomeState::new();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }
}
