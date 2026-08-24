//! Viewport capture/restore and fit-then-read (TUIN-016).

use rataflow::{Background, Flow, Viewport};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// Camera and selection to restore after a graph rebuild.
///
/// # Stability
///
/// **experimental** (TUIN-016).
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    /// Pan/zoom at capture time.
    pub viewport: Viewport,
    /// First selected node id, if any.
    pub selected: Option<String>,
}

/// Snapshot pan, zoom, and selection.
///
/// # Stability
///
/// **experimental** (TUIN-016).
#[must_use]
pub fn capture_view(flow: &Flow) -> ViewState {
    ViewState {
        viewport: flow.viewport,
        selected: flow.first_selected_node_id(),
    }
}

/// Restore a captured view. Selection is skipped when the node is gone.
///
/// # Stability
///
/// **experimental** (TUIN-016).
pub fn restore_view(flow: &mut Flow, state: &ViewState) {
    flow.viewport = state.viewport;
    if let Some(id) = &state.selected
        && flow.node(id).is_some()
    {
        flow.select_node(id);
    }
}

/// Rebuild a graph while keeping the previous camera and selection.
///
/// # Stability
///
/// **experimental** (TUIN-016).
pub fn rebuild_preserving_view(previous: &Flow, build: impl FnOnce() -> Flow) -> Flow {
    let state = capture_view(previous);
    let mut next = build();
    restore_view(&mut next, &state);
    next
}

/// Centre the given node and snap the viewport to 1:1 so its label is
/// legible even in a graph dense enough for semantic zoom to hide text.
///
/// Call after at least one frame has rendered — a pending fit-view request
/// is applied at render time and would override this zoom. Prefer
/// [`zoom_to_read_after_layout`] when fit-view may still be pending.
///
/// # Stability
///
/// **experimental** (TUIN-014).
pub fn zoom_to_read(flow: &mut Flow, node_id: &str) {
    flow.select_node(node_id);
    flow.center_on_selected();
    flow.zoom_to(1.0);
}

/// Apply deferred fit-view by rendering into an off-screen buffer, then
/// [`zoom_to_read`]. Use this instead of calling `zoom_to_read` before the
/// first real frame.
///
/// # Stability
///
/// **experimental** (TUIN-016).
pub fn zoom_to_read_after_layout(flow: &mut Flow, node_id: &str, width: u16, height: u16) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("infallible");
    let _ = terminal.draw(|frame| {
        frame.render_widget(Background::new(&*flow), frame.area());
        frame.render_widget(&mut *flow, frame.area());
    });
    zoom_to_read(flow, node_id);
}
