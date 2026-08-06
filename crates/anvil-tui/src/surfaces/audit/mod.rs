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

/// Deterministic audit fix handled by the shared CLI fix dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuditFixKind {
    ConsoleStatement,
}

/// A single audit issue.
#[derive(Debug, Clone)]
pub struct AuditIssue {
    pub severity: IssueSeverity,
    pub category: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    /// Non-user-facing discriminator for deterministic audit fixes.
    /// Today the audit surface only exposes console-statement removal.
    pub fixable: bool,
}

impl AuditIssue {
    /// Render `file:line`, omitting the line for a whole-file finding.
    ///
    /// `line: 0` means the finding is about the file itself (a committed
    /// `.env`, say) rather than a numbered line — the same sentinel
    /// [`Self::fix_kind`] already guards on. Printing it as `.env:0` implied a
    /// zero-based line number that no anvil surface uses (CIB-237).
    ///
    /// This mirrors `display_path::format_location` in the CLI crate; the two
    /// cannot share code because `anvil-cli` is a binary crate.
    #[must_use]
    pub fn location(&self) -> String {
        if self.line == 0 {
            self.file.clone()
        } else {
            format!("{}:{}", self.file, self.line)
        }
    }

    fn fix_kind(&self) -> Option<AuditFixKind> {
        (self.fixable && self.line != 0).then_some(AuditFixKind::ConsoleStatement)
    }

    #[must_use]
    pub fn is_fixable(&self) -> bool {
        self.fix_kind().is_some()
    }

