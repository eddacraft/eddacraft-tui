pub mod render;

use eddacraft_tui::keyboard::Action;

/// Status of a single diagnostic check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
    Skipped,
    Running,
}

impl CheckStatus {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Pass => "<<diamond>>",
            Self::Fail => "x",
            Self::Warn => "!",
            Self::Skipped => "o",
            Self::Running => "*",
        }
    }
}

/// A single diagnostic check result.
#[derive(Debug, Clone)]
pub struct DiagnosticCheck {
    pub name: String,
    pub category: String,
    pub status: CheckStatus,
    pub message: String,
    pub details: Option<String>,
    pub auto_fixable: bool,
}

/// Aggregate summary of all checks.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub skipped: usize,
}

impl DiagnosticSummary {
    pub fn from_checks(checks: &[DiagnosticCheck]) -> Self {
        let mut summary = Self {
            total: checks.len(),
            ..Default::default()
        };
        for check in checks {
            match check.status {
                CheckStatus::Pass => summary.passed += 1,
                CheckStatus::Fail => summary.failed += 1,
                CheckStatus::Warn => summary.warnings += 1,
                CheckStatus::Skipped => summary.skipped += 1,
                CheckStatus::Running => {}
            }
        }
        summary
    }
}

/// State for the doctor surface.
pub struct DoctorState {
    pub checks: Vec<DiagnosticCheck>,
    pub selected: usize,
    pub expanded: bool,
    pub should_quit: bool,
}

impl DoctorState {
    pub fn new(checks: Vec<DiagnosticCheck>) -> Self {
        Self {
            checks,
            selected: 0,
            expanded: false,
            should_quit: false,
        }
    }

    pub fn surface_name(&self) -> &'static str {
        "d o c t o r"
    }

    pub fn help_text(&self) -> &'static str {
        "j/k navigate  enter expand  q quit"
    }

    pub fn summary(&self) -> DiagnosticSummary {
        DiagnosticSummary::from_checks(&self.checks)
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.expanded = false;
                }
            }
            Action::Down => {
                if self.selected < self.checks.len().saturating_sub(1) {
                    self.selected += 1;
                    self.expanded = false;
                }
            }
            Action::Select => {
                self.expanded = !self.expanded;
            }
            Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_checks() -> Vec<DiagnosticCheck> {
        vec![
            DiagnosticCheck {
                name: "Node.js".to_string(),
                category: "Runtime".to_string(),
                status: CheckStatus::Pass,
                message: "v22.0.0 found".to_string(),
                details: Some("Path: /usr/bin/node".to_string()),
                auto_fixable: false,
            },
            DiagnosticCheck {
                name: "ESLint config".to_string(),
                category: "Linting".to_string(),
                status: CheckStatus::Fail,
                message: "No .eslintrc found".to_string(),
                details: Some("Run `npx eslint --init` to create one".to_string()),
                auto_fixable: true,
            },
            DiagnosticCheck {
                name: "Git hooks".to_string(),
                category: "Hooks".to_string(),
                status: CheckStatus::Warn,
                message: "Hooks not installed".to_string(),
                details: None,
                auto_fixable: true,
            },
        ]
    }

    #[test]
    fn summary_counts_correctly() {
        let checks = sample_checks();
        let summary = DiagnosticSummary::from_checks(&checks);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.warnings, 1);
    }

    #[test]
    fn navigate_and_expand() {
        let mut state = DoctorState::new(sample_checks());
        assert_eq!(state.selected, 0);
        assert!(!state.expanded);

        state.handle_key(Action::Select);
        assert!(state.expanded);

        state.handle_key(Action::Down);
        assert_eq!(state.selected, 1);
        assert!(!state.expanded); // collapsed on navigation
    }

    #[test]
    fn bounds_checking() {
        let mut state = DoctorState::new(sample_checks());
        state.handle_key(Action::Up); // already at 0
        assert_eq!(state.selected, 0);

        for _ in 0..10 {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.selected, 2); // max index
    }
}
