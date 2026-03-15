use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{AuditPanel, AuditState, IssueSeverity};

pub fn render(frame: &mut Frame, area: Rect, state: &AuditState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(1),   // Title
        Constraint::Ratio(1, 4), // Project panel
        Constraint::Ratio(1, 4), // Issues panel
        Constraint::Ratio(1, 4), // Historical panel
        Constraint::Ratio(1, 4), // Next steps panel
        Constraint::Length(2),   // Help text
    ])
    .split(area);

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        "Audit Results",
        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(title, chunks[0]);

    render_project_panel(frame, chunks[1], state, theme);
    render_issues_panel(frame, chunks[2], state, theme);
    render_historical_panel(frame, chunks[3], state, theme);
    render_next_steps_panel(frame, chunks[4], state, theme);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(theme.accent())),
        Span::styled(" navigate  ", Style::default().fg(theme.muted())),
        Span::styled("h/l", Style::default().fg(theme.accent())),
        Span::styled(" switch panel  ", Style::default().fg(theme.muted())),
        Span::styled("enter", Style::default().fg(theme.accent())),
        Span::styled(" details  ", Style::default().fg(theme.muted())),
        Span::styled("q", Style::default().fg(theme.accent())),
        Span::styled(" quit", Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(help, chunks[5]);
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
    let block = panel_block("Current Issues", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = state
        .data
        .issues
        .iter()
        .enumerate()
        .map(|(i, issue)| {
            let selected = focused && i == state.selected_item;
            let indicator = if selected { ">> " } else { "  " };
            let sev_colour = severity_colour(issue.severity, theme);
            let name_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };
            let fixable_marker = if issue.fixable { " [fix]" } else { "" };

            Line::from(vec![
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
            ])
        })
        .collect();

    // Show expanded details if applicable
    if state.expanded && focused {
        if let Some(issue) = state.data.issues.get(state.selected_item) {
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled("    Category: ", Style::default().fg(theme.muted())),
                Span::styled(&issue.category, Style::default().fg(theme.fg())),
            ]));
            if issue.fixable {
                lines.push(Line::from(Span::styled(
                    "    Auto-fixable: press 'f' to fix",
                    Style::default().fg(theme.accent()),
                )));
            }
        }
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
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

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
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

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}
