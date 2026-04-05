pub mod discovery;
mod discovery_render;
pub(crate) mod executor;
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

/// Output captured after running a step's command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub exit_code: Option<i32>,
}

/// A single step in a tutorial path.
#[derive(Debug, Clone)]
pub struct TutorialStep {
    pub title: String,
    pub description: String,
    pub instruction: String,
    /// Optional shell command to execute when the user presses Enter.
    pub command: Option<String>,
    pub completed: bool,
    /// Captured output from the last execution of `command`.
    pub output: Option<CommandOutput>,
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
    pub wants_back: bool,
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
            wants_back: false,
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
                if let Some(&path) = self.paths.get(self.path_selected) {
                    self.load_steps(path);
                }
            }
            Action::Back => self.wants_back = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn advance_step(&mut self) {
        if self.current_step < self.steps.len() {
            self.steps[self.current_step].completed = true;
            if self.current_step + 1 < self.steps.len() {
                self.current_step += 1;
            } else {
                self.phase = TutorialPhase::Complete;
            }
        }
    }

    /// Returns true if the current step has failed command output waiting for retry/skip.
    pub fn current_step_failed(&self) -> bool {
        self.steps
            .get(self.current_step)
            .and_then(|s| s.output.as_ref())
            .is_some_and(|o| !o.success)
    }

    fn handle_running(&mut self, action: Action) {
        // When a command has failed, only retry (r → Char('r')) and skip (s → Char('s'))
        // are active; everything else is ignored except Back and Quit.
        if self.current_step_failed() {
            match action {
                // 'r' — clear output and re-execute the command
                Action::Character('r') => {
                    if let Some(step) = self.steps.get_mut(self.current_step) {
                        step.output = None;
                        if let Some(cmd) = step.command.clone() {
                            let result = executor::execute_command(&cmd);
                            let succeeded = result.success;
                            step.output = Some(result);
                            if succeeded {
                                self.advance_step();
                            }
                        }
                    }
                }
                // 's' — skip: mark complete and advance without re-running
                Action::Character('s') => {
                    self.advance_step();
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
            return;
        }

        match action {
            Action::Select => {
                if let Some(step) = self.steps.get(self.current_step) {
                    if let Some(cmd) = step.command.clone() {
                        // Execute the command and store the output.
                        let result = executor::execute_command(&cmd);
                        let succeeded = result.success;
                        self.steps[self.current_step].output = Some(result);
                        if succeeded {
                            self.advance_step();
                        }
                        // On failure we stay on the same step; retry/skip prompt shown.
                    } else {
                        // No command — informational step, advance immediately.
                        self.advance_step();
                    }
                }
            }
            Action::Toggle => {
                // Toggle (space) only advances informational steps — it does not
                // execute commands, preventing accidental shell invocation.
                if let Some(step) = self.steps.get(self.current_step)
                    && step.command.is_none()
                {
                    self.advance_step();
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
            Action::Select | Action::Back => {
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

impl crate::surface::Surface for TutorialState {
    fn surface_name(&self) -> &'static str {
        "Tutorial"
    }

    fn help_text(&self) -> &'static str {
        match self.phase {
            TutorialPhase::PathSelect => "j/k navigate  enter select  esc back  q quit",
            TutorialPhase::Running => {
                if self.current_step_failed() {
                    "r retry  s skip  esc back  q quit"
                } else {
                    "enter run/next  space next  esc back  q quit"
                }
            }
            TutorialPhase::Complete => "enter choose another  esc back  q quit",
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
        self.phase = TutorialPhase::PathSelect;
        self.path_selected = 0;
        self.steps.clear();
        self.current_step = 0;
        self.chosen_path = None;
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

impl Default for TutorialState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a state pre-loaded with informational-only steps (no commands) so
    /// state-machine tests do not accidentally invoke real processes.
    fn state_with_plain_steps(count: usize) -> TutorialState {
        let mut state = TutorialState::new();
        state.steps = (0..count)
            .map(|i| TutorialStep {
                title: format!("Step {i}"),
                description: format!("Description {i}"),
                instruction: format!("Press enter to continue ({i})."),
                command: None,
                completed: false,
                output: None,
            })
            .collect();
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

    /// Build a state pre-loaded with a single step that has a given command.
    fn state_with_command_step(command: &str) -> TutorialState {
        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Cmd Step".to_string(),
            description: "A step with a command.".to_string(),
            instruction: format!("Run: {command}"),
            command: Some(command.to_string()),
            completed: false,
            output: None,
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

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
    fn step_progression_informational() {
        // Policy step 0 is "Introduction" — no command — so Select advances it.
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy
        let total_steps = state.steps.len();
        assert!(total_steps > 1);

        state.handle_key(Action::Select); // advance informational step 0
        assert_eq!(state.current_step, 1);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn completing_all_plain_steps_transitions_to_complete() {
        let total = 4;
        let mut state = state_with_plain_steps(total);

        for _ in 0..total {
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
        let total = 3;
        let mut state = state_with_plain_steps(total);

        for _ in 0..total {
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

    // --- Command execution tests ---

    #[test]
    fn successful_command_step_advances() {
        let mut state = state_with_command_step("echo hello");
        assert_eq!(state.current_step, 0);

        state.handle_key(Action::Select);

        // Command succeeds — step is completed and phase moves to Complete
        // (only one step in this state)
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        let output = state.steps[0]
            .output
            .as_ref()
            .expect("output should be present");
        assert!(output.success);
    }

    #[test]
    fn failed_command_step_stays_on_step() {
        // Use a command that will always fail with exit code 1.
        let mut state = state_with_command_step("exit 1");

        state.handle_key(Action::Select);

        // Command fails — step stays current and is not completed.
        assert_eq!(state.current_step, 0);
        assert!(!state.steps[0].completed);
        assert_eq!(state.phase, TutorialPhase::Running);
        let output = state.steps[0]
            .output
            .as_ref()
            .expect("output should be present");
        assert!(!output.success);
    }

    #[test]
    fn failed_command_help_text_shows_retry_skip() {
        let mut state = state_with_command_step("exit 1");
        state.handle_key(Action::Select); // executes and fails

        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert_eq!(help, "r retry  s skip  esc back  q quit");
    }

    #[test]
    fn retry_after_failure_re_executes_command() {
        // First run fails; second run succeeds because we swap to "echo ok".
        // We can't swap the command at runtime, so test retry with a succeeding command:
        // Use a command that fails first time... but since we can't vary behaviour
        // per call, test that retry with a succeeding command advances.
        // This verifies the retry path clears output and re-executes.
        let mut state = state_with_command_step("echo retry_test");

        // Simulate failure by injecting failed output directly.
        state.steps[0].output = Some(CommandOutput {
            stdout: String::new(),
            stderr: "simulated failure".to_string(),
            success: false,
            exit_code: Some(1),
        });

        // Verify we're in the "failed" state.
        assert!(state.current_step_failed());

        // Press 'r' to retry — the actual command is "echo retry_test" which succeeds.
        state.handle_key(Action::Character('r'));

        // Should advance past the step.
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn skip_after_failure_advances_without_re_running() {
        let mut state = state_with_command_step("exit 1");
        state.handle_key(Action::Select); // fails

        assert!(state.current_step_failed());

        state.handle_key(Action::Character('s')); // skip

        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn back_from_failed_command_returns_to_path_select() {
        let mut state = state_with_command_step("exit 1");
        state.handle_key(Action::Select); // fails

        state.handle_key(Action::Back);

        assert_eq!(state.phase, TutorialPhase::PathSelect);
    }

    #[test]
    fn toggle_does_not_execute_command_step() {
        let mut state = state_with_command_step("echo should_not_run");
        state.handle_key(Action::Toggle);

        // Toggle should be ignored for command steps — no output, no advance.
        assert_eq!(state.current_step, 0);
        assert!(!state.steps[0].completed);
        assert!(state.steps[0].output.is_none());
    }

    #[test]
    fn toggle_advances_informational_step() {
        let mut state = state_with_plain_steps(2);
        state.handle_key(Action::Toggle);

        assert_eq!(state.current_step, 1);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn informational_step_advances_without_executing() {
        let mut state = state_with_plain_steps(2);
        state.handle_key(Action::Select);

        assert_eq!(state.current_step, 1);
        assert!(state.steps[0].completed);
        assert!(state.steps[0].output.is_none());
    }
}
