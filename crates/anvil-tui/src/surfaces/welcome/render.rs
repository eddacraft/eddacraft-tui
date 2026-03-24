use eddacraft_tui::prelude::*;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{QuickStartOption, WelcomeState};

const LOGO_LINES: &[&str] = &[
    "\u{2588}\u{2588}\u{2588}\u{2588}     \u{2588}\u{2588}\u{2588}\u{2588}",
    "\u{2588}\u{2588}         \u{2588}\u{2588}",
    "\u{2588}\u{2588}  \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}",
    "\u{2588}\u{2588}         \u{2588}\u{2588}   a n v i l",
    "\u{2588}\u{2588}  \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}",
    "\u{2588}\u{2588}         \u{2588}\u{2588}",
    "\u{2588}\u{2588}\u{2588}\u{2588}     \u{2588}\u{2588}\u{2588}\u{2588}",
];

const TAGLINE: &str = "Structural governance for AI-assisted development";

pub fn render(frame: &mut Frame, area: Rect, state: &WelcomeState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(9), // Logo
        Constraint::Length(2), // Tagline
        Constraint::Length(1), // Spacer
        Constraint::Min(6),    // Menu
    ])
    .split(area);

    // Logo — block art in EMBER, "a n v i l" text in FG
    let logo_lines: Vec<Line> = LOGO_LINES
        .iter()
        .map(|line| {
            if line.contains("a n v i l") {
                let parts: Vec<&str> = line.splitn(2, "a n v i l").collect();
                Line::from(vec![
                    Span::styled(parts[0], Style::default().fg(theme.accent())),
                    Span::styled(
                        "a n v i l",
                        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::styled(*line, Style::default().fg(theme.accent()))
            }
        })
        .collect();
    let logo = Paragraph::new(Text::from(logo_lines));
    frame.render_widget(logo, chunks[0]);

    // Tagline
    let tagline = Paragraph::new(TAGLINE).style(Style::default().fg(theme.muted()));
    frame.render_widget(tagline, chunks[1]);

    // Menu
    let menu_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.muted()))
        .title(" Quick Start ");

    let menu_area = menu_block.inner(chunks[3]);
    frame.render_widget(menu_block, chunks[3]);

    let items: Vec<Line> = QuickStartOption::ALL
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let indicator = if i == state.selected { ">> " } else { "  " };
            let label_style = if i == state.selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };
            let desc_style = Style::default().fg(theme.muted());

            Line::from(vec![
                Span::styled(indicator, label_style),
                Span::styled(opt.label(), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(opt.description(), desc_style),
            ])
        })
        .collect();

    let menu = Paragraph::new(Text::from(items));
    frame.render_widget(menu, menu_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn snapshot_default_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    "Welcome",
                    "j/k navigate  enter select  q quit",
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_second_item_selected() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = WelcomeState::new();
        state.selected = 1;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    "Welcome",
                    "j/k navigate  enter select  q quit",
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
        let backend = TestBackend::new(40, 16);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}
