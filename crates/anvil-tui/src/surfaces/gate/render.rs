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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_result() -> super::super::GateResult {
        use super::super::GateCheck;

        super::super::GateResult {
            plan_id: "default".to_string(),
            overall_passed: false,
            score: 0.45,
            checks: vec![
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
                    message: "API key detected".to_string(),
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
            ],
            duration_ms: 3200,
            timestamp: "2026-03-16T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = GateState::new(sample_result());
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn snapshot_default_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = GateState::new(sample_result());
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    Surface::surface_name(&state),
                    Surface::help_text(&state),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_with_filter() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = GateState::new(sample_result());
        state.filter = super::super::FilterStatus::Failed;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    Surface::surface_name(&state),
                    Surface::help_text(&state),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn renders_in_small_area() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = GateState::new(sample_result());
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}
