//! `Heading` — a titled text line (`@eddacraft/render` shadcn built-in).
//!
//! The heading text is the `children` *prop* (a string), distinct from the
//! element-level child list — so `Heading` is a leaf. `level` (1–6) tunes the
//! emphasis: level 1 is the boldest (themed title style), deeper levels dim
//! toward body text.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use super::props::disp_or;
use crate::json_render::{Props, TuiComponent};
use crate::theme::{EddaCraftTheme, Theme};

/// Renders the `Heading` component.
pub struct Heading;

impl TuiComponent for Heading {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let text = disp_or(props, "children", "");
        let level = props
            .get("level")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        // Level 1 uses the themed title style; deeper levels fall back to base
        // text so a sub-heading reads as lighter than a section title.
        let style = if level <= 1 {
            theme.title()
        } else {
            theme.base()
        };
        frame.render_widget(Paragraph::new(Line::styled(text, style)), area);
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
    use serde_json::json;

    fn render(props: &Props) -> String {
        let mut terminal = Terminal::new(TestBackend::new(30, 1)).expect("backend");
        terminal
            .draw(|frame| Heading.render(props, frame, frame.area()))
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[test]
    fn renders_the_children_text() {
        let p = json!({ "children": "Gate Summary", "level": 2 });
        let text = render(p.as_object().expect("obj"));
        assert!(text.contains("Gate Summary"), "got {text:?}");
    }

    #[test]
    fn missing_text_is_blank_not_a_panic() {
        let text = render(&Props::new());
        assert!(text.trim().is_empty());
    }

    #[test]
    fn is_a_leaf() {
        assert!(
            Heading
                .layout_children(&Props::new(), Rect::new(0, 0, 10, 3), 2)
                .is_empty()
        );
    }
}
