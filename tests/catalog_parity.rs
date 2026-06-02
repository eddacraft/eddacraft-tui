//! Catalogue parity (TUIDASH-010): the Rust component registry must map every
//! component the `@eddacraft/render` web catalogue defines.
//!
//! The web catalogue's component names are vendored verbatim in
//! `tests/fixtures/json_render/catalog-names.json` (per ADR-047, so this crate
//! does not reach across package boundaries). This test treats that file as the
//! web source of truth and fails if either:
//!
//! 1. the in-crate [`Catalog::base`] mirror has drifted from it, or
//! 2. the [`base_registry`] leaves any catalogue component without a renderer.
//!
//! Refresh `catalog-names.json` when the web catalogue changes; a new web
//! component then fails this test until a Rust renderer (or explicit placeholder)
//! is registered for it.
#![cfg(feature = "json-render")]

use std::collections::BTreeSet;

use eddacraft_tui::json_render::{Catalog, base_registry, check_parity};

const CATALOG_NAMES: &str = include_str!("fixtures/json_render/catalog-names.json");

fn web_catalogue_names() -> Vec<String> {
    let parsed: serde_json::Value =
        serde_json::from_str(CATALOG_NAMES).expect("catalog-names.json is valid JSON");
    parsed["components"]
        .as_array()
        .expect("`components` is an array")
        .iter()
        .map(|v| v.as_str().expect("component name is a string").to_owned())
        .collect()
}

#[test]
fn in_crate_catalogue_mirror_matches_the_web_source() {
    // `Catalog::base()` is the hand-maintained mirror; the vendored fixture is
    // the web source. They must list exactly the same component names.
    let mirror: BTreeSet<String> = Catalog::base().names().map(str::to_owned).collect();
    let web: BTreeSet<String> = web_catalogue_names().into_iter().collect();
    assert_eq!(
        mirror, web,
        "Catalog::base() has drifted from @eddacraft/render \
         (vendored catalog-names.json) — update one to match the other"
    );
}

#[test]
fn registry_maps_every_web_catalogue_component() {
    // Build the catalogue straight from the vendored web names so this checks the
    // registry against the web source, not just the in-crate mirror.
    let web = Catalog::from_names(web_catalogue_names());
    let parity = check_parity(&web, &base_registry());
    assert!(
        parity.is_complete(),
        "these @eddacraft/render components have no TUI renderer: {:?}",
        parity.missing_in_tui
    );
}
