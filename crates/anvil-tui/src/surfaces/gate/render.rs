use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GateCheckStatus, GateState};

pub fn render(frame: &mut Frame, area: Rect, state: &GateState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header with summary
        Constraint::Min(6),    // 2-panel content
    ])
    .split(area);

    render_header(frame, chunks[0], state, theme);
    render_panels(frame, chunks[1], state, theme);
}

fn status_colour(status: GateCheckStatus, theme: &EddaCraftTheme) -> ratatui::style::Color {
    match status {
        GateCheckStatus::Passed => theme.success(),
        GateCheckStatus::Failed => theme.error(),
        GateCheckStatus::Warning => theme.warning(),
        GateCheckStatus::Skipped => theme.muted(),
    }
}

fn render_header(frame: &mut Frame, area: Rect, state: &GateState, theme: &EddaCraftTheme) {
    let summary = state.summary();
    let overall_icon = if state.result.overall_passed {
        "*"
    } else {
        "x"
    };
    let overall_colour = if state.result.overall_passed {
        theme.success()
    } else {
        theme.error()
    };

    let line1 = Line::from(vec![
        Span::styled(
            format!("{overall_icon} "),
            Style::default().fg(overall_colour),
        ),
        Span::styled(
            "Gate ",
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}  ", state.result.plan_id),
            Style::default().fg(theme.accent()),
        ),
        Span::styled(
            format!("Score: {:.0}%  ", state.result.score * 100.0),
            Style::default().fg(theme.fg()),
        ),
        Span::styled(
            format!("{}ms", state.result.duration_ms),
            Style::default().fg(theme.muted()),
        ),
    ]);

    let line2 = Line::from(vec![
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
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("{} skipped", summary.skipped),
            Style::default().fg(theme.muted()),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("filter: {}", state.filter.label()),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    frame.render_widget(Paragraph::new(Text::from(vec![line1, line2])), area);
}

fn render_panels(frame: &mut Frame, area: Rect, state: &GateState, theme: &EddaCraftTheme) {
    let columns = Layout::horizontal([
        Constraint::Percentage(50), // Check tree (left)
        Constraint::Percentage(50), // Detail panel (right)
    ])
    .split(area);

    render_check_tree(frame, columns[0], state, theme);
    render_detail_panel(frame, columns[1], state, theme);
}

fn render_check_tree(frame: &mut Frame, area: Rect, state: &GateState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Checks ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let filtered = state.filtered_checks();

    if filtered.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No checks match the current filter",
            Style::default().fg(theme.muted()),
        )));
        frame.render_widget(empty, inner);
        return;
    }

    let items: Vec<Line> = filtered
        .iter()
        .enumerate()
        .map(|(i, (_, check))| {
            let selected = i == state.selected;
            let indicator = if selected { ">> " } else { "  " };
            let icon_colour = status_colour(check.status, theme);
            let name_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            let expand_marker = if selected && state.expanded {
                " [-]"
            } else if check.details.is_some() {
                " [+]"
            } else {
                ""
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(
                    format!("{} ", check.status.icon()),
                    Style::default().fg(icon_colour),
                ),
                Span::styled(&check.name, name_style),
                Span::styled(
                    format!("  {:.0}%", check.score * 100.0),
                    Style::default().fg(theme.muted()),
                ),
                Span::styled(expand_marker, Style::default().fg(theme.muted())),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_detail_panel(frame: &mut Frame, area: Rect, state: &GateState, theme: &EddaCraftTheme) {
    let Some(check) = state.selected_check() else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.muted()))
            .title(" Detail ");
        frame.render_widget(block, area);
        return;
    };

    let icon_colour = status_colour(check.status, theme);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(format!(" {} ", check.name));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Status:  ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{} {}", check.status.icon(), check.status.label()),
                Style::default()
                    .fg(icon_colour)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Score:   ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{:.0}%", check.score * 100.0),
                Style::default().fg(theme.fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("ID:      ", Style::default().fg(theme.muted())),
            Span::styled(&check.id, Style::default().fg(theme.fg())),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled("Message: ", Style::default().fg(theme.muted())),
            Span::styled(&check.message, Style::default().fg(theme.fg())),
        ]),
    ];

    if let Some(file) = &check.file {
        let location = if let Some(line) = check.line {
            format!("{file}:{line}")
        } else {
            file.clone()
        };
        lines.push(Line::from(vec![
            Span::styled("File:    ", Style::default().fg(theme.muted())),
            Span::styled(location, Style::default().fg(theme.accent())),
        ]));
    }

    if let Some(details) = &check.details {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "Details:",
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        )));
        for detail_line in details.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {detail_line}"),
                Style::default().fg(theme.fg()),
            )));
        }
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}
