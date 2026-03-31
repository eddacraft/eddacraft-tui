use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{AuditPanel, AuditState, IssueSeverity};

/// Compute scroll offset to keep `selected` visible within `visible_rows`.
fn viewport_scroll(selected: usize, total: usize, visible_rows: usize) -> usize {
    if total <= visible_rows || visible_rows == 0 {
        return 0;
    }
    if selected < visible_rows {
        0
    } else {
        selected - visible_rows + 1
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &AuditState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Ratio(1, 4), // Project panel
        Constraint::Ratio(1, 4), // Issues panel
        Constraint::Ratio(1, 4), // Historical panel
        Constraint::Ratio(1, 4), // Next steps panel
    ])
    .split(area);

    render_project_panel(frame, chunks[0], state, theme);
    render_issues_panel(frame, chunks[1], state, theme);
    render_historical_panel(frame, chunks[2], state, theme);
    render_next_steps_panel(frame, chunks[3], state, theme);
}

fn panel_block<'a>(title: &'a str, focused: bool, theme: &EddaCraftTheme) -> Block<'a> {
    let border_colour = if focused {
        theme.accent()
    } else {
        theme.muted()
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_colour))
        .title(format!(" {title} "))
        .title_style(if focused {
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted())
        })
}

fn severity_colour(severity: IssueSeverity, theme: &EddaCraftTheme) -> ratatui::style::Color {
    match severity {
        IssueSeverity::Critical | IssueSeverity::High => theme.error(),
        IssueSeverity::Medium => theme.warning(),
        IssueSeverity::Low => theme.muted(),
        IssueSeverity::Info => theme.accent(),
    }
}

