pub mod render;

use eddacraft_tui::keyboard::Action;

/// Which view is active in the template browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserView {
    Categories,
    Templates,
    Detail,
}

impl BrowserView {
    #[must_use]
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Categories => Some(Self::Templates),
            Self::Templates => Some(Self::Detail),
            Self::Detail => None,
        }
    }

    #[must_use]
    pub fn prev(self) -> Option<Self> {
        match self {
            Self::Categories => None,
            Self::Templates => Some(Self::Categories),
            Self::Detail => Some(Self::Templates),
        }
    }
}

/// A template category grouping.
#[derive(Debug, Clone)]
pub struct TemplateCategory {
    pub name: String,
    pub description: String,
    pub template_count: usize,
}

/// A single template entry.
#[derive(Debug, Clone)]
pub struct TemplateEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub variables: Vec<TemplateVariable>,
}

/// A variable required by a template.
#[derive(Debug, Clone)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub default_value: Option<String>,
    pub required: bool,
}

/// State for the template browser surface.
pub struct BrowserState {
    pub categories: Vec<TemplateCategory>,
    pub templates: Vec<TemplateEntry>,
    pub view: BrowserView,
    pub cat_selected: usize,
    pub tmpl_selected: usize,
    pub var_selected: usize,
    pub search_term: String,
    pub search_mode: bool,
    pub should_quit: bool,
    pub wants_back: bool,
    pub chosen: Option<String>,
}

