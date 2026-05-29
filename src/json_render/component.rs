//! The [`TuiComponent`] trait — the contract a registered renderer implements.
//!
//! A [`RenderSpec`](crate::json_render::RenderSpec) is a flat graph of
//! [`Element`](crate::json_render::Element)s, each naming a catalogue component
//! `type` and carrying a heterogeneous [`Props`](crate::json_render::Props)
//! bag. To turn that into terminal output, the tree walker (a later TUIDASH
//! item) needs, for each component type, two things:
//!
//! 1. how to **draw the element itself** into a Ratatui area, and
//! 2. how to **carve that area up** for its children, so the walker can recurse.
//!
//! [`TuiComponent`] is that contract. The walker looks a type name up in the
//! [`TuiRegistry`](crate::json_render::TuiRegistry), asks the component to
//! [`layout_children`](TuiComponent::layout_children) the area for its child
//! ids, draws each child into its sub-area, and calls
//! [`render`](TuiComponent::render) to paint the element's own chrome.
//!
//! Components receive props *as* [`serde_json::Value`] (via the
//! [`Props`](crate::json_render::Props) map), matching the json-render wire
//! format — a component reads the keys it understands and ignores the rest.
//! Per the module constraint, **rendering must not panic**: a component that
//! cannot interpret its props renders a degraded/empty view rather than
//! unwrapping.

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::json_render::Props;

/// A renderer for one json-render component `type`.
///
/// Implementations are registered in a [`TuiRegistry`](crate::json_render::TuiRegistry)
/// under their catalogue type name (e.g. `"Stack"`, `"MetricCard"`) and looked
/// up dynamically while walking a [`RenderSpec`](crate::json_render::RenderSpec).
/// They are held as trait objects (`Box<dyn TuiComponent>`), so the trait is
/// deliberately object-safe: no generic methods, no `Self`-returning methods.
///
/// # Contract
///
/// - [`render`](Self::render) **must not panic** on any [`Props`] input —
///   malformed or missing props degrade to an empty/placeholder view (the
///   module's "rendering a spec must not panic" constraint). It paints only the
///   element's own surface; child elements are drawn separately by the walker.
/// - [`layout_children`](Self::layout_children) partitions `area` into one
///   [`Rect`] per child, in child order. It is pure geometry — it must not draw
///   — and the returned vector's length need not equal `child_count` (a leaf
///   component returns an empty vector and ignores any children).
///
/// # Theme and render context
///
/// Unlike [`Surface::render`](crate::surface::Surface::render), this trait does
/// **not** take a `theme` parameter. The trait must stay object-safe so the
/// registry can hold `Box<dyn TuiComponent>`, which rules out a `theme: &T`
/// generic. The shared dashboard palette also lives in `anvil-tui` rather than
/// here (per the module constraints), so it is not available at this layer. The
/// tree walker (TUIDASH-003) will thread theme/data context to components via a
/// `&dyn`-object render context or a borrowed context struct passed alongside
/// `props`; that is intentionally out of scope for this foundational item.
pub trait TuiComponent {
    /// Draw this element's own surface into `area`.
    ///
    /// `props` is the element's [`Props`] bag, read as [`serde_json::Value`]s.
    /// The implementation reads the keys it recognises and ignores the rest;
    /// it must not panic on absent or ill-typed props.
    ///
    /// Child elements are not drawn here — the tree walker draws them into the
    /// sub-areas returned by [`layout_children`](Self::layout_children).
    fn render(&self, props: &Props, frame: &mut Frame, area: Rect);

    /// Partition `area` into one sub-[`Rect`] per child element.
    ///
    /// `child_count` is the number of child ids the element declares. The
    /// returned rectangles are consumed in child order; returning fewer than
    /// `child_count` simply means trailing children are not given space (and so
    /// are not drawn). The default implementation lays children out as equal
    /// vertical rows, which is a sane fallback for container-like components;
    /// leaf components should override it to return an empty vector.
    ///
    /// This method performs no drawing — it is pure layout geometry, so the
    /// walker can compute child areas before recursing.
    fn layout_children(&self, _props: &Props, area: Rect, child_count: usize) -> Vec<Rect> {
        if child_count == 0 || area.height == 0 {
            return Vec::new();
        }
        // Equal vertical division, distributing the remainder to the leading
        // rows so the rows tile `area` exactly with no gap or overrun.
        let count = u16::try_from(child_count).unwrap_or(u16::MAX);
        let base = area.height / count;
        let extra = area.height % count;
        let mut rects = Vec::with_capacity(child_count);
        let mut y = area.y;
        for i in 0..count {
            let h = base + u16::from(i < extra);
            rects.push(Rect::new(area.x, y, area.width, h));
            y = y.saturating_add(h);
        }
        rects
    }
}
