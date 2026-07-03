pub mod render;

use eddacraft_tui::keyboard::Action;

use super::fix_request::FixRequest;

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

/// Concrete next action for a check. Pass / Skipped checks may carry
/// the default `Remediation` (empty summary, no command, no doc URL).
/// For Fail / Warn checks, `summary` must be non-empty — see
/// `eddacraft_anvil::commands::doctor::tests::every_check_fail_or_warn_branch_carries_remediation`
/// in the CLI crate for the invariant test that exercises every
/// `check_*` function.
///
/// The optional fields `command` and `doc_url` give the renderer a
/// structured place to surface a runnable command vs a documentation
/// link without re-parsing prose. They may both be set on the same
/// remediation (e.g. a setup command alongside a "read more" doc).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remediation {
    /// Human-readable summary of what the user should do.
    pub summary: String,
    /// A specific shell command the user can run, if any.
    pub command: Option<String>,
    /// An optional documentation URL for further reading; may be set
    /// alongside `command`.
    pub doc_url: Option<String>,
}

impl Remediation {
    /// True when the remediation has no actionable content.
    pub fn is_empty(&self) -> bool {
        self.summary.is_empty() && self.command.is_none() && self.doc_url.is_none()
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
    pub remediation: Remediation,
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
#[allow(clippy::struct_excessive_bools)]
pub struct DoctorState {
    pub checks: Vec<DiagnosticCheck>,
    pub selected: usize,
    pub expanded: bool,
    pub should_quit: bool,
    pub wants_back: bool,
    /// Pending fix request emitted when the user presses `f`.
    pub pending_fix: Option<FixRequest>,
    /// Outcome banner rendered after the host applies a pending fix and
    /// re-enters the TUI. The host is responsible for clearing this on the
    /// next user action so the banner is transient. `None` means no outcome
    /// to surface.
    pub last_fix_outcome: Option<FixOutcomeBanner>,
    /// `true` when hosted inside the welcome hub. The hub keeps `q` as
    /// "quit the whole program" and `esc` as "return to the menu", so the
    /// footer must say so honestly rather than advertising `esc`/`q` as
    /// equivalent (CIB-171). Standalone (`anvil doctor`) keeps `esc back`.
    pub embedded: bool,
}

/// Display-friendly summary of a completed fix attempt — populated by the
/// command host (`commands::doctor::run`) after `apply_fix_request` returns.
/// Kept separate from the host-side `FixOutcome` enum so the TUI crate can
/// render it without depending on the CLI crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixOutcomeBanner {
    Applied { summary: String },
    Refused { reason: String },
    Failed { reason: String },
}

impl DoctorState {
    pub fn new(checks: Vec<DiagnosticCheck>) -> Self {
        Self {
            checks,
            selected: 0,
            expanded: false,
            should_quit: false,
            wants_back: false,
            pending_fix: None,
            last_fix_outcome: None,
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

    pub fn summary(&self) -> DiagnosticSummary {
        DiagnosticSummary::from_checks(&self.checks)
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up if self.selected > 0 => {
                self.selected -= 1;
                self.expanded = false;
                self.last_fix_outcome = None;
            }
            Action::Down if self.selected < self.checks.len().saturating_sub(1) => {
                self.selected += 1;
                self.expanded = false;
                self.last_fix_outcome = None;
            }
            Action::Select => {
                self.expanded = !self.expanded;
                self.last_fix_outcome = None;
            }
            Action::Character('f') => {
                if let Some(check) = self.checks.get(self.selected)
                    && check.auto_fixable
                    && check.status != CheckStatus::Pass
                {
                    self.pending_fix = Some(FixRequest::DoctorCheck {
                        index: self.selected,
                    });
                }
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
}

impl crate::surface::Surface for DoctorState {
    fn surface_name(&self) -> &'static str {
        "Doctor"
    }

    fn help_text(&self) -> &'static str {
        let fixable = self
            .checks
            .get(self.selected)
            .is_some_and(|c| c.auto_fixable && c.status != CheckStatus::Pass);
        match (self.embedded, fixable) {
            (false, true) => "j/k navigate  enter expand  f fix  esc back  q quit",
            (false, false) => "j/k navigate  enter expand  esc back  q quit",
            (true, true) => "j/k navigate  enter expand  f fix  esc menu  q quit anvil",
            (true, false) => "j/k navigate  enter expand  esc menu  q quit anvil",
        }
    }

    fn handle_key(&mut self, action: Action) {
        DoctorState::handle_key(self, action);
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

    fn sample_checks() -> Vec<DiagnosticCheck> {
        vec![
            DiagnosticCheck {
                name: "Node.js".to_string(),
                category: "Runtime".to_string(),
                status: CheckStatus::Pass,
                message: "v22.0.0 found".to_string(),
                details: Some("Path: /usr/bin/node".to_string()),
                auto_fixable: false,
                remediation: Remediation::default(),
            },
            DiagnosticCheck {
                name: "ESLint config".to_string(),
                category: "Linting".to_string(),
                status: CheckStatus::Fail,
                message: "No .eslintrc found".to_string(),
                details: Some("Run `npx eslint --init` to create one".to_string()),
                auto_fixable: true,
                remediation: Remediation {
                    summary: "Create an ESLint config".to_string(),
                    command: Some("npx eslint --init".to_string()),
                    doc_url: None,
                },
            },
            DiagnosticCheck {
                name: "Git hooks".to_string(),
                category: "Hooks".to_string(),
                status: CheckStatus::Warn,
                message: "Hooks not installed".to_string(),
                details: None,
                auto_fixable: true,
                remediation: Remediation {
                    summary: "Install pre-commit hooks".to_string(),
                    command: Some("npx husky init".to_string()),
                    doc_url: None,
                },
            },
        ]
    }

    // CIB-171: standalone keeps "esc back  q quit"; hosted in the hub the
    // footer names the honest scope — "esc menu  q quit anvil".
    #[test]
    fn embedded_footer_names_hub_scope() {
        use crate::surface::Surface;
        let standalone = DoctorState::new(sample_checks());
        assert!(Surface::help_text(&standalone).ends_with("esc back  q quit"));

        let embedded = DoctorState::new(sample_checks()).embedded();
        assert!(embedded.embedded);
        let footer = Surface::help_text(&embedded);
        assert!(footer.contains("esc menu"), "got: {footer}");
        assert!(footer.contains("q quit anvil"), "got: {footer}");
        assert!(!footer.contains("esc back"), "got: {footer}");
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

    #[test]
    fn f_key_sets_pending_fix_on_fixable_check() {
        let mut state = DoctorState::new(sample_checks());
        // Navigate to ESLint config (index 1) — Fail + auto_fixable
        state.handle_key(Action::Down);
        state.handle_key(Action::Character('f'));
        assert_eq!(
            state.pending_fix,
            Some(FixRequest::DoctorCheck { index: 1 })
        );
    }

    #[test]
    fn f_key_no_op_on_passing_check() {
        let mut state = DoctorState::new(sample_checks());
        // Index 0 is Node.js — Pass, not fixable
        state.handle_key(Action::Character('f'));
        assert!(state.pending_fix.is_none());
    }

    #[test]
    fn f_key_no_op_on_non_fixable_check() {
        let checks = vec![DiagnosticCheck {
            name: "test".to_string(),
            category: "Test".to_string(),
            status: CheckStatus::Fail,
            message: "failed".to_string(),
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        }];
        let mut state = DoctorState::new(checks);
        state.handle_key(Action::Character('f'));
        assert!(state.pending_fix.is_none());
    }

    #[test]
    fn pending_fix_causes_should_quit_true() {
        let mut state = DoctorState::new(sample_checks());
        state.handle_key(Action::Down);
        assert!(!crate::surface::Surface::should_quit(&state));
        state.handle_key(Action::Character('f'));
        assert!(crate::surface::Surface::should_quit(&state));
    }

    #[test]
    fn reset_clears_fix_state() {
        let mut state = DoctorState::new(sample_checks());
        state.pending_fix = Some(FixRequest::DoctorCheck { index: 1 });
        <DoctorState as crate::surface::Surface>::reset(&mut state);
        assert!(state.pending_fix.is_none());
    }
}
