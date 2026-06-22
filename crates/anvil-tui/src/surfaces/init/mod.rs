pub mod render;

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::widgets::text_input::TextInputState;

/// Init wizard step progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitStep {
    Mode,
    Format,
    Directory,
    Checks,
    Summary,
}

impl InitStep {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mode => "Mode",
            Self::Format => "Format",
            Self::Directory => "Directory",
            Self::Checks => "Checks",
            Self::Summary => "Summary",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Mode => 0,
            Self::Format => 1,
            Self::Directory => 2,
            Self::Checks => 3,
            Self::Summary => 4,
        }
    }

    pub const TOTAL: usize = 5;

    pub fn next(self) -> Option<Self> {
        match self {
            Self::Mode => Some(Self::Format),
            Self::Format => Some(Self::Directory),
            Self::Directory => Some(Self::Checks),
            Self::Checks => Some(Self::Summary),
            Self::Summary => None,
        }
    }

    pub fn prev(self) -> Option<Self> {
        match self {
            Self::Mode => None,
            Self::Format => Some(Self::Mode),
            Self::Directory => Some(Self::Format),
            Self::Checks => Some(Self::Directory),
            Self::Summary => Some(Self::Checks),
        }
    }
}

/// Project initialisation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitMode {
    New,
    Existing,
    Minimal,
}

impl InitMode {
    pub const ALL: [Self; 3] = [Self::New, Self::Existing, Self::Minimal];

    pub fn label(self) -> &'static str {
        match self {
            Self::New => "New project",
            Self::Existing => "Existing project",
            Self::Minimal => "Minimal setup",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::New => "Create a new project with full anvil configuration",
            Self::Existing => "Add anvil to an existing codebase",
            Self::Minimal => "Bare-bones config with only essential checks",
        }
    }
}

/// Configuration file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Yaml,
    Json,
    Toml,
}

impl ConfigFormat {
    pub const ALL: [Self; 3] = [Self::Yaml, Self::Json, Self::Toml];

    pub fn label(self) -> &'static str {
        match self {
            Self::Yaml => "YAML (.anvil.yaml) (default)",
            Self::Json => "JSON (.anvil.json)",
            Self::Toml => "TOML (.anvil.toml)",
        }
    }
}

/// Available check that can be toggled on or off.
#[derive(Debug, Clone)]
pub struct AvailableCheck {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Configuration built during the wizard.
#[derive(Debug, Clone)]
pub struct InitConfig {
    pub mode: InitMode,
    pub format: ConfigFormat,
    pub directory: String,
    pub checks: Vec<String>,
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            mode: InitMode::New,
            format: ConfigFormat::Yaml,
            directory: ".".to_string(),
            checks: Vec::new(),
        }
    }
}

/// State for the init wizard surface.
pub struct InitState {
    pub step: InitStep,
    pub config: InitConfig,
    pub mode_selected: usize,
    pub format_selected: usize,
    pub text_input: TextInputState,
    pub check_toggles: Vec<AvailableCheck>,
    pub check_selected: usize,
    pub should_quit: bool,
    pub wants_back: bool,
    pub confirmed: bool,
}

