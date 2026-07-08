//! Activation surface (ACTTUI-001) — the opt-in `anvil start` TUI scaffold.
//!
//! This module is the foundation slice for the activation TUI (ADR-103). It
//! introduces the phased state machine and a [`Surface`] implementation that
//! renders the already-composed activation verdict inside the anvil shell
//! chrome. Later work items thread live orchestrator step events into the
//! phases (ACTTUI-002/-003), replace `demand` pickers with the consent phase
//! (ACTTUI-004), and build the collapsible verdict tree (ACTTUI-005).
//!
//! ## Scope boundary (ACTTUI-001)
//!
//! - The surface is **opt-in** and additive: `anvil start` only enters it when
//!   the caller passes `--tui` / sets `ANVIL_ACTIVATION_TUI` *and* the session
//!   is genuinely interactive (see `crates/anvil-cli/src/commands/start.rs`).
//! - v1 renders the verdict text the plain path would have printed, so the TUI
//!   never claims more (or less) than the byte-stable plain surface. The phased
//!   enum exists now so downstream items have a stable seam to build on; v1
//!   constructs the surface already at [`ActivationPhase::Verdict`].
//! - When project writes are gated (a non-default `ANVIL_HOME`, ADR-060) the
//!   shell chrome carries a persistent banner naming the gated posture.

pub mod render;

use eddacraft_tui::keyboard::Action;

/// Ordered phases of an `anvil start` activation run.
///
/// The ordering is meaningful: [`ActivationPhase::next`] advances through the
/// run and saturates at [`ActivationPhase::Done`]. v1 (ACTTUI-001) builds the
/// surface directly at [`ActivationPhase::Verdict`] because the orchestrator
/// runs synchronously before the surface opens; the earlier phases become live
/// once ACTTUI-002/-003 stream orchestrator step events.
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
    /// byte-stable plain path). Rendered read-only in v1.
    verdict: String,
    /// Lifecycle/log seam from the activation orchestrator. ACTTUI-002 records
    /// these lines so later TUI work can render progress and logs without
    /// scraping the plain human diagnostic text.
    log_lines: Vec<String>,
    /// TUI-friendly progress rows derived from orchestrator lifecycle events.
    progress_steps: Vec<ActivationProgressStep>,
    /// Show a branded daemon spinner in the working panel.
    daemon_spinner: bool,
    /// True when a non-default `ANVIL_HOME` gates durable project writes
    /// (ADR-060). Drives the persistent shell banner.
    project_writes_gated: bool,
    /// Set when the user asks to leave the surface.
    should_quit: bool,
}

impl ActivationSurface {
    /// Build a v1 surface positioned at [`ActivationPhase::Verdict`] with the
    /// already-composed verdict text.
    #[must_use]
    pub fn from_verdict(verdict: impl Into<String>, project_writes_gated: bool) -> Self {
        Self {
            phase: ActivationPhase::Verdict,
            verdict: verdict.into(),
            log_lines: Vec::new(),
            progress_steps: Vec::new(),
            daemon_spinner: false,
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
        Self {
            phase: ActivationPhase::Verdict,
            verdict: verdict.into(),
            log_lines,
            progress_steps: Vec::new(),
            daemon_spinner: false,
            project_writes_gated,
            should_quit: false,
        }
    }

    /// Build a surface with orchestrator progress rows and optional daemon
    /// spinner. ACTTUI-003 uses this to render the completed working phase from
    /// ACTTUI-002 lifecycle events; future live streaming can update the same
    /// fields while the run is in progress.
    #[must_use]
    pub fn from_verdict_with_progress(
        verdict: impl Into<String>,
        project_writes_gated: bool,
        log_lines: Vec<String>,
        progress_steps: Vec<ActivationProgressStep>,
        daemon_spinner: bool,
        phase: ActivationPhase,
    ) -> Self {
        Self {
            phase,
            verdict: verdict.into(),
            log_lines,
            progress_steps,
            daemon_spinner,
            project_writes_gated,
            should_quit: false,
        }
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
}

impl eddacraft_tui::surface::Surface for ActivationSurface {
    // The `Surface` trait fixes the return type to `&str`; the impl signature
    // cannot widen to `&'static str`, so silence `unnecessary_literal_bound`
    // (same precedent as `eddacraft_tui::runner`).
    #[allow(clippy::unnecessary_literal_bound)]
    fn surface_name(&self) -> &str {
        "Activation"
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn help_text(&self) -> &str {
        "q quit"
    }

    fn handle_key(&mut self, action: Action) {
        // v1 is a read-only verdict view: any quit/back request dismisses it.
        // Richer key handling (collapse/expand, smoke test) arrives with the
        // verdict tree in ACTTUI-005.
        if matches!(action, Action::Quit | Action::Back) {
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
        assert!(surface.log_lines().is_empty());
        assert!(surface.progress_steps().is_empty());
        assert!(!surface.daemon_spinner());
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
    fn surface_metadata_is_stable() {
        let surface = ActivationSurface::from_verdict("x", false);
        assert_eq!(surface.surface_name(), "Activation");
        assert_eq!(surface.help_text(), "q quit");
    }
}