fn render_project_panel(frame: &mut Frame, area: Rect, state: &AuditState, theme: &EddaCraftTheme) {
    let focused = state.focused_panel == AuditPanel::Project;
    let block = panel_block("Project", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let critical = state.data.issue_count_by_severity(IssueSeverity::Critical);
    let high = state.data.issue_count_by_severity(IssueSeverity::High);
    let medium = state.data.issue_count_by_severity(IssueSeverity::Medium);
    let low = state.data.issue_count_by_severity(IssueSeverity::Low);

    let lines = vec![
        Line::from(vec![
            Span::styled("Project:  ", Style::default().fg(theme.muted())),
            Span::styled(
                &state.data.project_name,
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Files:    ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}", state.data.total_files),
                Style::default().fg(theme.fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("Issues:   ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{critical} critical"),
                Style::default().fg(theme.error()),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(format!("{high} high"), Style::default().fg(theme.error())),
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{medium} medium"),
                Style::default().fg(theme.warning()),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(format!("{low} low"), Style::default().fg(theme.muted())),
        ]),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_issues_panel(frame: &mut Frame, area: Rect, state: &AuditState, theme: &EddaCraftTheme) {
    let focused = state.focused_panel == AuditPanel::Issues;
    let title = if !focused || state.data.issues.is_empty() {
        format!("Current Issues ({})", state.data.issues.len())
    } else {
        format!(
            "Current Issues ({}/{})",
            state.selected_item + 1,
            state.data.issues.len()
        )
    };
    let block = panel_block(&title, focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_rows = inner.height as usize;
    let mut lines: Vec<Line> = Vec::new();
    let mut selected_line_idx: usize = 0;

    for (i, issue) in state.data.issues.iter().enumerate() {
        let selected = focused && i == state.selected_item;
        if selected {
            selected_line_idx = lines.len();
        }

        let indicator = if selected { ">> " } else { "  " };
        let sev_colour = severity_colour(issue.severity, theme);
        let name_style = if selected {
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg())
        };
        let fixable_marker = if issue.fixable { " [fix]" } else { "" };

        lines.push(Line::from(vec![
            Span::styled(indicator, name_style),
            Span::styled(
                format!("{} ", issue.severity.label()),
                Style::default().fg(sev_colour),
            ),
            Span::styled(&issue.message, name_style),
            Span::styled(fixable_marker, Style::default().fg(theme.accent())),
            Span::styled(
                format!("  {}:{}", issue.file, issue.line),
                Style::default().fg(theme.muted()),
            ),
        ]));

        // Inline expanded details right after the selected item.
        // TODO: when the panel is very short, detail lines may be clipped with
        // no scroll affordance. A dedicated expansion_scroll offset on
        // AuditState would fix this but is out of scope for RCLI-027.
        if state.expanded && selected {
            lines.push(Line::from(vec![
                Span::styled("    Category: ", Style::default().fg(theme.muted())),
                Span::styled(&issue.category, Style::default().fg(theme.fg())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("    Severity: ", Style::default().fg(theme.muted())),
                Span::styled(
                    issue.severity.label_full(),
                    Style::default().fg(sev_colour),
                ),
            ]));
            if issue.fixable {
                lines.push(Line::from(Span::styled(
                    "    Auto-fixable: press 'f' to fix",
                    Style::default().fg(theme.accent()),
                )));
            }
            lines.push(Line::default());
        }
    }

    // When expanded, scroll to place the selected item at the top so detail
    // lines are visible below it, clamped to max_offset so we never scroll
    // past the last line. Otherwise use standard viewport scroll.
    let scroll_offset = if focused {
        if state.expanded {
            let max_offset = lines.len().saturating_sub(visible_rows);
            selected_line_idx.min(max_offset)
        } else {
            viewport_scroll(selected_line_idx, lines.len(), visible_rows)
        }
    } else {
        0
    };

    #[allow(clippy::cast_possible_truncation)]
    let para = Paragraph::new(Text::from(lines)).scroll((scroll_offset as u16, 0));
    frame.render_widget(para, inner);
}

fn render_historical_panel(
    frame: &mut Frame,
    area: Rect,
    state: &AuditState,
    theme: &EddaCraftTheme,
) {
    let focused = state.focused_panel == AuditPanel::Historical;
    let block = panel_block("Historical", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_rows = inner.height as usize;
    let scroll_offset = if focused {
        viewport_scroll(
            state.selected_item,
            state.data.historical_scores.len(),
            visible_rows,
        )
    } else {
        0
    };

    let lines: Vec<Line> = state
        .data
        .historical_scores
        .iter()
        .enumerate()
        .map(|(i, score)| {
            let selected = focused && i == state.selected_item;
            let indicator = if selected { ">> " } else { "  " };
            let name_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };
            let score_colour = if score.score >= 0.9 {
                theme.success()
            } else if score.score >= 0.7 {
                theme.warning()
            } else {
                theme.error()
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(&score.timestamp, name_style),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{:.0}%", score.score * 100.0),
                    Style::default().fg(score_colour),
                ),
                Span::styled(
                    format!("  {} issues", score.issue_count),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    #[allow(clippy::cast_possible_truncation)]
    let para = Paragraph::new(Text::from(lines)).scroll((scroll_offset as u16, 0));
    frame.render_widget(para, inner);
}

fn render_next_steps_panel(
    frame: &mut Frame,
    area: Rect,
    state: &AuditState,
    theme: &EddaCraftTheme,
) {
    let focused = state.focused_panel == AuditPanel::NextSteps;
    let block = panel_block("Next Steps", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_rows = inner.height as usize;
    let scroll_offset = if focused {
        viewport_scroll(
            state.selected_item,
            state.data.next_steps.len(),
            visible_rows,
        )
    } else {
        0
    };

    let lines: Vec<Line> = state
        .data
        .next_steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let selected = focused && i == state.selected_item;
            let indicator = if selected { ">> " } else { "  " };
            let name_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(format!("{}. ", i + 1), Style::default().fg(theme.muted())),
                Span::styled(step.as_str(), name_style),
            ])
        })
        .collect();

    #[allow(clippy::cast_possible_truncation)]
    let para = Paragraph::new(Text::from(lines)).scroll((scroll_offset as u16, 0));
    frame.render_widget(para, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_data() -> super::super::AuditData {
        super::super::AuditData {
            project_name: "test-project".to_string(),
            total_files: 42,
            issues: vec![
                super::super::AuditIssue {
                    severity: IssueSeverity::Critical,
                    category: "Security".to_string(),
                    message: "Hardcoded API key".to_string(),
                    file: "src/config.ts".to_string(),
                    line: 15,
                    fixable: false,
                },
                super::super::AuditIssue {
                    severity: IssueSeverity::Medium,
                    category: "Architecture".to_string(),
                    message: "Cross-boundary import".to_string(),
                    file: "src/utils/db.ts".to_string(),
                    line: 3,
                    fixable: true,
                },
            ],
            historical_scores: vec![super::super::HistoricalScore {
                timestamp: "2026-03-15".to_string(),
                score: 0.85,
                issue_count: 5,
            }],
            next_steps: vec![
                "Fix critical security issue".to_string(),
                "Review boundary violations".to_string(),
            ],
        }
    }

    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = super::super::AuditState::new(sample_data());
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn snapshot_default_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = super::super::AuditState::new(sample_data());
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
    fn snapshot_issues_focused() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = super::super::AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
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
    fn snapshot_issues_expanded() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = super::super::AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        state.expanded = true;
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
    fn snapshot_last_item_expanded() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = super::super::AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        state.selected_item = state.data.issues.len() - 1;
        state.expanded = true;
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
        let state = super::super::AuditState::new(sample_data());
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}
