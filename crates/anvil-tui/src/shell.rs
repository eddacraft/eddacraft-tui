use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Render the branded shell chrome around a surface content area.
///
/// Returns the inner `Rect` that the surface should render into.
pub fn render_shell(
    frame: &mut Frame,
    area: Rect,
    surface_name: &str,
    help_text: &str,
    theme: &EddaCraftTheme,
) -> Rect {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Header
        Constraint::Min(1),    // Content
        Constraint::Length(1), // Footer / help
    ])
    .split(area);

    // Header: "Anvil > SurfaceName"
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "Anvil",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" > ", Style::default().fg(theme.muted())),
        Span::styled(surface_name, Style::default().fg(theme.fg())),
    ]));
    frame.render_widget(header, chunks[0]);

    // Footer: help text
    let footer = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(theme.muted()),
    )));
    frame.render_widget(footer, chunks[2]);

    chunks[1]
}
