use eddacraft_tui::prelude::*;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

use super::{QuickStartOption, WelcomeState};

// Anvil brandmark — faithful to logos/svg/anvil-brandmark-white.svg
// Two L-shaped corner brackets framing a central anvil body
// (two horizontal bars connected by a short vertical column).
const LOGO_LINES: &[&str] = &[
    "████         ████",
    "██             ██",
    "██  █████████  ██",
    "██     ███     ██   a n v i l",
    "██  █████████  ██",
    "██             ██",
    "████         ████",
];

const TAGLINE: &str = "Structural governance for AI-assisted development";

/// Left padding for content within the welcome screen.
const PAD: &str = "    ";

pub fn render(frame: &mut Frame, area: Rect, state: &WelcomeState, theme: &EddaCraftTheme) {
    // Content heights: logo(7) + blank(1) + tagline(1) + spacer(2) + menu(3*N-1)
    let menu_item_count = QuickStartOption::ALL.len();
    let menu_height = menu_item_count * 3 - 1; // 2 lines per item + 1 blank between
    let content_height = 7 + 1 + 1 + 2 + menu_height;

    // Centre vertically — at least 1 row gap from header
    #[allow(clippy::cast_possible_truncation)]
    let content_h = content_height as u16;
    let top_pad = (area.height.saturating_sub(content_h) / 2).max(1);
    #[allow(clippy::cast_possible_truncation)]
    let menu_h = menu_height as u16;

    let chunks = Layout::vertical([
        Constraint::Length(top_pad), // Top padding
        Constraint::Length(7),       // Logo
        Constraint::Length(1),       // Blank
        Constraint::Length(1),       // Tagline
        Constraint::Length(2),       // Spacer
        Constraint::Min(menu_h),     // Menu items (flexible — absorbs overflow)
    ])
    .split(area);

    // Logo — block art in EMBER, "a n v i l" text in FG
    let logo_lines: Vec<Line> = LOGO_LINES
        .iter()
        .map(|line| {
            if let Some((before, _)) = line.split_once("a n v i l") {
                Line::from(vec![
                    Span::styled(PAD, Style::default()),
                    Span::styled(before, Style::default().fg(theme.accent())),
                    Span::styled(
                        "a n v i l",
                        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(PAD, Style::default()),
                    Span::styled(*line, Style::default().fg(theme.accent())),
                ])
            }
        })
        .collect();
    let logo = Paragraph::new(Text::from(logo_lines));
    frame.render_widget(logo, chunks[1]);

    // Tagline
    let tagline = Paragraph::new(Line::from(vec![
        Span::styled(PAD, Style::default()),
        Span::styled(TAGLINE, Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(tagline, chunks[3]);

    // Menu items — spaced with blank lines between
    let mut menu_lines: Vec<Line> = Vec::new();
    for (i, opt) in QuickStartOption::ALL.iter().enumerate() {
        if i > 0 {
            menu_lines.push(Line::raw(""));
        }
        let selected = i == state.selected;
        let indicator = if selected { " \u{25b8} " } else { "   " };
        let label_style = if selected {
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg())
        };
        let desc_style = Style::default().fg(theme.muted());

        menu_lines.push(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled(indicator, label_style),
            Span::styled(opt.label(), label_style),
        ]));
        menu_lines.push(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled("      ", Style::default()),
            Span::styled(opt.description(), desc_style),
        ]));
    }

    // Append status message below menu items if present
    if let Some(ref msg) = state.status_message {
        menu_lines.push(Line::raw(""));
        menu_lines.push(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled(format!("   {msg}"), Style::default().fg(theme.muted())),
        ]));
    }

    let menu = Paragraph::new(Text::from(menu_lines));
    frame.render_widget(menu, chunks[5]);
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
                    crate::surface::Surface::surface_name(&state),
                    crate::surface::Surface::help_text(&state),
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
                    crate::surface::Surface::surface_name(&state),
                    crate::surface::Surface::help_text(&state),
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
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}
