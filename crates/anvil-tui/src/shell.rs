//! Anvil-branded shell chrome with correct binary version.

use eddacraft_tui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

/// The binary's own version, from the workspace `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Render the Anvil-branded shell chrome around a surface content area.
///
/// Returns the inner `Rect` that the surface should render into.
pub fn render_shell(
    frame: &mut Frame,
    area: Rect,
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

    // Footer: help text (left) + watermark (right).
    let watermark = format!("[ \u{25a0} ] e d d a c r a f t  v{VERSION}");
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
    use eddacraft_tui::theme::EddaCraftTheme;
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

    #[test]
    fn version_matches_workspace() {
        let watermark = format!("v{VERSION}");
        assert!(
            watermark.starts_with("v0.3."),
            "expected workspace version, got: {watermark}"
        );
    }
}
