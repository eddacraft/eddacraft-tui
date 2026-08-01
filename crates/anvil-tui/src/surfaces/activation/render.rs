use eddacraft_tui::prelude::{
    CheckProgress, CheckStatus, ParallelProgress, ParallelProgressState, Spinner, SpinnerState,
};
use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget};

use super::{ActivationPhase, ActivationProgressStatus, ActivationSurface};
use crate::shell::inset_content;

/// Render the ACTTUI-001 foundation surface.
pub fn render(frame: &mut Frame, area: Rect, state: &ActivationSurface, theme: &EddaCraftTheme) {
    let area = inset_content(area);
    let progress_height = if state.progress_steps().is_empty() && !state.daemon_spinner() {
        0
    } else {
        8
    };
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(if state.project_writes_gated() { 3 } else { 0 }),
        Constraint::Length(progress_height),
        Constraint::Min(6),
    ])
    .split(area);

    render_phase_strip(frame, chunks[0], state.phase(), theme);
    if state.project_writes_gated() {
        render_gated_banner(frame, chunks[1], theme);
    }
    if progress_height > 0 {
        render_working_progress(frame, chunks[2], state, theme);
    }
    if state.tier_evidence_visible() {
        let mut panel_state = state.log_panel_state_mut();
        crate::surfaces::activation::log_panel::render(
            frame,
            chunks[3],
            state.tier_evidence_entries(),
            &mut panel_state,
            theme,
        );
    } else if matches!(
        state.phase(),
        ActivationPhase::Preflight | ActivationPhase::Working
    ) {
        let mut panel_state = state.log_panel_state_mut();
        crate::surfaces::activation::log_panel::render_activity(
            frame,
            chunks[3],
            state.tier_evidence_entries(),
            &mut panel_state,
            theme,
        );
    } else if state.phase() == ActivationPhase::Consent {
        if let Some(consent) = state.consent() {
            crate::surfaces::activation::consent::render(frame, chunks[3], consent, theme);
        } else {
            render_missing_consent_guard(frame, chunks[3], theme);
        }
    } else {
        crate::surfaces::activation::verdict::render(frame, chunks[3], state.verdict_view(), theme);
    }
}

fn render_missing_consent_guard(frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.warning()))
        .title(" Consent unavailable ");
    let text = vec![
        Line::styled(
            "Consent is unavailable for this activation run.",
            Style::default()
                .fg(theme.warning())
                .add_modifier(Modifier::BOLD),
        ),
        Line::from("No project or editor changes have been applied."),
        Line::styled(
            "Press q to leave, then re-run `anvil start`.",
            Style::default().fg(theme.muted()),
        ),
    ];
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn render_phase_strip(
    frame: &mut Frame,
    area: Rect,
    current: ActivationPhase,
    theme: &EddaCraftTheme,
) {
    let phases = [
        ActivationPhase::Preflight,
        ActivationPhase::Working,
        ActivationPhase::Consent,
        ActivationPhase::Verdict,
        ActivationPhase::Done,
    ];
    let spans: Vec<Span> = phases
        .iter()
        .enumerate()
        .flat_map(|(idx, phase)| {
            let style = match phase.cmp(&current) {
                std::cmp::Ordering::Less => Style::default().fg(theme.success()),
                std::cmp::Ordering::Equal => Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
                std::cmp::Ordering::Greater => Style::default().fg(theme.muted()),
            };
            let label = if *phase == current {
                format!("[{}]", phase.label())
            } else {
                phase.label().to_string()
            };
            let separator = if idx < phases.len() - 1 { " > " } else { "" };
            vec![
                Span::styled(label, style),
                Span::styled(separator, Style::default().fg(theme.muted())),
            ]
        })
        .collect();

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.muted()));
    frame.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

