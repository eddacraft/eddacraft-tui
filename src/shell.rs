use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellBranding {
    #[default]
    Plain,
    EddaCraft,
    Edda,
    Anvil,
    Custom(&'static str),
}

impl ShellBranding {
    #[must_use]
    pub fn mark(self) -> &'static str {
        match self {
            Self::Plain => "",
            Self::EddaCraft => "[■]",
            Self::Edda => "[=]",
            Self::Anvil => "[‡]",
            Self::Custom(mark) => mark,
        }
    }

    #[must_use]
    pub fn footer_wordmark(self, brand: &str) -> String {
        match self {
            Self::Plain => brand.to_lowercase(),
            Self::EddaCraft => "e d d a c r a f t".to_string(),
            Self::Edda => "e d d a".to_string(),
            Self::Anvil => "a n v i l".to_string(),
            Self::Custom(mark) => mark.to_string(),
        }
    }
}

/// Render branded shell chrome around a surface content area.
///
/// Returns the inner `Rect` that the surface should render into.
#[allow(clippy::too_many_arguments)]
pub fn render_shell(
    frame: &mut Frame,
    area: Rect,
    branding: ShellBranding,
    brand: &str,
    surface_name: &str,
    help_text: &str,
    theme: &impl Theme,
    version: &str,
) -> Rect {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Header
        Constraint::Min(1),    // Content
        Constraint::Length(1), // Footer / help
    ])
    .split(area);

    // Header: "Brand > SurfaceName"
    let mut header_spans = Vec::new();
    let mark = branding.mark();
    if !mark.is_empty() {
        header_spans.push(Span::styled(
            mark,
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ));
        header_spans.push(Span::raw(" "));
    }
    header_spans.push(Span::styled(
        brand,
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    ));
    header_spans.push(Span::styled(" > ", Style::default().fg(theme.muted())));
    header_spans.push(Span::styled(surface_name, Style::default().fg(theme.fg())));

    let header = Paragraph::new(Line::from(header_spans));
    frame.render_widget(header, chunks[0]);

    // Footer: help text (left) + watermark (right).
    // Watermark is prioritised — help text is truncated if needed.
    let footer_mark = if mark.is_empty() { "[ ]" } else { mark };
    let watermark = format!(
        "{footer_mark} {}  v{version}",
        branding.footer_wordmark(brand)
    );
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
                    ShellBranding::Anvil,
                    "anvil",
                    "Watch",
                    "j/k navigate  q quit",
                    &theme,
                    "0.3.0-beta",
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
                    ShellBranding::Anvil,
                    "anvil",
                    "Audit",
                    "h/l panels  q quit",
                    &theme,
                    "0.3.0-beta",
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
                    ShellBranding::Anvil,
                    "anvil",
                    "Gate",
                    "j/k navigate  enter expand  q quit",
                    &theme,
                    "0.3.0-beta",
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
                render_shell(
                    frame,
                    frame.area(),
                    ShellBranding::Anvil,
                    "anvil",
                    "Init",
                    "q quit",
                    &theme,
                    "0.3.0-beta",
                );
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
                render_shell(
                    frame,
                    frame.area(),
                    ShellBranding::Plain,
                    "MyApp",
                    "Home",
                    "q quit",
                    &theme,
                    "1.2.3",
                );
            })
            .unwrap();
    }

    #[test]
    fn uses_passed_version_in_footer() {
        let backend = TestBackend::new(60, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(
                    frame,
                    frame.area(),
                    ShellBranding::Anvil,
                    "anvil",
                    "Home",
                    "q quit",
                    &theme,
                    "9.9.9-test",
                );
            })
            .unwrap();

        let footer: String = (0..60)
            .map(|x| terminal.backend().buffer()[(x, 4)].symbol().to_string())
            .collect();

        assert!(footer.contains("v9.9.9-test"));
    }

    #[test]
    fn plain_branding_omits_logo_mark() {
        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(
                    frame,
                    frame.area(),
                    ShellBranding::Plain,
                    "custom",
                    "Home",
                    "q quit",
                    &theme,
                    "1.2.3",
                );
            })
            .unwrap();

        let header: String = (0..40)
            .map(|x| terminal.backend().buffer()[(x, 0)].symbol().to_string())
            .collect();

        assert!(header.starts_with("custom > Home"));
    }

    #[test]
    fn footer_uses_brand_specific_wordmark() {
        let backend = TestBackend::new(60, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(
                    frame,
                    frame.area(),
                    ShellBranding::Anvil,
                    "anvil",
                    "Home",
                    "q quit",
                    &theme,
                    "1.2.3",
                );
            })
            .unwrap();

        let footer: String = (0..60)
            .map(|x| terminal.backend().buffer()[(x, 4)].symbol().to_string())
            .collect();

        assert!(footer.contains("[‡] a n v i l  v1.2.3"));
    }
}
