//! Re-export from eddacraft-tui with Anvil-branded shell.

use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

/// Render the Anvil-branded shell chrome around a surface content area.
///
/// Returns the inner `Rect` that the surface should render into.
pub fn render_shell(
    frame: &mut Frame,
    area: Rect,
    surface_name: &str,
    help_text: &str,
    theme: &EddaCraftTheme,
) -> Rect {
    eddacraft_tui::shell::render_shell(frame, area, "Anvil", surface_name, help_text, theme)
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
