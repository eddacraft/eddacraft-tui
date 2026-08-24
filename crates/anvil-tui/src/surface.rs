//! Re-export Surface trait from eddacraft-tui, plus the anvil-internal
//! pointer extension.

pub use eddacraft_tui::surface::Surface;

/// Anvil-internal extension for surfaces that consume raw mouse events
/// (scroll-zoom, drag-pan, click-select — the flow-graph interactions).
///
/// Deliberately *not* part of the stable `eddacraft-tui` [`Surface`]
/// contract: adding a method to that downstream-implemented trait is a
/// breaking change (the ADR-115 lesson). The anvil event loop opts a
/// surface into mouse capture only through
/// `tui::run_pointer_surface_with_exit`, so key-only surfaces pay nothing.
pub trait PointerSurface: Surface {
    /// Process one raw mouse event. Coordinates are terminal-absolute;
    /// widgets that track their render area (rataflow does) map them
    /// internally.
    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent);
}