impl InitState {
    pub fn surface_name(&self) -> &'static str {
        "i n i t"
    }

    pub fn help_text(&self) -> &'static str {
        match self.step {
            InitStep::Mode | InitStep::Format => "j/k navigate  enter select  esc back  q quit",
            InitStep::Directory => "type directory  enter next  esc back  q quit",
            InitStep::Checks => "j/k navigate  space toggle  enter next  esc back  q quit",
            InitStep::Summary => "enter confirm  esc back  q quit",
        }
    }

    pub fn new(available_checks: Vec<AvailableCheck>) -> Self {
        Self {
            step: InitStep::Mode,
            config: InitConfig::default(),
            mode_selected: 0,
            format_selected: 0,
            text_input: TextInputState::default(),
            check_toggles: available_checks,
            check_selected: 0,
            should_quit: false,
            wants_back: false,
            confirmed: false,
        }
    }

    pub fn handle_key(&mut self, action: Action) {
        match self.step {
            InitStep::Mode => self.handle_mode_key(action),
            InitStep::Format => self.handle_format_key(action),
            InitStep::Directory => self.handle_directory_key(action),
            InitStep::Checks => self.handle_checks_key(action),
            InitStep::Summary => self.handle_summary_key(action),
        }
    }

    fn handle_mode_key(&mut self, action: Action) {
        match action {
            Action::Up if self.mode_selected > 0 => {
                self.mode_selected -= 1;
            }
            Action::Down if self.mode_selected < InitMode::ALL.len() - 1 => {
                self.mode_selected += 1;
            }
            Action::Select => {
                self.config.mode = InitMode::ALL[self.mode_selected];
                self.step = InitStep::Format;
            }
            Action::Back => self.wants_back = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_format_key(&mut self, action: Action) {
        match action {
            Action::Up if self.format_selected > 0 => {
                self.format_selected -= 1;
            }
            Action::Down if self.format_selected < ConfigFormat::ALL.len() - 1 => {
                self.format_selected += 1;
            }
            Action::Select => {
                self.config.format = ConfigFormat::ALL[self.format_selected];
                self.step = InitStep::Directory;
            }
            Action::Back => {
                self.step = InitStep::Mode;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_directory_key(&mut self, action: Action) {
        match action {
            Action::Character(c) => {
                self.text_input.insert(c);
            }
            Action::Backspace => {
                self.text_input.backspace();
            }
            Action::Delete => {
                self.text_input.delete();
            }
            Action::Home => {
                self.text_input.home();
            }
            Action::End => {
                self.text_input.end();
            }
            Action::Left => {
                self.text_input.move_left();
            }
            Action::Right => {
                self.text_input.move_right();
            }
            Action::Select => {
                self.config.directory = if self.text_input.value.is_empty() {
                    ".".to_string()
                } else {
                    self.text_input.value.clone()
                };
                self.step = InitStep::Checks;
            }
            Action::Back => {
                self.step = InitStep::Format;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_checks_key(&mut self, action: Action) {
        match action {
            Action::Up if self.check_selected > 0 => {
                self.check_selected -= 1;
            }
            Action::Down if self.check_selected < self.check_toggles.len().saturating_sub(1) => {
                self.check_selected += 1;
            }
            Action::Toggle => {
                if let Some(check) = self.check_toggles.get_mut(self.check_selected) {
                    check.enabled = !check.enabled;
                }
            }
            Action::Select => {
                self.config.checks = self
                    .check_toggles
                    .iter()
                    .filter(|c| c.enabled)
                    .map(|c| c.name.clone())
                    .collect();
                self.step = InitStep::Summary;
            }
            Action::Back => {
                self.step = InitStep::Directory;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_summary_key(&mut self, action: Action) {
        match action {
            Action::Select => {
                self.confirmed = true;
            }
            Action::Back => {
                self.step = InitStep::Checks;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

impl crate::surface::Surface for InitState {
    fn surface_name(&self) -> &'static str {
        "Init"
    }

    fn help_text(&self) -> &'static str {
        match self.step {
            InitStep::Mode | InitStep::Format => "j/k navigate  enter select  esc back  q quit",
            InitStep::Directory => "type path  enter confirm  esc back  q quit",
            InitStep::Checks => "j/k navigate  space toggle  enter confirm  esc back  q quit",
            InitStep::Summary => "enter confirm  esc back  q quit",
        }
    }

    fn handle_key(&mut self, action: Action) {
        self.handle_key(action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit || self.confirmed
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        self.should_quit = false;
        self.wants_back = false;
        self.confirmed = false;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_checks() -> Vec<AvailableCheck> {
        vec![
            AvailableCheck {
                name: "eslint".to_string(),
                description: "JavaScript/TypeScript linting".to_string(),
                enabled: true,
            },
            AvailableCheck {
                name: "secret-scan".to_string(),
                description: "Detect leaked secrets".to_string(),
                enabled: true,
            },
            AvailableCheck {
                name: "architecture".to_string(),
                description: "Module boundary enforcement".to_string(),
                enabled: false,
            },
        ]
    }

    #[test]
    fn starts_at_mode_step() {
        let state = InitState::new(sample_checks());
        assert_eq!(state.step, InitStep::Mode);
        assert!(!state.should_quit);
        assert!(!state.confirmed);
    }

    #[test]
    fn step_progression_forward() {
        assert_eq!(InitStep::Mode.next(), Some(InitStep::Format));
        assert_eq!(InitStep::Format.next(), Some(InitStep::Directory));
        assert_eq!(InitStep::Directory.next(), Some(InitStep::Checks));
        assert_eq!(InitStep::Checks.next(), Some(InitStep::Summary));
        assert_eq!(InitStep::Summary.next(), None);
    }

    #[test]
    fn step_progression_back() {
        assert_eq!(InitStep::Mode.prev(), None);
        assert_eq!(InitStep::Format.prev(), Some(InitStep::Mode));
        assert_eq!(InitStep::Directory.prev(), Some(InitStep::Format));
        assert_eq!(InitStep::Checks.prev(), Some(InitStep::Directory));
        assert_eq!(InitStep::Summary.prev(), Some(InitStep::Checks));
    }

    #[test]
    fn mode_selection_advances_to_format() {
        let mut state = InitState::new(sample_checks());
        state.handle_key(Action::Down); // Existing
        state.handle_key(Action::Select);
        assert_eq!(state.step, InitStep::Format);
        assert_eq!(state.config.mode, InitMode::Existing);
    }

    #[test]
    fn format_selection_advances_to_directory() {
        let mut state = InitState::new(sample_checks());
        state.step = InitStep::Format;
        state.handle_key(Action::Down); // Json
        state.handle_key(Action::Select);
        assert_eq!(state.step, InitStep::Directory);
        assert_eq!(state.config.format, ConfigFormat::Json);
    }

    #[test]
    fn directory_text_input() {
        let mut state = InitState::new(sample_checks());
        state.step = InitStep::Directory;
        state.handle_key(Action::Character('s'));
        state.handle_key(Action::Character('r'));
        state.handle_key(Action::Character('c'));
        assert_eq!(state.text_input.value, "src");
    }

    #[test]
    fn directory_empty_defaults_to_dot() {
        let mut state = InitState::new(sample_checks());
        state.step = InitStep::Directory;
        state.handle_key(Action::Select);
        assert_eq!(state.config.directory, ".");
        assert_eq!(state.step, InitStep::Checks);
    }

    #[test]
    fn check_toggle_flips_values() {
        let mut state = InitState::new(sample_checks());
        state.step = InitStep::Checks;

        // architecture starts disabled
        state.handle_key(Action::Down); // secret-scan
        state.handle_key(Action::Down); // architecture
        assert!(!state.check_toggles[2].enabled);

        state.handle_key(Action::Toggle);
        assert!(state.check_toggles[2].enabled);

        state.handle_key(Action::Toggle);
        assert!(!state.check_toggles[2].enabled);
    }

    #[test]
    fn checks_advance_collects_enabled() {
        let mut state = InitState::new(sample_checks());
        state.step = InitStep::Checks;
        state.handle_key(Action::Select);
        assert_eq!(state.step, InitStep::Summary);
        // eslint and secret-scan are enabled by default
        assert_eq!(state.config.checks.len(), 2);
        assert!(state.config.checks.contains(&"eslint".to_string()));
        assert!(state.config.checks.contains(&"secret-scan".to_string()));
    }

    #[test]
    fn summary_confirm_sets_flag() {
        let mut state = InitState::new(sample_checks());
        state.step = InitStep::Summary;
        state.handle_key(Action::Select);
        assert!(state.confirmed);
    }

    #[test]
    fn back_from_first_step_exits_surface() {
        let mut state = InitState::new(sample_checks());
        state.handle_key(Action::Back);
        assert_eq!(state.step, InitStep::Mode); // step unchanged
        assert!(state.wants_back); // signals exit to parent
    }

    #[test]
    fn back_from_format_returns_to_mode() {
        let mut state = InitState::new(sample_checks());
        state.step = InitStep::Format;
        state.handle_key(Action::Back);
        assert_eq!(state.step, InitStep::Mode);
    }

    #[test]
    fn mode_navigate_bounds() {
        let mut state = InitState::new(sample_checks());
        state.handle_key(Action::Up); // already at 0
        assert_eq!(state.mode_selected, 0);

        for _ in 0..10 {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.mode_selected, InitMode::ALL.len() - 1);
    }
}
