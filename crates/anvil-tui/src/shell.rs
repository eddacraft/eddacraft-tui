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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;

    fn style_annotation(cell: &ratatui::buffer::Cell) -> String {
        let has_fg = cell.fg != Color::Reset;
        let has_bg = cell.bg != Color::Reset;
        let has_mod = !cell.modifier.is_empty();

        if !has_fg && !has_bg && !has_mod {
            return String::new();
        }

        let mut parts: Vec<String> = Vec::new();
        if has_fg {
            parts.push(format!("fg:{}", cell.fg));
        }
        if has_bg {
            parts.push(format!("bg:{}", cell.bg));
        }
        if cell.modifier.contains(Modifier::BOLD) {
            parts.push("bold".into());
        }
        if cell.modifier.contains(Modifier::DIM) {
            parts.push("dim".into());
        }
        if cell.modifier.contains(Modifier::ITALIC) {
            parts.push("italic".into());
        }
        if cell.modifier.contains(Modifier::UNDERLINED) {
            parts.push("underlined".into());
        }
        if cell.modifier.contains(Modifier::REVERSED) {
            parts.push("reversed".into());
        }
        if cell.modifier.contains(Modifier::CROSSED_OUT) {
            parts.push("crossed_out".into());
        }
        format!("[{}]", parts.join(","))
    }

    fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
        let area = buf.area;
        let mut output = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = &buf[(x, y)];
                output.push_str(cell.symbol());
                output.push_str(&style_annotation(cell));
            }
            output.push('\n');
        }
        output
    }

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
                inner =
                    render_shell(frame, frame.area(), "Audit", "h/l panels  q quit", &theme);
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