    #[must_use]
    pub fn fix_request(&self) -> Option<FixRequest> {
        match self.fix_kind()? {
            AuditFixKind::ConsoleStatement => Some(FixRequest::AuditConsoleStatement {
                file: self.file.clone(),
                line: self.line,
            }),
        }
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
    pub security_scope: String,
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
#[allow(clippy::struct_excessive_bools)]
pub struct AuditState {
    pub data: AuditData,
    pub focused_panel: AuditPanel,
    pub selected_item: usize,
    pub expanded: bool,
    /// When `true`, only the focused panel renders, taking the full
    /// area. Same affordance as the watch surface — press `z` to
    /// toggle, `esc` while zoomed exits zoom before navigating back.
    pub zoomed: bool,
    pub should_quit: bool,
    pub wants_back: bool,
    pub pending_fix: Option<FixRequest>,
    /// `true` when hosted inside the welcome hub, where `q` quits the whole
    /// program and `esc` returns to the menu. The footer names that honest
    /// scope rather than advertising `esc`/`q` as equivalent (CIB-171).
    pub embedded: bool,
}

impl AuditState {
    pub fn new(data: AuditData) -> Self {
        Self {
            data,
            focused_panel: AuditPanel::Project,
            selected_item: 0,
            expanded: false,
            zoomed: false,
            should_quit: false,
            wants_back: false,
            pending_fix: None,
            embedded: false,
        }
    }

    /// Mark the surface as hosted inside the welcome hub so the footer names
    /// the honest scope of `q` (quit anvil) versus `esc` (menu) — CIB-171.
    #[must_use]
    pub fn embedded(mut self) -> Self {
        self.embedded = true;
        self
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

    fn selected_issue_is_fixable(&self) -> bool {
        if self.focused_panel != AuditPanel::Issues {
            return false;
        }
        self.data
            .issues
            .get(self.selected_item)
            .is_some_and(AuditIssue::is_fixable)
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
            Action::Up if self.selected_item > 0 => {
                self.selected_item -= 1;
                self.expanded = false;
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
            Action::Select
                if self.focused_panel == AuditPanel::Issues && !self.data.issues.is_empty() =>
            {
                self.expanded = !self.expanded;
            }
            Action::Character('f') => {
                if let Some(request) = self.selected_issue_fix_request() {
                    self.pending_fix = Some(request);
                }
            }
            Action::Character('z') => {
                self.zoomed = !self.zoomed;
                // Exiting zoom while expanded keeps the expansion; entering
                // zoom collapses so the focused panel uses every row.
                if self.zoomed {
                    self.expanded = false;
                }
            }
            Action::Back => {
                if self.zoomed {
                    self.zoomed = false;
                } else if self.expanded {
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
        if self.zoomed { "Audit [zoom]" } else { "Audit" }
    }

    fn help_text(&self) -> &'static str {
        // In the hub `q` quits the whole program (not just this surface), so
        // embedded footers say "q quit anvil"; at the base level `esc` returns
        // to the menu ("esc menu") rather than the surface's own "esc back".
        // Zoom/collapse keep their within-surface `esc` semantics (CIB-171).
        let fixable = self.selected_issue_is_fixable();
        if self.zoomed {
            match (self.embedded, fixable) {
                (false, true) => "j/k navigate  z unzoom  f fix  esc unzoom  q quit",
                (false, false) => "j/k navigate  z unzoom  esc unzoom  q quit",
                (true, true) => "j/k navigate  z unzoom  f fix  esc unzoom  q quit anvil",
                (true, false) => "j/k navigate  z unzoom  esc unzoom  q quit anvil",
            }
        } else if self.expanded {
            match (self.embedded, fixable) {
                (false, true) => "j/k navigate  h/l switch panel  f fix  esc collapse  q quit",
                (false, false) => "j/k navigate  h/l switch panel  esc collapse  q quit",
                (true, true) => "j/k navigate  h/l switch panel  f fix  esc collapse  q quit anvil",
                (true, false) => "j/k navigate  h/l switch panel  esc collapse  q quit anvil",
            }
        } else {
            match (self.embedded, fixable) {
                (false, true) => {
                    "j/k navigate  h/l switch panel  enter expand  z zoom  f fix  esc back  q quit"
                }
                (false, false) => {
                    "j/k navigate  h/l switch panel  enter expand  z zoom  esc back  q quit"
                }
                (true, true) => {
                    "j/k navigate  h/l switch panel  enter expand  z zoom  f fix  esc menu  q quit anvil"
                }
                (true, false) => {
                    "j/k navigate  h/l switch panel  enter expand  z zoom  esc menu  q quit anvil"
                }
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
            security_scope: "same secret rules; gate adds checks".to_string(),
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
            security_scope: "test scope".to_string(),
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
            "j/k navigate  h/l switch panel  enter expand  z zoom  esc back  q quit"
        );

        state.selected_item = 1;
        assert_eq!(
            <AuditState as crate::surface::Surface>::help_text(&state),
            "j/k navigate  h/l switch panel  enter expand  z zoom  f fix  esc back  q quit"
        );
    }

    // CIB-171: hosted in the hub the base-level footer names the honest scope
    // — "esc menu  q quit anvil"; zoom/collapse keep their within-surface esc
    // but still say "q quit anvil".
    #[test]
    fn embedded_footer_names_hub_scope() {
        use crate::surface::Surface;
        let mut state = AuditState::new(sample_data()).embedded();
        state.focused_panel = AuditPanel::Issues;
        assert!(state.embedded);
        assert_eq!(
            Surface::help_text(&state),
            "j/k navigate  h/l switch panel  enter expand  z zoom  esc menu  q quit anvil"
        );

        // Zoom keeps "esc unzoom" (within-surface) but q still quits anvil.
        state.zoomed = true;
        let footer = Surface::help_text(&state);
        assert!(footer.contains("esc unzoom"), "got: {footer}");
        assert!(footer.contains("q quit anvil"), "got: {footer}");
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

    #[test]
    fn fix_request_uses_fixable_discriminator_not_message_copy() {
        let issue = AuditIssue {
            severity: IssueSeverity::Low,
            category: "Quality".to_string(),
            message: "copy can change without breaking fix dispatch".to_string(),
            file: "src/index.ts".to_string(),
            line: 7,
            fixable: true,
        };

        assert_eq!(
            issue.fix_request(),
            Some(FixRequest::AuditConsoleStatement {
                file: "src/index.ts".to_string(),
                line: 7,
            })
        );
    }

    /// CIB-237: `line: 0` marks a whole-file finding, not line zero.
    #[test]
    fn location_omits_the_whole_file_sentinel() {
        let issue = AuditIssue {
            severity: IssueSeverity::High,
            category: "Security".to_string(),
            message: "Environment file may contain secrets".to_string(),
            file: ".env".to_string(),
            line: 0,
            fixable: false,
        };

        assert_eq!(issue.location(), ".env");
        assert!(!issue.is_fixable());
    }

    #[test]
    fn location_renders_real_line_numbers() {
        let issue = AuditIssue {
            severity: IssueSeverity::Low,
            category: "Quality".to_string(),
            message: "console statement found".to_string(),
            file: "src/index.ts".to_string(),
            line: 7,
            fixable: true,
        };

        assert_eq!(issue.location(), "src/index.ts:7");
    }
}
