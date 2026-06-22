pub mod discovery;
mod discovery_render;
pub(crate) mod executor;
pub mod fix;
mod fix_render;
pub mod paths;
pub mod render;
pub mod showcase;
pub mod verify;
pub mod watch_demo;
pub mod watch_demo_render;

use anvil_kernel_types::{Notification, NotificationClass, NotificationPriority};
use discovery::{FindingSeverity, ScanResults};
use eddacraft_tui::keyboard::Action;
use verify::{Verify, VerifyResult};

use crate::surfaces::fix_request::FixRequest;
use crate::surfaces::notifications::{NotificationSource, surface_notification};

/// Notice rendered when the file watcher can't be started and the tutorial
/// falls back to static mode. Shared between `anvil tutorial` and
/// `anvil welcome` so both entry points surface the same cause.
pub const STATIC_MODE_WATCHER_UNAVAILABLE: &str =
    "Live file watcher unavailable \u{2014} file saves won't retrigger checks.";

/// Available tutorial paths.
///
/// LAUNCH-014 introduced [`TutorialPath::ProtectionLoop`] as the
/// default first path: a short repo-local value walk that explains
/// anvil's protection loop, simulates a high-signal check on safe
/// fixture content, and points the user at `anvil start --verify` as
/// the next step. The remaining four paths are the deeper-learning
/// track for users who want the full taxonomy walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialPath {
    /// LAUNCH-014: the value-first default. Demonstrates the loop in
    /// 60 seconds without claiming pre-write protection.
    ProtectionLoop,
    Policy,
    Architecture,
    Drift,
    CI,
}

