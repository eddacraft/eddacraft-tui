use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{ActionResultLine, WatchPanel, WatchState, WatchStatus};

pub fn render(frame: &mut Frame, area: Rect, state: &WatchState, theme: &EddaCraftTheme) {
    // Reserve a 1-line footer at the bottom for the most recent --action
    // outcome (LAUNCH-002). Hidden when no action has run yet, so the 2x2
    // grid keeps the full area in the common case.
    let (grid_area, footer_area) = if state.data.last_action.is_some() {
        let split = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        (split[0], Some(split[1]))
    } else {
        (area, None)
    };

    // Zoom mode: collapse the 2x2 grid to the focused panel filling the
    // whole grid area. Useful at narrow widths where the four-up layout
    // becomes unreadable. Toggle via `z`; `esc` exits zoom first, then
    // navigates back on a subsequent press.
    if state.zoomed {
        match state.focused_panel {
            WatchPanel::Status => render_status_panel(frame, grid_area, state, theme),
            WatchPanel::Queue => render_queue_panel(frame, grid_area, state, theme),
            WatchPanel::History => render_history_panel(frame, grid_area, state, theme),
            WatchPanel::Stats => render_stats_panel(frame, grid_area, state, theme),
        }
    } else {
        // 2x2 grid: split vertically into top/bottom rows, each row split horizontally
        let rows =
            Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(grid_area);

        let top_cols =
            Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[0]);
        let bottom_cols =
            Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[1]);

        render_status_panel(frame, top_cols[0], state, theme);
        render_queue_panel(frame, top_cols[1], state, theme);
        render_history_panel(frame, bottom_cols[0], state, theme);
        render_stats_panel(frame, bottom_cols[1], state, theme);
    }

    if let (Some(footer), Some(line)) = (footer_area, state.data.last_action.as_ref()) {
        render_action_footer(frame, footer, line, theme);
    }
}

