use anvil_kernel_types::hooks::is_anvil_managed_command;
use anvil_kernel_types::{Notification, NotificationClass, NotificationPriority};
use eddacraft_tui::keyboard::Action;

use crate::surfaces::notifications::{NotificationSource, surface_notification};

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
    /// Git 2.54 native `hook.<event>.command` config-mode entries
    /// (introduced by GHOOK-002). Detected as a peer to Husky — neither
    /// is preferred over the other; whichever the user already set up wins
    /// the precedence check.
    ConfigHooks,
}

impl HookManager {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Husky => "Husky",
            Self::Lefthook => "Lefthook",
            Self::PreCommit => "pre-commit framework",
            Self::ConfigHooks => "Git config hooks",
        }
    }

    pub fn adapter_note(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Husky => Some("Anvil will add entries to your existing Husky hooks"),
            Self::Lefthook => Some("Anvil will add a run entry to your lefthook.yml"),
            Self::PreCommit => Some("Anvil will add a hook entry to your .pre-commit-config.yaml"),
            Self::ConfigHooks => Some("Anvil will manage your Git config-mode hooks (Git 2.54+)"),
        }
    }
}

/// Detect which hook manager (if any) is present under `project_dir`.
///
/// Precedence: `Husky` and `ConfigHooks` are peers — whichever is detected
/// first by the on-disk markers wins. We probe Husky first because the
/// `.husky/` directory is the historic default for projects that arrived
/// via `npx husky init` (the most common path today). Config-mode hooks
/// are checked via `git config --get-all hook.pre-commit.command`; any
/// output (Anvil-managed or user-authored) flips the detector. Falls
/// through to `Lefthook` and `pre-commit framework` for parity with the
/// pre-GHOOK-003 behaviour.
pub fn detect_hook_manager(project_dir: &std::path::Path) -> HookManager {
    if project_dir.join(".husky").exists() {
        return HookManager::Husky;
    }
    if has_config_mode_hook(project_dir, "pre-commit") {
        return HookManager::ConfigHooks;
    }
    if project_dir.join(".lefthook.yml").exists() || project_dir.join("lefthook.yml").exists() {
        HookManager::Lefthook
    } else if project_dir.join(".pre-commit-config.yaml").exists() {
        HookManager::PreCommit
    } else {
        HookManager::None
    }
}

/// True when at least one `hook.<event>.command` config-mode entry exists
/// for `event` in the repo at `project_dir`.
///
/// Best-effort: a missing or non-zero `git` invocation yields `false` so
/// onboarding never fails because of an environment without `git` on PATH.
/// Reuses [`is_anvil_managed_command`] from `anvil_kernel_types` only for
/// callers that need to distinguish Anvil-owned entries from user-authored
/// ones — the bare detection here returns `true` for either flavour, which
/// matches the precedence contract: any config-mode entry is treated as a
/// hook source.
fn has_config_mode_hook(project_dir: &std::path::Path, event: &str) -> bool {
    !list_config_mode_hook_commands(project_dir, event).is_empty()
}

