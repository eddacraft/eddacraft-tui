//! `BarChart` — a categorical bar chart (Ratatui [`BarChart`]).
//!
//! Reads a `data` array whose entries are either `{ "label", "value" }` objects
//! or bare numbers (unlabelled). Values clamp to non-negative integers (bar
//! heights are unsigned). Leaf component.
//!
//! Not part of the current `@eddacraft/render` web catalogue (see TUIDASH-010
//! parity notes).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::BarChart as RatatuiBarChart;
use serde_json::Value;

use super::props::round_to_u64;
use crate::json_render::{Props, TuiComponent};
use crate::theme::{EddaCraftTheme, Theme};

/// Renders the `BarChart` component.
pub struct BarChart;

impl BarChart {
    /// Extract `(label, height)` bars from the `data` prop.
    fn bars(props: &Props) -> Vec<(String, u64)> {
        let Some(items) = props.get("data").and_then(Value::as_array) else {
            return Vec::new();
        };
        items
            .iter()
            .filter_map(|item| {
                if let Some(obj) = item.as_object() {
                    let label = obj
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned();
                    let value = obj.get("value").and_then(Value::as_f64)?;
                    Some((label, round_to_u64(value)))
                } else {
                    item.as_f64().map(|v| (String::new(), round_to_u64(v)))
                }
            })
            .collect()
    }
}

impl TuiComponent for BarChart {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let bars = Self::bars(props);
        if bars.is_empty() {
            return;
        }
        let theme = EddaCraftTheme;
        let refs: Vec<(&str, u64)> = bars.iter().map(|(l, v)| (l.as_str(), *v)).collect();
        frame.render_widget(
            RatatuiBarChart::default()
                .data(&refs[..])
                .bar_width(3)
                .bar_gap(1)
                .bar_style(Style::default().fg(theme.accent()))
                .value_style(theme.base()),
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
    fn parses_labelled_objects_and_clamps_negatives() {
        let p = json!({ "data": [
            { "label": "pass", "value": 12 },
            { "label": "fail", "value": -3 },
            { "label": "warn", "value": 4.6 }
        ] });
        let bars = BarChart::bars(p.as_object().expect("obj"));
        assert_eq!(
            bars,
            vec![
                ("pass".to_owned(), 12),
                ("fail".to_owned(), 0),
                ("warn".to_owned(), 5),
            ]
        );
    }

    #[test]
    fn parses_bare_numbers() {
        let p = json!({ "data": [3, 1, 4] });
        let bars = BarChart::bars(p.as_object().expect("obj"));
        assert_eq!(bars.iter().map(|(_, v)| *v).collect::<Vec<_>>(), [3, 1, 4]);
    }

    #[test]
    fn renders_and_empty_is_a_noop() {
        let mut terminal = Terminal::new(TestBackend::new(30, 8)).expect("backend");
        let p = json!({ "data": [{ "label": "a", "value": 5 }] });
        terminal
            .draw(|frame| BarChart.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
        // Empty data renders nothing, without panic.
        terminal
            .draw(|frame| BarChart.render(&Props::new(), frame, frame.area()))
            .expect("draw");
    }
}
