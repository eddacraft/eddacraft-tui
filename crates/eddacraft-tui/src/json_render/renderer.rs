//! The tree renderer — walk a [`RenderSpec`] and draw it via a [`TuiRegistry`].
//!
//! A spec is a flat element graph (see [`spec`](crate::json_render::spec)); this
//! module turns it into terminal output. Starting at [`RenderSpec::root`], the
//! walker, for each element:
//!
//! 1. looks the element's `type` up in the [`TuiRegistry`];
//! 2. asks that [`TuiComponent`](crate::json_render::TuiComponent) to
//!    [`render`](crate::json_render::TuiComponent::render) its own chrome into
//!    the element's area;
//! 3. asks it to [`layout_children`](crate::json_render::TuiComponent::layout_children)
//!    the area into one sub-rect per child; then
//! 4. recurses into each child id, drawing it into its sub-rect.
//!
//! # Robustness
//!
//! The module's "rendering a spec must not panic" constraint is upheld even when
//! [`validate`](crate::json_render::validate) was **not** run first, so the
//! walker cannot assume a clean graph:
//!
//! - **Unknown component** (no renderer registered): drawn as a labelled
//!   placeholder — `[Type: not available in terminal]` (D-TUIDASH-001) — and its
//!   children are *not* descended into (there is no component to lay them out).
//! - **Dangling child** (a `children` id with no element): drawn as a small
//!   `[missing: id]` marker rather than indexing a missing key.
//! - **Cycles / pathological depth**: a [`MAX_DEPTH`] guard stops descent, so a
//!   `children` cycle that slipped past validation cannot loop forever.
//! - **Zero-area**: an empty rect short-circuits before any component runs.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::json_render::sanitize::sanitize;
use crate::json_render::{RenderSpec, TuiRegistry};

/// Maximum element-tree depth the renderer descends before drawing a guard
/// placeholder and stopping.
///
/// Authored dashboard specs nest only a handful of levels deep; this bound is an
/// order of magnitude beyond that. Its real job is a belt-and-braces stop for a
/// `children` cycle that reached the renderer without going through
/// [`validate`](crate::json_render::validate) (callers are not obliged to
/// validate first), so the recursive walk cannot run away.
pub const MAX_DEPTH: usize = 64;

/// Render `spec` into `area`, resolving each element's component against
/// `registry`.
///
/// This is the engine entry point: it draws the whole element tree, starting
/// from [`RenderSpec::root`], in one pass over the supplied [`Frame`]. It never
/// panics on a malformed spec — see the [module docs](self) for the degradation
/// rules. A zero-area `area` draws nothing.
pub fn render_spec(spec: &RenderSpec, registry: &TuiRegistry, frame: &mut Frame, area: Rect) {
    render_element(spec, registry, &spec.root, frame, area, 0);
}

/// Draw element `id` (and, for registered components, its children) into `area`.
fn render_element(
    spec: &RenderSpec,
    registry: &TuiRegistry,
    id: &str,
    frame: &mut Frame,
    area: Rect,
    depth: usize,
) {
    // Nothing can be drawn into an empty rect; stop before touching the element
    // so child layouts that ran out of space simply render nothing.
    if area.width == 0 || area.height == 0 {
        return;
    }

    if depth >= MAX_DEPTH {
        draw_placeholder(frame, area, "[max render depth reached]");
        return;
    }

    let Some(element) = spec.element(id) else {
        // A `children` entry pointing at no element. `validate` reports this as a
        // `DanglingChild`, but we may not have been validated — show a marker
        // instead of unwrapping a missing key. The id is spec-controlled, so it
        // is sanitised before display.
        draw_placeholder(frame, area, &format!("[missing: {}]", sanitize(id)));
        return;
    };

    let Some(component) = registry.get(&element.component_type) else {
        // No TUI equivalent for this component type (D-TUIDASH-001). Render a
        // labelled placeholder and do not descend — without a renderer there is
        // no layout for the children. Real catalogue components are registered
        // by the component-mapping work items (TUIDASH-004..-007).
        draw_placeholder(
            frame,
            area,
            &format!(
                "[{}: not available in terminal]",
                sanitize(&element.component_type)
            ),
        );
        return;
    };

    // NB: `element.visible` conditions are not evaluated yet — every element
    // renders unconditionally regardless of a `visible` expression. Conditional
    // visibility is deferred to a later work item; until then a spec author
    // cannot hide an element via `visible`.
    component.render(&element.props, frame, area);

    let rects = component.layout_children(&element.props, area, element.children.len());
    // `layout_children` may return fewer rects than children (trailing children
    // that did not fit get no space); `zip` draws exactly the pairs we have.
    for (child_id, child_area) in element.children.iter().zip(rects) {
        render_element(spec, registry, child_id, frame, child_area, depth + 1);
    }
}

