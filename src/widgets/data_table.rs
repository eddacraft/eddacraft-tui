//! Themed sortable data table built on Ratatui's [`Table`].
//!
//! Provides a thin layer over `ratatui::widgets::Table` that:
//! - Themes header, cursor, and border styles via [`Theme`].
//! - Renders an optional sort indicator (`▲` / `▼`) on the active column.
//! - Wraps `TableState` in [`DataTableState`] with cursor helpers.
//!
//! Sorting itself is the application's responsibility — pass already-sorted
//! `rows` plus a [`SortIndicator`] describing the current sort.
//!
//! ```rust,no_run
//! # use eddacraft_tui::prelude::*;
//! # use eddacraft_tui::widgets::data_table::{DataTable, DataTableState, SortDirection, SortIndicator};
//! # let theme = EddaCraftTheme;
//! let headers = ["Name", "Status"];
//! let rows = vec![
//!     vec!["build".to_string(), "ok".to_string()],
//!     vec!["test".to_string(), "fail".to_string()],
//! ];
//! let mut state = DataTableState::new();
//! state.move_down(rows.len());
//! let _ = DataTable::new(&theme, &headers, &rows)
//!     .sort(SortIndicator { column: 0, direction: SortDirection::Asc });
//! # let _ = state;
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, Row, StatefulWidget, Table, TableState, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Asc => "▲",
            Self::Desc => "▼",
        }
    }

    #[must_use]
    pub fn flipped(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SortIndicator {
    pub column: usize,
    pub direction: SortDirection,
}

pub struct DataTableState {
    pub(crate) table_state: TableState,
}

impl Default for DataTableState {
    fn default() -> Self {
        Self::new()
    }
}

impl DataTableState {
    /// Construct a new state with no row selected. Use [`Self::move_down`] /
    /// [`Self::move_up`] or [`Self::select`] to position the cursor once row
    /// data is known.
    #[must_use]
    pub fn new() -> Self {
        Self {
            table_state: TableState::default(),
        }
    }

    /// Construct a state with the cursor already positioned at `index`.
    /// Useful when the application persists and restores cursor position.
    #[must_use]
    pub fn with_selection(index: usize) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(index));
        Self { table_state }
    }

    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.table_state.selected()
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.table_state.select(index);
    }

    /// Visible scroll offset (top row index). Exposed so callers that build
    /// scrollbars or persist viewport position can read ratatui's internal
    /// scroll without a public field.
    #[must_use]
    pub fn scroll_offset(&self) -> usize {
        self.table_state.offset()
    }

    /// Move the cursor up, wrapping at the top. From an empty selection,
    /// snaps to the last row.
    pub fn move_up(&mut self, total: usize) {
        if total == 0 {
            self.table_state.select(None);
            return;
        }
        let next = match self.table_state.selected() {
            None | Some(0) => total - 1,
            Some(i) => i - 1,
        };
        self.table_state.select(Some(next));
    }

    /// Move the cursor down, wrapping at the bottom. From an empty selection,
    /// snaps to the first row.
    pub fn move_down(&mut self, total: usize) {
        if total == 0 {
            self.table_state.select(None);
            return;
        }
        let next = match self.table_state.selected() {
            None => 0,
            Some(i) if i + 1 >= total => 0,
            Some(i) => i + 1,
        };
        self.table_state.select(Some(next));
    }
}

pub struct DataTable<'a, T: Theme> {
    theme: &'a T,
    headers: &'a [&'a str],
    rows: &'a [Vec<String>],
    widths: Option<&'a [Constraint]>,
    sort: Option<SortIndicator>,
    block: Option<Block<'a>>,
}

impl<'a, T: Theme> DataTable<'a, T> {
    pub fn new(theme: &'a T, headers: &'a [&'a str], rows: &'a [Vec<String>]) -> Self {
        Self {
            theme,
            headers,
            rows,
            widths: None,
            sort: None,
            block: None,
        }
    }

    /// Override column widths. By default columns share the area equally.
    ///
    /// Panics in debug builds if `widths.len()` does not match
    /// `headers.len()`. In release builds the mismatch causes ratatui to
    /// silently allocate zero width to the uncovered columns, which renders
    /// as invisible columns rather than an error.
    #[must_use]
    pub fn widths(mut self, widths: &'a [Constraint]) -> Self {
        debug_assert_eq!(
            widths.len(),
            self.headers.len(),
            "DataTable::widths must supply one constraint per header column",
        );
        self.widths = Some(widths);
        self
    }

    /// Mark a column as actively sorted. The widget renders the
    /// [`SortDirection`] glyph next to that column's header label.
    ///
    /// Panics in debug builds if `sort.column` is out of bounds for
    /// `headers`. In release builds the indicator is silently dropped.
    #[must_use]
    pub fn sort(mut self, sort: SortIndicator) -> Self {
        debug_assert!(
            sort.column < self.headers.len(),
            "DataTable::sort column {} is out of bounds for {} headers",
            sort.column,
            self.headers.len(),
        );
        self.sort = Some(sort);
        self
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    fn header_row(&self) -> Row<'a> {
        let title_style = self.theme.title();
        let cells: Vec<Cell> = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let suffix = match self.sort {
                    Some(s) if s.column == i => format!(" {}", s.direction.glyph()),
                    _ => String::new(),
                };
                let text = format!("{h}{suffix}");
                Cell::from(Line::from(Span::styled(text, title_style)))
            })
            .collect();
        Row::new(cells).style(self.theme.base())
    }

    fn body_rows(&self) -> Vec<Row<'a>> {
        let cell_style = self.theme.base();
        self.rows
            .iter()
            .map(|row| {
                Row::new(
                    row.iter()
                        .map(|c| Cell::from(c.clone()).style(cell_style))
                        .collect::<Vec<_>>(),
                )
            })
            .collect()
    }
}