fn render_gated_banner(frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.warning()))
        .title(" ANVIL_HOME gated ");
    let text = Line::styled(
        "Project writes are gated for this candidate install; repo-scoped offers stay read-only.",
        Style::default().fg(theme.warning()),
    );
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn render_working_progress(
    frame: &mut Frame,
    area: Rect,
    state: &ActivationSurface,
    theme: &EddaCraftTheme,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let chunks = if state.daemon_spinner() {
        Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(area)
    } else {
        Layout::vertical([Constraint::Length(0), Constraint::Min(3)]).split(area)
    };

    if state.daemon_spinner() {
        let mut spinner_state = SpinnerState::default();
        Spinner::new(theme)
            .anvil()
            .label("Ensuring save-time daemon")
            .render(chunks[0], frame.buffer_mut(), &mut spinner_state);
    }

    let checks: Vec<CheckProgress> = state
        .progress_steps()
        .iter()
        .map(|step| {
            let mut check = CheckProgress::new(step.id.clone(), step.label.clone());
            check.status = map_status(step.status);
            check.progress = match step.status {
                ActivationProgressStatus::Running => 50,
                ActivationProgressStatus::Pending => 0,
                ActivationProgressStatus::Passed
                | ActivationProgressStatus::Skipped
                | ActivationProgressStatus::Failed => 100,
            };
            check.message.clone_from(&step.message);
            check
        })
        .collect();

    let mut progress_state = ParallelProgressState::default();
    progress_state.checks = checks;
    ParallelProgress::new(theme)
        .title("Activation progress")
        .show_overall(false)
        .show_eta(false)
        .render(chunks[1], frame.buffer_mut(), &mut progress_state);
}

