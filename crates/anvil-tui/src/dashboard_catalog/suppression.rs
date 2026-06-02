//! `SuppressionRequest` — a suppression's status, scope, and justification.
//!
//! Props: `status` (`pending`/`approved`/`denied`/…), `scope`, `justification`,
//! and `approver`. Renders a bordered card with a status badge and the
//! scope/justification/approver lines. Leaf component.

use eddacraft_tui::json_render::{Props, TuiComponent};
use eddacraft_tui::prelude::{
    BadgeStatus, Container, ContainerVariant, EddaCraftTheme, StatusBadge, Theme,
};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use super::props::{str_or, str_prop};

/// Renders the `SuppressionRequest` component.
pub struct SuppressionRequest;

fn badge_status(status: &str) -> BadgeStatus {
    match status {
        "approved" | "active" => BadgeStatus::Success,
        "denied" | "rejected" | "expired" => BadgeStatus::Error,
        "pending" => BadgeStatus::Warning,
        _ => BadgeStatus::Info,
    }
}

impl TuiComponent for SuppressionRequest {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let container = Container::new(&theme)
            .variant(ContainerVariant::Subtle)
            .title("Suppression");
        let inner = container.inner(area);
        frame.render_widget(container.to_block(), area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let status = str_or(props, "status", "info");
        let scope = str_prop(props, "scope").unwrap_or("");
        let badge_area = Rect::new(inner.x, inner.y, inner.width, 1);
        frame.render_widget(
            StatusBadge::new(badge_status(status), &theme).label(scope),
            badge_area,
        );

        let mut lines: Vec<Line> = Vec::new();
        if let Some(justification) = str_prop(props, "justification") {
            lines.push(Line::styled(justification.to_owned(), theme.base()));
        }
        if let Some(approver) = str_prop(props, "approver") {
            lines.push(Line::from(vec![
                Span::styled("approver: ", Style::default().fg(theme.muted())),
                Span::styled(approver.to_owned(), theme.base()),
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
                frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), body);
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
    fn shows_scope_justification_and_approver() {
        let p = json!({
            "status": "approved", "scope": "secrets/test-fixtures",
            "justification": "known test secrets", "approver": "josh"
        });
        let mut terminal = Terminal::new(TestBackend::new(44, 6)).expect("backend");
        terminal
            .draw(|frame| {
                SuppressionRequest.render(p.as_object().expect("obj"), frame, frame.area());
            })
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            text.contains("known test secrets"),
            "justification: {text:?}"
        );
        assert!(text.contains("josh"), "approver: {text:?}");
    }

    #[test]
    fn missing_props_and_tiny_area_do_not_panic() {
        let mut terminal = Terminal::new(TestBackend::new(3, 2)).expect("backend");
        terminal
            .draw(|frame| SuppressionRequest.render(&Props::new(), frame, frame.area()))
            .expect("draw");
    }
}
