use eddacraft_tui::keyboard::Action;

/// A git hook definition.
pub struct HookDef {
    pub name: &'static str,
    pub description: &'static str,
    pub command: &'static str,
}

/// Returns the set of hooks Anvil can install.
pub fn available_hooks() -> Vec<HookDef> {
    vec![
        HookDef {
            name: "pre-commit",
            description: "Run quality gate checks before each commit",
            command: "anvil gate --quick",
        },
        HookDef {
            name: "pre-push",
            description: "Run full quality gate before pushing",
            command: "anvil gate",
        },
    ]
}

/// Hook manager detected in the project directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookManager {
    None,
    Husky,
    Lefthook,
    PreCommit,
}

impl HookManager {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Husky => "Husky",
            Self::Lefthook => "Lefthook",
            Self::PreCommit => "pre-commit framework",
        }
    }

    pub fn adapter_note(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Husky => Some("Anvil will add entries to your existing Husky hooks"),
            Self::Lefthook => Some("Anvil will add a run entry to your lefthook.yml"),
            Self::PreCommit => Some("Anvil will add a hook entry to your .pre-commit-config.yaml"),
        }
    }
}

/// Detect which hook manager (if any) is present under `project_dir`.
pub fn detect_hook_manager(project_dir: &std::path::Path) -> HookManager {
    if project_dir.join(".husky").exists() {
        HookManager::Husky
    } else if project_dir.join(".lefthook.yml").exists()
        || project_dir.join("lefthook.yml").exists()
    {
        HookManager::Lefthook
    } else if project_dir.join(".pre-commit-config.yaml").exists() {
        HookManager::PreCommit
    } else {
        HookManager::None
    }
}

/// Phases of the hooks installation surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HooksPhase {
    /// Show available hooks and any detected hook manager; user selects which to install.
    Overview,
    /// Confirm before writing any files.
    Confirm,
    /// Installation complete, skipped, or failed.
    Done,
}

/// State for the git hooks installation surface.
#[allow(clippy::struct_excessive_bools)]
pub struct HooksState {
    pub phase: HooksPhase,
    pub hooks: Vec<HookDef>,
    pub hook_manager: HookManager,
    /// One toggle per hook in `hooks`, in the same order.
    pub selected_hooks: Vec<bool>,
    pub cursor: usize,
    pub installed: bool,
    pub install_error: Option<String>,
    pub should_quit: bool,
    pub wants_back: bool,
    /// Set when the user confirms installation from the Confirm phase.
    /// The caller reads `selected_hook_names()` and performs the install.
    pub wants_install: bool,
}

impl HooksState {
    pub fn new(project_dir: &std::path::Path) -> Self {
        let hooks = available_hooks();
        let count = hooks.len();
        let hook_manager = detect_hook_manager(project_dir);
        Self {
            phase: HooksPhase::Overview,
            hooks,
            hook_manager,
            selected_hooks: vec![true; count],
            cursor: 0,
            installed: false,
            install_error: None,
            should_quit: false,
            wants_back: false,
            wants_install: false,
        }
    }

    /// Returns the names of hooks that are currently toggled on.
    pub fn selected_hook_names(&self) -> Vec<&str> {
        self.hooks
            .iter()
            .zip(self.selected_hooks.iter())
            .filter_map(|(hook, &on)| if on { Some(hook.name) } else { None })
            .collect()
    }

    /// Signal that installation completed successfully.
    pub fn mark_installed(&mut self) {
        self.installed = true;
        self.install_error = None;
        self.phase = HooksPhase::Done;
    }

    /// Signal that installation failed with an error message.
    pub fn mark_failed(&mut self, error: String) {
        self.install_error = Some(error);
        self.phase = HooksPhase::Done;
    }

    pub fn handle_key(&mut self, action: Action) {
        match self.phase {
            HooksPhase::Overview => self.handle_overview_key(action),
            HooksPhase::Confirm => self.handle_confirm_key(action),
            HooksPhase::Done => self.handle_done_key(action),
        }
    }

