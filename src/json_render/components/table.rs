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

use crate::json_render::responsive::max_table_columns;
use crate::json_render::sanitize::sanitize_to_width;
use crate::json_render::{Props, TuiComponent};
use crate::theme::EddaCraftTheme;
use crate::widgets::data_table::DataTable;

/// Hard ceiling on rows materialised, regardless of area. A spec (or a `$data`
/// reference into a huge `.anvil/` array) could declare millions of rows;
/// without a cap every redraw would stringify them all even though only a
/// screenful is visible.
const MAX_TABLE_ROWS: usize = 1_000;

/// Renders the `Table` component.
pub struct Table;

/// Stringify a single JSON cell value for display, sanitising string cells of
/// control characters and keeping only the text that fits in `max_width` cells
/// (no column is wider than the table's area). Objects/arrays/null render as
/// an em dash rather than raw JSON, keeping cells terse.
fn cell_text(value: &Value, max_width: usize) -> String {
    match value {
        Value::String(s) => sanitize_to_width(s, max_width),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => "—".to_owned(),
    }
}

impl Table {
    /// Extract `(headers, rows)` from props for drawing into `area`, tolerating
    /// absent or ill-typed values by yielding empty collections.
    ///
    /// Only what `area` can show is stringified, so per-frame work does not
    /// grow with the spec:
    ///
    /// - columns: the leading [`max_table_columns`] headers (progressive column
    ///   hiding, TUIDASH-011), and the same number of cells per row — one cell
    ///   when there are no headers, as the table then draws a single column;
    /// - rows: at most `area.height`, and never more than [`MAX_TABLE_ROWS`];
    /// - cells: at most `area.width` cells of sanitised text.
    // `map_or_default` is stable from Rust 1.98. This crate's MSRV is 1.88.
    #[allow(clippy::map_unwrap_or)]
    fn data(props: &Props, area: Rect) -> (Vec<String>, Vec<Vec<String>>) {
        let max_width = usize::from(area.width);
        let columns = props.get("columns").and_then(Value::as_array);
        let keep = max_table_columns(area.width, columns.map_or(0, Vec::len));
        let headers = columns
            .map(|cols| {
                cols.iter()
                    .take(keep)
                    .map(|v| cell_text(v, max_width))
                    .collect()
            })
            .unwrap_or_default();
        let cells_per_row = keep.max(1);
        let rows = props
            .get("rows")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .take(MAX_TABLE_ROWS.min(usize::from(area.height)))
                    .map(|row| {
                        row.as_array()
                            .map(|cells| {
                                cells
                                    .iter()
                                    .take(cells_per_row)
                                    .map(|v| cell_text(v, max_width))
                                    .collect()
                            })
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
        // Progressive column hiding (TUIDASH-011) happens during extraction: in
        // a narrow area only as many leading columns as fit at a readable width
        // are kept, so cells stay legible rather than squeezing to a character
        // or two, and the dropped columns are never stringified.
        let (headers, rows) = Self::data(props, area);
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

    /// An area large enough that no extraction bound applies.
    const WIDE: Rect = Rect::new(0, 0, 200, 60);

    #[test]
    fn extraction_keeps_only_the_columns_that_fit() {
        // 10,000 columns in a 40-cell area: only the leading columns that the
        // responsive limit keeps are stringified, for headers and every row.
        let columns: Vec<String> = (0..10_000).map(|i| format!("C{i}")).collect();
        let row: Vec<u32> = (0..10_000).collect();
        let p = json!({ "columns": columns, "rows": [row.clone(), row] });
        let area = Rect::new(0, 0, 40, 10);
        let keep = max_table_columns(area.width, 10_000);
        let (headers, rows) = Table::data(p.as_object().expect("obj"), area);
        assert_eq!(headers.len(), keep);
        assert_eq!(headers[0], "C0");
        assert!(rows.iter().all(|row| row.len() == keep), "{rows:?}");
    }

    #[test]
    fn extraction_without_headers_keeps_the_one_drawn_column() {
        // With no headers the table draws a single column, so wider rows are
        // cut to that column before their cells are stringified.
        let row: Vec<u32> = (0..10_000).collect();
        let p = json!({ "rows": [row] });
        let (headers, rows) = Table::data(p.as_object().expect("obj"), WIDE);
        assert!(headers.is_empty());
        assert_eq!(rows, [["0"]]);
    }

    #[test]
    fn extraction_keeps_only_the_rows_that_fit() {
        let rows: Vec<Vec<u32>> = (0..500).map(|i| vec![i]).collect();
        let p = json!({ "columns": ["N"], "rows": rows });
        let area = Rect::new(0, 0, 40, 5);
        let (_, rows) = Table::data(p.as_object().expect("obj"), area);
        assert_eq!(rows.len(), 5, "no more rows than the area is tall");
        assert_eq!(rows[0], ["0"]);
    }

    #[test]
    fn extraction_cuts_cell_text_to_the_area_width() {
        let long = "x".repeat(100_000);
        let p = json!({ "columns": [long.clone()], "rows": [[long]] });
        let area = Rect::new(0, 0, 40, 5);
        let (headers, rows) = Table::data(p.as_object().expect("obj"), area);
        assert_eq!(headers[0].chars().count(), 40);
        assert_eq!(rows[0][0].chars().count(), 40);
    }

    #[test]
    fn cell_cut_counts_display_width_not_characters() {
        // Combining marks take no cells, so 25 decomposed `é` (50 chars) fit a
        // 40-cell area whole; wide characters take two cells each.
        let decomposed = "e\u{301}".repeat(25);
        let wide = "漢".repeat(100);
        let p = json!({ "columns": ["A"], "rows": [[decomposed.clone()], [wide]] });
        let area = Rect::new(0, 0, 40, 5);
        let (_, rows) = Table::data(p.as_object().expect("obj"), area);
        assert_eq!(rows[0][0], decomposed);
        assert_eq!(rows[1][0], "漢".repeat(20));
    }

    #[test]
    fn cell_cut_bounds_a_flood_of_zero_width_characters() {
        let flood = format!("a{}", "\u{301}".repeat(100_000));
        let p = json!({ "columns": ["A"], "rows": [[flood]] });
        let area = Rect::new(0, 0, 40, 5);
        let (_, rows) = Table::data(p.as_object().expect("obj"), area);
        assert!(
            rows[0][0].chars().count() <= 40 * 4,
            "zero-width flood is cut"
        );
        assert!(rows[0][0].starts_with('a'));
    }

    #[test]
    fn cell_cut_counts_visible_characters_after_sanitising() {
        // Control characters are stripped before the cut, so they cannot use
        // up the width and hide the visible text.
        let hostile = format!("{}ok", "\u{1b}".repeat(100));
        let p = json!({ "columns": ["A"], "rows": [[hostile]] });
        let area = Rect::new(0, 0, 40, 5);
        let (_, rows) = Table::data(p.as_object().expect("obj"), area);
        assert_eq!(rows[0][0], "ok");
    }

    #[test]
    fn extracts_headers_and_stringifies_mixed_cells() {
        let p = json!({
            "columns": ["Check", "Score", "Pass"],
            "rows": [["secrets", 92, true], ["lint", 88, false]]
        });
        let (headers, rows) = Table::data(p.as_object().expect("obj"), WIDE);
        assert_eq!(headers, ["Check", "Score", "Pass"]);
        assert_eq!(rows[0], ["secrets", "92", "true"]);
        assert_eq!(rows[1], ["lint", "88", "false"]);
    }

    #[test]
    fn missing_or_ill_typed_data_yields_empty() {
        let (h, r) = Table::data(&Props::new(), WIDE);
        assert!(h.is_empty() && r.is_empty());
        let p = json!({ "columns": "nope", "rows": 7 });
        let (h, r) = Table::data(p.as_object().expect("obj"), WIDE);
        assert!(h.is_empty() && r.is_empty());
    }

    fn rendered_text(props: &Props, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("backend");
        terminal
            .draw(|frame| Table.render(props, frame, frame.area()))
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
    fn drops_trailing_columns_in_a_narrow_area() {
        // Six columns: a wide area keeps them all; a narrow area sheds the
        // trailing ones (TUIDASH-011 progressive column hiding).
        let p = json!({
            "columns": ["C0", "C1", "C2", "C3", "C4", "C5"],
            "rows": [["a", "b", "c", "d", "e", "f"]]
        });
        let obj = p.as_object().expect("obj");

        let wide = rendered_text(obj, 200, 4);
        assert!(wide.contains("C5"), "wide keeps every column: {wide:?}");

        let narrow = rendered_text(obj, 40, 4);
        assert!(narrow.contains("C0"), "narrow keeps leading columns");
        assert!(
            !narrow.contains("C5"),
            "narrow drops trailing columns: {narrow:?}"
        );
    }

    #[test]
    fn narrow_table_keeps_a_grapheme_that_fits_whole() {
        // Review regression (#7763): `👍🏽a` is three cells wide, so a three-cell
        // table must keep the skin-tone modifier with its base rather than
        // cutting the cluster by summing per-character widths.
        let thumbs = "\u{1F44D}\u{1F3FD}";
        let cell = format!("{thumbs}a");
        let p = json!({ "rows": [[cell.clone()]] });
        let obj = p.as_object().expect("obj");
        let (_, rows) = Table::data(obj, Rect::new(0, 0, 3, 4));
        assert_eq!(rows, [[cell]]);
        let text = rendered_text(obj, 3, 4);
        assert!(
            !text.contains('\u{1F44D}') || text.contains(thumbs),
            "modifier stays with its base: {text:?}"
        );
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
