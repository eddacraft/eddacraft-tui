//! `MetricCard` — a single headline metric (custom Anvil catalogue component).
//!
//! A subtle bordered card showing a prominent `value`, a muted `label`, and an
//! optional `trend` arrow (`up`/`down`/`flat`). `format` is advisory — the spec
//! supplies the already-formatted `value` string, so it is rendered verbatim.
//! Leaf component.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::props::{str_or, str_prop};
use crate::json_render::{Props, TuiComponent};
use crate::theme::{EddaCraftTheme, Theme};
use crate::widgets::container::{Container, ContainerVariant};

/// Renders the `MetricCard` component.
pub struct MetricCard;

impl MetricCard {
    /// `(glyph, colour-picker)` for the trend arrow. The colour is directional,
    /// not good/bad — the spec carries no polarity, so up is rendered with the
    /// success colour and down with the error colour as a neutral convention.
    fn trend_span(props: &Props, theme: &EddaCraftTheme) -> Option<Span<'static>> {
        let (glyph, colour) = match str_prop(props, "trend")? {
            "up" => ("▲", theme.success()),
            "down" => ("▼", theme.error()),
            "flat" => ("▬", theme.muted()),
            _ => return None,
        };
        Some(Span::styled(
            format!(" {glyph}"),
            Style::default().fg(colour),
        ))
    }
}

impl TuiComponent for MetricCard {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let container = Container::new(&theme).variant(ContainerVariant::Subtle);
        let inner = container.inner(area);
        frame.render_widget(container.to_block(), area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let value = str_or(props, "value", "—");
        let label = str_or(props, "label", "");

        let mut value_line = vec![Span::styled(
            value.to_owned(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )];
        if let Some(trend) = Self::trend_span(props, &theme) {
            value_line.push(trend);
        }

        let lines = vec![
            Line::from(value_line),
            Line::styled(label.to_owned(), Style::default().fg(theme.muted())),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
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
        let mut terminal = Terminal::new(TestBackend::new(24, 5)).expect("backend");
        terminal
            .draw(|frame| MetricCard.render(props, frame, frame.area()))
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
    fn shows_value_label_and_trend() {
        let p = json!({ "label": "Pass Rate", "value": "94%", "trend": "up" });
        let text = render(p.as_object().expect("obj"));
        assert!(text.contains("94%"), "value: {text:?}");
        assert!(text.contains("Pass Rate"), "label: {text:?}");
        assert!(text.contains('▲'), "trend arrow: {text:?}");
    }

    #[test]
    fn missing_value_shows_em_dash_and_no_trend_glyph() {
        let p = json!({ "label": "Coverage" });
        let text = render(p.as_object().expect("obj"));
        assert!(text.contains('—'), "em dash for missing value: {text:?}");
        assert!(!text.contains('▲') && !text.contains('▼'));
    }

    #[test]
    fn is_a_leaf() {
        assert!(
            MetricCard
                .layout_children(&Props::new(), Rect::new(0, 0, 10, 4), 2)
                .is_empty()
        );
    }
}
