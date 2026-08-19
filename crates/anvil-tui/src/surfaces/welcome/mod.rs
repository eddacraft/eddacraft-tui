pub mod render;

use eddacraft_tui::prelude::*;

/// Quick-start options shown on the welcome screen.
///
/// CIB-246: the hub and the tutorial path picker are two menus over one
/// product, so they must not coin two names for the same thing. Only
/// [`Self::RunTutorial`] crosses into tutorial vocabulary — and it opens the
/// picker rather than any single path, so it is named after
/// [`crate::surfaces::tutorial::PATH_PICKER_TITLE`] rather than after a path.
/// The other options run commands (gate, watch, audit, doctor, docs) that the
/// tutorial has no name for, so a shared hub/path catalogue would force them
/// into a taxonomy they don't belong to; the one crossing point is pinned by
/// `tutorial_entry_is_named_after_the_path_picker` instead.
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
            Self::RunTutorial => "Choose a learning path",
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
            // CIB-351: the labelled item opens the picker, so the description
            // must not promise an intervening scan. It still names the loop
            // the paths teach, and stays short enough for an 80-column row
            // (CIB-246).
            Self::RunTutorial => "Walk through checks, findings, and the gate",
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
        let tutorial = QuickStartOption::RunTutorial.description();
        assert!(
            !tutorial.contains("Start with scan"),
            "hub tutorial entry must not promise an intervening scan: {tutorial}"
        );
        assert!(
            tutorial.contains("Walk through checks, findings, and the gate"),
            "hub tutorial entry should still name the loop the paths teach: {tutorial}"
        );
    }

    /// CIB-246: the hub entry that opens the tutorial must be named after the
    /// picker it opens — not after one of the paths inside it, and not with a
    /// third name of its own ("Learn the anvil model" was exactly that). Both
    /// halves are pinned so either surface drifting alone fails here instead
    /// of in the user's head on their second `anvil welcome`.
    #[test]
    fn tutorial_entry_is_named_after_the_path_picker() {
        use crate::surfaces::tutorial::{PATH_PICKER_TITLE, TutorialPath};

        let label = QuickStartOption::RunTutorial.label();
        assert!(
            label.eq_ignore_ascii_case(PATH_PICKER_TITLE),
            "hub tutorial entry {label:?} must name the picker it opens \
             ({PATH_PICKER_TITLE:?})",
        );
        assert!(
            TutorialPath::from_label(label).is_none(),
            "hub tutorial entry opens the picker, so it must not shadow a \
             single path label ({label:?})",
        );
    }
}
