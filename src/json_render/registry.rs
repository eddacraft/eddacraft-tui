//! [`TuiRegistry`] — dynamic component lookup by type name.
//!
//! The tree walker (a later TUIDASH item) does not know the concrete renderer
//! types ahead of time: a [`RenderSpec`](crate::json_render::RenderSpec) names
//! components by string `type`, so the mapping from name to renderer must be
//! resolved at run time. [`TuiRegistry`] is that map — type name →
//! [`TuiComponent`] trait object — built once (a base catalogue plus any
//! downstream registrations) and queried per element while walking the tree.
//!
//! A registry deliberately holds no spec or theme state: it is a pure name →
//! renderer table, so the same registry can render many specs.

use std::collections::BTreeMap;

use crate::json_render::component::TuiComponent;

/// A map from component type name to its [`TuiComponent`] renderer.
///
/// Renderers are stored as boxed trait objects so heterogeneous component types
/// can live in one table and be looked up dynamically by the string `type` from
/// a spec [`Element`](crate::json_render::Element). A [`BTreeMap`] keeps
/// [`names`](Self::names) deterministically ordered, which keeps catalogue-parity
/// diagnostics (TUIDASH-010) and any debug output stable.
#[derive(Default)]
pub struct TuiRegistry {
    components: BTreeMap<String, Box<dyn TuiComponent>>,
}

