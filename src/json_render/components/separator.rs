//! `Separator` — a horizontal rule (`@eddacraft/render` shadcn built-in).
//!
//! Maps to the eddacraft-tui [`Divider`] widget: a single themed line. It is a
//! leaf component — any children are ignored.

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::json_render::{Props, TuiComponent};
use crate::theme::EddaCraftTheme;
use crate::widgets::divider::{Divider, DividerVariant};

/// Renders the `Separator` component.
pub struct Separator;

impl TuiComponent for Separator {
    fn render(&self, _props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        // The divider draws on a single line; render it on the area's first row
        // so a generous vertical track around it stays blank rather than tiling
        // multiple rules.
        let line = Rect::new(area.x, area.y, area.width, 1);
        frame.render_widget(Divider::new(&theme).variant(DividerVariant::Light), line);
    }

    fn layout_children(&self, _props: &Props, _area: Rect, _child_count: usize) -> Vec<Rect> {
        // Leaf: a rule has no children.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use serde_json::json;

    fn props(v: serde_json::Value) -> Props {
        match v {
            serde_json::Value::Object(map) => map,
            _ => panic!("test props must be a JSON object"),
        }
    }

    #[test]
    fn is_a_leaf() {
        assert!(
            Separator
                .layout_children(&props(json!({})), Rect::new(0, 0, 20, 5), 3)
                .is_empty()
        );
    }

    #[test]
    fn draws_a_rule_without_panic() {
        let mut terminal = Terminal::new(TestBackend::new(10, 3)).expect("backend");
        terminal
            .draw(|frame| Separator.render(&props(json!({})), frame, frame.area()))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        // The first row should carry rule glyphs (non-space); rows below stay blank.
        let first_row: String = (0..10)
            .map(|x| {
                buffer
                    .cell((x, 0))
                    .map_or(" ", ratatui::buffer::Cell::symbol)
                    .to_owned()
            })
            .collect();
        assert!(first_row.trim().chars().count() > 0, "rule drawn on row 0");
    }

    #[test]
    fn zero_area_does_not_panic() {
        let mut terminal = Terminal::new(TestBackend::new(4, 2)).expect("backend");
        terminal
            .draw(|frame| Separator.render(&props(json!({})), frame, Rect::new(0, 0, 0, 0)))
            .expect("draw");
    }
}
