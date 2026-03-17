use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{ConfigFormat, InitMode, InitState, InitStep};

pub fn render(frame: &mut Frame, area: Rect, state: &InitState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Progress bar
        Constraint::Min(6),    // Step content
        Constraint::Length(2), // Help text
    ])
    .split(area);

    // Step progress indicator
    render_progress(frame, chunks[0], state, theme);

    // Step content
    match state.step {
        InitStep::Mode => render_mode_step(frame, chunks[1], state, theme),
        InitStep::Format => render_format_step(frame, chunks[1], state, theme),
        InitStep::Directory => render_directory_step(frame, chunks[1], state, theme),
        InitStep::Checks => render_checks_step(frame, chunks[1], state, theme),
        InitStep::Summary => render_summary_step(frame, chunks[1], state, theme),
    }

    // Help text
    let help_text = match state.step {
        InitStep::Mode | InitStep::Format => "j/k navigate  enter select  q quit",
        InitStep::Directory => "type path  enter confirm  esc back  q quit",
        InitStep::Checks => "j/k navigate  space toggle  enter next  esc back  q quit",
        InitStep::Summary => "enter confirm  esc back  q quit",
    };
    let help = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(theme.muted()),
    )));
    frame.render_widget(help, chunks[2]);
}

fn render_progress(frame: &mut Frame, area: Rect, state: &InitState, theme: &EddaCraftTheme) {
    let step_labels = ["Mode", "Format", "Directory", "Checks", "Summary"];
    let steps: Vec<Span> = (0..InitStep::TOTAL)
        .flat_map(|i| {
            let label = step_labels[i];
            let style = match i.cmp(&state.step.index()) {
                std::cmp::Ordering::Equal => Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
                std::cmp::Ordering::Less => Style::default().fg(theme.success()),
                std::cmp::Ordering::Greater => Style::default().fg(theme.muted()),
            };
            let separator = if i < InitStep::TOTAL - 1 { " > " } else { "" };
            vec![
                Span::styled(label, style),
                Span::styled(separator, Style::default().fg(theme.muted())),
            ]
        })
        .collect();

    frame.render_widget(Paragraph::new(Line::from(steps)), area);
}

fn render_mode_step(frame: &mut Frame, area: Rect, state: &InitState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Select Mode ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<Line> = InitMode::ALL
        .iter()
        .enumerate()
        .map(|(i, mode)| {
            let selected = i == state.mode_selected;
            let indicator = if selected { ">> " } else { "  " };
            let name_style = if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(mode.label(), name_style),
                Span::styled(
                    format!("  {}", mode.description()),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_format_step(frame: &mut Frame, area: Rect, state: &InitState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Select Format ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<Line> = ConfigFormat::ALL
        .iter()
        .enumerate()
        .map(|(i, fmt)| {
            let selected = i == state.format_selected;
            let indicator = if selected { ">> " } else { "  " };
            let name_style = if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(fmt.label(), name_style),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_directory_step(frame: &mut Frame, area: Rect, state: &InitState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Project Directory ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prompt = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            "Enter the project root directory:",
            Style::default().fg(theme.fg()),
        )),
        Line::from(Span::styled(
            format!(">> {}_", state.text_input.value),
            Style::default().fg(theme.accent()),
        )),
        Line::default(),
        Line::from(Span::styled(
            "Leave empty for current directory (.)",
            Style::default().fg(theme.muted()),
        )),
    ]));
    frame.render_widget(prompt, inner);
}

fn render_checks_step(frame: &mut Frame, area: Rect, state: &InitState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Select Checks ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<Line> = state
        .check_toggles
        .iter()
        .enumerate()
        .map(|(i, check)| {
            let selected = i == state.check_selected;
            let indicator = if selected { ">> " } else { "  " };
            let toggle_icon = if check.enabled { "[x]" } else { "[ ]" };
            let toggle_colour = if check.enabled {
                theme.success()
            } else {
                theme.muted()
            };
            let name_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(
                    format!("{toggle_icon} "),
                    Style::default().fg(toggle_colour),
                ),
                Span::styled(&check.name, name_style),
                Span::styled(
                    format!("  {}", check.description),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_summary_step(frame: &mut Frame, area: Rect, state: &InitState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Summary ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let enabled_checks: Vec<&str> = state
        .check_toggles
        .iter()
        .filter(|c| c.enabled)
        .map(|c| c.name.as_str())
        .collect();
    let checks_display = if enabled_checks.is_empty() {
        "none".to_string()
    } else {
        enabled_checks.join(", ")
    };

    let content = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled("Mode:      ", Style::default().fg(theme.muted())),
            Span::styled(state.config.mode.label(), Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled("Format:    ", Style::default().fg(theme.muted())),
            Span::styled(state.config.format.label(), Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled("Directory: ", Style::default().fg(theme.muted())),
            Span::styled(&state.config.directory, Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled("Checks:    ", Style::default().fg(theme.muted())),
            Span::styled(checks_display, Style::default().fg(theme.fg())),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "Press enter to confirm and initialise the project",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ]));
    frame.render_widget(content, inner);
}