/// Read every `hook.<event>.command` entry for `event` in the repo at
/// `project_dir`. Returns an empty vector when `git` is missing or the
/// invocation fails for any reason. Does not propagate errors — onboarding
/// is meant to be deterministic and read-only.
fn list_config_mode_hook_commands(project_dir: &std::path::Path, event: &str) -> Vec<String> {
    let key = format!("hook.{event}.command");
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .args(["config", "--get-all", &key])
        .output();
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(ToString::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// True when the repo at `project_dir` has at least one Anvil-managed
/// `hook.pre-commit.command` entry. Convenience accessor used by callers
/// (and tests) that want to distinguish Anvil-installed config hooks from
/// user-authored ones.
#[must_use]
pub fn has_anvil_config_hook(project_dir: &std::path::Path) -> bool {
    list_config_mode_hook_commands(project_dir, "pre-commit")
        .iter()
        .any(|cmd| is_anvil_managed_command(cmd))
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

impl NotificationSource for HooksState {
    fn notifications(&self) -> Vec<Notification> {
        let mut out = Vec::new();

        if let Some(err) = self.install_error.as_ref() {
            out.push(surface_notification(
                "onboarding-hooks",
                NotificationClass::Failure,
                NotificationPriority::High,
                "Hook install failed",
                err.clone(),
            ));
        } else if self.installed {
            out.push(surface_notification(
                "onboarding-hooks",
                NotificationClass::Info,
                NotificationPriority::Low,
                "Hooks installed",
                format!(
                    "Installed {} hook(s) via {}",
                    self.selected_hooks.iter().filter(|v| **v).count(),
                    self.hook_manager.label()
                ),
            ));
        }

        // Only warn about the missing hook manager while installation has not
        // succeeded. Once `installed == true` with `HookManager::None`, the
        // fallback to raw `.git/hooks` worked and the warning would contradict
        // the "Hooks installed" notification above.
        if self.hook_manager == HookManager::None && !self.installed {
            out.push(surface_notification(
                "onboarding-hooks",
                NotificationClass::Warning,
                NotificationPriority::Normal,
                "No hook manager detected",
                "Anvil will install raw git hooks under .git/hooks.",
            ));
        }

        out
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
        assert_eq!(HookManager::ConfigHooks.label(), "Git config hooks");
    }

    #[test]
    fn hook_manager_adapter_notes() {
        assert!(HookManager::None.adapter_note().is_none());
        assert!(HookManager::Husky.adapter_note().is_some());
        assert!(HookManager::Lefthook.adapter_note().is_some());
        assert!(HookManager::PreCommit.adapter_note().is_some());
        assert!(HookManager::ConfigHooks.adapter_note().is_some());
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

    // ---------- NotificationSource impl ----------

    #[test]
    fn notifications_warns_when_no_hook_manager() {
        let dir = empty_dir();
        let state = HooksState::new(&dir);
        let notifications = state.notifications();
        assert!(
            notifications
                .iter()
                .any(|n| n.title == "No hook manager detected"
                    && n.class == NotificationClass::Warning),
            "expected warning notification"
        );
        cleanup(&dir);
    }

    #[test]
    fn notifications_reports_installed() {
        let dir = empty_dir();
        std::fs::create_dir_all(dir.join(".husky")).unwrap();
        let mut state = HooksState::new(&dir);
        state.mark_installed();
        let notifications = state.notifications();
        assert!(
            notifications
                .iter()
                .any(|n| n.title == "Hooks installed" && n.class == NotificationClass::Info),
            "expected installed notification"
        );
        assert!(
            !notifications
                .iter()
                .any(|n| n.title == "No hook manager detected"),
            "detected hook manager should suppress warning"
        );
        cleanup(&dir);
    }

    #[test]
    fn notifications_reports_install_failure() {
        let dir = empty_dir();
        std::fs::create_dir_all(dir.join(".husky")).unwrap();
        let mut state = HooksState::new(&dir);
        state.mark_failed("git returned error".to_string());
        let notifications = state.notifications();
        let failure = notifications
            .iter()
            .find(|n| n.class == NotificationClass::Failure)
            .expect("failure notification present");
        assert_eq!(failure.priority, NotificationPriority::High);
        assert_eq!(failure.title, "Hook install failed");
        cleanup(&dir);
    }

    #[test]
    fn notifications_do_not_warn_when_installed_with_no_hook_manager() {
        // Council finding: without this, installed=true + hook_manager=None
        // emits both "Hooks installed" (Info) and "No hook manager detected"
        // (Warning), which is contradictory from the operator's view.
        let dir = empty_dir();
        let mut state = HooksState::new(&dir);
        assert_eq!(state.hook_manager, HookManager::None);
        state.mark_installed();
        let notifications = state.notifications();
        assert!(
            notifications.iter().any(|n| n.title == "Hooks installed"),
            "installed notification must be present",
        );
        assert!(
            !notifications
                .iter()
                .any(|n| n.title == "No hook manager detected"),
            "warning must be suppressed after successful install",
        );
        cleanup(&dir);
    }

    // ---------- GHOOK-003: config-mode hook detection ----------

    /// Initialise a real git repo at `dir`. Returns whether init succeeded
    /// — tests skip when `git` is missing rather than failing, since
    /// onboarding detection is not exercisable without a real `.git/config`.
    fn try_git_init(dir: &std::path::Path) -> bool {
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn add_config_hook(dir: &std::path::Path, event: &str, command: &str) {
        let key = format!("hook.{event}.command");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["config", "--add", &key, command])
            .status()
            .expect("git config --add");
        assert!(status.success(), "failed to seed config-mode hook in tests");
    }

    #[test]
    fn detect_config_hooks_when_present() {
        let dir = empty_dir();
        if !try_git_init(&dir) {
            eprintln!("skipping: git init unavailable");
            cleanup(&dir);
            return;
        }
        add_config_hook(&dir, "pre-commit", "ANVIL_HOOK=1 anvil gate --progress");

        assert_eq!(detect_hook_manager(&dir), HookManager::ConfigHooks);

        cleanup(&dir);
    }

    #[test]
    fn detect_config_hooks_with_user_authored_entry() {
        // Any `hook.pre-commit.command` entry — even a non-Anvil one —
        // counts as a hook source for precedence purposes. Anvil should
        // not pretend the repo has no hook just because it did not install
        // the entry itself.
        let dir = empty_dir();
        if !try_git_init(&dir) {
            eprintln!("skipping: git init unavailable");
            cleanup(&dir);
            return;
        }
        add_config_hook(&dir, "pre-commit", "npm run my-gate");

        assert_eq!(detect_hook_manager(&dir), HookManager::ConfigHooks);

        cleanup(&dir);
    }

    #[test]
    fn husky_takes_precedence_over_config_hooks() {
        // Husky and ConfigHooks are peers, but on-disk Husky wins the
        // first-match check — preserves the existing precedence contract
        // for repos that have both.
        let dir = empty_dir();
        if !try_git_init(&dir) {
            eprintln!("skipping: git init unavailable");
            cleanup(&dir);
            return;
        }
        std::fs::create_dir_all(dir.join(".husky")).unwrap();
        add_config_hook(&dir, "pre-commit", "ANVIL_HOOK=1 anvil gate --progress");

        assert_eq!(detect_hook_manager(&dir), HookManager::Husky);

        cleanup(&dir);
    }

    #[test]
    fn config_hooks_take_precedence_over_lefthook() {
        // Without Husky in the mix, a config-mode entry must win over a
        // lefthook.yml stub — we promote config-mode to a peer of Husky
        // rather than a tail-end fallback.
        let dir = empty_dir();
        if !try_git_init(&dir) {
            eprintln!("skipping: git init unavailable");
            cleanup(&dir);
            return;
        }
        std::fs::write(dir.join("lefthook.yml"), "").unwrap();
        add_config_hook(&dir, "pre-commit", "ANVIL_HOOK=1 anvil gate --progress");

        assert_eq!(detect_hook_manager(&dir), HookManager::ConfigHooks);

        cleanup(&dir);
    }

    #[test]
    fn has_anvil_config_hook_distinguishes_owners() {
        let dir = empty_dir();
        if !try_git_init(&dir) {
            eprintln!("skipping: git init unavailable");
            cleanup(&dir);
            return;
        }

        assert!(
            !has_anvil_config_hook(&dir),
            "fresh repo must report no Anvil-managed config hook",
        );

        add_config_hook(&dir, "pre-commit", "npm run my-gate");
        assert!(
            !has_anvil_config_hook(&dir),
            "user-authored entries must not be reported as Anvil-managed",
        );

        add_config_hook(&dir, "pre-commit", "ANVIL_HOOK=1 anvil gate --progress");
        assert!(
            has_anvil_config_hook(&dir),
            "Anvil-managed entry must be detected once installed",
        );

        cleanup(&dir);
    }

    #[test]
    fn has_anvil_config_hook_returns_false_outside_git_repo() {
        // No `.git` → `git config --get-all` fails → empty result. The
        // detector must not panic or propagate the error; onboarding is
        // read-only and best-effort.
        let dir = empty_dir();
        assert!(!has_anvil_config_hook(&dir));
        cleanup(&dir);
    }
}
