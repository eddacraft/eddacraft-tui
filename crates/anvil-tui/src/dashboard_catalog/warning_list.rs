//! `WarningList` — a severity-coded list of warnings.
//!
//! Props: `warnings`, an array of `{ severity, message, location }` objects.
//! Each renders as one line led by a severity-coloured marker, with the file
//! location dimmed. More warnings than rows are truncated with a `… N more`
//! tail. Leaf component.

use eddacraft_tui::json_render::{Props, TuiComponent, sanitize};
use eddacraft_tui::prelude::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use serde_json::Value;

use super::props::array_prop;

/// Renders the `WarningList` component.
pub struct WarningList;

fn severity_colour(severity: &str, theme: &EddaCraftTheme) -> Color {
    match severity {
        "error" | "critical" | "high" => theme.error(),
        "warn" | "warning" | "medium" => theme.warning(),
        _ => theme.muted(),
    }
}

impl TuiComponent for WarningList {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let warnings = array_prop(props, "warnings");
        let capacity = area.height as usize;

        let mut lines: Vec<Line> = Vec::new();
        // Reserve a row for the "… N more" tail when the list overflows.
        let shown = if warnings.len() > capacity {
            capacity.saturating_sub(1)
        } else {
            warnings.len()
        };

        for warning in warnings.iter().take(shown) {
            let severity = field(warning, "severity").unwrap_or("");
            let message = field(warning, "message").unwrap_or("");
            let location = field(warning, "location").or_else(|| field(warning, "file"));

            let mut spans = vec![
                Span::styled("● ", Style::default().fg(severity_colour(severity, &theme))),
                Span::styled(sanitize(message), theme.base()),
            ];
            if let Some(location) = location {
                spans.push(Span::styled(
                    format!("  {}", sanitize(location)),
                    Style::default().fg(theme.muted()),
                ));
            }
            lines.push(Line::from(spans));
        }

        let hidden = warnings.len() - shown;
        if hidden > 0 {
            lines.push(Line::styled(
                format!("… {hidden} more"),
                Style::default().fg(theme.muted()),
            ));
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn layout_children(&self, _props: &Props, _area: Rect, _child_count: usize) -> Vec<Rect> {
        Vec::new()
    }
}

/// Read a string field from a warning object value.
fn field<'a>(warning: &'a Value, key: &str) -> Option<&'a str> {
    warning.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use serde_json::json;

    use super::*;

    fn render(props: &Props, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(50, h)).expect("backend");
        terminal
            .draw(|frame| WarningList.render(props, frame, frame.area()))
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[test]
    fn lists_warnings_with_messages_and_locations() {
        let p = json!({ "warnings": [
            { "severity": "error", "message": "secret leaked", "location": "a.rs:5" },
            { "severity": "warn", "message": "todo left", "file": "b.rs:9" }
        ] });
        let text = render(p.as_object().expect("obj"), 4);
        assert!(text.contains("secret leaked") && text.contains("a.rs:5"));
        assert!(text.contains("todo left") && text.contains("b.rs:9"));
    }

    #[test]
    fn overflow_is_summarised_with_a_more_tail() {
        let warnings: Vec<_> = (0..10)
            .map(|i| json!({ "severity": "warn", "message": format!("w{i}") }))
            .collect();
        let p = json!({ "warnings": warnings });
        let text = render(p.as_object().expect("obj"), 3);
        assert!(text.contains("more"), "overflow tail shown: {text:?}");
    }

    #[test]
    fn empty_and_missing_do_not_panic() {
        let _ = render(&Props::new(), 3);
        let _ = render(json!({ "warnings": [] }).as_object().expect("obj"), 3);
    }
}
