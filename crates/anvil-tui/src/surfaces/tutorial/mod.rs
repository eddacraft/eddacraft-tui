pub mod paths;
pub mod render;

use eddacraft_tui::keyboard::Action;

/// Available tutorial paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialPath {
    Policy,
    Architecture,
    Drift,
    CI,
}

impl TutorialPath {
    pub fn label(self) -> &'static str {
        match self {
            Self::Policy => "Policy",
            Self::Architecture => "Architecture",
            Self::Drift => "Drift",
            Self::CI => "CI Integration",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Policy => "Learn to write and test gate policies",
            Self::Architecture => "Set up architecture boundary enforcement",
            Self::Drift => "Capture and compare configuration drift snapshots",
            Self::CI => "Integrate Anvil checks into your CI pipeline",
        }
    }
}

/// Current phase of the tutorial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialPhase {
    PathSelect,
    Running,
    Complete,
}

/// A single step in a tutorial path.
#[derive(Debug, Clone)]
pub struct TutorialStep {
    pub title: String,
    pub description: String,
    pub instruction: String,
    pub completed: bool,
}

/// State for the tutorial orchestrator surface.
pub struct TutorialState {
    pub phase: TutorialPhase,
    pub paths: Vec<TutorialPath>,
    pub path_selected: usize,
    pub chosen_path: Option<TutorialPath>,
    pub steps: Vec<TutorialStep>,
    pub current_step: usize,
    pub should_quit: bool,
}

impl TutorialState {
    pub fn new() -> Self {
        Self {
            phase: TutorialPhase::PathSelect,
            paths: vec![
                TutorialPath::Policy,
                TutorialPath::Architecture,
                TutorialPath::Drift,
                TutorialPath::CI,
            ],
            path_selected: 0,
            chosen_path: None,
            steps: Vec::new(),
            current_step: 0,
            should_quit: false,
        }
    }

    pub fn surface_name(&self) -> &'static str {
        "t u t o r i a l"
    }

    pub fn help_text(&self) -> &'static str {
        match self.phase {
            TutorialPhase::PathSelect => "j/k navigate  enter select  q quit",
            TutorialPhase::Running => "enter/space next step  esc back  q quit",
            TutorialPhase::Complete => "enter choose another  q quit",
        }
    }

    pub fn load_steps(&mut self, path: TutorialPath) {
        self.steps = match path {
            TutorialPath::Policy => paths::policy_steps(),
            TutorialPath::Architecture => paths::architecture_steps(),
            TutorialPath::Drift => paths::drift_steps(),
            TutorialPath::CI => paths::ci_steps(),
        };
        self.current_step = 0;
        self.chosen_path = Some(path);
        self.phase = TutorialPhase::Running;
    }

    pub fn handle_key(&mut self, action: Action) {
        match self.phase {
            TutorialPhase::PathSelect => self.handle_path_select(action),
            TutorialPhase::Running => self.handle_running(action),
            TutorialPhase::Complete => self.handle_complete(action),
        }
    }

    fn handle_path_select(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.path_selected > 0 {
                    self.path_selected -= 1;
                }
            }
            Action::Down => {
                if self.path_selected < self.paths.len().saturating_sub(1) {
                    self.path_selected += 1;
                }
            }
            Action::Select => {
                let path = self.paths[self.path_selected];
                self.load_steps(path);
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_running(&mut self, action: Action) {
        match action {
            Action::Select | Action::Toggle => {
                if self.current_step < self.steps.len() {
                    self.steps[self.current_step].completed = true;
                    if self.current_step + 1 < self.steps.len() {
                        self.current_step += 1;
                    } else {
                        self.phase = TutorialPhase::Complete;
                    }
                }
            }
            Action::Back => {
                self.phase = TutorialPhase::PathSelect;
                self.steps.clear();
                self.current_step = 0;
                self.chosen_path = None;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_complete(&mut self, action: Action) {
        match action {
            Action::Select => {
                self.phase = TutorialPhase::PathSelect;
                self.steps.clear();
                self.current_step = 0;
                self.chosen_path = None;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

impl Default for TutorialState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_path_select() {
        let state = TutorialState::new();
        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert_eq!(state.paths.len(), 4);
        assert_eq!(state.path_selected, 0);
        assert!(state.chosen_path.is_none());
    }

    #[test]
    fn path_selection_advances_to_running() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select);
        assert_eq!(state.phase, TutorialPhase::Running);
        assert_eq!(state.chosen_path, Some(TutorialPath::Policy));
        assert!(!state.steps.is_empty());
        assert_eq!(state.current_step, 0);
    }

    #[test]
    fn path_navigation() {
        let mut state = TutorialState::new();

        state.handle_key(Action::Down);
        assert_eq!(state.path_selected, 1);

        state.handle_key(Action::Down);
        assert_eq!(state.path_selected, 2);

        state.handle_key(Action::Up);
        assert_eq!(state.path_selected, 1);

        state.handle_key(Action::Up);
        assert_eq!(state.path_selected, 0);

        state.handle_key(Action::Up); // at min
        assert_eq!(state.path_selected, 0);
    }

    #[test]
    fn step_progression() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy
        let total_steps = state.steps.len();
        assert!(total_steps > 1);

        state.handle_key(Action::Select); // advance step
        assert_eq!(state.current_step, 1);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn completing_all_steps_transitions_to_complete() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy
        let total_steps = state.steps.len();

        for _ in 0..total_steps {
            state.handle_key(Action::Select);
        }

        assert_eq!(state.phase, TutorialPhase::Complete);
    }

    #[test]
    fn back_from_running_returns_to_path_select() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy
        assert_eq!(state.phase, TutorialPhase::Running);

        state.handle_key(Action::Back);
        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(state.steps.is_empty());
        assert!(state.chosen_path.is_none());
    }

    #[test]
    fn complete_returns_to_path_select() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy
        let total_steps = state.steps.len();

        for _ in 0..total_steps {
            state.handle_key(Action::Select);
        }
        assert_eq!(state.phase, TutorialPhase::Complete);

        state.handle_key(Action::Select); // return to path select
        assert_eq!(state.phase, TutorialPhase::PathSelect);
    }

    #[test]
    fn select_different_paths() {
        let mut state = TutorialState::new();

        // Select Architecture (index 1)
        state.handle_key(Action::Down);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen_path, Some(TutorialPath::Architecture));

        // Back and select Drift (index 2) — path_selected was 1 from before
        state.handle_key(Action::Back);
        state.handle_key(Action::Down);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen_path, Some(TutorialPath::Drift));
    }

    #[test]
    fn quit_from_any_phase() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);

        let mut state = TutorialState::new();
        state.handle_key(Action::Select);
        state.should_quit = false;
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn path_labels() {
        assert_eq!(TutorialPath::Policy.label(), "Policy");
        assert_eq!(TutorialPath::Architecture.label(), "Architecture");
        assert_eq!(TutorialPath::Drift.label(), "Drift");
        assert_eq!(TutorialPath::CI.label(), "CI Integration");
    }
}
