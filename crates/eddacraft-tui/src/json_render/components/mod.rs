//! Concrete [`TuiComponent`](crate::json_render::TuiComponent) implementations
//! for the `@eddacraft/render` base catalogue, plus the generic chart widgets.
//!
//! Each component maps one catalogue `type` name (e.g. `"Stack"`, `"MetricCard"`)
//! onto eddacraft-tui widgets or Ratatui primitives. They are registered into a
//! [`TuiRegistry`](crate::json_render::TuiRegistry) by [`base_registry`], which
//! the tree renderer ([`render_spec`](crate::json_render::render_spec)) walks.
//!
//! # Boundary (ADR-054)
//!
//! These are the **generic**, Anvil-agnostic components and therefore live in
//! `eddacraft-tui`. Anvil-domain composites (`GateResultCard`, `WarningList`, …)
//! live in `anvil-tui` and extend the registry returned here. Per the module
//! constraints, every component:
//!
//! - never panics on absent or ill-typed props (it degrades, reading props via
//!   the `as_*` accessors that return `None` rather than unwrapping);
//! - styles itself from the house [`EddaCraftTheme`] (a zero-size unit struct,
//!   so it is constructed locally rather than threaded through the trait).

use crate::json_render::TuiRegistry;

mod card;
mod grid;
mod separator;
mod stack;

pub use card::Card;
pub use grid::Grid;
pub use separator::Separator;
pub use stack::Stack;

/// Build a [`TuiRegistry`] with every generic base-catalogue component
/// registered under its catalogue type name.
///
/// This is the registry the dashboard surface renders specs against. Catalogue
/// component names that are not yet mapped fall through to the renderer's
/// placeholder (D-TUIDASH-001), so a partially-populated registry is safe.
///
/// Downstream surfaces (anvil-tui) extend the returned registry with their own
/// domain components before rendering.
#[must_use]
pub fn base_registry() -> TuiRegistry {
    let mut registry = TuiRegistry::new();
    register_layout(&mut registry);
    registry
}

/// Register the layout components (TUIDASH-004): `Stack`, `Grid`, `Card`,
/// `Separator`.
fn register_layout(registry: &mut TuiRegistry) {
    registry.register("Stack", Box::new(Stack));
    registry.register("Grid", Box::new(Grid));
    registry.register("Card", Box::new(Card));
    registry.register("Separator", Box::new(Separator));
}

/// Shared prop-reading helpers used across components.
///
/// All return a sensible default rather than panicking when a prop is absent or
/// the wrong JSON type, upholding the "rendering must not panic" constraint.
mod props {
    use crate::json_render::Props;

    /// Read a string prop, or `None` if absent / not a string.
    pub(super) fn str_prop<'a>(props: &'a Props, key: &str) -> Option<&'a str> {
        props.get(key).and_then(serde_json::Value::as_str)
    }

    /// Read a string prop with a fallback.
    pub(super) fn str_or<'a>(props: &'a Props, key: &str, default: &'a str) -> &'a str {
        str_prop(props, key).unwrap_or(default)
    }

    /// Read an integer-valued prop (JSON number, truncated), or `None`.
    pub(super) fn usize_prop(props: &Props, key: &str) -> Option<usize> {
        props
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .map(|n| usize::try_from(n).unwrap_or(usize::MAX))
    }

    /// Map a json-render `gap` token (`"none"|"sm"|"md"|"lg"|"xl"`, or a raw
    /// number) to a row/column spacing in terminal cells.
    ///
    /// Terminal space is scarce, so the scale is compressed relative to the
    /// web: `sm` and below collapse to no gap, `md` to one cell, `lg`/`xl` to
    /// two. An explicit numeric gap is clamped to a small ceiling so a spec
    /// authored for pixels cannot blow out the layout.
    pub(super) fn gap_spacing(props: &Props) -> u16 {
        match props.get("gap") {
            Some(v) if v.is_number() => {
                let n = v.as_u64().unwrap_or(0);
                u16::try_from(n.min(4)).unwrap_or(4)
            }
            Some(v) => match v.as_str().unwrap_or("") {
                "md" => 1,
                "lg" | "xl" => 2,
                _ => 0, // "none", "sm", unknown
            },
            None => 0,
        }
    }
}
