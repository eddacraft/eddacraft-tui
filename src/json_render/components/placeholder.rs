//! `Placeholder` — an explicit "not available in terminal" component.
//!
//! Per D-TUIDASH-001, some catalogue components have no meaningful terminal
//! rendering (e.g. `HeatMap`). Rather than leaving them unregistered — where the
//! tree renderer's unknown-component fallback would still draw a placeholder —
//! they are registered with this component so the registry *explicitly* declares
//! the type as known-but-unsupported. That keeps catalogue-parity diagnostics
//! (TUIDASH-010) honest: the type is mapped, it just degrades by design.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::json_render::{Props, TuiComponent};

/// A component that renders a labelled "not available in terminal" marker.
pub struct Placeholder {
    component_name: &'static str,
}

impl Placeholder {
    /// Create a placeholder for the named component type.
    #[must_use]
    pub fn new(component_name: &'static str) -> Self {
        Self { component_name }
    }
}

impl TuiComponent for Placeholder {
    fn render(&self, _props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let label = format!("[{}: not available in terminal]", self.component_name);
        frame.render_widget(
            Paragraph::new(Line::raw(label)).style(Style::default().fg(Color::DarkGray)),
            area,
        );
    }

    fn layout_children(&self, _props: &Props, _area: Rect, _child_count: usize) -> Vec<Rect> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    #[test]
    fn renders_named_placeholder() {
        let mut terminal = Terminal::new(TestBackend::new(40, 1)).expect("backend");
        terminal
            .draw(|frame| Placeholder::new("HeatMap").render(&Props::new(), frame, frame.area()))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            text.contains("[HeatMap: not available in terminal]"),
            "got {text:?}"
        );
    }
}