impl TuiRegistry {
    /// An empty registry. Use [`register`](Self::register) to populate it.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `component` under `name`, returning the previous renderer for
    /// that name if one was already registered (mirrors [`BTreeMap::insert`]).
    ///
    /// Re-registering a name replaces the renderer — last registration wins,
    /// which lets a downstream catalogue override a base component.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        component: Box<dyn TuiComponent>,
    ) -> Option<Box<dyn TuiComponent>> {
        self.components.insert(name.into(), component)
    }

    /// Look up the renderer registered for `name`, or `None` if the type is
    /// unregistered. The walker treats a `None` as a component with no TUI
    /// equivalent and substitutes a placeholder (D-TUIDASH-001).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&dyn TuiComponent> {
        self.components.get(name).map(AsRef::as_ref)
    }

    /// Whether a renderer is registered for `name`.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }

    /// The number of registered component types.
    #[must_use]
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Whether the registry has no registered components.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// All registered component type names, sorted.
    ///
    /// Used by the catalogue-parity check (TUIDASH-010) to compare the Rust
    /// registry against the web-side `@eddacraft/render` catalogue.
    #[must_use = "iterator is lazy and does nothing unless consumed"]
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.components.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Frame;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::text::Line;
    use ratatui::widgets::Paragraph;
    use serde_json::json;

    use super::*;
    use crate::json_render::Props;

    /// A minimal component standing in for a real widget mapping: it reads a
    /// `label` prop (degrading to a placeholder when it is absent or not a
    /// string) and paints it, proving props flow through as `serde_json` values.
    struct MockComponent;

    impl TuiComponent for MockComponent {
        fn render(&self, props: &Props, frame: &mut Frame, area: Rect) {
            // Reading props must never panic, even on a missing/ill-typed key.
            let label = props
                .get("label")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("<missing>");
            frame.render_widget(Paragraph::new(Line::raw(label.to_owned())), area);
        }
    }

    fn draw_with<F: FnOnce(&mut Frame)>(width: u16, height: u16, f: F) {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
        terminal.draw(|frame| f(frame)).expect("draw");
    }

    #[test]
    fn register_lookup_and_render_does_not_panic() {
        let mut registry = TuiRegistry::new();
        assert!(registry.is_empty());

        let previous = registry.register("Mock", Box::new(MockComponent));
        assert!(previous.is_none(), "first registration has no predecessor");
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("Mock"));

        // Dynamic lookup by type name returns the registered renderer.
        let component = registry.get("Mock").expect("Mock is registered");

        let props: Props = json!({ "label": "hello" })
            .as_object()
            .expect("object")
            .clone();

        // The headline acceptance check: looked-up component renders into a
        // real Frame area without panicking.
        draw_with(20, 3, |frame| {
            component.render(&props, frame, frame.area());
        });
    }

    #[test]
    fn missing_props_render_a_placeholder_without_panic() {
        let mut registry = TuiRegistry::new();
        registry.register("Mock", Box::new(MockComponent));
        let component = registry.get("Mock").expect("registered");

        // Empty props: the component must degrade, not unwrap-and-panic.
        let props = Props::new();
        draw_with(20, 3, |frame| {
            component.render(&props, frame, frame.area());
        });
    }

    #[test]
    fn unregistered_lookup_is_none() {
        let registry = TuiRegistry::new();
        assert!(registry.get("Nope").is_none());
        assert!(!registry.contains("Nope"));
    }

    #[test]
    fn re_registering_replaces_and_returns_previous() {
        let mut registry = TuiRegistry::new();
        assert!(registry.register("Mock", Box::new(MockComponent)).is_none());
        let previous = registry.register("Mock", Box::new(MockComponent));
        assert!(
            previous.is_some(),
            "second registration returns the displaced renderer"
        );
        assert_eq!(registry.len(), 1, "name still maps to a single renderer");
    }

    #[test]
    fn names_are_sorted_and_complete() {
        let mut registry = TuiRegistry::new();
        registry.register("Stack", Box::new(MockComponent));
        registry.register("Card", Box::new(MockComponent));
        registry.register("Heading", Box::new(MockComponent));
        let names: Vec<&str> = registry.names().collect();
        assert_eq!(names, ["Card", "Heading", "Stack"]);
    }

    #[test]
    fn default_layout_children_tiles_the_area_exactly() {
        let component = MockComponent;
        let area = Rect::new(0, 0, 10, 7);
        // 7 rows across 2 children: 4 + 3, contiguous, no overrun.
        let rects = component.layout_children(&Props::new(), area, 2);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0], Rect::new(0, 0, 10, 4));
        assert_eq!(rects[1], Rect::new(0, 4, 10, 3));
        let total: u16 = rects.iter().map(|r| r.height).sum();
        assert_eq!(total, area.height, "rows tile the area with no gap");
    }

    #[test]
    fn default_layout_children_handles_zero_children_and_zero_height() {
        let component = MockComponent;
        assert!(
            component
                .layout_children(&Props::new(), Rect::new(0, 0, 10, 5), 0)
                .is_empty()
        );
        assert!(
            component
                .layout_children(&Props::new(), Rect::new(0, 0, 10, 0), 3)
                .is_empty()
        );
    }

    #[test]
    fn default_layout_children_more_children_than_rows() {
        // 5 children but only 2 rows: the default gives the first 2 children one
        // row each and omits the rest — never a zero-height rect, so the walker's
        // `children.zip(rects)` draws exactly the 2 that fit (the "fewer rects =
        // not drawn" contract).
        let component = MockComponent;
        let area = Rect::new(0, 0, 10, 2);
        let rects = component.layout_children(&Props::new(), area, 5);
        assert_eq!(rects.len(), 2, "at most one row per available line");
        assert!(rects.iter().all(|r| r.height == 1), "no zero-height rects");
        let total: u16 = rects.iter().map(|r| r.height).sum();
        assert_eq!(total, area.height, "the rows still tile the area exactly");
    }

    #[test]
    fn wrong_prop_type_renders_placeholder_without_panic() {
        // The no-panic contract covers ill-typed props, not just absent ones:
        // a numeric `label` must degrade via `as_str() -> None`, not panic.
        let mut registry = TuiRegistry::new();
        registry.register("Mock", Box::new(MockComponent));
        let component = registry.get("Mock").expect("registered");
        let props: Props = json!({ "label": 42 }).as_object().expect("object").clone();
        draw_with(20, 3, |frame| {
            component.render(&props, frame, frame.area());
        });
    }
}
