use eddacraft_tui::prelude::*;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

use super::welcome::{OnboardingChoice, OnboardingWelcomeState};
use crate::shell::inset_content;

// anvil brandmark — same logo used on the standard welcome screen.
const LOGO_LINES: &[&str] = &[
    "████         ████",
    "██             ██",
    "██  █████████  ██",
    "██     ███     ██   a n v i l",
    "██  █████████  ██",
    "██             ██",
    "████         ████",
];

const TAGLINE: &str = "anvil catches architecture drift at save-time";
const SUBTITLE: &str = "Let's get you set up.";

/// Left padding for content.
const PAD: &str = "    ";

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &OnboardingWelcomeState,
    theme: &EddaCraftTheme,
) {
    let area = inset_content(area);
    let item_count = OnboardingChoice::ALL.len();

    // Full mode: 2 lines per item + 1 blank between = item_count * 3 - 1
    // Compact mode: 1 line per item (no descriptions, no blanks)
    let full_menu_height = item_count * 3 - 1;
    // logo(7) + blank(1) + tagline(1) + subtitle(1) + spacer(1) + menu
    let full_content_height = 7 + 1 + 1 + 1 + 1 + full_menu_height;
    #[allow(clippy::cast_possible_truncation)]
    let compact = (full_content_height as u16) > area.height;

    let menu_height = if compact {
        item_count
    } else {
        full_menu_height
    };

    // In compact mode: logo(7) + blank(1) + menu
    // In full mode: logo(7) + blank(1) + tagline(1) + subtitle(1) + spacer(1) + menu
    let content_height = if compact {
        7 + 1 + menu_height
    } else {
        7 + 1 + 1 + 1 + 1 + menu_height
    };

    #[allow(clippy::cast_possible_truncation)]
    let content_h = content_height as u16;
    let top_pad = (area.height.saturating_sub(content_h) / 2).max(1);
    #[allow(clippy::cast_possible_truncation)]
    let menu_h = menu_height as u16;

    let chunks = if compact {
        Layout::vertical([
            Constraint::Length(top_pad),
            Constraint::Length(7),   // Logo
            Constraint::Length(1),   // Blank
            Constraint::Min(menu_h), // Menu items
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length(top_pad),
            Constraint::Length(7),   // Logo
            Constraint::Length(1),   // Blank
            Constraint::Length(1),   // Tagline
            Constraint::Length(1),   // Subtitle
            Constraint::Length(1),   // Spacer
            Constraint::Min(menu_h), // Menu items
        ])
        .split(area)
    };

    // Logo — block art in accent colour, "a n v i l" label in fg+bold
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

    // Tagline + subtitle (hidden in compact mode), then menu
    let menu_chunk_idx = if compact {
        3
    } else {
        let tagline = Paragraph::new(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled(TAGLINE, Style::default().fg(theme.muted())),
        ]));
        frame.render_widget(tagline, chunks[3]);

        let subtitle = Paragraph::new(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled(SUBTITLE, Style::default().fg(theme.fg())),
        ]));
        frame.render_widget(subtitle, chunks[4]);

        // chunks[5] is the spacer — nothing to render
        6
    };

    let menu_lines = build_menu_lines(state, theme, compact);
    let menu = Paragraph::new(Text::from(menu_lines));
    frame.render_widget(menu, chunks[menu_chunk_idx]);
}

fn build_menu_lines<'a>(
    state: &'a OnboardingWelcomeState,
    theme: &'a EddaCraftTheme,
    compact: bool,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    for (i, opt) in OnboardingChoice::ALL.iter().enumerate() {
        if !compact && i > 0 {
            lines.push(Line::raw(""));
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

        lines.push(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled(indicator, label_style),
            Span::styled(opt.label(), label_style),
        ]));

        if !compact {
            let desc_style = Style::default().fg(theme.muted());
            lines.push(Line::from(vec![
                Span::styled(PAD, Style::default()),
                Span::styled("      ", Style::default()),
                Span::styled(opt.description(), desc_style),
            ]));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_without_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = OnboardingWelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_in_small_area() {
        let backend = ratatui::backend::TestBackend::new(40, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = OnboardingWelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_second_item_selected() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = OnboardingWelcomeState::new();
        state.selected = 1;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_third_item_selected() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = OnboardingWelcomeState::new();
        state.selected = 2;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}
