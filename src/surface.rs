use ratatui::Frame;
use ratatui::layout::Rect;

use crate::keyboard::Action;
use crate::theme::{EddaCraftTheme, Theme};

/// Trait implemented by every TUI surface, providing a uniform interface
/// for the CLI event loop to render and interact with any screen.
///
/// The type parameter `T` controls which theme the surface renders with.
/// It defaults to [`EddaCraftTheme`] so existing code that writes
/// `impl Surface for MyState` continues to work unchanged.
pub trait Surface<T: Theme = EddaCraftTheme> {
    /// Short name shown in the shell chrome header.
    fn surface_name(&self) -> &'static str;
    /// One-line help text shown in the bottom bar.
    fn help_text(&self) -> &'static str;
    /// Process a mapped keyboard action.
    fn handle_key(&mut self, action: Action);
    /// Whether the surface wants to exit the current surface.
    ///
    /// For the root surface this exits the program; for sub-surfaces it returns
    /// to the parent surface. Use `should_back()` for explicit navigation.
    fn should_quit(&self) -> bool;
    /// Whether the surface wants to go back to the previous screen.
    fn should_back(&self) -> bool {
        false
    }
    /// Reset the surface for re-entry (e.g. after returning from a sub-surface).
    fn reset(&mut self) {}
    /// Render the surface content into the given area.
    fn render(&self, frame: &mut Frame, area: Rect, theme: &T);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::text::Line;
    use ratatui::widgets::Paragraph;

    struct StubSurface {
        quit: bool,
    }

    impl Surface for StubSurface {
        fn surface_name(&self) -> &'static str {
            "Stub"
        }

        fn help_text(&self) -> &'static str {
            "q quit"
        }

        fn handle_key(&mut self, action: Action) {
            if action == Action::Quit {
                self.quit = true;
            }
        }

        fn should_quit(&self) -> bool {
            self.quit
        }

        fn render(&self, frame: &mut Frame, area: Rect, _theme: &EddaCraftTheme) {
            let content = Paragraph::new(Line::raw("stub content"));
            frame.render_widget(content, area);
        }
    }

    #[test]
    fn trait_object_renders_without_panic() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        let surface: Box<dyn Surface> = Box::new(StubSurface { quit: false });

        terminal
            .draw(|frame| {
                surface.render(frame, frame.area(), &theme);
            })
            .unwrap();
    }

    #[test]
    fn trait_object_handles_keys() {
        let mut surface = StubSurface { quit: false };
        assert!(!surface.should_quit());

        surface.handle_key(Action::Quit);
        assert!(surface.should_quit());
    }

    #[test]
    fn trait_object_metadata() {
        let surface = StubSurface { quit: false };
        assert_eq!(surface.surface_name(), "Stub");
        assert_eq!(surface.help_text(), "q quit");
    }
}