impl BrowserState {
    pub fn surface_name(&self) -> &'static str {
        "b r o w s e r"
    }

    pub fn help_text(&self) -> &'static str {
        if self.search_mode {
            "type to search  esc cancel"
        } else {
            match self.view {
                BrowserView::Categories => "j/k navigate  enter select  / search  q quit",
                BrowserView::Templates => "j/k navigate  enter detail  esc back  / search  q quit",
                BrowserView::Detail => "j/k navigate  esc back  q quit",
            }
        }
    }

    pub fn new(categories: Vec<TemplateCategory>, templates: Vec<TemplateEntry>) -> Self {
        Self {
            categories,
            templates,
            view: BrowserView::Categories,
            cat_selected: 0,
            tmpl_selected: 0,
            var_selected: 0,
            search_term: String::new(),
            search_mode: false,
            should_quit: false,
            wants_back: false,
            chosen: None,
        }
    }

    /// Get templates filtered by the currently selected category and search term.
    pub fn filtered_templates(&self) -> Vec<&TemplateEntry> {
        let cat_name = self
            .categories
            .get(self.cat_selected)
            .map(|c| c.name.as_str());

        self.templates
            .iter()
            .filter(|t| {
                cat_name.is_some_and(|cat| t.category == cat)
                    && (self.search_term.is_empty()
                        || t.name
                            .to_lowercase()
                            .contains(&self.search_term.to_lowercase())
                        || t.tags.iter().any(|tag| {
                            tag.to_lowercase()
                                .contains(&self.search_term.to_lowercase())
                        }))
            })
            .collect()
    }

    /// Get the currently selected template (if in Templates or Detail view).
    pub fn selected_template(&self) -> Option<&TemplateEntry> {
        let filtered = self.filtered_templates();
        filtered.get(self.tmpl_selected).copied()
    }

    pub fn handle_key(&mut self, action: Action) {
        if self.search_mode {
            self.handle_search_key(action);
            return;
        }

        match self.view {
            BrowserView::Categories => self.handle_categories_key(action),
            BrowserView::Templates => self.handle_templates_key(action),
            BrowserView::Detail => self.handle_detail_key(action),
        }
    }

    fn handle_search_key(&mut self, action: Action) {
        match action {
            Action::Character(c) => {
                self.search_term.push(c);
                self.tmpl_selected = 0;
            }
            Action::Backspace => {
                self.search_term.pop();
                self.tmpl_selected = 0;
            }
            Action::Select => {
                self.search_mode = false;
            }
            Action::Back => {
                self.search_mode = false;
                self.search_term.clear();
                self.tmpl_selected = 0;
            }
            _ => {}
        }
    }

    fn handle_categories_key(&mut self, action: Action) {
        match action {
            Action::Up if self.cat_selected > 0 => {
                self.cat_selected -= 1;
            }
            Action::Down if self.cat_selected < self.categories.len().saturating_sub(1) => {
                self.cat_selected += 1;
            }
            Action::Select | Action::Right if !self.categories.is_empty() => {
                self.view = BrowserView::Templates;
                self.tmpl_selected = 0;
                self.search_term.clear();
            }
            Action::Back => self.wants_back = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_templates_key(&mut self, action: Action) {
        let filtered_count = self.filtered_templates().len();

        match action {
            Action::Up if self.tmpl_selected > 0 => {
                self.tmpl_selected -= 1;
            }
            Action::Down if self.tmpl_selected < filtered_count.saturating_sub(1) => {
                self.tmpl_selected += 1;
            }
            Action::Select | Action::Right if filtered_count > 0 => {
                self.view = BrowserView::Detail;
                self.var_selected = 0;
            }
            Action::Back | Action::Left => {
                self.view = BrowserView::Categories;
            }
            Action::Character('/') => {
                self.search_mode = true;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, action: Action) {
        let var_count = self.selected_template().map_or(0, |t| t.variables.len());

        match action {
            Action::Up if self.var_selected > 0 => {
                self.var_selected -= 1;
            }
            Action::Down if self.var_selected < var_count.saturating_sub(1) => {
                self.var_selected += 1;
            }
            Action::Select => {
                if let Some(t) = self.selected_template() {
                    self.chosen = Some(t.id.clone());
                }
            }
            Action::Back | Action::Left => {
                self.view = BrowserView::Templates;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

impl crate::surface::Surface for BrowserState {
    fn surface_name(&self) -> &'static str {
        "Browser"
    }

    fn help_text(&self) -> &'static str {
        if self.search_mode {
            "type to search  enter confirm  esc cancel"
        } else {
            match self.view {
                BrowserView::Categories => "j/k navigate  enter/l drill in  esc back  q quit",
                BrowserView::Templates => {
                    "j/k navigate  enter/l detail  esc/h back  /search  q quit"
                }
                BrowserView::Detail => "j/k navigate  enter select  esc/h back  q quit",
            }
        }
    }

    fn handle_key(&mut self, action: Action) {
        self.handle_key(action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit || self.chosen.is_some()
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        self.should_quit = false;
        self.wants_back = false;
        self.chosen = None;
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

    fn sample_categories() -> Vec<TemplateCategory> {
        vec![
            TemplateCategory {
                name: "Starter".to_string(),
                description: "Basic project templates".to_string(),
                template_count: 2,
            },
            TemplateCategory {
                name: "Advanced".to_string(),
                description: "Full-featured templates".to_string(),
                template_count: 1,
            },
        ]
    }

    fn sample_templates() -> Vec<TemplateEntry> {
        vec![
            TemplateEntry {
                id: "basic-ts".to_string(),
                name: "Basic TypeScript".to_string(),
                description: "Minimal TypeScript project".to_string(),
                category: "Starter".to_string(),
                tags: vec!["typescript".to_string(), "minimal".to_string()],
                variables: vec![TemplateVariable {
                    name: "project_name".to_string(),
                    description: "Name of the project".to_string(),
                    default_value: Some("my-project".to_string()),
                    required: true,
                }],
            },
            TemplateEntry {
                id: "react-app".to_string(),
                name: "React App".to_string(),
                description: "React with anvil gates".to_string(),
                category: "Starter".to_string(),
                tags: vec!["react".to_string(), "frontend".to_string()],
                variables: vec![],
            },
            TemplateEntry {
                id: "monorepo".to_string(),
                name: "Monorepo".to_string(),
                description: "Multi-package workspace".to_string(),
                category: "Advanced".to_string(),
                tags: vec!["monorepo".to_string(), "nx".to_string()],
                variables: vec![],
            },
        ]
    }

    #[test]
    fn starts_at_categories() {
        let state = BrowserState::new(sample_categories(), sample_templates());
        assert_eq!(state.view, BrowserView::Categories);
    }

    #[test]
    fn drill_categories_to_templates() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.handle_key(Action::Select);
        assert_eq!(state.view, BrowserView::Templates);
    }

    #[test]
    fn drill_templates_to_detail() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.view = BrowserView::Templates;
        state.handle_key(Action::Select);
        assert_eq!(state.view, BrowserView::Detail);
    }

    #[test]
    fn back_from_templates_to_categories() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.view = BrowserView::Templates;
        state.handle_key(Action::Back);
        assert_eq!(state.view, BrowserView::Categories);
    }

    #[test]
    fn back_from_detail_to_templates() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.view = BrowserView::Detail;
        state.handle_key(Action::Back);
        assert_eq!(state.view, BrowserView::Templates);
    }

    #[test]
    fn back_from_categories_exits_surface() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.handle_key(Action::Back);
        assert_eq!(state.view, BrowserView::Categories); // view unchanged
        assert!(state.wants_back); // signals exit to parent
    }

    #[test]
    fn search_filters_templates() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.view = BrowserView::Templates;

        // Enter search mode
        state.handle_key(Action::Character('/'));
        assert!(state.search_mode);

        // Type "react"
        state.handle_key(Action::Character('r'));
        state.handle_key(Action::Character('e'));
        state.handle_key(Action::Character('a'));
        state.handle_key(Action::Character('c'));
        state.handle_key(Action::Character('t'));

        let filtered = state.filtered_templates();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "React App");
    }

    #[test]
    fn search_escape_clears() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.view = BrowserView::Templates;
        state.handle_key(Action::Character('/'));
        state.handle_key(Action::Character('x'));
        assert_eq!(state.search_term, "x");

        state.handle_key(Action::Back); // cancel search
        assert!(!state.search_mode);
        assert!(state.search_term.is_empty());
    }

    #[test]
    fn template_selection() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.view = BrowserView::Detail;
        state.handle_key(Action::Select);
        assert_eq!(state.chosen, Some("basic-ts".to_string()));
    }

    #[test]
    fn category_navigation_bounds() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.handle_key(Action::Up); // at 0
        assert_eq!(state.cat_selected, 0);

        for _ in 0..10 {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.cat_selected, 1); // max index
    }

    #[test]
    fn view_next_prev() {
        assert_eq!(BrowserView::Categories.next(), Some(BrowserView::Templates));
        assert_eq!(BrowserView::Templates.next(), Some(BrowserView::Detail));
        assert_eq!(BrowserView::Detail.next(), None);

        assert_eq!(BrowserView::Categories.prev(), None);
        assert_eq!(BrowserView::Templates.prev(), Some(BrowserView::Categories));
        assert_eq!(BrowserView::Detail.prev(), Some(BrowserView::Templates));
    }

    #[test]
    fn search_by_tag() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.view = BrowserView::Templates;
        state.search_term = "frontend".to_string();

        let filtered = state.filtered_templates();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "react-app");
    }

    #[test]
    fn filtered_respects_selected_category() {
        let mut state = BrowserState::new(sample_categories(), sample_templates());
        state.cat_selected = 1; // Advanced
        let filtered = state.filtered_templates();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "monorepo");
    }
}
