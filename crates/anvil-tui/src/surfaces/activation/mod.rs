//! Activation surface (ACTTUI-001) — the default `anvil start` TUI.
//!
//! This module is the foundation slice for the activation TUI (ADR-103). It
//! provides the phased state machine used for one continuous activation
//! session. Typed orchestrator events update the preflight and working phases;
//! the same surface then owns consent and the collapsible verdict.
//!
//! ## Scope boundary (ACTTUI-001)
//!
//! - The surface is the **default** interactive path (ACTTUI-013, ADR-103
//!   Release 2): `anvil start` enters it whenever the session is genuinely
//!   interactive, with no opt-in. `--no-tui` / `ANVIL_NO_TUI=1`, `--verify`,
//!   `--json`, `--watch`, CI, and piped output stay on the deterministic plain
//!   contracts (see `crates/anvil-cli/src/commands/start.rs`).
//! - The verdict still derives from the plain path's typed diagnostic, so the
//!   TUI never claims more (or less) than the byte-stable plain surface.
//! - When project writes are gated (a non-default `ANVIL_HOME`, ADR-060) the
//!   shell chrome carries a persistent banner naming the gated posture.

pub mod consent;
pub mod log_panel;
pub mod render;
pub mod verdict;

use std::cell::{RefCell, RefMut};

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::prelude::{LogEntry, LogPanelState};

pub use consent::{ConsentDisabledReason, ConsentItem, ConsentKind, ConsentState};
pub use log_panel::entries_from_verdict as tier_evidence_entries_from_verdict;
pub use verdict::{ProveRunner, VerdictModel, VerdictSection, VerdictTone, VerdictView};

/// Ordered phases of an `anvil start` activation run.
///
/// The ordering is meaningful: [`ActivationPhase::next`] advances through the
/// run and saturates at [`ActivationPhase::Done`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActivationPhase {
    /// Pre-flight: config probe, identity, daemon-ensure decision.
    Preflight,
    /// Work in progress: daemon ensure, init, baseline sample.
    Working,
    /// Awaiting consent: MCP / workflow / hook pickers.
    Consent,
    /// Protection verdict is ready to read.
    Verdict,
    /// Terminal state — the surface can be dismissed.
    Done,
}

impl ActivationPhase {
    /// The phase label shown in the shell chrome / phase indicator.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Preflight => "Preflight",
            Self::Working => "Working",
            Self::Consent => "Consent",
            Self::Verdict => "Verdict",
            Self::Done => "Done",
        }
    }

    /// Advance to the next phase, saturating at [`ActivationPhase::Done`].
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Preflight => Self::Working,
            Self::Working => Self::Consent,
            Self::Consent => Self::Verdict,
            Self::Verdict | Self::Done => Self::Done,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TierEvidencePane {
    Hidden,
    Visible,
}

impl TierEvidencePane {
    fn toggle(&mut self) {
        *self = match self {
            Self::Hidden => Self::Visible,
            Self::Visible => Self::Hidden,
        };
    }

    fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }
}

/// Display state for one activation progress row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationProgressStatus {
    Pending,
    Running,
    Passed,
    Skipped,
    Failed,
}

/// A TUI-friendly projection of one orchestrator step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationProgressStep {
    pub id: String,
    pub label: String,
    pub status: ActivationProgressStatus,
    pub message: Option<String>,
}

impl ActivationProgressStep {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        status: ActivationProgressStatus,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status,
            message: None,
        }
    }

    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Interactive state for the `anvil start` activation surface.
