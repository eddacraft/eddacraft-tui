//! `Alert` — a bordered callout with a severity colour (`@eddacraft/render`
//! shadcn built-in).
//!
//! Severity comes from `type` (or `variant`): `info` (default), `warning`,
//! `error`/`destructive`, `success`. `title` styles the border title; the body
//! is the `children` (or `description`) string. Leaf component.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use super::props::{disp, disp_or, str_prop};
use crate::json_render::{Props, TuiComponent};
use crate::theme::{EddaCraftTheme, Theme};

/// Renders the `Alert` component.
pub struct Alert;

impl Alert {
    fn severity_colour(props: &Props, theme: &EddaCraftTheme) -> Color {
        // `type` is the shadcn field; `variant` is a common alias.
        let kind = str_prop(props, "type")
            .or_else(|| str_prop(props, "variant"))
            .unwrap_or("info");
        match kind {
            "warning" | "warn" => theme.warning(),
            "error" | "destructive" => theme.error(),
            "success" => theme.success(),
            _ => theme.accent(),
        }
    }
}

impl TuiComponent for Alert {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let colour = Self::severity_colour(props, &theme);
        let title = disp_or(props, "title", "");
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colour));
        if !title.is_empty() {
            block = block.title(Span::styled(title, Style::default().fg(colour)));
        }
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let body = disp(props, "children").or_else(|| disp(props, "description"));
        if let Some(body) = body
            && inner.width > 0
            && inner.height > 0
        {
            frame.render_widget(
                Paragraph::new(body)
                    .style(theme.base())
                    .wrap(Wrap { trim: true }),
                inner,
            );
        }
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

    fn render(props: &Props, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("backend");
        terminal
            .draw(|frame| Alert.render(props, frame, frame.area()))
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
    fn renders_title_and_body() {
        let p = json!({ "type": "warning", "title": "Heads up", "children": "two open warnings" });
        let text = render(p.as_object().expect("obj"), 30, 5);
        assert!(text.contains("Heads up"), "title: {text:?}");
        assert!(text.contains("two open"), "body: {text:?}");
    }

    #[test]
    fn unknown_type_and_no_body_do_not_panic() {
        let p = json!({ "type": "cosmic" });
        let _ = render(p.as_object().expect("obj"), 20, 3);
    }
}
