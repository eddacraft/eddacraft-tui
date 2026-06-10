//! Anvil-domain dashboard components and the registry/catalogue that expose them.
//!
//! These composites (`GateResultCard`, `WarningList`, `DriftIndicator`,
//! `PlanCard`, `SuppressionRequest`, `EvidenceEntry`) combine eddacraft-tui
//! primitives into domain-meaningful widgets. Per ADR-054 the **generic** engine
//! and catalogue live in `eddacraft-tui`; these **Anvil-specific** components
//! live here and *extend* the base registry/catalogue:
//!
//! - [`anvil_registry`] = the generic [`base_registry`](eddacraft_tui::json_render::base_registry)
//!   plus these domain renderers — what the dashboard surface renders specs
//!   against.
//! - [`anvil_catalog`] = the generic [`Catalog::base`](eddacraft_tui::json_render::Catalog::base)
//!   plus these domain type names — what a spec validates against.
//!
//! Like the generic components, every renderer here degrades rather than panics
//! on absent or ill-typed props.

use eddacraft_tui::json_render::{Catalog, TuiRegistry, base_registry};

mod drift_indicator;
mod evidence_entry;
mod gate_result;
mod plan_card;
mod suppression;
mod warning_list;

pub use drift_indicator::DriftIndicator;
pub use evidence_entry::EvidenceEntry;
pub use gate_result::GateResultCard;
pub use plan_card::PlanCard;
pub use suppression::SuppressionRequest;
pub use warning_list::WarningList;

/// The shipped `gate-summary` dashboard spec — the single source of truth
/// for what `anvil init` seeds into `.anvil/dashboards/` and what the
/// spec-parity tests validate. Lives in crate assets rather than `.anvil/`
/// so the dogfood repo keeps its runtime tree untracked (ADR-073, CIB-053).
pub const GATE_SUMMARY_SPEC: &str =
    include_str!("../../assets/dashboards/gate-summary.dashboard.json");

/// The Anvil-domain component type names, in catalogue order.
pub const DOMAIN_COMPONENTS: [&str; 6] = [
    "GateResultCard",
    "WarningList",
    "DriftIndicator",
    "PlanCard",
    "SuppressionRequest",
    "EvidenceEntry",
];

/// A [`TuiRegistry`] with the generic base components plus the Anvil-domain
/// composites — the registry the dashboard surface renders against.
#[must_use]
pub fn anvil_registry() -> TuiRegistry {
    let mut registry = base_registry();
    registry.register("GateResultCard", Box::new(GateResultCard));
    registry.register("WarningList", Box::new(WarningList));
    registry.register("DriftIndicator", Box::new(DriftIndicator));
    registry.register("PlanCard", Box::new(PlanCard));
    registry.register("SuppressionRequest", Box::new(SuppressionRequest));
    registry.register("EvidenceEntry", Box::new(EvidenceEntry));
    registry
}

/// The base [`Catalog`] extended with the Anvil-domain component names — what a
/// dashboard spec is validated against before rendering.
#[must_use]
pub fn anvil_catalog() -> Catalog {
    let mut catalog = Catalog::base();
    for name in DOMAIN_COMPONENTS {
        catalog.insert(name);
    }
    catalog
}

/// Prop-reading helpers shared by the domain components. Each returns a default
/// rather than panicking on an absent or ill-typed prop.
mod props {
    use eddacraft_tui::json_render::{Props, sanitize};
    use serde_json::Value;

    /// Read a string prop for **matching** (status/variant enums), not display.
    pub(super) fn str_prop<'a>(props: &'a Props, key: &str) -> Option<&'a str> {
        props.get(key).and_then(Value::as_str)
    }

    /// Read a string prop for matching, with a fallback (matching only).
    pub(super) fn str_or<'a>(props: &'a Props, key: &str, default: &'a str) -> &'a str {
        str_prop(props, key).unwrap_or(default)
    }

    /// Read a **display** string prop, sanitised of control characters.
    pub(super) fn disp(props: &Props, key: &str) -> Option<String> {
        str_prop(props, key).map(sanitize)
    }

    /// Read a sanitised display string prop, falling back to trusted `default`.
    pub(super) fn disp_or(props: &Props, key: &str, default: &str) -> String {
        disp(props, key).unwrap_or_else(|| default.to_owned())
    }

    /// Read an array prop, or an empty slice if absent / not an array.
    pub(super) fn array_prop<'a>(props: &'a Props, key: &str) -> &'a [Value] {
        props
            .get(key)
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eddacraft_tui::json_render::check_parity;

    #[test]
    fn anvil_registry_covers_the_anvil_catalogue() {
        // Every catalogue name (base + domain) must have a renderer — no spec
        // component silently degrades to a placeholder.
        let parity = check_parity(&anvil_catalog(), &anvil_registry());
        assert!(
            parity.is_complete(),
            "unmapped components: {:?}",
            parity.missing_in_tui
        );
    }

    #[test]
    fn domain_components_are_registered() {
        let registry = anvil_registry();
        for name in DOMAIN_COMPONENTS {
            assert!(registry.contains(name), "{name} should be registered");
        }
    }
}