/// One-line footer summarising the most recent `--action` outcome.
/// ASCII-only to match the rest of the watch surface (the banner, per-event
/// labels, bare-exclude warning) — Windows legacy code-pages and CI log
/// captures may not handle the broader Unicode the rest of the TUI uses,
/// and the watch dashboard is the demo path so a mojibake'd footer would
/// be visible at exactly the wrong moment.
fn render_action_footer(
    frame: &mut Frame,
    area: Rect,
    line: &ActionResultLine,
    theme: &EddaCraftTheme,
) {
    let (glyph, colour) = if line.passed() {
        ("[*]", theme.success())
    } else {
        ("[x]", theme.error())
    };

    #[allow(clippy::cast_precision_loss)]
    let secs = line.duration_ms as f64 / 1000.0;
    let detail = match line.exit_code {
        Some(0) | None => format!("{glyph} {} ({secs:.1}s)", line.action),
        Some(code) => format!("{glyph} {} ({secs:.1}s, exit {code})", line.action),
    };

    let para = Paragraph::new(Line::from(Span::styled(
        detail,
        Style::default().fg(colour).add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(para, area);
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

fn render_status_panel(frame: &mut Frame, area: Rect, state: &WatchState, theme: &EddaCraftTheme) {
    let focused = state.focused_panel == WatchPanel::Status;
    let block = panel_block("Status", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let status = &state.data.status;
    let status_colour = match status {
        WatchStatus::Idle => theme.muted(),
        WatchStatus::Running => theme.accent(),
        WatchStatus::Passing => theme.success(),
        WatchStatus::Failing => theme.error(),
    };

    let mut lines = vec![
        Line::default(),
        Line::from(vec![
            Span::styled(
                format!("  {} ", status.icon()),
                Style::default()
                    .fg(status_colour)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                status.label(),
                Style::default()
                    .fg(status_colour)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            format!("  {} files watched", state.data.stats.files_watched),
            Style::default().fg(theme.muted()),
        )),
    ];

    // When failing, surface the most recent violation/error so the user
    // doesn't have to switch panels to see what went wrong.
    if matches!(status, WatchStatus::Failing)
        && let Some(last) = state.data.queue.back()
    {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("  {}", last.notification.title),
            Style::default().fg(theme.error()),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {}", last.notification.message),
            Style::default().fg(theme.muted()),
        )));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_queue_panel(frame: &mut Frame, area: Rect, state: &WatchState, theme: &EddaCraftTheme) {
    let focused = state.focused_panel == WatchPanel::Queue;
    let block = panel_block("Queue", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.data.queue.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No pending changes",
            Style::default().fg(theme.muted()),
        )));
        frame.render_widget(empty, inner);
        return;
    }

    let lines: Vec<Line> = state
        .data
        .queue
        .iter()
        .enumerate()
        .map(|(i, change)| {
            let selected = focused && i == state.selected_item;
            let indicator = if selected { ">> " } else { "  " };

            Line::from(vec![
                Span::styled(indicator, Style::default().fg(theme.fg())),
                Span::styled(
                    &change.notification.title,
                    if selected {
                        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg())
                    },
                ),
                Span::styled(
                    format!("  {} {}", change.notification.message, change.timestamp),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    // `selected_item` is shared across panels, so only apply scroll when
    // this panel is focused — otherwise navigating in a sibling panel
    // would scroll the Queue view even though its own selection cursor
    // is parked at 0.
    let scroll = if focused {
        scroll_offset_for(state.selected_item, lines.len(), inner.height)
    } else {
        0
    };
    frame.render_widget(Paragraph::new(Text::from(lines)).scroll((scroll, 0)), inner);
}

fn render_history_panel(frame: &mut Frame, area: Rect, state: &WatchState, theme: &EddaCraftTheme) {
    let focused = state.focused_panel == WatchPanel::History;
    let block = panel_block("History", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.data.history.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No runs yet",
            Style::default().fg(theme.muted()),
        )));
        frame.render_widget(empty, inner);
        return;
    }

    let lines: Vec<Line> = state
        .data
        .history
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

    // See `render_queue_panel` — only apply scroll when focused so an
    // unselected History panel doesn't scroll in sympathy with Queue
    // navigation.
    let scroll = if focused {
        scroll_offset_for(state.selected_item, lines.len(), inner.height)
    } else {
        0
    };
    frame.render_widget(Paragraph::new(Text::from(lines)).scroll((scroll, 0)), inner);
}

/// Compute a vertical scroll offset that keeps `selected` visible inside a
/// panel of `height` rows containing `total` lines. Returns 0 when the panel
/// can fit everything.
fn scroll_offset_for(selected: usize, total: usize, height: u16) -> u16 {
    let height = height as usize;
    if total <= height || height == 0 {
        return 0;
    }
    let max_offset = total - height;
    let offset = selected.saturating_sub(height.saturating_sub(1));
    u16::try_from(offset.min(max_offset)).unwrap_or(u16::MAX)
}

#[allow(clippy::cast_precision_loss)]
fn render_stats_panel(frame: &mut Frame, area: Rect, state: &WatchState, theme: &EddaCraftTheme) {
    let focused = state.focused_panel == WatchPanel::Stats;
    let block = panel_block("Stats", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let watch_stats = &state.data.stats;
    let display_pass_rate = (*state.anim_pass_rate).clamp(0.0, 1.0);
    let display_avg_duration = (*state.anim_avg_duration_ms).max(0.0);

    let pass_colour = if display_pass_rate >= 0.8 {
        theme.success()
    } else if display_pass_rate >= 0.5 {
        theme.warning()
    } else {
        theme.error()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("  Total runs:   ", Style::default().fg(theme.muted())),
            Span::styled(
                watch_stats.total_runs.to_string(),
                Style::default().fg(theme.fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Pass rate:    ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{:.0}%", display_pass_rate * 100.0),
                Style::default()
                    .fg(pass_colour)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Avg duration: ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{display_avg_duration:.0}ms"),
                Style::default().fg(theme.fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Files:        ", Style::default().fg(theme.muted())),
            Span::styled(
                watch_stats.files_watched.to_string(),
                Style::default().fg(theme.fg()),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_state() -> WatchState {
        use super::super::{QueuedNotification, RunHistory, WatchData, WatchStats};
        use anvil_kernel_types::{Notification, NotificationClass, NotificationPriority};

        WatchState::new(WatchData {
            status: WatchStatus::Passing,
            queue: std::collections::VecDeque::from([
                QueuedNotification {
                    notification: Notification::new(
                        NotificationClass::Finding,
                        NotificationPriority::High,
                        "src/main.rs",
                        "modified",
                    ),
                    timestamp: "10:30:01".to_string(),
                },
                QueuedNotification {
                    notification: Notification::new(
                        NotificationClass::Finding,
                        NotificationPriority::High,
                        "src/lib.rs",
                        "created",
                    ),
                    timestamp: "10:30:02".to_string(),
                },
            ]),
            history: vec![
                RunHistory {
                    passed: true,
                    checks_run: 5,
                    checks_passed: 5,
                    duration_ms: 1200,
                    timestamp: "10:29:50".to_string(),
                },
                RunHistory {
                    passed: false,
                    checks_run: 5,
                    checks_passed: 3,
                    duration_ms: 980,
                    timestamp: "10:28:30".to_string(),
                },
            ],
            stats: WatchStats {
                total_runs: 42,
                pass_rate: 0.88,
                avg_duration_ms: 1050,
                files_watched: 128,
            },
            last_action: None,
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
                render(frame, frame.area(), &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_queue_focused() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.focused_panel = WatchPanel::Queue;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render(frame, frame.area(), &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_idle_empty() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WatchState::new(super::super::WatchData {
            status: WatchStatus::Idle,
            queue: std::collections::VecDeque::new(),
            history: Vec::new(),
            stats: super::super::WatchStats {
                total_runs: 0,
                pass_rate: 0.0,
                avg_duration_ms: 0,
                files_watched: 0,
            },
            last_action: None,
        });
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render(frame, frame.area(), &state, &theme);
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

    // v0.5.0 — pressing `z` toggles a zoom mode where only the focused
    // panel fills the area, so users on narrow IDE side panes can read
    // queue/history content without competing with status/stats tiles.

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
    fn zoomed_render_shows_only_focused_panel() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.focused_panel = WatchPanel::Queue;
        state.zoomed = true;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        assert!(
            rendered.contains("Queue"),
            "zoomed render must show focused (Queue) panel; got:\n{rendered}"
        );
        // Other panel titles must not appear when zoomed.
        for hidden in ["Status", "History", "Stats"] {
            assert!(
                !rendered.contains(hidden),
                "zoomed render must hide non-focused panel `{hidden}`; got:\n{rendered}"
            );
        }
    }

    #[test]
    fn unzoomed_default_shows_all_four_panels() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = sample_state();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        for panel in ["Status", "Queue", "History", "Stats"] {
            assert!(
                rendered.contains(panel),
                "unzoomed render must show panel `{panel}`; got:\n{rendered}"
            );
        }
    }
}
