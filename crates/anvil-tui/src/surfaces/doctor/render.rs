use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{CheckStatus, DoctorState};

#[allow(clippy::too_many_lines)]
pub fn render(frame: &mut Frame, area: Rect, state: &DoctorState, theme: &EddaCraftTheme) {
    // Grow the detail panel when expanded so the remediation block has
    // room (summary + optional command + optional doc_url + fix hint
    // can run to ~5 lines on top of the technical details line).
    // Bound by the available area so the check list always keeps at
    // least 4 rows on tiny terminals.
    let detail_height = if state.expanded {
        detail_panel_height(state, area)
    } else {
        0
    };
    let chunks = Layout::vertical([
        Constraint::Length(3),             // Summary header
        Constraint::Min(4),                // Check list
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

        let r = &check.remediation;
        if !r.summary.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("\u{2192} {}", r.summary),
                Style::default().fg(theme.fg()),
            )));
        }
        if let Some(cmd) = &r.command {
            lines.push(Line::from(vec![
                Span::styled("  run:  ", Style::default().fg(theme.muted())),
                Span::styled(cmd.clone(), Style::default().fg(theme.accent())),
            ]));
        }
        if let Some(url) = &r.doc_url {
            lines.push(Line::from(vec![
                Span::styled("  docs: ", Style::default().fg(theme.muted())),
                Span::styled(url.clone(), Style::default().fg(theme.accent())),
            ]));
        }
        if check.auto_fixable {
            lines.push(Line::from(Span::styled(
                "Auto-fixable — press `f` (or `anvil doctor --fix`)",
                Style::default().fg(theme.accent()),
            )));
        }
        frame.render_widget(Paragraph::new(Text::from(lines)), detail_area);
    }
}

/// Maximum lines we will allocate to the detail panel before yielding
/// space back to the check list. 10 is enough for the worst-case
/// remediation (1 details + `summary` + `command` + `doc_url` + fix
/// hint = 5 content lines + 2 borders + 3 headroom for chrome).
const MAX_DETAIL_PANEL_HEIGHT: u16 = 10;

/// Lines reserved above the detail panel so the check list never
/// collapses: 3 for the summary header + 4 minimum list height.
const DETAIL_PANEL_RESERVED_ROWS: u16 = 7;

/// Minimum height we will allocate to the detail panel (matches the
/// pre-LAUNCH-005 hard-coded value so existing snapshots stay valid).
const MIN_DETAIL_PANEL_HEIGHT: u16 = 4;

/// Height needed for the detail panel: 1 line for `details`, optional
/// lines for `summary` / `command` / `doc_url` / fix hint, plus the
/// 2-line border. Bounded by the available area so the check list
/// always keeps at least `DETAIL_PANEL_RESERVED_ROWS` rows even on
/// small terminals.
///
/// Note: ratatui's `Paragraph` is constructed without `.wrap(...)` in
/// this surface, so each rendered line is **truncated horizontally**
/// at the panel's right edge — there is no vertical line wrapping and
/// no scroll. A `summary` longer than `area.width - 2` (the panel's
/// inner width) will be cut off, but it will not push subsequent
/// lines (command, doc_url, fix hint) off-screen. Per-check
/// remediations in `commands/doctor.rs` aim to fit one visual line at
/// typical widths (~78 chars at 80 columns); the `config-valid`
/// parse-error path is the known exception and accepts horizontal
/// truncation on narrow terminals.
#[allow(clippy::doc_markdown)] // "command, doc_url, fix hint" reads as prose, not symbols.
fn detail_panel_height(state: &DoctorState, area: Rect) -> u16 {
    let Some(check) = state.checks.get(state.selected) else {
        return MIN_DETAIL_PANEL_HEIGHT;
    };
    let mut content_lines: u16 = 1; // details line is always rendered
    let r = &check.remediation;
    if !r.summary.is_empty() {
        content_lines += 1;
    }
    if r.command.is_some() {
        content_lines += 1;
    }
    if r.doc_url.is_some() {
        content_lines += 1;
    }
    if check.auto_fixable {
        content_lines += 1;
    }
    let area_cap = area
        .height
        .saturating_sub(DETAIL_PANEL_RESERVED_ROWS)
        .max(MIN_DETAIL_PANEL_HEIGHT);
    (content_lines + 2).clamp(
        MIN_DETAIL_PANEL_HEIGHT,
        area_cap.min(MAX_DETAIL_PANEL_HEIGHT),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_state() -> DoctorState {
        use super::super::DiagnosticCheck;

        use super::super::Remediation;

        DoctorState::new(vec![
            DiagnosticCheck {
                name: "Node.js".to_string(),
                category: "Runtime".to_string(),
                status: CheckStatus::Pass,
                message: "v22.0.0 found".to_string(),
                details: Some("Path: /usr/bin/node".to_string()),
                auto_fixable: false,
                remediation: Remediation::default(),
            },
            DiagnosticCheck {
                name: "ESLint config".to_string(),
                category: "Linting".to_string(),
                status: CheckStatus::Fail,
                message: "No .eslintrc found".to_string(),
                details: Some("Run `npx eslint --init`".to_string()),
                auto_fixable: true,
                remediation: Remediation {
                    summary: "Create an ESLint config".to_string(),
                    command: Some("npx eslint --init".to_string()),
                    doc_url: None,
                },
            },
            DiagnosticCheck {
                name: "Git hooks".to_string(),
                category: "Hooks".to_string(),
                status: CheckStatus::Warn,
                message: "Hooks not installed".to_string(),
                details: None,
                auto_fixable: true,
                remediation: Remediation {
                    summary: "Install pre-commit hooks".to_string(),
                    command: Some("npx husky init".to_string()),
                    doc_url: None,
                },
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

    /// LAUNCH-005: snapshot the expanded state with the cursor on a
    /// Fail check so the new remediation rendering path (summary +
    /// `run:` + fix hint) is pixel-locked. The pre-existing
    /// `snapshot_expanded` selects the first check (Pass / empty
    /// remediation), so it does not exercise the new code.
    #[test]
    fn snapshot_expanded_with_remediation() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        // Index 1 = ESLint config = Fail with command + auto_fixable.
        state.selected = 1;
        state.expanded = true;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    "Doctor",
                    "j/k navigate  enter expand  f fix  q quit",
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
