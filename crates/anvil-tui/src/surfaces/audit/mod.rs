pub mod render;

use eddacraft_tui::keyboard::Action;

use super::fix_request::FixRequest;

/// Which panel is focused in the audit view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditPanel {
    Project,
    Issues,
    Historical,
    NextSteps,
}

impl AuditPanel {
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Project => Self::Issues,
            Self::Issues => Self::Historical,
            Self::Historical => Self::NextSteps,
            Self::NextSteps => Self::Project,
        }
    }

    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            Self::Project => Self::NextSteps,
            Self::Issues => Self::Project,
            Self::Historical => Self::Issues,
            Self::NextSteps => Self::Historical,
        }
    }
}

/// Severity level for an audit issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl IssueSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Critical => "CRIT",
            Self::High => "HIGH",
            Self::Medium => "MED",
            Self::Low => "LOW",
            Self::Info => "INFO",
        }
    }

    pub fn label_full(self) -> &'static str {
        match self {
            Self::Critical => "Critical",
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
            Self::Info => "Info",
        }
    }
}

/// A single audit issue.
#[derive(Debug, Clone)]
pub struct AuditIssue {
    pub severity: IssueSeverity,
    pub category: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub fixable: bool,
}

impl AuditIssue {
    #[must_use]
    pub fn fix_request(&self) -> Option<FixRequest> {
        if !self.fixable || self.line == 0 {
            return None;
        }
        if self.message == "console statement found" {
            return Some(FixRequest::AuditConsoleStatement {
                file: self.file.clone(),
                line: self.line,
            });
        }
        None
    }
}

/// A historical audit score entry.
#[derive(Debug, Clone)]
pub struct HistoricalScore {
    pub timestamp: String,
    pub score: f64,
    pub issue_count: usize,
}

/// All data needed by the audit results surface.
#[derive(Debug, Clone)]
pub struct AuditData {
    pub project_name: String,
    pub total_files: usize,
    pub issues: Vec<AuditIssue>,
    pub historical_scores: Vec<HistoricalScore>,
    pub next_steps: Vec<String>,
}

impl AuditData {
    pub fn issue_count_by_severity(&self, severity: IssueSeverity) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == severity)
            .count()
    }
}

/// State for the audit results surface.
pub struct AuditState {
    pub data: AuditData,
    pub focused_panel: AuditPanel,
    pub selected_item: usize,
    pub expanded: bool,
    pub should_quit: bool,
    pub wants_back: bool,
    pub pending_fix: Option<FixRequest>,
}

impl AuditState {
    pub fn new(data: AuditData) -> Self {
        Self {
            data,
            focused_panel: AuditPanel::Project,
            selected_item: 0,
            expanded: false,
            should_quit: false,
            wants_back: false,
            pending_fix: None,
        }
    }

    fn selected_issue_fix_request(&self) -> Option<FixRequest> {
        if self.focused_panel != AuditPanel::Issues {
            return None;
        }
        self.data
            .issues
            .get(self.selected_item)
            .and_then(AuditIssue::fix_request)
    }

    fn max_items_in_panel(&self) -> usize {
        match self.focused_panel {
            AuditPanel::Project => 0,
            AuditPanel::Issues => self.data.issues.len(),
            AuditPanel::Historical => self.data.historical_scores.len(),
            AuditPanel::NextSteps => self.data.next_steps.len(),
        }
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.selected_item > 0 {
                    self.selected_item -= 1;
                    self.expanded = false;
                }
            }
            Action::Down => {
                let max = self.max_items_in_panel().saturating_sub(1);
                if self.selected_item < max {
                    self.selected_item += 1;
                    self.expanded = false;
                }
            }
            Action::Right | Action::PageDown => {
                self.focused_panel = self.focused_panel.next();
                self.selected_item = 0;
                self.expanded = false;
            }
            Action::Left | Action::PageUp => {
                self.focused_panel = self.focused_panel.prev();
                self.selected_item = 0;
                self.expanded = false;
            }
            Action::Select => {
                if self.focused_panel == AuditPanel::Issues && !self.data.issues.is_empty() {
                    self.expanded = !self.expanded;
                }
            }
            Action::Character('f') => {
                if let Some(request) = self.selected_issue_fix_request() {
                    self.pending_fix = Some(request);
                }
            }
            Action::Back => {
                if self.expanded {
                    self.expanded = false;
                } else {
                    self.wants_back = true;
                }
            }
            Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

impl crate::surface::Surface for AuditState {
    fn surface_name(&self) -> &'static str {
        "Audit"
    }

