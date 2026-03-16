use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{CheckStatus, DoctorState};

#[allow(clippy::too_many_lines)]
pub fn render(frame: &mut Frame, area: Rect, state: &DoctorState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header + summary
        Constraint::Min(4),    // Check list
        Constraint::Length(4), // Detail panel (when expanded)
        Constraint::Length(2), // Help text
    ])
    .split(area);

    // Summary header
    let summary = state.summary();
    let summary_line = Line::from(vec![
        Span::styled(
            "Diagnostics  ",
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} passed", summary.passed),
            Style::default().fg(theme.success()),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("{} failed", summary.failed),
            Style::default().fg(theme.error()),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("{} warnings", summary.warnings),
            Style::default().fg(theme.warning()),
        ),
    ]);
    frame.render_widget(Paragraph::new(summary_line), chunks[0]);

    // Check list
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.muted()))
        .title(" Checks ");

    let list_area = list_block.inner(chunks[1]);
    frame.render_widget(list_block, chunks[1]);

    let items: Vec<Line> = state
        .checks
        .iter()
        .enumerate()
        .map(|(i, check)| {
            let selected = i == state.selected;
            let indicator = if selected { ">> " } else { "  " };
            let icon_colour = match check.status {
                CheckStatus::Pass => theme.success(),
                CheckStatus::Fail => theme.error(),
                CheckStatus::Warn => theme.warning(),
                CheckStatus::Skipped => theme.muted(),
                CheckStatus::Running => theme.accent(),
            };
            let name_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(
                    format!("{} ", check.status.icon()),
                    Style::default().fg(icon_colour),
                ),
                Span::styled(&check.name, name_style),
                Span::styled(
                    format!("  [{}]", check.category),
                    Style::default().fg(theme.muted()),
                ),
                Span::styled(
                    format!("  {}", check.message),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), list_area);

    // Detail panel (when expanded)
    if state.expanded {
        if let Some(check) = state.checks.get(state.selected) {
            let detail_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent()))
                .title(format!(" {} ", check.name));

            let detail_area = detail_block.inner(chunks[2]);
            frame.render_widget(detail_block, chunks[2]);

            let detail_text = check.details.as_deref().unwrap_or("No additional details");
            let mut lines = vec![Line::from(Span::styled(
                detail_text,
                Style::default().fg(theme.fg()),
            ))];
            if check.auto_fixable {
                lines.push(Line::from(Span::styled(
                    "Auto-fixable",
                    Style::default().fg(theme.accent()),
                )));
            }
            frame.render_widget(Paragraph::new(Text::from(lines)), detail_area);
        }
    }

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(theme.accent())),
        Span::styled(" navigate  ", Style::default().fg(theme.muted())),
        Span::styled("enter", Style::default().fg(theme.accent())),
        Span::styled(" details  ", Style::default().fg(theme.muted())),
        Span::styled("q", Style::default().fg(theme.accent())),
        Span::styled(" quit", Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(help, chunks[3]);
}