pub struct ActivationSurface {
    /// Current phase of the run.
    phase: ActivationPhase,
    /// The composed, plain-text activation verdict (identical content to the
    /// byte-stable plain path). Retained for fallback and contract tests.
    verdict: String,
    /// Structured, collapsible verdict view for ACTTUI-005.
    verdict_view: VerdictView,
    /// Typed tier/install/daemon evidence rendered through eddacraft-tui `LogPanel`.
    tier_evidence_entries: Vec<LogEntry>,
    /// Mutable `LogPanel` widget state; render only has `&self`, so this uses
    /// interior mutability like other stateful widget adapters in this surface.
    log_panel_state: RefCell<LogPanelState>,
    /// Visibility state for the tier-evidence panel.
    tier_evidence_pane: TierEvidencePane,
    /// Lifecycle/log seam from the activation orchestrator. ACTTUI-002 records
    /// these lines so later TUI work can render progress and logs without
    /// scraping the plain human diagnostic text.
    log_lines: Vec<String>,
    /// TUI-friendly progress rows derived from orchestrator lifecycle events.
    progress_steps: Vec<ActivationProgressStep>,
    /// Show a branded daemon spinner in the working panel.
    daemon_spinner: bool,
    /// Deferred write consent owned by the activation TUI.
    consent: Option<ConsentState>,
    /// True when a non-default `ANVIL_HOME` gates durable project writes
    /// (ADR-060). Drives the persistent shell banner.
    project_writes_gated: bool,
    /// Set when the user asks to leave the surface.
    should_quit: bool,
}

impl ActivationSurface {
    /// Build the first frame of a live activation run.
    #[must_use]
    pub fn live(project_writes_gated: bool, daemon_spinner: bool) -> Self {
        Self::from_verdict_with_progress(
            "ACTIVATION\n",
            project_writes_gated,
            Vec::new(),
            Vec::new(),
            daemon_spinner,
            ActivationPhase::Preflight,
        )
    }

    /// Replace the live activity projection as typed orchestrator events arrive.
    pub fn update_live_progress(
        &mut self,
        log_lines: Vec<String>,
        progress_steps: Vec<ActivationProgressStep>,
    ) {
        self.phase = ActivationPhase::Working;
        self.tier_evidence_entries = log_panel::entries_from_lifecycle(&log_lines);
        self.log_lines = log_lines;
        self.progress_steps = progress_steps;
        self.daemon_spinner = false;
    }

    /// Build a v1 surface positioned at [`ActivationPhase::Verdict`] with the
    /// already-composed verdict text.
    #[must_use]
    pub fn from_verdict(verdict: impl Into<String>, project_writes_gated: bool) -> Self {
        let verdict = verdict.into();
        let verdict_view = VerdictView::new(VerdictModel::from_plain(&verdict));
        Self {
            phase: ActivationPhase::Verdict,
            tier_evidence_entries: log_panel::entries_from_verdict(&verdict),
            verdict,
            verdict_view,
            log_panel_state: RefCell::new(default_log_panel_state()),
            tier_evidence_pane: TierEvidencePane::Hidden,
            log_lines: Vec::new(),
            progress_steps: Vec::new(),
            daemon_spinner: false,
            consent: None,
            project_writes_gated,
            should_quit: false,
        }
    }

    /// Build a verdict surface with orchestrator lifecycle/log lines attached.
    #[must_use]
    pub fn from_verdict_with_logs(
        verdict: impl Into<String>,
        project_writes_gated: bool,
        log_lines: Vec<String>,
    ) -> Self {
        let verdict = verdict.into();
        let verdict_view = VerdictView::new(VerdictModel::from_plain(&verdict));
        let mut tier_evidence_entries = log_panel::entries_from_verdict(&verdict);
        tier_evidence_entries.extend(log_panel::entries_from_lifecycle(&log_lines));
        Self {
            phase: ActivationPhase::Verdict,
            verdict,
            verdict_view,
            tier_evidence_entries,
            log_panel_state: RefCell::new(default_log_panel_state()),
            tier_evidence_pane: TierEvidencePane::Hidden,
            log_lines,
            progress_steps: Vec::new(),
            daemon_spinner: false,
            consent: None,
            project_writes_gated,
            should_quit: false,
        }
    }