impl TutorialPath {
    pub fn label(self) -> &'static str {
        match self {
            Self::ProtectionLoop => "anvil's protection loop",
            Self::Policy => "Policy checks",
            Self::Architecture => "Boundary findings",
            Self::Drift => "Configuration drift",
            Self::CI => "CI gate integration",
        }
    }

    pub fn from_label(s: &str) -> Option<Self> {
        // Legacy labels ("Policy", "Architecture", "Drift", "CI Integration")
        // are kept so progress files written by older builds still round-trip
        // into the correct enum variant after the onboarding rename.
        // LAUNCH-014's "anvil's protection loop" is new — no legacy
        // alias is required.
        match s {
            "anvil's protection loop" => Some(Self::ProtectionLoop),
            "Policy checks" | "Policy" => Some(Self::Policy),
            "Boundary findings" | "Architecture" => Some(Self::Architecture),
            "Configuration drift" | "Drift" => Some(Self::Drift),
            "CI gate integration" | "CI Integration" => Some(Self::CI),
            _ => None,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::ProtectionLoop => {
                "60-second walk: see what anvil checks, then activate in this repo"
            }
            Self::Policy => "Define checks that produce findings and influence the gate",
            Self::Architecture => "See how boundary checks turn imports into actionable findings",
            Self::Drift => "Capture state changes and review the findings between snapshots",
            Self::CI => "Carry checks, findings, and gate outcomes into your delivery workflow",
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
    /// Optional verification check to run after command execution.
    pub verify: Option<Verify>,
    /// Result of the last verification check.
    pub verify_result: Option<VerifyResult>,
    /// Contextual hint shown when verification fails.
    pub verify_hint: Option<String>,
    /// Optional filesystem path to watch for changes. When set and a
    /// file watcher is available, changes to this path (or files within
    /// it) trigger automatic re-verification without pressing Enter.
    pub watch_path: Option<String>,
    /// When true, pressing Enter on this step triggers the watch mode
    /// demo instead of normal advancement. The TUI loop exits and the
    /// CLI command launches the demo surface.
    pub watch_demo: bool,
}

/// State for the tutorial orchestrator surface.
#[allow(clippy::struct_excessive_bools)]
pub struct TutorialState {
    pub phase: TutorialPhase,
    pub paths: Vec<TutorialPath>,
    pub path_selected: usize,
    pub chosen_path: Option<TutorialPath>,
    pub steps: Vec<TutorialStep>,
    pub current_step: usize,
    pub should_quit: bool,
    pub wants_back: bool,
    /// Scan results from the discovery phase, threaded through to tutorials.
    pub scan_results: Option<ScanResults>,
    /// Findings filtered by the chosen tutorial domain.
    pub domain_findings: Option<ScanResults>,
    /// When true, command execution is disabled and all steps become
    /// informational (press-enter-to-continue). Set by the caller when the
    /// kernel watcher is unavailable.
    pub static_mode: bool,
    /// Notice displayed when static mode is active, explaining why interactive
    /// features are disabled.
    pub static_notice: Option<String>,
    /// Paths the user has previously completed (persisted across sessions).
    /// Used by the renderer to show checkmarks in the path selector.
    pub completed_paths: Vec<TutorialPath>,
    /// Transient notice shown when resuming an interrupted session.
    pub resuming_notice: Option<String>,
    /// Set to true when the tutorial wants to launch the watch mode demo.
    /// The TUI loop exits and the CLI command handles the transition.
    pub wants_watch_demo: bool,
    /// Pending fix request emitted when the user presses `f`.
    pub pending_fix: Option<FixRequest>,
}

impl TutorialState {
    pub fn new() -> Self {
        Self {
            phase: TutorialPhase::PathSelect,
            // LAUNCH-014: the value-first ProtectionLoop path is
            // listed first AND pre-selected so the default Enter
            // press lands the user on the concrete first-win walk
            // rather than the deeper-learning taxonomy paths.
            paths: vec![
                TutorialPath::ProtectionLoop,
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
            scan_results: None,
            domain_findings: None,
            static_mode: false,
            static_notice: None,
            completed_paths: Vec::new(),
            resuming_notice: None,
            wants_watch_demo: false,
            pending_fix: None,
        }
    }

    /// Enable static mode, disabling command execution and showing a notice.
    /// All steps become informational (press-enter-to-continue) regardless of
    /// whether they have a `command` attached.
    pub fn enable_static_mode(&mut self) {
        self.static_mode = true;
        self.static_notice =
            Some("Interactive mode unavailable \u{2014} showing guided walkthrough.".to_string());
    }

    /// Enable static mode with a caller-supplied notice so the user sees the
    /// specific cause (e.g. watcher failed) instead of the generic fallback.
    pub fn enable_static_mode_with_reason(&mut self, reason: impl Into<String>) {
        self.static_mode = true;
        self.static_notice = Some(reason.into());
    }

    /// Set which paths the user has previously completed (loaded from
    /// persistent progress file). The renderer uses this to show checkmarks.
    pub fn set_completed_paths(&mut self, paths: Vec<TutorialPath>) {
        self.completed_paths = paths;
    }

    /// Resume an interrupted session: load the path's steps and jump to
    /// `step_index`, marking earlier steps as completed per `steps_completed`.
    /// If the saved step count doesn't match the current path definition
    /// (e.g. after a tool upgrade), the stale session is discarded and the
    /// path starts fresh.
    pub fn resume_path(&mut self, path: TutorialPath, step_index: usize, steps_completed: &[bool]) {
        self.load_steps(path);
        // Stale session: step count changed since the session was saved.
        if steps_completed.len() != self.steps.len() {
            return;
        }
        for (i, step) in self.steps.iter_mut().enumerate() {
            if steps_completed.get(i).copied().unwrap_or(false) {
                step.completed = true;
            }
        }
        self.current_step = step_index.min(self.steps.len().saturating_sub(1));
        self.resuming_notice = Some(format!(
            "Resuming from step {} of {}.",
            self.current_step + 1,
            self.steps.len(),
        ));
    }

    pub fn set_scan_results(&mut self, results: ScanResults) {
        self.scan_results = Some(results);
    }

    fn next_fix_request(&self) -> Option<FixRequest> {
        let mut best: Option<(FindingSeverity, FixRequest)> = None;
        for finding in &self.domain_findings.as_ref()?.findings {
            let Some(request) = finding.fix_request() else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(severity, _)| finding.severity > *severity)
            {
                best = Some((finding.severity, request));
            }
        }
        best.map(|(_, request)| request)
    }

    pub fn load_steps(&mut self, path: TutorialPath) {
        self.steps = match path {
            TutorialPath::ProtectionLoop => paths::protection_loop_steps(),
            TutorialPath::Policy => paths::policy_steps(),
            TutorialPath::Architecture => paths::architecture_steps(),
            TutorialPath::Drift => paths::drift_steps(),
            TutorialPath::CI => paths::ci_steps(),
        };
        self.current_step = 0;
        self.chosen_path = Some(path);
        self.domain_findings = self.scan_results.as_ref().map(|r| r.filter_by_domain(path));
        self.phase = TutorialPhase::Running;
    }

    /// Called by the TUI loop when the file watcher detects changes.
    /// If the current step has a `watch_path` and verification, re-runs
    /// the verify check (and optionally the command). Returns `true` if
    /// the step was auto-advanced.
    pub fn handle_file_change(&mut self, changed_paths: &[std::path::PathBuf]) -> bool {
        if self.phase != TutorialPhase::Running || self.static_mode {
            return false;
        }
        let Some(step) = self.steps.get(self.current_step) else {
            return false;
        };
        let Some(ref watch_target) = step.watch_path else {
            return false;
        };
        // Skip if the step already completed or hasn't been attempted yet
        // when it has a command (user should press Enter first).
        if step.completed || (step.command.is_some() && step.output.is_none()) {
            return false;
        }

        // Normalise the watch target to an absolute path so it matches the
        // absolute paths emitted by the file watcher.
        let watch_target_path = std::path::PathBuf::from(watch_target);
        let target = if watch_target_path.is_absolute() {
            watch_target_path
        } else if let Ok(root) = std::env::current_dir() {
            root.join(&watch_target_path)
        } else {
            watch_target_path
        };
        let relevant = changed_paths
            .iter()
            .any(|p| p == &target || p.starts_with(&target));
        if !relevant {
            return false;
        }

        // For steps with a command, re-execute it then verify.
        // For steps without a command, verify directly (e.g. FileExists).
        if let Some(ref cmd) = step.command.clone() {
            let result = executor::execute_command(cmd);
            let success = result.success;
            self.steps[self.current_step].output = Some(result);
            if success && self.run_verify_current() {
                self.advance_step();
                return true;
            }
        } else if let Some(ref verify) = step.verify {
            // No command — verify directly with a placeholder output.
            let placeholder = CommandOutput {
                stdout: String::new(),
                stderr: String::new(),
                success: true,
                exit_code: Some(0),
            };
            let result = verify.check(&placeholder);
            let passed = result == VerifyResult::Pass;
            self.steps[self.current_step].verify_result = Some(result);
            if passed {
                self.advance_step();
                return true;
            }
        }

        false
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
            Action::Up if self.path_selected > 0 => {
                self.path_selected -= 1;
            }
            Action::Down if self.path_selected < self.paths.len().saturating_sub(1) => {
                self.path_selected += 1;
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

    pub fn advance_step(&mut self) {
        // Clear the resume notice on first interaction.
        self.resuming_notice = None;
        if self.current_step < self.steps.len() {
            self.steps[self.current_step].completed = true;
            if self.current_step + 1 < self.steps.len() {
                self.current_step += 1;
            } else {
                self.phase = TutorialPhase::Complete;
            }
        }
    }

    /// Returns true if the current step has failed command output or failed
    /// verification, waiting for retry/skip.
    pub fn current_step_failed(&self) -> bool {
        let Some(step) = self.steps.get(self.current_step) else {
            return false;
        };
        let command_failed = step.output.as_ref().is_some_and(|o| !o.success);
        let verify_failed = matches!(step.verify_result, Some(VerifyResult::Fail(_)));
        command_failed || verify_failed
    }

    /// Run the verification check for the current step against its stored
    /// output. Returns `true` if the step should advance (either no
    /// verification is configured, verification passed, or the step index
    /// is out of bounds).
    fn run_verify_current(&mut self) -> bool {
        let Some(step) = self.steps.get_mut(self.current_step) else {
            return true;
        };
        let Some(ref output) = step.output else {
            return true;
        };
        if let Some(ref verify) = step.verify {
            let result = verify.check(output);
            let passed = result == VerifyResult::Pass;
            step.verify_result = Some(result);
            passed
        } else {
            true
        }
    }

    fn handle_running(&mut self, action: Action) {
        // When a command has failed or verification has failed, only retry
        // (r) and skip (s) are active; everything else is ignored except
        // Back and Quit.
        if self.current_step_failed() {
            match action {
                // 'r' — clear output/verify state and re-execute the command
                Action::Character('r') => {
                    if let Some(step) = self.steps.get_mut(self.current_step) {
                        step.output = None;
                        step.verify_result = None;
                        if let Some(cmd) = step.command.clone() {
                            let result = executor::execute_command(&cmd);
                            let succeeded = result.success;
                            step.output = Some(result);
                            if succeeded && self.run_verify_current() {
                                self.advance_step();
                            }
                        }
                    }
                }
                // 's' — skip: mark complete and advance without re-running
                Action::Character('s') => {
                    self.advance_step();
                }
                Action::Back => self.wants_back = true,
                Action::Quit => self.should_quit = true,
                _ => {}
            }
            return;
        }

        match action {
            Action::Select => {
                if self.static_mode {
                    // Static mode: all steps are informational — advance
                    // without executing commands.
                    self.advance_step();
                } else if let Some(step) = self.steps.get(self.current_step)
                    && step.watch_demo
                {
                    // Watch demo step: signal the TUI loop to launch the demo.
                    self.wants_watch_demo = true;
                } else if let Some(step) = self.steps.get(self.current_step) {
                    if let Some(cmd) = step.command.clone() {
                        // Execute the command and store the output.
                        let result = executor::execute_command(&cmd);
                        let succeeded = result.success;
                        self.steps[self.current_step].output = Some(result);
                        if succeeded && self.run_verify_current() {
                            self.advance_step();
                        }
                        // On command failure we stay on the same step.
                    } else {
                        // No command — informational step, advance immediately.
                        self.advance_step();
                    }
                }
            }
            Action::Toggle => {
                // In static mode, Toggle (space) advances any step since
                // commands are never executed.
                if self.static_mode {
                    self.advance_step();
                } else if let Some(step) = self.steps.get(self.current_step)
                    && step.command.is_none()
                {
                    // Toggle only advances informational steps — it does not
                    // execute commands, preventing accidental shell invocation.
                    self.advance_step();
                }
            }
            Action::Character('f') => {
                if let Some(request) = self.next_fix_request() {
                    self.pending_fix = Some(request);
                }
            }
            Action::Back => self.wants_back = true,
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
                self.domain_findings = None;
            }
            Action::Back => self.wants_back = true,
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
                if self.static_mode {
                    "enter next  esc back  q quit"
                } else if self.current_step_failed() {
                    "r retry  s skip  esc back  q quit"
                } else if self.next_fix_request().is_some() {
                    "enter run/next  space next  f fix  esc back  q quit"
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
        self.should_quit || self.pending_fix.is_some()
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        self.should_quit = false;
        self.wants_back = false;
        self.pending_fix = None;
        self.phase = TutorialPhase::PathSelect;
        self.path_selected = 0;
        self.steps.clear();
        self.current_step = 0;
        self.chosen_path = None;
        self.scan_results = None;
        self.domain_findings = None;
        self.resuming_notice = None;
        // static_mode, static_notice, and completed_paths are intentionally
        // preserved — they represent environment/session state, not transient.
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

impl NotificationSource for TutorialState {
    fn notifications(&self) -> Vec<Notification> {
        let mut out = Vec::new();

        if let Some(notice) = self.static_notice.as_ref() {
            out.push(surface_notification(
                "tutorial",
                NotificationClass::Warning,
                NotificationPriority::High,
                "Interactive mode unavailable",
                notice,
            ));
        }

        if let Some(notice) = self.resuming_notice.as_ref() {
            out.push(surface_notification(
                "tutorial",
                NotificationClass::Info,
                NotificationPriority::Normal,
                "Tutorial resumed",
                notice,
            ));
        }

        // Only emit step-level failures while the tutorial is actively
        // running on a non-completed step. Once a step is skipped or the
        // phase flips to Complete, its stored `output`/`verify_result` are
        // stale and must not re-surface as live failures (adversarial F-002).
        if self.phase == TutorialPhase::Running
            && let Some(step) = self.steps.get(self.current_step)
            && !step.completed
        {
            if let Some(output) = &step.output
                && !output.success
            {
                // Do NOT echo stderr into the notification message. stderr
                // from shell commands regularly contains absolute paths,
                // credential-helper output, and $HOME/username — shipping
                // it via NotificationSource would leak that to every
                // telemetry subscriber (CWE-209). Keep the raw stderr on
                // `step.output` for local TUI rendering only.
                let message = format!(
                    "{} failed with exit code {}",
                    step.title,
                    output.exit_code.unwrap_or(-1),
                );
                out.push(surface_notification(
                    "tutorial",
                    NotificationClass::Failure,
                    NotificationPriority::High,
                    "Tutorial step failed",
                    message,
                ));
            }
            if matches!(step.verify_result, Some(VerifyResult::Fail(_))) {
                // verify_hint is author-controlled (tutorial path definitions),
                // not user input, so it is safe to surface verbatim.
                let hint = step
                    .verify_hint
                    .as_deref()
                    .unwrap_or("Verification failed.");
                out.push(surface_notification(
                    "tutorial",
                    NotificationClass::Failure,
                    NotificationPriority::High,
                    "Verification failed",
                    hint.to_string(),
                ));
            }
        }

        out
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
                verify: None,
                verify_result: None,
                verify_hint: None,
                watch_path: None,
                watch_demo: false,
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
            verify: None,
            verify_result: None,
            verify_hint: None,
            watch_path: None,
            watch_demo: false,
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

    /// Build a state with a command step that has verification attached.
    fn state_with_verified_step(command: &str, verify: Verify, hint: &str) -> TutorialState {
        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Verified Step".to_string(),
            description: "A step with verification.".to_string(),
            instruction: format!("Run: {command}"),
            command: Some(command.to_string()),
            completed: false,
            output: None,
            verify: Some(verify),
            verify_result: None,
            verify_hint: Some(hint.to_string()),
            watch_path: None,
            watch_demo: false,
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

    #[test]
    fn starts_at_path_select() {
        let state = TutorialState::new();
        assert_eq!(state.phase, TutorialPhase::PathSelect);
        // LAUNCH-014: paths now include the value-first
        // ProtectionLoop default plus the four deeper-learning
        // tracks. ProtectionLoop is index 0 / pre-selected.
        assert_eq!(state.paths.len(), 5);
        assert_eq!(state.paths[0], TutorialPath::ProtectionLoop);
        assert_eq!(state.path_selected, 0);
        assert!(state.chosen_path.is_none());
    }

    #[test]
    fn path_selection_advances_to_running() {
        // LAUNCH-014: hitting Enter from the default selection lands
        // the user on the ProtectionLoop walk, not the Policy
        // taxonomy path. The chosen_path assertion is the visible
        // pin against accidental reordering.
        let mut state = TutorialState::new();
        state.handle_key(Action::Select);
        assert_eq!(state.phase, TutorialPhase::Running);
        assert_eq!(state.chosen_path, Some(TutorialPath::ProtectionLoop));
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
        // LAUNCH-014: ProtectionLoop step 0 is "anvil's protection
        // loop in 60 seconds" — no command — so Select advances it.
        // (Was the Policy "Introduction" step before LAUNCH-014
        // reordered the default path; the assertion is identical
        // because both are informational.)
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose ProtectionLoop
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
    fn back_from_running_exits_tutorial() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose default ProtectionLoop
        assert_eq!(state.phase, TutorialPhase::Running);

        state.handle_key(Action::Back);
        assert!(state.wants_back);
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
    fn back_from_running_sets_wants_back() {
        let mut state = TutorialState::new();

        // LAUNCH-014 reordering: ProtectionLoop is index 0, Policy
        // index 1, Architecture index 2. Press Down twice to land
        // on Architecture so the original assertion still pins the
        // running-phase exit semantics rather than path selection.
        state.handle_key(Action::Down);
        state.handle_key(Action::Down);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen_path, Some(TutorialPath::Architecture));

        // Back exits the tutorial entirely
        state.handle_key(Action::Back);
        assert!(state.wants_back);
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
        assert_eq!(
            TutorialPath::ProtectionLoop.label(),
            "anvil's protection loop"
        );
        assert_eq!(TutorialPath::Policy.label(), "Policy checks");
        assert_eq!(TutorialPath::Architecture.label(), "Boundary findings");
        assert_eq!(TutorialPath::Drift.label(), "Configuration drift");
        assert_eq!(TutorialPath::CI.label(), "CI gate integration");
    }

    // --- LAUNCH-014: protection-loop path copy invariants ---

    #[test]
    fn protection_loop_path_is_default_first_path() {
        // The Enter key on a fresh tutorial must land the user on
        // the ProtectionLoop walk, not the Policy taxonomy. This is
        // the load-bearing invariant of LAUNCH-014.
        let state = TutorialState::new();
        assert_eq!(state.paths.first(), Some(&TutorialPath::ProtectionLoop));
        assert_eq!(state.path_selected, 0);
    }

    #[test]
    fn protection_loop_copy_uses_activation_state_vocabulary() {
        // The path body must reference the LAUNCH-008 activation
        // state literals so users recognise them when `anvil status
        // --verify` prints one. This pin protects the cross-surface
        // vocabulary contract.
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::ProtectionLoop);
        let body: String = state
            .steps
            .iter()
            .map(|s| format!("{}\n{}\n{}", s.title, s.description, s.instruction))
            .collect::<Vec<_>>()
            .join("\n");

        for state_word in [
            "protecting",
            "ready_restart_required",
            "watching",
            "needs_action",
            "unsupported",
        ] {
            assert!(
                body.contains(state_word),
                "ProtectionLoop copy must reference state `{state_word}` so it stays \
                 vocabulary-aligned with `anvil start --verify` / LAUNCH-008. body:\n{body}"
            );
        }
    }

    #[test]
    fn protection_loop_copy_does_not_claim_pre_write_protection() {
        // The tutorial does not have activation evidence, so its copy
        // must not promise pre-write protection or call the user's
        // repo "protected". The final step points at `anvil start
        // --verify` instead — the only surface that produces a
        // literal `ProtectionState`.
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::ProtectionLoop);
        let body: String = state
            .steps
            .iter()
            .map(|s| format!("{}\n{}\n{}", s.title, s.description, s.instruction))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();

        // Allow the literal state word `protecting` (referenced in
        // the vocabulary explainer) but reject phrasings that claim
        // present-tense protection of the user's repo. This pin
        // would catch copy edits like "you are now protected" or
        // "pre-write validation enabled".
        for forbidden in [
            "you are now protected",
            "you're now protected",
            "your repo is protected",
            "pre-write validation enabled",
            "pre-write validation active",
            "anvil is now intercepting",
        ] {
            assert!(
                !body.contains(forbidden),
                "ProtectionLoop copy must not include `{forbidden}` — only \
                 `anvil start --verify` is allowed to produce that claim. body:\n{body}"
            );
        }

        // The final step must point at `anvil start --verify` so the
        // user gets a real evidence-backed answer instead of trusting
        // the tutorial's word.
        assert!(
            body.contains("anvil start --verify"),
            "final ProtectionLoop step must direct users at `anvil start --verify`, body:\n{body}"
        );
    }

    #[test]
    fn protection_loop_round_trips_through_label() {
        // The progress-file label round-trip must work for the new
        // path so completed-state checkmarks survive a process
        // restart. (Mirrors `path_labels_round_trip` for the
        // existing four paths.)
        let path = TutorialPath::ProtectionLoop;
        assert_eq!(TutorialPath::from_label(path.label()), Some(path));
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
    fn back_from_failed_command_exits_tutorial() {
        let mut state = state_with_command_step("exit 1");
        state.handle_key(Action::Select); // fails

        state.handle_key(Action::Back);

        assert!(state.wants_back);
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

    // --- Scan results threading tests ---

    use discovery::{Finding, FindingSeverity, FindingSource, ScanResults};

    fn make_scan_results() -> ScanResults {
        ScanResults {
            findings: vec![
                Finding {
                    file: "src/main.rs".to_string(),
                    line: Some(10),
                    severity: FindingSeverity::Error,
                    source: FindingSource::AntiPattern,
                    title: "anti-pattern".to_string(),
                    message: "test".to_string(),
                    suggestion: "fix".to_string(),
                    warning_id: Some("AP-003".to_string()),
                },
                Finding {
                    file: "src/lib.rs".to_string(),
                    line: Some(20),
                    severity: FindingSeverity::Warning,
                    source: FindingSource::Architecture,
                    title: "boundary".to_string(),
                    message: "test".to_string(),
                    suggestion: "fix".to_string(),
                    warning_id: None,
                },
            ],
            files_scanned: 100,
            duration_ms: 250,
            truncated: false,
            files_skipped_by_ignore: 0,
        }
    }

    #[test]
    fn scan_results_default_none() {
        let state = TutorialState::new();
        assert!(state.scan_results.is_none());
        assert!(state.domain_findings.is_none());
    }

    #[test]
    fn set_scan_results_stores_results() {
        let mut state = TutorialState::new();
        let results = make_scan_results();
        state.set_scan_results(results);
        assert!(state.scan_results.is_some());
        assert_eq!(state.scan_results.as_ref().unwrap().findings.len(), 2);
    }

    #[test]
    fn load_steps_computes_domain_findings() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.load_steps(TutorialPath::Policy);

        assert!(state.domain_findings.is_some());
        let domain = state.domain_findings.as_ref().unwrap();
        // Policy gets AntiPattern + Secret, so only the AntiPattern finding
        assert_eq!(domain.findings.len(), 1);
        assert_eq!(domain.findings[0].source, FindingSource::AntiPattern);
    }

    #[test]
    fn load_steps_without_scan_results_leaves_domain_none() {
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Architecture);
        assert!(state.domain_findings.is_none());
    }

    #[test]
    fn reset_clears_scan_and_domain_findings() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.load_steps(TutorialPath::Policy);
        assert!(state.scan_results.is_some());
        assert!(state.domain_findings.is_some());

        <TutorialState as crate::surface::Surface>::reset(&mut state);
        assert!(state.scan_results.is_none());
        assert!(state.domain_findings.is_none());
    }

    // --- Verification integration tests ---

    #[test]
    fn verify_pass_advances_step() {
        // "echo hello" succeeds and stdout contains "hello" — should advance.
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("hello".to_string()),
            "Output should contain hello.",
        );
        state.handle_key(Action::Select);

        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert_eq!(state.steps[0].verify_result, Some(VerifyResult::Pass));
    }

    #[test]
    fn verify_fail_stays_on_step() {
        // "echo hello" succeeds but stdout does NOT contain "world" — should stay.
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("world".to_string()),
            "Output should contain world.",
        );
        state.handle_key(Action::Select);

        assert_eq!(state.phase, TutorialPhase::Running);
        assert_eq!(state.current_step, 0);
        assert!(!state.steps[0].completed);
        assert!(state.current_step_failed());
        assert!(matches!(
            state.steps[0].verify_result,
            Some(VerifyResult::Fail(_))
        ));
    }

    #[test]
    fn verify_fail_then_skip_advances() {
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("world".to_string()),
            "Output should contain world.",
        );
        state.handle_key(Action::Select); // verify fails
        assert!(state.current_step_failed());

        state.handle_key(Action::Character('s')); // skip
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn verify_fail_then_retry_clears_result() {
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("hello".to_string()),
            "Output should contain hello.",
        );

        // Inject a failed verify state to simulate prior failure.
        state.steps[0].output = Some(CommandOutput {
            stdout: "nope".to_string(),
            stderr: String::new(),
            success: true,
            exit_code: Some(0),
        });
        state.steps[0].verify_result = Some(VerifyResult::Fail(
            "Output did not contain expected text: hello".to_string(),
        ));
        assert!(state.current_step_failed());

        // Retry — the actual "echo hello" command succeeds and contains "hello".
        state.handle_key(Action::Character('r'));

        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert_eq!(state.steps[0].verify_result, Some(VerifyResult::Pass));
    }

    // --- Static mode tests ---

    #[test]
    fn static_mode_defaults_to_false() {
        let state = TutorialState::new();
        assert!(!state.static_mode);
        assert!(state.static_notice.is_none());
    }

    #[test]
    fn enable_static_mode_sets_flag_and_notice() {
        let mut state = TutorialState::new();
        state.enable_static_mode();
        assert!(state.static_mode);
        assert_eq!(
            state.static_notice.as_deref(),
            Some("Interactive mode unavailable \u{2014} showing guided walkthrough.")
        );
    }

    #[test]
    fn enable_static_mode_with_reason_overrides_notice() {
        let mut state = TutorialState::new();
        state.enable_static_mode_with_reason("watcher failed: inotify limit reached");
        assert!(state.static_mode);
        assert_eq!(
            state.static_notice.as_deref(),
            Some("watcher failed: inotify limit reached")
        );
    }

    #[test]
    fn static_mode_select_advances_command_step_without_executing() {
        let mut state = state_with_command_step("echo should_not_run");
        state.enable_static_mode();

        state.handle_key(Action::Select);

        // Step should advance without executing — no output stored.
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert!(state.steps[0].output.is_none());
    }

    #[test]
    fn static_mode_toggle_advances_command_step() {
        let mut state = state_with_command_step("echo should_not_run");
        state.enable_static_mode();

        state.handle_key(Action::Toggle);

        // In static mode, Toggle advances even command steps.
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert!(state.steps[0].output.is_none());
    }

    #[test]
    fn static_mode_informational_steps_still_advance() {
        let mut state = state_with_plain_steps(3);
        state.enable_static_mode();

        state.handle_key(Action::Select);
        assert_eq!(state.current_step, 1);

        state.handle_key(Action::Select);
        assert_eq!(state.current_step, 2);

        state.handle_key(Action::Select);
        assert_eq!(state.phase, TutorialPhase::Complete);
    }

    #[test]
    fn static_mode_help_text_shows_simplified() {
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert_eq!(help, "enter next  esc back  q quit");
    }

    #[test]
    fn static_mode_preserved_across_reset() {
        let mut state = TutorialState::new();
        state.enable_static_mode();

        <TutorialState as crate::surface::Surface>::reset(&mut state);

        assert!(state.static_mode);
        assert!(state.static_notice.is_some());
    }

    #[test]
    fn static_mode_current_step_failed_always_false() {
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        // In static mode, commands never execute, so current_step_failed()
        // should always return false.
        assert!(!state.current_step_failed());
    }

    #[test]
    fn static_mode_skip_still_works_defensively() {
        // Even though current_step_failed() is unreachable in static mode,
        // if output were injected (defensively), skip should still advance.
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        // Inject failure output to simulate an edge case.
        state.steps[0].output = Some(CommandOutput {
            stdout: String::new(),
            stderr: "simulated".to_string(),
            success: false,
            exit_code: Some(1),
        });
        assert!(state.current_step_failed());

        state.handle_key(Action::Character('s'));
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn static_mode_back_from_running_exits_tutorial() {
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        state.handle_key(Action::Back);
        assert!(state.wants_back);
    }

    #[test]
    fn static_mode_quit_from_running() {
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    // --- Progress persistence / resumption tests ---

    #[test]
    fn from_label_roundtrips() {
        for path in &[
            // LAUNCH-014: include the new ProtectionLoop default in
            // the round-trip pin so a label rename can't silently
            // break the resumption seam.
            TutorialPath::ProtectionLoop,
            TutorialPath::Policy,
            TutorialPath::Architecture,
            TutorialPath::Drift,
            TutorialPath::CI,
        ] {
            assert_eq!(TutorialPath::from_label(path.label()), Some(*path));
        }
        assert_eq!(TutorialPath::from_label("Nonexistent"), None);
    }

    #[test]
    fn from_label_accepts_legacy_labels() {
        // Pre-rename progress files still need to resume into the matching
        // enum variant after the labels were reframed for onboarding clarity.
        assert_eq!(
            TutorialPath::from_label("Policy"),
            Some(TutorialPath::Policy)
        );
        assert_eq!(
            TutorialPath::from_label("Architecture"),
            Some(TutorialPath::Architecture)
        );
        assert_eq!(TutorialPath::from_label("Drift"), Some(TutorialPath::Drift));
        assert_eq!(
            TutorialPath::from_label("CI Integration"),
            Some(TutorialPath::CI)
        );
    }

    #[test]
    fn set_completed_paths_stored() {
        let mut state = TutorialState::new();
        state.set_completed_paths(vec![TutorialPath::Policy, TutorialPath::Drift]);
        assert_eq!(state.completed_paths.len(), 2);
        assert!(state.completed_paths.contains(&TutorialPath::Policy));
        assert!(state.completed_paths.contains(&TutorialPath::Drift));
    }

    #[test]
    fn completed_paths_preserved_across_reset() {
        let mut state = TutorialState::new();
        state.set_completed_paths(vec![TutorialPath::Architecture]);

        <TutorialState as crate::surface::Surface>::reset(&mut state);

        assert_eq!(state.completed_paths, vec![TutorialPath::Architecture]);
    }

    #[test]
    fn resume_path_jumps_to_step() {
        let mut state = TutorialState::new();
        // Policy has 6 steps — provide a matching-length vec.
        let completed = vec![true, true, false, false, false, false];
        state.resume_path(TutorialPath::Policy, 2, &completed);

        assert_eq!(state.phase, TutorialPhase::Running);
        assert_eq!(state.chosen_path, Some(TutorialPath::Policy));
        assert_eq!(state.current_step, 2);
        assert!(state.steps[0].completed);
        assert!(state.steps[1].completed);
        assert!(!state.steps[2].completed);
    }

    #[test]
    fn resume_path_sets_notice() {
        let mut state = TutorialState::new();
        // Policy has 6 steps.
        state.resume_path(
            TutorialPath::Policy,
            2,
            &[true, true, false, false, false, false],
        );

        assert!(state.resuming_notice.is_some());
        let notice = state.resuming_notice.as_ref().unwrap();
        assert!(notice.contains("Resuming from step 3"));
    }

    #[test]
    fn resume_notice_cleared_on_advance() {
        let mut state = state_with_plain_steps(3);
        state.resuming_notice = Some("Resuming from step 2 of 3.".to_string());
        state.current_step = 1;
        state.steps[0].completed = true;

        state.handle_key(Action::Select); // advance step 1
        assert!(state.resuming_notice.is_none());
    }

    #[test]
    fn resume_clears_on_reset() {
        let mut state = TutorialState::new();
        // Drift has 6 steps.
        state.resume_path(
            TutorialPath::Drift,
            1,
            &[true, false, false, false, false, false],
        );
        assert!(state.resuming_notice.is_some());

        <TutorialState as crate::surface::Surface>::reset(&mut state);
        assert!(state.resuming_notice.is_none());
    }

    #[test]
    fn resume_stale_session_discarded() {
        let mut state = TutorialState::new();
        // Provide wrong-length steps_completed — simulates a stale session.
        state.resume_path(TutorialPath::CI, 2, &[true, true]);

        // Stale session discarded: starts at step 0 with no notice.
        assert_eq!(state.current_step, 0);
        assert!(state.resuming_notice.is_none());
        assert!(!state.steps[0].completed);
    }

    // --- File watcher integration tests ---

    fn state_with_watched_step(watch_path: &str) -> TutorialState {
        let dir = std::env::temp_dir().join("anvil_watch_test");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("marker.txt");

        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Watched Step".to_string(),
            description: "A step with file watching.".to_string(),
            instruction: "Create the target file.".to_string(),
            command: None,
            completed: false,
            output: None,
            verify: Some(Verify::FileExists(target.to_string_lossy().to_string())),
            verify_result: None,
            verify_hint: Some("File not found.".to_string()),
            watch_path: Some(watch_path.to_string()),
            watch_demo: false,
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

    #[test]
    fn handle_file_change_ignores_non_running_phase() {
        let mut state = TutorialState::new();
        // Phase is PathSelect, not Running.
        let advanced = state.handle_file_change(&[std::path::PathBuf::from("test.txt")]);
        assert!(!advanced);
    }

    #[test]
    fn handle_file_change_ignores_step_without_watch_path() {
        let mut state = state_with_plain_steps(2);
        let advanced = state.handle_file_change(&[std::path::PathBuf::from("test.txt")]);
        assert!(!advanced);
    }

    #[test]
    fn handle_file_change_ignores_irrelevant_paths() {
        let mut state = state_with_watched_step("/tmp/watched_dir");
        let advanced =
            state.handle_file_change(&[std::path::PathBuf::from("/other/unrelated.txt")]);
        assert!(!advanced);
        assert_eq!(state.current_step, 0);
    }

    #[test]
    fn handle_file_change_auto_verifies_file_exists() {
        let dir = std::env::temp_dir().join("anvil_watch_autotest");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("marker.txt");

        // Create the file so FileExists passes.
        std::fs::write(&target, "ok").unwrap();

        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Watched".to_string(),
            description: "desc".to_string(),
            instruction: "inst".to_string(),
            command: None,
            completed: false,
            output: None,
            verify: Some(Verify::FileExists(target.to_string_lossy().to_string())),
            verify_result: None,
            verify_hint: None,
            watch_path: Some(dir.to_string_lossy().to_string()),
            watch_demo: false,
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);

        let changed = dir.join("marker.txt");
        let advanced = state.handle_file_change(&[changed]);
        assert!(advanced);
        assert_eq!(state.phase, TutorialPhase::Complete);

        // Clean up.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn handle_file_change_stays_when_verify_fails() {
        let dir = std::env::temp_dir().join("anvil_watch_failtest");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("nonexistent.txt");

        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Watched".to_string(),
            description: "desc".to_string(),
            instruction: "inst".to_string(),
            command: None,
            completed: false,
            output: None,
            verify: Some(Verify::FileExists(target.to_string_lossy().to_string())),
            verify_result: None,
            verify_hint: None,
            watch_path: Some(dir.to_string_lossy().to_string()),
            watch_demo: false,
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);

        // Trigger with a file in the watched dir, but the verify target doesn't exist.
        let changed = dir.join("other.txt");
        let advanced = state.handle_file_change(&[changed]);
        assert!(!advanced);
        assert_eq!(state.current_step, 0);
        assert!(matches!(
            state.steps[0].verify_result,
            Some(VerifyResult::Fail(_))
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn handle_file_change_skipped_in_static_mode() {
        let mut state = state_with_watched_step("/tmp/watched_dir");
        state.enable_static_mode();

        let advanced =
            state.handle_file_change(&[std::path::PathBuf::from("/tmp/watched_dir/file.txt")]);
        assert!(!advanced);
    }

    // --- Fix key tests ---

    #[test]
    fn f_key_sets_pending_fix_when_fixable_domain_finding_present() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.handle_key(Action::Select); // choose Policy path
        assert!(state.domain_findings.is_some());
        assert!(!state.domain_findings.as_ref().unwrap().findings.is_empty());

        state.handle_key(Action::Character('f'));
        assert_eq!(
            state.pending_fix,
            Some(FixRequest::AntiPatternWarning {
                file: "src/main.rs".to_string(),
                line: 10,
                warning_id: "AP-003".to_string(),
            })
        );
    }

    #[test]
    fn f_key_no_op_without_domain_findings() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy path, no scan results
        assert!(state.domain_findings.is_none());

        state.handle_key(Action::Character('f'));
        assert!(state.pending_fix.is_none());
    }

    #[test]
    fn pending_fix_causes_should_quit_true() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.handle_key(Action::Select);
        assert!(!crate::surface::Surface::should_quit(&state));

        state.handle_key(Action::Character('f'));
        assert!(crate::surface::Surface::should_quit(&state));
    }

    #[test]
    fn reset_clears_pending_fix() {
        let mut state = TutorialState::new();
        state.pending_fix = Some(FixRequest::AntiPatternWarning {
            file: "src/main.rs".to_string(),
            line: 10,
            warning_id: "AP-003".to_string(),
        });
        <TutorialState as crate::surface::Surface>::reset(&mut state);
        assert!(state.pending_fix.is_none());
    }

    // --- NotificationSource impl ---

    #[test]
    fn notifications_empty_without_notices() {
        let state = TutorialState::new();
        assert!(state.notifications().is_empty());
    }

    #[test]
    fn notifications_include_static_notice_as_warning() {
        let mut state = TutorialState::new();
        state.enable_static_mode_with_reason("watcher unavailable");
        let notifications = state.notifications();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].class, NotificationClass::Warning);
        assert_eq!(notifications[0].priority, NotificationPriority::High);
        assert_eq!(
            notifications[0]
                .context
                .as_ref()
                .and_then(|c| c.source.as_deref()),
            Some("tutorial")
        );
    }

    #[test]
    fn notifications_include_resume_notice_as_info() {
        let mut state = TutorialState::new();
        state.resume_path(
            TutorialPath::Policy,
            1,
            &[true, false, false, false, false, false],
        );
        let notifications = state.notifications();
        let resume = notifications
            .iter()
            .find(|n| n.title == "Tutorial resumed")
            .expect("resume notification present");
        assert_eq!(resume.class, NotificationClass::Info);
        assert_eq!(resume.priority, NotificationPriority::Normal);
    }

    #[test]
    fn notifications_include_failure_when_command_fails() {
        let mut state = state_with_command_step("exit 1");
        state.handle_key(Action::Select);
        let notifications = state.notifications();
        let failure = notifications
            .iter()
            .find(|n| n.class == NotificationClass::Failure)
            .expect("failure notification present");
        assert_eq!(failure.priority, NotificationPriority::High);
        assert_eq!(failure.title, "Tutorial step failed");
    }

    #[test]
    fn notifications_include_failure_when_verify_fails() {
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("world".to_string()),
            "Output should contain world.",
        );
        state.handle_key(Action::Select);
        let notifications = state.notifications();
        assert!(
            notifications
                .iter()
                .any(|n| n.title == "Verification failed" && n.class == NotificationClass::Failure),
            "expected verification failure notification, got {notifications:?}"
        );
    }

    #[test]
    fn notifications_never_echo_stderr_contents() {
        // Security regression (CWE-209): failed-command notifications must
        // never embed the step's stderr, which frequently contains absolute
        // paths, credential-helper output, or $HOME/username fragments.
        let mut state = state_with_command_step("/bin/sh -c 'exit 7'");
        state.steps[0].output = Some(CommandOutput {
            stdout: String::new(),
            stderr: "/home/secret-user/work/tokens/.env: permission denied".to_string(),
            success: false,
            exit_code: Some(7),
        });
        let notifications = state.notifications();
        for n in &notifications {
            assert!(
                !n.message.contains("secret-user"),
                "notification leaked $HOME fragment: {:?}",
                n.message
            );
            assert!(
                !n.message.contains("/home/"),
                "notification leaked absolute path: {:?}",
                n.message
            );
            assert!(
                !n.message.contains("permission denied"),
                "notification leaked stderr text: {:?}",
                n.message
            );
        }
        // And we still report a failure — with the sanitised message.
        assert!(
            notifications
                .iter()
                .any(|n| n.title == "Tutorial step failed" && n.message.contains("exit code 7")),
            "expected sanitised failure notification, got {notifications:?}"
        );
    }

    #[test]
    fn notifications_suppressed_after_verify_fail_skip_complete() {
        // Adversarial F-002: after verify-fail -> skip -> phase=Complete,
        // advance_step() doesn't clear step.verify_result, but notifications()
        // must not re-surface the stale failure because the tutorial is done.
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("world".to_string()),
            "Output should contain world.",
        );
        state.handle_key(Action::Select); // command succeeds, verify fails
        assert!(state.current_step_failed());

        state.handle_key(Action::Character('s')); // skip
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);

        let notifications = state.notifications();
        assert!(
            !notifications
                .iter()
                .any(|n| n.class == NotificationClass::Failure),
            "completed tutorial must not emit Failure notifications: {notifications:?}",
        );
    }
}
