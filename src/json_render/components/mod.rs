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

/// Upper bound on the number of children a layout component (`Stack`, `Grid`)
/// lays out. `children` is spec-controlled and unbounded; feeding hundreds of
/// thousands of constraints to ratatui's `Layout` solver on every frame is a
/// per-frame denial-of-service. Far more than this can never be legibly visible
/// at once, so trailing children beyond the cap are simply not given space (and
/// so are not drawn) — the same "fewer rects = not drawn" contract used elsewhere.
pub(crate) const MAX_LAYOUT_CHILDREN: usize = 512;

/// Upper bound on chart data points (`LineChart`, `BarChart`, `SparklineChart`).
/// A `data` array is spec/`$data`-controlled and unbounded; materialising and
/// re-allocating hundreds of thousands of points every frame is a per-frame
/// denial-of-service. Far more than this cannot be distinguished on a terminal
/// chart anyway, so excess points are dropped — mirroring `MAX_TABLE_ROWS`.
pub(crate) const MAX_CHART_POINTS: usize = 1_000;

mod alert;
mod badge;
mod bar_chart;
mod card;
mod grid;
mod heading;
mod line_chart;
mod metric_card;
mod placeholder;
mod progress;
mod separator;
mod sparkline_chart;
mod stack;
mod status_badge;
mod table;
mod text;

pub use alert::Alert;
pub use badge::Badge;
pub use bar_chart::BarChart;
pub use card::Card;
pub use grid::Grid;
pub use heading::Heading;
pub use line_chart::LineChart;
pub use metric_card::MetricCard;
pub use placeholder::Placeholder;
pub use progress::Progress;
pub use separator::Separator;
pub use sparkline_chart::SparklineChart;
pub use stack::Stack;
pub use status_badge::StatusBadge;
pub use table::Table;
pub use text::Text;

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
    register_data(&mut registry);
    register_charts(&mut registry);
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

/// Register the data-display components (TUIDASH-005): `Heading`, `Text`,
/// `Badge`, `StatusBadge`, `Alert`, `Progress`, `MetricCard`, `Table`.
fn register_data(registry: &mut TuiRegistry) {
    registry.register("Heading", Box::new(Heading));
    registry.register("Text", Box::new(Text));
    registry.register("Badge", Box::new(Badge));
    registry.register("StatusBadge", Box::new(StatusBadge));
    registry.register("Alert", Box::new(Alert));
    registry.register("Progress", Box::new(Progress));
    registry.register("MetricCard", Box::new(MetricCard));
    registry.register("Table", Box::new(Table));
}

/// Register the chart components (TUIDASH-006): `SparklineChart`, plus the
/// `HeatMap` placeholder (no meaningful terminal rendering, D-TUIDASH-001).
/// `LineChart`/`BarChart` are added alongside these.
fn register_charts(registry: &mut TuiRegistry) {
    registry.register("LineChart", Box::new(LineChart));
    registry.register("BarChart", Box::new(BarChart));
    registry.register("SparklineChart", Box::new(SparklineChart));
    registry.register("HeatMap", Box::new(Placeholder::new("HeatMap")));
}

/// Shared prop-reading helpers used across components.
///
/// All return a sensible default rather than panicking when a prop is absent or
/// the wrong JSON type, upholding the "rendering must not panic" constraint.
mod props {
    use crate::json_render::Props;
    use crate::json_render::sanitize::sanitize;

    /// Read a string prop, or `None` if absent / not a string.
    ///
    /// For values that are **matched** (status/variant/direction enums), not
    /// displayed. Display strings must go through [`disp`]/[`disp_or`] so control
    /// characters cannot reach the terminal.
    pub(super) fn str_prop<'a>(props: &'a Props, key: &str) -> Option<&'a str> {
        props.get(key).and_then(serde_json::Value::as_str)
    }

    /// Read a string prop with a fallback (matching only — see [`str_prop`]).
    pub(super) fn str_or<'a>(props: &'a Props, key: &str, default: &'a str) -> &'a str {
        str_prop(props, key).unwrap_or(default)
    }

    /// Read a **display** string prop, sanitised of control characters, or
    /// `None` if absent / not a string.
    pub(super) fn disp(props: &Props, key: &str) -> Option<String> {
        str_prop(props, key).map(sanitize)
    }

    /// Read a sanitised display string prop, falling back to `default` (which is
    /// trusted caller-supplied text and is not sanitised).
    pub(super) fn disp_or(props: &Props, key: &str, default: &str) -> String {
        disp(props, key).unwrap_or_else(|| default.to_owned())
    }

    /// Round a value to a non-negative `u64` chart height, clamping out
    /// negatives and `u64::MAX` overflow. Bar and sparkline heights are
    /// unsigned; the cast is bounds-checked here so the lossy `as` is safe.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub(super) fn round_to_u64(v: f64) -> u64 {
        if v <= 0.0 {
            0
        } else if v >= u64::MAX as f64 {
            u64::MAX
        } else {
            v.round() as u64
        }
    }

    /// Read a prop that is a JSON array of numbers into a `Vec<f64>` of at most
    /// `max` entries, skipping non-numeric ones. The `take(max)` runs *during*
    /// iteration, so an attacker-controlled array never materialises a giant
    /// intermediate `Vec` — only `max` points are ever allocated. An absent or
    /// non-array prop yields an empty vec.
    pub(super) fn f64_array_capped(props: &Props, key: &str, max: usize) -> Vec<f64> {
        props
            .get(key)
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(serde_json::Value::as_f64)
                    .take(max)
                    .collect()
            })
            .unwrap_or_default()
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
