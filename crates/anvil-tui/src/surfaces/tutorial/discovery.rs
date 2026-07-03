use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::surface::Surface;
use crate::surfaces::fix_request::FixRequest;

use super::TutorialPath;
use super::discovery_render;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Severity level for a unified finding.
///
/// Ordered from most to least severe so that `PartialOrd` / `Ord` comparisons
/// work correctly: `Error > Warning > Info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingSeverity {
    Info, // lowest — must be first for derive Ord
    Warning,
    Error, // highest — must be last for derive Ord
}

/// Which scanner produced a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSource {
    Architecture,
    AntiPattern,
    Secret,
}

impl FindingSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warning => "WARN",
            Self::Info => "INFO",
        }
    }
}

impl FindingSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Architecture => "Architecture",
            Self::AntiPattern => "Anti-pattern",
            Self::Secret => "Secret",
        }
    }
}

/// A unified finding from any scan source.
#[derive(Debug, Clone)]
pub struct Finding {
    pub file: String,
    pub line: Option<usize>,
    pub severity: FindingSeverity,
    pub source: FindingSource,
    pub title: String,
    pub message: String,
    pub suggestion: String,
    pub warning_id: Option<String>,
}

impl Finding {
    #[must_use]
    pub fn fix_request(&self) -> Option<FixRequest> {
        let line = self.line?;
        if line == 0 {
            return None;
        }
        let warning_id = self.warning_id.as_deref()?;
        match warning_id {
            "AP-001" | "AP-003" | "AP-004" => Some(FixRequest::AntiPatternWarning {
                file: self.file.clone(),
                line,
                warning_id: warning_id.to_string(),
            }),
            _ => None,
        }
    }
}

/// Aggregated scan results from all sources, filtered and sorted.
#[derive(Debug, Clone, Default)]
pub struct ScanResults {
    pub findings: Vec<Finding>,
    pub files_scanned: usize,
    pub duration_ms: u64,
    /// True when the scan hit the file cap before exhausting the project.
    pub truncated: bool,
    /// SCAN-004: number of files that matched the scan criteria but were
    /// skipped because `.gitignore` excluded them (discovery honours gitignore
    /// unless `ANVIL_SCAN_ALL` is set). Surfaced so a "0 findings" result can
    /// be told apart from "we never looked at the directory that held the
    /// secret". Zero when the scan was truncated by the file cap or when
    /// `ANVIL_SCAN_ALL` is set, because neither case can be honestly
    /// attributed to gitignore.
    pub files_skipped_by_ignore: usize,
    /// CIB-170: true when these findings are curated showcase examples
    /// substituted for a clean-repo (or scan-failure) result, not real
    /// findings from the user's code. Drives a distinct "Example findings"
    /// banner and per-row badge so a user never mistakes the demo secret at
    /// `src/services/auth.rs:42` for a real leak in their own repo.
    pub is_showcase: bool,
}

impl ScanResults {
    /// Return all findings sorted by severity descending (Error first).
    pub fn sorted_findings(&self) -> Vec<&Finding> {
        let mut sorted: Vec<&Finding> = self.findings.iter().collect();
        // Sort descending: Error > Warning > Info.
        sorted.sort_by_key(|finding| std::cmp::Reverse(finding.severity));
        sorted
    }

    /// Filter findings relevant to a tutorial domain.
    ///
    /// - `ProtectionLoop` -> all findings (LAUNCH-014: the value-first
    ///   walk previews the loop on a deliberate fixture and does not
    ///   carve up the user's repo findings; downstream PRs may
    ///   narrow this to a high-signal subset, but blanket inclusion
    ///   is the honest v1 default).
    /// - Policy -> `AntiPattern` + Secret findings (policy rules catch these)
    /// - Architecture -> Architecture findings
    /// - Drift / CI / `DeveloperAcceleration` -> all findings (cross-cutting)
    #[must_use]
    pub fn filter_by_domain(&self, path: TutorialPath) -> ScanResults {
        let filtered_findings: Vec<Finding> = match path {
            TutorialPath::Policy => self
                .findings
                .iter()
                .filter(|f| matches!(f.source, FindingSource::AntiPattern | FindingSource::Secret))
                .cloned()
                .collect(),
            TutorialPath::Architecture => self
                .findings
                .iter()
                .filter(|f| matches!(f.source, FindingSource::Architecture))
                .cloned()
                .collect(),
            TutorialPath::ProtectionLoop
            | TutorialPath::DeveloperAcceleration
            | TutorialPath::Drift
            | TutorialPath::CI => self.findings.clone(),
        };
        ScanResults {
            findings: filtered_findings,
            files_scanned: self.files_scanned,
            duration_ms: self.duration_ms,
            truncated: self.truncated,
            files_skipped_by_ignore: self.files_skipped_by_ignore,
            is_showcase: self.is_showcase,
        }
    }
}

