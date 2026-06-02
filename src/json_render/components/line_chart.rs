//! `LineChart` — a time-series line plot (Ratatui [`Chart`]).
//!
//! Reads a `data` array of numbers and plots them as one braille line series,
//! with the array index as the x value. Axis bounds are derived from the data;
//! a flat series is padded so it still draws. `title` labels the chart. Leaf
//! component.
//!
//! Not part of the current `@eddacraft/render` web catalogue (see TUIDASH-010
//! parity notes). Ratatui line charts are coarser than the web Recharts
//! renderer — a deliberate, documented trade-off (module risk table).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::symbols::Marker;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType};

use super::props::{disp, f64_array};
use crate::json_render::{Props, TuiComponent};
use crate::theme::{EddaCraftTheme, Theme};

/// Renders the `LineChart` component.
pub struct LineChart;

impl TuiComponent for LineChart {
    // Series indices and length become f64 axis coordinates. A dashboard series
    // is far below f64's 2^52 exact-integer ceiling, so the precision cast is safe.
    #[allow(clippy::cast_precision_loss)]
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let mut values = f64_array(props, "data");
        if values.is_empty() {
            return;
        }
        // Cap points fed to the chart per frame (DoS guard).
        values.truncate(super::MAX_CHART_POINTS);
        let theme = EddaCraftTheme;

        let points: Vec<(f64, f64)> = values
            .iter()
            .enumerate()
            .map(|(i, &y)| (i as f64, y))
            .collect();

        let x_max = (values.len().saturating_sub(1)) as f64;
        let (mut y_min, mut y_max) = values
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                (lo.min(v), hi.max(v))
            });
        // A flat series has y_min == y_max, which collapses the y-axis; pad it
        // so the line still renders mid-area.
        if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }

        let datasets = vec![
            Dataset::default()
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(theme.accent()))
                .data(&points),
        ];

        let mut chart = Chart::new(datasets)
            .x_axis(Axis::default().bounds([0.0, x_max.max(1.0)]))
            .y_axis(Axis::default().bounds([y_min, y_max]));
        if let Some(title) = disp(props, "title") {
            chart = chart.block(
                ratatui::widgets::Block::default()
                    .title(title)
                    .style(theme.base()),
            );
        }
        frame.render_widget(chart, area);
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

    fn draw(props: &Props) {
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).expect("backend");
        terminal
            .draw(|frame| LineChart.render(props, frame, frame.area()))
            .expect("draw");
    }

    #[test]
    fn renders_a_series_without_panic() {
        draw(
            json!({ "data": [1, 3, 2, 5, 4, 6], "title": "Throughput" })
                .as_object()
                .unwrap(),
        );
    }

    #[test]
    fn flat_series_does_not_collapse_the_axis() {
        // Equal values would give a zero-height y-axis; the pad must keep it valid.
        draw(json!({ "data": [7, 7, 7, 7] }).as_object().unwrap());
    }

    #[test]
    fn huge_data_is_capped_and_does_not_blow_up() {
        // Far more points than MAX_CHART_POINTS must render without a giant
        // per-frame allocation or panic.
        let data: Vec<serde_json::Value> = (0..50_000).map(|i| json!(i % 7)).collect();
        draw(json!({ "data": data }).as_object().unwrap());
    }

    #[test]
    fn empty_data_and_leaf() {
        draw(&Props::new());
        assert!(
            LineChart
                .layout_children(&Props::new(), Rect::new(0, 0, 10, 5), 1)
                .is_empty()
        );
    }
}
