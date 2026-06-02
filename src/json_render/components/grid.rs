//! `Grid` — fixed-column grid layout (`@eddacraft/render` shadcn built-in).
//!
//! Places children left-to-right, top-to-bottom across `columns` equal columns.
//! Per D-TUIDASH-003 and the TUIDASH-004 outcome, a narrow terminal (width below
//! [`NARROW_WIDTH`]) collapses the grid to a single column (vertical stacking),
//! since side-by-side cells become unreadable when each is only a few cells wide.
//! Draws no chrome of its own.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use super::props::{gap_spacing, usize_prop};
use crate::json_render::{Props, TuiComponent};

/// Below this terminal width the grid collapses to one column. Two 3-wide cells
/// either side of a gap carry no readable content, so stacking is strictly
/// better; the threshold matches the web `md` breakpoint intent.
pub const NARROW_WIDTH: u16 = 100;

/// Renders the `Grid` component.
pub struct Grid;

impl Grid {
    fn cells(child_count: usize, area: Rect, props: &Props) -> Vec<Rect> {
        if child_count == 0 || area.width == 0 || area.height == 0 {
            return Vec::new();
        }
        // `columns` defaults to 2 (the common metric-row shape); 0 or absent is
        // coerced to a sane minimum so we never divide by zero.
        let mut columns = usize_prop(props, "columns").unwrap_or(2).max(1);
        if area.width < NARROW_WIDTH {
            columns = 1;
        }
        columns = columns.min(child_count);
        let rows = child_count.div_ceil(columns);
        let gap = gap_spacing(props);

        let row_areas = Layout::vertical(vec![Constraint::Fill(1); rows])
            .spacing(gap)
            .split(area);

        let mut rects = Vec::with_capacity(child_count);
        for (row_idx, row_area) in row_areas.iter().enumerate() {
            // The final row may hold fewer than `columns` children; size it for
            // exactly those so they are not squeezed into a fraction of the row.
            let remaining = child_count - row_idx * columns;
            let cols_here = remaining.min(columns);
            let col_areas = Layout::horizontal(vec![Constraint::Fill(1); cols_here])
                .spacing(gap)
                .split(*row_area);
            rects.extend(col_areas.iter().copied());
        }
        rects.truncate(child_count);
        rects
    }
}

impl TuiComponent for Grid {
    fn render(&self, _props: &Props, _frame: &mut Frame, _area: Rect) {
        // Pure layout: no chrome.
    }

    fn layout_children(&self, props: &Props, area: Rect, child_count: usize) -> Vec<Rect> {
        Self::cells(child_count, area, props)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn props(v: serde_json::Value) -> Props {
        match v {
            serde_json::Value::Object(map) => map,
            _ => panic!("test props must be a JSON object"),
        }
    }

    #[test]
    fn three_columns_render_side_by_side_when_wide() {
        let p = props(json!({ "columns": 3 }));
        // Wide enough to keep all three columns.
        let rects = Grid::cells(3, Rect::new(0, 0, 120, 6), &p);
        assert_eq!(rects.len(), 3);
        // One row, three columns: same y, increasing x.
        assert!(rects.iter().all(|r| r.y == 0));
        assert!(rects[0].x < rects[1].x && rects[1].x < rects[2].x);
    }

    #[test]
    fn collapses_to_one_column_when_narrow() {
        let p = props(json!({ "columns": 3 }));
        let rects = Grid::cells(3, Rect::new(0, 0, 80, 9), &p);
        assert_eq!(rects.len(), 3);
        // Single column: same x, stacked down.
        assert!(rects.iter().all(|r| r.x == 0));
        assert!(rects[0].y < rects[1].y && rects[1].y < rects[2].y);
    }

    #[test]
    fn wraps_to_multiple_rows() {
        // 5 children, 2 columns → 3 rows (2,2,1).
        let p = props(json!({ "columns": 2 }));
        let rects = Grid::cells(5, Rect::new(0, 0, 120, 12), &p);
        assert_eq!(rects.len(), 5);
        // Row 0: two cells; row 2: a single cell taking the full width.
        assert_eq!(rects[0].y, rects[1].y, "first two share a row");
        assert!(rects[4].y > rects[2].y, "fifth is on a later row");
        assert_eq!(rects[4].x, 0, "lone trailing cell starts at left");
    }

    #[test]
    fn zero_columns_prop_does_not_divide_by_zero() {
        let p = props(json!({ "columns": 0 }));
        let rects = Grid::cells(4, Rect::new(0, 0, 120, 8), &p);
        assert_eq!(rects.len(), 4, "coerced to at least one column");
    }

    #[test]
    fn missing_columns_and_empty_area() {
        let p = props(json!({}));
        assert_eq!(Grid::cells(2, Rect::new(0, 0, 120, 4), &p).len(), 2);
        assert!(Grid::cells(0, Rect::new(0, 0, 120, 4), &p).is_empty());
        assert!(Grid::cells(3, Rect::new(0, 0, 0, 0), &p).is_empty());
    }
}
