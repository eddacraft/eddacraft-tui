use eddacraft_tui::prelude::*;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{QuickStartOption, WelcomeState};

const LOGO: &str = r"
   _____              .__.__
  /  _  \   _______  _|__|  |
 /  /_\  \ /    \  \/ /  |  |
/    |    \   |  \   /|  |  |__
\____|__  /___|  /\_/ |__|____/
        \/     \/
";

const TAGLINE: &str = "Structural governance for AI-assisted development";

pub fn render(frame: &mut Frame, area: Rect, state: &WelcomeState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(8), // Logo
        Constraint::Length(2), // Tagline
        Constraint::Length(1), // Spacer
        Constraint::Min(6),    // Menu
        Constraint::Length(2), // Help text
    ])
    .split(area);

    // Logo
    let logo = Paragraph::new(Text::raw(LOGO)).style(Style::default().fg(theme.accent()));
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

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(theme.accent())),
        Span::styled(" navigate  ", Style::default().fg(theme.muted())),
        Span::styled("enter", Style::default().fg(theme.accent())),
        Span::styled(" select  ", Style::default().fg(theme.muted())),
        Span::styled("q", Style::default().fg(theme.accent())),
        Span::styled(" quit", Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(help, chunks[4]);
}