    fn handle_overview_key(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            Action::Down => {
                if self.cursor < self.hooks.len().saturating_sub(1) {
                    self.cursor += 1;
                }
            }
            Action::Toggle => {
                if let Some(on) = self.selected_hooks.get_mut(self.cursor) {
                    *on = !*on;
                }
            }
            Action::Select => {
                self.phase = HooksPhase::Confirm;
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

    fn handle_confirm_key(&mut self, action: Action) {
        match action {
            Action::Select => {
                if self.selected_hook_names().is_empty() {
                    // Nothing to install — skip to Done.
                    self.phase = HooksPhase::Done;
                } else {
                    // Exit the surface so the caller can perform the install,
                    // then optionally re-enter or show a loading screen.
                    self.wants_install = true;
                    self.should_quit = true;
                }
            }
            Action::Back => {
                self.phase = HooksPhase::Overview;
            }
            Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_done_key(&mut self, action: Action) {
        match action {
            Action::Select | Action::Back | Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

impl crate::surface::Surface for HooksState {
    fn surface_name(&self) -> &'static str {
        "Git Hooks"
    }

    fn help_text(&self) -> &'static str {
        match self.phase {
            HooksPhase::Overview => "j/k navigate  space toggle  enter confirm  esc back  q quit",
            HooksPhase::Confirm => "enter install  esc back  q quit",
            HooksPhase::Done => "enter/esc close",
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
        self.phase = HooksPhase::Overview;
        self.cursor = 0;
        for v in &mut self.selected_hooks {
            *v = true;
        }
        self.installed = false;
        self.install_error = None;
        self.should_quit = false;
        self.wants_back = false;
        self.wants_install = false;
    }

    fn render(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        theme: &eddacraft_tui::theme::EddaCraftTheme,
    ) {
        super::hooks_render::render(frame, area, self, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn empty_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "anvil_hooks_test_{}_{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &std::path::Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // ---------- detect_hook_manager ----------

    #[test]
    fn detect_none_in_empty_dir() {
        let dir = empty_dir();
        assert_eq!(detect_hook_manager(&dir), HookManager::None);
        cleanup(&dir);
    }

    #[test]
    fn detect_husky() {
        let dir = empty_dir();
        std::fs::create_dir_all(dir.join(".husky")).unwrap();
        assert_eq!(detect_hook_manager(&dir), HookManager::Husky);
        cleanup(&dir);
    }

    #[test]
    fn detect_lefthook_dot_yml() {
        let dir = empty_dir();
        std::fs::write(dir.join(".lefthook.yml"), "").unwrap();
        assert_eq!(detect_hook_manager(&dir), HookManager::Lefthook);
        cleanup(&dir);
    }

    #[test]
    fn detect_lefthook_yml() {
        let dir = empty_dir();
        std::fs::write(dir.join("lefthook.yml"), "").unwrap();
        assert_eq!(detect_hook_manager(&dir), HookManager::Lefthook);
        cleanup(&dir);
    }

    #[test]
    fn detect_pre_commit() {
        let dir = empty_dir();
        std::fs::write(dir.join(".pre-commit-config.yaml"), "").unwrap();
        assert_eq!(detect_hook_manager(&dir), HookManager::PreCommit);
        cleanup(&dir);
    }

    #[test]
    fn husky_takes_priority_over_lefthook() {
        let dir = empty_dir();
        std::fs::create_dir_all(dir.join(".husky")).unwrap();
        std::fs::write(dir.join("lefthook.yml"), "").unwrap();
        assert_eq!(detect_hook_manager(&dir), HookManager::Husky);
        cleanup(&dir);
    }

    // ---------- HooksState initial state ----------

    #[test]
    fn initial_state_overview_phase() {
        let dir = empty_dir();
        let state = HooksState::new(&dir);
        assert_eq!(state.phase, HooksPhase::Overview);
        cleanup(&dir);
    }

    #[test]
    fn initial_all_hooks_selected() {
        let dir = empty_dir();
        let state = HooksState::new(&dir);
        assert!(state.selected_hooks.iter().all(|&on| on));
        cleanup(&dir);
    }

    #[test]
    fn initial_cursor_zero() {
        let dir = empty_dir();
        let state = HooksState::new(&dir);
        assert_eq!(state.cursor, 0);
        cleanup(&dir);
    }

    #[test]
    fn initial_hook_count_matches_available() {
        let dir = empty_dir();
        let state = HooksState::new(&dir);
        assert_eq!(state.hooks.len(), available_hooks().len());
        assert_eq!(state.selected_hooks.len(), state.hooks.len());
        cleanup(&dir);
    }

    // ---------- Overview navigation ----------

    #[test]
    fn navigate_down_and_up() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Down);
        assert_eq!(state.cursor, 1);
        state.handle_key(Action::Up);
        assert_eq!(state.cursor, 0);
        cleanup(&dir);
    }

    #[test]
    fn cursor_does_not_go_below_zero() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Up);
        assert_eq!(state.cursor, 0);
        cleanup(&dir);
    }

    #[test]
    fn cursor_does_not_exceed_last_hook() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        for _ in 0..20 {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.cursor, state.hooks.len() - 1);
        cleanup(&dir);
    }

    // ---------- Toggle ----------

    #[test]
    fn toggle_deselects_hook() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        assert!(state.selected_hooks[0]);
        state.handle_key(Action::Toggle);
        assert!(!state.selected_hooks[0]);
        cleanup(&dir);
    }

    #[test]
    fn toggle_reselects_hook() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Toggle);
        state.handle_key(Action::Toggle);
        assert!(state.selected_hooks[0]);
        cleanup(&dir);
    }

