//! Catalogue parity — keep the Rust component registry in step with the
//! `@eddacraft/render` web catalogue.
//!
//! The web catalogue (TypeScript, `packages/libs/render/src/catalog-registry.ts`)
//! is the source of truth for *which* components a spec may name. If the web
//! side gains a component that the TUI registry does not map, specs that use it
//! degrade to a placeholder in the terminal — usually a surprise, not a choice.
//! [`check_parity`] surfaces that gap.
//!
//! Direction matters:
//!
//! - **`missing_in_tui`** — a catalogue component with no registered renderer.
//!   This is the actionable gap (a web component the TUI silently can't draw).
//! - **`tui_only`** — a registered renderer absent from the catalogue (e.g. the
//!   generic chart widgets the TUI offers ahead of the web). Informational, not
//!   a defect.
//!
//! Per the architecture's "warnings over blocks" posture, callers decide
//! severity; the catalogue-parity test treats a non-empty `missing_in_tui` as a
//! failure and reports `tui_only` for visibility.
//!
//! Fully automated diffing against a TS-exported JSON Schema is gated on
//! DASHAI-002 (which exports that schema); until then the catalogue name list is
//! vendored (`tests/fixtures/json_render/catalog-names.json`) and mirrored by
//! [`Catalog::base`](crate::json_render::Catalog::base).

use std::collections::BTreeSet;

use crate::json_render::{Catalog, TuiRegistry};

/// The outcome of comparing a component [`Catalog`] against a [`TuiRegistry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogParity {
    /// Catalogue component names with no registered renderer — the actionable
    /// gap. Sorted.
    pub missing_in_tui: Vec<String>,
    /// Registered renderer names not present in the catalogue (e.g. charts).
    /// Informational. Sorted.
    pub tui_only: Vec<String>,
}

impl CatalogParity {
    /// Whether every catalogue component has a renderer (the gap that matters).
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.missing_in_tui.is_empty()
    }
}

/// Compare a component `catalog` (the web mirror) against a `registry`.
#[must_use]
pub fn check_parity(catalog: &Catalog, registry: &TuiRegistry) -> CatalogParity {
    let registered: BTreeSet<&str> = registry.names().collect();
    let catalogued: BTreeSet<&str> = catalog.names().collect();

    let missing_in_tui = catalogued
        .iter()
        .filter(|name| !registered.contains(*name))
        .map(|name| (*name).to_owned())
        .collect();
    let tui_only = registered
        .iter()
        .filter(|name| !catalogued.contains(*name))
        .map(|name| (*name).to_owned())
        .collect();

    CatalogParity {
        missing_in_tui,
        tui_only,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_render::base_registry;

    #[test]
    fn base_registry_maps_every_base_catalogue_component() {
        // Every name in the in-crate catalogue mirror must have a renderer.
        let parity = check_parity(&Catalog::base(), &base_registry());
        assert!(
            parity.is_complete(),
            "unmapped catalogue components: {:?}",
            parity.missing_in_tui
        );
    }

    #[test]
    fn charts_are_reported_as_tui_only_extras() {
        // The generic chart widgets are registered but not in the base catalogue;
        // they must show up as informational extras, never as a parity failure.
        let parity = check_parity(&Catalog::base(), &base_registry());
        for chart in ["LineChart", "BarChart", "SparklineChart", "HeatMap"] {
            assert!(
                parity.tui_only.iter().any(|n| n == chart),
                "{chart} should be reported as a TUI-only extra; got {:?}",
                parity.tui_only
            );
        }
    }

    #[test]
    fn an_unmapped_catalogue_component_is_reported() {
        let mut catalog = Catalog::base();
        catalog.insert("FlameGraph"); // not registered
        let parity = check_parity(&catalog, &base_registry());
        assert!(!parity.is_complete());
        assert!(parity.missing_in_tui.iter().any(|n| n == "FlameGraph"));
    }
}
