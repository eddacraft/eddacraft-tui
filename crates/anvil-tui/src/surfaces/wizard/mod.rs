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
    pub text_input: TextInputState,
    pub should_quit: bool,
    pub confirmed: bool,
}

impl WizardState {
    pub fn new(templates: Vec<Template>) -> Self {
        Self {
            step: WizardStep::TemplateSelect,
            templates,
            template_selected: 0,
            config: WizardConfig::default(),
            text_input: TextInputState::default(),
            should_quit: false,
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
            Action::Up => {
                if self.template_selected > 0 {
                    self.template_selected -= 1;
                }
            }
            Action::Down => {
                if self.template_selected < self.templates.len().saturating_sub(1) {
                    self.template_selected += 1;
                }
            }
            Action::Select => {
                if let Some(t) = self.templates.get(self.template_selected) {
                    self.config.template_id = Some(t.id.clone());
                    self.step = WizardStep::ProjectName;
                }
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_name_key(&mut self, action: Action) {
        match action {
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
            Action::Select => {
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
    fn step_regression() {
        assert_eq!(WizardStep::TemplateSelect.prev(), None);
        assert_eq!(
            WizardStep::ProjectName.prev(),
            Some(WizardStep::TemplateSelect)
        );
    }
}
