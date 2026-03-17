use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{WizardState, WizardStep};

pub fn render(frame: &mut Frame, area: Rect, state: &WizardState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Progress bar
        Constraint::Min(6),    // Step content
    ])
    .split(area);

    // Step progress indicator
    render_progress(frame, chunks[0], state, theme);

    // Step content
    match state.step {
        WizardStep::TemplateSelect => render_template_step(frame, chunks[1], state, theme),
        WizardStep::ProjectName => render_name_step(frame, chunks[1], state, theme),
        WizardStep::Configure => render_configure_step(frame, chunks[1], state, theme),
        WizardStep::Summary => render_summary_step(frame, chunks[1], state, theme),
    }
}

fn render_progress(frame: &mut Frame, area: Rect, state: &WizardState, theme: &EddaCraftTheme) {
    let steps: Vec<Span> = (0..WizardStep::TOTAL)
        .flat_map(|i| {
            let label = match i {
                0 => "Template",
                1 => "Name",
                2 => "Configure",
                3 => "Summary",
                _ => "",
            };
            let style = match i.cmp(&state.step.index()) {
                std::cmp::Ordering::Equal => Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
                std::cmp::Ordering::Less => Style::default().fg(theme.success()),
                std::cmp::Ordering::Greater => Style::default().fg(theme.muted()),
            };
            let separator = if i < WizardStep::TOTAL - 1 { " > " } else { "" };
            vec![
                Span::styled(label, style),
                Span::styled(separator, Style::default().fg(theme.muted())),
            ]
        })
        .collect();

    frame.render_widget(Paragraph::new(Line::from(steps)), area);
}

fn render_template_step(
    frame: &mut Frame,
    area: Rect,
    state: &WizardState,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Select a Template ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<Line> = state
        .templates
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let selected = i == state.template_selected;
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
                Span::styled(&t.name, name_style),
                Span::styled(
                    format!("  {}", t.description),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_name_step(frame: &mut Frame, area: Rect, state: &WizardState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Project Name ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prompt = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            "Enter a name for your project:",
            Style::default().fg(theme.fg()),
        )),
        Line::from(Span::styled(
            format!(">> {}_", state.text_input.value),
            Style::default().fg(theme.accent()),
        )),
    ]));
    frame.render_widget(prompt, inner);
}

fn render_configure_step(
    frame: &mut Frame,
    area: Rect,
    state: &WizardState,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Configure ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let options: &[(bool, &str)] = &[
        (state.config.enable_watch, "Enable watch mode"),
        (state.config.enable_hooks, "Install git hooks"),
    ];

    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(i, (enabled, label))| {
            let selected = i == state.config_selected;
            let indicator = if selected { ">> " } else { "   " };
            let icon = if *enabled { "[x]" } else { "[ ]" };
            let icon_colour = if *enabled {
                theme.success()
            } else {
                theme.muted()
            };
            let label_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, label_style),
                Span::styled(format!("{icon} "), Style::default().fg(icon_colour)),
                Span::styled(*label, label_style),
            ])
        })
        .collect();

    let content = Paragraph::new(Text::from(lines));
    frame.render_widget(content, inner);
}

fn render_summary_step(frame: &mut Frame, area: Rect, state: &WizardState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Summary ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let template_name = state
        .config
        .template_id
        .as_ref()
        .and_then(|id| state.templates.iter().find(|t| &t.id == id))
        .map_or("none", |t| t.name.as_str());

    let content = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled("Project:  ", Style::default().fg(theme.muted())),
            Span::styled(&state.config.project_name, Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled("Template: ", Style::default().fg(theme.muted())),
            Span::styled(template_name, Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled("Watch:    ", Style::default().fg(theme.muted())),
            Span::styled(
                if state.config.enable_watch {
                    "enabled"
                } else {
                    "disabled"
                },
                Style::default().fg(theme.fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("Hooks:    ", Style::default().fg(theme.muted())),
            Span::styled(
                if state.config.enable_hooks {
                    "enabled"
                } else {
                    "disabled"
                },
                Style::default().fg(theme.fg()),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "Press enter to confirm and create the project",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ]));
    frame.render_widget(content, inner);
}