fn map_status(status: ActivationProgressStatus) -> CheckStatus {
    match status {
        ActivationProgressStatus::Pending => CheckStatus::Pending,
        ActivationProgressStatus::Running => CheckStatus::Running,
        ActivationProgressStatus::Passed => CheckStatus::Passed,
        ActivationProgressStatus::Skipped => CheckStatus::Skipped,
        ActivationProgressStatus::Failed => CheckStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// Concatenate only cell symbols (no style annotations) per row so
    /// substring assertions are stable. `buffer_to_string` interleaves style
    /// annotations between glyphs, which is right for snapshots but breaks
    /// `contains` checks.
    fn render_to_string(surface: &ActivationSurface, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), surface, &theme))
            .unwrap();
        let buf = terminal.backend().buffer();
        let area = buf.area;
        let mut out = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn renders_verdict_text_and_phase() {
        let surface = ActivationSurface::from_verdict(
            "ACTIVATION\n  state: protecting\n  next: done\n",
            false,
        );
        let out = render_to_string(&surface, 80, 20);
        assert!(out.contains("Preflight"));
        assert!(out.contains("Verdict"));
        assert!(out.contains("[Verdict]"));
        assert!(out.contains("state: protecting"));
        assert!(out.contains("Activation verdict"));
        assert!(out.contains("Smoke"));
    }

    #[test]
    fn renders_working_progress_from_orchestrator_steps() {
        let surface = ActivationSurface::from_verdict_with_progress(
            "ACTIVATION
  state: ready_restart_required
",
            false,
            vec![],
            vec![
                super::super::ActivationProgressStep::new(
                    "initial-probe",
                    "Initial probe",
                    ActivationProgressStatus::Passed,
                ),
                super::super::ActivationProgressStep::new(
                    "mcp-consent",
                    "MCP consent",
                    ActivationProgressStatus::Skipped,
                )
                .with_message("deferred to activation TUI"),
            ],
            true,
            ActivationPhase::Consent,
        );
        let out = render_to_string(&surface, 100, 28);
        assert!(out.contains("Activation progress"));
        assert!(out.contains("Ensuring save-time daemon"));
        assert!(out.contains("Initial probe"));
        assert!(!out.contains("Overall"));
        assert!(out.contains("MCP consent"));
        assert!(out.contains("Consent is unavailable"));
        assert!(!out.contains("Activation verdict"));
    }

    #[test]
    fn live_activity_hides_controls_until_the_surface_is_interactive() {
        let mut surface = ActivationSurface::live(false, false);
        surface.update_live_progress(
            vec!["Initial probe: started".to_string()],
            vec![super::super::ActivationProgressStep::new(
                "initial-probe",
                "Initial probe",
                ActivationProgressStatus::Running,
            )],
        );

        let out = render_to_string(&surface, 100, 24);

        assert!(out.contains("Activation activity"));
        assert!(out.contains("Initial probe: started"));
        assert!(!out.contains("Search:"));
        assert!(!out.contains("[j/k] scroll"));
    }

    #[test]
    fn live_tier_evidence_toggle_replaces_activity_panel() {
        let mut surface = ActivationSurface::live(false, false);
        surface.update_live_progress(
            vec!["Initial probe: started".to_string()],
            vec![super::super::ActivationProgressStep::new(
                "initial-probe",
                "Initial probe",
                ActivationProgressStatus::Running,
            )],
        );
        surface.toggle_tier_evidence();

        let out = render_to_string(&surface, 100, 24);

        assert!(out.contains("Tier evidence"));
        assert!(out.contains("Initial probe: started"));
        assert!(out.contains("[j/k] scroll"));
        assert!(!out.contains("Activation activity"));
    }

    #[test]
    fn renders_consent_phase_when_present() {
        let consent = crate::surfaces::activation::ConsentState::new(
            vec![crate::surfaces::activation::ConsentItem::new(
                "cursor",
                "Cursor MCP",
                "write config",
                crate::surfaces::activation::ConsentKind::Mcp,
            )],
            false,
        );
        let surface = ActivationSurface::from_verdict(
            "ACTIVATION
",
            false,
        )
        .with_consent(consent);
        let out = render_to_string(&surface, 100, 24);
        assert!(out.contains("Consent"));
        assert!(out.contains("Cursor MCP"));
        assert!(!out.contains("Activation verdict"));
    }

    #[test]
    fn consent_phase_without_model_renders_an_invariant_guard() {
        let surface = ActivationSurface::from_verdict_with_progress(
            "ACTIVATION\n  state: ready_restart_required\n",
            false,
            vec![],
            vec![],
            false,
            ActivationPhase::Consent,
        );

        let out = render_to_string(&surface, 100, 24);

        assert!(out.contains("Consent is unavailable"));
        assert!(!out.contains("Activation verdict"));
    }

    #[test]
    fn renders_log_panel_when_tier_evidence_is_toggled() {
        let mut surface = ActivationSurface::from_verdict(
            "ACTIVATION
  state: ready_restart_required
  mcp:
    Cursor: restart_handshake_verified (pending restart)
  install:
    Cursor: skipped — already up to date
",
            false,
        );
        surface.toggle_tier_evidence();

        let out = render_to_string(&surface, 110, 28);

        assert!(out.contains("Tier evidence"));
        assert!(out.contains("mcp/Cursor"));
        assert!(out.contains("restart_handshake_verified"));
        assert!(out.contains("install/Cursor"));
        assert!(!out.contains("Activation verdict"));
    }

    #[test]
    fn renders_gated_anvil_home_banner() {
        let surface = ActivationSurface::from_verdict("ACTIVATION\n  state: watching\n", true);
        let out = render_to_string(&surface, 100, 22);
        assert!(out.contains("ANVIL_HOME gated"));
        assert!(out.contains("Project writes are gated"));
    }

    #[test]
    fn snapshots_cover_every_activation_phase() {
        let verdict = "ACTIVATION\n  state: ready_restart_required\n  next: restart editor\n";

        let preflight = ActivationSurface::live(false, false);
        assert!(render_to_string(&preflight, 100, 22).contains("[Preflight]"));
        insta::assert_snapshot!(
            "activation_preflight",
            render_to_string(&preflight, 100, 22)
        );

        let working = ActivationSurface::from_verdict_with_progress(
            verdict,
            false,
            vec!["initial-probe: completed".to_string()],
            vec![super::super::ActivationProgressStep::new(
                "initial-probe",
                "Initial probe",
                ActivationProgressStatus::Passed,
            )],
            true,
            ActivationPhase::Working,
        );
        assert!(render_to_string(&working, 100, 24).contains("[Working]"));
        insta::assert_snapshot!("activation_working", render_to_string(&working, 100, 24));

        let consent = crate::surfaces::activation::ConsentState::new(
            vec![crate::surfaces::activation::ConsentItem::new(
                "mcp:cursor",
                "Cursor MCP",
                "Write ~/.cursor/mcp.json",
                crate::surfaces::activation::ConsentKind::Mcp,
            )],
            false,
        );
        let consent_surface = ActivationSurface::from_verdict(verdict, false).with_consent(consent);
        assert!(render_to_string(&consent_surface, 100, 22).contains("[Consent]"));
        insta::assert_snapshot!(
            "activation_consent",
            render_to_string(&consent_surface, 100, 22)
        );

        let verdict_surface = ActivationSurface::from_verdict(verdict, false);
        assert!(render_to_string(&verdict_surface, 100, 22).contains("[Verdict]"));
        insta::assert_snapshot!(
            "activation_verdict",
            render_to_string(&verdict_surface, 100, 22)
        );

        let mut done = ActivationSurface::from_verdict(verdict, false);
        done.phase = ActivationPhase::Done;
        assert!(render_to_string(&done, 100, 22).contains("[Done]"));
        insta::assert_snapshot!("activation_done", render_to_string(&done, 100, 22));
    }
}
