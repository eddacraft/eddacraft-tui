//! `SparklineChart` — a compact inline trend line (Ratatui [`Sparkline`]).
//!
//! Reads a `data` array of numbers and draws them as a single-row sparkline with
//! no axes — small enough to sit inside a `MetricCard`. Negative values clamp to
//! zero (a sparkline has no negative axis). Leaf component.
//!
//! Not part of the current `@eddacraft/render` web catalogue; it is a generic
//! chart the TUI offers ahead of the web (see TUIDASH-010 parity notes).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Sparkline;

use super::props::{f64_array, round_to_u64};
use crate::json_render::{Props, TuiComponent};
use crate::theme::{EddaCraftTheme, Theme};

/// Renders the `SparklineChart` component.
pub struct SparklineChart;

impl TuiComponent for SparklineChart {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        // Sparkline takes unsigned bar heights; clamp out negatives and round.
        let data: Vec<u64> = f64_array(props, "data")
            .into_iter()
            .map(round_to_u64)
            .collect();
        frame.render_widget(
            Sparkline::default()
                .data(&data)
                .style(Style::default().fg(theme.accent())),
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
    fn renders_data_without_panic() {
        let p = json!({ "data": [1, 4, 2, 8, 5, -3, 7] });
        let mut terminal = Terminal::new(TestBackend::new(14, 1)).expect("backend");
        terminal
            .draw(|frame| SparklineChart.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
    }

    #[test]
    fn empty_data_and_leaf() {
        let mut terminal = Terminal::new(TestBackend::new(10, 1)).expect("backend");
        terminal
            .draw(|frame| SparklineChart.render(&Props::new(), frame, frame.area()))
            .expect("draw");
        assert!(
            SparklineChart
                .layout_children(&Props::new(), Rect::new(0, 0, 10, 1), 2)
                .is_empty()
        );
    }
}