impl<T: Theme> StatefulWidget for DataTable<'_, T> {
    type State = DataTableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let widths_owned: Vec<Constraint>;
        let widths_ref: &[Constraint] = if let Some(w) = self.widths {
            w
        } else {
            let column_count = self.headers.len().max(1);
            widths_owned = (0..column_count)
                .map(|_| Constraint::Ratio(1, u32::try_from(column_count).unwrap_or(1)))
                .collect();
            &widths_owned
        };

        let header = self.header_row();
        let rows = self.body_rows();
        let mut table = Table::new(rows, widths_ref)
            .header(header)
            .row_highlight_style(self.theme.highlighted())
            .style(self.theme.base());

        if let Some(block) = self.block {
            table = table.block(block.border_style(self.theme.border_focused()));
        }

        StatefulWidget::render(table, area, buf, &mut state.table_state);
    }
}

/// Convenience [`Widget`] impl for stateless rendering (no cursor highlight).
impl<T: Theme> Widget for DataTable<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = DataTableState::new();
        state.select(None);
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::EddaCraftTheme;

    fn sample_rows() -> Vec<Vec<String>> {
        vec![
            vec!["build".into(), "ok".into()],
            vec!["test".into(), "fail".into()],
            vec!["lint".into(), "ok".into()],
        ]
    }

    #[test]
    fn new_starts_with_no_selection() {
        let state = DataTableState::new();
        assert_eq!(state.selected(), None);
    }

    #[test]
    fn with_selection_starts_at_index() {
        let state = DataTableState::with_selection(2);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn move_down_from_empty_selects_first_then_wraps() {
        let mut state = DataTableState::new();
        state.move_down(3);
        assert_eq!(state.selected(), Some(0));
        state.move_down(3);
        assert_eq!(state.selected(), Some(1));
        state.move_down(3);
        assert_eq!(state.selected(), Some(2));
        state.move_down(3);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_up_from_empty_wraps_to_last() {
        let mut state = DataTableState::new();
        state.move_up(3);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn empty_total_clears_selection() {
        let mut state = DataTableState::with_selection(1);
        state.move_down(0);
        assert_eq!(state.selected(), None);
        let mut state = DataTableState::with_selection(1);
        state.move_up(0);
        assert_eq!(state.selected(), None);
    }

    #[test]
    fn header_includes_sort_glyph_on_active_column() {
        let theme = EddaCraftTheme;
        let headers = ["Name", "Status"];
        let rows = sample_rows();
        let area = Rect::new(0, 0, 30, 6);
        let mut buf = Buffer::empty(area);
        let mut state = DataTableState::new();
        StatefulWidget::render(
            DataTable::new(&theme, &headers, &rows).sort(SortIndicator {
                column: 1,
                direction: SortDirection::Desc,
            }),
            area,
            &mut buf,
            &mut state,
        );
        let header_text: String = (0..30).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(header_text.contains("Status ▼"), "header={header_text:?}");
        assert!(!header_text.contains("Name ▲"), "header={header_text:?}");
        assert!(!header_text.contains("Name ▼"), "header={header_text:?}");
    }

    #[test]
    fn body_rows_render_text() {
        let theme = EddaCraftTheme;
        let headers = ["Name", "Status"];
        let rows = sample_rows();
        let area = Rect::new(0, 0, 30, 6);
        let mut buf = Buffer::empty(area);
        let mut state = DataTableState::new();
        StatefulWidget::render(
            DataTable::new(&theme, &headers, &rows),
            area,
            &mut buf,
            &mut state,
        );
        // Header is row 0, first body row at terminal row 1.
        let first_row: String = (0..30).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert!(first_row.contains("build"), "first_row={first_row:?}");
        let second_row: String = (0..30).map(|x| buf[(x, 2)].symbol().to_string()).collect();
        assert!(second_row.contains("test"), "second_row={second_row:?}");
    }

    #[test]
    fn cursor_row_uses_highlight_bg() {
        let theme = EddaCraftTheme;
        let headers = ["Name", "Status"];
        let rows = sample_rows();
        let area = Rect::new(0, 0, 30, 6);
        let mut buf = Buffer::empty(area);
        let mut state = DataTableState::new();
        state.select(Some(1));
        StatefulWidget::render(
            DataTable::new(&theme, &headers, &rows),
            area,
            &mut buf,
            &mut state,
        );
        // Selected row 1 is at terminal row 2 (header offsets by 1).
        let highlighted_bg = theme.highlighted().bg.unwrap();
        assert_eq!(buf[(0, 2)].bg, highlighted_bg);
    }

    #[test]
    fn sort_direction_flips_and_glyphs_resolve() {
        assert_eq!(SortDirection::Asc.glyph(), "▲");
        assert_eq!(SortDirection::Desc.glyph(), "▼");
        let flipped = SortDirection::Asc.flipped();
        assert_eq!(flipped.glyph(), "▼");
    }

    #[test]
    fn widget_impl_renders_without_cursor() {
        let theme = EddaCraftTheme;
        let headers = ["A"];
        let data = vec![vec!["x".into()]];
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        Widget::render(DataTable::new(&theme, &headers, &data), area, &mut buf);
        let body: String = (0..10).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert!(body.contains('x'));
    }
}
