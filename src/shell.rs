use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

/// Render branded shell chrome around a surface content area.
///
/// Returns the inner `Rect` that the surface should render into.
pub fn render_shell(
    frame: &mut Frame,
    area: Rect,
    brand: &str,
    surface_name: &str,
    help_text: &str,
    theme: &impl Theme,
) -> Rect {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Header
        Constraint::Min(1),    // Content
        Constraint::Length(1), // Footer / help
    ])
    .split(area);

    // Header: "Brand > SurfaceName"
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            brand,
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" > ", Style::default().fg(theme.muted())),
        Span::styled(surface_name, Style::default().fg(theme.fg())),
    ]));
    frame.render_widget(header, chunks[0]);

    // Footer: help text (left) + watermark (right).
    // Watermark is prioritised — help text is truncated if needed.
    let version = env!("CARGO_PKG_VERSION");
    let watermark = format!("[ \u{25a0} ] e d d a c r a f t  v{version}");
    let wm_width = watermark.width();
    let available = chunks[2].width as usize;
    let min_gap = 2;
    let max_help = available.saturating_sub(wm_width + min_gap);
    let help_display: String = if help_text.width() > max_help {
        let mut truncated = String::new();
        let mut w = 0;
        for ch in help_text.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if w + cw > max_help {
                break;
            }
            truncated.push(ch);
            w += cw;
        }
        truncated
    } else {
        help_text.to_string()
    };
    let padding = available.saturating_sub(help_display.width() + wm_width);
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(help_display, Style::default().fg(theme.muted())),
        Span::raw(" ".repeat(padding)),
        Span::styled(watermark, Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(footer, chunks[2]);

    chunks[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::snapshot::buffer_to_string;
    use crate::theme::EddaCraftTheme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(
                    frame,
                    frame.area(),
                    "Anvil",
                    "Watch",
                    "j/k navigate  q quit",
                    &theme,
                );
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
                inner = render_shell(
                    frame,
                    frame.area(),
                    "Anvil",
                    "Audit",
                    "h/l panels  q quit",
                    &theme,
                );
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
                    "Anvil",
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
                render_shell(frame, frame.area(), "Anvil", "Init", "q quit", &theme);
            })
            .unwrap();
    }

    #[test]
    fn custom_brand_name() {
        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(frame, frame.area(), "MyApp", "Home", "q quit", &theme);
            })
            .unwrap();
    }
}
