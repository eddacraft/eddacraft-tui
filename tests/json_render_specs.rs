//! Fidelity tests for the json-render spec engine against the real
//! `@eddacraft/render` template specs.
//!
//! The three templates are vendored verbatim under
//! `tests/fixtures/json_render/` (copied from `packages/libs/render/specs/`) so
//! this crate stays self-contained when mirrored out per ADR-047. Catalogue
//! parity between the vendored copies and the web source is TUIDASH-010's job;
//! here we only prove the parser round-trips the authored shapes and accepts
//! them against the base catalogue.
#![cfg(feature = "json-render")]

use eddacraft_tui::json_render::{self, Catalog};

const TEMPLATES: [(&str, &str); 3] = [
    (
        "gate-summary",
        include_str!("fixtures/json_render/gate-summary.dashboard.json"),
    ),
    (
        "watch-session",
        include_str!("fixtures/json_render/watch-session.dashboard.json"),
    ),
    (
        "architecture-health",
        include_str!("fixtures/json_render/architecture-health.dashboard.json"),
    ),
];

#[test]
fn template_specs_round_trip() {
    for (label, json) in TEMPLATES {
        let spec =
            json_render::parse(json).unwrap_or_else(|e| panic!("{label}: parse failed: {e}"));
        let reserialised = json_render::to_json_pretty(&spec)
            .unwrap_or_else(|e| panic!("{label}: serialise: {e}"));
        let reparsed = json_render::parse(&reserialised)
            .unwrap_or_else(|e| panic!("{label}: reparse failed: {e}"));
        assert_eq!(spec, reparsed, "{label}: semantic round-trip mismatch");
    }
}

#[test]
fn template_specs_validate_against_base_catalogue() {
    let catalog = Catalog::base();
    for (label, json) in TEMPLATES {
        let spec = json_render::parse(json).unwrap_or_else(|e| panic!("{label}: parse: {e}"));
        json_render::validate(&spec, &catalog)
            .unwrap_or_else(|errs| panic!("{label}: unexpected validation errors: {errs:?}"));
    }
}

#[test]
fn templates_root_resolves_and_children_reference_real_elements() {
    for (label, json) in TEMPLATES {
        let spec = json_render::parse(json).unwrap_or_else(|e| panic!("{label}: parse: {e}"));
        assert!(
            spec.root_element().is_some(),
            "{label}: root `{}` must resolve",
            spec.root
        );
        // Every child id referenced anywhere must address a real element.
        for (id, element) in &spec.elements {
            for child in &element.children {
                assert!(
                    spec.element(child).is_some(),
                    "{label}: element `{id}` references missing child `{child}`"
                );
            }
        }
    }
}
