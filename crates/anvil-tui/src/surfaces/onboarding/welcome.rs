use eddacraft_tui::keyboard::Action;

/// The three choices available on the onboarding welcome screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingChoice {
    /// Configure anvil for your project step by step.
    GuidedSetup,
    /// Jump straight into the interactive tutorial.
    SkipToTutorial,
    /// Create the first-run marker and exit to the standard welcome menu.
    SkipEntirely,
}

impl OnboardingChoice {
    pub const ALL: [Self; 3] = [Self::GuidedSetup, Self::SkipToTutorial, Self::SkipEntirely];

    pub fn label(self) -> &'static str {
        match self {
            Self::GuidedSetup => "Set up this project",
            Self::SkipToTutorial => "Explore the tutorial",
            Self::SkipEntirely => "Go to command menu",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::GuidedSetup => "Add anvil to your codebase and scan for issues",
            Self::SkipToTutorial => "Learn what anvil can do with a guided walkthrough",
            Self::SkipEntirely => "Skip setup \u{2014} you can always come back with `anvil start`",
        }
    }
}

/// State for the first-run onboarding welcome screen.
pub struct OnboardingWelcomeState {
    pub selected: usize,
    pub should_quit: bool,
    pub chosen: Option<OnboardingChoice>,
}

impl OnboardingWelcomeState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            should_quit: false,
            chosen: None,
        }
    }

    pub fn handle_key(&mut self, action: Action) {
        if self.chosen.is_some() || self.should_quit {
            return;
        }
        match action {
            Action::Up if self.selected > 0 => {
                self.selected -= 1;
            }
            Action::Down if self.selected < OnboardingChoice::ALL.len() - 1 => {
                self.selected += 1;
            }
            Action::Select => {
                self.chosen = Some(OnboardingChoice::ALL[self.selected]);
            }
            Action::Back | Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

impl crate::surface::Surface for OnboardingWelcomeState {
    fn surface_name(&self) -> &'static str {
        "Onboarding"
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
        super::welcome_render::render(frame, area, self, theme);
    }
}

impl Default for OnboardingWelcomeState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let state = OnboardingWelcomeState::new();
        assert_eq!(state.selected, 0);
        assert!(!state.should_quit);
        assert!(state.chosen.is_none());
    }

    #[test]
    fn navigate_down_and_up() {
        let mut state = OnboardingWelcomeState::new();
        state.handle_key(Action::Down);
        assert_eq!(state.selected, 1);
        state.handle_key(Action::Down);
        assert_eq!(state.selected, 2);
        state.handle_key(Action::Up);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn does_not_go_below_zero() {
        let mut state = OnboardingWelcomeState::new();
        state.handle_key(Action::Up);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn does_not_exceed_max() {
        let mut state = OnboardingWelcomeState::new();
        for _ in 0..10 {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.selected, OnboardingChoice::ALL.len() - 1);
    }

    #[test]
    fn select_sets_chosen_guided_setup() {
        let mut state = OnboardingWelcomeState::new();
        state.handle_key(Action::Select);
        assert_eq!(state.chosen, Some(OnboardingChoice::GuidedSetup));
    }

    #[test]
    fn select_sets_chosen_skip_to_tutorial() {
        let mut state = OnboardingWelcomeState::new();
        state.handle_key(Action::Down);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen, Some(OnboardingChoice::SkipToTutorial));
    }

    #[test]
    fn select_sets_chosen_skip_entirely() {
        let mut state = OnboardingWelcomeState::new();
        state.handle_key(Action::Down);
        state.handle_key(Action::Down);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen, Some(OnboardingChoice::SkipEntirely));
    }

    #[test]
    fn should_quit_when_chosen_is_some() {
        let mut state = OnboardingWelcomeState::new();
        assert!(!crate::surface::Surface::should_quit(&state));
        state.handle_key(Action::Select);
        assert!(crate::surface::Surface::should_quit(&state));
    }

    #[test]
    fn quit_sets_flag() {
        let mut state = OnboardingWelcomeState::new();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
        assert!(crate::surface::Surface::should_quit(&state));
    }

    #[test]
    fn back_quits() {
        let mut state = OnboardingWelcomeState::new();
        state.handle_key(Action::Back);
        assert!(state.should_quit);
    }

    #[test]
    fn default_matches_new() {
        let a = OnboardingWelcomeState::new();
        let b = OnboardingWelcomeState::default();
        assert_eq!(a.selected, b.selected);
        assert_eq!(a.should_quit, b.should_quit);
        assert_eq!(a.chosen, b.chosen);
    }

    #[test]
    fn choice_labels() {
        assert_eq!(OnboardingChoice::GuidedSetup.label(), "Set up this project");
        assert_eq!(
            OnboardingChoice::SkipToTutorial.label(),
            "Explore the tutorial"
        );
        assert_eq!(OnboardingChoice::SkipEntirely.label(), "Go to command menu");
    }

    #[test]
    fn choice_descriptions() {
        assert_eq!(
            OnboardingChoice::GuidedSetup.description(),
            "Add anvil to your codebase and scan for issues"
        );
        assert_eq!(
            OnboardingChoice::SkipToTutorial.description(),
            "Learn what anvil can do with a guided walkthrough"
        );
        assert_eq!(
            OnboardingChoice::SkipEntirely.description(),
            "Skip setup \u{2014} you can always come back with `anvil start`"
        );
    }
}
