//! `Stack` — the flex-like layout primitive (`@eddacraft/render` shadcn built-in).
//!
//! Lays its children out in one direction (`vertical` by default, or
//! `horizontal`), dividing the area into equal tracks with an optional `gap`.
//! It draws no chrome of its own — it is pure layout.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use super::props::{gap_spacing, str_or};
use crate::json_render::{Props, TuiComponent};

/// Renders the `Stack` component: equal-track layout along one axis.
pub struct Stack;

impl Stack {
    /// Equal tracks (`Fill(1)`) so children share the area evenly; the per-child
    /// natural size is unknown at this layer (the walker passes only the child
    /// count), so an even split is the honest generic default. Size-aware
    /// stacking is left to the responsive work (TUIDASH-011).
    ///
    /// The solver only receives one constraint per cell on the layout axis.
    /// More `Fill` constraints than cells makes ratatui fail the split
    /// (`ObjectiveUnbounded`) after a long solve. Children past that, up to
    /// [`MAX_LAYOUT_CHILDREN`](super::MAX_LAYOUT_CHILDREN), still get an empty
    /// rect at the stack origin so the walker keeps the child mapping and
    /// draws nothing. Children past the cap are not mapped;
    /// [`validate`](crate::json_render::validate) reports them.
    fn tracks(child_count: usize, area: Rect, props: &Props) -> Vec<Rect> {
        if child_count == 0 || area.width == 0 || area.height == 0 {
            return Vec::new();
        }
        let horizontal = str_or(props, "direction", "vertical") == "horizontal";
        let axis = usize::from(if horizontal { area.width } else { area.height });
        let solved = child_count.min(axis).min(super::MAX_LAYOUT_CHILDREN);
        let constraints = vec![Constraint::Fill(1); solved];
        let gap = gap_spacing(props);
        let layout = if horizontal {
            Layout::horizontal(constraints)
        } else {
            Layout::vertical(constraints)
        };
        let mut rects = layout.spacing(gap).split(area).to_vec();
        let mapped = child_count.min(super::MAX_LAYOUT_CHILDREN);
        if rects.len() < mapped {
            rects.resize(mapped, Rect::new(area.x, area.y, 0, 0));
        }
        rects
    }
}

impl TuiComponent for Stack {
    fn render(&self, _props: &Props, _frame: &mut Frame, _area: Rect) {
        // Pure layout: no chrome. Children are drawn by the walker into the
        // sub-rects from `layout_children`.
    }

    fn layout_children(&self, props: &Props, area: Rect, child_count: usize) -> Vec<Rect> {
        Self::tracks(child_count, area, props)
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
    fn vertical_is_the_default_direction() {
        let p = props(json!({}));
        let rects = Stack.layout_children(&p, Rect::new(0, 0, 10, 9), 3);
        assert_eq!(rects.len(), 3);
        // Stacked top-to-bottom, full width each, tiling the height.
        assert!(rects.iter().all(|r| r.width == 10));
        assert_eq!(rects[0].y, 0);
        assert!(rects[1].y > rects[0].y && rects[2].y > rects[1].y);
        let total: u16 = rects.iter().map(|r| r.height).sum();
        assert_eq!(total, 9, "vertical tracks tile the height");
    }

    #[test]
    fn horizontal_splits_across_width() {
        let p = props(json!({ "direction": "horizontal" }));
        let rects = Stack.layout_children(&p, Rect::new(0, 0, 12, 4), 3);
        assert_eq!(rects.len(), 3);
        assert!(rects.iter().all(|r| r.height == 4), "full height each");
        assert_eq!(rects[0].x, 0);
        assert!(rects[1].x > rects[0].x && rects[2].x > rects[1].x);
    }

    #[test]
    fn gap_inserts_spacing_between_tracks() {
        // `lg` gap = 2 cells between each of 2 tracks in a 12-row vertical area:
        // 2 tracks + one 2-row gap → tracks total 10 rows.
        let p = props(json!({ "gap": "lg" }));
        let rects = Stack.layout_children(&p, Rect::new(0, 0, 8, 12), 2);
        assert_eq!(rects.len(), 2);
        let gap = rects[1].y - (rects[0].y + rects[0].height);
        assert_eq!(gap, 2, "lg gap leaves two rows between tracks");
    }

    #[test]
    fn zero_children_or_empty_area_yields_no_rects() {
        let p = props(json!({}));
        assert!(
            Stack
                .layout_children(&p, Rect::new(0, 0, 10, 5), 0)
                .is_empty()
        );
        assert!(
            Stack
                .layout_children(&p, Rect::new(0, 0, 0, 0), 3)
                .is_empty()
        );
    }

    #[test]
    fn ill_typed_props_do_not_panic() {
        // direction as a number, gap as an object — must degrade, not panic.
        let p = props(json!({ "direction": 7, "gap": { "x": 1 } }));
        let rects = Stack.layout_children(&p, Rect::new(0, 0, 10, 6), 2);
        assert_eq!(rects.len(), 2); // fell back to vertical, zero gap
    }

    #[test]
    fn children_past_the_axis_keep_an_empty_mapping() {
        // Four rows cannot hold ten tracks. The solver sees four constraints;
        // the other six stay paired with an empty rect instead of being dropped.
        let p = props(json!({}));
        let rects = Stack.layout_children(&p, Rect::new(2, 3, 10, 4), 10);
        assert_eq!(rects.len(), 10);
        assert!(rects.iter().take(4).all(|r| r.height > 0 && r.width == 10));
        assert!(
            rects[4..].iter().all(|r| *r == Rect::new(2, 3, 0, 0)),
            "fallback rects are the stack origin with no size: {rects:?}"
        );
    }

    #[test]
    fn horizontal_children_past_the_axis_keep_an_empty_mapping() {
        let p = props(json!({ "direction": "horizontal" }));
        let rects = Stack.layout_children(&p, Rect::new(1, 1, 3, 6), 6);
        assert_eq!(rects.len(), 6);
        assert!(rects.iter().take(3).all(|r| r.width > 0 && r.height == 6));
        assert!(rects[3..].iter().all(|r| *r == Rect::new(1, 1, 0, 0)));
    }

    #[test]
    fn children_past_the_layout_cap_are_not_mapped() {
        let p = props(json!({}));
        let limit = super::super::MAX_LAYOUT_CHILDREN;
        let rects = Stack.layout_children(&p, Rect::new(0, 0, 10, 4), limit + 3);
        assert_eq!(rects.len(), limit);
        assert!(rects[4..].iter().all(|r| r.width == 0 && r.height == 0));
    }
}
