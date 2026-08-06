use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{AuditPanel, AuditState, IssueSeverity};

/// Compute scroll offset to keep `selected` visible within `visible_rows`.
fn viewport_scroll(selected: usize, total: usize, visible_rows: usize) -> usize {
    if total <= visible_rows || visible_rows == 0 {
        return 0;
    }
    let max_offset = total.saturating_sub(visible_rows);
    selected
        .saturating_sub(visible_rows.saturating_sub(1))
        .min(max_offset)
}

pub fn render(frame: &mut Frame, area: Rect, state: &AuditState, theme: &EddaCraftTheme) {
    // Zoom mode: render only the focused panel filling the area. Same
    // affordance as the watch surface — `z` toggles, `esc` exits zoom
    // before going back.
    if state.zoomed {
        match state.focused_panel {
            AuditPanel::Project => render_project_panel(frame, area, state, theme),
            AuditPanel::Issues => render_issues_panel(frame, area, state, theme),
            AuditPanel::Historical => render_historical_panel(frame, area, state, theme),
            AuditPanel::NextSteps => render_next_steps_panel(frame, area, state, theme),
        }
        return;
    }

    let panel_constraints = if state.focused_panel == AuditPanel::Issues && state.expanded {
        [
            Constraint::Length(10), // Project panel, including the wrapped security scope
            Constraint::Length(7),  // Selected issue plus all expanded detail lines
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2),
        ]
    } else {
        [
            Constraint::Length(10),  // Project panel, including the wrapped security scope
            Constraint::Ratio(1, 3), // Issues panel
            Constraint::Ratio(1, 3), // Historical panel
            Constraint::Ratio(1, 3), // Next steps panel
        ]
    };
    let chunks = Layout::vertical(panel_constraints).split(area);

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