    fn help_text(&self) -> &'static str {
        if self.expanded {
            if self.selected_issue_fix_request().is_some() {
                "j/k navigate  h/l switch panel  f fix  esc collapse  q quit"
            } else {
                "j/k navigate  h/l switch panel  esc collapse  q quit"
            }
        } else {
            if self.selected_issue_fix_request().is_some() {
                "j/k navigate  h/l switch panel  enter expand  f fix  esc back  q quit"
            } else {
                "j/k navigate  h/l switch panel  enter expand  esc back  q quit"
            }
        }
    }

    fn handle_key(&mut self, action: Action) {
        AuditState::handle_key(self, action);
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

    fn sample_data() -> AuditData {
        AuditData {
            project_name: "test-project".to_string(),
            total_files: 42,
            issues: vec![
                AuditIssue {
                    severity: IssueSeverity::Critical,
                    category: "Security".to_string(),
                    message: "Hardcoded API key detected".to_string(),
                    file: "src/config.ts".to_string(),
                    line: 15,
                    fixable: false,
                },
                AuditIssue {
                    severity: IssueSeverity::Medium,
                    category: "Quality".to_string(),
                    message: "console statement found".to_string(),
                    file: "src/utils/db.ts".to_string(),
                    line: 3,
                    fixable: true,
                },
                AuditIssue {
                    severity: IssueSeverity::Low,
                    category: "Quality".to_string(),
                    message: "console statement found".to_string(),
                    file: "src/index.ts".to_string(),
                    line: 1,
                    fixable: true,
                },
            ],
            historical_scores: vec![
                HistoricalScore {
                    timestamp: "2026-03-15".to_string(),
                    score: 0.85,
                    issue_count: 5,
                },
                HistoricalScore {
                    timestamp: "2026-03-16".to_string(),
                    score: 0.90,
                    issue_count: 3,
                },
            ],
            next_steps: vec![
                "Fix critical security issue in src/config.ts".to_string(),
                "Remove console statements from source files".to_string(),
                "Run auto-fix for fixable issues".to_string(),
            ],
        }
    }

    #[test]
    fn panel_navigation_wraps() {
        let mut state = AuditState::new(sample_data());
        assert_eq!(state.focused_panel, AuditPanel::Project);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, AuditPanel::Issues);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, AuditPanel::Historical);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, AuditPanel::NextSteps);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, AuditPanel::Project); // wraps
    }

    #[test]
    fn panel_navigation_backward_wraps() {
        let mut state = AuditState::new(sample_data());
        state.handle_key(Action::Left);
        assert_eq!(state.focused_panel, AuditPanel::NextSteps); // wraps backward
    }

    #[test]
    fn item_selection_within_issues() {
        let mut state = AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;

        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 1);
        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 2);
        state.handle_key(Action::Down); // at max
        assert_eq!(state.selected_item, 2);
        state.handle_key(Action::Up);
        assert_eq!(state.selected_item, 1);
    }

    #[test]
    fn panel_switch_resets_selection() {
        let mut state = AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        state.selected_item = 2;

        state.handle_key(Action::Right);
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn summary_counts_match_data() {
        let data = sample_data();
        assert_eq!(data.issue_count_by_severity(IssueSeverity::Critical), 1);
        assert_eq!(data.issue_count_by_severity(IssueSeverity::Medium), 1);
        assert_eq!(data.issue_count_by_severity(IssueSeverity::Low), 1);
        assert_eq!(data.issue_count_by_severity(IssueSeverity::High), 0);
    }

    #[test]
    fn expand_collapse_toggle() {
        let mut state = AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        assert!(!state.expanded);

        state.handle_key(Action::Select);
        assert!(state.expanded);

        state.handle_key(Action::Select);
        assert!(!state.expanded);
    }

    #[test]
    fn expand_collapses_on_navigation() {
        let mut state = AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        state.handle_key(Action::Select);
        assert!(state.expanded);

        state.handle_key(Action::Down);
        assert!(!state.expanded);
    }

    #[test]
    fn expand_only_works_on_issues_panel() {
        let mut state = AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Project;
        state.handle_key(Action::Select);
        assert!(!state.expanded);
    }

    #[test]
    fn back_collapses_expansion_first() {
        let mut state = AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        state.handle_key(Action::Select);
        assert!(state.expanded);
        assert!(!state.wants_back);

        state.handle_key(Action::Back);
        assert!(!state.expanded);
        assert!(!state.wants_back); // collapsed, didn't navigate back

        state.handle_key(Action::Back);
        assert!(state.wants_back); // now navigates back
    }

    #[test]
    fn back_navigates_immediately_from_non_issues_panel() {
        let mut state = AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Project;
        state.handle_key(Action::Back);
        assert!(state.wants_back);
    }

    #[test]
    fn expand_ignored_on_empty_issues() {
        let data = AuditData {
            project_name: "empty".to_string(),
            total_files: 0,
            issues: vec![],
            historical_scores: vec![],
            next_steps: vec![],
        };
        let mut state = AuditState::new(data);
        state.focused_panel = AuditPanel::Issues;
        state.handle_key(Action::Select);
        assert!(!state.expanded);
    }

    #[test]
    fn help_text_only_advertises_fix_for_fixable_issue() {
        let mut state = AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        assert_eq!(
            <AuditState as crate::surface::Surface>::help_text(&state),
            "j/k navigate  h/l switch panel  enter expand  esc back  q quit"
        );

        state.selected_item = 1;
        assert_eq!(
            <AuditState as crate::surface::Surface>::help_text(&state),
            "j/k navigate  h/l switch panel  enter expand  f fix  esc back  q quit"
        );
    }

    #[test]
    fn f_key_sets_pending_fix_for_fixable_issue() {
        let mut state = AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        state.selected_item = 1;

        state.handle_key(Action::Character('f'));
        assert_eq!(
            state.pending_fix,
            Some(FixRequest::AuditConsoleStatement {
                file: "src/utils/db.ts".to_string(),
                line: 3,
            })
        );
    }
}
