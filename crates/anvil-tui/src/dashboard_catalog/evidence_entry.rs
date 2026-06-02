//! `EvidenceEntry` — a single log-style evidence line.
//!
//! Props: `timestamp`, `actor`, and `message` (or `action`). Renders one line —
//! `timestamp  actor  message` — with the timestamp muted and the actor
//! accented, so a stack of entries reads like an audit log. Leaf component.

use eddacraft_tui::json_render::{Props, TuiComponent};
use eddacraft_tui::prelude::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::props::{disp, disp_or};

/// Renders the `EvidenceEntry` component.
pub struct EvidenceEntry;

impl TuiComponent for EvidenceEntry {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let timestamp = disp_or(props, "timestamp", "");
        let actor = disp_or(props, "actor", "");
        let message = disp(props, "message")
            .or_else(|| disp(props, "action"))
            .unwrap_or_default();

        let mut spans = Vec::new();
        if !timestamp.is_empty() {
            spans.push(Span::styled(
                format!("{timestamp}  "),
                Style::default().fg(theme.muted()),
            ));
        }
        if !actor.is_empty() {
            spans.push(Span::styled(
                format!("{actor}  "),
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ));
        }
        spans.push(Span::styled(message, theme.base()));
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
    fn renders_timestamp_actor_and_message() {
        let p = json!({ "timestamp": "12:30", "actor": "ci", "message": "gate passed" });
        let mut terminal = Terminal::new(TestBackend::new(40, 1)).expect("backend");
        terminal
            .draw(|frame| EvidenceEntry.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("12:30") && text.contains("ci") && text.contains("gate passed"));
    }

    #[test]
    fn missing_fields_do_not_panic() {
        let mut terminal = Terminal::new(TestBackend::new(20, 1)).expect("backend");
        terminal
            .draw(|frame| EvidenceEntry.render(&Props::new(), frame, frame.area()))
            .expect("draw");
    }
}
