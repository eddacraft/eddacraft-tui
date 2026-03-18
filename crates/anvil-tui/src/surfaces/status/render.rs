use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{StatusPanel, StatusState};

pub fn render(frame: &mut Frame, area: Rect, state: &StatusState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Ratio(1, 3), // Hooks panel
        Constraint::Ratio(1, 3), // Profile panel
        Constraint::Ratio(1, 3), // Results panel
    ])
    .split(area);

    render_hooks_panel(frame, chunks[0], state, theme);
    render_profile_panel(frame, chunks[1], state, theme);
    render_results_panel(frame, chunks[2], state, theme);
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
