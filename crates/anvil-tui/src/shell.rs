use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

const LOGO_LINES: [&str; 7] = [
    "\u{2588}\u{2588}\u{2588}\u{2588}     \u{2588}\u{2588}\u{2588}\u{2588}",
    "\u{2588}\u{2588}         \u{2588}\u{2588}",
    "\u{2588}\u{2588}  \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}",
    "\u{2588}\u{2588}         \u{2588}\u{2588}",
    "\u{2588}\u{2588}  \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}",
    "\u{2588}\u{2588}         \u{2588}\u{2588}",
    "\u{2588}\u{2588}\u{2588}\u{2588}     \u{2588}\u{2588}\u{2588}\u{2588}",
];

/// Key tokens that should be highlighted in help text.
const KEY_TOKENS: &[&str] = &[
    "enter/space",
    "enter",
    "space",
    "esc",
    "PgUp/PgDn",
    "j/k",
    "h/l",
    "n/N",
    "a/f/p/s/w",
    "/",
    "q",
];

/// Render the global shell chrome (header + footer) and return the core
/// `Rect` that the active surface should render into.
pub fn render_shell(
    frame: &mut Frame,
    area: Rect,
    surface_name: &str,
    help_text: &str,
    theme: &EddaCraftTheme,
) -> Rect {
    let layout = Layout::vertical([
        Constraint::Length(9), // Header
        Constraint::Min(10),   // Core (returned)
        Constraint::Length(5), // Footer
    ])
    .split(area);

    render_header(frame, layout[0], surface_name, theme);
    render_footer(frame, layout[2], help_text, theme);

    layout[1]
}

fn render_header(frame: &mut Frame, area: Rect, surface_name: &str, theme: &EddaCraftTheme) {
    // 1 line top padding, 7 logo lines, 1 line bottom padding
    let rows = Layout::vertical([
        Constraint::Length(1), // top padding
        Constraint::Length(7), // logo
        Constraint::Length(1), // bottom padding
    ])
    .split(area);

    let accent_style = Style::default().fg(theme.accent());

    let lines: Vec<Line> = LOGO_LINES
        .iter()
        .enumerate()
        .map(|(i, logo_text)| {
            if i == 3 {
                // Line 4 (0-indexed 3): logo + "a n v i l   //   <surface_name>"
                Line::from(vec![
                    Span::styled(*logo_text, accent_style),
                    Span::styled("   ", Style::default()),
                    Span::styled(
                        "a n v i l   ",
                        Style::default()
                            .fg(theme.fg())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("//   ", Style::default().fg(theme.muted())),
                    Span::styled(surface_name, Style::default().fg(theme.muted())),
                ])
            } else {
                Line::from(Span::styled(*logo_text, accent_style))
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), rows[1]);
}

fn render_footer(frame: &mut Frame, area: Rect, help_text: &str, theme: &EddaCraftTheme) {
    let columns = Layout::horizontal([
        Constraint::Percentage(80), // help
        Constraint::Percentage(20), // watermark
    ])
    .split(area);

    // Left: help text with highlighted keys
    let help_rows = Layout::vertical([
        Constraint::Length(1), // top padding
        Constraint::Min(1),    // help content
    ])
    .split(columns[0]);

    let help_spans = parse_help_spans(help_text, theme);
    frame.render_widget(Paragraph::new(Line::from(help_spans)), help_rows[1]);

    // Right: EddaCraft watermark
    let watermark_lines = vec![
        Line::from(vec![
            Span::styled("  [ ", Style::default().fg(theme.border())),
            Span::styled(
                "\u{25a0}",
                Style::default().fg(theme.border()),
            ),
            Span::styled(" ] ", Style::default().fg(theme.border())),
            Span::styled("e d d a c r a f t", Style::default().fg(theme.muted())),
        ]),
        Line::from(Span::styled(
            "        v0.9.2-beta",
            Style::default().fg(theme.border()),
        )),
    ];

    frame.render_widget(
        Paragraph::new(watermark_lines).alignment(Alignment::Right),
        columns[1],
    );
}

/// Parse help text and highlight key tokens in accent colour.
fn parse_help_spans<'a>(text: &'a str, theme: &EddaCraftTheme) -> Vec<Span<'a>> {
    let accent_style = Style::default().fg(theme.accent());
    let muted_style = Style::default().fg(theme.muted());

    let mut spans = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the earliest key token match
        let mut earliest_pos = remaining.len();
        let mut earliest_token = "";

        for &token in KEY_TOKENS {
            if let Some(pos) = remaining.find(token) {
                // Ensure the token is at a word boundary (preceded by space or start)
                let at_boundary = pos == 0
                    || remaining.as_bytes()[pos - 1] == b' '
                    || remaining.as_bytes()[pos - 1] == b'/';

                // Ensure the token ends at a word boundary (followed by space or end)
                let end = pos + token.len();
                let ends_at_boundary = end == remaining.len()
                    || remaining.as_bytes()[end] == b' ';

                if at_boundary && ends_at_boundary && pos < earliest_pos {
                    earliest_pos = pos;
                    earliest_token = token;
                }
            }
        }

        if earliest_token.is_empty() {
            // No more tokens found
            spans.push(Span::styled(remaining, muted_style));
            break;
        }

        // Push text before the token
        if earliest_pos > 0 {
            spans.push(Span::styled(&remaining[..earliest_pos], muted_style));
        }

        // Push the highlighted token
        spans.push(Span::styled(earliest_token, accent_style));

        // Advance past the token
        remaining = &remaining[earliest_pos + earliest_token.len()..];
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_help_highlights_keys() {
        let theme = EddaCraftTheme;
        let spans = parse_help_spans("j/k navigate  enter select  q quit", &theme);
        // Should have: "j/k", " navigate  ", "enter", " select  ", "q", " quit"
        assert!(spans.len() >= 6);
        // First span is "j/k" in accent
        assert_eq!(spans[0].content, "j/k");
        assert_eq!(spans[0].style.fg, Some(theme.accent()));
    }

    #[test]
    fn parse_help_empty_string() {
        let theme = EddaCraftTheme;
        let spans = parse_help_spans("", &theme);
        assert!(spans.is_empty());
    }

    #[test]
    fn logo_lines_count() {
        assert_eq!(LOGO_LINES.len(), 7);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::snapshot::buffer_to_string;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(frame, frame.area(), "Watch", "j/k navigate  q quit", &theme);
            })
            .unwrap();
    }

    #[test]
    fn returns_inner_area() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        let mut inner = Rect::default();
        terminal
            .draw(|frame| {
                inner = render_shell(frame, frame.area(), "Audit", "h/l panels  q quit", &theme);
            })
            .unwrap();

        // Inner area should be smaller than the full area (header + footer = 2 rows)
        assert_eq!(inner.height, 22);
        assert_eq!(inner.width, 80);
        assert_eq!(inner.y, 1);
    }

    #[test]
    fn snapshot_shell_chrome() {
        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(
                    frame,
                    frame.area(),
                    "Gate",
                    "j/k navigate  enter expand  q quit",
                    &theme,
                );
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn renders_in_small_area() {
        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(frame, frame.area(), "Init", "q quit", &theme);
            })
            .unwrap();
    }
}
