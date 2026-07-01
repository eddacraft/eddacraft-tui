pub mod render;

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::widgets::text_input::TextInputState;

/// Template available for scaffolding.
#[derive(Debug, Clone)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
}

/// Wizard step progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    TemplateSelect,
    ProjectName,
    Configure,
    Summary,
}

impl WizardStep {
    pub fn label(self) -> &'static str {
        match self {
            Self::TemplateSelect => "Select Template",
            Self::ProjectName => "Project Name",
            Self::Configure => "Configure",
            Self::Summary => "Summary",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::TemplateSelect => 0,
            Self::ProjectName => 1,
            Self::Configure => 2,
            Self::Summary => 3,
        }
    }

    pub const TOTAL: usize = 4;

    pub fn next(self) -> Option<Self> {
        match self {
            Self::TemplateSelect => Some(Self::ProjectName),
            Self::ProjectName => Some(Self::Configure),
            Self::Configure => Some(Self::Summary),
            Self::Summary => None,
        }
    }

    pub fn prev(self) -> Option<Self> {
        match self {
            Self::TemplateSelect => None,
            Self::ProjectName => Some(Self::TemplateSelect),
            Self::Configure => Some(Self::ProjectName),
            Self::Summary => Some(Self::Configure),
        }
    }
}

/// Configuration options set during the wizard.
#[derive(Debug, Clone, Default)]
pub struct WizardConfig {
    pub project_name: String,
    pub template_id: Option<String>,
    pub enable_watch: bool,
    pub enable_hooks: bool,
}

/// State for the APS onboarding wizard.
pub struct WizardState {
    pub step: WizardStep,
    pub templates: Vec<Template>,
    pub template_selected: usize,
    pub config: WizardConfig,
    pub config_selected: usize,
    pub text_input: TextInputState,
    pub should_quit: bool,
    pub wants_back: bool,
    pub confirmed: bool,
}

