use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::{ActivationPhase, ActivationSurface};
use crate::shell::inset_content;

/// Render the ACTTUI-001 foundation surface.
pub fn render(frame: &mut Frame, area: Rect, state: &ActivationSurface, theme: &EddaCraftTheme) {
    let area = inset_content(area);
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(if state.project_writes_gated() { 3 } else { 0 }),
        Constraint::Min(6),
    ])
    .split(area);

    render_phase_strip(frame, chunks[0], state.phase(), theme);
    if state.project_writes_gated() {
        render_gated_banner(frame, chunks[1], theme);
    }
    render_verdict(frame, chunks[2], state.verdict(), theme);
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
            let separator = if idx < phases.len() - 1 { " > " } else { "" };
            vec![
                Span::styled(phase.label(), style),
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

fn render_verdict(frame: &mut Frame, area: Rect, verdict: &str, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Activation verdict ");
    let lines: Vec<Line> = verdict
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("state:") {
                Line::styled(
                    line.to_owned(),
                    Style::default()
                        .fg(theme.accent())
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Line::raw(line.to_owned())
            }
        })
        .collect();
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
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
        assert!(out.contains("state: protecting"));
        assert!(out.contains("Activation verdict"));
    }

    #[test]
    fn renders_gated_anvil_home_banner() {
        let surface = ActivationSurface::from_verdict("ACTIVATION\n  state: watching\n", true);
        let out = render_to_string(&surface, 100, 22);
        assert!(out.contains("ANVIL_HOME gated"));
        assert!(out.contains("Project writes are gated"));
    }
}
