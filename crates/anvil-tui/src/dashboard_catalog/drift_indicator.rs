//! `DriftIndicator` — a drift-score gauge with trend.
//!
//! Props: `score` (0–100), `label`, and `trend` (`up`/`down`/`flat`). Renders a
//! label, the score with a trend arrow, and a gauge bar. Leaf component.

use eddacraft_tui::json_render::{Props, TuiComponent};
use eddacraft_tui::prelude::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};

use super::props::{disp_or, str_prop};

/// Renders the `DriftIndicator` component.
pub struct DriftIndicator;

impl DriftIndicator {
    fn score(props: &Props) -> f64 {
        let score = props
            .get("score")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
            .clamp(0.0, 100.0);
        // `clamp` passes NaN through, and `Gauge::ratio` (score/100) panics on a
        // non-finite ratio — collapse NaN/inf to zero.
        if score.is_finite() { score } else { 0.0 }
    }

    fn trend_span(props: &Props, theme: &EddaCraftTheme) -> Option<Span<'static>> {
        let (glyph, colour) = match str_prop(props, "trend")? {
            "up" => ("▲", theme.error()), // rising drift is bad
            "down" => ("▼", theme.success()),
            "flat" => ("▬", theme.muted()),
            _ => return None,
        };
        Some(Span::styled(
            format!(" {glyph}"),
            Style::default().fg(colour),
        ))
    }
}

impl TuiComponent for DriftIndicator {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let score = Self::score(props);
        let label = disp_or(props, "label", "Drift");

        // Header line (label + score + trend), then a gauge beneath it.
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).split(area);

        let mut header = vec![
            Span::styled(format!("{label}: "), Style::default().fg(theme.muted())),
            Span::styled(
                format!("{score:.0}"),
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        if let Some(trend) = Self::trend_span(props, &theme) {
            header.push(trend);
        }
        frame.render_widget(Paragraph::new(Line::from(header)), rows[0]);

        if rows[1].height > 0 {
            frame.render_widget(
                Gauge::default()
                    .ratio(score / 100.0)
                    .label(format!("{score:.0}/100"))
                    .gauge_style(Style::default().fg(theme.accent()).bg(theme.muted())),
                rows[1],
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
    use serde_json::json;

    use super::*;

    #[test]
    fn clamps_score_into_range() {
        let over = json!({ "score": 250 });
        assert!((DriftIndicator::score(over.as_object().expect("obj")) - 100.0).abs() < 1e-9);
        let under = json!({ "score": -5 });
        assert!(DriftIndicator::score(under.as_object().expect("obj")).abs() < 1e-9);
    }

    #[test]
    fn renders_label_and_score_without_panic() {
        let p = json!({ "label": "Drift", "score": 23, "trend": "up" });
        let mut terminal = Terminal::new(TestBackend::new(30, 4)).expect("backend");
        terminal
            .draw(|frame| DriftIndicator.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Drift"), "label: {text:?}");
        assert!(text.contains("23"), "score: {text:?}");
    }
}