/// Paint a single-line degradation marker into `area`.
///
/// Used for unknown components, dangling child references, and the depth guard.
/// It is intentionally plain (dim text, no border) so it reads as a diagnostic
/// rather than real content, and truncates to the area rather than wrapping.
fn draw_placeholder(frame: &mut Frame, area: Rect, label: &str) {
    let widget =
        Paragraph::new(Line::raw(label.to_owned())).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(widget, area);
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::text::Line;
    use ratatui::widgets::Paragraph;

    use super::*;
    use crate::json_render::{Props, TuiComponent, parse};

    /// A test component that paints a fixed marker string, proving the walker
    /// reached it. Layout falls back to the trait's default vertical split, so
    /// containers tile children down the area.
    struct Marker(&'static str);

    impl TuiComponent for Marker {
        fn render(&self, _props: &Props, frame: &mut Frame, area: Rect) {
            frame.render_widget(Paragraph::new(Line::raw(self.0)), area);
        }
    }

    /// A leaf marker that never claims space for children, used where a child
    /// must not be further subdivided.
    struct Leaf(&'static str);

    impl TuiComponent for Leaf {
        fn render(&self, _props: &Props, frame: &mut Frame, area: Rect) {
            frame.render_widget(Paragraph::new(Line::raw(self.0)), area);
        }
        fn layout_children(&self, _props: &Props, _area: Rect, _n: usize) -> Vec<Rect> {
            Vec::new()
        }
    }

    /// Flatten a rendered buffer into one string for `contains` assertions.
    fn buffer_text(buffer: &Buffer) -> String {
        buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    fn render_to_string(
        width: u16,
        height: u16,
        spec: &RenderSpec,
        registry: &TuiRegistry,
    ) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
        terminal
            .draw(|frame| render_spec(spec, registry, frame, frame.area()))
            .expect("draw");
        buffer_text(terminal.backend().buffer())
    }

    #[test]
    fn renders_a_multi_level_spec() {
        // Container > Grid > MetricCard x3 — the work item's acceptance shape.
        // Every level uses a distinct marker so we can prove each was reached.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "container",
                 "elements": {
                   "container": { "type": "Container", "props": {}, "children": ["grid"] },
                   "grid":      { "type": "Grid", "props": {}, "children": ["m1", "m2", "m3"] },
                   "m1": { "type": "MetricCard", "props": {}, "children": [] },
                   "m2": { "type": "MetricCard", "props": {}, "children": [] },
                   "m3": { "type": "MetricCard", "props": {}, "children": [] }
                 } }"#,
        )
        .expect("parse");

        let mut registry = TuiRegistry::new();
        registry.register("Container", Box::new(Marker("CONTAINER")));
        registry.register("Grid", Box::new(Marker("GRID")));
        registry.register("MetricCard", Box::new(Leaf("METRIC")));

        // Tall enough that the three metric rows each get their own line.
        let text = render_to_string(40, 9, &spec, &registry);
        // The grid's three metric children must all appear (the walker reached
        // each leaf), proving depth-first descent through two container levels.
        // The CONTAINER/GRID markers are legitimately overdrawn here: a parent
        // paints its chrome first, then its children paint over the same area
        // (correct z-order), and these stub markers all paint at the area's top
        // line. Parent chrome that children leave uncovered is exercised by
        // `parent_chrome_shows_where_children_do_not_cover`.
        assert_eq!(
            text.matches("METRIC").count(),
            3,
            "all three metrics render"
        );
    }

    #[test]
    fn parent_chrome_shows_where_children_do_not_cover() {
        // A container child that occupies only its sub-rect leaves the rest of
        // the parent area showing the parent's chrome — proving the parent is
        // drawn, not just skipped past to its children.

        // Banner gives its child only the bottom half, keeping the top for itself.
        struct Banner;
        impl TuiComponent for Banner {
            fn render(&self, _props: &Props, frame: &mut Frame, area: Rect) {
                frame.render_widget(Paragraph::new(Line::raw("BANNER")), area);
            }
            fn layout_children(&self, _props: &Props, area: Rect, _n: usize) -> Vec<Rect> {
                let half = area.height / 2;
                vec![Rect::new(
                    area.x,
                    area.y + half,
                    area.width,
                    area.height - half,
                )]
            }
        }

        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "page",
                 "elements": {
                   "page":  { "type": "Banner", "props": {}, "children": ["child"] },
                   "child": { "type": "Inner", "props": {}, "children": [] }
                 } }"#,
        )
        .expect("parse");

        let mut registry = TuiRegistry::new();
        registry.register("Banner", Box::new(Banner));
        registry.register("Inner", Box::new(Leaf("INNER")));

        let text = render_to_string(40, 6, &spec, &registry);
        assert!(text.contains("BANNER"), "parent chrome visible: {text:?}");
        assert!(
            text.contains("INNER"),
            "child drawn in its sub-rect: {text:?}"
        );
    }

    #[test]
    fn unknown_component_renders_a_placeholder() {
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "page",
                 "elements": {
                   "page": { "type": "Stack", "props": {}, "children": ["heat"] },
                   "heat": { "type": "HeatMap", "props": {}, "children": [] }
                 } }"#,
        )
        .expect("parse");

        let mut registry = TuiRegistry::new();
        registry.register("Stack", Box::new(Marker("STACK")));
        // HeatMap deliberately unregistered.

        let text = render_to_string(60, 4, &spec, &registry);
        assert!(
            text.contains("[HeatMap: not available in terminal]"),
            "unknown component degrades to a labelled placeholder, got: {text:?}"
        );
    }

    #[test]
    fn unknown_root_renders_only_a_placeholder_without_panic() {
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "page",
                 "elements": { "page": { "type": "Mystery", "props": {}, "children": [] } } }"#,
        )
        .expect("parse");
        let registry = TuiRegistry::new();
        let text = render_to_string(60, 3, &spec, &registry);
        assert!(text.contains("[Mystery: not available in terminal]"));
    }

    #[test]
    fn dangling_child_renders_a_marker_without_panic() {
        // `page` references a child that does not exist; the walker must not
        // index a missing key.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "page",
                 "elements": { "page": { "type": "Stack", "props": {}, "children": ["ghost"] } } }"#,
        )
        .expect("parse");
        let mut registry = TuiRegistry::new();
        registry.register("Stack", Box::new(Marker("STACK")));
        let text = render_to_string(40, 4, &spec, &registry);
        assert!(text.contains("[missing: ghost]"), "got: {text:?}");
    }

    #[test]
    fn cyclic_spec_terminates_and_does_not_panic() {
        // A self-cycle that bypassed `validate`. The MAX_DEPTH guard must stop
        // the recursion; if it did not, this test would hang rather than fail.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "a",
                 "elements": { "a": { "type": "Stack", "props": {}, "children": ["a"] } } }"#,
        )
        .expect("parse");
        let mut registry = TuiRegistry::new();
        registry.register("Stack", Box::new(Marker("A")));
        // Tall area so depth — not zero-height — is what stops descent, exercising
        // the MAX_DEPTH guard specifically.
        let text = render_to_string(40, 80, &spec, &registry);
        assert!(text.contains("[max render depth reached]"), "got: {text:?}");
    }

    #[test]
    fn visible_condition_is_currently_ignored() {
        // Documents present behaviour: an element with `visible: false` still
        // renders (conditions are not yet evaluated). A future implementer that
        // wires up visibility will flip this expectation deliberately.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "a",
                 "elements": { "a": { "type": "Mark", "props": {}, "children": [],
                     "visible": false } } }"#,
        )
        .expect("parse");
        let mut registry = TuiRegistry::new();
        registry.register("Mark", Box::new(Leaf("SHOWN")));
        let text = render_to_string(20, 2, &spec, &registry);
        assert!(text.contains("SHOWN"), "visible:false still renders today");
    }

    #[test]
    fn zero_area_draws_nothing() {
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "a",
                 "elements": { "a": { "type": "Stack", "props": {}, "children": [] } } }"#,
        )
        .expect("parse");
        let mut registry = TuiRegistry::new();
        registry.register("Stack", Box::new(Marker("A")));
        let mut terminal = Terminal::new(TestBackend::new(10, 5)).expect("backend");
        terminal
            .draw(|frame| render_spec(&spec, &registry, frame, Rect::new(0, 0, 0, 0)))
            .expect("draw");
        let text = buffer_text(terminal.backend().buffer());
        assert!(
            text.trim().is_empty(),
            "nothing drawn into a zero rect: {text:?}"
        );
    }
}
