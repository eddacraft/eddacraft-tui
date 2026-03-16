use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{WatchPanel, WatchState, WatchStatus};

pub fn render(frame: &mut Frame, area: Rect, state: &WatchState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Title
        Constraint::Min(6),    // 2x2 grid
        Constraint::Length(2), // Help text
    ])
    .split(area);

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        "Watch Dashboard",
        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(title, chunks[0]);

    // 2x2 grid: split vertically into top/bottom rows, each row split horizontally
    let rows =
        Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(chunks[1]);

    let top_cols =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[0]);
    let bottom_cols =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[1]);

    render_status_panel(frame, top_cols[0], state, theme);
    render_queue_panel(frame, top_cols[1], state, theme);
    render_history_panel(frame, bottom_cols[0], state, theme);
    render_stats_panel(frame, bottom_cols[1], state, theme);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(theme.accent())),
        Span::styled(" navigate  ", Style::default().fg(theme.muted())),
        Span::styled("h/l", Style::default().fg(theme.accent())),
        Span::styled(" switch panel  ", Style::default().fg(theme.muted())),
        Span::styled("PgUp/PgDn", Style::default().fg(theme.accent())),
        Span::styled(" up/down row  ", Style::default().fg(theme.muted())),
        Span::styled("q", Style::default().fg(theme.accent())),
        Span::styled(" quit", Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(help, chunks[2]);
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