fn truncate_with_ellipsis(text: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(text) <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }

    let target_width = max_width.saturating_sub(UnicodeWidthStr::width("…"));
    let mut truncated = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + char_width > target_width {
            break;
        }
        truncated.push(ch);
        width += char_width;
    }
    truncated.push('…');
    truncated
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

    let file_count = state.data.total_files.to_string();
    let fixed_summary_width = UnicodeWidthStr::width("Project:  ")
        + UnicodeWidthStr::width("  Files: ")
        + UnicodeWidthStr::width(file_count.as_str());
    let project_name = truncate_with_ellipsis(
        &state.data.project_name,
        usize::from(inner.width).saturating_sub(fixed_summary_width),
    );

    let lines = vec![
        Line::from(vec![
            Span::styled("Project:  ", Style::default().fg(theme.muted())),
            Span::styled(
                project_name,
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Files: ", Style::default().fg(theme.muted())),
            Span::styled(file_count, Style::default().fg(theme.fg())),
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
        Line::from(vec![
            Span::styled("Security scope: ", Style::default().fg(theme.muted())),
            Span::styled(&state.data.security_scope, Style::default().fg(theme.fg())),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
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
        let has_fix = issue.is_fixable();
        let fixable_marker = if has_fix { " [fix]" } else { "" };

        lines.push(Line::from(vec![
            Span::styled(indicator, name_style),
            Span::styled(
                format!("{} ", issue.severity.label()),
                Style::default().fg(sev_colour),
            ),
            Span::styled(&issue.message, name_style),
            Span::styled(fixable_marker, Style::default().fg(theme.accent())),
            Span::styled(
                format!("  {}", issue.location()),
                Style::default().fg(theme.muted()),
            ),
        ]));

        // Inline expanded details right after the selected item.
        if state.expanded && selected {
            lines.push(Line::from(vec![
                Span::styled("    Category: ", Style::default().fg(theme.muted())),
                Span::styled(&issue.category, Style::default().fg(theme.fg())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("    Severity: ", Style::default().fg(theme.muted())),
                Span::styled(issue.severity.label_full(), Style::default().fg(sev_colour)),
            ]));
            if has_fix {
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

    const TEST_SECURITY_SCOPE: &str = "audit covers secrets in source files and configuration, \
        reporting one entry per match. The broader gate also checks architecture, policy, \
        dependencies and lint. Counts therefore describe different surfaces and must not be \
        compared as equivalent safety verdicts. A quiet audit is only a quiet audit; it does not \
        prove the broader gate would pass. The overview and gate remain different decisions for \
        operators reviewing the same project.";

    fn sample_data() -> super::super::AuditData {
        super::super::AuditData {
            project_name: "test-project".to_string(),
            total_files: 42,
            security_scope: TEST_SECURITY_SCOPE.to_string(),
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
                    category: "Quality".to_string(),
                    message: "console statement found".to_string(),
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
                "Remove console statements".to_string(),
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
    fn expanded_issue_details_remain_visible_at_80x24() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = super::super::AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        state.selected_item = 1;
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

        let rendered = buffer_contents(terminal.backend().buffer());
        assert!(rendered.contains("Severity: Medium"), "{rendered}");
        assert!(
            rendered.contains("Auto-fixable: press 'f' to fix"),
            "{rendered}"
        );
    }

    #[test]
    fn project_panel_renders_security_scope_with_findings() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = super::super::AuditState::new(sample_data());
        assert!(
            !state.data.issues.is_empty(),
            "fixture must contain findings"
        );
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered_with_borders: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        let rendered: String = rendered_with_borders
            .chars()
            .map(|ch| {
                if "┌┐└┘─│".contains(ch) {
                    ' '
                } else {
                    ch
                }
            })
            .collect();
        let normalise = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalise(&rendered).contains(&normalise(TEST_SECURITY_SCOPE)),
            "{rendered}"
        );
    }

    #[test]
    fn project_panel_preserves_security_scope_with_long_project_name() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut data = sample_data();
        data.project_name = "project-name-that-is-deliberately-long-".repeat(4);
        let state = super::super::AuditState::new(data);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered_with_borders: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        let rendered: String = rendered_with_borders
            .chars()
            .map(|ch| {
                if "┌┐└┘─│".contains(ch) {
                    ' '
                } else {
                    ch
                }
            })
            .collect();
        let normalise = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalise(&rendered).contains(&normalise(TEST_SECURITY_SCOPE)),
            "{rendered}"
        );
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

    /// Exercises the scroll-clamping logic with a list taller than the
    /// viewport and expansion near the end.
    #[test]
    fn snapshot_expanded_scroll_many_issues() {
        let issues: Vec<super::super::AuditIssue> = (0..20)
            .map(|i| super::super::AuditIssue {
                severity: if i % 4 == 0 {
                    IssueSeverity::Critical
                } else {
                    IssueSeverity::Low
                },
                category: "Test".to_string(),
                message: if i % 3 == 0 {
                    "console statement found".to_string()
                } else {
                    format!("Issue {i}")
                },
                file: format!("src/file{i}.ts"),
                line: i + 1,
                fixable: i % 3 == 0,
            })
            .collect();

        let data = super::super::AuditData {
            project_name: "big-project".to_string(),
            total_files: 100,
            security_scope: TEST_SECURITY_SCOPE.to_string(),
            issues,
            historical_scores: vec![],
            next_steps: vec![],
        };

        let backend = TestBackend::new(80, 48);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = super::super::AuditState::new(data);
        state.focused_panel = AuditPanel::Issues;
        state.selected_item = 18; // near the end
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

    fn buffer_contents(buf: &ratatui::buffer::Buffer) -> String {
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn audit_zoomed_render_shows_only_focused_panel() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = super::super::AuditState::new(sample_data());
        state.focused_panel = AuditPanel::Issues;
        state.zoomed = true;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        assert!(
            rendered.contains("Issues"),
            "zoomed render must show focused (Issues) panel; got:\n{rendered}"
        );
        for hidden in ["Project", "Historical", "Next"] {
            assert!(
                !rendered.contains(hidden),
                "zoomed render must hide non-focused panel `{hidden}`"
            );
        }
    }

    // ── viewport_scroll unit tests ──────────────────────────────────

    #[test]
    fn viewport_scroll_zero_visible_rows() {
        assert_eq!(viewport_scroll(5, 10, 0), 0);
    }

    #[test]
    fn viewport_scroll_all_items_fit() {
        assert_eq!(viewport_scroll(3, 5, 10), 0);
        assert_eq!(viewport_scroll(0, 5, 5), 0);
    }

    #[test]
    fn viewport_scroll_near_end() {
        // 20 items, 5 visible — selecting item 18 should scroll but not
        // past max_offset (15).
        let offset = viewport_scroll(18, 20, 5);
        assert!(offset <= 15, "offset {offset} exceeds max 15");
        assert!(
            offset + 5 > 18,
            "selected item 18 not visible at offset {offset}"
        );
    }

    #[test]
    fn viewport_scroll_single_visible_row() {
        // With only 1 visible row the selected item IS the scroll offset,
        // clamped to max_offset.
        assert_eq!(viewport_scroll(3, 10, 1), 3);
        assert_eq!(viewport_scroll(9, 10, 1), 9);
    }

    #[test]
    fn viewport_scroll_never_exceeds_max_offset() {
        for selected in 0..25 {
            let offset = viewport_scroll(selected, 20, 5);
            assert!(
                offset <= 15,
                "selected={selected} offset={offset} exceeds max 15"
            );
        }
    }
}
