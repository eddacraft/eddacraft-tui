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
use ratatui::Terminal;
use ratatui::backend::TestBackend;

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
fn template_specs_render_through_the_engine_without_panic() {
    // End-to-end: parse each real template, render it through the base registry
    // at a roomy terminal size, and assert nothing fell through to the renderer's
    // "[Type: not available in terminal]" placeholder. That proves `base_registry`
    // maps every component the shipped templates actually use.
    let registry = json_render::base_registry();
    for (label, json) in TEMPLATES {
        let spec = json_render::parse(json).unwrap_or_else(|e| panic!("{label}: parse: {e}"));
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test backend");
        terminal
            .draw(|frame| json_render::render_spec(&spec, &registry, frame, frame.area()))
            .unwrap_or_else(|e| panic!("{label}: draw: {e}"));
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            !text.contains("not available in terminal"),
            "{label}: an unmapped component degraded to a placeholder"
        );
        assert!(
            !text.contains("[missing:"),
            "{label}: a dangling child reference surfaced while rendering"
        );
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
