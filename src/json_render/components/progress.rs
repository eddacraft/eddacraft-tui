//! `Progress` — a determinate progress bar (`@eddacraft/render` shadcn built-in).
//!
//! Maps onto Ratatui's stateless [`Gauge`]. The house [`ProgressBar`] is
//! deliberately *not* used here: it animates `display_fraction` toward a target
//! across frames, but spec rendering rebuilds every component each frame with no
//! persistent per-element state, so an animated bar would always paint at zero.
//! [`Gauge`] fills immediately to the value, which is what a dashboard wants.
//!
//! [`ProgressBar`]: crate::widgets::progress_bar::ProgressBar

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Gauge;

use super::props::str_prop;
use crate::json_render::{Props, TuiComponent};
use crate::theme::{EddaCraftTheme, Theme};

/// Renders the `Progress` component.
pub struct Progress;

impl Progress {
    /// Resolve the fill fraction in `0.0..=1.0` from `value` against `max`
    /// (default 100). Out-of-range or absent values clamp rather than panic —
    /// [`Gauge::ratio`] panics outside the unit interval.
    fn fraction(props: &Props) -> f64 {
        let value = props
            .get("value")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let max = props
            .get("max")
            .and_then(serde_json::Value::as_f64)
            .filter(|m| *m > 0.0)
            .unwrap_or(100.0);
        (value / max).clamp(0.0, 1.0)
    }
}

impl TuiComponent for Progress {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let ratio = Self::fraction(props);
        let label = str_prop(props, "label")
            .map_or_else(|| format!("{:.0}%", ratio * 100.0), str::to_owned);
        frame.render_widget(
            Gauge::default()
                .ratio(ratio)
                .label(label)
                .gauge_style(Style::default().fg(theme.accent()).bg(theme.muted())),
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

    fn frac(v: serde_json::Value) -> f64 {
        let serde_json::Value::Object(map) = v else {
            panic!("test props must be a JSON object");
        };
        Progress::fraction(&map)
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn computes_fraction_against_default_and_explicit_max() {
        assert!(approx(frac(json!({ "value": 50 })), 0.5));
        assert!(approx(frac(json!({ "value": 3, "max": 4 })), 0.75));
    }

    #[test]
    fn out_of_range_values_clamp_so_gauge_never_panics() {
        assert!(approx(frac(json!({ "value": 250 })), 1.0));
        assert!(approx(frac(json!({ "value": -10 })), 0.0));
        assert!(approx(frac(json!({ "value": 5, "max": 0 })), 0.05)); // max<=0 ignored
        assert!(approx(frac(json!({})), 0.0));
    }

    #[test]
    fn renders_without_panic() {
        let p = json!({ "value": 92, "label": "92/100" });
        let mut terminal = Terminal::new(TestBackend::new(20, 1)).expect("backend");
        terminal
            .draw(|frame| Progress.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
    }
}
