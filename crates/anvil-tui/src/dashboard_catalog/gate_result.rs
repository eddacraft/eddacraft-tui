//! `GateResultCard` — a gate-run summary card.
//!
//! Props: `status` (`pass`/`fail`/…), `score`, `timestamp`, and `checks` (an
//! array of `{ name, status }`). Renders a bordered card with a pass/fail
//! status badge, the score/timestamp, and a short per-check list. Leaf
//! component.

use eddacraft_tui::json_render::{Props, TuiComponent};
use eddacraft_tui::prelude::{
    BadgeStatus, Container, ContainerVariant, EddaCraftTheme, StatusBadge, Theme,
};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use serde_json::Value;

use super::props::{array_prop, str_or, str_prop};

/// Renders the `GateResultCard` component.
pub struct GateResultCard;

fn badge_status(status: &str) -> BadgeStatus {
    match status {
        "pass" | "success" | "ok" => BadgeStatus::Success,
        "fail" | "error" => BadgeStatus::Error,
        "warn" | "warning" => BadgeStatus::Warning,
        _ => BadgeStatus::Info,
    }
}

impl TuiComponent for GateResultCard {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let container = Container::new(&theme)
            .variant(ContainerVariant::Primary)
            .title("Gate Result");
        let inner = container.inner(area);
        frame.render_widget(container.to_block(), area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let status = str_or(props, "status", "info");
        let label = str_prop(props, "score")
            .map_or_else(|| status.to_uppercase(), |s| format!("score {s}"));
        // Render the status badge on the first inner row.
        let badge_area = Rect::new(inner.x, inner.y, inner.width, 1);
        frame.render_widget(
            StatusBadge::new(badge_status(status), &theme).label(&label),
            badge_area,
        );

        // Then the timestamp and a short check list beneath it.
        let mut lines: Vec<Line> = Vec::new();
        if let Some(ts) = str_prop(props, "timestamp") {
            lines.push(Line::styled(
                ts.to_owned(),
                Style::default().fg(theme.muted()),
            ));
        }
        for check in array_prop(props, "checks") {
            let name = check.get("name").and_then(Value::as_str).unwrap_or("");
            let cstatus = check.get("status").and_then(Value::as_str).unwrap_or("");
            let marker = if matches!(badge_status(cstatus), BadgeStatus::Success) {
                "✓"
            } else {
                "✗"
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{marker} "), theme.base()),
                Span::styled(name.to_owned(), theme.base()),
            ]));
        }
        if !lines.is_empty() {
            let body = Rect::new(
                inner.x,
                inner.y + 1,
                inner.width,
                inner.height.saturating_sub(1),
            );
            if body.height > 0 {
                frame.render_widget(Paragraph::new(lines), body);
            }
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
    fn shows_status_score_and_checks() {
        let p = json!({
            "status": "pass", "score": "92/100", "timestamp": "12:30",
            "checks": [{ "name": "secrets", "status": "pass" }, { "name": "lint", "status": "fail" }]
        });
        let mut terminal = Terminal::new(TestBackend::new(40, 8)).expect("backend");
        terminal
            .draw(|frame| GateResultCard.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("score 92/100"), "score: {text:?}");
        assert!(
            text.contains("secrets") && text.contains("lint"),
            "checks: {text:?}"
        );
    }

    #[test]
    fn missing_props_and_tiny_area_do_not_panic() {
        let mut terminal = Terminal::new(TestBackend::new(3, 2)).expect("backend");
        terminal
            .draw(|frame| GateResultCard.render(&Props::new(), frame, frame.area()))
            .expect("draw");
    }
}
