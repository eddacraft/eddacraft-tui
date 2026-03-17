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