// ---------------------------------------------------------------------------
// Surface state
// ---------------------------------------------------------------------------

/// Current phase of the discovery surface.
#[derive(Debug, PartialEq)]
pub enum DiscoveryPhase {
    Scanning {
        files_scanned: usize,
        spinner_tick: usize,
    },
    Results {
        selected: usize,
    },
    Continue,
}

/// TUI surface that displays scan progress, then findings, then a continue
/// prompt.
///
/// This is a pure state machine — scanning is driven externally. The caller
/// must call [`DiscoveryState::update_progress`], [`DiscoveryState::set_results`],
/// and [`DiscoveryState::tick`] as appropriate.
pub struct DiscoveryState {
    pub phase: DiscoveryPhase,
    pub results: Option<ScanResults>,
    pub should_quit: bool,
    pub wants_back: bool,
    /// Set when the user presses Enter on the Continue screen to advance.
    /// Distinguishes "advance to next step" from "quit/abandon".
    pub wants_continue: bool,
}

impl DiscoveryState {
    pub fn new() -> Self {
        Self {
            phase: DiscoveryPhase::Scanning {
                files_scanned: 0,
                spinner_tick: 0,
            },
            results: None,
            should_quit: false,
            wants_back: false,
            wants_continue: false,
        }
    }

    /// Update the file count shown during the scanning phase.
    ///
    /// No-ops if not currently in the `Scanning` phase.
    pub fn update_progress(&mut self, files_scanned: usize) {
        if let DiscoveryPhase::Scanning {
            files_scanned: ref mut count,
            ..
        } = self.phase
        {
            *count = files_scanned;
        }
    }

    /// Transition from `Scanning` → `Results` (or `Continue` if no findings).
    ///
    /// Stores the results. If scanning has already finished this is a no-op
    /// (results are never overwritten).
    pub fn set_results(&mut self, results: ScanResults) {
        if matches!(self.phase, DiscoveryPhase::Scanning { .. }) {
            let has_findings = !results.findings.is_empty();
            self.results = Some(results);
            if has_findings {
                self.phase = DiscoveryPhase::Results { selected: 0 };
            } else {
                self.phase = DiscoveryPhase::Continue;
            }
        }
    }

    /// Advance the spinner animation by one tick.
    ///
    /// Should be called once per render cycle (e.g. on a timer tick event).
    /// No-ops outside the `Scanning` phase.
    pub fn tick(&mut self) {
        if let DiscoveryPhase::Scanning {
            ref mut spinner_tick,
            ..
        } = self.phase
        {
            *spinner_tick = spinner_tick.wrapping_add(1);
        }
    }

    /// Skip the scan immediately, transitioning to `Continue` with empty results.
    pub fn skip_scan(&mut self) {
        if matches!(self.phase, DiscoveryPhase::Scanning { .. }) {
            self.results.get_or_insert_with(ScanResults::default);
            self.phase = DiscoveryPhase::Continue;
        }
    }

    // ---- key handling helpers ------------------------------------------

