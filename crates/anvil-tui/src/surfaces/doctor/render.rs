use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{CheckStatus, DoctorState};

#[allow(clippy::too_many_lines)]
pub fn render(frame: &mut Frame, area: Rect, state: &DoctorState, theme: &EddaCraftTheme) {
    let detail_height = if state.expanded { 4 } else { 0 };
    let chunks = Layout::vertical([
        Constraint::Length(3),            // Summary header
        Constraint::Min(4),               // Check list
        Constraint::Length(detail_height), // Detail panel (when expanded)
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
    if state.expanded
        && let Some(check) = state.checks.get(state.selected)
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_state() -> DoctorState {
        use super::super::DiagnosticCheck;

        DoctorState::new(vec![
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
                details: Some("Run `npx eslint --init`".to_string()),
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
        ])
    }

    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = sample_state();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn snapshot_default_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = sample_state();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    "Doctor",
                    "j/k navigate  enter expand  q quit",
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_expanded() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.expanded = true;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    "Doctor",
                    "j/k navigate  enter expand  q quit",
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
        let state = sample_state();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}