    /// Build a surface with orchestrator progress rows and optional daemon
    /// spinner. The live path updates the same fields while the run is in
    /// progress, then the consent and verdict phases reuse the final projection.
    #[must_use]
    pub fn from_verdict_with_progress(
        verdict: impl Into<String>,
        project_writes_gated: bool,
        log_lines: Vec<String>,
        progress_steps: Vec<ActivationProgressStep>,
        daemon_spinner: bool,
        phase: ActivationPhase,
    ) -> Self {
        let verdict = verdict.into();
        let verdict_view = VerdictView::new(VerdictModel::from_plain(&verdict));
        let mut tier_evidence_entries = log_panel::entries_from_verdict(&verdict);
        tier_evidence_entries.extend(log_panel::entries_from_lifecycle(&log_lines));
        Self {
            phase,
            verdict,
            verdict_view,
            tier_evidence_entries,
            log_panel_state: RefCell::new(default_log_panel_state()),
            tier_evidence_pane: TierEvidencePane::Hidden,
            log_lines,
            progress_steps,
            daemon_spinner,
            consent: None,
            project_writes_gated,
            should_quit: false,
        }
    }

    /// Build a progress surface from caller-supplied typed verdict and evidence
    /// projections. The plain verdict is retained solely as the fallback text
    /// contract; it is not parsed to construct either TUI model.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_typed_with_progress(
        verdict: impl Into<String>,
        verdict_model: VerdictModel,
        tier_evidence_entries: Vec<LogEntry>,
        project_writes_gated: bool,
        log_lines: Vec<String>,
        progress_steps: Vec<ActivationProgressStep>,
        daemon_spinner: bool,
        phase: ActivationPhase,
    ) -> Self {
        Self {
            phase,
            verdict: verdict.into(),
            verdict_view: VerdictView::new(verdict_model),
            tier_evidence_entries,
            log_panel_state: RefCell::new(default_log_panel_state()),
            tier_evidence_pane: TierEvidencePane::Hidden,
            log_lines,
            progress_steps,
            daemon_spinner,
            consent: None,
            project_writes_gated,
            should_quit: false,
        }
    }

    /// Attach deferred consent rows to this surface.
    #[must_use]
    pub fn with_consent(mut self, consent: ConsentState) -> Self {
        self.phase = ActivationPhase::Consent;
        self.consent = Some(consent);
        self
    }

    /// Replace the fallback parsed verdict with a structured model from the CLI.
    #[must_use]
    pub fn with_verdict_model(mut self, model: VerdictModel) -> Self {
        self.verdict_view = VerdictView::new(model);
        self
    }

    /// Replace tier-evidence rows with a caller-supplied typed set.
    #[must_use]
    pub fn with_tier_evidence_entries(mut self, entries: Vec<LogEntry>) -> Self {
        self.tier_evidence_entries = entries;
        self
    }

    /// Toggle the ACTTUI-006 tier-evidence panel. Exposed as a state method so
    /// tests and future key maps can drive the same behaviour as `e`.
    pub fn toggle_tier_evidence(&mut self) {
        self.tier_evidence_pane.toggle();
    }

    #[must_use]
    pub fn tier_evidence_visible(&self) -> bool {
        self.tier_evidence_pane.is_visible()
    }

    #[must_use]
    pub fn tier_evidence_entries(&self) -> &[LogEntry] {
        &self.tier_evidence_entries
    }

    pub(crate) fn log_panel_state_mut(&self) -> RefMut<'_, LogPanelState> {
        self.log_panel_state.borrow_mut()
    }

    /// Current phase (used by tests and future step-event wiring).
    #[must_use]
    pub fn phase(&self) -> ActivationPhase {
        self.phase
    }

    /// Advance to the next phase. Foundation seam for ACTTUI-002/-003.
    pub fn advance(&mut self) {
        self.phase = self.phase.next();
    }

    /// Whether the persistent gated-`ANVIL_HOME` banner should render.
    #[must_use]
    pub fn project_writes_gated(&self) -> bool {
        self.project_writes_gated
    }

    /// The composed verdict text (used by the renderer and tests).
    #[must_use]
    pub fn verdict(&self) -> &str {
        &self.verdict
    }

    /// Structured verdict view rendered on the Verdict phase (ACTTUI-005).
    #[must_use]
    pub fn verdict_view(&self) -> &VerdictView {
        &self.verdict_view
    }

    /// Mutable access for CLI wiring (e.g. ACTTUI-016 Prove runner).
    pub fn verdict_view_mut(&mut self) -> &mut VerdictView {
        &mut self.verdict_view
    }

    /// Attach an in-surface Prove runner to the verdict view.
    #[must_use]
    pub fn with_prove(mut self, prove: verdict::ProveRunner) -> Self {
        self.verdict_view.set_prove(prove);
        self
    }

    /// Orchestrator lifecycle/log lines captured for future in-surface
    /// rendering.
    #[must_use]
    pub fn log_lines(&self) -> &[String] {
        &self.log_lines
    }

    #[must_use]
    pub fn progress_steps(&self) -> &[ActivationProgressStep] {
        &self.progress_steps
    }

    #[must_use]
    pub fn daemon_spinner(&self) -> bool {
        self.daemon_spinner
    }

    #[must_use]
    pub fn consent(&self) -> Option<&ConsentState> {
        self.consent.as_ref()
    }

    fn handle_log_panel_key(&mut self, action: Action) {
        let panel_state = self.log_panel_state.get_mut();
        let visible_count = panel_state
            .filtered_indices(&self.tier_evidence_entries)
            .len();
        match action {
            Action::Up | Action::Character('k' | 'K') => panel_state.scroll_up(),
            Action::Down | Action::Character('j' | 'J') => panel_state.scroll_down(visible_count),
            Action::Character('g') => panel_state.jump_to_top(),
            Action::Character('G') => panel_state.jump_to_bottom(visible_count),
            Action::Back => self.tier_evidence_pane = TierEvidencePane::Hidden,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

fn default_log_panel_state() -> LogPanelState {
    let mut state = LogPanelState::default();
    state.auto_scroll = true;
    state
}

impl eddacraft_tui::surface::Surface for ActivationSurface {
    // The `Surface` trait fixes the return type to `&str`; the impl signature
    // cannot widen to `&'static str`, so silence `unnecessary_literal_bound`
    // (same precedent as `eddacraft_tui::runner`).
    #[allow(clippy::unnecessary_literal_bound)]
    fn surface_name(&self) -> &str {
        "Activation"
    }

    // One exit-key story with the welcome surface (first-run council C-014):
    // `q` always quits and `esc` backs out one level — closing the evidence
    // panel or the unsafe-drift overlay first, quitting from the base panes.
    #[allow(clippy::unnecessary_literal_bound)]
    fn help_text(&self) -> &str {
        if self.tier_evidence_pane.is_visible() {
            "j/k scroll  g/G top/bottom  esc close  q quit"
        } else if matches!(
            self.phase,
            ActivationPhase::Preflight | ActivationPhase::Working
        ) {
            "activation running"
        } else if self.phase == ActivationPhase::Consent {
            match &self.consent {
                // The unsafe-drift confirm overlay owns the keys while open.
                Some(consent) if consent.unsafe_confirm_index.is_some() => {
                    "y confirm  n/esc cancel  q quit"
                }
                // CIB-245: `h`/`l` step between consent sections when the run
                // offers more than one.
                // `enter select` must stay advertised in both arms: it is the
                // only key that opens the unsafe-drift confirm overlay, and
                // ACTTUI-015 makes this bar the sole key legend.
                Some(consent) if consent.steps().len() > 1 => {
                    "j/k navigate  h/l section  space toggle  enter select  a apply  esc/q quit"
                }
                Some(_) => "j/k navigate  space toggle  enter select  a apply  esc/q quit",
                None => "esc/q quit",
            }
        } else {
            "j/k navigate  enter expand  t prove  e evidence  esc/q quit"
        }
    }

    fn handle_key(&mut self, action: Action) {
        // `e` toggles tier evidence. The production key map (`KeyHandler::map`)
        // reserves `j`/`k`/`h`/`l` for arrow navigation, so the toggle must
        // live on a key that still arrives as `Action::Character`. It is
        // inert during Consent — the consent pane's help never advertises it,
        // and letting it fire there would silently swap the pane out from
        // under the picker (and swallow the next consent keystroke).
        if matches!(action, Action::Character('e' | 'E')) && self.phase != ActivationPhase::Consent
        {
            self.toggle_tier_evidence();
            return;
        }

        if self.tier_evidence_pane.is_visible() {
            self.handle_log_panel_key(action);
            return;
        }

        if let Some(consent) = self.consent.as_mut() {
            match action {
                Action::Up => consent.previous(),
                Action::Down => consent.next(),
                // CIB-245: section stepping. Inert while the unsafe-drift
                // overlay owns the keys, so a stray `h` cannot skip the
                // decision the overlay is asking for.
                Action::Left if consent.unsafe_confirm_index.is_none() => consent.previous_step(),
                Action::Right if consent.unsafe_confirm_index.is_none() => consent.next_step(),
                Action::Toggle => consent.toggle_current(),
                Action::Select => consent.select_current(),
                Action::Character('y' | 'Y') if consent.unsafe_confirm_index.is_some() => {
                    consent.confirm_unsafe(true);
                }
                Action::Character('n' | 'N') | Action::Back
                    if consent.unsafe_confirm_index.is_some() =>
                {
                    consent.confirm_unsafe(false);
                }
                Action::Character('a' | 'A') if consent.unsafe_confirm_index.is_none() => {
                    consent.submit();
                    self.should_quit = true;
                }
                Action::Quit | Action::Back => self.should_quit = true,
                _ => {}
            }
            return;
        }

        // Verdict phase (ACTTUI-005): the structured view owns tree navigation,
        // section expand/collapse, and the prove key; it reports back when
        // the user asks to leave.
        if self.verdict_view.handle_key(action) {
            self.should_quit = true;
        }
    }

    fn should_quit(&self) -> bool {
        self.should_quit
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
    use eddacraft_tui::surface::Surface;

    #[test]
    fn phase_advances_and_saturates() {
        assert_eq!(ActivationPhase::Preflight.next(), ActivationPhase::Working);
        assert_eq!(ActivationPhase::Working.next(), ActivationPhase::Consent);
        assert_eq!(ActivationPhase::Consent.next(), ActivationPhase::Verdict);
        assert_eq!(ActivationPhase::Verdict.next(), ActivationPhase::Done);
        assert_eq!(ActivationPhase::Done.next(), ActivationPhase::Done);
    }

    #[test]
    fn phase_ordering_is_run_ordered() {
        assert!(ActivationPhase::Preflight < ActivationPhase::Verdict);
        assert!(ActivationPhase::Verdict < ActivationPhase::Done);
    }

    #[test]
    fn from_verdict_starts_at_verdict_phase() {
        let surface = ActivationSurface::from_verdict("ACTIVATION\n  state: protecting\n", false);
        assert_eq!(surface.phase(), ActivationPhase::Verdict);
        assert!(!surface.project_writes_gated());
        assert!(surface.verdict().contains("state: protecting"));
        assert_eq!(surface.verdict_view().model().state_label, "protecting");
        assert!(surface.log_lines().is_empty());
        assert!(surface.progress_steps().is_empty());
        assert!(!surface.daemon_spinner());
        assert!(surface.consent().is_none());
    }

    #[test]
    fn from_verdict_with_logs_records_orchestrator_buffer() {
        let surface = ActivationSurface::from_verdict_with_logs(
            "ACTIVATION\n  state: protecting\n",
            false,
            vec!["initial-probe: completed".to_string()],
        );
        assert_eq!(surface.phase(), ActivationPhase::Verdict);
        assert_eq!(surface.log_lines(), ["initial-probe: completed"]);
    }

    #[test]
    fn from_verdict_with_progress_records_rows_and_phase() {
        let surface = ActivationSurface::from_verdict_with_progress(
            "ACTIVATION
  state: ready_restart_required
",
            false,
            vec!["daemon: ensured".to_string()],
            vec![ActivationProgressStep::new(
                "daemon",
                "Daemon ensure",
                ActivationProgressStatus::Passed,
            )],
            true,
            ActivationPhase::Consent,
        );
        assert_eq!(surface.phase(), ActivationPhase::Consent);
        assert_eq!(surface.log_lines(), ["daemon: ensured"]);
        assert_eq!(surface.progress_steps().len(), 1);
        assert!(surface.daemon_spinner());
    }

    #[test]
    fn live_surface_moves_from_preflight_to_working_as_events_arrive() {
        let mut surface = ActivationSurface::live(false, true);
        assert_eq!(surface.phase(), ActivationPhase::Preflight);
        assert!(surface.daemon_spinner());

        surface.update_live_progress(
            vec!["initial-probe: started".to_string()],
            vec![ActivationProgressStep::new(
                "initial-probe",
                "Initial probe",
                ActivationProgressStatus::Running,
            )],
        );

        assert_eq!(surface.phase(), ActivationPhase::Working);
        assert_eq!(surface.log_lines(), ["initial-probe: started"]);
        assert_eq!(surface.progress_steps().len(), 1);
        assert!(!surface.daemon_spinner());
    }

    #[test]
    fn with_consent_switches_to_consent_phase_and_handles_selection() {
        let consent = ConsentState::new(
            vec![ConsentItem::new(
                "cursor",
                "Cursor MCP",
                "write config",
                ConsentKind::Mcp,
            )],
            false,
        );
        let mut surface = ActivationSurface::from_verdict("x", false).with_consent(consent);
        assert_eq!(surface.phase(), ActivationPhase::Consent);
        surface.handle_key(Action::Toggle);
        assert_eq!(surface.consent().unwrap().selected_ids(), ["cursor"]);
    }

    /// CIB-245 regression: grouping consent into steps must not drop
    /// `enter select` from the help bar. Unsafe-drift rows are unselectable,
    /// so Enter is the only key that opens their confirm overlay — and this
    /// bar is the only place any consent key is advertised (ACTTUI-015).
    #[test]
    fn multi_section_consent_help_still_advertises_enter_select() {
        let consent = ConsentState::new(
            vec![
                ConsentItem::new(
                    "project",
                    "Project configuration",
                    "Create .anvil.yaml",
                    ConsentKind::Project,
                )
                .repo_scoped(),
                ConsentItem::new(
                    "cursor",
                    "Cursor MCP",
                    "Write ~/.cursor/mcp.json",
                    ConsentKind::Mcp,
                )
                .unsafe_drift("existing foreign mcpServers.anvil entry"),
            ],
            false,
        );
        let surface = ActivationSurface::from_verdict("x", false).with_consent(consent);
        assert!(surface.consent().unwrap().steps().len() > 1);
        let help = surface.help_text();
        assert!(help.contains("enter select"), "{help}");
        assert!(help.contains("h/l section"), "{help}");
    }

    #[test]
    fn evidence_key_is_inert_during_consent() {
        // Council C-001 (session council-eb8c0152): `e` fired before the
        // phase branch and hijacked the consent pane. It must be inert in
        // Consent — routed through the production key map, the pane stays
        // put and the next consent keystroke still lands.
        let consent = ConsentState::new(
            vec![ConsentItem::new(
                "cursor",
                "Cursor MCP",
                "write config",
                ConsentKind::Mcp,
            )],
            false,
        );
        let mut surface = ActivationSurface::from_verdict("x", false).with_consent(consent);
        surface.handle_key(production_action('e'));
        assert!(!surface.tier_evidence_visible());
        assert_eq!(surface.phase(), ActivationPhase::Consent);
        surface.handle_key(Action::Toggle);
        assert_eq!(surface.consent().unwrap().selected_ids(), ["cursor"]);
    }

    #[test]
    fn unsafe_overlay_help_advertises_only_its_live_keys() {
        use eddacraft_tui::surface::Surface as _;
        // The confirm overlay swallows the base consent keys, so the help
        // bar must switch to the overlay's own y/n/esc story while open.
        let consent = ConsentState::new(
            vec![ConsentItem::new(
                "cursor",
                "Cursor MCP",
                "write config",
                ConsentKind::Mcp,
            )],
            false,
        );
        let mut surface = ActivationSurface::from_verdict("x", false).with_consent(consent);
        assert!(surface.help_text().contains("a apply"));
        surface.consent.as_mut().unwrap().unsafe_confirm_index = Some(0);
        assert_eq!(surface.help_text(), "y confirm  n/esc cancel  q quit");
    }

    #[test]
    fn consent_quit_still_exits_surface() {
        let consent = ConsentState::new(Vec::new(), false);
        let mut surface = ActivationSurface::from_verdict("x", false).with_consent(consent);
        surface.handle_key(Action::Quit);
        assert!(surface.should_quit());
        assert!(!surface.consent().unwrap().submitted());
    }

    #[test]
    fn consent_apply_exits_surface_and_marks_submission() {
        let consent = ConsentState::new(
            vec![ConsentItem::new(
                "cursor",
                "Cursor MCP",
                "write config",
                ConsentKind::Mcp,
            )],
            false,
        );
        let mut surface = ActivationSurface::from_verdict("x", false).with_consent(consent);
        surface.handle_key(Action::Toggle);
        surface.handle_key(Action::Character('a'));

        assert!(surface.should_quit());
        assert!(surface.consent().unwrap().submitted());
        assert_eq!(surface.consent().unwrap().selected_ids(), ["cursor"]);
    }

    #[test]
    fn consent_unsafe_acknowledgement_only_applies_when_overlay_is_open() {
        let consent = ConsentState::new(
            vec![ConsentItem::new(
                "cursor",
                "Cursor MCP",
                "write config",
                ConsentKind::Mcp,
            )],
            false,
        );
        let mut surface = ActivationSurface::from_verdict("x", false).with_consent(consent);
        surface.handle_key(Action::Character('y'));
        assert_eq!(surface.consent().unwrap().unsafe_confirmed, None);
    }

    #[test]
    fn advance_moves_phase_forward() {
        let mut surface = ActivationSurface::from_verdict("x", false);
        surface.advance();
        assert_eq!(surface.phase(), ActivationPhase::Done);
    }

    #[test]
    fn quit_action_requests_exit() {
        let mut surface = ActivationSurface::from_verdict("x", false);
        assert!(!surface.should_quit());
        surface.handle_key(Action::Quit);
        assert!(surface.should_quit());
    }

    #[test]
    fn back_action_also_dismisses_readonly_view() {
        let mut surface = ActivationSurface::from_verdict("x", false);
        surface.handle_key(Action::Back);
        assert!(surface.should_quit());
    }

    #[test]
    fn verdict_key_handling_toggles_tree_and_prove_toast() {
        let model = VerdictModel::new(
            "protecting",
            "Protecting — pre-write validation is live.",
            vec![VerdictSection::new(
                "layers",
                "Layers",
                vec!["L0 mcp pre-write".to_string()],
            )],
        );
        let mut surface =
            ActivationSurface::from_verdict("ACTIVATION\n  state: protecting\n", false)
                .with_verdict_model(model);
        assert!(!surface.verdict_view().is_expanded("layers"));
        surface.handle_key(Action::Select);
        assert!(surface.verdict_view().is_expanded("layers"));
        surface.handle_key(Action::Character('t'));
        assert!(surface.verdict_view().toast().is_some());
    }

    #[test]
    fn tier_evidence_entries_include_plain_install_and_lifecycle_rows() {
        let surface = ActivationSurface::from_verdict_with_logs(
            "ACTIVATION
  state: protecting
  mcp:
    Cursor: live_validation
  install:
    Cursor: skipped — already up to date
",
            false,
            vec!["anvil: ensuring the per-user save-time daemon is running…".to_string()],
        );

        assert!(surface.tier_evidence_entries().iter().any(|entry| {
            entry.source == "mcp/Cursor" && entry.message == "tier: live_validation"
        }));
        assert!(surface.tier_evidence_entries().iter().any(|entry| {
            entry.source == "install/Cursor" && entry.message == "skipped — already up to date"
        }));
        assert!(
            surface
                .tier_evidence_entries()
                .iter()
                .any(|entry| entry.source == "orchestrator")
        );
    }

    /// Route a character through the production key map. The run loop maps
    /// keys with `KeyHandler::map`, which reserves `j`/`k`/`h`/`l` for arrow
    /// navigation — tests that inject raw `Action::Character` values can pass
    /// while the advertised key is dead in a real terminal (C-013).
    fn production_action(ch: char) -> Action {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        eddacraft_tui::keyboard::KeyHandler::map(KeyEvent::new(
            KeyCode::Char(ch),
            KeyModifiers::NONE,
        ))
    }

    #[test]
    fn e_key_toggles_tier_evidence_through_the_production_key_map() {
        let mut surface = ActivationSurface::from_verdict(
            "ACTIVATION
  state: ready_restart_required
  mcp:
    Cursor: restart_handshake_verified (pending restart)
",
            false,
        );

        assert!(!surface.tier_evidence_visible());
        surface.handle_key(production_action('e'));
        assert!(surface.tier_evidence_visible());
        assert_eq!(
            surface.verdict_view().model().state_label,
            "ready_restart_required"
        );
        surface.handle_key(production_action('e'));
        assert!(!surface.tier_evidence_visible());
    }

    #[test]
    fn tier_evidence_opens_without_any_startup_flag() {
        // C-013: the pane must be available on demand for every TUI run — it
        // must not depend on `--why` (or any other flag) at process start.
        let mut surface = ActivationSurface::from_verdict(
            "ACTIVATION
  state: protecting
  mcp:
    Cursor: live_validation
",
            false,
        );
        surface.handle_key(production_action('e'));
        assert!(surface.tier_evidence_visible());
        assert!(!surface.tier_evidence_entries().is_empty());
    }

    #[test]
    fn surface_metadata_is_stable() {
        let surface = ActivationSurface::from_verdict("x", false);
        assert_eq!(surface.surface_name(), "Activation");
        assert_eq!(
            surface.help_text(),
            "j/k navigate  enter expand  t prove  e evidence  esc/q quit"
        );
    }

    #[test]
    fn help_text_names_one_exit_key_story_per_pane() {
        // C-014: activation and welcome share the `esc/q quit` exit story.
        let mut surface = ActivationSurface::from_verdict("x", false);
        assert!(surface.help_text().ends_with("esc/q quit"));

        surface.handle_key(production_action('e'));
        assert_eq!(
            surface.help_text(),
            "j/k scroll  g/G top/bottom  esc close  q quit"
        );

        let consent = ConsentState::new(
            vec![ConsentItem::new(
                "cursor",
                "Cursor MCP",
                "write config",
                ConsentKind::Mcp,
            )],
            false,
        );
        let consent_surface = ActivationSurface::from_verdict("x", false).with_consent(consent);
        assert!(consent_surface.help_text().ends_with("esc/q quit"));
    }
}
