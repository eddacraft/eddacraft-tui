pub mod event_adapter;
pub mod render;

use eddacraft_tui::keyboard::Action;

/// Status of a single gate check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateCheckStatus {
    Passed,
    Failed,
    Skipped,
    Warning,
}

impl GateCheckStatus {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Passed => "*",
            Self::Failed => "x",
            Self::Skipped => "o",
            Self::Warning => "!",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Passed => "PASS",
            Self::Failed => "FAIL",
            Self::Skipped => "SKIP",
            Self::Warning => "WARN",
        }
    }
}

/// A single gate check result.
#[derive(Debug, Clone)]
pub struct GateCheck {
    pub id: String,
    pub name: String,
    pub status: GateCheckStatus,
    pub score: f64,
    pub message: String,
    pub details: Option<String>,
    pub file: Option<String>,
    pub line: Option<usize>,
}

/// Full gate run result.
#[derive(Debug, Clone)]
pub struct GateResult {
    pub plan_id: String,
    pub overall_passed: bool,
    pub score: f64,
    pub checks: Vec<GateCheck>,
    pub duration_ms: u64,
    pub timestamp: String,
}

/// Filter for check statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterStatus {
    All,
    Passed,
    Failed,
    Warning,
    Skipped,
}

impl FilterStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Warning => "warning",
            Self::Skipped => "skipped",
        }
    }

    fn matches(self, status: GateCheckStatus) -> bool {
        match self {
            Self::All => true,
            Self::Passed => status == GateCheckStatus::Passed,
            Self::Failed => status == GateCheckStatus::Failed,
            Self::Warning => status == GateCheckStatus::Warning,
            Self::Skipped => status == GateCheckStatus::Skipped,
        }
    }
}

/// State for the gate explorer surface.
#[allow(clippy::struct_excessive_bools)]
pub struct GateState {
    pub result: GateResult,
    pub selected: usize,
    pub expanded: bool,
    pub filter: FilterStatus,
    pub search_term: String,
    pub search_mode: bool,
    pub should_quit: bool,
    pub wants_back: bool,
    /// `true` when hosted inside the welcome hub, where `q` quits the whole
    /// program and `esc` returns to the menu. The footer names that honest
    /// scope rather than advertising `esc`/`q` as equivalent (CIB-171).
    pub embedded: bool,
}

