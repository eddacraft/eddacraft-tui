use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{TutorialPhase, TutorialState};

pub fn render(frame: &mut Frame, area: Rect, state: &TutorialState, theme: &EddaCraftTheme) {
    match state.phase {
        TutorialPhase::PathSelect => {
            render_path_select(frame, area, state, theme);
        }
        TutorialPhase::Running => {
            let chunks = Layout::vertical([
                Constraint::Length(3), // Progress indicator
                Constraint::Min(6),    // Content
            ])
            .split(area);

            render_step_progress(frame, chunks[0], state, theme);
            render_step_content(frame, chunks[1], state, theme);
        }
        TutorialPhase::Complete => {
            render_complete(frame, area, state, theme);
        }
    }
}

fn render_path_select(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Choose a Tutorial Path ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<Line> = state
        .paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let selected = i == state.path_selected;
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
                Span::styled(path.label(), name_style),
                Span::styled(
                    format!("  {}", path.description()),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_step_progress(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    let path_label = state.chosen_path.map_or("Tutorial", TutorialPath::label);

    let total = state.steps.len();
    let completed = state.steps.iter().filter(|s| s.completed).count();

    let spans: Vec<Span> = state
        .steps
        .iter()
        .enumerate()
        .flat_map(|(i, _step)| {
            let style = match i.cmp(&state.current_step) {
                std::cmp::Ordering::Less => Style::default().fg(theme.success()),
                std::cmp::Ordering::Equal => Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
                std::cmp::Ordering::Greater => Style::default().fg(theme.muted()),
            };
            let marker = match i.cmp(&state.current_step) {
                std::cmp::Ordering::Less => "*",
                std::cmp::Ordering::Equal => ">",
                std::cmp::Ordering::Greater => "o",
            };
            let separator = if i < total - 1 { " - " } else { "" };
            vec![
                Span::styled(marker, style),
                Span::styled(separator, Style::default().fg(theme.muted())),
            ]
        })
        .collect();

    let lines = vec![
        Line::from(Span::styled(
            format!("{path_label}  ({completed}/{total})"),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(spans),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_step_content(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    let Some(step) = state.steps.get(state.current_step) else {
        return;
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(format!(" {} ", step.title));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::default(),
        Line::from(Span::styled(
            &step.description,
            Style::default().fg(theme.fg()),
        )),
        Line::default(),
        Line::from(Span::styled(
            &step.instruction,
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_complete(frame: &mut Frame, area: Rect, state: &TutorialState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.success()))
        .title(" Well Done ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let path_label = state
        .chosen_path
        .map_or("the tutorial", TutorialPath::label);
    let total = state.steps.len();

    let lines = vec![
        Line::default(),
        Line::from(Span::styled(
            format!("You completed all {total} steps of the {path_label} tutorial."),
            Style::default().fg(theme.fg()),
        )),
        Line::default(),
        Line::from(Span::styled(
            "Press enter to choose another tutorial path, or q to quit.",
            Style::default().fg(theme.accent()),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

use super::TutorialPath;

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
        let area = buf.area;
        let mut output = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                output.push_str(buf[(x, y)].symbol());
            }
            output.push('\n');
        }
        output
    }

    #[test]
    fn renders_without_panic_path_select() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = TutorialState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn snapshot_path_select() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = TutorialState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_running_phase() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Policy);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_complete_phase() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Drift);
        // Complete all steps
        for step in &mut state.steps {
            step.completed = true;
        }
        state.phase = TutorialPhase::Complete;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn renders_in_small_area() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = TutorialState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}
