use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{StatusPanel, StatusState};

pub fn render(frame: &mut Frame, area: Rect, state: &StatusState, theme: &EddaCraftTheme) {
    // DISTRIB-002 + INSIGHTS-004: reserve a 1-line strip at the bottom
    // for the update hint or the first-week insights nudge (insights
    // takes precedence when both would apply; rare). The strip is
    // omitted when neither applies.
    let (body_area, hint_area) =
        if state.data.insights_hint.is_some() || state.data.update_hint.is_some() {
            let s = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
            (s[0], Some(s[1]))
        } else {
            (area, None)
        };

    // Zoom mode: only the focused panel renders, taking the full area.
    // Press `z` to toggle, `esc` exits zoom before navigating back.
    if state.zoomed {
        match state.focused_panel {
            StatusPanel::Hooks => render_hooks_panel(frame, body_area, state, theme),
            StatusPanel::Profile => render_profile_panel(frame, body_area, state, theme),
            StatusPanel::Results => render_results_panel(frame, body_area, state, theme),
        }
    } else {
        let chunks = Layout::vertical([
            Constraint::Ratio(1, 3), // Hooks panel
            Constraint::Ratio(1, 3), // Profile panel
            Constraint::Ratio(1, 3), // Results panel
        ])
        .split(body_area);

        render_hooks_panel(frame, chunks[0], state, theme);
        render_profile_panel(frame, chunks[1], state, theme);
        render_results_panel(frame, chunks[2], state, theme);
    }

    if let (Some(strip), Some(hint)) = (hint_area, state.data.insights_hint.as_ref()) {
        // INSIGHTS-004 nudge: muted, low-noise adoption signal.
        let para = Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default()
                .fg(theme.muted())
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(para, strip);
    } else if let (Some(strip), Some(hint)) = (hint_area, state.data.update_hint.as_ref()) {
        let colour = if hint.advisory_ids.is_empty() {
            theme.accent()
        } else {
            theme.error()
        };
        let para = Paragraph::new(Line::from(Span::styled(
            hint.render_line(),
            Style::default().fg(colour).add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(para, strip);
    }
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

fn render_hooks_panel(frame: &mut Frame, area: Rect, state: &StatusState, theme: &EddaCraftTheme) {
    let focused = state.focused_panel == StatusPanel::Hooks;
    let block = panel_block("Hooks", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = state
        .data
        .hooks
        .iter()
        .enumerate()
        .map(|(i, hook)| {
            let selected = focused && i == state.selected_item;
            let indicator = if selected { ">> " } else { "  " };
            let status_icon = if hook.active { "*" } else { "o" };
            let status_colour = if hook.active {
                theme.success()
            } else {
                theme.muted()
            };

            Line::from(vec![
                Span::styled(indicator, Style::default().fg(theme.fg())),
                Span::styled(
                    format!("{status_icon} "),
                    Style::default().fg(status_colour),
                ),
                Span::styled(
                    &hook.name,
                    if selected {
                        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg())
                    },
                ),
                Span::styled(
                    format!("  {}", hook.path),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_profile_panel(
    frame: &mut Frame,
    area: Rect,
    state: &StatusState,
    theme: &EddaCraftTheme,
) {
    let focused = state.focused_panel == StatusPanel::Profile;
    let block = panel_block("Profile", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(vec![
        Span::styled("Active: ", Style::default().fg(theme.muted())),
        Span::styled(
            &state.data.profile.name,
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    for (i, check) in state.data.profile.checks.iter().enumerate() {
        let selected = focused && i == state.selected_item;
        let indicator = if selected { ">> " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(indicator, Style::default().fg(theme.fg())),
            Span::styled(
                format!("* {check}"),
                if selected {
                    Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg())
                },
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_results_panel(
    frame: &mut Frame,
    area: Rect,
    state: &StatusState,
    theme: &EddaCraftTheme,
) {
    let focused = state.focused_panel == StatusPanel::Results;
    let block = panel_block("Recent Runs", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = state
        .data
        .recent_runs
        .iter()
        .enumerate()
        .map(|(i, run)| {
            let selected = focused && i == state.selected_item;
            let indicator = if selected { ">> " } else { "  " };
            let status_icon = if run.passed { "*" } else { "x" };
            let status_colour = if run.passed {
                theme.success()
            } else {
                theme.error()
            };

            Line::from(vec![
                Span::styled(indicator, Style::default().fg(theme.fg())),
                Span::styled(
                    format!("{status_icon} "),
                    Style::default().fg(status_colour),
                ),
                Span::styled(
                    format!("{}/{} checks  ", run.checks_passed, run.checks_run),
                    if selected {
                        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg())
                    },
                ),
                Span::styled(
                    format!("{}ms  ", run.duration_ms),
                    Style::default().fg(theme.muted()),
                ),
                Span::styled(&run.timestamp, Style::default().fg(theme.muted())),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_state() -> StatusState {
        use super::super::{GateRunResult, HookStatus, ProfileInfo, StatusData};

        StatusState::new(StatusData {
            hooks: vec![
                HookStatus {
                    name: "pre-commit".to_string(),
                    active: true,
                    path: ".husky/pre-commit".to_string(),
                },
                HookStatus {
                    name: "commit-msg".to_string(),
                    active: false,
                    path: ".husky/commit-msg".to_string(),
                },
            ],
            profile: ProfileInfo {
                name: "dev".to_string(),
                checks: vec![
                    "eslint".to_string(),
                    "secret-scan".to_string(),
                    "architecture".to_string(),
                ],
                path: ".anvil/profiles/dev.yaml".to_string(),
            },
            recent_runs: vec![
                GateRunResult {
                    timestamp: "2026-03-16T10:00:00Z".to_string(),
                    passed: true,
                    score: 0.95,
                    checks_run: 5,
                    checks_passed: 5,
                    duration_ms: 2400,
                },
                GateRunResult {
                    timestamp: "2026-03-15T15:30:00Z".to_string(),
                    passed: false,
                    score: 0.6,
                    checks_run: 5,
                    checks_passed: 3,
                    duration_ms: 1800,
                },
            ],
            update_hint: None,
            insights_hint: None,
        })
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
    fn snapshot_results_focused() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.focused_panel = StatusPanel::Results;
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
        let state = sample_state();
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
    fn status_zoomed_render_shows_only_focused_panel() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.focused_panel = StatusPanel::Profile;
        state.zoomed = true;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        assert!(
            rendered.contains("Profile"),
            "zoomed render must show focused (Profile) panel; got:\n{rendered}"
        );
        for hidden in ["Hooks", "Results"] {
            assert!(
                !rendered.contains(hidden),
                "zoomed render must hide non-focused panel `{hidden}`"
            );
        }
    }
}