impl GateState {
    pub fn surface_name(&self) -> &'static str {
        "g a t e"
    }

    pub fn help_text(&self) -> &'static str {
        if self.search_mode {
            "type to search  esc cancel  enter confirm"
        } else if self.embedded {
            "j/k navigate  enter expand  / search  n/N failures  a/f/p/s/w filter  esc menu  q quit anvil"
        } else {
            "j/k navigate  enter expand  / search  n/N failures  a/f/p/s/w filter  esc back  q quit"
        }
    }

    /// Mark the surface as hosted inside the welcome hub so the footer names
    /// the honest scope of `q` (quit anvil) versus `esc` (menu) — CIB-171.
    #[must_use]
    pub fn embedded(mut self) -> Self {
        self.embedded = true;
        self
    }

    pub fn new(result: GateResult) -> Self {
        Self {
            result,
            selected: 0,
            expanded: false,
            filter: FilterStatus::All,
            search_term: String::new(),
            search_mode: false,
            should_quit: false,
            wants_back: false,
            embedded: false,
        }
    }

    /// Get checks filtered by status and search term.
    pub fn filtered_checks(&self) -> Vec<(usize, &GateCheck)> {
        self.result
            .checks
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                self.filter.matches(c.status)
                    && (self.search_term.is_empty()
                        || c.name
                            .to_lowercase()
                            .contains(&self.search_term.to_lowercase())
                        || c.id
                            .to_lowercase()
                            .contains(&self.search_term.to_lowercase()))
            })
            .collect()
    }

    /// Get the currently selected check from the filtered list.
    pub fn selected_check(&self) -> Option<&GateCheck> {
        let filtered = self.filtered_checks();
        filtered.get(self.selected).map(|(_, c)| *c)
    }

    /// Compute summary stats for the full (unfiltered) check list.
    pub fn summary(&self) -> GateSummary {
        GateSummary::from_checks(&self.result.checks)
    }

    /// Jump to the next check with `Failed` status in the filtered list.
    pub fn jump_next_failure(&mut self) {
        let filtered = self.filtered_checks();
        if filtered.is_empty() {
            return;
        }
        let start = self.selected + 1;
        for i in 0..filtered.len() {
            let idx = (start + i) % filtered.len();
            if filtered[idx].1.status == GateCheckStatus::Failed {
                self.selected = idx;
                self.expanded = false;
                return;
            }
        }
    }

    /// Jump to the previous check with `Failed` status in the filtered list.
    pub fn jump_prev_failure(&mut self) {
        let filtered = self.filtered_checks();
        if filtered.is_empty() {
            return;
        }
        let len = filtered.len();
        for i in 1..=len {
            let idx = (self.selected + len - i) % len;
            if filtered[idx].1.status == GateCheckStatus::Failed {
                self.selected = idx;
                self.expanded = false;
                return;
            }
        }
    }

    fn set_filter(&mut self, filter: FilterStatus) {
        self.filter = filter;
        self.selected = 0;
        self.expanded = false;
    }

    pub fn handle_key(&mut self, action: Action) {
        if self.search_mode {
            self.handle_search_key(action);
            return;
        }

        let filtered_count = self.filtered_checks().len();

        match action {
            Action::Up if self.selected > 0 => {
                self.selected -= 1;
                self.expanded = false;
            }
            Action::Down if self.selected < filtered_count.saturating_sub(1) => {
                self.selected += 1;
                self.expanded = false;
            }
            Action::Select => {
                self.expanded = !self.expanded;
            }
            Action::Character('n') => {
                self.jump_next_failure();
            }
            Action::Character('N') => {
                self.jump_prev_failure();
            }
            Action::Character('a') => {
                self.set_filter(FilterStatus::All);
            }
            Action::Character('p') => {
                self.set_filter(FilterStatus::Passed);
            }
            Action::Character('f') => {
                self.set_filter(FilterStatus::Failed);
            }
            Action::Character('s') => {
                self.set_filter(FilterStatus::Skipped);
            }
            Action::Character('w') => {
                self.set_filter(FilterStatus::Warning);
            }
            Action::Character('/') => {
                self.search_mode = true;
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

    fn handle_search_key(&mut self, action: Action) {
        match action {
            Action::Character(c) => {
                self.search_term.push(c);
                self.selected = 0;
            }
            Action::Backspace => {
                self.search_term.pop();
                self.selected = 0;
            }
            Action::Select => {
                self.search_mode = false;
            }
            Action::Back => {
                self.search_mode = false;
                self.search_term.clear();
                self.selected = 0;
            }
            _ => {}
        }
    }
}

/// Summary statistics for gate checks.
#[derive(Debug, Clone, Default)]
pub struct GateSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub skipped: usize,
}

impl GateSummary {
    pub fn from_checks(checks: &[GateCheck]) -> Self {
        let mut s = Self {
            total: checks.len(),
            ..Default::default()
        };
        for check in checks {
            match check.status {
                GateCheckStatus::Passed => s.passed += 1,
                GateCheckStatus::Failed => s.failed += 1,
                GateCheckStatus::Warning => s.warnings += 1,
                GateCheckStatus::Skipped => s.skipped += 1,
            }
        }
        s
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total as f64
    }
}

impl crate::surface::Surface for GateState {
    fn surface_name(&self) -> &'static str {
        "Gate"
    }

    fn help_text(&self) -> &'static str {
        if self.search_mode {
            "type to search  enter confirm  esc cancel"
        } else if self.embedded {
            "j/k navigate  enter expand  n/N next/prev fail  /search  a/p/f/w/s filter  esc menu  q quit anvil"
        } else {
            "j/k navigate  enter expand  n/N next/prev fail  /search  a/p/f/w/s filter  esc back  q quit"
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

    fn sample_checks() -> Vec<GateCheck> {
        vec![
            GateCheck {
                id: "eslint".to_string(),
                name: "ESLint".to_string(),
                status: GateCheckStatus::Passed,
                score: 1.0,
                message: "No issues found".to_string(),
                details: Some("Checked 42 files".to_string()),
                file: None,
                line: None,
            },
            GateCheck {
                id: "secret-scan".to_string(),
                name: "Secret scan".to_string(),
                status: GateCheckStatus::Failed,
                score: 0.0,
                message: "API key detected in config.ts".to_string(),
                details: Some("Line 15: AWS_SECRET_KEY=...".to_string()),
                file: Some("src/config.ts".to_string()),
                line: Some(15),
            },
            GateCheck {
                id: "architecture".to_string(),
                name: "Architecture".to_string(),
                status: GateCheckStatus::Warning,
                score: 0.7,
                message: "2 boundary violations".to_string(),
                details: None,
                file: None,
                line: None,
            },
            GateCheck {
                id: "dependency".to_string(),
                name: "Dependencies".to_string(),
                status: GateCheckStatus::Passed,
                score: 1.0,
                message: "All dependencies up to date".to_string(),
                details: None,
                file: None,
                line: None,
            },
            GateCheck {
                id: "coverage".to_string(),
                name: "Coverage".to_string(),
                status: GateCheckStatus::Skipped,
                score: 0.0,
                message: "No coverage config found".to_string(),
                details: None,
                file: None,
                line: None,
            },
            GateCheck {
                id: "type-check".to_string(),
                name: "Type check".to_string(),
                status: GateCheckStatus::Failed,
                score: 0.0,
                message: "3 type errors".to_string(),
                details: Some(
                    "src/index.ts(5): TS2322\nsrc/utils.ts(12): TS2345\nsrc/utils.ts(20): TS7006"
                        .to_string(),
                ),
                file: Some("src/index.ts".to_string()),
                line: Some(5),
            },
        ]
    }

    fn sample_result() -> GateResult {
        GateResult {
            plan_id: "default".to_string(),
            overall_passed: false,
            score: 0.45,
            checks: sample_checks(),
            duration_ms: 3200,
            timestamp: "2026-03-16T10:00:00Z".to_string(),
        }
    }

    // CIB-171: standalone keeps "esc back  q quit"; hosted in the hub the
    // footer names the honest scope — "esc menu  q quit anvil".
    #[test]
    fn embedded_footer_names_hub_scope() {
        use crate::surface::Surface;
        let standalone = GateState::new(sample_result());
        assert!(Surface::help_text(&standalone).ends_with("esc back  q quit"));

        let embedded = GateState::new(sample_result()).embedded();
        assert!(embedded.embedded);
        let footer = Surface::help_text(&embedded);
        assert!(footer.contains("esc menu"), "got: {footer}");
        assert!(footer.contains("q quit anvil"), "got: {footer}");
        assert!(!footer.contains("esc back"), "got: {footer}");

        // Search-mode footer is unaffected in either scope.
        let mut searching = GateState::new(sample_result()).embedded();
        searching.search_mode = true;
        assert_eq!(
            Surface::help_text(&searching),
            "type to search  enter confirm  esc cancel"
        );
    }

    #[test]
    fn filter_applies_correctly() {
        let mut state = GateState::new(sample_result());

        state.set_filter(FilterStatus::Failed);
        let filtered = state.filtered_checks();
        assert_eq!(filtered.len(), 2);
        assert!(
            filtered
                .iter()
                .all(|(_, c)| c.status == GateCheckStatus::Failed)
        );

        state.set_filter(FilterStatus::Passed);
        let filtered = state.filtered_checks();
        assert_eq!(filtered.len(), 2);

        state.set_filter(FilterStatus::Warning);
        let filtered = state.filtered_checks();
        assert_eq!(filtered.len(), 1);

        state.set_filter(FilterStatus::Skipped);
        let filtered = state.filtered_checks();
        assert_eq!(filtered.len(), 1);

        state.set_filter(FilterStatus::All);
        let filtered = state.filtered_checks();
        assert_eq!(filtered.len(), 6);
    }

    #[test]
    fn search_narrows_results() {
        let mut state = GateState::new(sample_result());
        state.search_term = "secret".to_string();

        let filtered = state.filtered_checks();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.id, "secret-scan");
    }

    #[test]
    fn search_by_id() {
        let mut state = GateState::new(sample_result());
        state.search_term = "eslint".to_string();

        let filtered = state.filtered_checks();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.name, "ESLint");
    }

    #[test]
    fn search_and_filter_combine() {
        let mut state = GateState::new(sample_result());
        state.filter = FilterStatus::Failed;
        state.search_term = "type".to_string();

        let filtered = state.filtered_checks();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.id, "type-check");
    }

    #[test]
    fn failure_jumping_next() {
        let mut state = GateState::new(sample_result());
        assert_eq!(state.selected, 0); // ESLint (passed)

        state.jump_next_failure();
        // Should jump to index 1 (secret-scan, failed)
        assert_eq!(state.selected, 1);

        state.jump_next_failure();
        // Should jump to index 5 (type-check, failed)
        assert_eq!(state.selected, 5);

        state.jump_next_failure();
        // Should wrap to index 1 (secret-scan, failed)
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn failure_jumping_prev() {
        let mut state = GateState::new(sample_result());
        state.selected = 5; // type-check (failed)

        state.jump_prev_failure();
        // Should jump to index 1 (secret-scan, failed)
        assert_eq!(state.selected, 1);

        state.jump_prev_failure();
        // Should wrap to index 5 (type-check, failed)
        assert_eq!(state.selected, 5);
    }

    #[test]
    fn failure_jumping_no_failures() {
        let result = GateResult {
            plan_id: "clean".to_string(),
            overall_passed: true,
            score: 1.0,
            checks: vec![GateCheck {
                id: "lint".to_string(),
                name: "Lint".to_string(),
                status: GateCheckStatus::Passed,
                score: 1.0,
                message: "Clean".to_string(),
                details: None,
                file: None,
                line: None,
            }],
            duration_ms: 100,
            timestamp: "2026-03-16T10:00:00Z".to_string(),
        };

        let mut state = GateState::new(result);
        state.jump_next_failure();
        assert_eq!(state.selected, 0); // unchanged
    }

    #[test]
    fn expand_collapse() {
        let mut state = GateState::new(sample_result());
        assert!(!state.expanded);

        state.handle_key(Action::Select);
        assert!(state.expanded);

        state.handle_key(Action::Select);
        assert!(!state.expanded);
    }

    #[test]
    fn navigation_collapses() {
        let mut state = GateState::new(sample_result());
        state.handle_key(Action::Select);
        assert!(state.expanded);

        state.handle_key(Action::Down);
        assert!(!state.expanded);
    }

    #[test]
    fn filter_via_keys() {
        let mut state = GateState::new(sample_result());
        state.handle_key(Action::Character('f'));
        assert_eq!(state.filter, FilterStatus::Failed);

        state.handle_key(Action::Character('a'));
        assert_eq!(state.filter, FilterStatus::All);

        state.handle_key(Action::Character('p'));
        assert_eq!(state.filter, FilterStatus::Passed);

        state.handle_key(Action::Character('w'));
        assert_eq!(state.filter, FilterStatus::Warning);

        state.handle_key(Action::Character('s'));
        assert_eq!(state.filter, FilterStatus::Skipped);
    }

    #[test]
    fn search_mode_toggle() {
        let mut state = GateState::new(sample_result());
        state.handle_key(Action::Character('/'));
        assert!(state.search_mode);

        state.handle_key(Action::Character('e'));
        assert_eq!(state.search_term, "e");

        state.handle_key(Action::Back);
        assert!(!state.search_mode);
        assert!(state.search_term.is_empty());
    }

    #[test]
    fn score_calculation() {
        let summary = GateSummary::from_checks(&sample_checks());
        assert_eq!(summary.total, 6);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.warnings, 1);
        assert_eq!(summary.skipped, 1);

        let rate = summary.pass_rate();
        assert!((rate - 2.0 / 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn score_empty_checks() {
        let summary = GateSummary::from_checks(&[]);
        assert!(summary.pass_rate().abs() < f64::EPSILON);
    }

    #[test]
    fn navigation_bounds() {
        let mut state = GateState::new(sample_result());
        state.handle_key(Action::Up); // at 0
        assert_eq!(state.selected, 0);

        for _ in 0..20 {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.selected, 5); // max index (6 checks)
    }

    #[test]
    fn filter_resets_selection() {
        let mut state = GateState::new(sample_result());
        state.selected = 3;
        state.handle_key(Action::Character('f')); // filter to failed
        assert_eq!(state.selected, 0);
    }
}
