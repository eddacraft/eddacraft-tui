//! `StatusBadge` — pass/fail/warn/info indicator (a `@eddacraft/render`
//! base-catalogue component; generic, not Anvil-specific).
//!
//! A direct 1:1 onto the eddacraft-tui [`StatusBadge`](crate::widgets::status_badge::StatusBadge)
//! widget. The spec `status` enum (`pass`/`fail`/`warn`/`info`) maps onto the
//! widget's [`BadgeStatus`]; `label` is the trailing text. Leaf component.

use ratatui::Frame;
use ratatui::layout::Rect;

use super::props::{disp, str_or};
use crate::json_render::{Props, TuiComponent};
use crate::theme::EddaCraftTheme;
use crate::widgets::status_badge::{BadgeStatus, StatusBadge as StatusBadgeWidget};

/// Renders the `StatusBadge` component.
pub struct StatusBadge;

/// Map the spec status string onto the widget's [`BadgeStatus`]. Synonyms are
/// accepted so specs authored against gate vocabulary (`ok`, `error`) still
/// resolve; anything unrecognised degrades to the neutral `Info`.
fn badge_status(status: &str) -> BadgeStatus {
    match status {
        "pass" | "success" | "ok" => BadgeStatus::Success,
        "fail" | "error" => BadgeStatus::Error,
        "warn" | "warning" => BadgeStatus::Warning,
        "running" => BadgeStatus::Running,
        "skip" | "skipped" => BadgeStatus::Skipped,
        _ => BadgeStatus::Info,
    }
}

impl TuiComponent for StatusBadge {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let status = badge_status(str_or(props, "status", "info"));
        let label = disp(props, "label").unwrap_or_default();
        frame.render_widget(StatusBadgeWidget::new(status, &theme).label(&label), area);
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
    fn maps_status_synonyms() {
        // `BadgeStatus` does not derive `PartialEq`, so match rather than eq.
        assert!(matches!(badge_status("pass"), BadgeStatus::Success));
        assert!(matches!(badge_status("ok"), BadgeStatus::Success));
        assert!(matches!(badge_status("fail"), BadgeStatus::Error));
        assert!(matches!(badge_status("warn"), BadgeStatus::Warning));
        assert!(matches!(badge_status("info"), BadgeStatus::Info));
        assert!(matches!(badge_status("mystery"), BadgeStatus::Info));
    }

    #[test]
    fn renders_label_without_panic() {
        let p = json!({ "status": "pass", "label": "secrets: clean" });
        let mut terminal = Terminal::new(TestBackend::new(30, 1)).expect("backend");
        terminal
            .draw(|frame| StatusBadge.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("secrets: clean"), "got {text:?}");
    }
}
