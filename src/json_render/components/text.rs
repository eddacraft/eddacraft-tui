//! `Text` — a body-text paragraph (`@eddacraft/render` shadcn built-in).
//!
//! Renders the `children` string prop as wrapped body text. A `variant` of
//! `"muted"` dims it; anything else uses the base text style. Leaf component.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Paragraph, Wrap};

use super::props::{str_or, str_prop};
use crate::json_render::{Props, TuiComponent};
use crate::theme::{EddaCraftTheme, Theme};

/// Renders the `Text` component.
pub struct Text;

impl TuiComponent for Text {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let body = str_or(props, "children", "");
        let style = if str_prop(props, "variant") == Some("muted") {
            Style::default().fg(theme.muted())
        } else {
            theme.base()
        };
        frame.render_widget(
            Paragraph::new(body.to_owned())
                .style(style)
                .wrap(Wrap { trim: true }),
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
    use serde_json::json;

    #[test]
    fn wraps_and_renders_body_text() {
        let p = json!({ "children": "a longer sentence that should wrap across lines" });
        let mut terminal = Terminal::new(TestBackend::new(12, 5)).expect("backend");
        terminal
            .draw(|frame| Text.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("longer"), "body text rendered: {text:?}");
    }

    #[test]
    fn empty_and_leaf() {
        let mut terminal = Terminal::new(TestBackend::new(10, 2)).expect("backend");
        terminal
            .draw(|frame| Text.render(&Props::new(), frame, frame.area()))
            .expect("draw");
        assert!(
            Text.layout_children(&Props::new(), Rect::new(0, 0, 10, 3), 1)
                .is_empty()
        );
    }
}
