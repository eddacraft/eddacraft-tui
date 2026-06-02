//! `Table` — a column/row data table (`@eddacraft/render` shadcn built-in; the
//! old spec's `DataTable`).
//!
//! Maps onto the eddacraft-tui [`DataTable`] widget, rendered as a plain
//! (stateless) [`Widget`](ratatui::widgets::Widget) — spec rendering carries no
//! per-element selection state, so the non-interactive form is used. `columns`
//! is an array of header strings; `rows` is an array of arrays of cell values
//! (each cell stringified). Leaf component.
//!
//! [`DataTable`]: crate::widgets::data_table::DataTable

use ratatui::Frame;
use ratatui::layout::Rect;
use serde_json::Value;

use crate::json_render::{Props, TuiComponent};
use crate::theme::EddaCraftTheme;
use crate::widgets::data_table::DataTable;

/// Renders the `Table` component.
pub struct Table;

/// Stringify a single JSON cell value for display. Objects/arrays/null render
/// as an em dash rather than raw JSON, keeping cells terse.
fn cell_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => "—".to_owned(),
    }
}

impl Table {
    /// Extract `(headers, rows)` from props, tolerating absent or ill-typed
    /// values by yielding empty collections.
    fn data(props: &Props) -> (Vec<String>, Vec<Vec<String>>) {
        let headers = props
            .get("columns")
            .and_then(Value::as_array)
            .map(|cols| cols.iter().map(cell_text).collect())
            .unwrap_or_default();
        let rows = props
            .get("rows")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .map(|row| {
                        row.as_array()
                            .map(|cells| cells.iter().map(cell_text).collect())
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        (headers, rows)
    }
}

impl TuiComponent for Table {
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = EddaCraftTheme;
        let (headers, rows) = Self::data(props);
        // `DataTable::new` borrows `&[&str]`, so the header strings must outlive
        // the widget; they do — both live to the end of this call.
        let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
        frame.render_widget(DataTable::new(&theme, &header_refs, &rows), area);
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
    fn extracts_headers_and_stringifies_mixed_cells() {
        let p = json!({
            "columns": ["Check", "Score", "Pass"],
            "rows": [["secrets", 92, true], ["lint", 88, false]]
        });
        let (headers, rows) = Table::data(p.as_object().expect("obj"));
        assert_eq!(headers, ["Check", "Score", "Pass"]);
        assert_eq!(rows[0], ["secrets", "92", "true"]);
        assert_eq!(rows[1], ["lint", "88", "false"]);
    }

    #[test]
    fn missing_or_ill_typed_data_yields_empty() {
        let (h, r) = Table::data(&Props::new());
        assert!(h.is_empty() && r.is_empty());
        let p = json!({ "columns": "nope", "rows": 7 });
        let (h, r) = Table::data(p.as_object().expect("obj"));
        assert!(h.is_empty() && r.is_empty());
    }

    #[test]
    fn renders_rows_without_panic() {
        let p = json!({ "columns": ["A", "B"], "rows": [["1", "2"], ["3", "4"]] });
        let mut terminal = Terminal::new(TestBackend::new(20, 6)).expect("backend");
        terminal
            .draw(|frame| Table.render(p.as_object().expect("obj"), frame, frame.area()))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains('A') && text.contains('1'), "got {text:?}");
    }
}