    #[test]
    fn toggle_second_hook() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Down);
        state.handle_key(Action::Toggle);
        assert!(state.selected_hooks[0]);
        assert!(!state.selected_hooks[1]);
        cleanup(&dir);
    }

    // ---------- selected_hook_names ----------

    #[test]
    fn selected_hook_names_all_selected() {
        let dir = empty_dir();
        let state = HooksState::new(&dir);
        let names = state.selected_hook_names();
        assert_eq!(names.len(), state.hooks.len());
        cleanup(&dir);
    }

    #[test]
    fn selected_hook_names_after_deselect() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Toggle); // deselect first
        let names = state.selected_hook_names();
        assert_eq!(names.len(), state.hooks.len() - 1);
        assert!(!names.contains(&"pre-commit"));
        cleanup(&dir);
    }

    #[test]
    fn selected_hook_names_none_selected() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        for _ in 0..state.hooks.len() {
            state.handle_key(Action::Toggle);
            state.handle_key(Action::Down);
        }
        // Reset cursor, deselect all
        let mut state2 = HooksState::new(&dir);
        for i in 0..state2.selected_hooks.len() {
            state2.selected_hooks[i] = false;
        }
        assert!(state2.selected_hook_names().is_empty());
        cleanup(&dir);
    }

    // ---------- Phase transitions ----------

    #[test]
    fn select_in_overview_advances_to_confirm() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Select);
        assert_eq!(state.phase, HooksPhase::Confirm);
        cleanup(&dir);
    }

    #[test]
    fn back_in_confirm_returns_to_overview() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Select); // Overview → Confirm
        state.handle_key(Action::Back); // Confirm → Overview
        assert_eq!(state.phase, HooksPhase::Overview);
        cleanup(&dir);
    }

    #[test]
    fn select_in_confirm_signals_install_and_exits() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Select); // → Confirm
        state.handle_key(Action::Select); // → wants_install + should_quit
        assert!(state.wants_install);
        assert!(state.should_quit);
        assert_eq!(state.phase, HooksPhase::Confirm);
        cleanup(&dir);
    }

    #[test]
    fn select_in_confirm_with_none_selected_goes_to_done() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        for v in &mut state.selected_hooks {
            *v = false;
        }
        state.handle_key(Action::Select); // → Confirm
        state.handle_key(Action::Select); // → Done (nothing to install)
        assert_eq!(state.phase, HooksPhase::Done);
        assert!(!state.wants_install);
        cleanup(&dir);
    }

    #[test]
    fn select_in_done_sets_should_quit() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.phase = HooksPhase::Done;
        state.handle_key(Action::Select);
        assert!(state.should_quit);
        cleanup(&dir);
    }

    #[test]
    fn back_in_done_sets_should_quit() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.phase = HooksPhase::Done;
        state.handle_key(Action::Back);
        assert!(state.should_quit);
        cleanup(&dir);
    }

    // ---------- Back/Quit from Overview ----------

    #[test]
    fn back_in_overview_sets_wants_back() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Back);
        assert!(state.wants_back);
        assert!(!state.should_quit);
        cleanup(&dir);
    }

    #[test]
    fn quit_in_overview_sets_should_quit() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
        cleanup(&dir);
    }

    #[test]
    fn quit_in_confirm_sets_should_quit() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.phase = HooksPhase::Confirm;
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
        cleanup(&dir);
    }

    // ---------- mark_installed / mark_failed ----------

    #[test]
    fn mark_installed_sets_flags() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.mark_installed();
        assert!(state.installed);
        assert!(state.install_error.is_none());
        assert_eq!(state.phase, HooksPhase::Done);
        cleanup(&dir);
    }

    #[test]
    fn mark_failed_sets_error() {
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.mark_failed("permission denied".to_string());
        assert!(!state.installed);
        assert_eq!(state.install_error.as_deref(), Some("permission denied"));
        assert_eq!(state.phase, HooksPhase::Done);
        cleanup(&dir);
    }

    // ---------- Surface trait ----------

    #[test]
    fn surface_should_back_reflects_wants_back() {
        use crate::surface::Surface;
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        assert!(!Surface::should_back(&state));
        state.wants_back = true;
        assert!(Surface::should_back(&state));
        cleanup(&dir);
    }

    #[test]
    fn surface_should_quit_reflects_should_quit() {
        use crate::surface::Surface;
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        assert!(!Surface::should_quit(&state));
        state.should_quit = true;
        assert!(Surface::should_quit(&state));
        cleanup(&dir);
    }

    #[test]
    fn surface_reset_clears_all_state() {
        use crate::surface::Surface;
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        state.phase = HooksPhase::Done;
        state.cursor = 1;
        state.selected_hooks[0] = false;
        state.installed = true;
        state.install_error = Some("err".to_string());
        state.should_quit = true;
        state.wants_back = true;
        state.wants_install = true;
        Surface::reset(&mut state);
        assert_eq!(state.phase, HooksPhase::Overview);
        assert_eq!(state.cursor, 0);
        assert!(state.selected_hooks.iter().all(|&on| on));
        assert!(!state.installed);
        assert!(state.install_error.is_none());
        assert!(!state.should_quit);
        assert!(!state.wants_back);
        assert!(!state.wants_install);
        cleanup(&dir);
    }

    // ---------- HookManager labels and adapter notes ----------

    #[test]
    fn hook_manager_labels() {
        assert_eq!(HookManager::None.label(), "none");
        assert_eq!(HookManager::Husky.label(), "Husky");
        assert_eq!(HookManager::Lefthook.label(), "Lefthook");
        assert_eq!(HookManager::PreCommit.label(), "pre-commit framework");
    }

    #[test]
    fn hook_manager_adapter_notes() {
        assert!(HookManager::None.adapter_note().is_none());
        assert!(HookManager::Husky.adapter_note().is_some());
        assert!(HookManager::Lefthook.adapter_note().is_some());
        assert!(HookManager::PreCommit.adapter_note().is_some());
    }

    // ---------- available_hooks ----------

    #[test]
    fn available_hooks_has_two_entries() {
        let hooks = available_hooks();
        assert_eq!(hooks.len(), 2);
        assert_eq!(hooks[0].name, "pre-commit");
        assert_eq!(hooks[1].name, "pre-push");
    }

    #[test]
    fn hooks_have_non_empty_fields() {
        for hook in available_hooks() {
            assert!(!hook.name.is_empty());
            assert!(!hook.description.is_empty());
            assert!(!hook.command.is_empty());
        }
    }

    // ---------- detect with non-existent path ----------

    #[test]
    fn detect_returns_none_for_nonexistent_path() {
        assert_eq!(
            detect_hook_manager(Path::new("/tmp/anvil_nonexistent_xyz_999999")),
            HookManager::None
        );
    }
}
