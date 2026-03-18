use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{WatchPanel, WatchState, WatchStatus};

pub fn render(frame: &mut Frame, area: Rect, state: &WatchState, theme: &EddaCraftTheme) {
    // 2x2 grid: split vertically into top/bottom rows, each row split horizontally
    let rows = Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);

    let top_cols =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[0]);
    let bottom_cols =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[1]);

    render_status_panel(frame, top_cols[0], state, theme);
    render_queue_panel(frame, top_cols[1], state, theme);
    render_history_panel(frame, bottom_cols[0], state, theme);
    render_stats_panel(frame, bottom_cols[1], state, theme);
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

    let lines = vec![
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
                    &change.file,
                    if selected {
                        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg())
                    },
                ),
                Span::styled(
                    format!("  {} {}", change.kind, change.timestamp),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
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

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

#[allow(clippy::cast_precision_loss)]
fn render_stats_panel(frame: &mut Frame, area: Rect, state: &WatchState, theme: &EddaCraftTheme) {
    let focused = state.focused_panel == WatchPanel::Stats;
    let block = panel_block("Stats", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let watch_stats = &state.data.stats;
    let pass_colour = if watch_stats.pass_rate >= 0.8 {
        theme.success()
    } else if watch_stats.pass_rate >= 0.5 {
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
                format!("{:.0}%", watch_stats.pass_rate * 100.0),
                Style::default()
                    .fg(pass_colour)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Avg duration: ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}ms", watch_stats.avg_duration_ms),
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
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn sample_state() -> WatchState {
        use super::super::{QueuedChange, RunHistory, WatchData, WatchStats};

        WatchState::new(WatchData {
            status: WatchStatus::Passing,
            queue: vec![
                QueuedChange {
                    file: "src/main.rs".to_string(),
                    kind: "modified".to_string(),
                    timestamp: "10:30:01".to_string(),
                },
                QueuedChange {
                    file: "src/lib.rs".to_string(),
                    kind: "created".to_string(),
                    timestamp: "10:30:02".to_string(),
                },
            ],
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
            queue: Vec::new(),
            history: Vec::new(),
            stats: super::super::WatchStats {
                total_runs: 0,
                pass_rate: 0.0,
                avg_duration_ms: 0,
                files_watched: 0,
            },
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
}
