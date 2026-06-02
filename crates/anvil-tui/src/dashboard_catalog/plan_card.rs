//! `PlanCard` — an APS plan/module summary with a progress bar.
//!
//! Props: `title`, `status`, and progress as either `progress` (0–100) or a
//! `done`/`total` pair. Renders a bordered card with the title, a status line,
//! and a progress gauge labelled `done/total`. Leaf component.

use eddacraft_tui::json_render::{Props, TuiComponent};
use eddacraft_tui::prelude::{Container, ContainerVariant, EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Gauge, Paragraph};

use super::props::{disp, disp_or};

/// Renders the `PlanCard` component.
pub struct PlanCard;

impl PlanCard {
    /// `(ratio, label)` for the progress gauge. Prefers an explicit `done`/
    /// `total` pair (rendered `done/total`); falls back to a `progress`
    /// percentage; defaults to empty/zero.
    fn progress(props: &Props) -> (f64, String) {
        let num = |k: &str| props.get(k).and_then(serde_json::Value::as_f64);
        // `Gauge::ratio` panics on a non-finite ratio, so every path collapses
        // NaN/inf to zero (clamp alone does not sanitise NaN).
        let finite = |r: f64| if r.is_finite() { r } else { 0.0 };
        if let (Some(done), Some(total)) = (num("done"), num("total")) {
            let ratio = if total > 0.0 {
                finite((done / total).clamp(0.0, 1.0))
            } else {
                0.0
            };
            return (ratio, format!("{done:.0}/{total:.0}"));
        }
        let pct = finite(num("progress").unwrap_or(0.0).clamp(0.0, 100.0));
        (pct / 100.0, format!("{pct:.0}%"))
    }
}

impl TuiComponent for PlanCard {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let title = disp_or(props, "title", "Plan");
        let container = Container::new(&theme)
            .variant(ContainerVariant::Secondary)
            .title(&title);
        let inner = container.inner(area);
        frame.render_widget(container.to_block(), area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let rows = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).split(inner);
        if let Some(status) = disp(props, "status") {
            frame.render_widget(
                Paragraph::new(Line::styled(status, Style::default().fg(theme.muted()))),
                rows[0],
            );
        }
        if rows[1].height > 0 {
            let (ratio, label) = Self::progress(props);
            frame.render_widget(
                Gauge::default()
                    .ratio(ratio)
                    .label(label)
                    .gauge_style(Style::default().fg(theme.success()).bg(theme.muted())),
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
    fn computes_progress_from_done_total_or_percent() {
        let dt = json!({ "done": 3, "total": 4 });
        let (ratio, label) = PlanCard::progress(dt.as_object().expect("obj"));
        assert!((ratio - 0.75).abs() < 1e-9);
        assert_eq!(label, "3/4");

        let pct = json!({ "progress": 40 });
        let (ratio, label) = PlanCard::progress(pct.as_object().expect("obj"));
        assert!((ratio - 0.4).abs() < 1e-9);
        assert_eq!(label, "40%");

        // total == 0 must not divide by zero.
        let zero = json!({ "done": 1, "total": 0 });
        assert!(PlanCard::progress(zero.as_object().expect("obj")).0.abs() < 1e-9);
    }

    #[test]
    fn renders_title_and_status_without_panic() {
        let p = json!({ "title": "TUIDASH", "status": "In Progress", "done": 6, "total": 12 });
        let mut terminal = Terminal::new(TestBackend::new(30, 5)).expect("backend");
        terminal
            .draw(|frame| PlanCard.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("TUIDASH"), "title: {text:?}");
    }
}
