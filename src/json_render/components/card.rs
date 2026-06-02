//! `Card` — a bordered, optionally-titled container (`@eddacraft/render` shadcn
//! built-in; also the home for the old spec's `Section`).
//!
//! Maps to the eddacraft-tui [`Container`] widget: a themed [`Block`] with a
//! border and optional title. Children render inside the border, stacked
//! vertically.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use super::props::disp;
use crate::json_render::{Props, TuiComponent};
use crate::theme::EddaCraftTheme;
use crate::widgets::container::{Container, ContainerVariant};

/// Renders the `Card` component.
pub struct Card;

impl TuiComponent for Card {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        let theme = EddaCraftTheme;
        // Sanitise the title — it is rendered into the terminal border.
        let title = disp(props, "title");
        let mut container = Container::new(&theme).variant(ContainerVariant::Secondary);
        if let Some(title) = &title {
            container = container.title(title);
        }
        frame.render_widget(container.to_block(), area);
    }

    fn layout_children(&self, _props: &Props, area: Rect, child_count: usize) -> Vec<Rect> {
        let theme = EddaCraftTheme;
        // Children live inside the border, not over it.
        let inner = Container::new(&theme).inner(area);
        if child_count == 0 || inner.width == 0 || inner.height == 0 {
            return Vec::new();
        }
        Layout::vertical(vec![Constraint::Fill(1); child_count])
            .split(inner)
            .to_vec()
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
    fn children_are_laid_out_inside_the_border() {
        let area = Rect::new(0, 0, 20, 10);
        let rects = Card.layout_children(&props(json!({})), area, 2);
        assert_eq!(rects.len(), 2);
        // Inside the border: indented from the area edges on every side.
        for r in &rects {
            assert!(r.x > area.x, "inset from left border");
            assert!(r.x + r.width < area.x + area.width, "inset from right");
            assert!(r.y > area.y, "below the top border");
        }
    }

    #[test]
    fn renders_title_in_the_border_without_panic() {
        let spec_props = props(json!({ "title": "Checks" }));
        let mut terminal = Terminal::new(TestBackend::new(20, 5)).expect("backend");
        terminal
            .draw(|frame| Card.render(&spec_props, frame, frame.area()))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            text.contains("Checks"),
            "title appears in the border: {text:?}"
        );
    }

    #[test]
    fn untitled_card_and_tiny_area_do_not_panic() {
        let mut terminal = Terminal::new(TestBackend::new(2, 1)).expect("backend");
        terminal
            .draw(|frame| Card.render(&props(json!({})), frame, frame.area()))
            .expect("draw");
        // A 2x1 area has no inner space — no child rects, no panic.
        assert!(
            Card.layout_children(&props(json!({})), Rect::new(0, 0, 2, 1), 3)
                .is_empty()
        );
    }
}
