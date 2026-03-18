use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

/// Trait implemented by every TUI surface, providing a uniform interface
/// for the CLI event loop to render and interact with any screen.
pub trait Surface {
    /// Short name shown in the shell chrome header.
    fn surface_name(&self) -> &'static str;
    /// One-line help text shown in the bottom bar.
    fn help_text(&self) -> &'static str;
    /// Process a mapped keyboard action.
    fn handle_key(&mut self, action: Action);
    /// Whether the surface wants to exit.
    fn should_quit(&self) -> bool;
    /// Render the surface content into the given area.
    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
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

    #[test]
    fn all_concrete_surfaces_implement_trait() {
        // Verify each concrete surface can be used as a trait object.
        use crate::surfaces::welcome::WelcomeState;
        let welcome = WelcomeState::new();
        let _surface: &dyn Surface = &welcome;
        assert_eq!(_surface.surface_name(), "Welcome");
    }
}