impl WizardState {
    pub fn surface_name(&self) -> &'static str {
        "w i z a r d"
    }

    pub fn help_text(&self) -> &'static str {
        match self.step {
            WizardStep::TemplateSelect => "j/k navigate  enter select  esc back  q quit",
            WizardStep::ProjectName => "type name  enter next  esc back  ctrl+c quit",
            WizardStep::Configure => "j/k navigate  space toggle  enter next  esc back  q quit",
            WizardStep::Summary => "enter confirm  esc back  q quit",
        }
    }

    pub fn new(templates: Vec<Template>) -> Self {
        Self {
            step: WizardStep::TemplateSelect,
            templates,
            template_selected: 0,
            config: WizardConfig::default(),
            config_selected: 0,
            text_input: TextInputState::default(),
            should_quit: false,
            wants_back: false,
            confirmed: false,
        }
    }

    pub fn handle_key(&mut self, action: Action) {
        match self.step {
            WizardStep::TemplateSelect => self.handle_template_key(action),
            WizardStep::ProjectName => self.handle_name_key(action),
            WizardStep::Configure => self.handle_configure_key(action),
            WizardStep::Summary => self.handle_summary_key(action),
        }
    }

    fn handle_template_key(&mut self, action: Action) {
        match action {
            Action::Up if self.template_selected > 0 => {
                self.template_selected -= 1;
            }
            Action::Down if self.template_selected < self.templates.len().saturating_sub(1) => {
                self.template_selected += 1;
            }
            Action::Select => {
                if let Some(t) = self.templates.get(self.template_selected) {
                    self.config.template_id = Some(t.id.clone());
                    self.step = WizardStep::ProjectName;
                }
            }
            Action::Back => self.wants_back = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_name_key(&mut self, action: Action) {
        match action {
            Action::Character(c) => self.text_input.insert(c),
            Action::Backspace => self.text_input.backspace(),
            Action::Delete => self.text_input.delete(),
            Action::Left => self.text_input.move_left(),
            Action::Right => self.text_input.move_right(),
            Action::Home => self.text_input.home(),
            Action::End => self.text_input.end(),
            Action::Select => {
                self.config.project_name = self.text_input.value.clone();
                if !self.config.project_name.is_empty() {
                    self.step = WizardStep::Configure;
                }
            }
            Action::Back => {
                self.step = WizardStep::TemplateSelect;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_configure_key(&mut self, action: Action) {
        match action {
            Action::Up if self.config_selected > 0 => {
                self.config_selected -= 1;
            }
            Action::Down if self.config_selected < 1 => {
                self.config_selected += 1;
            }
            Action::Toggle | Action::Select => {
                match self.config_selected {
                    0 => self.config.enable_watch = !self.config.enable_watch,
                    1 => self.config.enable_hooks = !self.config.enable_hooks,
                    _ => {}
                }
                // If Select on last option, also advance
                if action == Action::Select && self.config_selected == 1 {
                    self.step = WizardStep::Summary;
                }
            }
            Action::Right => {
                self.step = WizardStep::Summary;
            }
            Action::Back => {
                self.step = WizardStep::ProjectName;
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
                self.step = WizardStep::Configure;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

impl crate::surface::Surface for WizardState {
    fn surface_name(&self) -> &'static str {
        "Wizard"
    }

    fn help_text(&self) -> &'static str {
        match self.step {
            WizardStep::TemplateSelect => "j/k navigate  enter select  esc back  q quit",
            // #2881: this is a free-text field — 'q' is a literal character, so
            // quit is Ctrl+C, not 'q'.
            WizardStep::ProjectName => "type name  enter confirm  esc back  ctrl+c quit",
            WizardStep::Configure => "j/k navigate  space toggle  l/enter next  esc back  q quit",
            WizardStep::Summary => "enter confirm  esc back  q quit",
        }
    }

    fn text_entry_active(&self) -> bool {
        // #2881: only the project-name field captures free text.
        matches!(self.step, WizardStep::ProjectName)
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

    fn sample_templates() -> Vec<Template> {
        vec![
            Template {
                id: "basic".to_string(),
                name: "Basic".to_string(),
                description: "Minimal setup with core checks".to_string(),
                tags: vec!["starter".to_string()],
            },
            Template {
                id: "full".to_string(),
                name: "Full".to_string(),
                description: "All checks, hooks, and watch mode".to_string(),
                tags: vec!["recommended".to_string()],
            },
        ]
    }

    #[test]
    fn starts_at_template_select() {
        let state = WizardState::new(sample_templates());
        assert_eq!(state.step, WizardStep::TemplateSelect);
    }

    #[test]
    fn template_selection_advances_to_name() {
        let mut state = WizardState::new(sample_templates());
        state.handle_key(Action::Select);
        assert_eq!(state.step, WizardStep::ProjectName);
        assert_eq!(state.config.template_id, Some("basic".to_string()));
    }

    #[test]
    fn back_navigation() {
        let mut state = WizardState::new(sample_templates());
        state.step = WizardStep::Configure;
        state.handle_key(Action::Back);
        assert_eq!(state.step, WizardStep::ProjectName);
    }

    #[test]
    fn summary_confirm() {
        let mut state = WizardState::new(sample_templates());
        state.step = WizardStep::Summary;
        state.handle_key(Action::Select);
        assert!(state.confirmed);
    }

    #[test]
    fn step_progression() {
        assert_eq!(
            WizardStep::TemplateSelect.next(),
            Some(WizardStep::ProjectName)
        );
        assert_eq!(WizardStep::ProjectName.next(), Some(WizardStep::Configure));
        assert_eq!(WizardStep::Configure.next(), Some(WizardStep::Summary));
        assert_eq!(WizardStep::Summary.next(), None);
    }

    #[test]
    fn name_step_handles_text_input() {
        let mut state = WizardState::new(sample_templates());
        state.handle_key(Action::Select); // advance to ProjectName
        assert_eq!(state.step, WizardStep::ProjectName);

        state.handle_key(Action::Character('m'));
        state.handle_key(Action::Character('y'));
        state.handle_key(Action::Character('-'));
        state.handle_key(Action::Character('p'));
        assert_eq!(state.text_input.value, "my-p");

        state.handle_key(Action::Backspace);
        assert_eq!(state.text_input.value, "my-");
    }

    #[test]
    fn name_step_requires_non_empty() {
        let mut state = WizardState::new(sample_templates());
        state.step = WizardStep::ProjectName;

        // Enter with empty name should NOT advance
        state.handle_key(Action::Select);
        assert_eq!(state.step, WizardStep::ProjectName);

        // Type something then enter
        state.handle_key(Action::Character('a'));
        state.handle_key(Action::Select);
        assert_eq!(state.step, WizardStep::Configure);
        assert_eq!(state.config.project_name, "a");
    }

    #[test]
    fn configure_step_toggles_options() {
        let mut state = WizardState::new(sample_templates());
        state.step = WizardStep::Configure;

        assert!(!state.config.enable_watch);
        assert!(!state.config.enable_hooks);

        // Toggle watch mode (selected by default at index 0)
        state.handle_key(Action::Toggle);
        assert!(state.config.enable_watch);

        // Navigate to hooks and toggle
        state.handle_key(Action::Down);
        assert_eq!(state.config_selected, 1);
        state.handle_key(Action::Toggle);
        assert!(state.config.enable_hooks);

        // Toggle watch off again
        state.handle_key(Action::Up);
        state.handle_key(Action::Toggle);
        assert!(!state.config.enable_watch);
    }

    #[test]
    fn configure_step_advances_with_right() {
        let mut state = WizardState::new(sample_templates());
        state.step = WizardStep::Configure;
        state.handle_key(Action::Right);
        assert_eq!(state.step, WizardStep::Summary);
    }

    #[test]
    fn step_regression() {
        assert_eq!(WizardStep::TemplateSelect.prev(), None);
        assert_eq!(
            WizardStep::ProjectName.prev(),
            Some(WizardStep::TemplateSelect)
        );
    }
}
