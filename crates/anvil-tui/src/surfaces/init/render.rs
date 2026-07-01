use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{ConfigFormat, InitMode, InitState, InitStep};
use crate::shell::inset_content;

pub fn render(frame: &mut Frame, area: Rect, state: &InitState, theme: &EddaCraftTheme) {
    let area = inset_content(area);
    let chunks = Layout::vertical([
        Constraint::Length(3), // Progress bar
        Constraint::Min(6),    // Step content
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
        directory_input_line(state, theme),
        Line::default(),
        Line::from(Span::styled(
            "Leave empty for current directory (.)",
            Style::default().fg(theme.muted()),
        )),
    ]));
    frame.render_widget(prompt, inner);
}

/// Render the directory input with the cursor drawn at its actual position
/// (`text_input.cursor()`) as a reversed cell, rather than a fixed trailing
/// `_`. #2881: the old trailing-underscore hid the real cursor, so after an
/// arrow-key move the caret looked like it was at the end while insertions
/// landed elsewhere.
fn directory_input_line<'a>(state: &'a InitState, theme: &EddaCraftTheme) -> Line<'a> {
    let accent = Style::default().fg(theme.accent());
    let value = &state.text_input.value;
    let cursor = state.text_input.cursor().min(value.len());
    let (before, rest) = value.split_at(cursor);
    let mut rest_chars = rest.chars();
    // The character under the cursor is reversed; at end-of-input the cursor is
    // a reversed space so it stays visible.
    let (under_cursor, after) = match rest_chars.next() {
        Some(c) => (c.to_string(), rest_chars.as_str().to_string()),
        None => (" ".to_string(), String::new()),
    };
    Line::from(vec![
        Span::styled(">> ", accent),
        Span::styled(before.to_string(), accent),
        Span::styled(under_cursor, accent.add_modifier(Modifier::REVERSED)),
        Span::styled(after, accent),
    ])
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_state() -> InitState {
        use super::super::AvailableCheck;

        InitState::new(vec![
            AvailableCheck {
                name: "eslint".to_string(),
                description: "JS/TS linting".to_string(),
                enabled: true,
            },
            AvailableCheck {
                name: "secret-scan".to_string(),
                description: "Detect leaked secrets".to_string(),
                enabled: true,
            },
            AvailableCheck {
                name: "architecture".to_string(),
                description: "Boundary enforcement".to_string(),
                enabled: false,
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
    fn snapshot_mode_step() {
        let backend = TestBackend::new(80, 20);
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
    fn snapshot_format_step() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.step = InitStep::Format;
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
        let snapshot = crate::test_utils::snapshot::buffer_to_string(&buf)
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(snapshot);
    }

    #[test]
    fn snapshot_directory_step() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.step = InitStep::Directory;
        state.text_input.value = "src/app".to_string();
        state.text_input.set_cursor(state.text_input.value.len()); // caret at end, as after typing
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
    fn directory_cursor_renders_at_its_actual_position() {
        // #2881 (problem 2): the caret must sit where the cursor is, not be a
        // fixed trailing marker. With the cursor between "src" and "/app", the
        // reversed cell is on "/", and the visible text is unchanged.
        let mut state = sample_state();
        state.step = InitStep::Directory;
        state.text_input.value = "src/app".to_string();
        state.text_input.set_cursor(3);
        let theme = EddaCraftTheme;

        let line = directory_input_line(&state, &theme);
        let reversed: Vec<&str> = line
            .spans
            .iter()
            .filter(|s| s.style.add_modifier.contains(Modifier::REVERSED))
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(
            reversed,
            ["/"],
            "reversed cursor cell sits on the char at the cursor"
        );

        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, ">> src/app", "the full input text is preserved");
    }

    #[test]
    fn snapshot_checks_step() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.step = InitStep::Checks;
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
    fn snapshot_summary_step() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.step = InitStep::Summary;
        state.config.mode = InitMode::Existing;
        state.config.format = ConfigFormat::Json;
        state.config.directory = "/home/user/project".to_string();
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
}
