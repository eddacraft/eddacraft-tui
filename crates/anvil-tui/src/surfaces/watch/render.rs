use std::time::SystemTime;

use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::time_display::format_live_timestamp;
use super::{ActionResultLine, WatchPanel, WatchState, WatchStatus};

fn advisory_warning_count(state: &WatchState) -> usize {
    state
        .data
        .queue
        .iter()
        .filter(|queued| {
            matches!(
                queued.notification.class,
                anvil_kernel_types::NotificationClass::Finding
                    | anvil_kernel_types::NotificationClass::Warning
            )
        })
        .count()
}

pub fn render(frame: &mut Frame, area: Rect, state: &WatchState, theme: &EddaCraftTheme) {
    // Reserve up to two 1-line strips at the bottom: the DISTRIB-002
    // update-available hint or INSIGHTS-004 first-week nudge (when set),
    // then the most recent --action outcome (LAUNCH-002). Hint goes above
    // the action footer.
    let has_hint = state.data.daemon_fallback_notice.is_some()
        || state.data.update_hint.is_some()
        || state.data.insights_hint.is_some();
    let (grid_area, hint_area, footer_area) =
        split_footer(area, has_hint, state.data.last_action.is_some());

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

    if let (Some(strip), Some(notice)) = (hint_area, state.data.daemon_fallback_notice.as_ref()) {
        render_daemon_fallback_notice(frame, strip, notice, theme);
    } else if let (Some(strip), Some(hint)) = (hint_area, state.data.insights_hint.as_ref()) {
        // INSIGHTS-004: render the nudge using the update-hint visual
        // style (single line, accent-ish) but without advisory colouring.
        let para = ratatui::widgets::Paragraph::new(ratatui::text::Line::from(
            ratatui::text::Span::styled(
                hint,
                ratatui::style::Style::default()
                    .fg(theme.muted())
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ));
        frame.render_widget(para, strip);
    } else if let (Some(strip), Some(hint)) = (hint_area, state.data.update_hint.as_ref()) {
        render_update_hint(frame, strip, hint, theme);
    }
    if let (Some(footer), Some(line)) = (footer_area, state.data.last_action.as_ref()) {
        render_action_footer(frame, footer, line, theme);
    }
}

fn render_daemon_fallback_notice(
    frame: &mut Frame,
    area: Rect,
    notice: &str,
    theme: &EddaCraftTheme,
) {
    let para = Paragraph::new(Line::from(Span::styled(
        notice,
        Style::default()
            .fg(theme.error())
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(para, area);
}

/// Compute the grid + optional 1-line footers. Extracted from `render`
/// so tests can assert the layout deterministically without needing
/// the full ratatui frame setup.
pub(crate) fn split_footer(
    area: Rect,
    has_hint: bool,
    has_action: bool,
) -> (Rect, Option<Rect>, Option<Rect>) {
    match (has_hint, has_action) {
        (false, false) => (area, None, None),
        (true, false) => {
            let s = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
            (s[0], Some(s[1]), None)
        }
        (false, true) => {
            let s = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
            (s[0], None, Some(s[1]))
        }
        (true, true) => {
            let s = Layout::vertical([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);
            (s[0], Some(s[1]), Some(s[2]))
        }
    }
}

/// DISTRIB-002: render the single-line "update available" hint at
/// `area`. ASCII-only by convention (matches the action footer
/// comment above) so the line survives Windows legacy code-pages and
/// CI log captures.
fn render_update_hint(
    frame: &mut Frame,
    area: Rect,
    hint: &crate::surfaces::UpdateHint,
    theme: &EddaCraftTheme,
) {
    let colour = if hint.advisory_ids.is_empty() {
        theme.accent()
    } else {
        // Security advisories elevate to the error/warn colour so the
        // user does not skim past a CVE row.
        theme.error()
    };
    let para = Paragraph::new(Line::from(Span::styled(
        hint.render_line(),
        Style::default().fg(colour).add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(para, area);
}

/// One-line footer summarising the most recent `--action` outcome.
/// ASCII-only to match the rest of the watch surface (the banner, per-event
/// labels, bare-exclude warning) — Windows legacy code-pages and CI log
/// captures may not handle the broader Unicode the rest of the TUI uses,
/// and the watch dashboard is the demo path so a mojibake'd footer would
/// be visible at exactly the wrong moment.
///
/// Glyphs:
/// - `[*]` — child exited 0
/// - `[x]` — child exited non-zero (footer shows the exit code)
/// - `[!]` — child did not run to a recorded exit, or daemon assurance is
///   degraded. For child errors, `error_detail` carries the cause (spawn
///   failure, cancellation, wait error). It is rendered verbatim so the user
///   can tell `(spawn failed: Permission denied)` apart from `(cancelled)`
///   apart from `(wait failed: ...)` — fixes #1279 review: the previous
///   "spawn failed: …" prefix lied about cancellations and signal-kills.
fn render_action_footer(
    frame: &mut Frame,
    area: Rect,
    line: &ActionResultLine,
    theme: &EddaCraftTheme,
) {
    #[allow(clippy::cast_precision_loss)]
    let secs = line.duration_ms as f64 / 1000.0;

    let (colour, detail) = if line.errored() {
        let cause = line.error_detail.as_deref().unwrap_or("did not complete");
        (theme.error(), format!("[!] {} ({cause})", line.action))
    } else if line.assurance_degraded {
        let assurance = line
            .assurance_detail
            .as_deref()
            .unwrap_or("daemon assurance degraded");
        (
            theme.warning(),
            format!("[!] {} ({secs:.1}s, {assurance})", line.action),
        )
    } else if line.passed() {
        let detail = match line.assurance_detail.as_deref() {
            Some(assurance) => format!("[*] {} ({secs:.1}s, {assurance})", line.action),
            None => format!("[*] {} ({secs:.1}s)", line.action),
        };
        (theme.success(), detail)
    } else {
        let code = line.exit_code.unwrap_or(-1);
        (
            theme.error(),
            format!("[x] {} ({secs:.1}s, exit {code})", line.action),
        )
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

    let warning_count = advisory_warning_count(state);
    if warning_count > 0 {
        lines.push(Line::from(Span::styled(
            format!("  Warnings: {warning_count}"),
            Style::default()
                .fg(theme.warning())
                .add_modifier(Modifier::BOLD),
        )));
    }

    if let Some(warmup) = state.data.warmup.as_ref() {
        let progress = if warmup.total == 0 {
            String::from("starting")
        } else {
            format!("{}/{}", warmup.current.min(warmup.total), warmup.total)
        };
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("  {} ({progress})", warmup.phase),
            Style::default().fg(theme.accent()),
        )));
        if warmup.total >= 1_000 {
            lines.push(Line::from(Span::styled(
                "  Large repository warm-up may take a moment",
                Style::default().fg(theme.muted()),
            )));
        }
    }

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

    // Format timestamps at render time so stored kernel ISO8601 stays raw
    // while the live dashboard shows relative ages (CIB-266).
    let now = SystemTime::now();
    let lines: Vec<Line> = state
        .data
        .queue
        .iter()
        .enumerate()
        .map(|(i, change)| {
            let selected = focused && i == state.selected_item;
            let indicator = if selected { ">> " } else { "  " };
            let ts = format_live_timestamp(&change.timestamp, now);

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
                    format!("  {} {ts}", change.notification.message),
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

    let now = SystemTime::now();
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
            let ts = format_live_timestamp(&run.timestamp, now);

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
                Span::styled(ts, Style::default().fg(theme.muted())),
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
    use anvil_kernel_types::{Notification, NotificationClass, NotificationPriority};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::super::QueuedNotification;

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
            warmup: None,
            last_action: None,
            update_hint: None,
            insights_hint: None,
            daemon_fallback_notice: None,
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
            warmup: None,
            last_action: None,
            update_hint: None,
            insights_hint: None,
            daemon_fallback_notice: None,
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

    fn action_result(action: &str) -> ActionResultLine {
        ActionResultLine {
            action: action.to_string(),
            exit_code: Some(0),
            duration_ms: 1200,
            timestamp: "10:30:00".to_string(),
            error_detail: None,
            daemon_notice: None,
            assurance_detail: None,
            assurance_degraded: false,
        }
    }

    #[test]
    fn daemon_certified_action_footer_names_antipattern_scope() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        let mut line = action_result("check");
        line.assurance_detail = Some("antipattern-only certified".to_string());
        state.data.last_action = Some(line);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        assert!(
            rendered.contains("[*] check (1.2s, antipattern-only certified)"),
            "daemon-certified footer must disclose its family scope, got:\n{rendered}"
        );
        assert!(
            !rendered.contains("[*] check (1.2s)"),
            "daemon-certified footer must never render a bare success claim, got:\n{rendered}"
        );
    }

    #[test]
    fn degraded_daemon_action_footer_dominates_exit_zero_as_warning() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        let mut line = action_result("check");
        line.assurance_detail =
            Some("stale{cross-file-resolution-needed}; antipattern-only partial".to_string());
        line.assurance_degraded = true;
        state.data.last_action = Some(line);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        assert!(
            rendered.contains(
                "[!] check (1.2s, stale{cross-file-resolution-needed}; antipattern-only partial)"
            ),
            "degraded assurance must dominate an exit-zero action, got:\n{rendered}"
        );
        assert_eq!(
            terminal.backend().buffer()[(0, 23)].fg,
            theme.warning(),
            "degraded assurance must use the warning colour"
        );
    }

    #[test]
    fn refused_daemon_attestation_uses_warning_footer() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        let mut line = action_result("check");
        line.assurance_detail = Some("refusing attestation: no check family reported".to_string());
        line.assurance_degraded = true;
        state.data.last_action = Some(line);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        assert!(
            rendered.contains("[!] check (1.2s, refusing attestation: no check family reported)"),
            "refused attestation must dominate an exit-zero action, got:\n{rendered}"
        );
        assert_eq!(terminal.backend().buffer()[(0, 23)].fg, theme.warning());
    }

    #[test]
    fn ordinary_gate_action_footer_remains_unchanged() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.data.last_action = Some(action_result("gate"));
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        assert!(
            rendered.contains("[*] gate (1.2s)"),
            "ordinary child action footer changed unexpectedly, got:\n{rendered}"
        );
        assert!(!rendered.contains("antipattern-only"));
        assert_eq!(terminal.backend().buffer()[(0, 23)].fg, theme.success());
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

    #[test]
    fn advisory_warning_does_not_render_failing_status() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.data.status = WatchStatus::Passing;
        state.data.queue = std::collections::VecDeque::from([QueuedNotification {
            notification: Notification::new(
                NotificationClass::Warning,
                NotificationPriority::Normal,
                "src/lib.rs",
                "new public symbol detected",
            ),
            timestamp: "10:30:03".to_string(),
        }]);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        assert!(
            rendered.contains("Warnings"),
            "advisory findings should render as warnings; got:\n{rendered}"
        );
        assert!(
            !rendered.contains("Failing"),
            "advisory findings must not render as Failing; got:\n{rendered}"
        );
    }

    #[test]
    fn warmup_progress_renders_phase_and_progress() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.data.status = WatchStatus::Running;
        state.data.warmup = Some(super::super::WatchWarmup {
            phase: "Building graph".to_string(),
            current: 3,
            total: 10,
        });
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());

        assert!(rendered.contains("Building graph"), "got:\n{rendered}");
        assert!(rendered.contains("3/10"), "got:\n{rendered}");
    }

    // ─── DISTRIB-002 update-available hint ─────────────────────────

    #[test]
    fn update_hint_renders_one_line_at_bottom() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.data.update_hint = Some(crate::surfaces::UpdateHint {
            latest_version: "0.7.0-beta".into(),
            current_version: "0.6.2-beta".into(),
            advisory_ids: vec![],
        });
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());
        assert!(
            rendered.contains("Update available"),
            "missing hint line, got:\n{rendered}"
        );
        assert!(rendered.contains("0.7.0-beta"));
        assert!(rendered.contains("anvil update"));
    }

    #[test]
    fn update_hint_with_advisory_names_id_on_the_line() {
        let backend = TestBackend::new(140, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.data.update_hint = Some(crate::surfaces::UpdateHint {
            latest_version: "0.7.0-beta".into(),
            current_version: "0.6.2-beta".into(),
            advisory_ids: vec!["GHSA-aaaa-bbbb-cccc".into()],
        });
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());
        assert!(rendered.contains("GHSA-aaaa-bbbb-cccc"));
        assert!(rendered.contains("security advisory"));
    }

    /// DISTRIB-002 spec validation test
    /// (`watch::tests::update_hint_rate_limited`): the TUI render is
    /// driven entirely by the presence of `update_hint` in `WatchData`.
    /// When the rate-limit gate (owned by anvil-cli's
    /// `update_hint::record_if_due`) suppresses the hint, the consumer
    /// sets `update_hint = None` and the watch surface MUST NOT render
    /// any "Update available" line. The primitive's rate-limit
    /// behaviour is covered exhaustively in
    /// `update_hint::tests::record_if_due_*` in anvil-cli.
    #[test]
    fn update_hint_rate_limited() {
        // `None` simulates the rate-limit gate suppressing the hint;
        // the render must omit the "Update available" line entirely.
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.data.update_hint = None;
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());
        assert!(
            !rendered.contains("Update available"),
            "rate-limited state must not render the hint, got:\n{rendered}"
        );

        // When the gate allows it, `Some(hint)` is rendered.
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        state.data.update_hint = Some(crate::surfaces::UpdateHint {
            latest_version: "0.7.0-beta".into(),
            current_version: "0.6.2-beta".into(),
            advisory_ids: vec![],
        });
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let rendered = buffer_contents(terminal.backend().buffer());
        assert!(
            rendered.contains("Update available"),
            "allowed state must render the hint, got:\n{rendered}"
        );
    }

    #[test]
    fn daemon_fallback_notice_renders_in_footer_strip() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.data.daemon_fallback_notice =
            Some("daemon: unavailable -- scoped fallback".to_string());
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered = buffer_contents(terminal.backend().buffer());
        assert!(
            rendered.contains("daemon: unavailable -- scoped fallback"),
            "daemon fallback notice must be visible in TUI mode, got:\n{rendered}"
        );
    }

    #[test]
    fn split_footer_reserves_correct_lines_for_each_combination() {
        let area = Rect::new(0, 0, 80, 24);
        let (grid, hint, action) = split_footer(area, false, false);
        assert_eq!(grid.height, 24);
        assert!(hint.is_none() && action.is_none());

        let (grid, hint, action) = split_footer(area, true, false);
        assert_eq!(grid.height, 23);
        assert!(hint.is_some() && action.is_none());

        let (grid, hint, action) = split_footer(area, false, true);
        assert_eq!(grid.height, 23);
        assert!(hint.is_none() && action.is_some());

        let (grid, hint, action) = split_footer(area, true, true);
        assert_eq!(grid.height, 22);
        let hint = hint.unwrap();
        let action = action.unwrap();
        // Hint sits above action so the action footer is closest to
        // the prompt edge, matching the comment in `render`.
        assert!(hint.y < action.y);
    }
}