    fn handle_scanning(&mut self, action: Action) {
        match action {
            Action::Character('s') => self.skip_scan(),
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_results(&mut self, action: Action) {
        match action {
            Action::Up => {
                if let DiscoveryPhase::Results { ref mut selected } = self.phase
                    && *selected > 0
                {
                    *selected -= 1;
                }
            }
            Action::Down => {
                if let DiscoveryPhase::Results { ref mut selected } = self.phase {
                    let max = self
                        .results
                        .as_ref()
                        .map_or(0, |r| r.findings.len().saturating_sub(1));
                    if *selected < max {
                        *selected += 1;
                    }
                }
            }
            Action::Select => {
                self.phase = DiscoveryPhase::Continue;
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

    fn handle_continue(&mut self, action: Action) {
        match action {
            Action::Select => {
                self.wants_continue = true;
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

impl Default for DiscoveryState {
    fn default() -> Self {
        Self::new()
    }
}

impl Surface for DiscoveryState {
    fn surface_name(&self) -> &'static str {
        "Discovery"
    }

    fn help_text(&self) -> &'static str {
        match self.phase {
            DiscoveryPhase::Scanning { .. } => "s skip  q quit",
            DiscoveryPhase::Results { .. } => "j/k navigate  enter continue  esc back  q quit",
            DiscoveryPhase::Continue => "enter continue  esc back  q quit",
        }
    }

    fn handle_key(&mut self, action: Action) {
        match self.phase {
            DiscoveryPhase::Scanning { .. } => self.handle_scanning(action),
            DiscoveryPhase::Results { .. } => self.handle_results(action),
            DiscoveryPhase::Continue => self.handle_continue(action),
        }
    }

    fn should_quit(&self) -> bool {
        self.should_quit || self.wants_continue
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        self.phase = DiscoveryPhase::Scanning {
            files_scanned: 0,
            spinner_tick: 0,
        };
        self.results = None;
        self.should_quit = false;
        self.wants_back = false;
        self.wants_continue = false;
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        discovery_render::render(frame, area, self, theme);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_finding(severity: FindingSeverity, title: &str) -> Finding {
        Finding {
            file: "src/main.rs".to_string(),
            line: Some(10),
            severity,
            source: FindingSource::AntiPattern,
            title: title.to_string(),
            message: "test message".to_string(),
            suggestion: "fix it".to_string(),
            warning_id: None,
        }
    }

    fn make_finding_with_source(
        severity: FindingSeverity,
        source: FindingSource,
        title: &str,
    ) -> Finding {
        Finding {
            file: "src/main.rs".to_string(),
            line: Some(10),
            severity,
            source,
            title: title.to_string(),
            message: "test message".to_string(),
            suggestion: "fix it".to_string(),
            warning_id: None,
        }
    }

    fn make_results(findings: Vec<Finding>) -> ScanResults {
        let files_scanned = 42;
        ScanResults {
            findings,
            files_scanned,
            duration_ms: 500,
            truncated: false,
            files_skipped_by_ignore: 0,
            is_showcase: false,
        }
    }

    #[test]
    fn fix_request_rejects_zero_line_findings() {
        let finding = Finding {
            file: "src/app.ts".to_string(),
            line: Some(0),
            severity: FindingSeverity::Warning,
            source: FindingSource::AntiPattern,
            title: "Avoid escape hatch".to_string(),
            message: "eslint disable".to_string(),
            suggestion: "Use a scoped disable".to_string(),
            warning_id: Some("AP-001".to_string()),
        };

        assert_eq!(finding.fix_request(), None);
    }

    // ── FindingSeverity ordering ─────────────────────────────────────────

    #[test]
    fn severity_ordering_error_gt_warning_gt_info() {
        assert!(FindingSeverity::Error > FindingSeverity::Warning);
        assert!(FindingSeverity::Warning > FindingSeverity::Info);
        assert!(FindingSeverity::Error > FindingSeverity::Info);
    }

    // ── ScanResults::sorted_findings ──────────────────────────────────────

    #[test]
    fn sorted_findings_returns_sorted_by_severity_descending() {
        let results = make_results(vec![
            make_finding(FindingSeverity::Info, "info finding"),
            make_finding(FindingSeverity::Error, "error finding"),
            make_finding(FindingSeverity::Warning, "warning finding"),
        ]);
        let sorted = results.sorted_findings();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].severity, FindingSeverity::Error);
        assert_eq!(sorted[1].severity, FindingSeverity::Warning);
        assert_eq!(sorted[2].severity, FindingSeverity::Info);
    }

    #[test]
    fn sorted_findings_empty_results() {
        let results = make_results(vec![]);
        assert!(results.sorted_findings().is_empty());
    }

    #[test]
    fn sorted_findings_single_item() {
        let results = make_results(vec![make_finding(FindingSeverity::Warning, "w")]);
        assert_eq!(results.sorted_findings().len(), 1);
    }

    // ── Initial state ────────────────────────────────────────────────────

    #[test]
    fn new_starts_in_scanning_phase() {
        let state = DiscoveryState::new();
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Scanning {
                files_scanned: 0,
                spinner_tick: 0
            }
        ));
        assert!(state.results.is_none());
        assert!(!state.should_quit);
        assert!(!state.wants_back);
    }

    // ── update_progress ──────────────────────────────────────────────────

    #[test]
    fn update_progress_sets_file_count() {
        let mut state = DiscoveryState::new();
        state.update_progress(42);
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Scanning {
                files_scanned: 42,
                ..
            }
        ));
    }

    #[test]
    fn update_progress_noop_in_results_phase() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Error,
            "e",
        )]));
        assert!(matches!(state.phase, DiscoveryPhase::Results { .. }));
        state.update_progress(99);
        assert!(matches!(state.phase, DiscoveryPhase::Results { .. }));
    }

    // ── tick ─────────────────────────────────────────────────────────────

    #[test]
    fn tick_advances_spinner() {
        let mut state = DiscoveryState::new();
        state.tick();
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Scanning {
                spinner_tick: 1,
                ..
            }
        ));
        state.tick();
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Scanning {
                spinner_tick: 2,
                ..
            }
        ));
    }

    #[test]
    fn tick_noop_in_results_phase() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Warning,
            "w",
        )]));
        state.tick(); // should not panic
        assert!(matches!(state.phase, DiscoveryPhase::Results { .. }));
    }

    // ── set_results ──────────────────────────────────────────────────────

    #[test]
    fn set_results_with_findings_transitions_to_results() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Error,
            "e",
        )]));
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Results { selected: 0 }
        ));
        assert!(state.results.is_some());
    }

    #[test]
    fn set_results_with_no_findings_transitions_to_continue() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![]));
        assert!(matches!(state.phase, DiscoveryPhase::Continue));
    }

    #[test]
    fn set_results_noop_if_already_in_results() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Error,
            "e",
        )]));
        // Call again — should not overwrite or change phase
        state.set_results(make_results(vec![]));
        assert!(matches!(state.phase, DiscoveryPhase::Results { .. }));
        assert_eq!(state.results.as_ref().unwrap().findings.len(), 1);
    }

    // ── skip_scan ────────────────────────────────────────────────────────

    #[test]
    fn skip_scan_transitions_to_continue() {
        let mut state = DiscoveryState::new();
        state.skip_scan();
        assert!(matches!(state.phase, DiscoveryPhase::Continue));
    }

    #[test]
    fn skip_scan_noop_after_results_set() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Warning,
            "w",
        )]));
        state.skip_scan();
        assert!(matches!(state.phase, DiscoveryPhase::Results { .. }));
    }

    // ── Key handling: Scanning phase ─────────────────────────────────────

    #[test]
    fn scanning_s_key_skips_to_continue() {
        let mut state = DiscoveryState::new();
        state.handle_key(Action::Character('s'));
        assert!(matches!(state.phase, DiscoveryPhase::Continue));
        assert!(!state.should_quit);
    }

    #[test]
    fn scanning_quit_sets_should_quit() {
        let mut state = DiscoveryState::new();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn scanning_other_keys_noop() {
        let mut state = DiscoveryState::new();
        state.handle_key(Action::Down);
        assert!(matches!(state.phase, DiscoveryPhase::Scanning { .. }));
        assert!(!state.should_quit);
    }

    // ── Key handling: Results phase ──────────────────────────────────────

    #[test]
    fn results_up_down_navigation() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![
            make_finding(FindingSeverity::Error, "e"),
            make_finding(FindingSeverity::Warning, "w"),
            make_finding(FindingSeverity::Info, "i"),
        ]));

        assert!(matches!(
            state.phase,
            DiscoveryPhase::Results { selected: 0 }
        ));

        state.handle_key(Action::Down);
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Results { selected: 1 }
        ));

        state.handle_key(Action::Down);
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Results { selected: 2 }
        ));

        state.handle_key(Action::Down); // at max
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Results { selected: 2 }
        ));

        state.handle_key(Action::Up);
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Results { selected: 1 }
        ));

        state.handle_key(Action::Up);
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Results { selected: 0 }
        ));

        state.handle_key(Action::Up); // at min
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Results { selected: 0 }
        ));
    }

    #[test]
    fn results_select_transitions_to_continue() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Error,
            "e",
        )]));
        state.handle_key(Action::Select);
        assert!(matches!(state.phase, DiscoveryPhase::Continue));
    }

    #[test]
    fn results_back_sets_wants_back() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Error,
            "e",
        )]));
        state.handle_key(Action::Back);
        assert!(state.wants_back);
    }

    #[test]
    fn results_quit_sets_should_quit() {
        let mut state = DiscoveryState::new();
        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Error,
            "e",
        )]));
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    // ── Key handling: Continue phase ─────────────────────────────────────

    #[test]
    fn continue_select_sets_wants_continue() {
        let mut state = DiscoveryState::new();
        state.skip_scan();
        state.handle_key(Action::Select);
        assert!(state.wants_continue);
        assert!(!state.should_quit);
        // should_quit() returns true when wants_continue is set so the surface
        // loop exits; caller distinguishes continue vs quit by checking wants_continue
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn should_quit_true_when_wants_continue() {
        let mut state = DiscoveryState::new();
        state.skip_scan();
        state.handle_key(Action::Select);
        assert!(state.wants_continue);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn continue_back_sets_wants_back() {
        let mut state = DiscoveryState::new();
        state.skip_scan();
        state.handle_key(Action::Back);
        assert!(state.wants_back);
    }

    #[test]
    fn continue_quit_sets_should_quit() {
        let mut state = DiscoveryState::new();
        state.skip_scan();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    // ── Surface trait ────────────────────────────────────────────────────

    #[test]
    fn surface_name_returns_expected() {
        let state = DiscoveryState::new();
        assert_eq!(state.surface_name(), "Discovery");
    }

    #[test]
    fn help_text_changes_per_phase() {
        let mut state = DiscoveryState::new();
        assert!(state.help_text().contains("skip"));

        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Error,
            "e",
        )]));
        assert!(state.help_text().contains("navigate"));

        state.handle_key(Action::Select);
        assert!(state.help_text().contains("continue"));
    }

    #[test]
    fn reset_returns_to_initial_state() {
        let mut state = DiscoveryState::new();
        state.update_progress(50);
        state.set_results(make_results(vec![make_finding(
            FindingSeverity::Error,
            "e",
        )]));
        state.should_quit = true;
        state.wants_continue = true;
        state.reset();

        assert!(matches!(
            state.phase,
            DiscoveryPhase::Scanning {
                files_scanned: 0,
                spinner_tick: 0
            }
        ));
        assert!(state.results.is_none());
        assert!(!state.should_quit);
        assert!(!state.wants_back);
        assert!(!state.wants_continue);
    }

    // ── Full flow ────────────────────────────────────────────────────────

    #[test]
    fn full_flow_scanning_to_results_to_continue() {
        let mut state = DiscoveryState::new();
        assert!(matches!(state.phase, DiscoveryPhase::Scanning { .. }));

        state.update_progress(100);
        state.tick();
        state.tick();

        state.set_results(make_results(vec![
            make_finding(FindingSeverity::Warning, "w1"),
            make_finding(FindingSeverity::Error, "e1"),
        ]));
        assert!(matches!(
            state.phase,
            DiscoveryPhase::Results { selected: 0 }
        ));

        state.handle_key(Action::Down);
        state.handle_key(Action::Select);
        assert!(matches!(state.phase, DiscoveryPhase::Continue));

        state.handle_key(Action::Select);
        assert!(state.wants_continue);
        // should_quit() returns true when wants_continue is set so the surface
        // loop exits; caller distinguishes continue vs quit by checking wants_continue
        assert!(Surface::should_quit(&state));
    }

    // ── ScanResults::filter_by_domain ───────────────────────────────────

    fn make_mixed_findings() -> ScanResults {
        ScanResults {
            findings: vec![
                make_finding_with_source(
                    FindingSeverity::Error,
                    FindingSource::AntiPattern,
                    "anti-pattern issue",
                ),
                make_finding_with_source(
                    FindingSeverity::Warning,
                    FindingSource::Secret,
                    "secret leak",
                ),
                make_finding_with_source(
                    FindingSeverity::Error,
                    FindingSource::Architecture,
                    "boundary violation",
                ),
            ],
            files_scanned: 150,
            duration_ms: 300,
            truncated: false,
            files_skipped_by_ignore: 3,
            is_showcase: true,
        }
    }

    #[test]
    fn filter_by_domain_policy_gets_antipattern_and_secret() {
        let results = make_mixed_findings();
        let filtered = results.filter_by_domain(TutorialPath::Policy);
        assert_eq!(filtered.findings.len(), 2);
        assert!(
            filtered
                .findings
                .iter()
                .all(|f| matches!(f.source, FindingSource::AntiPattern | FindingSource::Secret))
        );
    }

    #[test]
    fn filter_by_domain_preserves_files_skipped_by_ignore() {
        // SCAN-004: provenance must survive domain filtering — the skipped
        // count is a property of the scan, not of the findings subset.
        // CIB-170: the showcase flag is likewise a property of the scan
        // substitution, not the findings subset, so it must survive too —
        // otherwise a domain-filtered showcase view would drop the "Example"
        // framing and look like real findings.
        let results = make_mixed_findings();
        let filtered = results.filter_by_domain(TutorialPath::Architecture);
        assert_eq!(filtered.files_skipped_by_ignore, 3);
        assert!(filtered.is_showcase);
    }

    #[test]
    fn filter_by_domain_architecture_gets_architecture_only() {
        let results = make_mixed_findings();
        let filtered = results.filter_by_domain(TutorialPath::Architecture);
        assert_eq!(filtered.findings.len(), 1);
        assert_eq!(filtered.findings[0].source, FindingSource::Architecture);
    }

    #[test]
    fn filter_by_domain_drift_gets_all() {
        let results = make_mixed_findings();
        let filtered = results.filter_by_domain(TutorialPath::Drift);
        assert_eq!(filtered.findings.len(), 3);
    }

    #[test]
    fn filter_by_domain_ci_gets_all() {
        let results = make_mixed_findings();
        let filtered = results.filter_by_domain(TutorialPath::CI);
        assert_eq!(filtered.findings.len(), 3);
    }

    #[test]
    fn filter_by_domain_developer_acceleration_gets_all() {
        let results = make_mixed_findings();
        let filtered = results.filter_by_domain(TutorialPath::DeveloperAcceleration);
        assert_eq!(filtered.findings.len(), 3);
    }

    #[test]
    fn filter_by_domain_protection_loop_gets_all_and_preserves_metadata() {
        // LAUNCH-014: the value-first ProtectionLoop default returns
        // every finding (no domain narrowing in v1) AND preserves the
        // scan metadata. PR #1294 review fix (Copilot) — without this
        // pin, accidentally narrowing the new default path's findings
        // would break the spec invariant silently.
        let results = make_mixed_findings();
        let filtered = results.filter_by_domain(TutorialPath::ProtectionLoop);
        assert_eq!(
            filtered.findings.len(),
            3,
            "ProtectionLoop must return every finding regardless of source"
        );
        assert_eq!(filtered.files_scanned, 150);
        assert_eq!(filtered.duration_ms, 300);
        assert!(!filtered.truncated);
    }

    #[test]
    fn filter_by_domain_preserves_metadata() {
        let results = make_mixed_findings();
        let filtered = results.filter_by_domain(TutorialPath::Policy);
        assert_eq!(filtered.files_scanned, 150);
        assert_eq!(filtered.duration_ms, 300);
    }

    #[test]
    fn filter_by_domain_empty_results() {
        let results = ScanResults::default();
        let filtered = results.filter_by_domain(TutorialPath::Architecture);
        assert!(filtered.findings.is_empty());
        assert_eq!(filtered.files_scanned, 0);
        assert_eq!(filtered.duration_ms, 0);
    }
}
