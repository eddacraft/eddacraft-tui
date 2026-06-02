//! `Badge` — a small inline label chip (`@eddacraft/render` shadcn built-in).
//!
//! Distinct from [`StatusBadge`](super::status_badge): a `Badge` is a generic
//! coloured chip whose look is driven by a shadcn `variant`
//! (`default`/`secondary`/`destructive`/`outline`), not a pass/fail status. Leaf
//! component; the label is the `children` (or `label`) string prop.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::props::{disp, str_prop};
use crate::json_render::{Props, TuiComponent};
use crate::theme::{EddaCraftTheme, Theme};

/// Renders the `Badge` component.
pub struct Badge;

impl TuiComponent for Badge {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        // Accept either `children` (shadcn) or `label` (common alias).
        let label = disp(props, "children")
            .or_else(|| disp(props, "label"))
            .unwrap_or_default();
        let colour = match str_prop(props, "variant").unwrap_or("default") {
            "destructive" => theme.error(),
            "secondary" => theme.muted(),
            "success" => theme.success(),
            "warning" => theme.warning(),
            _ => theme.accent(),
        };
        let chip = Span::styled(format!(" {label} "), Style::default().fg(colour));
        frame.render_widget(Paragraph::new(Line::from(chip)), area);
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
    fn renders_label_from_children_or_label_alias() {
        for key in ["children", "label"] {
            let p = json!({ key: "v0.3.0" });
            let mut terminal = Terminal::new(TestBackend::new(12, 1)).expect("backend");
            terminal
                .draw(|frame| Badge.render(p.as_object().expect("obj"), frame, frame.area()))
                .expect("draw");
            let text: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect();
            assert!(text.contains("v0.3.0"), "key {key}: {text:?}");
        }
    }

    #[test]
    fn unknown_variant_and_missing_label_do_not_panic() {
        let p = json!({ "variant": "neon" });
        let mut terminal = Terminal::new(TestBackend::new(8, 1)).expect("backend");
        terminal
            .draw(|frame| Badge.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
    }
}
